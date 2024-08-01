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
use std::hash::{Hash, Hasher};
use std::path::Path;

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::util::input_validator::validate_grpc_endpoint_uri;

/// The root configuration file model.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub(crate) struct Configuration {
    /// The servers that can be used for deployments.
    pub servers: Vec<TargetServer>,
}

/// A target server that can execute deployments.
#[derive(Serialize, Deserialize, Clone, Debug, Eq)]
pub(crate) struct TargetServer {
    /// The id of the server.
    pub id: String,
    /// The address of the server gRPC endpoint.
    pub address: String,
    /// The additional tags of the server, can be used to group them.
    pub tags: Vec<String>,
}

impl Configuration {
    /// Loads the configuration from the given file path, returning an error if the file reading or toml parsing fails.
    ///
    /// # Arguments
    /// * `file_path` - The path to load the configuration from.
    pub async fn load_from_file(file_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let toml_file_content = fs::read_to_string(file_path).await?;
        let parsed_configuration: Configuration = toml::from_str(&toml_file_content)?;
        Ok(parsed_configuration)
    }

    /// Saves the current configuration state into the file at the given path.
    ///
    /// # Arguments
    /// * `file_path` - The path where the configuration should be stored.
    pub async fn save_to_file(&self, file_path: impl AsRef<Path>) -> anyhow::Result<()> {
        let serialized =
            toml::to_string_pretty(&self).context("unable to serialize config to toml")?;
        fs::write(file_path, serialized).await?;
        Ok(())
    }

    /// Validates that the configuration options in this file are all set correctly for the client to function.
    pub fn validate(&self) -> anyhow::Result<()> {
        let mut known_server_ids = HashSet::<&String>::new();
        let mut known_server_addresses = HashSet::<String>::new();
        for server in &self.servers {
            // validate that all server ids are unique
            if !known_server_ids.insert(&server.id) {
                bail!("detected duplicate server id: {}", server.id)
            }

            // validate the endpoint uri & check if it is used twice
            let endpoint_uri = validate_grpc_endpoint_uri(&server.address)?;
            if !known_server_addresses.insert(endpoint_uri.to_string()) {
                bail!("detected duplicate server address: {}", server.address)
            }
        }

        Ok(())
    }

    /// Get a configured server by the given id, returning `None` if no server with the given id is registered.
    ///
    /// # Arguments
    /// * `id` - The id of the server to get.
    pub fn get_server_by_id(&self, id: &String) -> Option<&TargetServer> {
        self.servers.iter().find(|server| server.id.eq(id))
    }

    /// Get all servers that have the given tag configured.
    ///
    /// # Arguments
    /// * `tag` - The tag that the servers must have to be returned.
    pub fn get_servers_with_tag(&self, tag: &String) -> Vec<&TargetServer> {
        self.servers
            .iter()
            .filter(|server| server.tags.contains(tag))
            .collect()
    }
}

/// An implementation for partial eq for the `TargetServer` type which only checks if the id of the server is the same.
impl PartialEq<Self> for TargetServer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// An implementation for hash for the `TargetServer` type which only includes the id into the hash.
impl Hash for TargetServer {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.id.hash(hasher)
    }
}
