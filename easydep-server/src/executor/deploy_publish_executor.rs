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

use log::{error, info};
use octocrab::models::repos::Release;
use symlink::{remove_symlink_dir, symlink_dir};
use tokio::fs::remove_dir_all;
use tokio::sync::mpsc::Sender;
use tonic::Status;

use crate::accessor::deployment_accessor::DeploymentAccessor;
use crate::config::{Configuration, DeploymentConfiguration};
use crate::easydep::ExecutedActionEntry;
use crate::executor::script_executor::{execute_scripts, ScriptType};

/// Executes all steps required to publish a deployment (script execution, symlink creation, etc.).
/// Also discords old releases according to the configuration file.
///
/// # Arguments
/// * `release` - The release that is currently being deployed.
/// * `deployment_directory` - The directory in which the deployment is stored.
/// * `global_configuration` - The server configuration.
/// * `deployment_accessor` - The accessor for deployments stored on the disk.
/// * `deployment_configuration` - The deployment profile configuration for the current deployment.
/// * `output_sender` - The sender to which log line output should be sent.
pub async fn publish_deployment(
    release: &Release,
    deployment_directory: &PathBuf,
    global_configuration: &Configuration,
    deployment_accessor: &DeploymentAccessor,
    deployment_configuration: &DeploymentConfiguration,
    output_sender: &Sender<Result<ExecutedActionEntry, Status>>,
) {
    // symlink the "current" directory to the pulled deployed directory
    let published_directory =
        deployment_accessor.get_current_release_directory(deployment_configuration);
    remove_symlink_dir(&published_directory).ok();
    if let Err(err) = symlink_dir(deployment_directory, published_directory) {
        let error_message = format!("unable to symlink release directory: {err}");
        output_sender
            .send(Err(Status::internal(error_message)))
            .await
            .ok();
        return;
    }

    // execute the scripts provided for publishing
    execute_scripts(
        release,
        &ScriptType::Publish,
        deployment_directory,
        deployment_configuration,
        output_sender,
    )
    .await;

    // remove the oldest release if needed
    if global_configuration.retained_releases > 1 {
        discard_oldest_release(
            &global_configuration.retained_releases,
            deployment_accessor,
            deployment_configuration,
        )
        .await;
    }
}

/// Discards the oldest release stored on the disk unless the stored
/// release count is less than the required retained release count.
///
/// # Arguments
/// * `retained_releases` - The number of releases that should be retained.
/// * `deployment_accessor` - The accessor for deployments stored on the disk.
/// * `deployment_configuration` - The deployment profile configuration for the current deployment.
async fn discard_oldest_release(
    retained_releases: &u16,
    deployment_accessor: &DeploymentAccessor,
    deployment_configuration: &DeploymentConfiguration,
) {
    match deployment_accessor
        .get_release_directories_for_profile(deployment_configuration)
        .await
    {
        Ok(release_directories) => {
            if *retained_releases as usize >= release_directories.len() {
                info!("Not removing a release as less releases are stored than retention count");
                return;
            }

            if let Some(oldest_release) = release_directories.last() {
                let (release_directory, release_id) = oldest_release;
                if release_directory.exists() {
                    info!("Removing oldest stored release {release_id}");
                    if let Err(err) = remove_dir_all(release_directory).await {
                        error!("Unable to delete release directory: {err:?}")
                    }
                }
            }
        }
        Err(err) => error!("unable to get oldest release from releases directory: {err:?}"),
    }
}
