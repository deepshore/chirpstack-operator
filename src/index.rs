use crate::crd::Chirpstack;
use base64::{engine::general_purpose, Engine as _};
use dashmap::DashMap;
use futures::future::join_all;
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use k8s_openapi::Resource as KubeResource;
use kube::{
    core::{NamespaceResourceScope, Resource},
    runtime::reflector::ObjectRef,
    Api, Client, ResourceExt,
};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt::Debug};

#[derive(Clone, Debug)]
pub struct Index {
    index: DashMap<ObjectKey, HashSet<ObjectRef<Chirpstack>>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ObjectKey {
    pub kind: String,
    pub namespace: String,
    pub name: String,
}

impl Index {
    pub fn new() -> Index {
        Index {
            index: DashMap::new(),
        }
    }

    pub fn update(&self, chirpstack: &Chirpstack) {
        let config_map_names = extract_config_map_names(&chirpstack);
        let secret_names = extract_secret_names(&chirpstack);
        let namespace = chirpstack
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

        let chirpstack_ref = ObjectRef::from_obj(chirpstack);

        self.index.iter_mut().for_each(|mut entry| {
            entry.value_mut().remove(&chirpstack_ref);
        });

        for key in keys {
            self.index
                .entry(key)
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

    pub fn get_affected<T>(&self, resource: &T) -> Vec<ObjectRef<Chirpstack>>
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
            None => Vec::<ObjectRef<Chirpstack>>::new(),
        }
    }
}

fn extract_config_map_names(chirpstack: &Chirpstack) -> Vec<String> {
    let mut names = Vec::<String>::new();
    names.push(
        chirpstack
            .spec
            .server
            .configuration
            .config_files
            .config_map_name
            .clone(),
    );
    match &chirpstack.spec.server.configuration.adr_plugin_files {
        Some(adr_plugin_files) => names.push(adr_plugin_files.config_map_name.clone()),
        None => (),
    }
    names
}

fn extract_secret_names(chirpstack: &Chirpstack) -> Vec<String> {
    let mut names: Vec<String> = chirpstack
        .spec
        .server
        .configuration
        .env_secrets
        .iter()
        .map(|name| name.clone())
        .collect();
    names.extend(
        chirpstack
            .spec
            .server
            .configuration
            .certificates
            .iter()
            .map(|cert| cert.secret_name.clone()),
    );
    names
}

async fn serialize_resources<T>(
    names: Vec<String>,
    namespace: &String,
    client: Client,
) -> Vec<String>
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
    join_all(names.into_iter().map(|name| {
        let api: Api<T> = Api::namespaced(client.clone(), namespace);
        async move {
            let result = match api.get(&name).await {
                Ok(resource) => match serde_json::to_string(&resource) {
                    Ok(json) => Ok(json),
                    Err(e) => Err(format!("{e:?}")),
                },
                Err(e) => Err(format!("{e:?}")),
            };
            match result {
                Ok(o) => o,
                Err(e) => {
                    log::warn!("Unable to get {:?} {:?}: {:?}", T::KIND, name, e);
                    "".to_string()
                }
            }
        }
    }))
    .await
}

pub async fn determine_hash(client: &Client, chirpstack: &Chirpstack) -> String {
    let namespace = chirpstack
        .namespace()
        .unwrap_or("default".to_string())
        .clone();
    let config_map_names = extract_config_map_names(chirpstack);
    let secret_names = extract_secret_names(chirpstack);

    let to_hash: Vec<String> =
        serialize_resources::<ConfigMap>(config_map_names, &namespace, client.clone())
            .await
            .into_iter()
            .chain(
                serialize_resources::<Secret>(secret_names, &namespace, client.clone())
                    .await
                    .into_iter(),
            )
            .collect();

    let mut hasher = Sha256::new();
    to_hash.into_iter().for_each(|s| {
        hasher.update(s);
    });
    let hash = general_purpose::STANDARD.encode(&hasher.finalize());
    hash
}
