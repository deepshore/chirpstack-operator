use crate::{builder::meta_data::MetaData, crd::Chirpstack};
use droperator::error::Error;
use kube::{
    api::{DeleteParams, ListParams, Patch, PatchParams, PostParams},
    core::{NamespaceResourceScope, Resource},
    Api, Client, ResourceExt,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

const CONTROLLER_NAME: &str = "chirpstack-controller";

pub async fn delete_resource<K>(client: &Client, resource: &K) -> Result<(), Error>
where
    K: Clone
        + Debug
        + Resource<Scope = NamespaceResourceScope>
        + ResourceExt
        + DeserializeOwned
        + Serialize
        + Send
        + Sync,
    K::DynamicType: Default,
{
    let api: Api<K> = Api::namespaced(
        client.clone(),
        resource.namespace().as_deref().unwrap_or("default"),
    );

    let dp = DeleteParams::default();

    match api.delete(&resource.name_any(), &dp).await {
        Ok(_) => {
            log::info!(
                "Deleted {}: {}",
                K::kind(&K::DynamicType::default()),
                resource.name_any()
            );
            Ok(())
        }
        Err(kube::Error::Api(ae)) if ae.code == 404 => {
            log::info!(
                "{} not found, nothing to delete",
                K::kind(&K::DynamicType::default())
            );
            Ok(())
        }
        Err(e) => Err(Error::KubeError(e)),
    }
}

pub async fn apply_resource<K>(client: &Client, resource: &K) -> Result<(), Error>
where
    K: Resource<Scope = NamespaceResourceScope>
        + Debug
        + Clone
        + ResourceExt
        + DeserializeOwned
        + Serialize
        + Send
        + Sync
        + k8s_openapi::Resource,
    K::DynamicType: Default,
{
    let pp = PatchParams::apply(CONTROLLER_NAME);
    let data = serde_json::to_value(resource)?;
    let patch = Patch::Apply(data);
    let api: Api<K> = Api::namespaced(
        client.clone(),
        resource.namespace().as_deref().unwrap_or("default"),
    );

    log::debug!(
        "applying resource: {:?}",
        match serde_json::to_string(&resource) {
            Ok(s) => s,
            Err(e) => e.to_string(),
        }
    );
    match api.patch(&resource.name_any(), &pp, &patch).await {
        Ok(o) => {
            log::info!("Applied {}: {}", K::KIND, o.name_any());
            Ok(())
        }
        Err(kube::Error::Api(ae)) if ae.code == 404 => {
            log::info!("{} not found, creating...", resource.name_any());
            let post_params = PostParams {
                field_manager: Some(CONTROLLER_NAME.to_string()),
                ..Default::default()
            };
            match api.create(&post_params, resource).await {
                Ok(_o) => Ok(()),
                Err(e) => Err(Error::KubeError(e)),
            }
        }
        Err(e) => Err(Error::KubeError(e)),
    }
}

pub async fn find_and_delete<T>(client: &Client, chirpstack: &Chirpstack) -> Result<(), Error>
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
    let meta_data = MetaData::from(chirpstack);
    let namespace = meta_data
        .object_meta
        .namespace
        .unwrap_or("default".to_string());
    let resources: Api<T> = Api::namespaced(client.clone(), &namespace);
    let lp = ListParams::default().labels(&format!("app={0}", &meta_data.app_name));
    for r in resources.list(&lp).await? {
        delete_resource(client, &r).await?;
    }
    Ok(())
}
