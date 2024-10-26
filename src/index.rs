use crate::crd::Chirpstack;
use kube::{
    runtime::reflector::ObjectRef,
    core::{NamespaceResourceScope, Resource},
    ResourceExt,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObjectKey {
    pub name: String,
    pub namespace: String,
}

impl<T> From<&T> for ObjectKey
where
    T: Resource<Scope = NamespaceResourceScope, DynamicType = ()>,
{
    fn from(resource: &T) -> Self {
        ObjectKey {
            name: resource.name_any(),
            namespace: resource
                .namespace()
                .unwrap_or_else(|| "default".to_string()),
        }
    }
}

type IndexHashMap = Arc<RwLock<HashMap<ObjectKey, HashSet<ObjectRef<Chirpstack>>>>>;

#[derive(Debug)]
pub struct Index {
    pub get_objectkeys: fn(&Chirpstack) -> Vec<ObjectKey>,

    index: IndexHashMap,
}

impl Index {
    pub fn new(get_objectkeys: fn(&Chirpstack) -> Vec<ObjectKey>) -> Index {
        Index {
            get_objectkeys: get_objectkeys,
            index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update(&self, chirpstack: &Chirpstack) {
        let chirpstack_ref = ObjectRef::from_obj(chirpstack);
        let objectkeys = (self.get_objectkeys)(&chirpstack);
        let mut mut_index = self.index.write().await;

        mut_index.values_mut().for_each(|chirpstack_refs| {
            chirpstack_refs.remove(&chirpstack_ref);
        });

        for objectkey in objectkeys {
            mut_index
                .entry(objectkey)
                .or_insert_with(HashSet::new)
                .insert(chirpstack_ref.clone());
        }
    }

    pub async fn remove(&self, chirpstack: &Chirpstack) {
        let chirpstack_ref = ObjectRef::from_obj(chirpstack);
        let mut mut_index = self.index.write().await;

        mut_index.values_mut().for_each(|chirpstack_refs| {
            chirpstack_refs.remove(&chirpstack_ref);
        });
    }

    pub async fn get_affected(&self, objectkey: &ObjectKey) -> Vec<ObjectRef<Chirpstack>> {
        let index = self.index.read().await;
        match index.get(objectkey) {
            Some(item) => item.into_iter().cloned().collect(),
            None => Vec::<ObjectRef<Chirpstack>>::new(),
        }
    }
}
