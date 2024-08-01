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

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// The CLI interface of easyde
#[derive(Parser, Debug, Clone)]
#[command(disable_version_flag = true)]
pub(crate) struct Cli {
    /// The command that was executed.
    #[command(subcommand)]
    pub command: RootCommands,
    /// The path where the client configuration file is located.
    #[arg(short = 'c', long = "config-path", env = "EASYDEP_CONFIG_PATH")]
    pub configuration_path: PathBuf,
}

/// Holds the collection of top-level commands.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum RootCommands {
    /// Manages the client configuration.
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Access to the status of registered server(s).tus.
    Status {
        /// The ids of the server(s) to get the status of. If empty the status of all servers will be displayed.
        server_ids: Vec<String>,
    },
    /// Manages deployments on the remote servers.
    Deploy {
        #[command(subcommand)]
        action: DeployCommands,
    },
}

/// The subcommand to manage the client configuration file.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum ConfigCommands {
    /// Lists the servers that are registered in the configuration.
    List,
    /// Adds a new server to the configuration.
    Add {
        /// The id of the server.
        server_id: String,
        /// The host and port of the server gRPC endpoint.
        server_host: String,
        /// The tags to add for the server, these can be used to easily deploy to a group of servers later.
        server_tags: Vec<String>,
    },
    /// Removes a server from the configuration.
    Remove {
        /// The id of the server to remove from the configuration.
        server_id: String,
    },
}

/// The subcommand to manage deployments on one or multiple servers.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum DeployCommands {
    /// Get the deployment status on the given server(s).
    Status {
        /// The profile to get the deployment status of.
        profile: String,
        /// The server(s) to retrieve the information from. If empty all servers will be displayed.
        server_ids: Vec<String>,
    },
    /// Starts the deployment process for the given release using the given profile.
    Start {
        /// The profile to use to execute the deployment.
        profile: String,
        /// The id of the release that should be deployed.
        release_id: u64,
        /// The server(s) to execute the deployment on. If empty it will be deployed on all servers.
        server_ids: Vec<String>,
    },
    /// Publishes a previously started deployment.
    Publish {
        /// The id of the release that should be published.
        release_id: u64,
        /// The server(s) to publish the deployment on. If empty it will be published on all servers.
        server_ids: Vec<String>,
    },
    /// Deletes a started but not yet published deployment from the given server(s).
    Delete {
        /// The id of the release to delete.
        release_id: u64,
        /// The server(s) to delete the deployment on. If empty it will be deleted on all servers.
        server_ids: Vec<String>,
    },
    /// Rolls back to the previous deployment of the given profile on the given target server(s).
    Rollback {
        /// The profile to roll the deployment back of.
        profile: String,
        /// The server(s) to roll back the deployment on. If empty it will be rolled back on all servers.
        server_ids: Vec<String>,
    },
}
