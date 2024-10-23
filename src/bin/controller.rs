use kube::{Api, Client};
use kube::runtime::{
    controller::{Controller, Action},
    watcher,
    finalizer::{finalizer, Event},
};
use kube::runtime::finalizer::Error as FinalizerError;
use futures::{StreamExt};
use thiserror::Error;
use co_rust::crd::Chirpstack;
use std::sync::Arc;
use std::time::Duration;
use env_logger;

#[derive(Debug, Error)]
enum Error {}

async fn apply(chirpstack: Arc<Chirpstack>) -> Result<Action, Error> {
    log::info!("APPLY {chirpstack:?}");
    Ok(Action::requeue(Duration::from_secs(300)))
}

async fn cleanup(chirpstack: Arc<Chirpstack>) -> Result<Action, Error> {
    log::info!("CLEANUP {chirpstack:?}");
    Ok(Action::requeue(Duration::from_secs(300)))
}

async fn reconcile(chirpstack: Arc<Chirpstack>, context: Arc<Context>) -> Result<Action, FinalizerError<Error>> {
    log::info!("{chirpstack:?}");
    let client = context.client.clone();
    let api: Api<Chirpstack> = Api::namespaced(client, chirpstack.metadata.namespace.as_deref().or(Some("default")).unwrap());
    finalizer(
        &api,
        "chirpstack-finalizer",
        chirpstack,
        |event| async {
            match event {
                Event::Apply(chirpstack) => apply(chirpstack).await,
                Event::Cleanup(chirpstack) => cleanup(chirpstack).await,
            }
        },
    ).await
}

fn error_policy(_obj: Arc<Chirpstack>, _error: &FinalizerError<Error>, _ctx: Arc<Context>) -> Action {
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
    let context = Arc::new(
        Context{
            client: client,
        }
    );
    Controller::new(api, watcher::Config::default())
        //.owns(watcher::Config::default())
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
