use crate::cli::Cli;
use crate::config::{load_config, Config};
use crate::history::{self, HistoryEntry};
use crate::openai;
use crate::types::Delimiter;
use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use std::io::{self, IsTerminal, Read, Write};

pub async fn run(cli: &Cli) -> Result<()> {
    let config = load_config()?;
    let user_input = resolve_user_input(cli)?;
    let model_name = cli.model.clone().unwrap_or_else(|| config.defaults.model.clone());
    let preset_name = cli
        .prompt
        .clone()
        .unwrap_or_else(|| config.defaults.prompt.clone());
    let strip_thinking = resolve_strip_thinking(cli, &config, &model_name)?;
    let minimal = if cli.minimal {
        true
    } else {
        config.defaults.minimal.unwrap_or(false)
    };

    let stdout_is_tty = io::stdout().is_terminal();
    let show_status = stdout_is_tty && !minimal;
    let colorize = stdout_is_tty && !minimal;
    let status = Status::new(show_status);

    let api_key = resolve_api_key(&config, &model_name).await?;
    let client = reqwest::Client::new();

    status.show("sending")?;
    status.show("waiting for response")?;
    let result = openai::chat_completion(
        &client,
        &config,
        &model_name,
        &preset_name,
        &user_input,
        &api_key,
    )
    .await;

    status.show("receiving response")?;

    match result {
        Ok(result) => {
            if history_enabled(&config) {
                if let Err(err) = record_history(
                    &config,
                    &model_name,
                    &preset_name,
                    &user_input,
                    &result.content,
                ) {
                    eprintln!("warning: failed to write history: {err}");
                }
            }
            status.clear()?;
            let mut output_text = result.content;
            if strip_thinking {
                output_text = strip_thinking_text(&output_text, &result.thinking_delimiters);
            }
            let output = if colorize {
                color_thinking(&output_text, &result.thinking_delimiters)
            } else {
                output_text
            };
            print!("{output}");
            io::stdout().flush()?;
        }
        Err(err) => {
            let _ = status.show("error");
            return Err(err);
        }
    }

    Ok(())
}

fn resolve_user_input(cli: &Cli) -> Result<String> {
    if !cli.args.is_empty() {
        return Ok(cli.args.join(" "));
    }

    if io::stdin().is_terminal() {
        bail!("no user input provided (pass args or pipe stdin)");
    }

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    if buffer.trim().is_empty() {
        bail!("stdin input is empty");
    }
    Ok(buffer)
}

