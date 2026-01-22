use crate::paths::config_path;
use crate::types::Delimiter;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub defaults: Defaults,
    pub models: HashMap<String, ModelConfig>,
    #[serde(default)]
    pub prompts: HashMap<String, PresetConfig>,
    #[serde(default)]
    pub history: HistoryConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Defaults {
    pub model: String,
    pub prompt: String,
    pub minimal: Option<bool>,
    pub strip_thinking: Option<bool>,
    #[serde(default)]
    pub thinking_delimiters: Vec<Delimiter>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PresetConfig {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HistoryConfig {
    pub enabled: Option<bool>,
    pub max_entries: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    pub endpoint: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub api_key: Option<String>,
    pub api_key_command: Option<String>,
    pub options: Option<String>,
    pub strip_thinking: Option<bool>,
    #[serde(default)]
    pub thinking_delimiters: Vec<Delimiter>,
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("failed to parse config at {}", path.display()))?;
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &Config) -> Result<()> {
    if !config.models.contains_key(&config.defaults.model) {
        bail!("defaults.model '{}' not found in [models]", config.defaults.model);
    }
    if !config.prompts.contains_key(&config.defaults.prompt) {
        bail!(
            "defaults.prompt '{}' not found in [prompts]",
            config.defaults.prompt
        );
    }
    if let Some(max_entries) = config.history.max_entries {
        if max_entries == 0 {
            bail!("history.max_entries must be greater than zero");
        }
    }
    for (name, model) in &config.models {
        if model.api_key.is_some() && model.api_key_command.is_some() {
            bail!("model '{name}' has both api_key and api_key_command set");
        }
        if model.api_key.is_none() && model.api_key_command.is_none() {
            bail!("model '{name}' must define api_key or api_key_command");
        }
        if let Some(options) = &model.options {
            let value: serde_json::Value = serde_json::from_str(options)
                .with_context(|| format!("model '{name}' options is not valid JSON"))?;
            if !value.is_object() {
                bail!("model '{name}' options must be a JSON object");
            }
        }
        if model.endpoint.trim().is_empty() {
            bail!("model '{name}' endpoint must not be empty");
        }
        if model.model.trim().is_empty() {
            bail!("model '{name}' model id must not be empty");
        }
    }
    Ok(())
}

pub fn write_default_config() -> Result<std::path::PathBuf> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("config already exists at {}", path.display()))?;
    file.write_all(DEFAULT_CONFIG.as_bytes())?;
    Ok(path)
}

const DEFAULT_CONFIG: &str = r#"[defaults]
model = "openai"
prompt = "default"
minimal = false
strip_thinking = false
thinking_delimiters = [
  { start = "<think>", end = "</think>" },
]

[history]
enabled = false
max_entries = 50

[prompts.default]
text = "You are a helpful assistant."

[models.openai]
endpoint = "https://api.openai.com"
model = "gpt-4o-mini"
system_prompt = ""
api_key = "YOUR_API_KEY"
# api_key_command = "pass show openai/api-key"
# options = "{\"reasoning\":{\"enabled\":true}}"
# strip_thinking = false
"#;
