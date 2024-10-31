use crate::error::Error;
use dashmap::DashMap;
use kube::{
    core::{NamespaceResourceScope, Resource},
    ResourceExt,
};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

pub struct ResourceLock {
    locks: DashMap<String, Arc<Mutex<()>>>,
}

impl ResourceLock {
    pub fn new() -> Self {
        ResourceLock {
            locks: DashMap::new(),
        }
    }

    pub async fn lock<R>(&self, resource: &R) -> Result<OwnedMutexGuard<()>, Error>
    where
        R: Clone + Debug + Resource<Scope = NamespaceResourceScope> + ResourceExt,
    {
        let key = resource.uid().ok_or(Error::MissingField(format!(
            "{:?}: {}",
            resource,
            "uid".to_string()
        )))?;
        let mutex = {
            let entry = self
                .locks
                .entry(key)
                .or_insert_with(|| Arc::new(Mutex::new(())));
            Arc::clone(entry.value())
        };
        Ok(mutex.lock_owned().await)
    }
}
