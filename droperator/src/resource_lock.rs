use crate::error::Error;
use std::collections::HashMap;
use kube::{
    core::{NamespaceResourceScope, Resource},
    ResourceExt,
};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

pub struct ResourceLock {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl ResourceLock {
    pub fn new() -> Self {
        ResourceLock {
            locks: Mutex::new(HashMap::new()),
        }
    }

    pub async fn lock<R>(&self, resource: &R) -> Result<OwnedMutexGuard<()>, Error>
    where
        R: Clone + Debug + Resource<Scope = NamespaceResourceScope> + ResourceExt,
    {
        let mut unlocked = self.locks.lock().await;
        let key = resource.uid().ok_or(Error::MissingField(format!(
            "{:?}: {}",
            resource,
            "uid".to_string()
        )))?;
        let mutex = {
            let entry = unlocked
                .entry(key)
                .or_insert_with(|| Arc::new(Mutex::new(())));
            Arc::clone(entry)
        };
        Ok(mutex.lock_owned().await)
    }
}
