use anyhow::{bail, Result};
use clap_complete::Shell;

pub fn render(shell: Shell) -> Result<String> {
    match shell {
        Shell::Bash => Ok(bash_script()),
        Shell::Zsh => Ok(zsh_script()),
        Shell::Fish => Ok(fish_script()),
        _ => bail!("unsupported shell; use bash, zsh, or fish"),
    }
}

fn bash_script() -> String {
    r#"# bash completion for ai

_ai_complete() {
  local kind="$1"
  case "$kind" in
    models)
      ai --list-models 2>/dev/null
      ;;
    prompts)
      ai --list-prompts 2>/dev/null
      ;;
  esac
}

_ai() {
  local cur prev
  COMPREPLY=()
  cur="${COMP_WORDS[COMP_CWORD]}"
  prev="${COMP_WORDS[COMP_CWORD-1]}"

  case "$prev" in
    --model)
      COMPREPLY=( $(compgen -W "$(_ai_complete models)" -- "$cur") )
      return 0
      ;;
    --prompt)
      COMPREPLY=( $(compgen -W "$(_ai_complete prompts)" -- "$cur") )
      return 0
      ;;
    --completions)
      COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") )
      return 0
      ;;
    --history)
      return 0
      ;;
  esac

  if [[ "$cur" == --* ]]; then
    COMPREPLY=( $(compgen -W "--model --prompt --minimal --strip-thinking --init --history --history-clear --completions --help --version" -- "$cur") )
    return 0
  fi

  return 0
}

complete -F _ai ai
"#
    .to_string()
}

fn zsh_script() -> String {
    r#"#compdef ai

local -a models prompts
models=(${(f)"$(ai --list-models 2>/dev/null)"})
prompts=(${(f)"$(ai --list-prompts 2>/dev/null)"})

_arguments \
  '--model=[select model]:model:->models' \
  '--prompt=[select preset prompt]:prompt:->prompts' \
  '--minimal[disable status output and thinking colorization]' \
  '--strip-thinking[remove thinking text from output]' \
  '--init[create default config file]' \
  '--history=[replay history item]:index:' \
  '--history-clear[delete stored history]' \
  '--completions=[print shell completions]:shell:(bash zsh fish)' \
  '--help[show help]' \
  '--version[show version]' \
  '*:prompt:'

case $state in
  models)
    _describe 'models' models
    ;;
  prompts)
    _describe 'prompts' prompts
    ;;
esac
"#
    .to_string()
}

fn fish_script() -> String {
    r#"# fish completion for ai

complete -c ai -f -l model -r -a "(ai --list-models 2>/dev/null)" -d "Model"
complete -c ai -f -l prompt -r -a "(ai --list-prompts 2>/dev/null)" -d "Preset prompt"
complete -c ai -f -l minimal -d "Disable status output and thinking colorization"
complete -c ai -f -l strip-thinking -d "Remove thinking text from output"
complete -c ai -f -l init -d "Create default config file"
complete -c ai -f -l history -r -d "Replay history item"
complete -c ai -f -l history-clear -d "Delete stored history"
complete -c ai -f -l completions -r -a "bash zsh fish" -d "Print shell completions"
complete -c ai -f -l help -d "Show help"
complete -c ai -f -l version -d "Show version"
"#
    .to_string()
}
