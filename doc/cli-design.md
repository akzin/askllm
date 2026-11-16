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
| `temperature` | Sampling temperature; omitted when the key is absent |
| `top_p` | Nucleus sampling; omitted when the key is absent |
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

The model calls the `run` tool with one exact command string. On Linux the tool uses `bash`; commands may chain steps with `&&` and pipe with `|`. On Windows the tool takes a `shell` argument: `cmd` for one simple command, or `powershell` for everything else. The displayed command string is also the string passed to the shell; no model request occurs between confirmation and execution. Every `run` call requires `y` confirmation, with all other input, including Enter, declining execution. A chain is confirmed once as a whole, and every stage is validated separately; on Linux `;`, `||`, `|&` and redirects are blocked; in PowerShell `&&`, `||`, `|&` and redirects are blocked. The model can mark a successful command as self-contained with `follow_up: false`; askllm then stops after the output. A failed command always returns to the model, which may explain it or call another tool.

Commands run on the user's terminal: stdin is inherited, so password prompts and line-based questions go directly to the terminal and are never captured. Stdout and stderr are captured and shown live. The captured output and the exit code are returned to the model as the tool result. Non-zero exits are reported directly. The result is bounded to 24 KiB, keeping the head and tail. Full-screen programs that need a terminal on stdout, such as `nano`, are not supported.

Read-only exploration uses the separate `explore` tool. Its first use displays the default allowlist and asks for one-time approval. Individual commands outside the allowlist require explicit approval before being added. Shell composition, redirects, substitutions, and private-path requests are rejected independently of allowlist entries.

## Windows

On Windows the same two tools map to different backends:

- `explore` runs `cmd.exe /C` with a narrow read-only allowlist. The default
  list is `dir`, `dir *`, `where *`, `whoami`, `whoami *`, `ipconfig`,
  `ipconfig /all`, `ver`, `vol`, `hostname`, `tasklist`, `tasklist *`,
  `getmac` and `getmac *`. One simple command is accepted; `&`, `|`, `>`, `<`,
  `%`, `!`, `^`, `;`, recursive flags (`/s`, `/r`) and interactive flags
  (`/p`) are rejected. `ipconfig /release`, `/renew` and `/flushdns` are also
  rejected. Private paths are blocked.
- `run` takes `shell = "cmd"` or `shell = "powershell"` (the default). With
  `cmd`, one simple command is accepted; no pipes, redirects, `&`, `&&`, `||`,
  `%`, `!`, `^` or `;`. It accepts writes and file reads after confirmation,
  but blocks shell hosts, scripts and system-level tools. With `powershell`,
  the command runs through `powershell.exe -NoLogo -NoProfile -NonInteractive
  -EncodedCommand`; use `;` to separate steps and `|` to pipe, and `&&`, `||`,
  `|&` and redirects are rejected. The blocklist rejects dangerous cmdlets,
  aliases, shell hosts (`cmd`, `wsl`, `powershell`) and destructive tools.
  Every `run` command is confirmed.
- The config file is `%APPDATA%\askllm\config.toml`.
- No separate console window is opened; the child inherits askllm's console
  and its output is captured through pipes.
- A `run` call with cmd starts in roughly 30 ms. A `run` call with PowerShell
  pays startup, typically 0.35-0.8 seconds.
- PowerShell output is forced to UTF-8. cmd output follows the active OEM code
  page, so non-ASCII names can be mangled.

The tool definitions and system instructions are platform-aware: on Windows
the `run` description exposes the `shell` choice and the `explore` description
lists the cmd checks. Exit codes are unchanged: `2` for a missing prompt and
`1` for runtime errors.
