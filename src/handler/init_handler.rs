use std::fs;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::Path;
use std::process::Command;

use anyhow::anyhow;
use fs_extra::dir::{copy, CopyOptions};
use log::info;
use secrecy::ExposeSecret;
use symlink::{remove_symlink_auto, symlink_auto};

use crate::entity::deployment::DeploymentInformation;
use crate::entity::options::Options;
use crate::handler::github::read_installation_token;
use crate::handler::{call_and_aggregate_command, call_and_aggregate_lifecycle_script};
use crate::helper::process_helper::{CommandResult, CommandResultCollection};

pub(crate) async fn init_deployment(
    options: &Options,
    info: &DeploymentInformation,
) -> anyhow::Result<CommandResultCollection, anyhow::Error> {
    let deploy_base_dir = info.base_directory();
    let result = internal_init_deployment(options, info).await;
    call_and_aggregate_lifecycle_script(options, &deploy_base_dir, "init", result).await
}

async fn internal_init_deployment(
    options: &Options,
    info: &DeploymentInformation,
) -> anyhow::Result<CommandResultCollection, anyhow::Error> {
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
        create_dir_all(path)?;
    }

    // get the repository base directory
    let repository_directory = path.join(".easydep_base_repo");
    if repository_directory.exists() {
        info!("Easydep base repo directory exists, changing remote fetch url");

        // set the git url of the existing repository to the new one that is not expired yet
        let mut git_remote_set_url_command = Command::new("git");
        git_remote_set_url_command
            .arg("remote")
            .arg("set-url")
            .arg("origin")
            .arg(fetch_url)
            .current_dir(&repository_directory);

        let command_success =
            call_and_aggregate_command(git_remote_set_url_command, &mut command_results).await?;
        if !command_success {
            return Ok(CommandResultCollection {
                failed_command: true,
                results: command_results,
            });
        }
    } else {
        info!("Easydep base repo directory is missing, executing initial clone");

        // clone the repository initially
        let mut git_clone_command = Command::new("git");
        git_clone_command
            .arg("clone")
            .arg("--no-checkout")
            .arg(fetch_url)
            .arg(".easydep_base_repo")
            .current_dir(&options.base_directory);

        let command_success =
            call_and_aggregate_command(git_clone_command, &mut command_results).await?;
        if !command_success {
            return Ok(CommandResultCollection {
                failed_command: true,
                results: command_results,
            });
        }
    }

    // check if the deployment is still in the expected state before continuing
    info.switch_to_requested_state()?;

    // copy the created base directory to the target deployment directory
    let deploy_repo_dir = info.base_directory();
    let copy_options = CopyOptions::new()
        .overwrite(true)
        .copy_inside(true)
        .content_only(true);
    copy(&repository_directory, &deploy_repo_dir, &copy_options)?;

    // fetch the updated content from the remote
    info!(
        "Fetching git remote for deployment directory {:?}",
        deploy_repo_dir
    );
    let mut git_fetch_command = Command::new("git");
    git_fetch_command
        .arg("fetch")
        .arg("origin")
        .arg("--prune")
        .arg("--tags")
        .current_dir(&deploy_repo_dir);
    let command_success =
        call_and_aggregate_command(git_fetch_command, &mut command_results).await?;
    if !command_success {
        return Ok(CommandResultCollection {
            failed_command: true,
            results: command_results,
        });
    }

    // reset the directory to the target tag
    info!(
        "Resetting deployment directory {:?} to tag {}",
        deploy_repo_dir, info.tag_name
    );
    let mut git_reset_command = Command::new("git");
    git_reset_command
        .arg("reset")
        .arg("--hard")
        .arg(&info.tag_name)
        .current_dir(&deploy_repo_dir);
    let command_success =
        call_and_aggregate_command(git_reset_command, &mut command_results).await?;
    if !command_success {
        return Ok(CommandResultCollection {
            failed_command: true,
            results: command_results,
        });
    }

    // write revision file if requested
    let revision_file_name = &options.git_revision_file;
    if !revision_file_name.is_empty() {
        let revision_file_path = &deploy_repo_dir.join(revision_file_name);
        info!("Writing current revision into {:?}", revision_file_path);

        let rev_parse_output = Command::new("git")
            .arg("rev-parse")
            .arg(&info.tag_name)
            .current_dir(&deploy_repo_dir)
            .output()?;
        if rev_parse_output.status.success() {
            fs::write(revision_file_path, rev_parse_output.stdout)?;
        } else {
            let stderr_output = String::from_utf8_lossy(rev_parse_output.stderr.as_slice());
            return Err(anyhow!(
                "Unable to execute rev-parse git command, got output status: {}, Log: {}",
                rev_parse_output.status,
                stderr_output
            ));
        }
    }

    // remove the git directory, ignore possible errors
    let git_path = deploy_repo_dir.join(".git");
    remove_dir_all(git_path).ok();

    // create all requested additional symlinks
    let additional_symlinks = options.parse_additional_symlinks();
    for additional_symlink in additional_symlinks {
        let link_target = deploy_repo_dir.join(additional_symlink.link_name);
        info!(
            "Trying to add additional symlink: {:?} -> {:?}",
            link_target, additional_symlink.target
        );

        // create the parent directory of the link, if missing
        if let Some(parent) = link_target.parent() {
            create_dir_all(parent)?;
        }

        remove_symlink_auto(&link_target).ok();
        symlink_auto(additional_symlink.target, link_target)?;
    }

    // check if the deployment is still in the expected state before continuing
    info.switch_to_requested_state()?;

    // run the deploy script (if it exists)
    info!(
        "Executing deployment script in {:?} ({})",
        deploy_repo_dir, info.tag_name
    );
    let script_dir = format!(".easydep{}", options.environment_suffix());
    let deploy_script_path = deploy_repo_dir.join(&script_dir).join("execute.sh");
    if deploy_script_path.exists() {
        let mut script_execute_command = Command::new("bash");
        script_execute_command
            .arg(format!("{}/execute.sh", script_dir))
            .current_dir(deploy_repo_dir);
        let command_success =
            call_and_aggregate_command(script_execute_command, &mut command_results).await?;
        if !command_success {
            return Ok(CommandResultCollection {
                failed_command: true,
                results: command_results,
            });
        }
    }

    Ok(CommandResultCollection {
        failed_command: false,
        results: command_results,
    })
}
