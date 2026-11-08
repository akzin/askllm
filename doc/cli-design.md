# CLI design

## Initial interface

```text
askllm what is the weather?
```

The prompt is required and consists of all command-line arguments after `askllm`, joined with spaces. Quotes are not required. The CLI streams the answer while it arrives and handles tool calls in the same conversation.

## Configuration

| File key | Meaning |
|---|---|---|
| `base_url` | OpenAI-compatible API base URL |
| `model` | Model name for the configured endpoint |
| `api_key` | Optional bearer token |
| `system_prompt` | Complete system prompt, including any line breaks |
| `reasoning_effort` | Reasoning effort sent to the endpoint; defaults to `none`, an empty value omits the field |
| `command_execution_enabled` | Enable command execution; defaults to `true`, but each command still requires confirmation |
| `[explore].allow` | Whole-command patterns for read-only exploration |
| `[explore].approved` | Whether the default exploration allowlist was approved |

The default configuration file is `~/.config/askllm/config.toml`. Do not prompt for missing configuration. A backend that requires a key fails immediately with a clear error; a local endpoint may run without a key.

## Errors

- missing prompt: short usage message to stderr, exit `2`;
- backend unavailable: clear error to stderr, exit `1`;
- invalid backend response or stream: error to stderr, exit `1`.

## Examples

```text
askllm summarize this sentence
askllm what is the weather? > answer.txt
```

## System prompt

The CLI sends `system_prompt` from the TOML file and appends the internal response-format and safety instructions. It does not perform placeholder replacement.

```text
Respond in Dutch.
Answer briefly and directly.
For command-line questions, provide the exact command first.
Mention when sudo or another permission is required.
Do not invent system details or add unnecessary explanation.
```

## Command execution

The model calls the `bash` tool with one exact command string, which may chain steps with `&&` and pipe with `|`. The displayed command string is also the string passed to the shell; no model request occurs between confirmation and execution. Every tool call requires `y` confirmation, with all other input, including Enter, declining execution. A chain is confirmed once as a whole, and every stage is validated separately; `;`, `||`, `|&` and redirects are blocked. The model can mark a successful command as self-contained with `follow_up: false`; askllm then stops after the output. A failed command always returns to the model, which may explain it or call another tool.

Commands run on the user's terminal: stdin is inherited, so password prompts and line-based questions go directly to the terminal and are never captured. Stdout and stderr are captured and shown live. The captured output and the exit code are returned to the model as the tool result. Non-zero exits are reported directly. The result is bounded to 24 KiB, keeping the head and tail. Full-screen programs that need a terminal on stdout, such as `nano`, are not supported.

Read-only exploration uses the separate `explore` tool. Its first use displays the default allowlist and asks for one-time approval. Individual commands outside the allowlist require explicit approval before being added. Shell composition, redirects, substitutions, and private-path requests are rejected independently of allowlist entries.
