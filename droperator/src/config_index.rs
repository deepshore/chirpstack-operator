use base64::{engine::general_purpose, Engine as _};
use dashmap::DashMap;
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use k8s_openapi::Resource as KubeResource;
use kube::{
    api::ObjectMeta,
    core::{NamespaceResourceScope, Resource},
    runtime::reflector::ObjectRef,
    Api, Client, ResourceExt,
};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt::Debug, hash::Hash};

pub trait Config {
    fn get_config_map_names(&self) -> Vec<String>;
    fn get_secret_names(&self) -> Vec<String>;
}

#[derive(Clone, Debug)]
pub struct ConfigIndex<R>
where
    R: Clone + Config + Debug + Resource<Scope = NamespaceResourceScope> + ResourceExt,
    R::DynamicType: Default + Debug + Clone + Hash + Eq,
{
    index: DashMap<ObjectKey, HashSet<ObjectRef<R>>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct ObjectKey {
    kind: String,
    namespace: String,
    name: String,
}

impl<R> ConfigIndex<R>
where
    R: Clone + Config + Debug + Resource<Scope = NamespaceResourceScope> + ResourceExt,
    R::DynamicType: Default + Debug + Clone + Hash + Eq,
{
    pub fn new() -> Self {
        ConfigIndex {
            index: DashMap::new(),
        }
    }

    pub fn update(&self, resource: &R) {
        let config_map_names = resource.get_config_map_names();
        let secret_names = resource.get_secret_names();
        let namespace = resource
            .namespace()
            .unwrap_or("default".to_string())
            .clone();

        let mut keys = Vec::<ObjectKey>::with_capacity(config_map_names.len() + secret_names.len());
        keys.extend(config_map_names.iter().map(|name| ObjectKey {
            kind: ConfigMap::KIND.to_string(),
            namespace: namespace.clone(),
            name: name.clone(),
        }));
        keys.extend(secret_names.iter().map(|name| ObjectKey {
            kind: Secret::KIND.to_string(),
            namespace: namespace.clone(),
            name: name.clone(),
        }));

        self.remove(resource);

        let resource_ref = ObjectRef::from_obj(resource);
        for key in keys {
            self.index
                .entry(key)
                .or_insert_with(HashSet::new)
                .insert(resource_ref.clone());
        }
    }

    pub fn remove(&self, resource: &R) {
        let resource_ref = ObjectRef::from_obj(resource);

        for mut entry in self.index.iter_mut() {
            entry.value_mut().remove(&resource_ref);
        }
    }

    pub fn get_affected<T>(&self, resource: &T) -> Vec<ObjectRef<R>>
    where
        T: Resource<Scope = NamespaceResourceScope>
            + Debug
            + Clone
            + ResourceExt
            + DeserializeOwned
            + Serialize
            + Send
            + Sync
            + k8s_openapi::Resource,
        T::DynamicType: Default,
    {
        let namespace = resource
            .namespace()
            .unwrap_or("default".to_string())
            .clone();
        let key = ObjectKey {
            kind: T::KIND.to_string(),
            namespace,
            name: resource.name_any(),
        };
        match self.index.get(&key) {
            Some(item) => item.value().into_iter().cloned().collect(),
            None => Vec::<ObjectRef<R>>::new(),
        }
    }
}

pub async fn determine_hash<R>(resource: &R, client: &Client) -> String
where
    R: Resource<Scope = NamespaceResourceScope>
        + Config
        + Debug
        + Clone
        + ResourceExt
        + DeserializeOwned
        + Serialize,
    R::DynamicType: Default,
{
    let namespace = resource
        .namespace()
        .unwrap_or("default".to_string())
        .clone();
    let config_map_names = resource.get_config_map_names();
    let secret_names = resource.get_secret_names();

    let mut to_hash = vec![];
    to_hash.append(
        &mut serialize_resources::<ConfigMap>(&config_map_names, &namespace, client.clone()).await,
    );
    to_hash.append(
        &mut serialize_resources::<Secret>(&secret_names, &namespace, client.clone()).await,
    );

    let mut hasher = Sha256::new();
    for s in to_hash {
        hasher.update(s);
    }

    general_purpose::STANDARD.encode(&hasher.finalize())
}

async fn serialize_resources<R>(
    names: &Vec<String>,
    namespace: &String,
    client: Client,
) -> Vec<String>
where
    R: Resource<Scope = NamespaceResourceScope>
        + Debug
        + Clone
        + ResourceExt
        + DeserializeOwned
        + Serialize
        + k8s_openapi::Resource,
    R::DynamicType: Default,
{
    let mut results = vec![];
    for name in names {
        let api: Api<R> = Api::namespaced(client.clone(), namespace);
        let result = match api.get(name).await {
            Ok(resource) => {
                let mut normalized_resource = resource.clone();
                *(normalized_resource.meta_mut()) = ObjectMeta::default();
                match serde_json::to_string(&normalized_resource) {
                    Ok(json) => Ok(json),
                    Err(e) => Err(format!("{e:?}")),
                }
            }
            Err(e) => Err(format!("{e:?}")),
        };
        results.push(match result {
            Ok(o) => o,
            Err(e) => {
                log::warn!("Unable to get {:?} {:?}: {:?}", R::KIND, name, e);
                "".to_string()
            }
        });
    }
    results
}
