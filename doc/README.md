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
- OpenRouter as the default OpenAI-compatible backend;
- a local backend as a development override;
- authentication through an environment variable, never interactively in the CLI;
- streaming output directly to stdout;
- a system prompt containing the user's location, operating system, and an instruction to answer briefly.

## Out of scope

Chat history, tools, files, multiple providers, terminal UI, and interactive configuration.

## Design assumption

“Openroute” means an OpenAI-compatible route. The endpoint, model name, and system prompt are configured in `~/.config/askllm/config.toml`. The initial backend is your local server at `http://localhost:20128/v1`; no authentication is required.
