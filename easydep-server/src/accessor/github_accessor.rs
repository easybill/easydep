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

use jsonwebtoken::EncodingKey;
use octocrab::models::repos::Release;
use octocrab::models::{AppId, Installation};
use octocrab::Octocrab;
use secrecy::SecretString;
use tokio::fs;

use crate::config::{Configuration, DeploymentConfiguration};

/// An accessor for content stored on GitHub which can be accessed from a GitHub app. Only methods that are directly
/// related to the deployment process are exposed.
pub struct GitHubAccessor {
    github_client: Octocrab,
}

impl GitHubAccessor {
    /// Constructs a new GitHub accessor instance from the app settings provided in the given configuration.
    ///
    /// # Arguments
    /// * `config` - The server configuration containing the GitHub app settings.
    pub async fn new(config: &Configuration) -> anyhow::Result<Self> {
        let gh_app_rsa_key_content = fs::read(&config.github_app_pem_key_path).await?;
        let gh_app_rsa_key = EncodingKey::from_rsa_pem(gh_app_rsa_key_content.as_slice())?;
        let github_client = Octocrab::builder()
            .app(AppId::from(config.github_app_id), gh_app_rsa_key)
            .build()?;
        Ok(Self { github_client })
    }

    /// Get the app installation token that can be used to make git https requests to repos the underlying app has access to.
    ///
    /// # Arguments
    /// * `deploy_config` - The deployment configuration to get the installation token for.
    pub async fn read_github_app_installation_token(
        &self,
        deploy_config: &DeploymentConfiguration,
    ) -> anyhow::Result<SecretString> {
        let installation = self.find_installation(deploy_config).await?;
        let (_, token) = self
            .github_client
            .installation_and_token(installation.id)
            .await?;
        Ok(token)
    }

    /// Get the release with the given id in the repo associated with the given deployment configuration.
    ///
    /// # Arguments
    /// * `release_id` - The id of the release to get.
    /// * `deploy_config` - The deployment config for which the release should be retrieved.
    pub async fn get_release_by_id(
        &self,
        release_id: &u64,
        deploy_config: &DeploymentConfiguration,
    ) -> anyhow::Result<Release> {
        let installation = self.find_installation(deploy_config).await?;
        let app_scoped_client = self.github_client.installation(installation.id);
        let release = app_scoped_client
            .repos(
                &deploy_config.source_repo_owner,
                &deploy_config.source_repo_name,
            )
            .releases()
            .get(*release_id)
            .await?;
        Ok(release)
    }

    /// Finds the GitHub app installation for the repository in the given deployment configuration.
    ///
    /// # Arguments
    /// * `deploy_config` - The deployment configuration to get the GitHub app installation for.
    async fn find_installation(
        &self,
        deploy_config: &DeploymentConfiguration,
    ) -> anyhow::Result<Installation> {
        let installation = self
            .github_client
            .apps()
            .get_repository_installation(
                &deploy_config.source_repo_owner,
                &deploy_config.source_repo_name,
            )
            .await?;
        Ok(installation)
    }
}
