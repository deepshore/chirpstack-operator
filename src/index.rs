use crate::crd::Chirpstack;
use dashmap::DashMap;
use kube::{
    core::{NamespaceResourceScope, Resource},
    runtime::reflector::ObjectRef,
    ResourceExt,
};
use std::{collections::HashSet, sync::Arc};

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

type IndexMap = Arc<DashMap<ObjectKey, HashSet<ObjectRef<Chirpstack>>>>;

#[derive(Clone, Debug)]
pub struct Index {
    pub get_objectkeys: fn(&Chirpstack) -> Vec<ObjectKey>,

    index: IndexMap,
}

impl Index {
    pub fn new(get_objectkeys: fn(&Chirpstack) -> Vec<ObjectKey>) -> Index {
        Index {
            get_objectkeys: get_objectkeys,
            index: Arc::new(DashMap::new()),
        }
    }

    pub fn update(&self, chirpstack: &Chirpstack) {
        let chirpstack_ref = ObjectRef::from_obj(chirpstack);
        let objectkeys = (self.get_objectkeys)(&chirpstack);

        self.index.iter_mut().for_each(|mut entry| {
            entry.value_mut().remove(&chirpstack_ref);
        });

        for objectkey in objectkeys {
            self.index
                .entry(objectkey)
                .or_insert_with(HashSet::new)
                .insert(chirpstack_ref.clone());
        }
    }

    pub fn remove(&self, chirpstack: &Chirpstack) {
        let chirpstack_ref = ObjectRef::from_obj(chirpstack);

        self.index.iter_mut().for_each(|mut entry| {
            entry.value_mut().remove(&chirpstack_ref);
        });
    }

    pub fn get_affected(&self, objectkey: &ObjectKey) -> Vec<ObjectRef<Chirpstack>> {
        match self.index.get(objectkey) {
            Some(item) => item.value().into_iter().cloned().collect(),
            None => Vec::<ObjectRef<Chirpstack>>::new(),
        }
    }
}
