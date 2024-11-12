use droperator::config_index::Config;
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
                #[serde(default = "default_image")]
                pub image: Image,
                pub replicas: i32,
                #[serde(default)]
                pub pod_annotations: Vec<KeyValue>,
                #[serde(default)]
                pub pod_labels: Vec<KeyValue>,
                #[serde(default)]
                pub extra_env_vars: Vec<EnvVar>,
            }

            fn default_image() -> Image {
                Image {
                    registry: "docker.io".to_string(),
                    repository: "chirpstack/chirpstack".to_string(),
                    tag: "4".to_string(),
                }
            }
        }

        pub mod service {
            use super::super::super::types::ServiceType;
            use schemars::JsonSchema;
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
            #[serde(rename_all = "camelCase")]
            pub struct Service {
                #[serde(rename = "type")]
                #[serde(default)]
                pub service_type: ServiceType,
                #[serde(default = "default_port")]
                pub port: i32,

                #[serde(skip_serializing_if = "Option::is_none")]
                pub node_port: Option<i32>,
            }

            fn default_port() -> i32 {
                8080
            }

            impl Default for Service {
                fn default() -> Service {
                    Service {
                        service_type: ServiceType::default(),
                        port: 8080,
                        node_port: None,
                    }
                }
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
                #[serde(skip_serializing_if = "Option::is_none", default)]
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

            #[serde(default)]
            pub workload: workload::Workload,

            #[serde(default)]
            pub service: Service,

            #[serde(default)]
            pub configuration: configuration::Configuration,
        }

        pub mod workload {
            use super::super::super::types::Image;
            use schemars::JsonSchema;
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
            #[serde(rename_all = "camelCase")]
            pub struct Workload {
                pub image: Image,
                pub replicas: i32,
            }

            impl Default for Workload {
                fn default() -> Workload {
                    Workload {
                        image: Image {
                            registry: "docker.io".to_string(),
                            repository: "chirpstack/chirpstack-rest-api".to_string(),
                            tag: "4".to_string(),
                        },
                        replicas: 1,
                    }
                }
            }
        }

        pub mod configuration {
            use schemars::JsonSchema;
            use serde::{Deserialize, Serialize};

            #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
            #[serde(rename_all = "camelCase")]
            pub struct Configuration {
                #[serde(default = "default_insecure")]
                pub insecure: bool,
            }

            fn default_insecure() -> bool {
                true
            }
        }
    }
}

pub mod status {
    use super::types::WorkloadType;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};
    use std::time::SystemTime;

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct Status {
        pub state: State,
        pub errors: Vec<String>,
        pub bookkeeping: Bookkeeping,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct Bookkeeping {
        #[serde(default = "default_last_reconciliation_attempt")]
        pub last_reconciliation_attempt: SystemTime,
        pub last_observed_generation: Option<i64>,
        pub last_observed_workload_type: Option<WorkloadType>,
        pub last_observed_config_hash: Option<String>,
    }

    impl Default for Bookkeeping {
        fn default() -> Self {
            Bookkeeping {
                last_reconciliation_attempt: SystemTime::UNIX_EPOCH.clone(),
                last_observed_generation: None,
                last_observed_workload_type: None,
                last_observed_config_hash: None,
            }
        }
    }

    fn default_last_reconciliation_attempt() -> SystemTime {
        SystemTime::UNIX_EPOCH.clone()
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema, PartialEq, Eq)]
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
    use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

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
        pub registry: String,
        pub repository: String,
        pub tag: String,
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

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema, PartialEq, Eq, Hash)]
    #[serde(rename_all = "camelCase")]
    pub struct ConfigFiles {
        pub config_map_name: String,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema, PartialEq, Eq, Hash)]
    #[serde(rename_all = "camelCase")]
    pub struct ConfigMapName {
        pub config_map_name: String,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema, PartialEq, Eq, Hash)]
    #[serde(rename_all = "camelCase")]
    pub struct Certificate {
        pub name: String,
        pub secret_name: String,
    }

    #[derive(Debug, Serialize, Deserialize, Default, Clone, JsonSchema, PartialEq, Eq)]
    #[serde(rename_all = "camelCase")]
    pub struct Monitoring {
        pub port: i32,
        pub target_port: IntOrString,
    }
}

impl Config for Chirpstack {
    fn get_config_map_names(&self) -> Vec<String> {
        let mut names = Vec::<String>::new();
        names.push(
            self.spec
                .server
                .configuration
                .config_files
                .config_map_name
                .clone(),
        );
        match &self.spec.server.configuration.adr_plugin_files {
            Some(adr_plugin_files) => names.push(adr_plugin_files.config_map_name.clone()),
            None => (),
        }
        names
    }

    fn get_secret_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .spec
            .server
            .configuration
            .env_secrets
            .iter()
            .map(|name| name.clone())
            .collect();
        names.extend(
            self.spec
                .server
                .configuration
                .certificates
                .iter()
                .map(|cert| cert.secret_name.clone()),
        );
        names
    }
}
