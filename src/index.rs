use crate::crd::Chirpstack;
use kube::{
    ResourceExt,
    core::{NamespaceResourceScope, Resource},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObjectRef {
    pub name: String,
    pub namespace: String,
}

impl<T> From<&T> for ObjectRef
where
    T: Resource<Scope = NamespaceResourceScope, DynamicType = ()>
{
    fn from(resource: &T) -> Self {
        ObjectRef {
            name: resource.name_any(),
            namespace: resource
                .namespace()
                .unwrap_or_else(|| "default".to_string()),
        }
    }
}

type IndexHashMap = Arc<RwLock<HashMap<ObjectRef, HashSet<ObjectRef>>>>;

pub struct Index {
    pub get_objectrefs: fn(&Chirpstack) -> Vec<ObjectRef>,

    index: IndexHashMap,
}

impl Index {
    pub fn new(get_objectrefs: fn(&Chirpstack) -> Vec<ObjectRef>) -> Index {
        Index {
            get_objectrefs: get_objectrefs,
            index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update(&self, chirpstack: &Chirpstack) {
        let chirpstack_ref = ObjectRef::from(chirpstack);
        let objectrefs = (self.get_objectrefs)(&chirpstack);
        let mut mut_index = self.index.write().await;

        mut_index.values_mut().for_each(|chirpstack_refs| {
            chirpstack_refs.remove(&chirpstack_ref);
        });

        for objectref in objectrefs {
            mut_index
                .entry(objectref)
                .or_insert_with(HashSet::new)
                .insert(chirpstack_ref.clone());
        }
    }

    pub async fn remove(&self, chirpstack: &Chirpstack) {
        let chirpstack_ref = ObjectRef::from(chirpstack);
        let mut mut_index = self.index.write().await;

        mut_index.values_mut().for_each(|chirpstack_refs| {
            chirpstack_refs.remove(&chirpstack_ref);
        });
    }

    pub async fn get_affected(&self, objectref: &ObjectRef) -> Vec<ObjectRef> {
        let index = self.index.read().await;
        match index.get(objectref) {
            Some(item) => item.into_iter().cloned().collect(),
            None => Vec::<ObjectRef>::new(),
        }
    }
}
