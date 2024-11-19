use chirpstack_operator::{
    builder,
    crd::{types::WorkloadType, Chirpstack},
    k8s_helper::{apply_resource, delete_resource, find_and_delete},
    prometheus::create_prometheus_watcher,
    status::{StatusHandler, StatusHandlerStatus},
};
use droperator::{
    config_index::ConfigIndex,
    error::{Error, ReconcilerError},
    resource_lock::ResourceLock,
};
use env_logger;
use futures::StreamExt;
use k8s_openapi::api::apps::v1::{Deployment, StatefulSet};
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::{
    runtime::{
        controller::{Action, Controller},
        finalizer::{finalizer, Event},
        watcher,
    },
    Api, Client, ResourceExt,
};
use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::time::Instant;
use std::{sync::Arc, time::Duration};

async fn apply(
    context: Arc<Context>,
    chirpstack: Arc<Chirpstack>,
    status: &StatusHandler,
) -> Result<Action, Error> {
    let client = context.client.clone();

    if status.is_different_workload_type() {
        let result = match status.get_last_observed_workload_type() {
            WorkloadType::StatefulSet => {
                find_and_delete::<StatefulSet>(&client, chirpstack.as_ref()).await
            }
            WorkloadType::Deployment => {
                find_and_delete::<Deployment>(&client, chirpstack.as_ref()).await
            }
        };
        match result {
            Ok(_) => (),
            Err(e) => {
                log::warn!("Unable to delete previous WorkloadType: {e:?}. Ignoring and hoping for the best.");
                ()
            }
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
    apply_resource(
        &client,
        &builder::server::service_account::build(chirpstack.as_ref()),
    )
    .await?;

    if chirpstack.spec.rest_api.enabled {
        apply_resource(
            &client,
            &builder::rest_api::deployment::build(chirpstack.as_ref()),
        )
        .await?;
        apply_resource(
            &client,
            &builder::rest_api::service::build(chirpstack.as_ref()),
        )
        .await?;
    } else {
        delete_resource(
            &client,
            &builder::rest_api::deployment::build(chirpstack.as_ref()),
        )
        .await?;
        delete_resource(
            &client,
            &builder::rest_api::service::build(chirpstack.as_ref()),
        )
        .await?;
    }
    if chirpstack.spec.server.configuration.monitoring.is_some() {
        apply_resource(
            &client,
            &builder::server::monitoring_service::build(chirpstack.as_ref()),
        )
        .await?;
    } else {
        delete_resource(
            &client,
            &builder::server::monitoring_service::build(chirpstack.as_ref()),
        )
        .await?;
    }
    Ok(Action::requeue(Duration::from_secs(300)))
}

async fn cleanup(context: Arc<Context>, chirpstack: Arc<Chirpstack>) -> Result<Action, Error> {
    log::info!("running cleanup for Chirpstack {:?}", chirpstack.name_any());
    context.index.remove(chirpstack.as_ref());

    Ok(Action::await_change())
}

struct ReconcileLoopMetrics {
    start_time: Instant,
}

impl ReconcileLoopMetrics {
    fn start() -> Self {
        counter!("operator_reconcile_total").increment(1);
        ReconcileLoopMetrics {
            start_time: Instant::now(),
        }
    }

    fn action(&self) {
        counter!("operator_reconcile_action_total").increment(1);
    }

    fn stop(self, metric: &'static str) {
        let duration = self.start_time.elapsed().as_secs_f64();
        counter!(metric).increment(1);
        histogram!("operator_reconcile_duration_seconds").record(duration);
    }
}

async fn reconcile(
    chirpstack_trigger: Arc<Chirpstack>,
    context: Arc<Context>,
) -> Result<Action, ReconcilerError> {
    let metrics = ReconcileLoopMetrics::start();

    log::debug!(
        "locking reconciliation for {:?}",
        chirpstack_trigger.name_any()
    );
    let _lock = context
        .crd_lock
        .lock(chirpstack_trigger.as_ref())
        .await
        .map_err(ReconcilerError::from);
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
            log::warn!("Could not refetch resource {chirpstack_trigger:?}: {e:?}");
            Err(Error::from(e))
        }
    }?;
    context.index.update(chirpstack.as_ref());
    let status = StatusHandler::new(
        context.client.clone(),
        Arc::unwrap_or_clone(chirpstack.clone()),
    )
    .await;
    match status.status() {
        StatusHandlerStatus::NeedsReconciliation => {
            metrics.action();
            let _ = status.start_reconciliation();
            finalizer(
                &api,
                "chirpstack-finalizer",
                chirpstack.clone(),
                |event| async {
                    let result = match event {
                        Event::Apply(chirpstack) => {
                            apply(context.clone(), chirpstack, &status).await
                        }
                        Event::Cleanup(chirpstack) => cleanup(context.clone(), chirpstack).await,
                    };
                    match result {
                        Ok(action) => {
                            metrics.stop("operator_reconcile_success_total");

                            let _ = status.done_without_errors().await;
                            Ok(action)
                        }
                        Err(e) => {
                            metrics.stop("operator_reconcile_errors_total");

                            let _ = status.done_with_errors(vec![format!("{e:?}")]).await;
                            Err(e)
                        }
                    }
                },
            )
            .await
            .map_err(ReconcilerError::from)
        }
        StatusHandlerStatus::HasError => {
            metrics.stop("operator_reconcile_throttle_error_total");

            log::info!("is in error state but not yet ready for the next attempt, requeueing...");
            Ok(Action::requeue(status.error_timeout.clone()))
        }
        StatusHandlerStatus::Clean => {
            metrics.stop("operator_reconcile_no_action_total");

            log::info!("no action needed, requeueing...");
            Ok(Action::requeue(Duration::from_secs(300)))
        }
    }
}

fn error_policy(_obj: Arc<Chirpstack>, _error: &ReconcilerError, _ctx: Arc<Context>) -> Action {
    Action::requeue(Duration::from_secs(5))
}

struct Context {
    client: Client,
    index: ConfigIndex<Chirpstack>,
    crd_lock: ResourceLock,
}

build_info::build_info!(fn build_info_function);

fn get_version_info_long() -> Option<String> {
    let build_info = build_info_function();
    Some(format!(
        "{} v{} {} {}{}",
        build_info.crate_info.name,
        build_info.crate_info.version,
        build_info.version_control.as_ref()?.git()?.branch.as_ref()?,
        build_info.version_control.as_ref()?.git()?.commit_short_id,
        if build_info.version_control.as_ref()?.git()?.dirty {
            "!!!".to_string()
        } else {
            "".to_string()
        },
    ))
}

fn get_version_info_short() -> String {
    let build_info = build_info_function();
    format!(
        "{} v{}",
        build_info.crate_info.name,
        build_info.crate_info.version,
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("{}", get_version_info_long().or_else(|| Some(get_version_info_short())).unwrap());

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    PrometheusBuilder::new()
        .with_http_listener(([127, 0, 0, 1], 8383))
        .set_buckets(&[0.1, 0.5, 1.0, 3.0])?
        .install()?;

    let client = Client::try_default().await?;

    create_prometheus_watcher(client.clone());

    let chirpstack_api: Api<Chirpstack> = Api::all(client.clone());
    let cm_api = Api::<ConfigMap>::all(client.clone());
    let secret_api = Api::<Secret>::all(client.clone());

    let context = Arc::new(Context {
        client: client,
        index: ConfigIndex::new(),
        crd_lock: ResourceLock::new(),
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
