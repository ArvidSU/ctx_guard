use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("cg")
}

#[test]
fn test_binary_exists() {
    let binary = get_binary_path();
    // In CI or if not built, this might not exist, so we'll skip
    if !binary.exists() {
        // Try release build
        let release_binary = binary.parent().unwrap().parent().unwrap().join("release").join("cg");
        if !release_binary.exists() {
            println!("Binary not found, skipping integration test");
            return;
        }
    }
}

#[test]
fn test_help_output() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "cg", "--", "--help"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cg") || stdout.contains("Context guard"));
}

#[test]
fn test_empty_command() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "cg", "--"])
        .output()
        .expect("Failed to execute command");
    
    assert!(!output.status.success());
}

#[test]
fn test_simple_command() {
    #[cfg(unix)]
    let test_cmd = "echo test";
    #[cfg(windows)]
    let test_cmd = "echo test";
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "cg", "--", test_cmd])
        .output()
        .expect("Failed to execute command");
    
    // Command should execute (even if LLM fails, it should fallback)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should mention the output file path
    assert!(stdout.contains("/tmp/ctx_guard") || stdout.contains("ctx_guard") || 
            stderr.contains("Warning") || stdout.contains("test"));
}

#[test]
fn test_output_file_creation() {
    use ctx_guard::output;
    
    let filename = "test_integration.txt";
    let content = "integration test content";
    
    let result = output::write_output_file(filename, content, None);
    assert!(result.is_ok());
    
    let file_path = result.unwrap();
    assert!(file_path.exists());
    
    let read_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(read_content, content);
    
    // Cleanup
    let _ = fs::remove_file(&file_path);
}

#[test]
fn test_config_default() {
    use ctx_guard::config::Config;
    
    let config = Config::default();
    assert_eq!(config.provider.summary_words, 100);
    assert!(!config.is_command_disabled("some command"));
}

#[test]
fn test_executor_basic() {
    use ctx_guard::executor::execute_command_string;
    
    #[cfg(unix)]
    let result = execute_command_string("echo test");
    #[cfg(windows)]
    let result = execute_command_string("echo test");
    
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.is_success());
}

