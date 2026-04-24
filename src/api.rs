// api.rs
//
// Purpose: OpenAI-compatible HTTP client for LLM completions
//
// This module:
// - Sends requests to OpenAI-compatible endpoints
// - Handles request/response serialization
// - Supports both single-turn and multi-turn conversations

use serde::{Deserialize, Serialize};

/// API message format
#[derive(Debug, Clone, Serialize)]
pub struct ApiMessage {
    pub role: &'static str,
    pub content: String,
}

/// Send a completion request with conversation history.
pub async fn complete_with_history(
    endpoint: &str,
    model: Option<&str>,
    messages: Vec<ApiMessage>,
    timeout: u64,
) -> Result<String, Error> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()?;

    let mut body = serde_json::json!({
        "messages": messages
    });

    if let Some(m) = model {
        body["model"] = serde_json::json!(m);
    }

    let url = format!("{}/v1/chat/completions", endpoint);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
        return Err(Error::Http(format!("{}: {}", status, text)));
    }

    let completion: CompletionResponse = serde_json::from_str(&text)
        .map_err(|e| Error::Parse(format!("Failed to parse response: {}", e)))?;

    completion
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| Error::Parse("No choices in response".into()))
}

/// Send a single-turn completion request.
pub async fn complete(
    endpoint: &str,
    model: Option<&str>,
    prompt: &str,
    timeout: u64,
) -> Result<String, Error> {
    let messages = vec![ApiMessage {
        role: "user",
        content: prompt.to_string(),
    }];
    complete_with_history(endpoint, model, messages, timeout).await
}

#[derive(Debug)]
pub enum Error {
    Http(String),
    Parse(String),
    Network(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Network(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Parse(e.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(s) => write!(f, "HTTP error: {}", s),
            Error::Parse(s) => write!(f, "Parse error: {}", s),
            Error::Network(s) => write!(f, "Network error: {}", s),
        }
    }
}

#[derive(Debug, Deserialize)]
struct CompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}