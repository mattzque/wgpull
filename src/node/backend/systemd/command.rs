use anyhow::Result;
use shared_lib::command::CommandExecutor;

pub struct SystemdCommand<T: CommandExecutor> {
    executor: T,
}

impl<T: CommandExecutor> SystemdCommand<T> {
    pub fn new(executor: T) -> SystemdCommand<T> {
        Self { executor }
    }

    pub async fn service_is_running(&self, service: &str) -> bool {
        self.executor
            .execute_with_args("systemctl", &["is-active", service])
            .await
            .is_ok()
    }

    pub async fn service_is_enabled(&self, service: &str) -> bool {
        self.executor
            .execute_with_args("systemctl", &["is-enabled", service])
            .await
            .is_ok()
    }

    pub async fn service_is_running_and_enabled(&self, service: &str) -> bool {
        self.service_is_enabled(service).await && self.service_is_running(service).await
    }

    pub async fn delete_wireguard_interface(&self, interface: &str) -> Result<()> {
        self.executor
            .execute_with_args("networkctl", &["delete", interface])
            .await?;
        Ok(())
    }

    pub async fn networkd_reload(&self) -> Result<()> {
        self.executor
            .execute_with_args("networkctl", &["reload"])
            .await?;
        Ok(())
    }
}
