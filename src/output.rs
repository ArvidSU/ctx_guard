use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("Failed to create output directory: {0}")]
    DirectoryError(#[from] std::io::Error),
}

const OUTPUT_DIR: &str = "/tmp/ctx_guard";

pub fn ensure_output_dir() -> Result<PathBuf, OutputError> {
    let dir = Path::new(OUTPUT_DIR);
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }
    Ok(dir.to_path_buf())
}

pub fn generate_output_filename(command: &str) -> String {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let command_slug = command
        .replace(' ', "_")
        .replace('/', "_")
        .replace('\\', "_")
        .replace('|', "_")
        .replace('&', "_")
        .replace(';', "_")
        .replace('>', "_")
        .replace('<', "_")
        .replace('*', "_")
        .replace('?', "_")
        .replace('"', "_")
        .replace('\'', "_")
        .chars()
        .take(50)
        .collect::<String>();
    
    format!("{command_slug}_{timestamp}.txt")
}

pub fn write_output_file(filename: &str, content: &str) -> Result<PathBuf, OutputError> {
    let dir = ensure_output_dir()?;
    let file_path = dir.join(filename);
    fs::write(&file_path, content)?;
    Ok(file_path)
}

pub fn format_fallback_output(output: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = output.lines().collect();
    
    if lines.len() <= max_lines * 2 {
        return output.to_string();
    }
    
    let first_lines: Vec<&str> = lines.iter().take(max_lines).copied().collect();
    let last_lines: Vec<&str> = lines.iter().rev().take(max_lines).rev().copied().collect();
    
    format!(
        "{}\n\n... ({} lines omitted) ...\n\n{}",
        first_lines.join("\n"),
        lines.len() - (max_lines * 2),
        last_lines.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_output_filename() {
        let filename = generate_output_filename("npx jest");
        assert!(filename.starts_with("npx_jest_"));
        assert!(filename.ends_with(".txt"));
        assert!(filename.contains("_"));
    }

    #[test]
    fn test_generate_output_filename_special_chars() {
        let filename = generate_output_filename("curl -v https://example.com");
        assert!(filename.contains("curl"));
        // The space before -v should be replaced with underscore
        assert!(filename.contains("-v") || filename.contains("_v"));
        assert!(!filename.contains(" "));
        assert!(!filename.contains("://"));
    }

    #[test]
    fn test_generate_output_filename_long_command() {
        let long_command = "a".repeat(100);
        let filename = generate_output_filename(&long_command);
        // Should be truncated to 50 chars for command slug
        let parts: Vec<&str> = filename.split('_').collect();
        assert!(parts[0].len() <= 50);
    }

    #[test]
    fn test_format_fallback_output_short() {
        let output = "line1\nline2\nline3";
        let formatted = format_fallback_output(output, 20);
        assert_eq!(formatted, output);
    }

    #[test]
    fn test_format_fallback_output_long() {
        let lines: Vec<String> = (1..=100).map(|i| format!("line{}", i)).collect();
        let output = lines.join("\n");
        let formatted = format_fallback_output(&output, 20);
        
        assert!(formatted.contains("line1"));
        assert!(formatted.contains("line100"));
        assert!(formatted.contains("... (60 lines omitted) ..."));
        assert!(!formatted.contains("line50")); // Should be in omitted section
    }

    #[test]
    fn test_ensure_output_dir() {
        let result = ensure_output_dir();
        assert!(result.is_ok());
        let dir = result.unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    #[test]
    fn test_write_output_file() {
        let filename = "test_output.txt";
        let content = "test content";
        
        let result = write_output_file(filename, content);
        assert!(result.is_ok());
        
        let file_path = result.unwrap();
        assert!(file_path.exists());
        
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);
        
        // Cleanup
        let _ = fs::remove_file(&file_path);
    }
}

