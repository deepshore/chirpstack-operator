use kube_runtime::finalizer;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kubernetes API error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

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
