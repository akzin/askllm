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
  "stream": true
}
```

## Response

Parse the OpenAI-compatible SSE stream and write each `choices[0].delta.content` chunk directly to stdout. The `[DONE]` event ends the response. Missing required fields or malformed events are protocol errors.

## Modules

1. `cli`: arguments, exit codes, and output.
2. `config`: TOML file loading.
3. `client`: HTTP request, bearer token, and timeout.
4. `protocol`: request/stream models and validation.

The client only knows the OpenAI-compatible protocol; provider-specific logic does not belong in the CLI. The model name and complete system prompt are read from TOML and passed unchanged to the provider. No environment variables are read by the application.

## Next implementation step

Build a minimal happy-path test with a fake local HTTP server that emits SSE chunks: prompt in, streamed model answer out.
