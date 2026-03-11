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

use std::mem::discriminant;
use std::sync::Arc;

use octocrab::models::repos::Release;
use tokio::sync::RwLock;

use crate::executor::deploy_executor::DeployExecutor;

/// The state of actions that can be executed by this service.
#[derive(Clone, Debug)]
pub(crate) enum CurrentAction {
    /// The executor is currently idling and not doing anything.
    Idle,
    /// The executor is currently rolling back to an old release.
    RollingBack(Box<Release>),
    /// The executor is currently deploying a fresh release.
    Executing(Arc<DeployExecutor>),
}

/// The holder for the current global deployment status.
#[derive(Clone, Debug)]
pub(crate) struct DeploymentStatusAccessor {
    inner: Arc<RwLock<CurrentAction>>,
}

impl DeploymentStatusAccessor {
    /// Constructs a new holder instance with the current action set to idle.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CurrentAction::Idle)),
        }
    }

    /// Get the current action.
    pub async fn get_action(&self) -> CurrentAction {
        self.inner.read().await.clone()
    }

    /// Sets the current action of this holder.
    pub async fn set_action(&self, new_action: CurrentAction) {
        let mut guard = self.inner.write().await;
        *guard = new_action;
    }

    /// Sets the current action to the given new action if the enum variant of the
    /// current action matches the enum variant of the expected action. This does not
    /// compare the values inside the enum which are irrelevant for this operation
    /// (simple check for state changes).
    pub async fn compare_and_set_action_by_variant(
        &self,
        expected: &CurrentAction,
        new_action: CurrentAction,
    ) -> bool {
        let mut guard = self.inner.write().await;
        let expected_enum_variant = discriminant(expected);
        let current_enum_variant = discriminant(&*guard);
        if expected_enum_variant == current_enum_variant {
            *guard = new_action;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: We use JSON deserialization here because octocrab's Author struct
    // is #[non_exhaustive], preventing direct struct construction from outside the crate.
    fn make_test_release() -> Release {
        serde_json::from_value(serde_json::json!({
            "url": "https://api.github.com/repos/test/test/releases/1",
            "html_url": "https://github.com/test/test/releases/tag/v1",
            "assets_url": "https://api.github.com/repos/test/test/releases/1/assets",
            "upload_url": "",
            "id": 1,
            "node_id": "R_1",
            "tag_name": "v1.0.0",
            "target_commitish": "main",
            "draft": false,
            "prerelease": false,
            "assets": []
        }))
        .expect("failed to construct test release")
    }

    #[tokio::test]
    async fn test_new_starts_idle() {
        let accessor = DeploymentStatusAccessor::new();
        assert!(matches!(accessor.get_action().await, CurrentAction::Idle));
    }

    #[tokio::test]
    async fn test_set_and_get_action() {
        let accessor = DeploymentStatusAccessor::new();
        let release = make_test_release();
        accessor
            .set_action(CurrentAction::RollingBack(Box::new(release)))
            .await;
        assert!(matches!(
            accessor.get_action().await,
            CurrentAction::RollingBack(_)
        ));
    }

    #[tokio::test]
    async fn test_compare_and_set_succeeds_on_matching_variant() {
        let accessor = DeploymentStatusAccessor::new();
        let release = make_test_release();
        let result = accessor
            .compare_and_set_action_by_variant(
                &CurrentAction::Idle,
                CurrentAction::RollingBack(Box::new(release)),
            )
            .await;
        assert!(result);
        assert!(matches!(
            accessor.get_action().await,
            CurrentAction::RollingBack(_)
        ));
    }

    #[tokio::test]
    async fn test_compare_and_set_fails_on_mismatched_variant() {
        let accessor = DeploymentStatusAccessor::new();
        let release1 = make_test_release();
        let release2 = make_test_release();
        let result = accessor
            .compare_and_set_action_by_variant(
                &CurrentAction::RollingBack(Box::new(release1)),
                CurrentAction::RollingBack(Box::new(release2)),
            )
            .await;
        assert!(!result);
        assert!(matches!(accessor.get_action().await, CurrentAction::Idle));
    }
}
