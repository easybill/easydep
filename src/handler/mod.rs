pub(crate) mod cancel_handler;
pub(crate) mod finish_handler;
pub(crate) mod github;
pub(crate) mod init_handler;

use crate::helper::process_helper::{run_command, CommandResult};
use std::fmt::Debug;
use std::path::PathBuf;
use std::process::Command;

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

pub(crate) async fn call_followup_lifecycle_script<T: Debug>(
    deploy_base_directory: &PathBuf,
    lifecycle_event_name: &str,
    previous_result: anyhow::Result<T, anyhow::Error>,
) -> anyhow::Result<Option<CommandResult>, anyhow::Error> {
    let state = LifecycleState::from_result(&previous_result);
    let command_result =
        call_lifecycle_script(deploy_base_directory, lifecycle_event_name, state).await?;

    if previous_result.is_err() {
        Err(previous_result.unwrap_err())
    } else {
        Ok(command_result)
    }
}

pub(crate) async fn call_and_aggregate_lifecycle_script(
    deploy_base_directory: &PathBuf,
    lifecycle_event_name: &str,
    previous_result: Result<Vec<CommandResult>, anyhow::Error>,
) -> anyhow::Result<Vec<CommandResult>, anyhow::Error> {
    let state = LifecycleState::from_result(&previous_result);
    let command_result =
        call_lifecycle_script(deploy_base_directory, lifecycle_event_name, state).await?;

    // return the previous result if there was an error
    if previous_result.is_err() {
        return previous_result;
    }

    // get the output vec from the previous input & aggregate it with the new command output
    let mut results = previous_result.unwrap();
    if let Some(result) = command_result {
        results.push(result);
    }

    Ok(results)
}

pub(crate) async fn call_lifecycle_script(
    deploy_base_directory: &PathBuf,
    lifecycle_event_name: &str,
    state: LifecycleState,
) -> anyhow::Result<Option<CommandResult>, anyhow::Error> {
    // resolve the target script path
    let script_name = format!("{}_{:?}.sh", lifecycle_event_name, state).to_lowercase();
    let script_path = deploy_base_directory.join(".easydep").join(&script_name);

    // run the script if it exists
    if script_path.exists() {
        let script_name_for_command = format!(".easydep/{}", script_name);
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
