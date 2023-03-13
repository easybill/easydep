pub(crate) mod cache;
pub(crate) mod entity;
pub(crate) mod handler;
pub(crate) mod helper;
pub(crate) mod http;

use crate::cache::DeploymentCache;
use crate::entity::deployment::{DeploymentInformation, DeploymentState};
use crate::entity::options::Options;
use crate::handler::cancel_handler::cancel_deployment;
use crate::handler::finish_handler::finish_deployment;
use crate::handler::init_handler::init_deployment;
use crate::helper::process_helper::{pretty_print_output, CommandResult};
use crate::http::auth::handle_auth;
use crate::http::error_handling::HandlerError;
use axum::body::Body;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{middleware, routing, Extension, Router, Server};
use chrono::{TimeZone, Utc};
use clap::Parser;
use entity::requests::{CancelRequest, InitRequest, PublishRequest};
use std::net::SocketAddr;
use std::ops::Add;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    let options = Options::parse();

    let cache_time_seconds = options.release_cache_seconds * 60;
    let deploy_cache = DeploymentCache::new(cache_time_seconds);

    let routing: Router<(), Body> = Router::new()
        .route("/deploy/start", routing::post(handle_deploy_start_request))
        .route(
            "/deploy/publish",
            routing::post(handle_deploy_publish_request),
        )
        .route(
            "/deploy/cancel",
            routing::post(handle_deploy_cancel_request),
        )
        .layer(middleware::from_fn(handle_auth))
        .layer(Extension(options.clone()))
        .layer(Extension(deploy_cache));

    let address = options
        .bind_host
        .parse::<SocketAddr>()
        .expect("Cannot parse provided bind host address");
    Server::bind(&address)
        .serve(routing.into_make_service())
        .await?;

    Ok(())
}

fn interpret_command_results(command_outputs: Vec<CommandResult>) -> (String, bool) {
    let mut process_failed = false;
    let mut emitted_log_lines = Vec::<String>::new();

    for command_output in command_outputs {
        // join the pretty printed command output
        let mut pretty_printed = pretty_print_output(&command_output);
        emitted_log_lines.append(pretty_printed.as_mut());

        // check if the process failed
        let process_exited_successfully = command_output.status.success();
        if !process_exited_successfully {
            process_failed = true;
        }
    }

    let joined_log_lines = emitted_log_lines.join("\n").to_string();
    (joined_log_lines, process_failed)
}

async fn handle_deploy_start_request(
    Extension(options): Extension<Options>,
    Extension(deploy_cache): Extension<DeploymentCache>,
    info: Query<InitRequest>,
) -> anyhow::Result<impl IntoResponse, HandlerError> {
    let request = info.0;

    // ensure that the request is not processed twice
    let information = deploy_cache.read_deployment(&request.release_id)?;
    if information.is_some() {
        return Ok((
            StatusCode::BAD_REQUEST,
            String::from("Deployment with same id already requested"),
        ));
    }

    // construct the deployment information
    let new_information = DeploymentInformation::new(&request, &options);
    let deployment_information =
        deploy_cache.insert_deployment(request.release_id, new_information)?;

    // execute the deployment
    let command_outputs = init_deployment(&options, &deployment_information).await?;

    // move to the next deployment state
    deployment_information.switch_to_requested_state()?;
    deployment_information.set_state(DeploymentState::Publishable)?;

    // interpret the command execution result
    let (joined_output, process_failed) = interpret_command_results(command_outputs);
    if process_failed {
        let full_response = format!(
            "At least one process did not exit successfully. See the log for more details!\n\n{}",
            joined_output
        );
        Ok((StatusCode::INTERNAL_SERVER_ERROR, full_response))
    } else {
        Ok((StatusCode::OK, joined_output))
    }
}

