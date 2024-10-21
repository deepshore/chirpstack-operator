use kube::{Api, Client};
use kube::runtime::{controller::{Controller, Action}, watcher};
use futures::StreamExt;
use thiserror::Error;
use co_rust::crd::Chirpstack;
use std::sync::Arc;
use std::time::Duration;
use env_logger;

#[derive(Debug, Error)]
enum Error {}

async fn reconcile(g: Arc<Chirpstack>, _ctx: Arc<()>) -> Result<Action, Error> {
    log::info!("{g:?}");
    Ok(Action::requeue(Duration::from_secs(300)))
}

fn error_policy(_obj: Arc<Chirpstack>, _error: &Error, _ctx: Arc<()>) -> Action {
    Action::requeue(Duration::from_secs(60))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("starting...");
    let client = Client::try_default().await?;
    let context = Arc::new(());
    let root_objects: Api<Chirpstack> = Api::all(client);
    Controller::new(root_objects, watcher::Config::default())
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
