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
use anyhow::Context;
use clap::Parser;
use env_logger::Env;
use log::{error, info};
use std::process::exit;

use crate::cli::{Cli, ConfigCommands, DeployCommands, RootCommands};
use crate::config::Configuration;
use crate::executor::config_commands::{
    add_server_to_config, display_configured_servers, remove_server_from_config,
};
use crate::executor::deployment_commands::{
    delete_unpublished_deployment_on_servers, display_servers_deployment_status,
    publish_deployment_on_servers, rollback_deployment_on_servers, start_deployment_on_servers,
};
use crate::executor::status_commands::display_servers_status;

mod cli;
pub(crate) mod config;
pub(crate) mod executor;
pub(crate) mod util;

const GIT_SHA: &str = env!("GIT_HASH");
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) mod easydep {
    tonic::include_proto!("easydep");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initializes the logger, using the "info" level if the RUST_LOG environment variable isn't set
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_module_path(false)
        .format_target(false)
        .format_timestamp_secs()
        .try_init()
        .context("unable to initialize logging")?;
    info!(
        "Running easydep version {} (git commit {})",
        VERSION, GIT_SHA
    );

    // load & validate the configuration from the specified file path, create it if it does not exist yet
    let cli = Cli::parse();
    let configuration = if cli.configuration_path.exists() {
        let configuration = Configuration::load_from_file(&cli.configuration_path).await?;
        configuration.validate()?;
        info!(
            "Loaded configuration with {} target servers",
            configuration.servers.len()
        );
        configuration
    } else {
        info!("Creating and storing new configuration...");
        let configuration = Configuration::default();
        configuration.save_to_file(&cli.configuration_path).await?;
        configuration
    };

    // execute the requested command and display the error message if an error occurred
    let command_execution_result = match cli.command {
        RootCommands::Config { action } => match action {
            ConfigCommands::List => {
                display_configured_servers(configuration);
                Ok(())
            }
            ConfigCommands::Add {
                server_id,
                server_host,
                server_tags,
            } => {
                add_server_to_config(
                    configuration,
                    cli.configuration_path,
                    server_id,
                    server_host,
                    server_tags,
                )
                .await
            }
            ConfigCommands::Remove { server_id } => {
                remove_server_from_config(configuration, cli.configuration_path, server_id).await
            }
        },
        RootCommands::Status { server_ids } => {
            display_servers_status(configuration, server_ids).await
        }
        RootCommands::Deploy { action } => match action {
            DeployCommands::Status {
                profile,
                server_ids,
            } => display_servers_deployment_status(configuration, profile, server_ids).await,
            DeployCommands::Start {
                profile,
                release_id,
                server_ids,
            } => start_deployment_on_servers(configuration, profile, release_id, server_ids).await,
            DeployCommands::Publish {
                release_id,
                server_ids,
            } => publish_deployment_on_servers(configuration, release_id, server_ids).await,
            DeployCommands::Rollback {
                profile,
                server_ids,
            } => rollback_deployment_on_servers(configuration, profile, server_ids).await,
            DeployCommands::Delete {
                release_id,
                server_ids,
            } => {
                delete_unpublished_deployment_on_servers(configuration, release_id, server_ids)
                    .await
            }
        },
    };
    if let Err(err) = command_execution_result {
        error!("Issue occurred while executing requested command: {}", err);
        exit(1)
    }

    Ok(())
}
