use crate::crd::{spec::Chirpstack, types::ServiceType};
use droperator::metadata::MakeMetadata;
use k8s_openapi::api::core::v1::{Service, ServicePort, ServiceSpec};

pub fn build(chirpstack: &Chirpstack) -> Service {
    let metadata = chirpstack.make_metadata(Some("rest-api".to_string()));
    let service_spec = &chirpstack.spec.rest_api.service;

    // Build service ports
    let mut ports = vec![ServicePort {
        name: Some("web".to_string()),
        port: service_spec.port,
        protocol: Some("TCP".to_string()),
        target_port: Some(k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(8080)),
        ..Default::default()
    }];

    // If nodePort is specified and service type is NodePort
    if service_spec.service_type == ServiceType::NodePort {
        if let Some(node_port) = service_spec.node_port {
            if let Some(port) = ports.get_mut(0) {
                port.node_port = Some(node_port);
            }
        }
    }

    // Build the service spec
    let spec = ServiceSpec {
        type_: Some(service_spec.service_type.to_string()),
        ports: Some(ports),
        selector: Some(metadata.labels.clone()),
        ..Default::default()
    };

    // Build the Service
    Service {
        metadata: metadata.object_meta,
        spec: Some(spec),
        ..Default::default()
    }
}
