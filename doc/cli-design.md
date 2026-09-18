# CLI design

## Initial interface

```text
askllm what is the weather?
```

The prompt is required and consists of all command-line arguments after `askllm`, joined with spaces. Quotes are not required. The CLI requests a streaming response and writes each text chunk directly to stdout, followed by a newline when the response is complete.

## Configuration

| File key | Meaning |
|---|---|---|
| `base_url` | OpenAI-compatible API base URL |
| `model` | OpenRouter or local model name |
| `api_key` | Optional bearer token |
| `system_prompt` | Complete system prompt, including any line breaks |

The default configuration file is `~/.config/askllm/config.toml`. Do not prompt for missing configuration. OpenRouter without a key fails immediately with a clear error; a local endpoint may run without a key.

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

The CLI sends `system_prompt` from the TOML file as one complete system message. It does not perform placeholder replacement or modify the text.

```text
Respond in Dutch.
Answer briefly and directly.
For command-line questions, provide the exact command first.
Mention when sudo or another permission is required.
Do not invent system details or add unnecessary explanation.
```

JSON output, stdin, and interactive chat are deferred to a later iteration. Streaming is the default terminal output mode in v0.1.
