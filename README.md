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
curl -fsSL https://raw.githubusercontent.com/akzin/askllm/master/install.sh | sh -s -- akzin/askllm
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

# Command execution is enabled by default, but every command requires confirmation.
command_execution_enabled = true

# Drop the model's reasoning tokens. Set to "" to leave it to the endpoint.
reasoning_effort = "none"
```

`base_url` must point to an OpenAI Chat Completions-compatible API. The example expects a local service listening on port `20128`.

The model answers in its own words, streamed to the terminal, and can call two tools. `bash` runs exactly the command it proposes, only after a `y` confirmation; Enter and every answer other than `y` mean no. `explore` runs one read-only command from the configured allowlist without confirmation. Redirects, substitutions, `;`, `||`, `|&`, known destructive operations, and private files are blocked. After a command, the output and exit code go back to the model and it continues the same answer; non-zero exit codes are reported. If the model marks a successful command as self-contained, askllm stops after the output. Visible `&&` and `|` chains are confirmed once as a whole and every stage is validated separately.

Read-only exploration is opt-in. The first requested exploration displays a default allowlist and asks once for approval. Approved commands and any individually approved additions are saved in the `explore` section of the configuration file; the rest of the file, including comments and order, is left untouched. Shell operators and references to private files are always blocked during exploration.

## Usage

Pass the question as the command-line arguments:

```sh
askllm how do I install a Snap package?
```

The response streams directly to the terminal. No environment variables or command-line configuration flags are required.

If a command is proposed, askllm shows the exact command and asks `Run this? [yN]`. Set `command_execution_enabled = false` to answer without tools.

## Development

Run the test suite and build a release binary with:

```sh
cargo test
cargo build --release
```

GitHub Actions runs formatting, Clippy, tests, and a release build. Pushing a tag beginning with `v` publishes the release binary.
