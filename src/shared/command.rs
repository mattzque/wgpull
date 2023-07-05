use std::io::Write;
use std::{
    io::{Error, ErrorKind, Result},
    process::Stdio,
};

/// CommandExecutor Return type: (stdout, stderr)
type Output = (String, String);

/// Trait for system command execution that wraps `std::process::Command`.
/// This is used to allow for mocking in tests.
pub trait CommandExecutor {
    /// Runs a single executable without arguments.
    fn execute(&self, command: &str) -> Result<Output>;

    /// Runs a single executable with arguments.
    fn execute_with_args(&self, command: &str, args: &[&str]) -> Result<Output>;

    /// Runs a single executable with arguments, passing the provided stdin.
    fn execute_with_args_and_io(&self, command: &str, args: &[&str], stdin: &str)
        -> Result<Output>;
}

#[derive(Default)]
pub struct SystemCommandExecutor;

fn decode_output(output: std::process::Output) -> Result<Output> {
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
        let stderr = String::from_utf8(output.stderr)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
        Ok((stdout, stderr))
    } else {
        Err(Error::new(
            ErrorKind::Other,
            "Command returned non-zero exit code.",
        ))
    }
}

impl CommandExecutor for SystemCommandExecutor {
    fn execute(&self, command: &str) -> Result<Output> {
        decode_output(std::process::Command::new(command).output()?)
    }

    fn execute_with_args(&self, command: &str, args: &[&str]) -> Result<Output> {
        decode_output(std::process::Command::new(command).args(args).output()?)
    }

    fn execute_with_args_and_io(
        &self,
        command: &str,
        args: &[&str],
        stdin: &str,
    ) -> Result<Output> {
        let mut child = std::process::Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        child.stdin.as_mut().unwrap().write_all(stdin.as_bytes())?;

        let output = child.wait_with_output()?;

        decode_output(output)
    }
}
