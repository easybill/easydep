use std::fmt::Debug;
use std::path::PathBuf;
use std::process::Command;

use crate::entity::options::Options;
use crate::helper::process_helper::{run_command, CommandResult, CommandResultCollection};

pub(crate) mod cancel_handler;
pub(crate) mod finish_handler;
pub(crate) mod github;
pub(crate) mod init_handler;
pub(crate) mod initial_handler;
pub(crate) mod release_discard;

#[derive(PartialEq, Debug, Clone)]
pub(crate) enum LifecycleState {
    Success,
    Failure,
}

impl LifecycleState {
    pub fn from_result<T, E>(result: &Result<T, E>) -> Self {
        if result.is_ok() {
            LifecycleState::Success
        } else {
            LifecycleState::Failure
        }
    }
}

pub(crate) async fn call_and_aggregate_command(
    command: Command,
    results: &mut Vec<CommandResult>,
) -> anyhow::Result<bool> {
    let command_result = run_command(command).await?;
    let exit_status = command_result.status;
    results.push(command_result);
    Ok(exit_status.success())
}

pub(crate) async fn call_followup_lifecycle_script<T: Debug>(
    options: &Options,
    deploy_base_directory: &PathBuf,
    lifecycle_event_name: &str,
    previous_result: anyhow::Result<T, anyhow::Error>,
) -> anyhow::Result<Option<CommandResult>, anyhow::Error> {
    let state = LifecycleState::from_result(&previous_result);
    let command_result =
        call_lifecycle_script(options, deploy_base_directory, lifecycle_event_name, state).await?;

    previous_result?;
    Ok(command_result)
}

pub(crate) async fn call_and_aggregate_lifecycle_script(
    options: &Options,
    deploy_base_directory: &PathBuf,
    lifecycle_event_name: &str,
    previous_result: Result<CommandResultCollection, anyhow::Error>,
) -> anyhow::Result<CommandResultCollection, anyhow::Error> {
    let state = match &previous_result {
        Ok(result) => {
            if result.failed_command {
                LifecycleState::Failure
            } else {
                LifecycleState::Success
            }
        }
        Err(_) => LifecycleState::Failure,
    };
    let command_result =
        call_lifecycle_script(options, deploy_base_directory, lifecycle_event_name, state).await?;

    // return the previous result if there was an error
    #[allow(clippy::question_mark)]
    if previous_result.is_err() {
        return previous_result;
    }

    // get the output vec from the previous input & aggregate it with the new command output
    let mut result_collection = previous_result.unwrap();
    match command_result {
        Some(result) => {
            let success = result.status.success();
            result_collection.results.push(result);
            Ok(CommandResultCollection {
                results: result_collection.results,
                failed_command: result_collection.failed_command || !success,
            })
        }
        None => Ok(result_collection),
    }
}

pub(crate) async fn call_lifecycle_script(
    options: &Options,
    deploy_base_directory: &PathBuf,
    lifecycle_event_name: &str,
    state: LifecycleState,
) -> anyhow::Result<Option<CommandResult>, anyhow::Error> {
    // resolve the target script path
    let script_dir = format!(".easydep{}", options.environment_suffix());
    let script_name = format!("{}_{:?}.sh", lifecycle_event_name, state).to_lowercase();
    let script_path = deploy_base_directory.join(&script_dir).join(&script_name);

    // run the script if it exists
    if script_path.exists() {
        let script_name_for_command = format!("{}/{}", script_dir, script_name);
        let mut script_command = Command::new("bash");
        script_command
            .arg(script_name_for_command)
            .current_dir(deploy_base_directory);

        // run the command and return the result
        let command_result = run_command(script_command).await?;
        Ok(Some(command_result))
    } else {
        // script does not exist
        Ok(None)
    }
}
