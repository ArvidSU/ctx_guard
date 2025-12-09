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
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_prompt")]
    pub prompt: String,
    #[serde(default = "default_summary_words")]
    pub summary_words: u32,
    #[serde(default = "default_output_length_threshold")]
    pub output_length_threshold: u32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            r#type: default_provider_type(),
            url: default_provider_url(),
            model: default_model(),
            prompt: default_prompt(),
            summary_words: default_summary_words(),
            output_length_threshold: default_output_length_threshold(),
        }
    }
}

fn default_provider_type() -> String {
    "lmstudio".to_string()
}

fn default_provider_url() -> String {
    "http://127.0.0.1:1234".to_string()
}

fn default_model() -> String {
    "local-model".to_string()
}

fn default_prompt() -> String {
    r#"You are a command output analyzer that provides concise, actionable summaries for AI agents.

${recent_commands}

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

fn default_output_length_threshold() -> u32 {
    default_summary_words()
}

fn default_clean_up_days() -> u32 {
    5
}

fn default_command_context_minutes() -> u32 {
    0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub commands: HashMap<String, CommandOverride>,
    #[serde(default = "default_clean_up_days")]
    pub clean_up_days: u32,
    #[serde(default = "default_command_context_minutes")]
    pub command_context_minutes: u32,
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
                model: default_model(),
                prompt: default_prompt(),
                summary_words: default_summary_words(),
                output_length_threshold: default_output_length_threshold(),
            },
            commands: HashMap::new(),
            clean_up_days: default_clean_up_days(),
            command_context_minutes: default_command_context_minutes(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from_path(None)
    }

    pub fn load_from_path(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let config_path = config_path.unwrap_or_else(get_config_path);
        
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

    /// Returns the minimum output length (in words) required before we attempt
    /// to generate a summary. This is always at least the configured summary length
    /// to avoid summarizing outputs that are already shorter than the summary.
    pub fn get_output_length_threshold(&self, command: &str) -> u32 {
        let summary_words = self.get_summary_words(command);
        self.provider
            .output_length_threshold
            .max(summary_words)
    }

    pub fn is_command_disabled(&self, command: &str) -> bool {
        if let Some(override_config) = self.commands.get(command) {
            // In TOML, `command = false` means "disabled" (don't generate summary)
            matches!(override_config, CommandOverride::Disabled(false))
        } else {
            false
        }
    }

    pub fn format_prompt(&self, command: &str, exit_code: i32, output: &str, summary_words: u32, recent_commands: Option<&[(String, i32)]>) -> String {
        let recent_commands_text = if let Some(commands) = recent_commands {
            if commands.is_empty() {
                String::new()
            } else {
                let commands_list: Vec<String> = commands.iter()
                    .map(|(cmd, code)| {
                        let status = if *code == 0 { "succeeded" } else { "failed" };
                        format!("- {}, {}", cmd, status)
                    })
                    .collect();
                format!("recently run commands:\n{}\n\n", commands_list.join("\n"))
            }
        } else {
            String::new()
        };

        self.provider.prompt
            .replace("${recent_commands}", &recent_commands_text)
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
        assert_eq!(config.model, "local-model");
        assert!(config.prompt.contains("${command}"));
        assert!(config.prompt.contains("${exit_code}"));
        assert!(config.prompt.contains("${output}"));
        assert!(config.prompt.contains("${summary_words}"));
        assert_eq!(config.summary_words, 100);
        assert_eq!(config.output_length_threshold, 100);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.provider.r#type, "lmstudio");
        assert_eq!(config.provider.model, "local-model");
        assert_eq!(config.provider.summary_words, 100);
        assert_eq!(config.provider.output_length_threshold, 100);
        assert!(config.commands.is_empty());
        assert_eq!(config.clean_up_days, 5);
    }

    #[test]
    fn test_format_prompt() {
        let config = Config::default();
        let prompt = config.format_prompt("echo hello", 0, "hello", 50, None);
        
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
    fn test_format_prompt_with_recent_commands() {
        let config = Config::default();
        let recent = vec![
            ("cd workspace".to_string(), 0),
            ("ls".to_string(), 0),
            ("npx jest".to_string(), 1),
        ];
        let prompt = config.format_prompt("npm run build", 0, "output", 50, Some(&recent));
        
        assert!(prompt.contains("recently run commands"));
        assert!(prompt.contains("cd workspace"));
        assert!(prompt.contains("succeeded"));
        assert!(prompt.contains("npx jest"));
        assert!(prompt.contains("failed"));
        assert!(prompt.contains("npm run build"));
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
    fn test_get_output_length_threshold_defaults_to_summary_length() {
        let config = Config::default();
        assert_eq!(config.get_output_length_threshold("any command"), 100);
    }

    #[test]
    fn test_get_output_length_threshold_respects_maximum() {
        let mut config = Config::default();
        config.provider.summary_words = 150;
        config.provider.output_length_threshold = 120;
        // Threshold should never go below the summary length
        assert_eq!(config.get_output_length_threshold("any command"), 150);

        // If the configured threshold is higher, we keep it
        config.provider.output_length_threshold = 200;
        assert_eq!(config.get_output_length_threshold("any command"), 200);
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
model = "custom-model"
summary_words = 50
output_length_threshold = 75

[commands]
"npx jest".summary_words = 200
"curl -v https://example.com" = false
"#;
        
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.provider.url, "http://localhost:8080");
        assert_eq!(config.provider.model, "custom-model");
        assert_eq!(config.provider.summary_words, 50);
        assert_eq!(config.provider.output_length_threshold, 75);
        // Even with a lower threshold, we enforce the summary length floor
        assert_eq!(config.get_output_length_threshold("npx jest"), 200);
        assert_eq!(config.get_summary_words("npx jest"), 200);
        assert!(config.is_command_disabled("curl -v https://example.com"));
    }
}

