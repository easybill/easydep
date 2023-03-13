use crate::entity::deployment::DeploymentInformation;
use crate::entity::options::Options;
use crate::handler::call_and_aggregate_lifecycle_script;
use crate::handler::github::read_installation_token;
use crate::helper::process_helper::{run_command, CommandResult};
use fs_extra::dir::{copy, CopyOptions};
use secrecy::ExposeSecret;
use std::fs::{create_dir, remove_dir_all};
use std::path::Path;
use std::process::Command;

pub(crate) async fn init_deployment(
    options: &Options,
    info: &DeploymentInformation,
) -> anyhow::Result<Vec<CommandResult>, anyhow::Error> {
    let deploy_base_dir = info.base_directory();
    let result = internal_init_deployment(options, info).await;
    call_and_aggregate_lifecycle_script(&deploy_base_dir, "init", result).await
}

async fn internal_init_deployment(
    options: &Options,
    info: &DeploymentInformation,
) -> anyhow::Result<Vec<CommandResult>, anyhow::Error> {
    let mut command_results = Vec::<CommandResult>::new();

    // read the installation token of the app and build the git fetch url from it
    let installation_token = read_installation_token(options).await?;
    let fetch_url = format!(
        "https://x-access-token:{}@github.com/{}/{}.git",
        installation_token.expose_secret(),
        &options.github_repo_org,
        &options.github_repo_name
    );

    // create the deployment base directory if it doesn't exist yet
    let path = Path::new(&options.base_directory);
    if !path.exists() {
        create_dir(path)?;
    }

    // get the repository base directory
    let repository_directory = path.join(".easydep_base_repo");
    if repository_directory.exists() {
        // set the git url of the existing repository to the new one that is not expired yet
        let mut git_remote_set_url_command = Command::new("git");
        git_remote_set_url_command
            .arg("remote")
            .arg("set-url")
            .arg("origin")
            .arg(fetch_url)
            .current_dir(&repository_directory);
        command_results.push(run_command(git_remote_set_url_command).await?);
    } else {
        // clone the repository initially
        let mut git_clone_command = Command::new("git");
        git_clone_command
            .arg("clone")
            .arg("--no-checkout")
            .arg(fetch_url)
            .arg(".easydep_base_repo")
            .current_dir(&options.base_directory);
        command_results.push(run_command(git_clone_command).await?);
    }

    // check if the deployment is still in the expected state before continuing
    info.switch_to_requested_state()?;

    // copy the created base directory to the target deployment directory
    let deploy_repo_dir = info.base_directory();
    let options = CopyOptions::new()
        .overwrite(true)
        .copy_inside(true)
        .content_only(true);
    copy(&repository_directory, &deploy_repo_dir, &options)?;

    // fetch the updated content from the remote
    let mut git_fetch_command = Command::new("git");
    git_fetch_command
        .arg("fetch")
        .arg("origin")
        .arg("--prune")
        .arg("--tags")
        .current_dir(&deploy_repo_dir);
    command_results.push(run_command(git_fetch_command).await?);

    // reset the directory to the target tag
    let mut git_reset_command = Command::new("git");
    git_reset_command
        .arg("reset")
        .arg("--hard")
        .arg(&info.tag_name)
        .current_dir(&deploy_repo_dir);
    command_results.push(run_command(git_reset_command).await?);

    // remove the git directory, ignore possible errors
    let git_path = deploy_repo_dir.join(".git");
    remove_dir_all(git_path).ok();

    // check if the deployment is still in the expected state before continuing
    info.switch_to_requested_state()?;

    // run the deploy script (if it exists)
    let deploy_script_path = deploy_repo_dir.join(".easydep").join("execute.sh");
    if deploy_script_path.exists() {
        let mut script_execute_command = Command::new("bash");
        script_execute_command
            .arg(".easydep/execute.sh")
            .current_dir(deploy_repo_dir);
        command_results.push(run_command(script_execute_command).await?);
    }

    Ok(command_results)
}