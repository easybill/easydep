use crate::entity::deployment::DeploymentInformation;
use anyhow::anyhow;
use cached::{Cached, TimedCache};
use crossbeam::sync::ShardedLock;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct DeploymentCache {
    cache: Arc<ShardedLock<TimedCache<u64, Arc<DeploymentInformation>>>>,
}

impl DeploymentCache {
    pub fn new(cache_time_secs: u64) -> Self {
        let cache: TimedCache<u64, Arc<DeploymentInformation>> =
            TimedCache::with_lifespan_and_refresh(cache_time_secs, true);
        Self {
            cache: Arc::new(ShardedLock::new(cache)),
        }
    }

    pub fn insert_deployment(
        &self,
        release_id: u64,
        deployment_info: DeploymentInformation,
    ) -> anyhow::Result<Arc<DeploymentInformation>, anyhow::Error> {
        let lock_result = self.cache.write();
        match lock_result {
            Ok(mut guard) => {
                let information = Arc::new(deployment_info);
                guard.cache_set(release_id, Arc::clone(&information));
                Ok(information)
            }
            Err(_) => Err(anyhow!("Issue acquiring deployment insert write lock")),
        }
    }

    pub fn read_deployment(
        &self,
        release_id: &u64,
    ) -> anyhow::Result<Option<Arc<DeploymentInformation>>, anyhow::Error> {
        let lock_result = self.cache.write();
        match lock_result {
            Ok(mut guard) => {
                let cache_read_result = guard.cache_get(release_id);
                Ok(cache_read_result.cloned())
            }
            Err(_) => Err(anyhow!("Issue acquiring deployment read write lock")),
        }
    }

    pub fn remove_deployment(&self, release_id: &u64) -> anyhow::Result<(), anyhow::Error> {
        let lock_result = self.cache.write();
        match lock_result {
            Ok(mut guard) => {
                guard.cache_remove(release_id);
                Ok(())
            }
            Err(_) => Err(anyhow!("Issue acquiring deployment read write lock")),
        }
    }
}
