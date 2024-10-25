use crate::crd::Chirpstack;
use kube::ResourceExt;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChirpstackRef {
    name: String,
    namespace: String,
}

impl From<&Chirpstack> for ChirpstackRef {
    fn from(chirpstack: &Chirpstack) -> Self {
        ChirpstackRef {
            name: chirpstack.name_any(),
            namespace: chirpstack
                .namespace()
                .unwrap_or_else(|| "default".to_string()),
        }
    }
}

type IndexHashMap = Arc<RwLock<HashMap<String, HashSet<ChirpstackRef>>>>;

pub struct Index {
    pub get_names: fn(&Chirpstack) -> Vec<String>,

    index: IndexHashMap,
}

impl Index {
    pub fn new(get_names: fn(&Chirpstack) -> Vec<String>) -> Index {
        Index {
            get_names: get_names,
            index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update(&self, chirpstack: &Chirpstack) {
        let chirpstack_ref = ChirpstackRef::from(chirpstack);
        let names = (self.get_names)(&chirpstack);
        let mut mut_index = self.index.write().await;

        mut_index.values_mut().for_each(|chirpstack_refs| {
            chirpstack_refs.remove(&chirpstack_ref);
        });

        for name in names {
            mut_index
                .entry(name)
                .or_insert_with(HashSet::new)
                .insert(chirpstack_ref.clone());
        }
    }

    pub async fn remove(&self, chirpstack: &Chirpstack) {
        let chirpstack_ref = ChirpstackRef::from(chirpstack);
        let mut mut_index = self.index.write().await;

        mut_index.values_mut().for_each(|chirpstack_refs| {
            chirpstack_refs.remove(&chirpstack_ref);
        });
    }

    pub async fn get_affected(&self, name: &String) -> Vec<ChirpstackRef> {
        let index = self.index.read().await;
        match index.get(name) {
            Some(item) => item.into_iter().cloned().collect(),
            None => Vec::<ChirpstackRef>::new(),
        }
    }
}
