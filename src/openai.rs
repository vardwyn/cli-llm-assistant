use crate::config::{Config, ModelConfig};
use crate::types::Delimiter;
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug)]
pub struct ChatResult {
    pub content: String,
    pub thinking_delimiters: Vec<Delimiter>,
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
    #[serde(default)]
    reasoning: Option<Value>,
    #[serde(default)]
    reasoning_details: Option<Vec<Value>>,
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

    let mut request = serde_json::Map::new();
    request.insert("model".to_string(), Value::String(model_cfg.model.clone()));
    request.insert("messages".to_string(), serde_json::to_value(messages)?);

    if let Some(options) = model_cfg.options.as_deref() {
        let value: Value = serde_json::from_str(options)
            .with_context(|| format!("options for model '{}' is not valid JSON", model_name))?;
        let object = value.as_object().with_context(|| {
            format!("options for model '{}' must be a JSON object", model_name)
        })?;
        for (key, value) in object {
            if request.contains_key(key) {
                bail!("options for model '{}' cannot override '{key}'", model_name);
            }
            request.insert(key.clone(), value.clone());
        }
    }

    let url = build_chat_url(&model_cfg.endpoint);
    let response = client
        .post(url)
        .bearer_auth(api_key)
        .json(&Value::Object(request))
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
    let message = parsed
        .choices
        .get(0)
        .map(|choice| &choice.message)
        .context("missing assistant message in response")?;
    let mut content = message.content.clone().unwrap_or_default();
    let reasoning = extract_reasoning(message);

    let thinking_delimiters = if !model_cfg.thinking_delimiters.is_empty() {
        model_cfg.thinking_delimiters
    } else {
        config.defaults.thinking_delimiters.clone()
    };

    if let Some(reasoning_text) = reasoning {
        if !contains_any_delimiter(&content, &thinking_delimiters) {
            let wrapped = wrap_reasoning(&reasoning_text, &thinking_delimiters);
            if content.trim().is_empty() {
                content = wrapped;
            } else if wrapped.trim().is_empty() {
                content = reasoning_text + "\n" + &content;
            } else {
                content = wrapped + "\n" + &content;
            }
        }
    }

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

fn extract_reasoning(message: &AssistantMessage) -> Option<String> {
    if let Some(value) = &message.reasoning {
        if let Some(text) = value.as_str() {
            if !text.trim().is_empty() {
                return Some(text.to_string());
            }
        } else if !value.is_null() {
            let text = value.to_string();
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }

    let details = message.reasoning_details.as_ref()?;
    let mut parts = Vec::new();
    for item in details {
        if let Some(text) = item.get("text").and_then(|value| value.as_str()) {
            if !text.trim().is_empty() {
                parts.push(text.to_string());
            }
        } else if let Some(text) = item.as_str() {
            if !text.trim().is_empty() {
                parts.push(text.to_string());
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

fn contains_any_delimiter(content: &str, delimiters: &[Delimiter]) -> bool {
    delimiters.iter().any(|delim| {
        (!delim.start.is_empty() && content.contains(&delim.start))
            || (!delim.end.is_empty() && content.contains(&delim.end))
    })
}

fn wrap_reasoning(reasoning: &str, delimiters: &[Delimiter]) -> String {
    for delim in delimiters {
        if !delim.start.is_empty() && !delim.end.is_empty() {
            return format!("{}{}{}", delim.start, reasoning, delim.end);
        }
    }
    for delim in delimiters {
        if !delim.end.is_empty() {
            return format!("{}{}", reasoning, delim.end);
        }
    }
    reasoning.to_string()
}
