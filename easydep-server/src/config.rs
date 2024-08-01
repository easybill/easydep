/*
 * This file is part of easydep, licensed under the MIT License (MIT).
 *
 * Copyright (c) 2024 easybill GmbH
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str;

use anyhow::bail;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::process::Command;

/// The global configuration for the current EasyDep instance.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Configuration {
    /// The host and port to which the gRPC server should be bound.
    pub bind_host: String,
    /// The base directory in which deployments should be stored.
    pub base_directory: String,
    /// The id of the GitHub app.
    pub github_app_id: u64,
    /// The private key of the GitHub app in PEM format.
    pub github_app_pem_key_path: String,
    /// The amount of releases to keep locally on each server.
    pub retained_releases: u16,
    /// The deployment configurations that are defined. Each
    /// map key is the name of the configuration, mapped to
    /// the associated configuration.
    deployment_configs: Vec<DeploymentConfiguration>,
}

/// The configuration for each deployment configuration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct DeploymentConfiguration {
    /// The identifier of this deployment configuration.
    pub id: String,
    /// The name of the deployment target, used for example in directories.
    /// This name can be re-used for multiple configurations.
    pub target: String,
    /// Indicates if this configuration cannot be directly used for deployment
    /// and only for other configurations to extend it.
    pub extend_only: bool,
    /// The owner name of the repository from where the deployment
    /// can be triggered. Release ids when triggering a release will
    /// be resolved against this repository setting.
    pub source_repo_owner: String,
    /// The name of the repository from where the deployment
    /// can be triggered. Release ids when triggering a release will
    /// be resolved against this repository setting.
    pub source_repo_name: String,
    /// The names of all branches that are allowed to trigger a deployment
    /// using this configuration. If empty, all branches are allowed to
    /// trigger a deployment using this config.
    pub allowed_repo_branches: Vec<String>,
    /// The inverse of the allowed branches: The names of branches that are
    /// explicitly not allowed to trigger a deployment using this configuration.
    /// If empty, no branches will be denied the deployment using this config.
    /// Note: denied branches will be checked before allowed branches.
    pub denied_repo_branches: Vec<String>,
    /// The path to a file in a deployed directory where the checked-out revision
    /// should be stored. If not given the revision is not stored into a file.
    pub revision_file_name: Option<String>,
    /// The names of the configurations that are extended by this configuration.
    /// The extended configuration is executed first.
    pub extended_script_configurations: Vec<String>,
    /// The symlinks that should be created as part of this configuration.
    symlinks: Vec<String>,
}

/// Represents a symlink that can be provided to a deployment configuration.
/// These symlinks are created before any scripts are executed.
#[derive(Debug, Clone)]
pub(crate) struct Symlink {
    /// The source path in the directory being deployed which
    /// should be linked to the provided target path.
    pub source: String,
    /// The path to which the symlink should point.
    pub target: String,
}

impl Configuration {
    /// Loads the main configuration from the given file path. This
    /// method returns an error in case the given file path cannot
    /// be read or the configuration cannot be parsed.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path to the file to load the configuration from.
    pub async fn load_from_file(file_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let toml_file_content = fs::read_to_string(file_path).await?;
        let parsed_configuration: Configuration = toml::from_str(&toml_file_content)?;
        Ok(parsed_configuration)
    }

    /// Validates this configuration, returning the first validation error.
    pub async fn validate(&self) -> anyhow::Result<()> {
        // path to base deployment directory must be absolute, for example for symlinks to be correct
        // as we use bash internally on any platform the root must start with "/" (even on windows: /c/...)
        // therefore this check does not use .is_absolute.
        let base_dir_path = PathBuf::from(&self.base_directory);
        if !base_dir_path.starts_with("/") {
            bail!("base dir path must be absolute")
        }

        // check if all deployment configuration ids are unique
        let mut known_deployment_configs = HashSet::<&String>::new();
        for deployment_config in &self.deployment_configs {
            if !known_deployment_configs.insert(&deployment_config.id) {
                bail!(
                    "detected duplicate deployment configuration id: {}",
                    &deployment_config.id
                )
            }
        }

        // ensure that git is installed
        match Command::new("git").arg("--version").output().await {
            Ok(output) if output.status.success() => {
                info!(
                    "Detected {}",
                    String::from_utf8_lossy(output.stdout.as_slice()).trim()
                );
            }
            Ok(output) => bail!("git version check received unexpected {}", output.status),
            Err(err) => bail!("unable to detect running git version: {}", err),
        };

        Ok(())
    }

    /// Returns the deployment configuration with the given name,
    /// which can be None if no configuration with the name
    /// is registered.
    ///
    /// # Arguments
    /// * `id` - The id of the deployment configuration to get.
    pub fn get_deployment_configuration(&self, id: &String) -> Option<DeploymentConfiguration> {
        self.deployment_configs
            .iter()
            .find(|config| config.id.eq(id))
            .cloned()
    }

    /// Get the ids of the configured deployment configurations.
    pub fn get_deployment_configuration_ids(&self) -> Vec<String> {
        self.deployment_configs
            .iter()
            .filter(|config| !config.extend_only)
            .map(|config| config.id.clone())
            .collect()
    }
}

impl DeploymentConfiguration {
    /// Checks if the given branch is allowed to trigger a deployment
    /// using this deployment configuration. Note that denied branches
    /// are checked before allowed branches.
    ///
    /// # Arguments
    /// * `branch_name` - The name of the branch to check.
    pub fn is_branch_allowed_to_use_config(&self, branch_name: &String) -> bool {
        if self.denied_repo_branches.contains(branch_name) {
            false
        } else {
            self.allowed_repo_branches.is_empty()
                || self.allowed_repo_branches.contains(branch_name)
        }
    }

    /// Parses the symlinks that are provided to this configuration.
    pub fn get_symlinks(&self) -> Vec<Symlink> {
        self.symlinks
            .iter()
            .map(|part| part.split_once(':'))
            .filter(|split| split.is_some())
            .map(|split| {
                let (source, target) = split.unwrap();
                Symlink {
                    source: source.to_string(),
                    target: target.to_string(),
                }
            })
            .collect()
    }
}
