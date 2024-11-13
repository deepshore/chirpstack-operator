use kube_runtime::finalizer;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kubernetes API error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    #[error("ParseGroupVersionError: {0}")]
    ParseGroupVersionError(#[from] kube::core::gvk::ParseGroupVersionError),

    #[error("Missing field: {0}")]
    MissingField(String),

    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ReconcilerError {
    #[error("Error: {0}")]
    Error(#[from] Error),

    #[error("Finalizer error: {0}")]
    FinalizerError(#[from] finalizer::Error<Error>),
}
