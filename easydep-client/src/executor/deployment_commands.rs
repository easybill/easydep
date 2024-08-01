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

use anyhow::{anyhow, bail};
use futures::StreamExt;
use log::{error, info, warn};
use prost::UnknownEnumValue;
use tonic::transport::Channel;
use tonic::Streaming;

use crate::config::{Configuration, TargetServer};
use crate::easydep::deployment_service_client::DeploymentServiceClient;
use crate::easydep::{
    Action, ActionStatus, DeployDeleteRequest, DeployPublishRequest, DeployRollbackRequest,
    DeployStartRequest, DeployStatusRequest, ExecutedActionEntry, LogType,
};
use crate::util::server_connector::execute_for_servers;
use crate::util::server_selector::select_target_servers;

/// Displays the deployment status of the given release profile on the requested servers.
///
/// # Arguments
/// * `configuration` - The client configuration.
/// * `profile` - The profile to get the deployment status of.
/// * `server_ids` - The ids of the servers to get the deployment status from.
pub(crate) async fn display_servers_deployment_status(
    configuration: Configuration,
    profile: String,
    server_ids: Vec<String>,
) -> anyhow::Result<()> {
    let target_servers = select_target_servers(&configuration, &server_ids)?;
    execute_for_servers(
        target_servers,
        open_deployment_client_connection,
        move |server, mut client| {
            let profile = profile.clone();
            async move {
                let request = DeployStatusRequest { profile };
                let response = client.get_deployment_status(request).await?;
                let response_message = response.get_ref();
                info!(
                    "[{}] --| Status for profile   : {}",
                    server.id, response_message.profile
                );
                info!(
                    "[{}] --| Deployed Release     : {} (id: {})",
                    server.id, response_message.tag_name, response_message.release_id
                );
                info!(
                    "[{}] --| Release Created From : {}",
                    server.id, response_message.target_commit
                );
                Ok(())
            }
        },
    )
    .await?;
    Ok(())
}

/// Starts the deployment process for the given release with the given profile on the given target servers. This method
/// returns an error result if one of the execution fails, and consolidates multiple errors into a single one.
///
/// # Arguments
/// * `configuration` - The client configuration.
/// * `profile` - The name of the profile to use for the deployment.
/// * `release_id` - The id of the release to deploy.
/// * `server_ids` - The ids of the servers to start the deployment process on.
pub(crate) async fn start_deployment_on_servers(
    configuration: Configuration,
    profile: String,
    release_id: u64,
    server_ids: Vec<String>,
) -> anyhow::Result<()> {
    let target_servers = select_target_servers(&configuration, &server_ids)?;
    execute_for_servers(
        target_servers,
        open_deployment_client_connection,
        move |server, mut client| {
            let profile = profile.clone();
            async move {
                let request = DeployStartRequest {
                    profile,
                    release_id,
                };
                let response_stream = client.start_deployment(request).await?.into_inner();
                stream_executed_actions(server, response_stream).await
            }
        },
    )
    .await?;
    Ok(())
}

/// Publishes a previously started deployment on the requested servers.
///
/// # Arguments
/// * `configuration` - The client configuration.
/// * `release_id` - The id of the release that should get published.
/// * `server_ids` - The ids of the servers to publish the deployment on.
pub(crate) async fn publish_deployment_on_servers(
    configuration: Configuration,
    release_id: u64,
    server_ids: Vec<String>,
) -> anyhow::Result<()> {
    let target_servers = select_target_servers(&configuration, &server_ids)?;
    execute_for_servers(
        target_servers,
        open_deployment_client_connection,
        move |server, mut client| async move {
            let request = DeployPublishRequest { release_id };
            let response_stream = client.publish_deployment(request).await?.into_inner();
            stream_executed_actions(server, response_stream).await
        },
    )
    .await?;
    Ok(())
}

/// Requests to roll back to the previous deployment of the given profile on the given target servers.
///
/// # Arguments
/// * `configuration` - The client configuration.
/// * `profile` - The release profile of which the rollback to the previous release should happen.
/// * `server_ids` - The ids of the servers to roll back to the previous deployment on.
pub(crate) async fn rollback_deployment_on_servers(
    configuration: Configuration,
    profile: String,
    server_ids: Vec<String>,
) -> anyhow::Result<()> {
    let target_servers = select_target_servers(&configuration, &server_ids)?;
    execute_for_servers(
        target_servers,
        open_deployment_client_connection,
        move |server, mut client| {
            let profile = profile.clone();
            async move {
                let request = DeployRollbackRequest { profile };
                let response_stream = client.rollback_deployment(request).await?.into_inner();
                stream_executed_actions(server, response_stream).await
            }
        },
    )
    .await?;
    Ok(())
}

