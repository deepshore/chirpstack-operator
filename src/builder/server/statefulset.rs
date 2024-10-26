use crate::builder::meta_data::MetaData;
use crate::crd::spec::Chirpstack;
use crate::crd::types::WorkloadType;
use k8s_openapi::api::apps::v1::{StatefulSet, StatefulSetSpec};
use k8s_openapi::api::core::v1::{
    ConfigMapVolumeSource, Container, ContainerPort, EnvFromSource, EnvVar, PodSpec,
    PodTemplateSpec, SecretEnvSource, SecretVolumeSource, Volume, VolumeMount,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use std::collections::BTreeMap;

pub fn build(chirpstack: &Chirpstack) -> StatefulSet {
    assert!(chirpstack.spec.server.workload.workload_type == WorkloadType::StatefulSet);

    let meta_data = MetaData::from(chirpstack);

    // Build pod labels
    let mut pod_labels = meta_data.labels.clone();
    if !chirpstack.spec.server.workload.pod_labels.is_empty() {
        for label in &chirpstack.spec.server.workload.pod_labels {
            pod_labels.insert(label.key.clone(), label.value.clone());
        }
    }

    // Build pod annotations
    let mut pod_annotations = BTreeMap::new();
    if !chirpstack.spec.server.workload.pod_annotations.is_empty() {
        for annotation in &chirpstack.spec.server.workload.pod_annotations {
            pod_annotations.insert(annotation.key.clone(), annotation.value.clone());
        }
    }

    // Construct the container image
    let image = format!(
        "{}/{}:{}",
        chirpstack.spec.server.workload.image.registry,
        chirpstack.spec.server.workload.image.repository,
        chirpstack.spec.server.workload.image.tag
    );

    // Build environment variables
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

    for env in &chirpstack.spec.server.workload.extra_env_vars {
        env_vars.push(EnvVar {
            name: env.name.clone(),
            value: Some(env.value.clone()),
            ..Default::default()
        });
    }

    // Build envFrom sources
    let mut env_from = vec![];
    for secret_name in &chirpstack.spec.server.configuration.env_secrets {
        env_from.push(EnvFromSource {
            secret_ref: Some(SecretEnvSource {
                name: secret_name.clone(),
                ..Default::default()
            }),
            ..Default::default()
        });
    }

    // Build volume mounts
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

    // Define container ports
    let ports = vec![ContainerPort {
        container_port: 8080,
        name: Some("web".to_string()),
        ..Default::default()
    }];

    // Build the container
    let container = Container {
        name: "chirpstack".to_string(),
        image: Some(image),
        args: Some(vec!["-c".to_string(), "/etc/chirpstack".to_string()]),
        env: Some(env_vars),
        env_from: if env_from.is_empty() {
            None
        } else {
            Some(env_from)
        },
        ports: Some(ports),
        volume_mounts: Some(volume_mounts),
        ..Default::default()
    };

    // Build volumes
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

    // Build the PodSpec
    let pod_spec = PodSpec {
        containers: vec![container],
        volumes: Some(volumes),
        ..Default::default()
    };

    // Build the PodTemplateSpec
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

    // Build the StatefulSetSpec
    let statefulset_spec = StatefulSetSpec {
        replicas: Some(chirpstack.spec.server.workload.replicas),
        selector: meta_data.label_selector.clone(),
        service_name: meta_data.app_name.clone(),
        template: pod_template_spec,
        ..Default::default()
    };

    // Assemble the StatefulSet
    StatefulSet {
        metadata: meta_data.object_meta.clone(),
        spec: Some(statefulset_spec),
        ..Default::default()
    }
}
