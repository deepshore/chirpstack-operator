use crate::crd::Chirpstack;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta, OwnerReference};
use kube::{Resource, ResourceExt};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct MetaData {
    pub app_name: String,
    pub labels: BTreeMap<String, String>,
    pub label_selector: LabelSelector,
    pub object_meta: ObjectMeta,
}

impl MetaData {
    pub fn new_rest_api(chirpstack: &Chirpstack) -> Self {
        MetaData::new(chirpstack, "chirpstack-rest-api".to_string())
    }

    pub fn new_server(chirpstack: &Chirpstack) -> Self {
        MetaData::new(chirpstack, "chirpstack".to_string())
    }

    pub fn new(chirpstack: &Chirpstack, app_name_prefix: String) -> Self {
        let app_name = format!("{}-{}", app_name_prefix, chirpstack.name_any());

        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), app_name.clone());
        let label_selector = LabelSelector {
            match_labels: Some(labels.clone()),
            ..Default::default()
        };

        let owner_reference = OwnerReference {
            api_version: Chirpstack::api_version(&()).to_string(),
            kind: Chirpstack::kind(&()).to_string(),
            name: chirpstack.name_any(),
            uid: chirpstack.meta().uid.clone().unwrap_or_default(),
            controller: Some(true),
            block_owner_deletion: Some(true),
        };

        let namespace = chirpstack
            .namespace()
            .unwrap_or_else(|| "default".to_string());
        let object_meta = ObjectMeta {
            name: Some(app_name.clone()),
            namespace: Some(namespace),
            labels: Some(labels.clone()),
            owner_references: Some(vec![owner_reference]),
            ..Default::default()
        };

        MetaData {
            app_name,
            labels,
            label_selector,
            object_meta,
        }
    }
}

impl From<&Chirpstack> for MetaData {
    fn from(chirpstack: &Chirpstack) -> Self {
        MetaData::new_server(chirpstack)
    }
}
