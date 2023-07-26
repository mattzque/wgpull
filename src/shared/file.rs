use std::fs::Permissions;

use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait FileAccessor: Send + Sync {
    async fn write(&self, path: &str, content: &str) -> Result<()>;
    async fn read(&self, path: &str) -> Result<String>;
    async fn set_permissions(&self, path: &str, permissions: Permissions) -> Result<()>;
}

pub struct SystemFileAccessor;

#[async_trait]
impl FileAccessor for SystemFileAccessor {
    async fn write(&self, path: &str, content: &str) -> Result<()> {
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    async fn read(&self, path: &str) -> Result<String> {
        let content = tokio::fs::read_to_string(path).await?;
        Ok(content)
    }

    async fn set_permissions(&self, path: &str, permissions: Permissions) -> Result<()> {
        tokio::fs::set_permissions(path, permissions).await?;
        Ok(())
    }
}
