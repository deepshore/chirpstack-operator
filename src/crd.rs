pub use spec::Chirpstack;

pub mod spec {
    use super::status::Status;
    use kube_derive::CustomResource;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[kube(
        status = "Status",
        group = "applications.deepshore.de",
        version = "v1alpha1",
        kind = "Chirpstack",
        namespaced
    )]
    #[serde(rename_all = "camelCase")]
    pub struct Spec {
        pub server: server::Server,
        pub rest_api: rest_api::RestApi,
    }

    pub mod server {
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct Server {
            pub workload: workload::Workload,
            pub service: service::Service,
            pub configuration: configuration::Configuration,
        }

        pub mod workload {
            use super::super::super::types::{EnvVar, Image, KeyValue, WorkloadType};
            use schemars::JsonSchema;
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
            #[serde(rename_all = "camelCase")]
            pub struct Workload {
                #[serde(rename = "type")]
                pub workload_type: WorkloadType,
                pub image: Image,
                pub replicas: i32,
                #[serde(default)]
                pub pod_annotations: Vec<KeyValue>,
                #[serde(default)]
                pub pod_labels: Vec<KeyValue>,
                #[serde(default)]
                pub extra_env_vars: Vec<EnvVar>,
            }
        }

        pub mod service {
            use super::super::super::types::ServiceType;
            use schemars::JsonSchema;
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
            #[serde(rename_all = "camelCase")]
            pub struct Service {
                #[serde(rename = "type")]
                #[serde(default)]
                pub service_type: ServiceType,
                pub port: i32,

                #[serde(skip_serializing_if = "Option::is_none")]
                pub node_port: Option<i32>,
            }
        }

        pub mod configuration {
            use super::super::super::types::{Certificate, ConfigFiles, ConfigMapName, Monitoring};
            use schemars::JsonSchema;
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
            #[serde(rename_all = "camelCase")]
            pub struct Configuration {
                pub config_files: ConfigFiles,
                #[serde(default)]
                pub env_secrets: Vec<String>,
                pub adr_plugin_files: Option<ConfigMapName>,
                #[serde(default)]
                pub certificates: Vec<Certificate>,
                pub monitoring: Option<Monitoring>,
            }
        }
    }

    pub mod rest_api {
        use super::server::service::Service;
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct RestApi {
            pub enabled: bool,

            #[serde(skip_serializing_if = "Option::is_none")]
            pub workload: Option<workload::Workload>,

            #[serde(skip_serializing_if = "Option::is_none")]
            pub service: Option<Service>,

            #[serde(skip_serializing_if = "Option::is_none")]
            pub configuration: Option<configuration::Configuration>,
        }

        pub mod workload {
            use super::super::super::types::Image;
            use schemars::JsonSchema;
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
            #[serde(rename_all = "camelCase")]
            pub struct Workload {
                pub image: Image,
                pub replicas: i32,
            }
        }

        pub mod configuration {
            use schemars::JsonSchema;
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
            #[serde(rename_all = "camelCase")]
            pub struct Configuration {
                pub insecure: bool,
            }
        }
    }
}

pub mod status {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct Status {
        pub checking: Field,
        pub reconciling: Field,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    pub enum State {
        #[default]
        Processing,
        Error,
        Done,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct Field {
        pub status: State,
        pub message: String,
    }
}

pub mod types {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use std::fmt;

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema, PartialEq)]
    #[serde(rename_all = "lowercase")]
    pub enum WorkloadType {
        #[default]
        Deployment,
        StatefulSet,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct Image {
        #[serde(default = "default_registry")]
        pub registry: String,

        #[serde(default = "default_repository")]
        pub repository: String,

        #[serde(default = "default_tag")]
        pub tag: String,
    }

    fn default_registry() -> String {
        "docker.io".to_string()
    }

    fn default_repository() -> String {
        "chirpstack/chirpstack".to_string()
    }

    fn default_tag() -> String {
        "4".to_string()
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct KeyValue {
        pub key: String,
        pub value: String,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct EnvVar {
        pub name: String,
        pub value: String,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema, PartialEq)]
    #[serde(rename_all = "PascalCase")]
    pub enum ServiceType {
        #[default]
        ClusterIP,
        NodePort,
    }

    impl fmt::Display for ServiceType {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{self:?}")
        }
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct ConfigFiles {
        pub config_map_name: String,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct ConfigMapName {
        pub config_map_name: String,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct Certificate {
        pub name: String,
        pub secret_name: String,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct Monitoring {
        pub port: String,
        pub target_port: String,
    }
}