async fn resolve_api_key(config: &Config, model_name: &str) -> Result<String> {
    let model = config
        .models
        .get(model_name)
        .with_context(|| format!("model '{}' not found in [models]", model_name))?;

    if let Some(key) = &model.api_key {
        return Ok(key.clone());
    }

    let command = model
        .api_key_command
        .as_ref()
        .with_context(|| format!("model '{}' has no api_key_command", model_name))?;

    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await
        .with_context(|| format!("failed to run api_key_command: {command}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("api_key_command failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        bail!("api_key_command returned empty output");
    }
    Ok(trimmed.to_string())
}

fn history_enabled(config: &Config) -> bool {
    config.history.enabled.unwrap_or(false)
}

fn history_max_entries(config: &Config) -> usize {
    config.history.max_entries.unwrap_or(50)
}

fn resolve_strip_thinking(cli: &Cli, config: &Config, model_name: &str) -> Result<bool> {
    if cli.strip_thinking {
        return Ok(true);
    }
    let model = config
        .models
        .get(model_name)
        .with_context(|| format!("model '{}' not found in [models]", model_name))?;
    Ok(model
        .strip_thinking
        .unwrap_or(config.defaults.strip_thinking.unwrap_or(false)))
}

fn record_history(
    config: &Config,
    model_name: &str,
    preset_name: &str,
    user_input: &str,
    response: &str,
) -> Result<()> {
    let entry = HistoryEntry {
        timestamp: history::now_timestamp()?,
        model: model_name.to_string(),
        preset_name: preset_name.to_string(),
        user_input: user_input.to_string(),
        response: response.to_string(),
    };
    history::append_entry(entry, history_max_entries(config))
}

pub fn color_thinking(content: &str, delimiters: &[Delimiter]) -> String {
    if delimiters.is_empty() {
        return content.to_string();
    }

    let mut result = String::new();
    let mut index = 0;

    while index < content.len() {
        let mut next_start = None;
        let mut next_start_delim = None;
        let mut next_end = None;
        let mut next_end_delim = None;

        for delim in delimiters {
            if !delim.start.is_empty() {
                if let Some(pos) = content[index..].find(&delim.start) {
                    let abs = index + pos;
                    if next_start.map_or(true, |current| abs < current) {
                        next_start = Some(abs);
                        next_start_delim = Some(delim);
                    }
                }
            }
            if !delim.end.is_empty() {
                if let Some(pos) = content[index..].find(&delim.end) {
                    let abs = index + pos;
                    if next_end.map_or(true, |current| abs < current) {
                        next_end = Some(abs);
                        next_end_delim = Some(delim);
                    }
                }
            }
        }

        let next_start_idx = next_start;
        let next_end_idx = next_end;

        match (next_start_idx, next_end_idx) {
            (None, None) => {
                result.push_str(&content[index..]);
                break;
            }
            (Some(start_idx), Some(end_idx)) if start_idx <= end_idx => {
                result.push_str(&content[index..start_idx]);
                let delim = next_start_delim.expect("start delimiter missing");
                let after_start = start_idx + delim.start.len();
                if let Some(end_rel) = content[after_start..].find(&delim.end) {
                    let end_idx = after_start + end_rel + delim.end.len();
                    let slice = &content[start_idx..end_idx];
                    result.push_str(&format!("{}", slice.dimmed()));
                    index = end_idx;
                } else {
                    let slice = &content[start_idx..];
                    result.push_str(&format!("{}", slice.dimmed()));
                    break;
                }
            }
            (_, Some(end_idx)) => {
                let delim = next_end_delim.expect("end delimiter missing");
                let end_idx = end_idx + delim.end.len();
                let slice = &content[index..end_idx];
                result.push_str(&format!("{}", slice.dimmed()));
                index = end_idx;
            }
            _ => {
                result.push_str(&content[index..]);
                break;
            }
        }
    }

    result
}

pub fn strip_thinking_text(content: &str, delimiters: &[Delimiter]) -> String {
    if delimiters.is_empty() {
        return content.to_string();
    }

    let mut result = String::new();
    let mut index = 0;

    while index < content.len() {
        let mut next_start = None;
        let mut next_start_delim = None;
        let mut next_end = None;
        let mut next_end_delim = None;

        for delim in delimiters {
            if !delim.start.is_empty() {
                if let Some(pos) = content[index..].find(&delim.start) {
                    let abs = index + pos;
                    if next_start.map_or(true, |current| abs < current) {
                        next_start = Some(abs);
                        next_start_delim = Some(delim);
                    }
                }
            }
            if !delim.end.is_empty() {
                if let Some(pos) = content[index..].find(&delim.end) {
                    let abs = index + pos;
                    if next_end.map_or(true, |current| abs < current) {
                        next_end = Some(abs);
                        next_end_delim = Some(delim);
                    }
                }
            }
        }

        match (next_start, next_end) {
            (None, None) => {
                result.push_str(&content[index..]);
                break;
            }
            (Some(start_idx), Some(end_idx)) if start_idx <= end_idx => {
                result.push_str(&content[index..start_idx]);
                let delim = next_start_delim.expect("start delimiter missing");
                let after_start = start_idx + delim.start.len();
                if let Some(end_rel) = content[after_start..].find(&delim.end) {
                    index = after_start + end_rel + delim.end.len();
                } else {
                    break;
                }
            }
            (_, Some(end_idx)) => {
                let delim = next_end_delim.expect("end delimiter missing");
                index = end_idx + delim.end.len();
            }
            _ => {
                result.push_str(&content[index..]);
                break;
            }
        }
    }

    result
}

struct Status {
    enabled: bool,
}

impl Status {
    fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    fn show(&self, message: &str) -> io::Result<()> {
        if self.enabled {
            print!("\r\x1b[2K{message}");
            io::stdout().flush()?;
        }
        Ok(())
    }

    fn clear(&self) -> io::Result<()> {
        if self.enabled {
            print!("\r\x1b[2K");
            io::stdout().flush()?;
        }
        Ok(())
    }
}
