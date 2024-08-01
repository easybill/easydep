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
use std::process::Stdio;

use anyhow::bail;
use octocrab::models::repos::Release;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::mpsc::Sender;
use tonic::Status;

use crate::config::DeploymentConfiguration;
use crate::easydep::{Action, ExecutedActionEntry};
use crate::process_streamer::ProcessStreamer;

/// The type of scripts that can be executed.
pub(crate) enum ScriptType {
    /// The script executed when initializing a deployment.
    Init,
    /// The script executed when publishing a deployment.
    Publish,
    /// The script executed when deleting a deployment.
    Delete,
}

/// Executes the given scripts for the given release profile.
/// This includes the scripts that are coming from extended configurations.
///
/// # Arguments
/// * `release` - The release that is currently being deployed.
/// * `script_type` - The type of scripts to execute.
/// * `deployment_directory` - The directory in which the deployment is stored.
/// * `deployment_configuration` - The deployment profile configuration for the current deployment.
/// * `output_sender` - The sender to which log line output should be sent.
pub async fn execute_scripts(
    release: &Release,
    script_type: &ScriptType,
    deployment_directory: &PathBuf,
    deployment_configuration: &DeploymentConfiguration,
    output_sender: &Sender<Result<ExecutedActionEntry, Status>>,
) {
    let (script_action, script_action_name) = match script_type {
        ScriptType::Init => (Action::InitScript, "init".to_string()),
        ScriptType::Publish => (Action::FinishScript, "publish".to_string()),
        ScriptType::Delete => (Action::DeleteScript, "delete".to_string()),
    };

    // execute the extended scripts first
    let extended_configurations = &deployment_configuration.extended_script_configurations;
    for extended_configuration in extended_configurations {
        let script_path = get_script_path(extended_configuration, &script_action_name);
        if check_and_execute_script(
            release,
            &script_path,
            &script_action,
            deployment_directory,
            output_sender,
        )
        .await
        .is_err()
        {
            return;
        }
    }

    // execute the main script
    let main_script_path = get_script_path(&deployment_configuration.id, &script_action_name);
    check_and_execute_script(
        release,
        &main_script_path,
        &script_action,
        deployment_directory,
        output_sender,
    )
    .await
    .ok();
}

/// Checks if the script at the given file path exists and executes it if that is the case.
///
/// # Arguments
/// * `release` - The release that is currently being deployed.
/// * `script_path` - The path where the script file should be located.
/// * `script_action` - The script action that is represented by the script.
/// * `deployment_directory` - The directory in which the deployment is stored.
/// * `output_sender` - The sender to which log line output should be sent.
async fn check_and_execute_script(
    release: &Release,
    script_path: &String,
    script_action: &Action,
    deployment_directory: &PathBuf,
    output_sender: &Sender<Result<ExecutedActionEntry, Status>>,
) -> anyhow::Result<()> {
    let full_script_path = deployment_directory.join(script_path);
    if let Ok(script_file_exists) = fs::try_exists(full_script_path).await {
        if script_file_exists {
            if let Err(err) = execute_script(
                release,
                script_path,
                script_action,
                deployment_directory,
                output_sender,
            )
            .await
            {
                let error_message = format!("unable to execute script at {script_path:?}: {err}");
                output_sender
                    .send(Err(Status::internal(error_message)))
                    .await
                    .ok();
                bail!("issue executing script")
            }
        }
    }
    Ok(())
}

/// Executes a script. This method assumes that the script file exists. `bash` is used to execute the script.
///
/// # Arguments
/// * `release` - The release that is currently being deployed.
/// * `script_path` - The path where the script file should be located.
/// * `script_action` - The script action that is represented by the script.
/// * `deployment_directory` - The directory in which the deployment is stored.
/// * `output_sender` - The sender to which log line output should be sent.
async fn execute_script(
    release: &Release,
    script_path: &String,
    script_action: &Action,
    deployment_directory: &PathBuf,
    output_sender: &Sender<Result<ExecutedActionEntry, Status>>,
) -> anyhow::Result<()> {
    match Command::new("bash")
        .arg(script_path)
        .current_dir(deployment_directory)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(script_process) => {
            let mut process_streamer = ProcessStreamer::new(
                *script_action,
                release.id.0,
                script_process,
                output_sender.clone(),
            );
            if let Err(err) = process_streamer.await_child_and_stream().await {
                let error_message = format!("issue while waiting for script to complete: {err}");
                output_sender
                    .send(Err(Status::internal(error_message)))
                    .await
                    .ok();
                Err(err)
            } else {
                Ok(())
            }
        }
        Err(err) => {
            let error_message =
                format!("unable to spawn process to execute lifecycle script: {err}");
            output_sender
                .send(Err(Status::internal(error_message)))
                .await
                .ok();
            Err(err.into())
        }
    }
}

fn get_script_path(script_configuration: &String, script_action_name: &String) -> String {
    format!(
        ".easydep/{}/{}.sh",
        script_configuration, script_action_name
    )
}
