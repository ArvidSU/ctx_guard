use chrono::{Local, NaiveDateTime, TimeZone};
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

/// Clean up old files from the output directory that are older than the specified number of days.
/// Files that don't match the expected naming pattern are skipped.
/// Errors during cleanup are logged but don't cause the function to fail.
pub fn cleanup_old_files(days: u32) {
    let output_dir = match ensure_output_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Warning: Failed to access output directory for cleanup: {}", e);
            return;
        }
    };

    let cutoff_time = Local::now() - chrono::Duration::days(days as i64);
    
    let entries = match fs::read_dir(&output_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("Warning: Failed to read output directory for cleanup: {}", e);
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Failed to read directory entry during cleanup: {}", e);
                continue;
            }
        };

        let path = entry.path();
        
        // Skip if not a file
        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => {
                eprintln!("Warning: Skipping file with invalid name: {:?}", path);
                continue;
            }
        };

        // Parse timestamp from filename
        // Format: {command_slug}_{YYYYMMDD_HHMMSS}.txt
        // The timestamp is always the last two underscore-separated parts before .txt
        if !filename.ends_with(".txt") {
            continue;
        }

        let without_ext = &filename[..filename.len() - 4];
        let parts: Vec<&str> = without_ext.split('_').collect();
        
        // Need at least 3 parts: command_slug, date (YYYYMMDD), time (HHMMSS)
        if parts.len() < 3 {
            // Doesn't match expected pattern, skip
            continue;
        }

        // Get the last two parts (date and time)
        let date_part = parts[parts.len() - 2];
        let time_part = parts[parts.len() - 1];
        
        // Verify they match the expected format (8 digits for date, 6 digits for time)
        if date_part.len() != 8 || time_part.len() != 6 {
            continue;
        }
        
        // Check if they're numeric
        if !date_part.chars().all(|c| c.is_ascii_digit()) || 
           !time_part.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        
        let timestamp_str = format!("{}_{}", date_part, time_part);
        
        let file_time = match NaiveDateTime::parse_from_str(&timestamp_str, "%Y%m%d_%H%M%S") {
            Ok(dt) => dt,
            Err(_) => {
                // Doesn't match expected timestamp format, skip
                continue;
            }
        };

        // Convert to Local DateTime for comparison
        let file_datetime = match Local.from_local_datetime(&file_time) {
            chrono::LocalResult::Single(dt) => dt,
            _ => {
                eprintln!("Warning: Invalid datetime for file: {}", filename);
                continue;
            }
        };

        // Delete if older than cutoff
        if file_datetime < cutoff_time {
            if let Err(e) = fs::remove_file(&path) {
                eprintln!("Warning: Failed to delete old file {}: {}", filename, e);
            }
        }
    }
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

    #[test]
    fn test_cleanup_old_files_deletes_old() {
        let dir = ensure_output_dir().unwrap();
        
        // Create a file with an old timestamp (10 days ago)
        let old_date = Local::now() - chrono::Duration::days(10);
        let old_timestamp = old_date.format("%Y%m%d_%H%M%S").to_string();
        let old_filename = format!("test_command_{}.txt", old_timestamp);
        let old_path = dir.join(&old_filename);
        fs::write(&old_path, "old content").unwrap();
        assert!(old_path.exists());
        
        // Create a file with a recent timestamp (1 day ago)
        let recent_date = Local::now() - chrono::Duration::days(1);
        let recent_timestamp = recent_date.format("%Y%m%d_%H%M%S").to_string();
        let recent_filename = format!("test_command_{}.txt", recent_timestamp);
        let recent_path = dir.join(&recent_filename);
        fs::write(&recent_path, "recent content").unwrap();
        assert!(recent_path.exists());
        
        // Clean up files older than 5 days
        cleanup_old_files(5);
        
        // Old file should be deleted
        assert!(!old_path.exists(), "Old file should have been deleted");
        
        // Recent file should still exist
        assert!(recent_path.exists(), "Recent file should still exist");
        
        // Cleanup test files
        let _ = fs::remove_file(&recent_path);
    }

    #[test]
    fn test_cleanup_old_files_preserves_recent() {
        let dir = ensure_output_dir().unwrap();
        
        // Create a file with a recent timestamp (2 days ago)
        let recent_date = Local::now() - chrono::Duration::days(2);
        let recent_timestamp = recent_date.format("%Y%m%d_%H%M%S").to_string();
        let recent_filename = format!("test_command_{}.txt", recent_timestamp);
        let recent_path = dir.join(&recent_filename);
        fs::write(&recent_path, "recent content").unwrap();
        assert!(recent_path.exists());
        
        // Clean up files older than 5 days
        cleanup_old_files(5);
        
        // Recent file should still exist
        assert!(recent_path.exists(), "Recent file should still exist");
        
        // Cleanup test file
        let _ = fs::remove_file(&recent_path);
    }

    #[test]
    fn test_cleanup_old_files_skips_invalid_names() {
        let dir = ensure_output_dir().unwrap();
        
        // Create files with invalid names
        let invalid1 = dir.join("invalid_file.txt");
        let invalid2 = dir.join("no_timestamp.txt");
        let invalid3 = dir.join("wrong_format_20240101.txt");
        
        fs::write(&invalid1, "content").unwrap();
        fs::write(&invalid2, "content").unwrap();
        fs::write(&invalid3, "content").unwrap();
        
        assert!(invalid1.exists());
        assert!(invalid2.exists());
        assert!(invalid3.exists());
        
        // Clean up files older than 5 days
        cleanup_old_files(5);
        
        // Invalid files should still exist (they should be skipped)
        assert!(invalid1.exists(), "Invalid file should be skipped");
        assert!(invalid2.exists(), "Invalid file should be skipped");
        assert!(invalid3.exists(), "Invalid file should be skipped");
        
        // Cleanup test files
        let _ = fs::remove_file(&invalid1);
        let _ = fs::remove_file(&invalid2);
        let _ = fs::remove_file(&invalid3);
    }

    #[test]
    fn test_cleanup_old_files_handles_empty_directory() {
        // This should not panic or error
        cleanup_old_files(5);
    }

    #[test]
    fn test_cleanup_old_files_exact_cutoff() {
        let dir = ensure_output_dir().unwrap();
        
        // Create a file just under the cutoff (4 days and 23 hours ago, so it's less than 5 days old)
        // This ensures the file is definitely within the cutoff when cleanup runs
        let cutoff_date = Local::now() - chrono::Duration::days(4) - chrono::Duration::hours(23);
        let cutoff_timestamp = cutoff_date.format("%Y%m%d_%H%M%S").to_string();
        let cutoff_filename = format!("test_command_{}.txt", cutoff_timestamp);
        let cutoff_path = dir.join(&cutoff_filename);
        fs::write(&cutoff_path, "cutoff content").unwrap();
        assert!(cutoff_path.exists());
        
        // Clean up files older than 5 days (this file is less than 5 days old, so it should be kept)
        cleanup_old_files(5);
        
        // File within cutoff should still exist (we delete files OLDER than the cutoff)
        assert!(cutoff_path.exists(), "File within cutoff should still exist");
        
        // Cleanup test file
        let _ = fs::remove_file(&cutoff_path);
    }
}

