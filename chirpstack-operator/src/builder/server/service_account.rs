use k8s_openapi::api::core::v1::ServiceAccount;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::ResourceExt;

use crate::crd::spec::Chirpstack;

pub fn build(chirpstack: &Chirpstack) -> ServiceAccount {
    let crd_name = chirpstack.name_any();
    let namespace = chirpstack
        .namespace()
        .unwrap_or_else(|| "default".to_string());

    let sa_name = format!("chirpstack-{}", crd_name);

    ServiceAccount {
        metadata: ObjectMeta {
            name: Some(sa_name),
            namespace: Some(namespace),
            ..Default::default()
        },
        ..Default::default()
    }
}
