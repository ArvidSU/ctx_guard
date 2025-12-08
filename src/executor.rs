use std::process::{Command, Output};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("Failed to execute command: {0}")]
    ExecutionError(#[from] std::io::Error),
}

pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub combined_output: String,
}

impl ExecutionResult {
    pub fn from_output(output: Output) -> Self {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined_output = if stderr.is_empty() {
            stdout.clone()
        } else {
            format!("{stdout}\n{stderr}")
        };

        Self {
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            combined_output,
        }
    }

    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

pub fn execute_command(command: &str, args: &[&str]) -> Result<ExecutionResult, ExecutorError> {
    let output = Command::new(command)
        .args(args)
        .output()?;

    Ok(ExecutionResult::from_output(output))
}

pub fn execute_command_string(command_str: &str) -> Result<ExecutionResult, ExecutorError> {
    if command_str.trim().is_empty() {
        return Err(ExecutorError::ExecutionError(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Empty command")
        ));
    }

    // Use shell to execute the command so it handles things like `npx jest` properly
    #[cfg(unix)]
    let output = Command::new("sh")
        .arg("-c")
        .arg(command_str)
        .output()?;

    #[cfg(windows)]
    let output = Command::new("cmd")
        .arg("/C")
        .arg(command_str)
        .output()?;

    Ok(ExecutionResult::from_output(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_result_is_success() {
        let output = Command::new("true").output().unwrap();
        let result = ExecutionResult::from_output(output);
        assert!(result.is_success());
        assert_eq!(result.exit_code, 0);
    }

    #[test]
    fn test_execution_result_is_failure() {
        #[cfg(unix)]
        let output = Command::new("false").output().unwrap();
        #[cfg(windows)]
        let output = Command::new("cmd").arg("/C").arg("exit /b 1").output().unwrap();
        
        let result = ExecutionResult::from_output(output);
        assert!(!result.is_success());
        assert_ne!(result.exit_code, 0);
    }

    #[test]
    fn test_execution_result_combined_output() {
        #[cfg(unix)]
        let output = Command::new("sh").arg("-c").arg("echo stdout; echo stderr >&2").output().unwrap();
        #[cfg(windows)]
        let output = Command::new("cmd").arg("/C").arg("echo stdout && echo stderr >&2").output().unwrap();
        
        let result = ExecutionResult::from_output(output);
        assert!(!result.stdout.is_empty() || !result.stderr.is_empty());
        assert!(!result.combined_output.is_empty());
    }

    #[test]
    fn test_execute_command_string_empty() {
        let result = execute_command_string("");
        assert!(result.is_err());
        
        let result = execute_command_string("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_command_string_success() {
        #[cfg(unix)]
        let result = execute_command_string("echo hello");
        #[cfg(windows)]
        let result = execute_command_string("echo hello");
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_success());
    }

    #[test]
    fn test_execute_command_string_failure() {
        #[cfg(unix)]
        let result = execute_command_string("false");
        #[cfg(windows)]
        let result = execute_command_string("exit /b 1");
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.is_success());
    }

    #[test]
    fn test_execute_command() {
        #[cfg(unix)]
        let result = execute_command("echo", &["test"]);
        #[cfg(windows)]
        let result = execute_command("echo", &["test"]);
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_success());
    }
}

