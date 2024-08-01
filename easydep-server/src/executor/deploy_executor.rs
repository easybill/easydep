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

use octocrab::models::repos::Release;
use secrecy::SecretString;
use tokio::sync::mpsc::Sender;
use tonic::Status;

use crate::accessor::deploy_status_accessor::{DeployExecutionState, DeployStatusAccessor};
use crate::accessor::deployment_accessor::DeploymentAccessor;
use crate::config::{Configuration, DeploymentConfiguration};
use crate::easydep::ExecutedActionEntry;
use crate::executor::deploy_delete_excutor::delete_deployment;
use crate::executor::deploy_init_executor::init_deployment;
use crate::executor::deploy_publish_executor::publish_deployment;

/// Holds the information about a single deployment.
#[derive(Clone, Debug)]
pub(crate) struct DeployExecutor {
    /// The release that is being deployed.
    release: Release,
    /// The directory into which the release is deployed.
    deployment_directory: PathBuf,
    /// The token to access git https resources on GitHub with.
    github_access_token: SecretString,
    /// The parsed global server configuration.
    global_configuration: Configuration,
    /// The accessor for releases stored on the disk.
    deployment_accessor: DeploymentAccessor,
    /// The deployment profile configuration used for the current deployed.
    deployment_configuration: DeploymentConfiguration,
    /// The status accessor for the current deployment.
    deployment_status_accessor: DeployStatusAccessor,
}

impl DeployExecutor {
    /// Constructs a new deployment executor that is in the preparing state.
    ///
    /// # Arguments
    /// * `release` - The release that is being deployed.
    /// * `github_access_token` - An access token for git https operations for the target repository of the release.
    /// * `global_configuration` - The server configuration.
    /// * `deployment_accessor` - The accessor for deployment information stored on the disk.
    /// * `deployment_configuration` - The deployment profile configuration for the current release.
    pub fn new(
        release: Release,
        github_access_token: SecretString,
        global_configuration: Configuration,
        deployment_accessor: DeploymentAccessor,
        deployment_configuration: DeploymentConfiguration,
    ) -> Self {
        let deployment_directory =
            deployment_accessor.get_release_directory(&deployment_configuration, &release.id.0);
        let deployment_status_accessor = DeployStatusAccessor::new();
        Self {
            release,
            deployment_directory,
            github_access_token,
            global_configuration,
            deployment_accessor,
            deployment_configuration,
            deployment_status_accessor,
        }
    }

    /// Get the id of the release that is being deployed.
    pub fn get_release_id(&self) -> u64 {
        self.release.id.0
    }

    /// Get the release that is currently being deployed.
    pub fn get_release(&self) -> &Release {
        &self.release
    }

    /// Get the status accessor associated with this deployment executor.
    pub fn get_status_accessor(&self) -> &DeployStatusAccessor {
        &self.deployment_status_accessor
    }

    /// Starts to prepare this deployment. This method does not make
    /// any status checks and assumes that they have been done before.
    ///
    /// # Arguments
    /// * `output_sender` - The sender for output log lines that are logged by scripts run in the steps.
    pub async fn prepare_deployment(
        &self,
        output_sender: Sender<Result<ExecutedActionEntry, Status>>,
    ) {
        init_deployment(
            &self.release,
            &self.deployment_directory,
            &self.github_access_token,
            &self.deployment_configuration,
            &output_sender,
        )
        .await;
        self.deployment_status_accessor
            .set_state(DeployExecutionState::Prepared)
            .await;
    }

    /// Publishes this deployment. This method does not make
    /// any status checks and assumes that they have been done before.
    ///
    /// # Arguments
    /// * `output_sender` - The sender for output log lines that are logged by scripts run in the steps.
    pub async fn publish_deployment(
        &self,
        output_sender: Sender<Result<ExecutedActionEntry, Status>>,
    ) {
        publish_deployment(
            &self.release,
            &self.deployment_directory,
            &self.global_configuration,
            &self.deployment_accessor,
            &self.deployment_configuration,
            &output_sender,
        )
        .await;
        self.deployment_status_accessor
            .set_state(DeployExecutionState::Published)
            .await;
    }

    /// Deletes this deployment. This method does not make
    /// any status checks and assumes that they have been done before.
    ///
    /// # Arguments
    /// * `output_sender` - The sender for output log lines that are logged by scripts run in the steps.
    pub async fn delete_deployment(
        &self,
        output_sender: Sender<Result<ExecutedActionEntry, Status>>,
    ) {
        delete_deployment(
            &self.release,
            &self.deployment_directory,
            &self.deployment_configuration,
            &output_sender,
        )
        .await;
        self.deployment_status_accessor
            .set_state(DeployExecutionState::Deleted)
            .await;
    }
}
