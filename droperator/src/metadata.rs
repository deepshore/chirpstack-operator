use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta, OwnerReference};
use kube::{CustomResourceExt, Resource, ResourceExt};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Metadata {
    pub namespace: String,
    pub app_name: String,
    pub labels: BTreeMap<String, String>,
    pub label_selector: LabelSelector,
    pub object_meta: ObjectMeta,
}

pub trait MakeMetadata<T> {
    fn make_metadata(&self, describer: Option<String>) -> Metadata;
}

impl<T: CustomResourceExt + ResourceExt + Resource<DynamicType = ()>> MakeMetadata<T> for T {
    fn make_metadata(&self, describer: Option<String>) -> Metadata {
        let describer_string = if let Some(d) = describer {
            format!("-{}", d)
        } else {
            "".to_string()
        };
        let app_name = format!(
            "{}{}-{}",
            T::kind(&()).to_string().to_lowercase(),
            describer_string,
            self.name_any()
        );

        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), app_name.clone());
        let label_selector = LabelSelector {
            match_labels: Some(labels.clone()),
            ..Default::default()
        };

        let owner_reference = OwnerReference {
            api_version: T::api_version(&()).to_string(),
            kind: T::kind(&()).to_string(),
            name: self.name_any(),
            uid: self.meta().uid.clone().unwrap_or_default(),
            controller: Some(true),
            block_owner_deletion: Some(true),
        };

        let object_meta = ObjectMeta {
            name: Some(app_name.clone()),
            namespace: self.namespace(),
            labels: Some(labels.clone()),
            owner_references: Some(vec![owner_reference]),
            ..Default::default()
        };

        let namespace = self.namespace().unwrap_or_else(|| "default".to_string());

        Metadata {
            namespace,
            app_name,
            labels,
            label_selector,
            object_meta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kube_derive::CustomResource;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[kube(
        group = "mygroup",
        version = "v1alpha1",
        kind = "MyCustomResource",
        namespaced
    )]
    pub struct MyCustomResourceSpec {
        #[serde(default)]
        foo: Option<String>,
    }

    #[test]
    fn test_make_metadata_without_describer() {
        let mut resource = MyCustomResource::new("test-1", MyCustomResourceSpec::default());
        resource.metadata.namespace = Some("test-namespace".to_string());
        resource.metadata.uid = Some("12345".to_string());

        let metadata = resource.make_metadata(None);

        assert_eq!(metadata.namespace, "test-namespace");
        let expected_app_name = "mycustomresource-test-1";
        assert_eq!(metadata.app_name, expected_app_name);

        let mut expected_labels = BTreeMap::new();
        expected_labels.insert("app".to_string(), metadata.app_name.clone());
        assert_eq!(metadata.labels, expected_labels);

        assert_eq!(
            metadata.label_selector.match_labels,
            Some(expected_labels.clone())
        );

        assert_eq!(metadata.object_meta.name, Some(metadata.app_name.clone()));
        assert_eq!(
            metadata.object_meta.namespace,
            Some("test-namespace".to_string())
        );
        assert_eq!(metadata.object_meta.labels, Some(expected_labels.clone()));

        let owner_refs = metadata.object_meta.owner_references.unwrap();
        assert_eq!(owner_refs.len(), 1);
        let owner_ref = &owner_refs[0];
        assert_eq!(owner_ref.api_version, MyCustomResource::api_version(&()));
        assert_eq!(owner_ref.kind, MyCustomResource::kind(&()));
        assert_eq!(owner_ref.name, resource.name_any());
        assert_eq!(owner_ref.uid, "12345");
        assert_eq!(owner_ref.controller, Some(true));
        assert_eq!(owner_ref.block_owner_deletion, Some(true));
    }

    #[test]
    fn test_make_metadata_with_describer() {
        let mut resource = MyCustomResource::new("test-1", MyCustomResourceSpec::default());
        resource.metadata.namespace = Some("test-namespace".to_string());
        resource.metadata.uid = Some("12345".to_string());

        let metadata = resource.make_metadata(Some("desc".to_string()));

        assert_eq!(metadata.namespace, "test-namespace");
        let expected_app_name = "mycustomresource-desc-test-1";
        assert_eq!(metadata.app_name, expected_app_name);

        let mut expected_labels = BTreeMap::new();
        expected_labels.insert("app".to_string(), metadata.app_name.clone());
        assert_eq!(metadata.labels, expected_labels);

        assert_eq!(
            metadata.label_selector.match_labels,
            Some(expected_labels.clone())
        );

        assert_eq!(metadata.object_meta.name, Some(metadata.app_name.clone()));
        assert_eq!(
            metadata.object_meta.namespace,
            Some("test-namespace".to_string())
        );
        assert_eq!(metadata.object_meta.labels, Some(expected_labels.clone()));

        let owner_refs = metadata.object_meta.owner_references.unwrap();
        assert_eq!(owner_refs.len(), 1);
        let owner_ref = &owner_refs[0];
        assert_eq!(owner_ref.api_version, MyCustomResource::api_version(&()));
        assert_eq!(owner_ref.kind, MyCustomResource::kind(&()));
        assert_eq!(owner_ref.name, resource.name_any());
        assert_eq!(owner_ref.uid, "12345");
        assert_eq!(owner_ref.controller, Some(true));
        assert_eq!(owner_ref.block_owner_deletion, Some(true));
    }

    #[test]
    fn test_make_metadata_with_default_namespace() {
        let mut resource = MyCustomResource::new("test-1", MyCustomResourceSpec::default());
        resource.metadata.uid = Some("12345".to_string());

        let metadata = resource.make_metadata(None);

        assert_eq!(metadata.namespace, "default");
        let expected_app_name = "mycustomresource-test-1";
        assert_eq!(metadata.app_name, expected_app_name);
    }
}
