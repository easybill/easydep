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

use tokio::sync::RwLock;

/// The states a running deployment can be in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DeployExecutionState {
    Preparing,
    Prepared,
    Publishing,
    Published,
    Deleting,
    Deleted,
}

/// The holder for the current status of a running deployment.
#[derive(Clone, Debug)]
pub(crate) struct DeployStatusAccessor {
    inner: Arc<RwLock<DeployExecutionState>>,
}

impl DeployStatusAccessor {
    /// Creates a new deployment status accessor instance that is in the preparing state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(DeployExecutionState::Preparing)),
        }
    }

    /// Sets the given new state.
    ///
    /// # Arguments
    /// * `new_state` - The new state to set.
    pub async fn set_state(&self, new_state: DeployExecutionState) {
        let mut write_guard = self.inner.write().await;
        *write_guard = new_state;
    }

    /// Check if the current executor is in the given expected state, if that is the case the state is switched to the
    /// given new state and `true` is returned. If that is not the case the state is unchanged and `false` is returned.
    ///
    /// # Arguments
    /// * `expected_state` - The state that is expected, the switch only happens if matching the current state.
    /// * `new_state` - The new state to switch to if the current state matches the given expected state.
    ///
    /// # Returns
    /// * `bool` - `true` if the state matched and was changed, `false` otherwise.
    pub async fn compare_and_set_state(
        &self,
        expected_state: &DeployExecutionState,
        new_state: DeployExecutionState,
    ) -> bool {
        let mut write_guard = self.inner.write().await;
        if &*write_guard == expected_state {
            *write_guard = new_state;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_starts_in_preparing() {
        let accessor = DeployStatusAccessor::new();
        let state = accessor.inner.read().await;
        assert_eq!(*state, DeployExecutionState::Preparing);
    }

    #[tokio::test]
    async fn test_set_state() {
        let accessor = DeployStatusAccessor::new();
        accessor.set_state(DeployExecutionState::Published).await;
        let state = accessor.inner.read().await;
        assert_eq!(*state, DeployExecutionState::Published);
    }

    #[tokio::test]
    async fn test_compare_and_set_succeeds_on_match() {
        let accessor = DeployStatusAccessor::new();
        let result = accessor
            .compare_and_set_state(
                &DeployExecutionState::Preparing,
                DeployExecutionState::Prepared,
            )
            .await;
        assert!(result);
        let state = accessor.inner.read().await;
        assert_eq!(*state, DeployExecutionState::Prepared);
    }

    #[tokio::test]
    async fn test_compare_and_set_fails_on_mismatch() {
        let accessor = DeployStatusAccessor::new();
        let result = accessor
            .compare_and_set_state(
                &DeployExecutionState::Published,
                DeployExecutionState::Deleted,
            )
            .await;
        assert!(!result);
        let state = accessor.inner.read().await;
        assert_eq!(*state, DeployExecutionState::Preparing);
    }

    #[tokio::test]
    async fn test_full_lifecycle_to_published() {
        let accessor = DeployStatusAccessor::new();
        assert!(
            accessor
                .compare_and_set_state(
                    &DeployExecutionState::Preparing,
                    DeployExecutionState::Prepared,
                )
                .await
        );
        assert!(
            accessor
                .compare_and_set_state(
                    &DeployExecutionState::Prepared,
                    DeployExecutionState::Publishing,
                )
                .await
        );
        assert!(
            accessor
                .compare_and_set_state(
                    &DeployExecutionState::Publishing,
                    DeployExecutionState::Published,
                )
                .await
        );
        let state = accessor.inner.read().await;
        assert_eq!(*state, DeployExecutionState::Published);
    }

    #[tokio::test]
    async fn test_full_lifecycle_to_deleted() {
        let accessor = DeployStatusAccessor::new();
        assert!(
            accessor
                .compare_and_set_state(
                    &DeployExecutionState::Preparing,
                    DeployExecutionState::Prepared,
                )
                .await
        );
        assert!(
            accessor
                .compare_and_set_state(
                    &DeployExecutionState::Prepared,
                    DeployExecutionState::Deleting,
                )
                .await
        );
        assert!(
            accessor
                .compare_and_set_state(
                    &DeployExecutionState::Deleting,
                    DeployExecutionState::Deleted,
                )
                .await
        );
        let state = accessor.inner.read().await;
        assert_eq!(*state, DeployExecutionState::Deleted);
    }
}
