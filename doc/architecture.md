# Technical design

## Stack

- **Language/runtime:** Rust.
- **Protocol:** OpenAI Chat Completions-compatible `POST /chat/completions`.
- **Transport:** HTTP client with a timeout and Server-Sent Events (SSE) parsing.
- **Backend:** local OpenAI-compatible server at `http://localhost:20128/v1`.

## Request

```json
{
  "model": "openai/gpt-oss-20b",
  "messages": [
    {"role": "system", "content": "<complete system_prompt read from ~/.config/askllm/config.toml>"},
    {"role": "user", "content": "what is the weather?"}
  ],
  "stream": true,
  "tools": [
    {"type": "function", "function": {"name": "run", "parameters": {}}},
    {"type": "function", "function": {"name": "explore", "parameters": {}}}
  ],
  "reasoning_effort": "none",
  "temperature": 0.0,
  "top_p": 1.0
}
```

`temperature` and `top_p` are optional and only included when configured in
the TOML file.

## Response

Parse the OpenAI-compatible SSE stream and print `choices[0].delta.content` as it arrives. Accumulate `choices[0].delta.tool_calls` by `index`; reasoning deltas are ignored. When the stream ends with tool calls, the CLI confirms and runs them, appends `assistant` and `tool` messages, and streams the next round in the same conversation. A successful command marked self-contained ends the conversation after its output; a failed command always returns to the model. The `[DONE]` event ends a round. Missing required fields or malformed events are protocol errors.

## Modules

1. `main`: arguments and exit codes.
2. `config`: TOML loading, atomic writes, and persisted exploration consent.
3. `chat`: one HTTP client with timeouts, SSE streaming, and tool-call assembly.
4. `tools`: `run` and `explore` handling, confirmation outcomes, and output bounding.
5. `command`: shared capture, confirmation, allowlist matching, output bounds, the shell selector, and the platform facade.
6. `posix`: the Unix backend: `bash -c` for `run`, `sh -c` for `explore`, the POSIX validator and the Unix default allowlist.
7. `windows_cmd`: the Windows cmd backend: `cmd.exe /C` for `explore` and for `run` with `shell = "cmd"`, with strict token validation and its default allowlist.
8. `windows_powershell`: the Windows PowerShell backend: `powershell.exe`, base64/UTF-16LE `-EncodedCommand`, and the PowerShell blocklist for `run` with `shell = "powershell"`.
9. `shell`: the POSIX command parser (Unix only).
10. `protocol`: request/stream models, tool definitions, and tool-call assembly.
11. `ui`: terminal rendering, prompts, and probe previews.
12. `client`: the tool loop, instructions, and the retry nudge.

## Platform backends

The backend is selected at compile time. `command` exposes the same functions
to `tools` on every platform:

- `explore` on Unix: `sh -c` with the POSIX allowlist and blocklist.
- `run` on Unix: `bash --noprofile --norc -c` with the POSIX chain/pipeline
  validator.
- `explore` on Windows: `cmd.exe /C`; only a simple command is accepted. The
  tokenizer rejects `&`, `|`, `>`, `<`, `%`, `!`, `^`, `;` and control
  characters, recursive flags and private paths.
- `run` on Windows with `shell = "cmd"`: `cmd.exe /C`, one simple command, no
  operators, redirects or expansion. It accepts writes and file reads after
  confirmation, but blocks shell hosts, scripts and system-level tools.
- `run` on Windows with `shell = "powershell"`: `powershell.exe -NoLogo
  -NoProfile -NonInteractive -EncodedCommand`; the command is UTF-16LE/base64
  encoded, output encoding is forced to UTF-8, and a blocklist rejects
  dangerous cmdlets, aliases, shell hosts and redirection. Every `run` command
  is confirmed.

`explore` has no shell parameter; it is always cmd on Windows and sh on Unix.
Only `run` exposes a shell choice. The model may use `run` during research as
well as for the final command. A declined `run` returns a tool message instead
of ending the turn, so the model can explain the command for the user to run
themselves or suggest an alternative.

The child inherits askllm's console and writes to pipes; no new console window
is created. Windows-specific modules are compiled only with `cfg(windows)`, so
the Linux build has no dependency on PowerShell or cmd.

The chat layer only knows the OpenAI-compatible protocol; provider-specific logic does not belong in the CLI. The model name and configured system prompt are read from TOML; internal command-safety and response-format instructions are appended. Configuration lives in TOML. `NO_COLOR` controls styling, and `HOME` or `XDG_CONFIG_HOME` can affect the config path.

## Tests

`chat` keeps a fake local SSE server test for streaming content and tool-call assembly. Confirmation stays end-to-end manual because it reads the terminal. POSIX execution tests run on Unix; the Windows cmd and PowerShell validators and their simple execution paths have `#[cfg(windows)]` tests. CI runs the same formatting, Clippy, test and release-build steps on `ubuntu-latest` and `windows-latest`.
