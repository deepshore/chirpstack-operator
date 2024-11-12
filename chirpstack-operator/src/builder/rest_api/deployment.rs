use crate::builder::meta_data::MetaData;
use crate::crd::spec::Chirpstack;
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, ContainerPort, PodSpec, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

pub fn build(chirpstack: &Chirpstack) -> Deployment {
    let meta_data = MetaData::new_rest_api(chirpstack);

    // Get workload and configuration
    let workload = &chirpstack.spec.rest_api.workload;
    let configuration = &chirpstack.spec.rest_api.configuration;

    // Construct the container image
    let image = format!(
        "{}/{}:{}",
        workload.image.registry, workload.image.repository, workload.image.tag
    );

    // Build container args
    let mut args = vec![
        "--server".to_string(),
        format!(
            "{}:{}",
            meta_data.app_name.clone(),
            chirpstack.spec.server.service.port
        ),
        "--bind".to_string(),
        "0.0.0.0:8080".to_string(),
    ];

    // Add "--insecure" if configuration.insecure is true
    if configuration.insecure {
        args.push("--insecure".to_string());
    }

    // Define container ports
    let ports = vec![ContainerPort {
        container_port: 8080,
        ..Default::default()
    }];

    // Build the container
    let container = Container {
        name: "chirpstack-rest-api".to_string(),
        image: Some(image),
        args: Some(args),
        ports: Some(ports),
        ..Default::default()
    };

    // Build the PodSpec
    let pod_spec = PodSpec {
        containers: vec![container],
        ..Default::default()
    };

    // Build the PodTemplateSpec
    let pod_template_spec = PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some(meta_data.labels.clone()),
            ..Default::default()
        }),
        spec: Some(pod_spec),
    };

    // Build the DeploymentSpec
    let deployment_spec = DeploymentSpec {
        replicas: Some(workload.replicas),
        selector: meta_data.label_selector.clone(),
        template: pod_template_spec,
        ..Default::default()
    };

    // Build the Deployment
    Deployment {
        metadata: meta_data.object_meta.clone(),
        spec: Some(deployment_spec),
        ..Default::default()
    }
}
