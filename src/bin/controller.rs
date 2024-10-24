use co_rust::crd::Chirpstack;
use co_rust::error::Error;
use env_logger;
use futures::StreamExt;
use kube::runtime::{
    controller::{Action, Controller},
    finalizer::{finalizer, Event},
    watcher,
};
use kube::{Api, Client, ResourceExt, api::{DeleteParams, PatchParams, Patch, PostParams}, core::{NamespaceResourceScope, Resource as KubeResource}};
use kube::api::ListParams;
use std::sync::Arc;
use std::time::Duration;
use co_rust::builder as builder;
use co_rust::crd::types::WorkloadType;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use k8s_openapi::api::apps::v1::{Deployment, StatefulSet};

const CONTROLLER_NAME: &str = "chirpstack-operator";

async fn delete_resource<K>(
    client: &Client,
    resource: &K,
) -> Result<(), Error>
where
    K: Clone
        + Debug
        + KubeResource<Scope = NamespaceResourceScope, DynamicType = ()>
        + ResourceExt
        + DeserializeOwned
        + Serialize
        + Send
        + Sync
        + 'static,
    K::DynamicType: Default,
{
    let api: Api<K> = Api::namespaced(
        client.clone(),
        resource.namespace().as_deref().unwrap_or("default"),
    );

    let dp = DeleteParams::default();

    match api.delete(&resource.name_any(), &dp).await {
        Ok(status) => {
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

async fn apply_resource<K>(
    client: &Client,
    resource: &K,
) -> Result<(), Error>
where
    K: KubeResource<Scope = NamespaceResourceScope, DynamicType = ()> + Debug + Clone + ResourceExt + DeserializeOwned + Serialize + Send + Sync + k8s_openapi::Resource,
    K::DynamicType: Default,
{
    let pp = PatchParams::apply(CONTROLLER_NAME);
    let data = serde_json::to_value(resource)?;
    let patch = Patch::Apply(data);
    let api: Api<K> = Api::namespaced(
        client.clone(),
        resource
            .namespace()
            .as_deref()
            .unwrap_or("default")
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
        },
        Err(kube::Error::Api(ae)) if ae.code == 404 => {
            log::info!("{} not found, creating...", K::KIND);
            let post_params = PostParams {
                field_manager: Some(CONTROLLER_NAME.to_string()),
                ..Default::default()
            };
            match api.create(&post_params, resource).await {
                Ok(o) => Ok(()),
                Err(e) => Err(Error::KubeError(e)),
            }
        },
        Err(e) => Err(Error::KubeError(e)),
    }
}

async fn apply(chirpstack: Arc<Chirpstack>, client: Client) -> Result<Action, Error> {
    log::info!("APPLY {chirpstack:?}");
    let result = match chirpstack.spec.server.workload.workload_type {
        WorkloadType::Deployment => apply_resource(&client, &builder::server::deployment::build(chirpstack.as_ref())).await,
        WorkloadType::StatefulSet => apply_resource(&client, &builder::server::statefulset::build(chirpstack.as_ref())).await,
    };
    match result {
        Ok(o) => Ok(Action::requeue(Duration::from_secs(300))),
        Err(e) => Err(e)
    }
}

async fn cleanup(chirpstack: Arc<Chirpstack>, client: Client) -> Result<Action, Error> {
    log::info!("**** CLEANUP");
    let cr_name = chirpstack.name_any();
    let namespace = chirpstack.namespace().unwrap_or("default".to_string());

    // Define the label selector
    let lp = ListParams::default().labels(&format!("app=chirpstack-{}", cr_name));

    // Delete Deployments
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
    let dl = deployments.list(&lp).await?;
    log::info!("deployments: {dl:?}");
    for d in dl {
        log::info!("{d:?}");
        deployments.delete(&d.name_any(), &DeleteParams::default()).await?;
    }

    // Delete StatefulSets
    let statefulsets: Api<StatefulSet> = Api::namespaced(client.clone(), &namespace);
    let ssl = statefulsets.list(&lp).await?;
    log::info!("statefulsets: {ssl:?}");
    for ss in ssl {
        log::info!("{ss:?}");
        statefulsets.delete(&ss.name_any(), &DeleteParams::default()).await?;
    }

    // Delete Services
    //let services: Api<Service> = Api::namespaced(client.clone(), &namespace);
    //for svc in services.list(&lp).await? {
    //    services.delete(&svc.name_any(), &DeleteParams::default()).await?;
    //}

    // Delete other resources similarly...

    Ok(Action::requeue(Duration::from_secs(300)))
}

async fn reconcile(
    chirpstack: Arc<Chirpstack>,
    context: Arc<Context>,
) -> Result<Action, kube::runtime::finalizer::Error<Error>> {
    log::info!("{chirpstack:?}");
    let api: Api<Chirpstack> = Api::namespaced(
        context.client.clone(),
        chirpstack
            .metadata
            .namespace
            .as_deref()
            .unwrap_or("default")
    );
    finalizer(&api, "chirpstack-finalizer", chirpstack, |event| async {
        match event {
            Event::Apply(chirpstack) => apply(chirpstack, context.client.clone()).await,
            Event::Cleanup(chirpstack) => cleanup(chirpstack, context.client.clone()).await,
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

struct Context {
    client: Client,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("starting...");
    let client = Client::try_default().await?;
    let api: Api<Chirpstack> = Api::all(client.clone());
    let context = Arc::new(Context { client: client });
    Controller::new(api, watcher::Config::default())
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
