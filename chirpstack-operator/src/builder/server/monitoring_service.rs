use crate::crd::{spec::Chirpstack};
use k8s_openapi::api::core::v1::{Service, ServicePort, ServiceSpec};
use droperator::metadata::MakeMetadata;

pub fn build(chirpstack: &Chirpstack) -> Service {
    let metadata = chirpstack.make_metadata(Some("metrics".to_string()));
    let monitoring = chirpstack.spec.server.configuration.monitoring.clone().unwrap_or_default();

    let ports = vec![ServicePort {
        port: monitoring.port,
        protocol: Some("TCP".to_string()),
        target_port: Some(monitoring.target_port),
        ..Default::default()
    }];

    let spec = ServiceSpec {
        type_: Some("ClusterIP".to_string()),
        ports: Some(ports),
        selector: Some(metadata.labels),
        cluster_ip: if chirpstack.spec.server.workload.replicas > 1 {
            Some("None".to_string())
        } else {
            None
        },
        ..Default::default()
    };

    Service {
        metadata: metadata.object_meta,
        spec: Some(spec),
        ..Default::default()
    }
}
