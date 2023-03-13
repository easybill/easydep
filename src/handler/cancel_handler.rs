use crate::entity::deployment::DeploymentInformation;
use std::fs::remove_dir_all;

pub(crate) async fn cancel_deployment(
    info: &DeploymentInformation,
) -> anyhow::Result<(), anyhow::Error> {
    let deployment_dir = info.base_directory();
    remove_dir_all(deployment_dir)?;
    Ok(())
}
