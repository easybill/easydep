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
use std::path::PathBuf;

use anyhow::bail;
use log::info;

use crate::config::{Configuration, TargetServer};
use crate::util::input_validator::validate_grpc_endpoint_uri;

/// Prints the servers that are registered in the client configuration into the console.
///
/// # Arguments
/// * `configuration` - The current client configuration.
pub(crate) fn display_configured_servers(configuration: Configuration) {
    for target_server in configuration.servers {
        if target_server.tags.is_empty() {
            info!("--| {}: ip: {}", target_server.id, target_server.address);
        } else {
            info!(
                "--| {}: ip: {}, tags: {}",
                target_server.id,
                target_server.address,
                target_server.tags.join(", ")
            );
        }
    }
}

/// Adds a new server with the given properties into the configuration. If a server with the same id or address is
/// already registered an error is returned. The given tags will be deduplicated.
///
/// # Arguments
/// * `configuration` - The current client configuration.
/// * `config_path` - The path from where the configuration is loaded.
/// * `server_id` - The given id of the server to register.
/// * `server_address` - The gRPC endpoint address of the server to register.
/// * `tags` - The tags of the server to register.
pub(crate) async fn add_server_to_config(
    mut configuration: Configuration,
    config_path: PathBuf,
    server_id: String,
    server_address: String,
    tags: Vec<String>,
) -> anyhow::Result<()> {
    // check if the id is already taken
    let server_id = server_id.trim().to_string();
    if configuration.get_server_by_id(&server_id).is_some() {
        bail!("server id {} is already taken", server_id)
    }

    // check if the server address is already in use
    let server_address = validate_grpc_endpoint_uri(&server_address)?.to_string();
    let server_with_address = configuration
        .servers
        .iter()
        .find(|server| server.address == server_address);
    if server_with_address.is_some() {
        bail!("server address {} is already taken", server_address)
    }

    // deduplicate the tags and register the server into the configuration file
    let tags: HashSet<String> = tags
        .into_iter()
        .filter(|tag| !tag.trim().is_empty())
        .collect();
    let new_server = TargetServer {
        id: server_id,
        address: server_address,
        tags: Vec::from_iter(tags),
    };
    configuration.servers.push(new_server);
    configuration.save_to_file(config_path).await?;
    info!("Successfully added new server into configuration");

    Ok(())
}

/// Removes a server from the configuration, returning an error if no server with the id is registered.
///
/// # Arguments
/// * `configuration` - The current client configuration.
/// * `config_path` - The path from where the configuration is loaded.
/// * `server_id` - The given id of the server to unregister.
pub(crate) async fn remove_server_from_config(
    mut configuration: Configuration,
    config_path: PathBuf,
    server_id: String,
) -> anyhow::Result<()> {
    // check if the given server is registered
    if configuration.get_server_by_id(&server_id).is_none() {
        bail!("no server with id {} is registered", server_id)
    }

    // server is in configuration, remove it
    let new_servers = configuration
        .servers
        .into_iter()
        .filter(|server| server.id != server_id)
        .collect();
    configuration.servers = new_servers;
    configuration.save_to_file(config_path).await?;
    info!("Successfully removed server from configuration");

    Ok(())
}
