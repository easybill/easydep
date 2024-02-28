use crate::entity::deployment::DeploymentInformation;
use crate::entity::options::Options;
use crate::handler::call_followup_lifecycle_script;
use crate::handler::release_discard::discard_oldest_release;
use crate::helper::process_helper::CommandResult;
use log::{error, info};
use std::path::Path;
use symlink::{remove_symlink_dir, symlink_dir};

pub(crate) async fn finish_deployment(
    options: &Options,
    info: &DeploymentInformation,
) -> anyhow::Result<Option<CommandResult>, anyhow::Error> {
    let deploy_base_dir = info.base_directory();
    let result = internal_finish_deployment(options, info).await;
    let finish_script_result =
        call_followup_lifecycle_script(options, &deploy_base_dir, "publish", result).await;

    // cleanup (by removing the oldest release)
    info!("Published one release, trying to discord the oldest release");
    if let Err(error) = discard_oldest_release(options) {
        error!("Unable to delete oldest release: {}", error);
    }

    finish_script_result
}

async fn internal_finish_deployment(
    options: &Options,
    info: &DeploymentInformation,
) -> anyhow::Result<(), anyhow::Error> {
    // get the paths to link
    let deployment_dir = info.base_directory();
    let deployment_link_path = Path::new(&options.base_directory).join(&options.deploy_link_dir);

    // remove the current symlink and create a new one
    remove_symlink_dir(&deployment_link_path).ok();
    symlink_dir(deployment_dir, deployment_link_path)?;

    Ok(())
}
