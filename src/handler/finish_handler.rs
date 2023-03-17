use crate::entity::deployment::DeploymentInformation;
use crate::entity::options::Options;
use crate::handler::call_followup_lifecycle_script;
use crate::handler::release_discard::discord_oldest_release;
use crate::helper::process_helper::CommandResult;
use log::info;
use std::path::Path;
use symlink::{remove_symlink_dir, symlink_dir};

pub(crate) async fn finish_deployment(
    options: &Options,
    info: &DeploymentInformation,
) -> anyhow::Result<Option<CommandResult>, anyhow::Error> {
    let deploy_base_dir = info.base_directory();
    let result = internal_finish_deployment(options, info).await;
    call_followup_lifecycle_script(&deploy_base_dir, "publish", result).await
}

async fn internal_finish_deployment(
    options: &Options,
    info: &DeploymentInformation,
) -> anyhow::Result<(), anyhow::Error> {
    // get the paths to link
    let deployment_dir = info.base_directory();
    let deployment_link_path = Path::new(&options.deploy_link_dir);

    // remove the current symlink and create a new one
    remove_symlink_dir(deployment_link_path).ok();
    symlink_dir(deployment_dir, deployment_link_path)?;

    // cleanup (by removing the oldest release)
    info!("Published one release, trying to discord the oldest release");
    discord_oldest_release(options)?;

    Ok(())
}
