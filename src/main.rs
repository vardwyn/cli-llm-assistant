use clap::Parser;

mod cli;
mod client;
mod config;
mod history;
mod openai;
mod paths;
mod types;

use anyhow::{bail, Result};
use cli::Cli;

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

    if cli.init {
        ensure_no_prompt_args(&cli, "--init")?;
        let path = config::write_default_config()?;
        println!("wrote default config to {}", path.display());
        return Ok(());
    }

    if cli.history_clear {
        ensure_no_prompt_args(&cli, "--history-clear")?;
        if cli.history.is_some() {
            bail!("--history and --history-clear cannot be used together");
        }
        history::clear_history()?;
        println!("history cleared");
        return Ok(());
    }

    if let Some(index) = cli.history {
        ensure_no_prompt_args(&cli, "--history")?;
        let response = history::replay_response(index)?;
        print!("{response}");
        return Ok(());
    }

    client::run(&cli).await
}

fn ensure_no_prompt_args(cli: &Cli, flag: &str) -> Result<()> {
    if cli.model.is_some() || cli.prompt.is_some() || !cli.args.is_empty() || cli.minimal {
        bail!("{flag} cannot be combined with prompt flags or arguments");
    }
    Ok(())
}
