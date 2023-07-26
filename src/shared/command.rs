use async_trait::async_trait;
use log::debug;
use std::process::Stdio;
use tokio::io::{AsyncWriteExt, Error, ErrorKind, Result};
use tokio::process::Command;

/// CommandExecutor Return type: (stdout, stderr)
type Output = (String, String);

/// Trait for system command execution that wraps `std::process::Command`.
/// This is used to allow for mocking in tests.
#[async_trait]
pub trait CommandExecutor: Send + Sync {
    /// Runs a single executable without arguments.
    async fn execute(&self, command: &str) -> Result<Output>;

    /// Runs a single executable with arguments.
    async fn execute_with_args(&self, command: &str, args: &[&str]) -> Result<Output>;

    /// Runs a single executable with arguments, passing the provided stdin.
    async fn execute_with_args_and_io(
        &self,
        command: &str,
        args: &[&str],
        stdin: &str,
    ) -> Result<Output>;
}

pub struct SystemCommandExecutor;

async fn decode_output(child: tokio::process::Child) -> Result<Output> {
    let output = child.wait_with_output().await?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        debug!("decode_output stdout: {:?}", stdout);
        debug!("decode_output stderr: {:?}", stderr);
        Ok((stdout, stderr))
    } else {
        Err(Error::new(
            ErrorKind::Other,
            "Command returned non-zero exit code.",
        ))
    }
}

#[async_trait]
impl CommandExecutor for SystemCommandExecutor {
    async fn execute(&self, command: &str) -> Result<Output> {
        debug!("execute({:?})", command);
        let child = Command::new(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        decode_output(child).await
    }

    async fn execute_with_args(&self, command: &str, args: &[&str]) -> Result<Output> {
        debug!("execute_with_args({:?}, {:?})", command, args);
        let child = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        decode_output(child).await
    }

    async fn execute_with_args_and_io(
        &self,
        command: &str,
        args: &[&str],
        stdin: &str,
    ) -> Result<Output> {
        debug!("execute_with_args_and_io({:?}, {:?})", command, args);
        let mut child = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin_stream) = child.stdin.take() {
            stdin_stream.write_all(stdin.as_bytes()).await?;
        }

        decode_output(child).await
    }
}
