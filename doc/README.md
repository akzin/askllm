# askllm

## v0.1 goal

`askllm what is the weather?` returns a short text answer, for example:

```text
25 degrees, with some rain
```

The first version is intentionally small:

- one-shot prompt as positional arguments;
- prompts may contain multiple unquoted words;
- answer written to stdout;
- errors written to stderr with a non-zero exit code;
- an OpenAI-compatible endpoint configured in TOML;
- a local backend by default;
- an optional `api_key` in the TOML file, never prompted for in the CLI;
- streaming output directly to stdout;
- a system prompt supplied in TOML;
- native tool calls with confirmation for command execution.

## Out of scope

Chat history, files, multiple providers, terminal UI, and interactive configuration.

## Design assumption

“Openroute” means an OpenAI-compatible route. The endpoint, model name, API key, and system prompt are configured in `~/.config/askllm/config.toml`. The initial backend is a local OpenAI-compatible server; a key is optional when the server does not require one.
