use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "ai", version, about = "One-shot LLM chat client")]
pub struct Cli {
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub prompt: Option<String>,
    #[arg(long)]
    pub minimal: bool,
    #[arg(long)]
    pub strip_thinking: bool,
    #[arg(long)]
    pub init: bool,
    #[arg(long)]
    pub history_clear: bool,
    #[arg(long)]
    pub history: Option<usize>,
    #[arg(long, value_enum)]
    pub completions: Option<clap_complete::Shell>,
    #[arg(long)]
    pub list_models: bool,
    #[arg(long)]
    pub list_prompts: bool,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}
