# ai

One-shot CLI for OpenAI-compatible chat endpoints.

## Quick start

Create a default config:

```bash
ai --init
```

Edit the generated file at `$XDG_CONFIG_HOME/ai/config.toml` (or `~/.config/ai/config.toml`), then run:

```bash
ai "hello from cli"
```

You can also pipe stdin:

```bash
echo "summarize this" | ai
```

## Flags

- `--model NAME` select a model from `[models]` for one call.
- `--prompt NAME` select a preset prompt from `[prompts]` for one call.
- `--minimal` disable status output and thinking colorization for that invocation.
- `--init` create a default config file.
- `--history N` replay the N-th most recent response to stdout (1 = last).
- `--history-clear` delete stored history.

## Config

The client reads `$XDG_CONFIG_HOME/ai/config.toml` (falls back to `~/.config/ai/config.toml`).

### Example

```toml
[defaults]
model = "openai"
prompt = "concise"
minimal = false
thinking_delimiters = [
  { start = "<think>", end = "</think>" },
  { start = "[thought]", end = "[/thought]" },
]

[history]
enabled = true
max_entries = 100

[prompts.concise]
text = "Be concise and direct."

[models.openai]
endpoint = "https://api.openai.com"
model = "gpt-4o-mini"
system_prompt = "You are a helpful assistant."
api_key_command = "pass show openai/api-key"
thinking_delimiters = [
  { start = "<think>", end = "</think>" },
]
```

### Notes

- `endpoint` accepts either `https://host` or `https://host/v1` or full `/v1/chat/completions`.
- `api_key` and `api_key_command` are mutually exclusive.
- `defaults.prompt` is required; use an empty prompt text if you want no additional preset prompt.
- `system_prompt` (model system prompt) is always applied first.
- The selected named `prompt` (preset prompt) is applied next.
- The user input (CLI args or stdin) is applied last.
- `history` is stored in `$XDG_CACHE_HOME/ai/history.json`.

## Building

Release build:

```bash
cargo build --release
```

Static binary (example for musl on Linux):

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```
