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
    {"type": "function", "function": {"name": "bash", "parameters": {}}},
    {"type": "function", "function": {"name": "explore", "parameters": {}}}
  ],
  "reasoning_effort": "none"
}
```

## Response

Parse the OpenAI-compatible SSE stream and print `choices[0].delta.content` as it arrives. Accumulate `choices[0].delta.tool_calls` by `index`; reasoning deltas are ignored. When the stream ends with tool calls, the CLI confirms and runs them, appends `assistant` and `tool` messages, and streams the next round in the same conversation. A successful command marked self-contained ends the conversation after its output; a failed command always returns to the model. The `[DONE]` event ends a round. Missing required fields or malformed events are protocol errors.

## Modules

1. `main`: arguments and exit codes.
2. `config`: TOML loading, atomic writes, and persisted exploration consent.
3. `chat`: one HTTP client with timeouts, SSE streaming, and tool-call assembly.
4. `tools`: `bash` and `explore` handling, confirmation outcomes, and output bounding.
5. `command`: command validation, terminal execution, output capture, and allowlist matching.
6. `protocol`: request/stream models, tool definitions, and tool-call assembly.
7. `ui`: terminal rendering, prompts, and probe previews.
8. `client`: the tool loop, instructions, and the retry nudge.

The chat layer only knows the OpenAI-compatible protocol; provider-specific logic does not belong in the CLI. The model name and configured system prompt are read from TOML; internal command-safety and response-format instructions are appended. Configuration lives in TOML. `NO_COLOR` controls styling, and `HOME` or `XDG_CONFIG_HOME` can affect the config path.

## Tests

`chat` keeps a fake local SSE server test for streaming content and tool-call assembly. Confirmation stays end-to-end manual because it reads the terminal; the remaining tool logic is covered by unit tests.
