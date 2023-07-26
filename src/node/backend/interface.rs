use std::sync::Arc;

use crate::node::config::NodeConfigFile;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shared_lib::{command::CommandExecutor, file::FileAccessor};

use super::{super::state::NodeState, systemd::SystemdBackend, uci::UciBackend};

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BackendType {
    Systemd,
    Uci,
}

#[async_trait]
pub trait Backend {
    /// Check for compatibility of the backend with the current system.
    async fn is_compatible(&self) -> bool;

    /// Updates the local state of the system with the given node state.
    /// This returns true if the state of the system has changed.
    async fn update_local_state(&self, state: &NodeState) -> Result<bool>;

    /// Gets the hostname of the local system.
    async fn get_hostname(&self) -> Result<String>;
}

pub fn get_backend_impl(
    backend: BackendType,
    config: &NodeConfigFile,
    executor: Arc<dyn CommandExecutor>,
    file_accessor: Arc<dyn FileAccessor>,
) -> Box<dyn Backend> {
    match backend {
        BackendType::Systemd => Box::new(SystemdBackend::new(
            &config.systemd,
            executor,
            file_accessor,
        )),
        BackendType::Uci => Box::new(UciBackend::new(&config.uci, executor)),
    }
}
