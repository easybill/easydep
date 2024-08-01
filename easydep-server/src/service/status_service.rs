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

use tonic::{Request, Response, Status};

use crate::accessor::deploy_action_accessor::{CurrentAction, DeploymentStatusAccessor};
use crate::easydep::status_service_server::StatusService;
use crate::easydep::{DeployCurrentAction, StatusRequest, StatusResponse};

pub struct StatusServiceImpl {
    version: String,
    deploy_configs: Vec<String>,
    deploy_status_accessor: DeploymentStatusAccessor,
}

impl StatusServiceImpl {
    pub fn new(
        version: String,
        deploy_configs: Vec<String>,
        deploy_status_accessor: DeploymentStatusAccessor,
    ) -> Self {
        Self {
            version,
            deploy_configs,
            deploy_status_accessor,
        }
    }
}

#[tonic::async_trait]
impl StatusService for StatusServiceImpl {
    async fn get_status(
        &self,
        _request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let (current_action, current_release_id, current_release_tag) =
            match self.deploy_status_accessor.get_action().await {
                CurrentAction::Idle => (DeployCurrentAction::Idle, None, None),
                CurrentAction::Executing(executor) => {
                    let current_release = executor.get_release();
                    (
                        DeployCurrentAction::Deploying,
                        Some(current_release.id.0),
                        Some(current_release.tag_name.clone()),
                    )
                }
                CurrentAction::RollingBack(current_release) => (
                    DeployCurrentAction::RollingBack,
                    Some(current_release.id.0),
                    Some(current_release.tag_name.clone()),
                ),
            };
        let response = StatusResponse {
            version: self.version.clone(),
            current_action: i32::from(current_action),
            release_id: current_release_id,
            release_tag: current_release_tag,
            deployment_configurations: self.deploy_configs.clone(),
        };
        Ok(Response::new(response))
    }
}
