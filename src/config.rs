use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

// Include the default config.toml at compile time
const DEFAULT_CONFIG: &str = include_str!("../config.toml");

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse TOML: {0}")]
    ParseError(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default = "default_provider_type")]
    pub r#type: String,
    #[serde(default = "default_provider_url")]
    pub url: String,
    #[serde(default = "default_prompt")]
    pub prompt: String,
    #[serde(default = "default_summary_words")]
    pub summary_words: u32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            r#type: default_provider_type(),
            url: default_provider_url(),
            prompt: default_prompt(),
            summary_words: default_summary_words(),
        }
    }
}

fn default_provider_type() -> String {
    "lmstudio".to_string()
}

fn default_provider_url() -> String {
    "http://127.0.0.1:1234".to_string()
}

fn default_prompt() -> String {
    r#"You are a command output analyzer that provides concise, actionable summaries for AI agents.

Command executed: ${command}
Exit code: ${exit_code}
Output:

${output}

Generate a summary in ${summary_words} words or less following these guidelines:

1. PRIORITIZE ACTIONABLE INFORMATION:
   - If the command failed, identify the root cause and suggest specific fixes
   - Include relevant file paths, line numbers, or error codes when available
   - Highlight what needs attention vs. what succeeded

2. STRUCTURE FOR CLARITY:
   - Start with the outcome (success/failure) and key metrics if relevant
   - Focus on errors, warnings, or unexpected behavior first
   - Mention important details like test results, build status, or data counts

3. BE SPECIFIC:
   - Use exact error messages, file names, or identifiers when critical
   - Avoid vague statements like "something went wrong"
   - Include numbers, percentages, or counts when they provide context

4. FORMAT FOR TERMINAL:
   - Use plain text only (no markdown, no special formatting)
   - Keep it scannable with clear, short sentences
   - If the output is very long, focus on the most important parts

5. NEXT STEPS:
   - If errors exist, suggest concrete actions to resolve them
   - If successful, note any important results or follow-up actions needed

Remember: This summary will help an AI agent decide whether to investigate the full output file or proceed with the next task."#.to_string()
}

fn default_summary_words() -> u32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub commands: HashMap<String, CommandOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommandOverride {
    Disabled(bool),
    SummaryWords { summary_words: u32 },
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderConfig {
                r#type: default_provider_type(),
                url: default_provider_url(),
                prompt: default_prompt(),
                summary_words: default_summary_words(),
            },
            commands: HashMap::new(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = get_config_path();
        
        if !config_path.exists() {
            // Create the config directory if it doesn't exist
            if let Some(config_dir) = config_path.parent() {
                fs::create_dir_all(config_dir)?;
            }
            
            // Write the default config file
            fs::write(&config_path, DEFAULT_CONFIG)?;
        }

        let contents = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn get_summary_words(&self, command: &str) -> u32 {
        if let Some(override_config) = self.commands.get(command) {
            match override_config {
                CommandOverride::Disabled(_) => self.provider.summary_words,
                CommandOverride::SummaryWords { summary_words } => *summary_words,
            }
        } else {
            self.provider.summary_words
        }
    }

    pub fn is_command_disabled(&self, command: &str) -> bool {
        if let Some(override_config) = self.commands.get(command) {
            // In TOML, `command = false` means "disabled" (don't generate summary)
            matches!(override_config, CommandOverride::Disabled(false))
        } else {
            false
        }
    }

    pub fn format_prompt(&self, command: &str, exit_code: i32, output: &str, summary_words: u32) -> String {
        self.provider.prompt
            .replace("${command}", command)
            .replace("${exit_code}", &exit_code.to_string())
            .replace("${output}", output)
            .replace("${summary_words}", &summary_words.to_string())
    }
}

fn get_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::home_dir() {
        config_dir.join(".ctx_guard").join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_default() {
        let config = ProviderConfig::default();
        assert_eq!(config.r#type, "lmstudio");
        assert_eq!(config.url, "http://127.0.0.1:1234");
        assert!(config.prompt.contains("${command}"));
        assert!(config.prompt.contains("${exit_code}"));
        assert!(config.prompt.contains("${output}"));
        assert!(config.prompt.contains("${summary_words}"));
        assert_eq!(config.summary_words, 100);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.provider.r#type, "lmstudio");
        assert_eq!(config.provider.summary_words, 100);
        assert!(config.commands.is_empty());
    }

    #[test]
    fn test_format_prompt() {
        let config = Config::default();
        let prompt = config.format_prompt("echo hello", 0, "hello", 50);
        
        assert!(prompt.contains("echo hello"));
        assert!(prompt.contains("0"));
        assert!(prompt.contains("hello"));
        assert!(prompt.contains("50"));
        assert!(!prompt.contains("${command}"));
        assert!(!prompt.contains("${exit_code}"));
        assert!(!prompt.contains("${output}"));
        assert!(!prompt.contains("${summary_words}"));
    }

    #[test]
    fn test_get_summary_words_default() {
        let config = Config::default();
        assert_eq!(config.get_summary_words("some command"), 100);
    }

    #[test]
    fn test_get_summary_words_override() {
        let mut config = Config::default();
        config.commands.insert(
            "npx jest".to_string(),
            CommandOverride::SummaryWords { summary_words: 200 }
        );
        assert_eq!(config.get_summary_words("npx jest"), 200);
        assert_eq!(config.get_summary_words("other command"), 100);
    }

    #[test]
    fn test_is_command_disabled() {
        let mut config = Config::default();
        assert!(!config.is_command_disabled("some command"));
        
        // In TOML, `command = false` means disabled
        config.commands.insert(
            "curl -v https://example.com".to_string(),
            CommandOverride::Disabled(false)
        );
        assert!(config.is_command_disabled("curl -v https://example.com"));
        assert!(!config.is_command_disabled("other command"));
        
        config.commands.insert(
            "another command".to_string(),
            CommandOverride::Disabled(true)
        );
        assert!(!config.is_command_disabled("another command"));
    }

    #[test]
    fn test_config_deserialize() {
        let toml_str = r#"
[provider]
type = "lmstudio"
url = "http://localhost:8080"
summary_words = 50

[commands]
"npx jest".summary_words = 200
"curl -v https://example.com" = false
"#;
        
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.provider.url, "http://localhost:8080");
        assert_eq!(config.provider.summary_words, 50);
        assert_eq!(config.get_summary_words("npx jest"), 200);
        assert!(config.is_command_disabled("curl -v https://example.com"));
    }
}

