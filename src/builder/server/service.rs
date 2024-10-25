use k8s_openapi::api::core::v1::{Service, ServicePort, ServiceSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::ResourceExt;
use std::collections::BTreeMap;

use crate::crd::{spec::Chirpstack, types::ServiceType};

pub fn build(chirpstack: &Chirpstack) -> Service {
    let crd_name = chirpstack.name_any();
    let namespace = chirpstack.namespace().unwrap_or("default".to_string());
    let app_name = format!("chirpstack-{}", crd_name);

    // Initialize labels
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), app_name.clone());

    // Build metadata
    let metadata = ObjectMeta {
        name: Some(app_name.clone()),
        namespace: Some(namespace.clone()),
        labels: Some(labels.clone()),
        ..Default::default()
    };

    // Build service ports
    let mut ports = vec![ServicePort {
        port: chirpstack.spec.server.service.port,
        protocol: Some("TCP".to_string()),
        target_port: Some(k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(8080)),
        ..Default::default()
    }];

    // If nodePort is specified and service type is NodePort
    if chirpstack.spec.server.service.service_type == ServiceType::NodePort {
        if let Some(node_port) = chirpstack.spec.server.service.node_port {
            if let Some(port) = ports.get_mut(0) {
                port.node_port = Some(node_port);
            }
        }
    }

    // Build the service spec
    let spec = ServiceSpec {
        type_: Some(chirpstack.spec.server.service.service_type.to_string()),
        ports: Some(ports),
        selector: Some(labels),
        ..Default::default()
    };

    // Build the Service
    Service {
        metadata,
        spec: Some(spec),
        ..Default::default()
    }
}

//// Optional function to build the monitoring service if monitoring is enabled
//fn build_server_metrics_service(chirpstack: &Chirpstack) -> Option<Service> {
//    let monitoring = chirpstack.spec.server.configuration.monitoring.as_ref()?;
//    let crd_name = chirpstack.name_any();
//    let namespace = chirpstack
//        .namespace()
//        .unwrap_or_else(|| "default".to_string());
//    let app_name = format!("chirpstack-{}", crd_name);
//    let service_name = format!("{}-metrics", app_name);
//
//    // Initialize labels
//    let mut labels = BTreeMap::new();
//    labels.insert("app".to_string(), app_name.clone());
//
//    // Build metadata
//    let metadata = ObjectMeta {
//        name: Some(service_name.clone()),
//        namespace: Some(namespace.clone()),
//        labels: Some(labels.clone()),
//        ..Default::default()
//    };
//
//    // Build service ports
//    let ports = vec![ServicePort {
//        port: monitoring.port.parse().unwrap_or(8080), // Default to 8080 if parsing fails
//        protocol: Some("TCP".to_string()),
//        target_port: Some(
//            k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::String(
//                monitoring.target_port.clone(),
//            ),
//        ),
//        ..Default::default()
//    }];
//
//    // Build the service spec
//    let spec = ServiceSpec {
//        type_: Some("ClusterIP".to_string()),
//        ports: Some(ports),
//        selector: Some(labels),
//        cluster_ip: if chirpstack.spec.server.workload.replicas > 1 {
//            Some("None".to_string())
//        } else {
//            None
//        },
//        ..Default::default()
//    };
//
//    // Build the Service
//    Some(Service {
//        metadata,
//        spec: Some(spec),
//        ..Default::default()
//    })
//}
