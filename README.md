# askllm

`askllm` is a small Rust command-line client for an OpenAI-compatible local LLM endpoint. Ask a question and receive the answer as streamed text:

```sh
askllm how do I install a Snap package?
```

The default configuration is designed for a local Ubuntu-aware setup. It uses the endpoint and model configured in `~/.config/askllm/config.toml`.

## Installation

### Install the latest Linux release

The installer downloads the latest `askllm` binary from GitHub Releases and installs it into `~/.local/bin`:

```sh
curl -fsSL https://raw.githubusercontent.com/akzin/askllm/main/install.sh | sh -s -- akzin/askllm
```

The installer currently installs the prebuilt Linux `x86_64` release and does not require `sudo`. The Rust source is portable, but releases for macOS and Windows are not published yet.

Make sure `~/.local/bin` is in your `PATH`:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

The installer installs only the binary. You must create the configuration file yourself.

### Install from source

Install Rust and Cargo, then run:

```sh
cargo install --path .
```

## Configuration

Create the configuration directory and copy the example:

```sh
mkdir -p ~/.config/askllm
cp askllm.toml.example ~/.config/askllm/config.toml
```

The resulting file is read from:

```text
~/.config/askllm/config.toml
```

Example configuration:

```toml
base_url = "http://localhost:20128/v1"
model = "gh/gpt-5.6-luna"

system_prompt = """
Respond in Dutch.
Answer briefly and directly.
For command-line questions, provide the exact command first.
Mention when sudo or another permission is required.
Do not invent system details or add unnecessary explanation.
"""
```

`base_url` must point to an OpenAI Chat Completions-compatible API. The example expects a local service listening on port `20128`.

## Usage

Pass the question as the command-line arguments:

```sh
askllm how do I install a Snap package?
```

The response is streamed directly to the terminal. No environment variables or command-line configuration flags are required.

## Development

Run the test suite and build a release binary with:

```sh
cargo test
cargo build --release
```

GitHub Actions runs formatting, Clippy, tests, and a release build. Pushing a tag beginning with `v` publishes the release binary.
