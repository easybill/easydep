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
use std::future::IntoFuture;
use std::net::SocketAddr;
use std::process::exit;

use anyhow::Context;
use clap::Parser;
use env_logger::Env;
use log::{error, info};
use tonic::transport::Server;

use crate::accessor::deploy_action_accessor::DeploymentStatusAccessor;
use crate::accessor::github_accessor::GitHubAccessor;
use crate::config::Configuration;
use crate::easydep::deployment_service_server::DeploymentServiceServer;
use crate::easydep::status_service_server::StatusServiceServer;
use crate::service::deployment_service::DeploymentServiceImpl;
use crate::service::status_service::StatusServiceImpl;

mod accessor;
mod config;
mod executor;
mod process_streamer;
mod service;

const GIT_SHA: &str = env!("GIT_HASH");
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) mod easydep {
    tonic::include_proto!("easydep");
}

/// The command line options model.
#[derive(Parser, Clone, Debug)]
struct CommandLineOptions {
    /// The path were the main configuration file is located.
    #[arg(long = "config-path", env = "EASYDEP_CONFIG_PATH")]
    pub configuration_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initializes the logger, using the "info" level if the RUST_LOG environment variable isn't set
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .try_init()
        .context("unable to initialize logging")?;
    info!(
        "Running easydep version {} (git commit {})",
        VERSION, GIT_SHA
    );

    info!("Loading configuration...");
    let command_line_options = CommandLineOptions::parse();
    let configuration = Configuration::load_from_file(&command_line_options.configuration_path)
        .await
        .context("couldn't parse configuration file")?;
    configuration
        .validate()
        .await
        .context("issue detected while validating configuration")?;
    let bind_address = configuration
        .bind_host
        .parse::<SocketAddr>()
        .context("couldn't parse provided host address")?;

    let version_string = format!("{}+{}", VERSION, GIT_SHA);
    let deployment_configurations = configuration.get_deployment_configuration_ids();
    let deploy_status_accessor = DeploymentStatusAccessor::new();
    let status_service = StatusServiceImpl::new(
        version_string,
        deployment_configurations,
        deploy_status_accessor.clone(),
    );

    info!("Preparing GitHub api client...");
    let github_accessor = GitHubAccessor::new(&configuration)
        .await
        .context("couldn't initialize GitHub client")?;
    let deployment_service =
        DeploymentServiceImpl::new(configuration, github_accessor, deploy_status_accessor).await;

    info!("Binding gRPC server to {}...", bind_address);
    let tonic_serve_future = Server::builder()
        .add_service(StatusServiceServer::new(status_service))
        .add_service(DeploymentServiceServer::new(deployment_service))
        .serve(bind_address)
        .into_future();
    let exit_code = tokio::select! {
        _ = tonic_serve_future => {
            error!("Tonic server http endpoint failed");
            100
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Quit signal received, exiting!");
            0
        }
    };
    exit(exit_code)
}
