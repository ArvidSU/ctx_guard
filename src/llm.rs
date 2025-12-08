use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to parse JSON response: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("No content in LLM response")]
    NoContent,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

pub struct LlmClient {
    client: Client,
    base_url: String,
}

impl LlmClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn summarize(&self, model: &str, prompt: &str) -> Result<String, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        
        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.7,
            max_tokens: 500,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LlmError::RequestError(
                reqwest::Error::from(response.error_for_status().unwrap_err())
            ));
        }

        let chat_response: ChatResponse = response.json().await?;
        
        if let Some(choice) = chat_response.choices.first() {
            Ok(choice.message.content.trim().to_string())
        } else {
            Err(LlmError::NoContent)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_client_new() {
        let client = LlmClient::new("http://127.0.0.1:1234");
        assert_eq!(client.base_url, "http://127.0.0.1:1234");
    }

    #[test]
    fn test_llm_client_new_with_trailing_slash() {
        let client = LlmClient::new("http://127.0.0.1:1234/");
        assert_eq!(client.base_url, "http://127.0.0.1:1234");
    }

    #[test]
    fn test_llm_client_new_with_path() {
        let client = LlmClient::new("http://127.0.0.1:1234/v1");
        assert_eq!(client.base_url, "http://127.0.0.1:1234/v1");
    }
}

