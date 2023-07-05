use anyhow::Result;
use serde::{Deserialize, Serialize};
use shared_lib::command::SystemCommandExecutor;
use crate::node::config::NodeConfigFile;

use super::{super::state::NodeState, uci::{UciBackend, UciCommand}, systemd::{SystemdBackend, SystemdCommand}};

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BackendType {
    Systemd,
    Uci,
}

pub trait Backend {
    /// Check for compatibility of the backend with the current system.
    fn is_compatible(&self) -> bool;

    /// Updates the local state of the system with the given node state.
    /// This returns true if the state of the system has changed.
    fn update_local_state(&self, state: &NodeState) -> Result<bool>;

    /// Gets the hostname of the local system.
    fn get_hostname(&self) -> Result<String>;
}

pub fn get_backend_impl(backend: BackendType, config: &NodeConfigFile) -> Box<dyn Backend> {
    let executor = SystemCommandExecutor::default();
    match backend {
        BackendType::Systemd => Box::new(SystemdBackend::new(&config.systemd, SystemdCommand::new(executor))),
        BackendType::Uci => Box::new(UciBackend::new(&config.uci, UciCommand::new(executor))),
    }
}