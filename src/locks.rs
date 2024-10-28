use crate::{crd::Chirpstack, index::ObjectKey};
use dashmap::DashMap;
use kube::ResourceExt;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

pub struct Locks {
    locks: DashMap<ObjectKey, Arc<Mutex<()>>>,
}

impl Locks {
    pub fn new() -> Locks {
        Locks {
            locks: DashMap::new(),
        }
    }

    pub async fn lock(&self, resource: &Chirpstack) -> OwnedMutexGuard<()> {
        let namespace = resource
            .namespace()
            .unwrap_or("default".to_string())
            .clone();
        let key = ObjectKey {
            kind: "Chirpstack".to_string(),
            namespace,
            name: resource.name_any(),
        };

        let mutex = {
            let entry = self
                .locks
                .entry(key)
                .or_insert_with(|| Arc::new(Mutex::new(())));
            Arc::clone(entry.value())
        };
        mutex.clone().lock_owned().await
    }
}
