# CLI design

## Initial interface

```text
llmask --model openai/gpt-oss-20b what is the weather?
```

The prompt is required and consists of all remaining command-line arguments joined with spaces. Quotes are not required. `--model` is required unless `LLMASK_MODEL` is set. The CLI requests a streaming response and writes each text chunk directly to stdout, followed by a newline when the response is complete.

## Configuration

| Variable | Default | Meaning |
|---|---|---|
| `LLMASK_BASE_URL` | `https://openrouter.ai/api/v1` | OpenAI-compatible API base URL |
| `LLMASK_MODEL` | empty | OpenRouter or local model name |
| `LLMASK_API_KEY` | empty | Optional bearer token |
| `LLMASK_LOCATION` | required | Location used in the system prompt |
| `LLMASK_OS` | `Ubuntu` | Operating system used in the system prompt |

No configuration file in v0.1. Do not prompt for missing configuration. OpenRouter without a key fails immediately with a clear error; a local endpoint may run without a key.

## Options

```text
llmask --model <provider/model> <prompt words...>
```

`--model` takes precedence over `LLMASK_MODEL`. The model value is passed unchanged in the OpenAI-compatible request.

## Errors

- missing prompt: short usage message to stderr, exit `2`;
- backend unavailable: clear error to stderr, exit `1`;
- invalid backend response or stream: error to stderr, exit `1`.

## Examples

```text
llmask --model openai/gpt-oss-20b summarize this sentence
LLMASK_MODEL=anthropic/claude-sonnet-4 LLMASK_LOCATION=Amsterdam llmask what is the weather? > answer.txt
```

## System prompt

The CLI always builds this system message, replacing `{location}` with `LLMASK_LOCATION` and `{os}` with `LLMASK_OS`:

```text
The user lives in {location} and uses {os}. Respond in Dutch. Answer briefly and directly. For command-line questions, provide the exact command first, explain only what is necessary, and mention when sudo or another permission is required. Do not invent system details or add unnecessary explanation.
```

JSON output, stdin, and interactive chat are deferred to a later iteration. Streaming is the default terminal output mode in v0.1.
