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

use std::path::PathBuf;

use anyhow::bail;
use tokio::fs::read_dir;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;

use crate::config::{Configuration, DeploymentConfiguration};

/// An accessor for deployments that are stored on the disk.
#[derive(Clone, Debug)]
pub struct DeploymentAccessor {
    deployment_base_dir: PathBuf,
}

impl DeploymentAccessor {
    /// Constructs a new deployment accessor with the given base directory.
    ///
    /// # Arguments
    /// * `config` - The server configuration, used to get the deployment base directory.
    pub fn new(config: &Configuration) -> Self {
        let deployment_base_dir = PathBuf::from(&config.base_directory);
        Self {
            deployment_base_dir,
        }
    }

    /// Get the path to the symlink directory based on the given deployment profile.
    ///
    /// # Arguments
    /// * `profile` - The profile to get the current symlink directory path of.
    pub fn get_current_release_directory(&self, profile: &DeploymentConfiguration) -> PathBuf {
        self.deployment_base_dir
            .join(format!("current-{}", profile.target))
    }

    /// Get the directory where the releases for the given profile are stored.
    ///
    /// # Arguments
    /// * `profile` - The profile to get the release storing directory of.
    pub fn get_releases_directory(&self, profile: &DeploymentConfiguration) -> PathBuf {
        self.deployment_base_dir
            .join("releases")
            .join(&profile.target)
    }

    /// Get the path to the directory where the for the given profile is stored.
    ///
    /// # Arguments
    /// * `profile` - The profile to get the release subdirectory of.
    /// * `release_id` - The id of the release to get the release directory for.
    pub fn get_release_directory(
        &self,
        profile: &DeploymentConfiguration,
        release_id: &u64,
    ) -> PathBuf {
        self.get_releases_directory(profile)
            .join(release_id.to_string())
    }

    /// Get all release directories that were created for the given deployment profile.
    /// The returned vec is sorted by the release id, descending.
    ///
    /// # Arguments
    /// * `profile` - The release profile to get the deployed directories of.
    pub async fn get_release_directories_for_profile(
        &self,
        profile: &DeploymentConfiguration,
    ) -> anyhow::Result<Vec<(PathBuf, u64)>> {
        // get the content in the releases directory
        let releases_directory = self.get_releases_directory(profile);
        let mut directory_content = match read_dir(&releases_directory).await {
            Ok(directory_content) => ReadDirStream::new(directory_content),
            Err(err) => bail!("unable to read entries from deploy directory: {err}"),
        };

        // find the directories that were created from a release
        // (by checking if the directory names can be parsed as a numeric id)
        let mut release_directories: Vec<(PathBuf, u64)> = Vec::new();
        while let Some(entry) = directory_content.next().await {
            if let Ok(entry) = entry {
                if entry
                    .file_type()
                    .await
                    .map(|file_type| file_type.is_dir())
                    .unwrap_or(false)
                {
                    if let Some(dir_name) = entry
                        .path()
                        .file_name()
                        .and_then(|dir_name| dir_name.to_str().map(|name| name.to_string()))
                    {
                        if let Ok(id) = dir_name.parse::<u64>() {
                            release_directories.push((entry.path(), id));
                        }
                    }
                }
            }
        }

        // sort the parsed release directories, descending
        release_directories.sort_by(|left, right| right.1.cmp(&left.1));
        Ok(release_directories)
    }
}
