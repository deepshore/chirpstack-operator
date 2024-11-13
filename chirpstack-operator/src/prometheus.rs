use droperator::error::Error;
use kube::api::{Api, DynamicObject, PatchParams, Patch};
use kube::core::{ApiResource, GroupVersionKind};
use kube::{Client, ResourceExt};
use std::env;
use serde::Deserialize;
use serde_yaml::Value;
use tokio::time::{sleep, Duration};

const SERVICEMONITOR_YAML: &str = include_str!("assets/service_monitor.yaml");

async fn apply_service_monitor(client: Client, namespace: String) -> Result<(), Error> {
    let yaml_str = serde_yaml::Deserializer::from_str(SERVICEMONITOR_YAML).into_iter().next().unwrap();
    let yaml_value: Value = Value::deserialize(yaml_str)?;
    let mut dynamic_obj: DynamicObject = serde_yaml::from_value(yaml_value)?;
    let gvk = GroupVersionKind::try_from(dynamic_obj.types.clone().unwrap_or_default())?;
    let api: Api<DynamicObject> =
        Api::namespaced_with(client.clone(), &namespace, &ApiResource::from_gvk(&gvk));

    dynamic_obj.metadata.name =
        Some("chirpstack-operator-controller-manager-metrics-monitor".to_string());
    dynamic_obj.metadata.namespace = Some(namespace);

    let patch_params = PatchParams::apply("chirpstack-operator").force();
    let name = dynamic_obj.name_any();
    let patch = Patch::Apply(dynamic_obj);
    api.patch(&name, &patch_params, &patch).await?;

    log::info!("Prometheus ServiceMonitor installed");
    Ok(())
}

async fn watch_for_prometheus(client: Client, namespace: String) {
    log::debug!("Trying to install ServiceMonitor");
    while let Err(e) = apply_service_monitor(client.clone(), namespace.clone()).await {
        log::debug!("ServiceMonitor installation failed: {e:?}");
        sleep(Duration::from_secs(60)).await;
        log::debug!("Retrying to install ServiceMonitor");
    }
}

pub fn create_prometheus_watcher(client: Client) {
    let namespace = env::var("OPERATOR_NAMESPACE").unwrap_or_else(|_| "default".to_string());
    tokio::spawn(watch_for_prometheus(client, namespace));
}
