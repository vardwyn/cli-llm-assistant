use clap::{CommandFactory, Parser};

mod cli;
mod client;
mod config;
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

    if let Some(shell) = cli.completions {
        if cli.init || cli.history_clear || cli.history.is_some() {
            bail!("--completions cannot be combined with other flags");
        }
        ensure_no_prompt_args(&cli, "--completions", false)?;
        let mut cmd = Cli::command();
        clap_complete::generate(shell, &mut cmd, "ai", &mut std::io::stdout());
        return Ok(());
    }

    if cli.init {
        ensure_no_prompt_args(&cli, "--init", false)?;
        let path = config::write_default_config()?;
        println!("wrote default config to {}", path.display());
        return Ok(());
    }

    if cli.history_clear {
        ensure_no_prompt_args(&cli, "--history-clear", false)?;
        if cli.history.is_some() {
            bail!("--history and --history-clear cannot be used together");
        }
        history::clear_history()?;
        println!("history cleared");
        return Ok(());
    }

    if let Some(index) = cli.history {
        ensure_no_prompt_args(&cli, "--history", true)?;
        let config = config::load_config()?;
        let entry = history::replay_entry(index)?;
        let minimal = if cli.minimal {
            true
        } else {
            config.defaults.minimal.unwrap_or(false)
        };
        let stdout_is_tty = std::io::stdout().is_terminal();
        let colorize = stdout_is_tty && !minimal;
        let delimiters = config
            .models
            .get(&entry.model)
            .filter(|model| !model.thinking_delimiters.is_empty())
            .map(|model| model.thinking_delimiters.clone())
            .unwrap_or_else(|| config.defaults.thinking_delimiters.clone());

        let output = if colorize {
            client::color_thinking(&entry.response, &delimiters)
        } else {
            entry.response
        };
        print!("{output}");
        return Ok(());
    }

    client::run(&cli).await
}

fn ensure_no_prompt_args(cli: &Cli, flag: &str, allow_minimal: bool) -> Result<()> {
    if cli.model.is_some() || cli.prompt.is_some() || !cli.args.is_empty() {
        bail!("{flag} cannot be combined with prompt flags or arguments");
    }
    if !allow_minimal && cli.minimal {
        bail!("{flag} cannot be combined with --minimal");
    }
    Ok(())
}
