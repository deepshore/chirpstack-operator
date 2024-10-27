use co_rust::{
    builder,
    builder::meta_data::MetaData,
    crd::{types::WorkloadType, Chirpstack, status::{State}},
    error::{Error, ReconcilerError},
    index::{Index},
    locks::Locks,
    status::StatusHandler,
};
use env_logger;
use futures::{StreamExt};
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::{
    api::{Patch, PatchParams, PostParams, DeleteParams, ListParams},
    core::{NamespaceResourceScope, Resource},
    runtime::{
        controller::{Action, Controller},
        finalizer::{finalizer, Event},
        watcher,
    },
    Api, Client, ResourceExt,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, sync::Arc, time::Duration};
use k8s_openapi::api::apps::v1::{StatefulSet, Deployment};

const CONTROLLER_NAME: &str = "chirpstack-controller";

async fn delete_resource<K>(
    client: &Client,
    resource: &K,
) -> Result<(), Error>
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

async fn find_and_delete<T>(client: &Client, chirpstack: &Chirpstack) -> Result<(), Error>
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
    let namespace = meta_data.object_meta.namespace.unwrap_or("default".to_string());
    let resources: Api<T> = Api::namespaced(client.clone(), &namespace);
    let lp = ListParams::default().labels(&format!("app={0}", &meta_data.app_name));
    for r in resources.list(&lp).await? {
        delete_resource(client, &r).await?;
    }
    Ok(())
}

async fn apply(context: Arc<Context>, chirpstack: Arc<Chirpstack>, status: &StatusHandler) -> Result<Action, Error> {
    let client = context.client.clone();

    if status.is_different_workload_type() {
        let result = match status.get_last_observed_workload_type() {
            WorkloadType::StatefulSet => find_and_delete::<StatefulSet>(&client, chirpstack.as_ref()).await,
            WorkloadType::Deployment => find_and_delete::<Deployment>(&client, chirpstack.as_ref()).await,
        };
        match result {
            Ok(_) => (),
            Err(e) => {
                log::warn!("Unable to delete previous WorkloadType: {e:?}. Ignoring and hoping for the best.");
                ()
            },
        }
    }

    match chirpstack.spec.server.workload.workload_type {
        WorkloadType::Deployment => {
            apply_resource(
                &client,
                &builder::server::deployment::build(chirpstack.as_ref(), status.get_hash()),
            )
            .await
        }
        WorkloadType::StatefulSet => {
            apply_resource(
                &client,
                &builder::server::statefulset::build(chirpstack.as_ref(), status.get_hash()),
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
    context.index.remove(chirpstack.as_ref());

    Ok(Action::await_change())
}

async fn reconcile(
    chirpstack_trigger: Arc<Chirpstack>,
    context: Arc<Context>,
) -> Result<Action, ReconcilerError> {
    log::debug!("locking reconciliation for {:?}", chirpstack_trigger.name_any());
    let _lock = context.locks.lock(chirpstack_trigger.as_ref());
    log::info!("reconciling Chirpstack {:?}", chirpstack_trigger.name_any());
    let api: Api<Chirpstack> = Api::namespaced(
        context.client.clone(),
        chirpstack_trigger
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default"),
    );
    let chirpstack = match api.get(&chirpstack_trigger.name_any()).await {
        Ok(o) => Ok(Arc::new(o)),
        Err(e) => {
            log::warn!("****** REFETCH {e:?}");
            Err(Error::from(e))
        }
    }?;
    context.index.update(chirpstack.as_ref());
    let status = StatusHandler::new(context.client.clone(), Arc::unwrap_or_clone(chirpstack.clone())).await;
    if status.is_different_generation() || status.is_different_config_hash() {
        let _ = status.update(State::Processing, "reconciling new generation".to_string()).await;
        finalizer(&api, "chirpstack-finalizer", chirpstack.clone(), |event| async {
            let result = match event {
                Event::Apply(chirpstack) => apply(context.clone(), chirpstack, &status).await,
                Event::Cleanup(chirpstack) => cleanup(context.clone(), chirpstack).await,
            };
            match result {
                Ok(_) => {
                    let _ = status.update(State::Done, "reconciled".to_string()).await;
                    Ok(Action::requeue(Duration::from_secs(300)))
                },
                Err(e) => {
                    let _ = status.update(State::Error, format!("{e:?}")).await;
                    Err(e)
                },
            }
        })
        .await.map_err(ReconcilerError::from)
    } else {
        log::info!("no action needed, requeueing...");
        Ok(Action::requeue(Duration::from_secs(300)))
    }
}

fn error_policy(
    _obj: Arc<Chirpstack>,
    _error: &ReconcilerError,
    _ctx: Arc<Context>,
) -> Action {
    Action::requeue(Duration::from_secs(60))
}

struct Context {
    client: Client,
    index: Index,
    locks: Locks,
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
        index: Index::new(),
        locks: Locks::new(),
    });

    Controller::new(chirpstack_api, watcher::Config::default())
        .watches(cm_api, watcher::Config::default(), {
            let context = context.clone();
            move |cm| context.index.get_affected(&cm)
        })
        .watches(secret_api.clone(), watcher::Config::default(), {
            let context = context.clone();
            move |secret| context.index.get_affected(&secret)
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
