use crate::config::{Config, ModelConfig};
use crate::types::Delimiter;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ChatResult {
    pub content: String,
    pub thinking_delimiters: Vec<Delimiter>,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ErrorDetails,
}

#[derive(Debug, Deserialize)]
struct ErrorDetails {
    message: String,
}

pub async fn chat_completion(
    client: &Client,
    config: &Config,
    model_name: &str,
    preset_name: &str,
    user_input: &str,
    api_key: &str,
) -> Result<ChatResult> {
    let model_cfg = resolve_model(config, model_name)?;
    let preset_cfg = config
        .prompts
        .get(preset_name)
        .with_context(|| format!("preset prompt '{}' not found in [prompts]", preset_name))?;
    let preset_text = preset_cfg.text.clone();

    let mut messages = Vec::new();
    if let Some(model_system_prompt) = model_cfg.system_prompt.as_deref() {
        if !model_system_prompt.trim().is_empty() {
            messages.push(Message {
                role: "system".to_string(),
                content: model_system_prompt.to_string(),
            });
        }
    }
    if !preset_text.trim().is_empty() {
        messages.push(Message {
            role: "system".to_string(),
            content: preset_text,
        });
    }
    messages.push(Message {
        role: "user".to_string(),
        content: user_input.to_string(),
    });

    let request = ChatRequest {
        model: model_cfg.model.clone(),
        messages,
    };

    let url = build_chat_url(&model_cfg.endpoint);
    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if let Ok(parsed) = serde_json::from_str::<ErrorResponse>(&body) {
            bail!("api error ({}): {}", status, parsed.error.message);
        }
        bail!("api error ({}): {}", status, body);
    }

    let parsed: ChatResponse = response.json().await?;
    let content = parsed
        .choices
        .get(0)
        .and_then(|choice| choice.message.content.clone())
        .unwrap_or_default();

    let thinking_delimiters = if !model_cfg.thinking_delimiters.is_empty() {
        model_cfg.thinking_delimiters
    } else {
        config.defaults.thinking_delimiters.clone()
    };

    Ok(ChatResult {
        content,
        thinking_delimiters,
    })
}

fn resolve_model(config: &Config, name: &str) -> Result<ModelConfig> {
    config
        .models
        .get(name)
        .cloned()
        .with_context(|| format!("model '{}' not found in [models]", name))
}

fn build_chat_url(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
}
