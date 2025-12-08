use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutputError {
    #[error("Failed to create output directory: {0}")]
    DirectoryError(#[from] std::io::Error),
}

const OUTPUT_DIR: &str = "/tmp/ctx_guard";
const METADATA_START: &str = "---CTX_GUARD_METADATA---";
const METADATA_END: &str = "---END_METADATA---";

#[derive(Debug, Clone)]
pub struct CommandMetadata {
    pub command: String,
    pub exit_code: i32,
    pub timestamp: DateTime<Local>,
    pub summary: Option<String>,
}

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

fn format_metadata(metadata: &CommandMetadata) -> String {
    let summary_line = if let Some(ref summary) = metadata.summary {
        format!("summary: {}\n", summary.replace('\n', " ").replace('\r', " "))
    } else {
        "summary: \n".to_string()
    };
    
    format!(
        "{}\ncommand: {}\nexit_code: {}\ntimestamp: {}\n{}\n{}\n",
        METADATA_START,
        metadata.command,
        metadata.exit_code,
        metadata.timestamp.to_rfc3339(),
        summary_line,
        METADATA_END
    )
}

pub fn write_output_file(filename: &str, content: &str, metadata: Option<&CommandMetadata>) -> Result<PathBuf, OutputError> {
    let dir = ensure_output_dir()?;
    let file_path = dir.join(filename);
    
    let file_content = if let Some(meta) = metadata {
        format!("{}\n\n{}", format_metadata(meta), content)
    } else {
        content.to_string()
    };
    
    fs::write(&file_path, file_content)?;
    Ok(file_path)
}

pub fn parse_metadata_from_file(file_path: &Path) -> Option<CommandMetadata> {
    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return None,
    };
    
    if !content.starts_with(METADATA_START) {
        return None;
    }
    
    let metadata_end_pos = content.find(METADATA_END)?;
    let metadata_section = &content[..metadata_end_pos + METADATA_END.len()];
    
    let mut command = None;
    let mut exit_code = None;
    let mut timestamp = None;
    let mut summary = None;
    
    for line in metadata_section.lines() {
        if line.starts_with("command: ") {
            command = Some(line[9..].trim().to_string());
        } else if line.starts_with("exit_code: ") {
            exit_code = Some(line[11..].trim().parse().ok()?);
        } else if line.starts_with("timestamp: ") {
            timestamp = DateTime::parse_from_rfc3339(line[11..].trim())
                .ok()
                .map(|dt| dt.with_timezone(&Local));
        } else if line.starts_with("summary: ") {
            let summary_text = line[9..].trim();
            summary = if summary_text.is_empty() {
                None
            } else {
                Some(summary_text.to_string())
            };
        }
    }
    
    Some(CommandMetadata {
        command: command?,
        exit_code: exit_code?,
        timestamp: timestamp?,
        summary,
    })
}

pub fn get_recent_commands(minutes: u32) -> Vec<(String, i32, DateTime<Local>)> {
    let output_dir = match ensure_output_dir() {
        Ok(dir) => dir,
        Err(_) => {
            eprintln!("DEBUG: Failed to get output directory");
            return Vec::new();
        },
    };
    
    let now = Local::now();
    let cutoff_time = now - chrono::Duration::minutes(minutes as i64);
    
    let entries = match fs::read_dir(&output_dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("DEBUG: Failed to read directory: {}", e);
            return Vec::new();
        },
    };
    
    let mut recent_commands = Vec::new();
    
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        let path = entry.path();
        if !path.is_file() || !path.to_string_lossy().ends_with(".txt") {
            continue;
        }
        
        let metadata = match parse_metadata_from_file(&path) {
            Some(m) => m,
            None => continue,
        };
        
        // Only include commands within the time window
        if metadata.timestamp >= cutoff_time {
            recent_commands.push((metadata.command, metadata.exit_code, metadata.timestamp));
        }
    }
    
    // Sort chronologically (oldest first)
    recent_commands.sort_by_key(|(_, _, timestamp)| *timestamp);
    
    recent_commands
}

pub fn update_output_file_summary(file_path: &PathBuf, summary: &str) -> Result<(), OutputError> {
    let content = fs::read_to_string(file_path)?;
    
    if !content.starts_with(METADATA_START) {
        // File doesn't have metadata, can't update
        return Ok(());
    }
    
    let metadata_end_pos = match content.find(METADATA_END) {
        Some(pos) => pos,
        None => return Ok(()),
    };
    
    let metadata_section = &content[..metadata_end_pos + METADATA_END.len()];
    let output_section = &content[metadata_end_pos + METADATA_END.len()..];
    
    // Parse existing metadata
    let mut command = None;
    let mut exit_code = None;
    let mut timestamp = None;
    
    for line in metadata_section.lines() {
        if line.starts_with("command: ") {
            command = Some(line[9..].trim().to_string());
        } else if line.starts_with("exit_code: ") {
            exit_code = line[11..].trim().parse().ok();
        } else if line.starts_with("timestamp: ") {
            timestamp = DateTime::parse_from_rfc3339(line[11..].trim())
                .ok()
                .map(|dt| dt.with_timezone(&Local));
        }
    }
    
    if let (Some(cmd), Some(code), Some(ts)) = (command, exit_code, timestamp) {
        let updated_metadata = CommandMetadata {
            command: cmd,
            exit_code: code,
            timestamp: ts,
            summary: Some(summary.to_string()),
        };
        
        let updated_content = format!("{}\n\n{}", format_metadata(&updated_metadata), output_section.trim_start_matches('\n'));
        fs::write(file_path, updated_content)?;
    } else {
        // If we can't parse the metadata, we can't update it
        return Ok(());
    }
    
    Ok(())
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
        
        let result = write_output_file(filename, content, None);
        assert!(result.is_ok());
        
        let file_path = result.unwrap();
        assert!(file_path.exists());
        
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);
        
        // Cleanup
        let _ = fs::remove_file(&file_path);
    }

    #[test]
    fn test_write_output_file_with_metadata() {
        let filename = "test_output_metadata.txt";
        let content = "test content";
        let metadata = CommandMetadata {
            command: "echo test".to_string(),
            exit_code: 0,
            timestamp: Local::now(),
            summary: None,
        };
        
        let result = write_output_file(filename, content, Some(&metadata));
        assert!(result.is_ok());
        
        let file_path = result.unwrap();
        assert!(file_path.exists());
        
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert!(read_content.contains(METADATA_START));
        assert!(read_content.contains("command: echo test"));
        assert!(read_content.contains("exit_code: 0"));
        assert!(read_content.contains("test content"));
        
        // Cleanup
        let _ = fs::remove_file(&file_path);
    }

    #[test]
    fn test_parse_metadata_from_file() {
        let filename = "test_parse_metadata.txt";
        let metadata = CommandMetadata {
            command: "ls -la".to_string(),
            exit_code: 0,
            timestamp: Local::now(),
            summary: Some("Listed files".to_string()),
        };
        
        let file_path = write_output_file(filename, "output content", Some(&metadata)).unwrap();
        
        let parsed = parse_metadata_from_file(&file_path);
        assert!(parsed.is_some());
        let parsed_meta = parsed.unwrap();
        assert_eq!(parsed_meta.command, "ls -la");
        assert_eq!(parsed_meta.exit_code, 0);
        assert_eq!(parsed_meta.summary, Some("Listed files".to_string()));
        
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

