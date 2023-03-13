use crate::entity::deployment::DeploymentInformation;
use crate::entity::options::Options;
use crate::handler::call_followup_lifecycle_script;
use crate::helper::process_helper::CommandResult;
use std::path::Path;
use symlink::{remove_symlink_auto, remove_symlink_dir, symlink_auto, symlink_dir};

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
    remove_symlink_dir(&deployment_link_path).ok();
    symlink_dir(&deployment_dir, deployment_link_path)?;

    // create all requested additional symlinks
    let additional_symlinks = options.parse_additional_symlinks();
    for additional_symlink in additional_symlinks {
        let link_target = deployment_dir.join(additional_symlink.link_name);
        println!(
            "Trying to link {:?} to {:?}",
            &link_target, &additional_symlink.target
        );
        remove_symlink_auto(&link_target).ok();
        symlink_auto(additional_symlink.target, link_target)?;
    }

    Ok(())
}
