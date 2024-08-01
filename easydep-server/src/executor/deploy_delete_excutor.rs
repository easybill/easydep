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

use log::error;
use octocrab::models::repos::Release;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tonic::Status;

use crate::config::DeploymentConfiguration;
use crate::easydep::ExecutedActionEntry;
use crate::executor::script_executor::{execute_scripts, ScriptType};

/// Calls the delete script of the deployment and removes the deployment directory after.
///
/// # Arguments
/// * `release` - The release associated with the deployment.
/// * `deployment_directory` - The directory where the deployment is checked out.
/// * `deployment_configuration` - The deployment profile configuration used for the current deployment.
/// * `output_sender` - The sender to send status information to which will be sent to the client.
pub async fn delete_deployment(
    release: &Release,
    deployment_directory: &PathBuf,
    deployment_configuration: &DeploymentConfiguration,
    output_sender: &Sender<Result<ExecutedActionEntry, Status>>,
) {
    // execute the rollback scripts
    execute_scripts(
        release,
        &ScriptType::Delete,
        deployment_directory,
        deployment_configuration,
        output_sender,
    )
    .await;

    // remove the created directory
    if let Err(err) = fs::remove_dir_all(&deployment_directory).await {
        error!(
            "Unable to delete old deployment directory {:?}: {}",
            deployment_directory, err
        );
    }
}
