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

use std::path::{Path, PathBuf};
use std::process::Stdio;

use log::error;
use octocrab::models::repos::Release;
use secrecy::{ExposeSecret, SecretString};
use symlink::{remove_symlink_auto, symlink_auto};
use tokio::fs;
use tokio::process::Command;
use tokio::sync::mpsc::Sender;
use tonic::Status;

use crate::config::DeploymentConfiguration;
use crate::easydep::{Action, ActionStatus, ExecutedActionEntry, LogEntry, LogType};
use crate::executor::script_executor::{execute_scripts, ScriptType};
use crate::process_streamer::ProcessStreamer;

/// Initializes a deployment. This includes steps like git checkout, script execution etc.
///
/// # Arguments
/// * `release` - The release that is currently being deployed.
/// * `deployment_directory` - The directory in which the deployment is stored.
/// * `github_access_token` - The access token for git https operations on GitHub.
/// * `deployment_configuration` - The deployment profile configuration for the current deployment.
/// * `output_sender` - The sender to which log line output should be sent.
pub async fn init_deployment(
    release: &Release,
    deployment_directory: &PathBuf,
    github_access_token: &SecretString,
    deployment_configuration: &DeploymentConfiguration,
    output_sender: &Sender<Result<ExecutedActionEntry, Status>>,
) {
    // get the directory into which the deployment should be executed and
    // check if the directory already exists (prevent duplicate execution)
    match fs::try_exists(&deployment_directory).await {
        Ok(directory_existence) => {
            if directory_existence {
                // directory already exists -> deployment was already executed from elsewhere
                output_sender.send(Err(Status::failed_precondition("deployment directory already exists, deployment was likely triggered already"))).await.ok();
                return;
            }
        }
        Err(err) => {
            // unable to stat existence of directory, return this as an error
            let error_message = format!(
                "unable to stat existence of deployment directory {:?}: {err}",
                &deployment_directory
            );
            output_sender
                .send(Err(Status::internal(error_message)))
                .await
                .ok();
            return;
        }
    }

    // execute the git clone command
    let repository_url = format!(
        "https://x-access-token:{github_access_token}@github.com/{repo_owner}/{repo_name}.git",
        github_access_token = github_access_token.expose_secret(),
        repo_owner = deployment_configuration.source_repo_owner,
        repo_name = deployment_configuration.source_repo_name
    );
    match Command::new("git")
        .arg("clone")
        // we check out a single commit resulting in a detached head state, suppress the resulting warning
        .arg("-c")
        .arg("advice.detachedHead=false")
        // skip downloading the full history
        .arg("--depth")
        .arg("1")
        // clone the tag that is associated with the release
        .arg("--branch")
        .arg(&release.tag_name)
        // clone from the repo url with access & directly into the deployment folder
        .arg(repository_url)
        .arg(deployment_directory)
        // redirect streams to current application
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(git_clone_process) => {
            let mut clone_process_streamer = ProcessStreamer::new(
                Action::GitClone,
                release.id.0,
                git_clone_process,
                output_sender.clone(),
            );
            if let Err(err) = clone_process_streamer.await_child_and_stream().await {
                let error_message =
                    format!("issue while waiting for git clone process to complete: {err}");
                output_sender
                    .send(Err(Status::internal(error_message)))
                    .await
                    .ok();
                return;
            }
        }
        Err(err) => {
            let error_message = format!("issue while spawning git clone process: {err}");
            output_sender
                .send(Err(Status::internal(error_message)))
                .await
                .ok();
            return;
        }
    }

    // write the checked-out revision into a file, if specified in the deployment configuration
    if let Some(revision_file_path) = &deployment_configuration.revision_file_name {
        match Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .current_dir(deployment_directory)
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                // successfully fetched current git head
                let rev_file_path = deployment_directory.join(revision_file_path);
                if let Err(err) = fs::write(&rev_file_path, output.stdout).await {
                    error!(
                        "Unable to write revision file to {:?}: {}",
                        rev_file_path, err
                    );
                }
            }
            Ok(output) => {
                // the command did not complete with a successful status code
                let stderr_output = String::from_utf8_lossy(output.stderr.as_slice());
                let error_message = format!("unable to parse head-ref: {stderr_output}");
                output_sender
                    .send(Err(Status::internal(error_message)))
                    .await
                    .ok();
                return;
            }
            Err(err) => {
                // some error occurred while spawning the command
                let error_message = format!("unable to parse head-ref: {err}");
                output_sender
                    .send(Err(Status::internal(error_message)))
                    .await
                    .ok();
                return;
            }
        }
    }

    // create the requested additional symlinks
    let symlinks = deployment_configuration.get_symlinks();
    for symlink in symlinks {
        let source_path = format!(
            "{deploy_directory:?}/{symlink_source}",
            deploy_directory = &deployment_directory,
            symlink_source = symlink.source,
        );
        output_sender
            .send(Ok(ExecutedActionEntry {
                release_id: release.id.0,
                current_action: i32::from(Action::SymlinkCreate),
                action_status: i32::from(ActionStatus::Running),
                action_log_entry: Some(LogEntry {
                    stream_type: i32::from(LogType::Stdout),
                    content: format!("creating symlink {} -> {}", source_path, symlink.target),
                }),
            }))
            .await
            .ok();

        // create the parent directory of the symlink source if it does not exist already
        // this is required to actually create the symlink when the path is nested
        let source_path = Path::new(source_path.as_str());
        if let Some(parent) = source_path.parent() {
            fs::create_dir_all(parent).await.ok();
        }

        // create the symlink between the source path in the deployment folder and the external target folder
        let target_path = Path::new(symlink.target.as_str());
        remove_symlink_auto(source_path).ok();
        if let Err(err) = symlink_auto(target_path, source_path) {
            error!(
                "Unable to symlink {:?} -> {:?}: {}",
                target_path, source_path, err
            );
        }
    }

    // execute the init scripts
    execute_scripts(
        release,
        &ScriptType::Init,
        deployment_directory,
        deployment_configuration,
        output_sender,
    )
    .await;
}
