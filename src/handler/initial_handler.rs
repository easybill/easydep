use crate::entity::deployment::DeploymentInformation;
use crate::entity::options::Options;
use crate::handler::cancel_handler::cancel_deployment;
use crate::handler::finish_handler::finish_deployment;
use crate::handler::github::read_latest_release;
use crate::handler::init_handler::init_deployment;
use crate::helper::process_helper::{pretty_print_output, CommandResult};
use anyhow::anyhow;
use log::info;
use std::path::Path;

pub(crate) async fn handle_initial_start(options: &Options) -> anyhow::Result<(), anyhow::Error> {
    let latest_release = read_latest_release(options).await?;
    if let Some(release) = latest_release {
        info!(
            "Resolved latest release to be {} (tag: {})",
            release.id, release.tag_name
        );

        // check if the release already exists
        let base_directory = Path::new(&options.base_directory).join("releases");
        let release_directory = base_directory.join(release.id.to_string());

        // check if the release already exists
        if !release_directory.exists() {
            info!("Latest release wasn't deployment before, pulling now...");
            let deploy_information =
                DeploymentInformation::new(release.tag_name, release.id.0, options);

            // execute the init & print out the result
            let init_result = init_deployment(options, &deploy_information).await?;
            if interpret_and_print_command_results(init_result.results) {
                // failed, execute the cancel handler
                cancel_deployment(&deploy_information).await?;
                return Err(anyhow!("Init handler wasn't able to process the release!"));
            }

            // publish the release
            let publish_result = finish_deployment(options, &deploy_information).await?;
            if let Some(result) = publish_result {
                print_command_result(&result);
            }
        }
    }

    Ok(())
}

fn interpret_and_print_command_results(results: Vec<CommandResult>) -> bool {
    let mut process_failed = false;
    for output in results {
        print_command_result(&output);

        let success = output.status.success();
        if !success {
            process_failed = true;
        }
    }
    process_failed
}

fn print_command_result(result: &CommandResult) {
    let pretty_output = pretty_print_output(result);
    for line in pretty_output {
        info!("{}", line);
    }
}
