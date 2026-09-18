# Technical design

## Stack

- **Language/runtime:** Rust.
- **Protocol:** OpenAI Chat Completions-compatible `POST /chat/completions`.
- **Transport:** HTTP client with a timeout and Server-Sent Events (SSE) parsing.
- **Backend:** OpenRouter by default; any OpenAI-compatible local server is allowed as an override.

## Request

```json
{
  "model": "openai/gpt-oss-20b",
  "messages": [
    {"role": "system", "content": "The user lives in Amsterdam and uses Ubuntu. Respond in Dutch. Answer briefly and directly. For command-line questions, provide the exact command first, explain only what is necessary, and mention when sudo or another permission is required. Do not invent system details or add unnecessary explanation."},
    {"role": "user", "content": "what is the weather?"}
  ],
  "stream": true
}
```

## Response

Parse the OpenAI-compatible SSE stream and write each `choices[0].delta.content` chunk directly to stdout. The `[DONE]` event ends the response. Missing required fields or malformed events are protocol errors.

## Modules

1. `cli`: arguments, exit codes, and output.
2. `config`: environment variables and defaults.
3. `client`: HTTP request, bearer token, and timeout.
4. `protocol`: request/stream models and validation.

The client only knows the OpenAI-compatible protocol; provider-specific logic does not belong in the CLI. The model name is a free-form string from `--model` or `LLMASK_MODEL` and is passed directly to OpenRouter. The system prompt is built locally from `LLMASK_LOCATION`, `LLMASK_OS`, and the fixed instruction to answer briefly and give practical terminal guidance.

## Next implementation step

Build a minimal happy-path test with a fake local HTTP server that emits SSE chunks: prompt in, streamed model answer out. Then add a manual OpenRouter check with a real model and `LLMASK_API_KEY`.
