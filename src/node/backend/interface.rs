use crate::node::config::NodeConfigFile;
use anyhow::Result;
use axum::async_trait;
use serde::{Deserialize, Serialize};
use shared_lib::{command::SystemCommandExecutor, file::SystemFileAccessor};

use super::{
    super::state::NodeState,
    systemd::{SystemdBackend, SystemdCommand},
    uci::{UciBackend, UciCommand},
};

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

pub fn get_backend_impl(backend: BackendType, config: &NodeConfigFile) -> Box<dyn Backend> {
    let executor = SystemCommandExecutor;
    match backend {
        BackendType::Systemd => Box::new(SystemdBackend::new(
            &config.systemd,
            SystemdCommand::new(executor),
            Box::new(SystemCommandExecutor),
            Box::new(SystemFileAccessor),
        )),
        BackendType::Uci => Box::new(UciBackend::new(&config.uci, UciCommand::new(executor))),
    }
}
