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
