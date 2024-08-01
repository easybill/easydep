/*
 * This file is part of easydep, licensed under the MIT License (MIT).
 *
 * Copyright (c) 2024 easybill GmbH
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use std::sync::Arc;

use log::{error, info};
use tokio::fs;
use tokio::sync::mpsc::channel;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::accessor::deploy_action_accessor::{CurrentAction, DeploymentStatusAccessor};
use crate::accessor::deploy_status_accessor::DeployExecutionState;
use crate::accessor::deployment_accessor::DeploymentAccessor;
use crate::accessor::github_accessor::GitHubAccessor;
use crate::config::Configuration;
use crate::easydep::deployment_service_server::DeploymentService;
use crate::easydep::{
    DeployDeleteRequest, DeployPublishRequest, DeployRollbackRequest, DeployStartRequest,
    DeployStatusRequest, DeployStatusResponse, ExecutedActionEntry,
};
use crate::executor::deploy_executor::DeployExecutor;
use crate::executor::deploy_publish_executor::publish_deployment;
use crate::executor::script_executor::{execute_scripts, ScriptType};

pub struct DeploymentServiceImpl {
    config: Configuration,
    github_accessor: GitHubAccessor,
    deployment_accessor: DeploymentAccessor,
    deployment_status_accessor: DeploymentStatusAccessor,
}

impl DeploymentServiceImpl {
    pub async fn new(
        config: Configuration,
        github_accessor: GitHubAccessor,
        deployment_status_accessor: DeploymentStatusAccessor,
    ) -> Self {
        let deployment_accessor = DeploymentAccessor::new(&config);
        Self {
            config,
            github_accessor,
            deployment_accessor,
            deployment_status_accessor,
        }
    }
}

#[tonic::async_trait]
impl DeploymentService for DeploymentServiceImpl {
    type StartDeploymentStream = ReceiverStream<Result<ExecutedActionEntry, Status>>;

    async fn start_deployment(
        &self,
        request: Request<DeployStartRequest>,
    ) -> Result<Response<Self::StartDeploymentStream>, Status> {
        let request_message = request.get_ref();
        let release_id = &request_message.release_id;
        let release_profile = &request_message.profile;
        info!(
            "received request to init deployment for release {} with profile {}",
            release_id, release_profile
        );

        // get the requested deployment profile configuration & the requested release information
        // read the GitHub access token to ensure we can even execute a deployment for the requested repository
        let deploy_config = match self.config.get_deployment_configuration(release_profile) {
            Some(deployment_configuration) => deployment_configuration,
            None => {
                return Err(Status::failed_precondition(
                    "requested deployment config is not registered",
                ))
            }
        };
        let release = match self
            .github_accessor
            .get_release_by_id(release_id, &deploy_config)
            .await
        {
            Ok(release) => release,
            Err(err) => {
                let error_message = format!("unable to find requested release: {err:?}");
                return Err(Status::failed_precondition(error_message));
            }
        };
        let github_access_token = match self
            .github_accessor
            .read_github_app_installation_token(&deploy_config)
            .await
        {
            Ok(github_access_token) => github_access_token,
            Err(err) => {
                let error_message = format!("unable to get github access token: {}", err);
                return Err(Status::internal(error_message));
            }
        };

        // check if the profile can only be used by extending it, not directly
        if deploy_config.extend_only {
            return Err(Status::failed_precondition(
                "the requested deployment profile cannot be used directly",
            ));
        }

        // check if the deployment profile can actually use the requested branch
        if !deploy_config.is_branch_allowed_to_use_config(&release.target_commitish) {
            return Err(Status::failed_precondition(
                "branch is not allowed to use requested deployment configuration",
            ));
        }

        // prepare the data needed for the deployment
        let (data_sender, data_receiver) = channel::<Result<ExecutedActionEntry, Status>>(50);
        let deployment_executor = DeployExecutor::new(
            release,
            github_access_token,
            self.config.clone(),
            self.deployment_accessor.clone(),
            deploy_config,
        );

        // check if another action is already running to prevent
        // issues with them getting in the way of each other
        let deployment_executor_arc = Arc::new(deployment_executor);
        let deployment_action = CurrentAction::Executing(deployment_executor_arc.clone());
        if !self
            .deployment_status_accessor
            .compare_and_set_action_by_variant(&CurrentAction::Idle, deployment_action)
            .await
        {
            return Err(Status::failed_precondition(
                "another action was started first, try again afterwards",
            ));
        }

        // execute the deployment
        tokio::spawn(async move {
            deployment_executor_arc
                .prepare_deployment(data_sender)
                .await;
        });
        Ok(Response::new(ReceiverStream::new(data_receiver)))
    }

    type PublishDeploymentStream = ReceiverStream<Result<ExecutedActionEntry, Status>>;

    async fn publish_deployment(
        &self,
        request: Request<DeployPublishRequest>,
    ) -> Result<Response<Self::PublishDeploymentStream>, Status> {
        let request_message = request.get_ref();
        let release_id = request_message.release_id;
        info!("Received request to publish deployment {}", release_id);

        // get the previously triggered deployment & validate it is in the correct state to be published
        let deployment_executor = match self.deployment_status_accessor.get_action().await {
            CurrentAction::Executing(executor) if executor.get_release_id() == release_id => {
                executor
            }
            _ => {
                return Err(Status::failed_precondition(
                    "no deployment or another deployment is currently being executed",
                ))
            }
        };
        if !deployment_executor
            .get_status_accessor()
            .compare_and_set_state(
                &DeployExecutionState::Prepared,
                DeployExecutionState::Publishing,
            )
            .await
        {
            return Err(Status::failed_precondition(
                "the deployment is not in the correct state to be published",
            ));
        }

        // trigger the publishing step of the deployment
        let deploy_status_accessor = self.deployment_status_accessor.clone();
        let (data_sender, data_receiver) = channel::<Result<ExecutedActionEntry, Status>>(50);
        tokio::spawn(async move {
            deployment_executor.publish_deployment(data_sender).await;
            deploy_status_accessor.set_action(CurrentAction::Idle).await;
        });
        Ok(Response::new(ReceiverStream::new(data_receiver)))
    }

    type RollbackDeploymentStream = ReceiverStream<Result<ExecutedActionEntry, Status>>;

    async fn rollback_deployment(
        &self,
        request: Request<DeployRollbackRequest>,
    ) -> Result<Response<Self::RollbackDeploymentStream>, Status> {
        let request_message = request.get_ref();
        let release_profile = &request_message.profile;
        info!(
            "received request to rollback to previous deployment on profile {}",
            release_profile
        );

        // get the requested deployment profile configuration & the requested release information
        let deploy_config = match self.config.get_deployment_configuration(release_profile) {
            Some(deployment_configuration) => deployment_configuration,
            None => {
                return Err(Status::failed_precondition(
                    "requested deployment config is not registered",
                ))
            }
        };

        // check if the profile can only be used by extending it, not directly
        if deploy_config.extend_only {
            return Err(Status::failed_precondition(
                "the requested deployment profile cannot be used directly",
            ));
        }

        // get the previous deployment to execute
        let (curr_release_directory, prev_release_directory, prev_release_id) = match self
            .deployment_accessor
            .get_release_directories_for_profile(&deploy_config)
            .await
        {
            Ok(releases) => match releases.get(1) {
                Some(release) => {
                    let current_release = releases.first().unwrap(); // if there is something at index 1 there must be something at index 0
                    (current_release.0.clone(), release.0.clone(), release.1)
                }
                None => return Err(Status::failed_precondition(
                    "no deployment to roll back to, only 1 or 0 deployments were already executed",
                )),
            },
            Err(err) => {
                let error_message = format!("Unable to resolve deployments: {}", err);
                return Err(Status::internal(error_message));
            }
        };
        let github_release_info = match self
            .github_accessor
            .get_release_by_id(&prev_release_id, &deploy_config)
            .await
        {
            Ok(release) => release,
            Err(err) => {
                let error_message = format!(
                    "Unable to resolve GitHub release for old release {}: {}",
                    prev_release_id, err
                );
                return Err(Status::failed_precondition(error_message));
            }
        };

        // check if another action is already running to prevent issues with them getting in the way of each other
        let release_boxed = Box::new(github_release_info);
        let rollback_action = CurrentAction::RollingBack(release_boxed.clone());
        if !self
            .deployment_status_accessor
            .compare_and_set_action_by_variant(&CurrentAction::Idle, rollback_action)
            .await
        {
            return Err(Status::failed_precondition(
                "another action was started first, try again afterwards",
            ));
        }

        // execute the deployment init script again and instantly publish the deployment
        // this works under the assumption that the deployment directory exists as it was just resolved
        let global_config = self.config.clone();
        let deployment_accessor = self.deployment_accessor.clone();
        let deployment_status_accessor = self.deployment_status_accessor.clone();
        let (data_sender, data_receiver) = channel::<Result<ExecutedActionEntry, Status>>(50);
        tokio::spawn(async move {
            execute_scripts(
                &release_boxed,
                &ScriptType::Init,
                &prev_release_directory,
                &deploy_config,
                &data_sender,
            )
            .await;
            publish_deployment(
                &release_boxed,
                &prev_release_directory,
                &global_config,
                &deployment_accessor,
                &deploy_config,
                &data_sender,
            )
            .await;
            if let Err(err) = fs::remove_dir_all(&curr_release_directory).await {
                error!(
                    "Unable to delete old release directory {:?}: {}, ",
                    curr_release_directory, err
                );
            }
            deployment_status_accessor
                .set_action(CurrentAction::Idle)
                .await;
        });
        Ok(Response::new(ReceiverStream::new(data_receiver)))
    }

    type DeleteUnpublishedDeploymentStream = ReceiverStream<Result<ExecutedActionEntry, Status>>;

    async fn delete_unpublished_deployment(
        &self,
        request: Request<DeployDeleteRequest>,
    ) -> Result<Response<Self::DeleteUnpublishedDeploymentStream>, Status> {
        let request_message = request.get_ref();
        let release_id = request_message.release_id;
        info!(
            "Received request to deleted unpublished deployment {}",
            release_id
        );

        // get the previously triggered deployment & validate it is in the correct state to be rolled back
        let deployment_executor = match self.deployment_status_accessor.get_action().await {
            CurrentAction::Executing(executor) if executor.get_release_id() == release_id => {
                executor
            }
            _ => {
                return Err(Status::failed_precondition(
                    "no deployment or another deployment is currently being executed",
                ))
            }
        };
        if !deployment_executor
            .get_status_accessor()
            .compare_and_set_state(
                &DeployExecutionState::Prepared,
                DeployExecutionState::Deleting,
            )
            .await
        {
            return Err(Status::failed_precondition(
                "the deployment is not in the correct state to be deleted",
            ));
        }

        // trigger the deletion
        let deployment_status_accessor = self.deployment_status_accessor.clone();
        let (data_sender, data_receiver) = channel::<Result<ExecutedActionEntry, Status>>(50);
        tokio::spawn(async move {
            deployment_executor.delete_deployment(data_sender).await;
            deployment_status_accessor
                .set_action(CurrentAction::Idle)
                .await;
        });
        Ok(Response::new(ReceiverStream::new(data_receiver)))
    }

    async fn get_deployment_status(
        &self,
        request: Request<DeployStatusRequest>,
    ) -> Result<Response<DeployStatusResponse>, Status> {
        // get the requested deployment config
        let request_message = request.get_ref();
        let deploy_config = match self
            .config
            .get_deployment_configuration(&request_message.profile)
        {
            Some(deployment_configuration) => deployment_configuration,
            None => {
                return Err(Status::failed_precondition(
                    "requested deployment config is not registered",
                ))
            }
        };

        // get the id of the last deployed release
        let last_deployed_release_id = match self
            .deployment_accessor
            .get_release_directories_for_profile(&deploy_config)
            .await
        {
            Ok(release_directories) => match release_directories.first() {
                Some(release_directory) => release_directory.1,
                None => {
                    return Err(Status::failed_precondition(
                        "no release executed with profile yet",
                    ))
                }
            },
            Err(err) => {
                let error_message = format!("unable to resolve deployed releases: {err}");
                return Err(Status::internal(error_message));
            }
        };

        // get the release information from GitHub
        let github_release_info = match self
            .github_accessor
            .get_release_by_id(&last_deployed_release_id, &deploy_config)
            .await
        {
            Ok(release) => release,
            Err(err) => {
                let error_message = format!("unable to resolve release info for {last_deployed_release_id} from GitHub: {err}");
                return Err(Status::internal(error_message));
            }
        };

        let response = DeployStatusResponse {
            profile: deploy_config.id,
            release_id: last_deployed_release_id,
            tag_name: github_release_info.tag_name,
            target_commit: github_release_info.target_commitish,
        };
        Ok(Response::new(response))
    }
}
