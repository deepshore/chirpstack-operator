use crate::crd::types::WorkloadType;
use crate::crd::Chirpstack;
use droperator::metadata::MakeMetadata;
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    ConfigMapVolumeSource, Container, ContainerPort, EnvVar, PodSpec,
    PodTemplateSpec, SecretVolumeSource, Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use std::collections::BTreeMap;

pub fn build(chirpstack: &Chirpstack, dependent_hash: String) -> Deployment {
    assert!(chirpstack.spec.server.workload.workload_type == WorkloadType::Deployment);

    let metadata = chirpstack.make_metadata(None);

    let mut pod_labels = metadata.labels.clone();
    if !chirpstack.spec.server.workload.pod_labels.is_empty() {
        for label in &chirpstack.spec.server.workload.pod_labels {
            pod_labels.insert(label.key.clone(), label.value.clone());
        }
    }

    let mut pod_annotations = BTreeMap::new();
    if !chirpstack.spec.server.workload.pod_annotations.is_empty() {
        for annotation in &chirpstack.spec.server.workload.pod_annotations {
            pod_annotations.insert(annotation.key.clone(), annotation.value.clone());
        }
    }
    pod_annotations.insert("dependent.resources.hash".to_string(), dependent_hash);

    let image = format!(
        "{}/{}:{}",
        chirpstack.spec.server.workload.image.registry,
        chirpstack.spec.server.workload.image.repository,
        chirpstack.spec.server.workload.image.tag
    );

    let mut env_vars = vec![EnvVar {
        name: "CHIRPSTACK_SERVER_POD_NAME".to_string(),
        value_from: Some(k8s_openapi::api::core::v1::EnvVarSource {
            field_ref: Some(k8s_openapi::api::core::v1::ObjectFieldSelector {
                field_path: "metadata.name".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }];
    env_vars.append(&mut chirpstack.spec.server.workload.extra_env_vars.clone());
    env_vars.append(
        &mut chirpstack
            .spec
            .server
            .configuration
            .env
            .clone()
            .or_else(|| Some(vec![]))
            .unwrap(),
    );

    let mut volume_mounts = vec![VolumeMount {
        name: "configuration-chirpstack".to_string(),
        mount_path: "/etc/chirpstack".to_string(),
        read_only: Some(true),
        ..Default::default()
    }];

    for cert in &chirpstack.spec.server.configuration.certificates {
        volume_mounts.push(VolumeMount {
            name: cert.name.clone(),
            mount_path: format!("/certs/{}", cert.name),
            read_only: Some(true),
            ..Default::default()
        });
    }

    if chirpstack
        .spec
        .server
        .configuration
        .adr_plugin_files
        .is_some()
    {
        volume_mounts.push(VolumeMount {
            name: "adr-plugins".to_string(),
            mount_path: "/adr-plugins".to_string(),
            read_only: Some(true),
            ..Default::default()
        });
    }

    let ports = vec![ContainerPort {
        container_port: 8080,
        name: Some("web".to_string()),
        ..Default::default()
    }];

    let container = Container {
        name: "chirpstack".to_string(),
        image: Some(image),
        args: Some(vec!["-c".to_string(), "/etc/chirpstack".to_string()]),
        env: Some(env_vars),
        env_from: chirpstack.spec.server.configuration.env_from.clone(),
        ports: Some(ports),
        volume_mounts: Some(volume_mounts),
        ..Default::default()
    };

    let mut volumes = vec![Volume {
        name: "configuration-chirpstack".to_string(),
        config_map: Some(ConfigMapVolumeSource {
            name: chirpstack
                .spec
                .server
                .configuration
                .config_files
                .config_map_name
                .clone(),
            ..Default::default()
        }),
        ..Default::default()
    }];

    for cert in &chirpstack.spec.server.configuration.certificates {
        volumes.push(Volume {
            name: cert.name.clone(),
            secret: Some(SecretVolumeSource {
                secret_name: Some(cert.secret_name.clone()),
                ..Default::default()
            }),
            ..Default::default()
        });
    }

    if let Some(adr_plugin_files) = &chirpstack.spec.server.configuration.adr_plugin_files {
        volumes.push(Volume {
            name: "adr-plugins".to_string(),
            config_map: Some(ConfigMapVolumeSource {
                name: adr_plugin_files.config_map_name.clone(),
                ..Default::default()
            }),
            ..Default::default()
        });
    }

    let pod_spec = PodSpec {
        containers: vec![container],
        volumes: Some(volumes),
        ..Default::default()
    };

    let pod_template_spec = PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some(pod_labels),
            annotations: if pod_annotations.is_empty() {
                None
            } else {
                Some(pod_annotations)
            },
            ..Default::default()
        }),
        spec: Some(pod_spec),
    };

    let deployment_spec = DeploymentSpec {
        replicas: Some(chirpstack.spec.server.workload.replicas),
        selector: metadata.label_selector.clone(),
        template: pod_template_spec,
        ..Default::default()
    };

    Deployment {
        metadata: metadata.object_meta.clone(),
        spec: Some(deployment_spec),
        ..Default::default()
    }
}
