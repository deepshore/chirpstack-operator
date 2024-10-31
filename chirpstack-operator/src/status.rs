use crate::{
    crd::{status::Field, status::State, status::Status, types::WorkloadType, Chirpstack},
};
use droperator::{
    error::Error,
    config_index::determine_hash,
};
use kube::{
    api::{Patch, PatchParams},
    Api, Client, ResourceExt,
};

pub struct StatusHandler {
    client: Client,
    chirpstack: Chirpstack,
    hash: String,
}

impl StatusHandler {
    pub async fn new(client: Client, chirpstack: Chirpstack) -> StatusHandler {
        StatusHandler {
            hash: determine_hash(&chirpstack, &client).await,
            client,
            chirpstack,
        }
    }

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    pub fn get_last_observed_workload_type(&self) -> WorkloadType {
        self.chirpstack
            .status
            .as_ref()
            .unwrap()
            .last_observed_workload_type
            .clone()
            .unwrap()
    }

    pub fn is_different_generation(&self) -> bool {
        !(self.chirpstack.status.is_some()
            && (self
                .chirpstack
                .status
                .as_ref()
                .unwrap()
                .last_observed_generation
                == self.chirpstack.metadata.generation))
    }

    pub fn is_different_workload_type(&self) -> bool {
        self.chirpstack.status.is_some()
            && self
                .chirpstack
                .status
                .as_ref()
                .unwrap()
                .last_observed_workload_type
                .is_some()
            && (self.chirpstack.spec.server.workload.workload_type
                != self
                    .chirpstack
                    .status
                    .as_ref()
                    .unwrap()
                    .last_observed_workload_type
                    .clone()
                    .unwrap())
    }

    pub fn is_different_config_hash(&self) -> bool {
        !(self.chirpstack.status.is_some()
            && self
                .chirpstack
                .status
                .as_ref()
                .unwrap()
                .last_observed_config_hash
                .is_some()
            && (self
                .chirpstack
                .status
                .as_ref()
                .unwrap()
                .last_observed_config_hash
                .as_ref()
                .unwrap()
                == &self.hash))
    }

    pub async fn update(&self, state: State, message: String) -> Result<(), Error> {
        match state {
            State::Done => {
                self.update_remote(Status {
                    reconciling: Field {
                        status: State::Done,
                        message,
                    },
                    last_observed_generation: self.chirpstack.metadata.generation,
                    last_observed_workload_type: Some(
                        self.chirpstack.spec.server.workload.workload_type.clone(),
                    ),
                    last_observed_config_hash: Some(self.hash.clone()),
                })
                .await
            }
            other => {
                self.update_remote(Status {
                    reconciling: Field {
                        status: other,
                        message,
                    },
                    last_observed_generation: self
                        .chirpstack
                        .status
                        .as_ref()
                        .and_then(|status| status.last_observed_generation),
                    last_observed_workload_type: self
                        .chirpstack
                        .status
                        .as_ref()
                        .and_then(|status| status.last_observed_workload_type.clone()),
                    last_observed_config_hash: self
                        .chirpstack
                        .status
                        .as_ref()
                        .and_then(|status| status.last_observed_config_hash.clone()),
                })
                .await
            }
        }
    }

    async fn update_remote(&self, status: Status) -> Result<(), Error> {
        let pp = PatchParams::default();
        let data = serde_json::json!({"status": status});

        let patch = Patch::Merge(data);
        let api: Api<Chirpstack> = Api::namespaced(
            self.client.clone(),
            self.chirpstack.namespace().as_deref().unwrap_or("default"),
        );

        match api
            .patch_status(&self.chirpstack.name_any(), &pp, &patch)
            .await
        {
            Ok(_) => {
                log::info!(
                    "Updated status of {} to {:?}",
                    self.chirpstack.name_any(),
                    status
                );
                Ok(())
            }
            Err(e) => {
                log::warn!(
                    "Unable to update status on {:?} to {:?}: {:?}",
                    self.chirpstack.name_any(),
                    status,
                    e
                );
                Err(Error::KubeError(e))
            }
        }
    }
}