async fn handle_deploy_publish_request(
    Extension(options): Extension<Options>,
    Extension(deploy_cache): Extension<DeploymentCache>,
    info: Query<PublishRequest>,
) -> anyhow::Result<impl IntoResponse, HandlerError> {
    let request = info.0;

    // get the existing request
    let read_result = deploy_cache.read_deployment(&request.release_id)?;
    if read_result.is_none() {
        return Ok((
            StatusCode::BAD_REQUEST,
            String::from("Unknown deployment to finish"),
        ));
    }

    let deploy_information = read_result.unwrap();
    let deployment_state = deploy_information.read_state()?;

    // check if the deployment is in the correct state
    if deployment_state != DeploymentState::Publishable {
        return Ok((
            StatusCode::BAD_REQUEST,
            String::from("The requested deployment is not in the correct state to get published!"),
        ));
    }

    // move the deployment to the linking state & ensure that there are no further requests
    // for state changes before continuing
    deploy_information.switch_to_requested_state()?;
    deploy_information.set_state(DeploymentState::Linking)?;

    // get the base time when the deployment should happen
    let deployment_base_instant = Utc.timestamp_opt(request.base_time, 0).single();
    if deployment_base_instant.is_none() {
        return Ok((
            StatusCode::BAD_REQUEST,
            String::from("Invalid release time base instant"),
        ));
    }

    // get the time until we should sleep
    let sleep_seconds = chrono::Duration::seconds(options.deploy_publish_delay);
    let deployment_base_time = deployment_base_instant.unwrap() + sleep_seconds;

    // get the time that we actually need to sleep
    let sleep_duration = (deployment_base_time - Utc::now()).num_seconds();
    if sleep_duration > 0 {
        sleep(Duration::from_secs(sleep_duration as u64)).await;
    }

    // link the deployment and remove it from the cache
    deploy_cache.remove_deployment(&request.release_id).ok();
    let finish_result = finish_deployment(&options, &deploy_information).await?;

    // pretty print the command result, if present
    match finish_result {
        Some(result) => {
            let pretty_printed_result = pretty_print_output(&result);
            Ok((StatusCode::OK, pretty_printed_result.join("\n").to_string()))
        }
        None => Ok((
            StatusCode::OK,
            String::from("Deployment finish completed successfully"),
        )),
    }
}

async fn handle_deploy_cancel_request(
    Extension(deploy_cache): Extension<DeploymentCache>,
    info: Query<CancelRequest>,
) -> anyhow::Result<impl IntoResponse, HandlerError> {
    let request = info.0;

    // get the existing request
    let read_result = deploy_cache.read_deployment(&request.release_id)?;
    if read_result.is_none() {
        return Ok((
            StatusCode::BAD_REQUEST,
            String::from("Unknown deployment to cancel"),
        ));
    }

    let deploy_information = read_result.unwrap();

    // check if the deployment is in an invalid state
    deploy_information.switch_to_requested_state()?;
    let deployment_state = deploy_information.read_state()?;

    if deployment_state >= DeploymentState::Linking {
        return Ok((
            StatusCode::BAD_REQUEST,
            format!(
                "Deployment is in invalid state to get cancelled: {:?}",
                deployment_state
            ),
        ));
    }

    // request the movement to the cancelled state (if needed)
    if deployment_state != DeploymentState::Publishable {
        deploy_information.set_requested_state(DeploymentState::Cancelled)?;

        // wait for the deployment to get cancelled
        // we sleep 5 seconds during each check,
        let wait_timeout = Instant::now().add(Duration::from_secs(5 * 60));
        while wait_timeout > Instant::now() {
            // wait for the state to be moved to cancelled
            let state = deploy_information.read_state()?;
            if state == DeploymentState::Cancelled {
                break;
            }

            // sleep a little, check again
            sleep(Duration::from_secs(5)).await;
        }
    } else {
        // mark the deployment as cancelled
        deploy_information.set_state(DeploymentState::Cancelled)?;
    }

    // cancel the deployment & remove it from the cache
    deploy_cache.remove_deployment(&request.release_id).ok();
    cancel_deployment(&deploy_information).await?;

    // pretty print the emitted log lines & return them
    Ok((
        StatusCode::OK,
        String::from("Deployment cancelled successfully"),
    ))
}
