use crate::crd::{
    status::Bookkeeping, status::State, status::Status, types::WorkloadType, Chirpstack,
};
use droperator::{config_index::determine_hash, error::Error};
use kube::{
    api::{Patch, PatchParams},
    Api, Client, ResourceExt,
};
use std::time::{Duration, SystemTime};

pub struct StatusHandler {
    pub error_timeout: Duration,

    client: Client,
    chirpstack: Chirpstack,
    hash: String,
    this_reconciliation_attempt: SystemTime,
}

pub enum StatusHandlerStatus {
    NeedsReconciliation,
    HasError,
    Clean,
}

impl StatusHandler {
    pub async fn new(client: Client, chirpstack: Chirpstack) -> StatusHandler {
        StatusHandler {
            hash: determine_hash(&chirpstack, &client).await,
            client,
            chirpstack,
            error_timeout: Duration::from_secs(30),
            this_reconciliation_attempt: SystemTime::now(),
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
            .bookkeeping
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
                .bookkeeping
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
                .bookkeeping
                .last_observed_workload_type
                .is_some()
            && (self.chirpstack.spec.server.workload.workload_type
                != self
                    .chirpstack
                    .status
                    .as_ref()
                    .unwrap()
                    .bookkeeping
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
                .bookkeeping
                .last_observed_config_hash
                .is_some()
            && (self
                .chirpstack
                .status
                .as_ref()
                .unwrap()
                .bookkeeping
                .last_observed_config_hash
                .as_ref()
                .unwrap()
                == &self.hash))
    }

    pub fn is_time_to_retry(&self) -> bool {
        let elapsed = self
            .chirpstack
            .status
            .as_ref()
            .unwrap()
            .bookkeeping
            .last_reconciliation_attempt
            .elapsed();
        self.chirpstack.status.is_none()
            || (elapsed.is_ok() && (elapsed.unwrap() >= self.error_timeout))
    }

    pub fn needs_reconciliation(&self) -> bool {
        self.chirpstack.status.is_none()
            || match self.chirpstack.status.as_ref().unwrap().state {
                State::Done => self.is_different_generation() || self.is_different_config_hash(),
                _ => self.is_time_to_retry(),
            }
    }

    pub fn status(&self) -> StatusHandlerStatus {
        if self.needs_reconciliation() {
            StatusHandlerStatus::NeedsReconciliation
        } else {
            match self.chirpstack.status.as_ref().unwrap().state {
                State::Done => StatusHandlerStatus::Clean,
                _ => StatusHandlerStatus::HasError,
            }
        }
    }

    pub async fn start_reconciliation(&self) -> Result<(), Error> {
        if self
            .chirpstack
            .status
            .as_ref()
            .and_then(|status| Some(status.state == State::Processing))
            .or(Some(false))
            .unwrap()
        {
            log::warn!("State is Processing at start of reconciliation. Either the controller was killed in process or there is a bug!");
        }
        self.update_remote(Status {
            state: State::Processing,
            errors: vec![],
            bookkeeping: Bookkeeping {
                last_reconciliation_attempt: self.this_reconciliation_attempt,
                last_observed_generation: self.chirpstack.metadata.generation,
                last_observed_config_hash: Some(self.hash.clone()),
                last_observed_workload_type: self
                    .chirpstack
                    .status
                    .as_ref()
                    .and_then(|status| status.bookkeeping.last_observed_workload_type.clone()),
            },
        })
        .await
    }

    pub async fn done_without_errors(&self) -> Result<(), Error> {
        self.update_remote(Status {
            state: State::Done,
            errors: vec![],
            bookkeeping: Bookkeeping {
                last_reconciliation_attempt: self.this_reconciliation_attempt,
                last_observed_generation: self.chirpstack.metadata.generation,
                last_observed_config_hash: Some(self.hash.clone()),
                last_observed_workload_type: Some(
                    self.chirpstack.spec.server.workload.workload_type.clone(),
                ),
            },
        })
        .await
    }

    pub async fn done_with_errors(&self, errors: Vec<String>) -> Result<(), Error> {
        self.update_remote(Status {
            state: State::Error,
            errors,
            bookkeeping: Bookkeeping {
                last_reconciliation_attempt: self.this_reconciliation_attempt,
                last_observed_generation: self.chirpstack.metadata.generation,
                last_observed_config_hash: Some(self.hash.clone()),
                last_observed_workload_type: self
                    .chirpstack
                    .status
                    .as_ref()
                    .and_then(|status| status.bookkeeping.last_observed_workload_type.clone()),
            },
        })
        .await
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
