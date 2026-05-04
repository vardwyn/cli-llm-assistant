# ai

Prompt in, answer out - a tiny CLI client for one-shot chats with OpenAI-compatible LLMs

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

## Examples

Select a configured model for one call:

```bash
ai --model qwen "explain this error: EADDRINUSE"
```

Use a preset prompt from `[prompts]`:

```bash
ai --prompt concise "write a git commit message for these changes"
```

Pipe command output into a model:

```bash
cargo test 2>&1 | ai --prompt debug
```

Run quietly for scripts by disabling status output and thinking colorization:

```bash
git diff | ai --minimal "summarize this patch"
```

Remove model reasoning text from the final output:

```bash
ai --strip-thinking "give me only the final answer"
```

Replay the most recent stored response:

```bash
ai --history 1
```

List configured model and prompt names:

```bash
ai --list-models
ai --list-prompts
```

## Flags

- `--model NAME` select a model from `[models]` for one call.
- `--prompt NAME` select a preset prompt from `[prompts]` for one call.
- `--minimal` disable status output and thinking colorization for that invocation.
- `--strip-thinking` remove thinking text from output (before colorization).
- `--init` create a default config file.
- `--history N` replay the N-th most recent response to stdout (1 = last). Colorization obeys `--minimal`.
- `--history-clear` delete stored history.
- `--list-models` print available model names from config.
- `--list-prompts` print available prompt names from config.
- `--completions <shell>` print shell completions (`bash`, `zsh`, `fish`).

## Config

The client reads `$XDG_CONFIG_HOME/ai/config.toml` (falls back to `~/.config/ai/config.toml`).

### Example

Example config with multiple models and prompt presets:

```toml
[defaults]
model = "fast"
prompt = "concise"
minimal = false
strip_thinking = false
thinking_delimiters = [
  { start = "<think>", end = "</think>" },
]

[history]
enabled = true
max_entries = 100

[prompts.concise]
text = "Be concise and direct."

[prompts.debug]
text = "Find the likely root cause and suggest the smallest useful fix."

[models.fast]
endpoint = "https://api.openai.com"
model = "gpt-4o-mini"
system_prompt = "You are a practical CLI assistant."
api_key_command = "pass show openai/api-key"

[models.reasoning]
endpoint = "https://api.openai.com/v1"
model = "o3-mini"
system_prompt = "Think carefully, then answer clearly."
api_key_command = "pass show openai/api-key"
options = "{\"reasoning\":{\"effort\":\"medium\"}}"
strip_thinking = true
```

### Notes

- `endpoint` accepts either `https://host` or `https://host/v1` or full `/v1/chat/completions`.
- `api_key` and `api_key_command` are mutually exclusive.
- `defaults.prompt` is required; use an empty prompt text if you want no additional preset prompt.
- `system_prompt` (model system prompt) is always applied first.
- The selected named `prompt` (preset prompt) is applied next.
- The user input (CLI args or stdin) is applied last.
- `options` (per model) is a raw JSON object merged into the request body; it cannot override `model` or `messages`.
- `strip_thinking` can be set in defaults or per model; `--strip-thinking` forces it on.
- If the API returns a separate reasoning field, it is injected before the main content using the configured thinking delimiters (unless the content already contains delimiters).
- If a response contains only the end delimiter (no start), everything up to that end delimiter is treated as thinking text.
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

## Shell completions

Generate and install (examples):

```bash
ai --completions bash > /etc/bash_completion.d/ai
```

```bash
ai --completions zsh > /usr/local/share/zsh/site-functions/_ai
```

```bash
ai --completions fish > ~/.config/fish/completions/ai.fish
```
