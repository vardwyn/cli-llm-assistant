use clap::Parser;

mod cli;
mod client;
mod config;
mod completions;
mod history;
mod openai;
mod paths;
mod types;

use anyhow::{bail, Result};
use cli::Cli;
use std::io::IsTerminal;

#[tokio::main]
async fn main() {
    if let Err(err) = real_main().await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

async fn real_main() -> Result<()> {
    let cli = Cli::parse();

    if cli.init && (cli.history_clear || cli.history.is_some()) {
        bail!("--init cannot be combined with history flags");
    }

    if cli.list_models || cli.list_prompts {
        if cli.list_models && cli.list_prompts {
            bail!("--list-models and --list-prompts cannot be used together");
        }
        if cli.init || cli.history_clear || cli.history.is_some() || cli.completions.is_some() {
            bail!("--list-models/--list-prompts cannot be combined with other modes");
        }
        if cli.model.is_some() || cli.prompt.is_some() || !cli.args.is_empty() {
            bail!("--list-models/--list-prompts cannot be combined with prompt flags or arguments");
        }
        let config = config::load_config()?;
        if cli.list_models {
            for name in config.models.keys() {
                println!("{name}");
            }
        } else {
            for name in config.prompts.keys() {
                println!("{name}");
            }
        }
        return Ok(());
    }

    if let Some(shell) = cli.completions {
        if cli.init || cli.history_clear || cli.history.is_some() {
            bail!("--completions cannot be combined with other flags");
        }
        ensure_no_prompt_args(&cli, "--completions", false, false)?;
        let script = completions::render(shell)?;
        print!("{script}");
        return Ok(());
    }

    if cli.init {
        ensure_no_prompt_args(&cli, "--init", false, false)?;
        let path = config::write_default_config()?;
        println!("wrote default config to {}", path.display());
        return Ok(());
    }

    if cli.history_clear {
        ensure_no_prompt_args(&cli, "--history-clear", false, false)?;
        if cli.history.is_some() {
            bail!("--history and --history-clear cannot be used together");
        }
        history::clear_history()?;
        println!("history cleared");
        return Ok(());
    }

    if let Some(index) = cli.history {
        ensure_no_prompt_args(&cli, "--history", true, true)?;
        let config = config::load_config()?;
        let entry = history::replay_entry(index)?;
        let minimal = if cli.minimal {
            true
        } else {
            config.defaults.minimal.unwrap_or(false)
        };
        let strip_thinking = resolve_strip_thinking(&cli, &config, &entry.model)?;
        let stdout_is_tty = std::io::stdout().is_terminal();
        let colorize = stdout_is_tty && !minimal;
        let delimiters = config
            .models
            .get(&entry.model)
            .filter(|model| !model.thinking_delimiters.is_empty())
            .map(|model| model.thinking_delimiters.clone())
            .unwrap_or_else(|| config.defaults.thinking_delimiters.clone());

        let mut output_text = entry.response;
        if strip_thinking {
            output_text = client::strip_thinking_text(&output_text, &delimiters);
        }
        let output = if colorize {
            client::color_thinking(&output_text, &delimiters)
        } else {
            output_text
        };
        print!("{output}");
        return Ok(());
    }

    client::run(&cli).await
}

fn ensure_no_prompt_args(
    cli: &Cli,
    flag: &str,
    allow_minimal: bool,
    allow_strip: bool,
) -> Result<()> {
    if cli.model.is_some() || cli.prompt.is_some() || !cli.args.is_empty() {
        bail!("{flag} cannot be combined with prompt flags or arguments");
    }
    if !allow_minimal && cli.minimal {
        bail!("{flag} cannot be combined with --minimal");
    }
    if !allow_strip && cli.strip_thinking {
        bail!("{flag} cannot be combined with --strip-thinking");
    }
    Ok(())
}

fn resolve_strip_thinking(cli: &Cli, config: &config::Config, model_name: &str) -> Result<bool> {
    if cli.strip_thinking {
        return Ok(true);
    }
    let model = config.models.get(model_name);
    Ok(model
        .and_then(|model| model.strip_thinking)
        .unwrap_or(config.defaults.strip_thinking.unwrap_or(false)))
}
