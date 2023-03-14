use crate::entity::options::Options;
use crate::entity::requests::InitRequest;
use anyhow::anyhow;
use crossbeam::sync::ShardedLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct DeploymentInformation {
    pub tag_name: String,
    pub release_id: u64,
    options: Options,
    state: Arc<ShardedLock<DeploymentState>>,
    requested_state: Arc<ShardedLock<Option<DeploymentState>>>,
}

#[derive(PartialEq, PartialOrd, Clone, Debug)]
pub(crate) enum DeploymentState {
    Init,
    Publishable,
    Linking,
    Cancelled,
}

impl DeploymentInformation {
    pub fn new_from_request(request: &InitRequest, options: &Options) -> Self {
        Self {
            tag_name: request.tag_name.clone(),
            release_id: request.release_id,
            options: options.clone(),
            state: Arc::new(ShardedLock::new(DeploymentState::Init)),
            requested_state: Arc::new(ShardedLock::new(None)),
        }
    }

    pub fn new(tag_name: String, release_id: u64, options: &Options) -> Self {
        Self {
            tag_name,
            release_id,
            options: options.clone(),
            state: Arc::new(ShardedLock::new(DeploymentState::Init)),
            requested_state: Arc::new(ShardedLock::new(None)),
        }
    }

    pub fn base_directory(&self) -> PathBuf {
        Path::new(".")
            .join(&self.options.base_directory)
            .join("releases")
            .join(self.release_id.to_string())
    }

    pub fn set_requested_state(&self, state: DeploymentState) -> anyhow::Result<(), anyhow::Error> {
        let lock_result = self.requested_state.write();
        match lock_result {
            Ok(mut guard) => {
                *guard = Some(state);
                Ok(())
            }
            Err(_) => Err(anyhow!("Issue acquiring requested state write lock")),
        }
    }

    pub fn switch_to_requested_state(&self) -> anyhow::Result<(), anyhow::Error> {
        let lock_result = self.requested_state.write();
        match lock_result {
            Ok(mut guard) => {
                let current_request = guard.clone();
                *guard = None;

                match current_request {
                    Some(state) => {
                        self.set_state(state)?;
                        Err(anyhow!("State switch was requested and executed"))
                    }
                    None => Ok(()),
                }
            }
            Err(_) => Err(anyhow!("Issue acquiring requested state write lock")),
        }
    }

    pub fn set_state(&self, new_state: DeploymentState) -> anyhow::Result<(), anyhow::Error> {
        let lock_result = self.state.write();
        match lock_result {
            Ok(mut guard) => {
                *guard = new_state;
                Ok(())
            }
            Err(_) => Err(anyhow!("Issue acquiring state write lock")),
        }
    }

    pub fn read_state(&self) -> anyhow::Result<DeploymentState, anyhow::Error> {
        let lock_result = self.state.read();
        match lock_result {
            Ok(guard) => Ok(guard.clone()),
            Err(_) => Err(anyhow!("Issue acquiring state read lock")),
        }
    }
}
