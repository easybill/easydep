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

use log::info;
use tonic::transport::Channel;

use crate::config::{Configuration, TargetServer};
use crate::easydep::status_service_client::StatusServiceClient;
use crate::easydep::{DeployCurrentAction, StatusRequest};
use crate::util::server_connector::execute_for_servers;
use crate::util::server_selector::select_target_servers;

/// Displays the status information of the requested servers.
///
/// # Arguments
/// * `configuration` - The client configuration.
/// * `server_ids` - The ids of the servers to display the status of.
pub(crate) async fn display_servers_status(
    configuration: Configuration,
    server_ids: Vec<String>,
) -> anyhow::Result<()> {
    let target_servers = select_target_servers(&configuration, &server_ids)?;
    execute_for_servers(
        target_servers,
        open_status_client_connection,
        |server, mut client| async move {
            let response = client.get_status(StatusRequest {}).await?;
            let response_message = response.get_ref();
            let server_status = DeployCurrentAction::try_from(response_message.current_action)
                .map(|status| match status {
                    DeployCurrentAction::Idle => "idling".to_string(),
                    DeployCurrentAction::Deploying => "deploying".to_string(),
                    DeployCurrentAction::RollingBack => "rolling back".to_string(),
                })
                .unwrap_or_else(|_| "unknown".to_string());

            // display general server information
            info!(
                "[{}] --| Easydep Version              : {}",
                server.id, response_message.version
            );
            info!(
                "[{}] --| Available Deployment Targets : {}",
                server.id,
                response_message.deployment_configurations.join(", ")
            );
            info!(
                "[{}] --| Current Status               : {}",
                server.id, server_status
            );

            // if the release id is supplied the release tag is also present, display both
            if let Some((current_release, current_tag)) = response_message
                .release_id
                .as_ref()
                .zip(response_message.release_tag.as_ref())
            {
                info!(
                    "[{}] --| Working On Release           : {} (id: {})",
                    server.id, current_tag, current_release
                );
            }

            Ok(())
        },
    )
    .await?;
    Ok(())
}

/// Opens a client connection for the status gRPC service to the endpoint of the given target server.
///
/// # Arguments
/// * `server` - The target server to connect to.
async fn open_status_client_connection(
    server: TargetServer,
) -> anyhow::Result<StatusServiceClient<Channel>> {
    StatusServiceClient::connect(server.address.clone())
        .await
        .map_err(Into::into)
}