/// Deletes a deployment that wasn't published before on the given target servers.
///
/// # Arguments
/// * `configuration` - The client configuration.
/// * `release_id` - The id of the release that should be deleted.
/// * `server_ids` - The ids of the servers on which the deployment should be deleted.
pub(crate) async fn delete_unpublished_deployment_on_servers(
    configuration: Configuration,
    release_id: u64,
    server_ids: Vec<String>,
) -> anyhow::Result<()> {
    let target_servers = select_target_servers(&configuration, &server_ids)?;
    execute_for_servers(
        target_servers,
        open_deployment_client_connection,
        move |server, mut client| async move {
            let request = DeployDeleteRequest { release_id };
            let response_stream = client
                .delete_unpublished_deployment(request)
                .await?
                .into_inner();
            stream_executed_actions(server, response_stream).await
        },
    )
    .await?;
    Ok(())
}

/// Opens a client connection for the deployment gRPC service to the endpoint of the given target server.
///
/// # Arguments
/// * `server` - The target server to connect to.
async fn open_deployment_client_connection(
    server: TargetServer,
) -> anyhow::Result<DeploymentServiceClient<Channel>> {
    DeploymentServiceClient::connect(server.address.clone())
        .await
        .map_err(Into::into)
}

/// Streams the executed action entries returned by the provided stream into the console until the stream finished
/// (which means that the remote server closed the connection). This means that script execution lines are logged into
/// the console and some information about the current lifecycle state.
///
/// # Arguments
/// * `server` - The server of which the output is streamed into the console.
/// * `stream` - The data stream containing the executed action entries coming from the server.
///
/// # Returns
/// * `anyhow::Result<()>` - `Ok` if the execution completed successfully on the remote, `Err` if some error occurred.
async fn stream_executed_actions(
    server: TargetServer,
    mut stream: Streaming<ExecutedActionEntry>,
) -> anyhow::Result<()> {
    let mut encountered_failed_script = false;
    while let Some(data) = stream.next().await {
        match data {
            Ok(action_entry) => {
                // print the log line, if present
                if let Some(log_entry) = action_entry.action_log_entry {
                    let current_action =
                        format_action_name(Action::try_from(action_entry.current_action));
                    let log_stream =
                        LogType::try_from(log_entry.stream_type).unwrap_or(LogType::Stdout);
                    match log_stream {
                        LogType::Stdout => info!(
                            "[{} @ {}] --| {}",
                            server.id, current_action, log_entry.content
                        ),
                        LogType::Stderr => warn!(
                            "[{} @ {}] --| {}",
                            server.id, current_action, log_entry.content
                        ),
                    }
                }

                // display information about the current action status
                if let Ok(action_status) = ActionStatus::try_from(action_entry.action_status) {
                    match action_status {
                        ActionStatus::Started => {
                            info!("[{}] --| Script Execution Started", server.id);
                        }
                        ActionStatus::CompletedSuccess => {
                            info!(
                                "[{}] --| Script Execution Completed Successfully",
                                server.id
                            );
                        }
                        ActionStatus::CompletedFailure => {
                            error!("[{}] --| Script Execution Failed", server.id);
                            encountered_failed_script = true;
                        }
                        ActionStatus::Running => {}
                    }
                }
            }
            Err(status) => bail!(
                "[{}] Server returned status {}: {}",
                server.id,
                status.code(),
                status.message()
            ),
        };
    }

    // consider this step as failed if one script failed
    if encountered_failed_script {
        Err(anyhow!(
            "Encountered at least one script on {} that did not complete successfully",
            server.id
        ))
    } else {
        Ok(())
    }
}

/// Formats the action in the given Result if Ok, returning a descriptor of the missing enum vale if Err.
///
/// # Arguments
/// * `maybe_action` - The Result either containing the action to format or an error indicating a missing enum value.
///
/// # Returns
/// * `String` - a human-readable for of the given action if `Ok`, the missing index of the enum value if `Err`.
fn format_action_name(maybe_action: Result<Action, UnknownEnumValue>) -> String {
    match maybe_action {
        Ok(action) => match action {
            Action::GitClone => "Git Clone".to_string(),
            Action::SymlinkCreate => "Symlinking".to_string(),
            Action::InitScript => "Init Script".to_string(),
            Action::FinishScript => "Finish Script".to_string(),
            Action::DeleteScript => "Delete Script".to_string(),
        },
        Err(action) => format!("{}", action),
    }
}
