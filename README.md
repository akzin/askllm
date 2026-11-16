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

The installer installs the prebuilt Linux `x86_64` release and does not require `sudo`. Windows `x86_64` releases are published as `askllm.exe`; macOS releases are not published yet.

Make sure `~/.local/bin` is in your `PATH`:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

The installer also creates `~/.config/askllm/config.toml` from the online example when the file does not exist. It never overwrites an existing config. Edit `base_url` and `model` before running askllm. Set `ASKLLM_NO_CONFIG=1` to skip config creation. When the release provides `SHA256SUMS`, the installer verifies the downloaded binary against it.

### Install on Windows

The installer downloads `askllm.exe` to `%LOCALAPPDATA%\Programs\askllm`, adds that directory to your user `PATH`, and creates `%APPDATA%\askllm\config.toml` from the online example if it does not exist. It never overwrites an existing config, and it verifies the downloaded binary against the release's `SHA256SUMS` when available.

```powershell
irm https://raw.githubusercontent.com/akzin/askllm/master/install.ps1 | iex
```

Use `-NoPath` or `-NoConfig` when you prefer manual setup. You can also download `askllm.exe` directly from the latest release, or build from source:

```powershell
cargo install --path .
```

The Windows build needs the MSVC toolchain: Visual Studio Build Tools with the C++ workload. The `run` tool can use `cmd.exe` or Windows PowerShell 5.1; `explore` uses `cmd.exe` for simple read-only checks. Neither starts a separate console window. Windows PowerShell 5.1 is part of Windows, so no extra shell is required.

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

On Windows the config directory comes from `%APPDATA%`, so the file is:

```text
%APPDATA%\askllm\config.toml
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

# Sampling: lower temperature makes command generation more predictable.
# Remove either key to leave it to the endpoint.
temperature = 0.0
top_p = 1.0
```

`base_url` must point to an OpenAI Chat Completions-compatible API. The example expects a local service listening on port `20128`.

The model answers in its own words, streamed to the terminal, and can call two tools. `explore` is the model's own read-only investigation area: it always uses cmd on Windows and sh on Linux, has no shell parameter, and runs from the configured allowlist without confirmation. `run` is the user's approval area: every command is shown and requires `y`; Enter and every other answer mean no. On Windows, `run` takes a `shell` argument: `cmd` for one simple command, or `powershell` (the default) for everything else. The model may use `run` during research as well as for the final command. If the user declines, the model receives that message and can still explain the command instead of stopping. The model has network access through `run`, so for live data such as weather it can use `Invoke-WebRequest` or `curl` instead of saying it cannot fetch live data. Redirects, substitutions, `||`, `|&`, known destructive operations, and private files are blocked; on Linux `;` is blocked too. After a command, the output and exit code go back to the model and it continues the same answer; non-zero exit codes are reported. If the model marks a successful command as self-contained, askllm stops after the output. Visible `&&`/`|` chains on Linux and `;`/`|` chains in PowerShell are confirmed once as a whole and every stage is validated separately.

Read-only exploration is opt-in. The first requested exploration displays a default allowlist and asks once for approval. Approved commands and any individually approved additions are saved in the `explore` section of the configuration file; the rest of the file, including comments and order, is left untouched. Shell operators and references to private files are always blocked during exploration.

On Windows, `run` accepts `shell = "cmd"` or `shell = "powershell"` (the default). Use cmd for one simple command; use PowerShell 5.1 for everything else. PowerShell separates steps with `;` and pipes with `|`; `&&`, `||`, `|&` and redirects are rejected. `explore` uses `cmd.exe` for simple read-only checks such as `dir`, `where`, `whoami`, `ipconfig`, `ver`, `vol`, `hostname`, `tasklist` and `getmac`; it has no shell parameter, and it rejects shell operators, variable expansion, redirects, recursive flags and private paths. A cmd call starts in roughly 30 ms, while a `run` call with PowerShell pays startup, typically 0.35-0.8 seconds. PowerShell output is forced to UTF-8; cmd output follows the active OEM code page. On Linux, `run` uses bash.

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

GitHub Actions runs formatting, Clippy, tests, and a release build on Linux and Windows. Pushing a tag beginning with `v` publishes both binaries. Windows verification needs the MSVC toolchain and PowerShell 5.1; the CI runner provides both.
