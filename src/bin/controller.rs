use base64::{
    engine::{general_purpose},
    Engine as _,
};
use co_rust::{
    builder,
    crd::{types::WorkloadType, Chirpstack},
    error::Error,
    index::{Index, ObjectKey},
};
use env_logger;
use futures::{future::join_all, StreamExt};
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::{
    api::{Patch, PatchParams, PostParams},
    core::{NamespaceResourceScope, Resource},
    runtime::{
        controller::{Action, Controller},
        finalizer::{finalizer, Event},
        watcher,
    },
    Api, Client, ResourceExt,
};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt::Debug, sync::Arc, time::Duration};

const CONTROLLER_NAME: &str = "chirpstack-controller";

async fn apply_resource<K>(client: &Client, resource: &K) -> Result<(), Error>
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
            log::info!("{} {} not found, creating...", K::KIND, resource.name_any());
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

async fn serialize_resources<T>(
    keys: Vec<ObjectKey>,
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
    join_all(keys.into_iter().map(|key| {
        let api: Api<T> = Api::namespaced(client.clone(), namespace);
        async move {
            let result = match api.get(&key.name).await {
                Ok(cm) => match serde_json::to_string(&cm) {
                    Ok(cm) => Ok(cm),
                    Err(e) => Err(format!("{e:?}")),
                },
                Err(e) => Err(format!("{e:?}")),
            };
            match result {
                Ok(o) => o,
                Err(e) => {
                    log::warn!("Unable to get {:?} {:?}: {:?}", T::KIND, key, e);
                    "".to_string()
                }
            }
        }
    }))
    .await
}

async fn apply(context: Arc<Context>, chirpstack: Arc<Chirpstack>) -> Result<Action, Error> {
    let mut_secret_index = &context.secret_index;
    let mut_config_map_index = &context.config_map_index;
    mut_secret_index.update(chirpstack.as_ref());
    mut_config_map_index.update(chirpstack.as_ref());

    let namespace = chirpstack
        .namespace()
        .unwrap_or("default".to_string())
        .clone();
    let config_map_keys = extract_config_map_refs(&chirpstack);
    let secret_keys = extract_secret_refs(&chirpstack);

    let to_hash: Vec<String> =
        serialize_resources::<ConfigMap>(config_map_keys, &namespace, context.client.clone())
            .await
            .into_iter()
            .chain(
                serialize_resources::<Secret>(secret_keys, &namespace, context.client.clone())
                    .await
                    .into_iter(),
            )
            .collect();

    let mut hasher = Sha256::new();
    to_hash.into_iter().for_each(|s| {
        hasher.update(s);
    });
    let hash = general_purpose::STANDARD.encode(&hasher.finalize());

    let client = context.client.clone();
    match chirpstack.spec.server.workload.workload_type {
        WorkloadType::Deployment => {
            apply_resource(
                &client,
                &builder::server::deployment::build(chirpstack.as_ref(), hash.clone()),
            )
            .await
        }
        WorkloadType::StatefulSet => {
            apply_resource(
                &client,
                &builder::server::statefulset::build(chirpstack.as_ref(), hash.clone()),
            )
            .await
        }
    }?;
    apply_resource(
        &client,
        &builder::server::service::build(chirpstack.as_ref()),
    )
    .await?;
    Ok(Action::requeue(Duration::from_secs(300)))
}

async fn cleanup(context: Arc<Context>, chirpstack: Arc<Chirpstack>) -> Result<Action, Error> {
    log::info!("running cleanup for Chirpstack {:?}", chirpstack.name_any());
    let mut_secret_index = &context.secret_index;
    let mut_config_map_index = &context.config_map_index;
    mut_secret_index.remove(chirpstack.as_ref());
    mut_config_map_index.remove(chirpstack.as_ref());

    Ok(Action::await_change())
}

async fn reconcile(
    chirpstack: Arc<Chirpstack>,
    context: Arc<Context>,
) -> Result<Action, kube::runtime::finalizer::Error<Error>> {
    log::info!("reconciling Chirpstack {:?}", chirpstack.name_any());
    let api: Api<Chirpstack> = Api::namespaced(
        context.client.clone(),
        chirpstack
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default"),
    );
    finalizer(&api, "chirpstack-finalizer", chirpstack, |event| async {
        match event {
            Event::Apply(chirpstack) => apply(context, chirpstack).await,
            Event::Cleanup(chirpstack) => cleanup(context, chirpstack).await,
        }
    })
    .await
}

fn error_policy(
    _obj: Arc<Chirpstack>,
    _error: &kube::runtime::finalizer::Error<Error>,
    _ctx: Arc<Context>,
) -> Action {
    Action::requeue(Duration::from_secs(60))
}

fn extract_config_map_refs(chirpstack: &Chirpstack) -> Vec<ObjectKey> {
    let mut v = Vec::<ObjectKey>::new();
    v.push(ObjectKey {
        namespace: chirpstack
            .namespace()
            .as_deref()
            .unwrap_or("default")
            .to_string(),
        name: chirpstack
            .spec
            .server
            .configuration
            .config_files
            .config_map_name
            .clone(),
    });
    match &chirpstack.spec.server.configuration.adr_plugin_files {
        Some(adr_plugin_files) => v.push(ObjectKey {
            namespace: chirpstack
                .namespace()
                .as_deref()
                .unwrap_or("default")
                .to_string(),
            name: adr_plugin_files.config_map_name.clone(),
        }),
        None => (),
    }
    v
}

fn extract_secret_refs(chirpstack: &Chirpstack) -> Vec<ObjectKey> {
    let mut keys: Vec<ObjectKey> = chirpstack
        .spec
        .server
        .configuration
        .env_secrets
        .iter()
        .map(|name| ObjectKey {
            namespace: chirpstack
                .namespace()
                .as_deref()
                .unwrap_or("default")
                .to_string(),
            name: name.clone(),
        })
        .collect();
    keys.extend(
        chirpstack
            .spec
            .server
            .configuration
            .certificates
            .iter()
            .map(|cert| ObjectKey {
                namespace: chirpstack
                    .namespace()
                    .as_deref()
                    .unwrap_or("default")
                    .to_string(),
                name: cert.secret_name.clone(),
            }),
    );
    keys
}

struct Context {
    client: Client,
    config_map_index: Index,
    secret_index: Index,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("starting...");

    let client = Client::try_default().await?;
    let chirpstack_api: Api<Chirpstack> = Api::all(client.clone());
    let cm_api = Api::<ConfigMap>::all(client.clone());
    let secret_api = Api::<Secret>::all(client.clone());

    let context = Arc::new(Context {
        client: client,
        config_map_index: Index::new(extract_config_map_refs),
        secret_index: Index::new(extract_secret_refs),
    });

    Controller::new(chirpstack_api, watcher::Config::default())
        .watches(cm_api, watcher::Config::default(), {
            let index = context.config_map_index.clone();
            move |cm| index.get_affected(&ObjectKey::from(&cm))
        })
        .watches(secret_api.clone(), watcher::Config::default(), {
            let index = context.secret_index.clone();
            move |secret| index.get_affected(&ObjectKey::from(&secret))
        })
        .run(reconcile, error_policy, context)
        .for_each(|res| async move {
            match res {
                Ok(o) => log::info!("reconciled {o:?}"),
                Err(e) => log::warn!("reconcile failed: {e:?}"),
            }
        })
        .await;
    Ok(())
}
