use crate::entity::deployment::DeploymentInformation;
use anyhow::anyhow;
use cached::{Cached, TtlCache};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug)]
pub(crate) struct DeploymentCache {
    cache: Arc<Mutex<TtlCache<u64, Arc<DeploymentInformation>>>>,
}

impl DeploymentCache {
    pub fn new(cache_time: Duration) -> anyhow::Result<Self> {
        let cache: TtlCache<u64, Arc<DeploymentInformation>> =
            TtlCache::builder().ttl(cache_time).refresh(true).build()?;
        Ok(Self {
            cache: Arc::new(Mutex::new(cache)),
        })
    }

    pub fn insert_deployment(
        &self,
        release_id: u64,
        deployment_info: DeploymentInformation,
    ) -> anyhow::Result<Arc<DeploymentInformation>, anyhow::Error> {
        let lock_result = self.cache.lock();
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
        let lock_result = self.cache.lock();
        match lock_result {
            Ok(mut guard) => {
                let cache_read_result = guard.cache_get(release_id);
                Ok(cache_read_result.cloned())
            }
            Err(_) => Err(anyhow!("Issue acquiring deployment read write lock")),
        }
    }

    pub fn remove_deployment(&self, release_id: &u64) -> anyhow::Result<(), anyhow::Error> {
        let lock_result = self.cache.lock();
        match lock_result {
            Ok(mut guard) => {
                guard.cache_remove(release_id);
                Ok(())
            }
            Err(_) => Err(anyhow!("Issue acquiring deployment read write lock")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::options::make_test_options;

    fn make_info(release_id: u64) -> DeploymentInformation {
        let options = make_test_options("", "");
        DeploymentInformation::new("v1.0.0".to_string(), release_id, &options)
    }

    #[test]
    fn read_unknown_id_returns_none() {
        let cache = DeploymentCache::new(Duration::from_secs(60)).unwrap();
        let result = cache.read_deployment(&999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn insert_read_remove_lifecycle() {
        let cache = DeploymentCache::new(Duration::from_secs(60)).unwrap();
        cache.insert_deployment(42, make_info(42)).unwrap();
        assert_eq!(cache.read_deployment(&42).unwrap().unwrap().release_id, 42);

        cache.remove_deployment(&42).unwrap();
        assert!(cache.read_deployment(&42).unwrap().is_none());
    }

    #[test]
    fn entry_expires_after_lifespan() {
        let cache = DeploymentCache::new(Duration::from_millis(10)).unwrap();
        cache.insert_deployment(42, make_info(42)).unwrap();
        assert_eq!(cache.read_deployment(&42).unwrap().unwrap().release_id, 42);

        std::thread::sleep(Duration::from_millis(20));
        assert!(cache.read_deployment(&42).unwrap().is_none());
    }
}
