use crate::style;
use anyhow::{anyhow, Context, Result};
use std::io::{self, BufRead, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub(crate) const MAX_COMMAND_BYTES: usize = 4096;
pub(crate) const MAX_CAPTURE_BYTES: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct CommandResult {
    pub exit_code: i32,
    pub output: String,
}

pub fn confirm<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> Result<bool> {
    ask_yes_no(input, output, "Run this? [yN] ")
}

pub fn ask_yes_no<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    prompt: &str,
) -> Result<bool> {
    write!(output, "{prompt}")?;
    output.flush()?;
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y"))
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Bash,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Cmd,
    PowerShell,
}

impl ShellKind {
    pub fn label(self) -> &'static str {
        match self {
            #[cfg(unix)]
            Self::Bash => "bash",
            #[cfg(windows)]
            Self::Cmd => "cmd",
            #[cfg(windows)]
            Self::PowerShell => "powershell",
        }
    }
}

#[cfg(unix)]
pub fn parse_shell(_value: Option<&str>) -> Result<ShellKind> {
    Ok(ShellKind::Bash)
}

#[cfg(windows)]
pub fn parse_shell(value: Option<&str>) -> Result<ShellKind> {
    match value.map(str::to_ascii_lowercase).as_deref() {
        None | Some("powershell") => Ok(ShellKind::PowerShell),
        Some("cmd") => Ok(ShellKind::Cmd),
        Some(other) => Err(anyhow!(
            "unknown shell `{other}`; use `cmd` or `powershell`"
        )),
    }
}

pub fn syntax_hint(shell: ShellKind) -> &'static str {
    match shell {
        #[cfg(unix)]
        ShellKind::Bash => "use && for steps and | for pipelines; no ;, ||, |& or redirects",
        #[cfg(windows)]
        ShellKind::Cmd => "use one simple command; no &, &&, ||, |, redirects or expansion",
        #[cfg(windows)]
        ShellKind::PowerShell => "use ; for steps and | for pipelines; no &&, ||, |& or redirects",
    }
}

#[cfg(unix)]
pub fn validate_run(command: &str, _shell: ShellKind) -> Result<()> {
    crate::posix::validate_run(command)
}

#[cfg(windows)]
pub fn validate_run(command: &str, shell: ShellKind) -> Result<()> {
    match shell {
        ShellKind::Cmd => crate::windows_cmd::validate_run(command),
        ShellKind::PowerShell => crate::windows_powershell::validate_run(command),
    }
}

#[cfg(unix)]
pub fn run_attached(command: &str, _shell: ShellKind) -> Result<CommandResult> {
    crate::posix::run_attached(command)
}

#[cfg(windows)]
pub fn run_attached(command: &str, shell: ShellKind) -> Result<CommandResult> {
    match shell {
        ShellKind::Cmd => crate::windows_cmd::run_attached(command),
        ShellKind::PowerShell => crate::windows_powershell::run_attached(command),
    }
}

pub(crate) fn capture_command(
    mut command: Command,
    stdin: Stdio,
    forward_output: bool,
) -> Result<CommandResult> {
    let mut child = command
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start command")?;
    let stdout = child
        .stdout
        .take()
        .context("failed to capture command output")?;
    let stderr = child
        .stderr
        .take()
        .context("failed to capture command output")?;
    let captured = Arc::new(Mutex::new(Vec::new()));
    let stdout_reader = forward(stdout, Arc::clone(&captured), false, forward_output);
    let stderr_reader = forward(stderr, Arc::clone(&captured), true, forward_output);

    let status = child.wait().context("failed waiting for command")?;
    join_reader(stdout_reader)?;
    join_reader(stderr_reader)?;

    let output = Arc::try_unwrap(captured)
        .map_err(|_| anyhow!("failed to read command output"))?
        .into_inner()
        .map_err(|_| anyhow!("failed to read command output"))?;
    Ok(CommandResult {
        exit_code: exit_status_code(status),
        output: String::from_utf8_lossy(&output).into_owned(),
    })
}

fn forward<R>(
    mut reader: R,
    captured: Arc<Mutex<Vec<u8>>>,
    is_stderr: bool,
    forward_output: bool,
) -> thread::JoinHandle<Result<()>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            let count = reader
                .read(&mut buffer)
                .context("failed reading command output")?;
            if count == 0 {
                break;
            }
            if let Ok(mut output) = captured.lock() {
                let remaining = MAX_CAPTURE_BYTES.saturating_sub(output.len());
                output.extend_from_slice(&buffer[..count.min(remaining)]);
            }
            if !forward_output {
                continue;
            }
            if is_stderr {
                let mut target = io::stderr().lock();
                write_output(&mut target, &buffer[..count])?;
                target.flush()?;
            } else {
                let mut target = io::stdout().lock();
                write_output(&mut target, &buffer[..count])?;
                target.flush()?;
            }
        }
        Ok(())
    })
}

pub(crate) fn exit_status_code(status: std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    1
}

fn write_output<W: Write>(target: &mut W, bytes: &[u8]) -> Result<()> {
    if style::enabled() {
        target.write_all(b"\x1b[90m")?;
        target.write_all(bytes)?;
        target.write_all(b"\x1b[0m")?;
    } else {
        target.write_all(bytes)?;
    }
    Ok(())
}

fn join_reader(handle: thread::JoinHandle<Result<()>>) -> Result<()> {
    handle
        .join()
        .map_err(|_| anyhow!("failed to collect command output"))?
}

pub(crate) fn matches_allowlist(command: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|pattern| wildcard_match(pattern, command))
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    let (mut pattern_index, mut value_index, mut star, mut retry) = (0, 0, None, 0);
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();

    while value_index < value.len() {
        if pattern.get(pattern_index) == Some(&value[value_index]) {
            pattern_index += 1;
            value_index += 1;
        } else if pattern.get(pattern_index) == Some(&b'*') {
            star = Some(pattern_index);
            pattern_index += 1;
            retry = value_index;
        } else if let Some(star_index) = star {
            pattern_index = star_index + 1;
            retry += 1;
            value_index = retry;
        } else {
            return false;
        }
    }
    pattern[pattern_index..].iter().all(|byte| *byte == b'*')
}

#[cfg(unix)]
pub use crate::posix::{default_explore_allowlist, run_exploration, validate_explore};
#[cfg(windows)]
pub use crate::windows_cmd::{default_explore_allowlist, run_exploration, validate_explore};

#[cfg(test)]
mod tests {
    use super::{confirm, matches_allowlist};
    use std::io::Cursor;

    #[test]
    fn confirmation_defaults_to_no() {
        for answer in ["n\n", "\n", "yes\n", "j\n", "J\n"] {
            assert!(!confirm(&mut Cursor::new(answer), &mut Vec::new()).unwrap());
        }
        for answer in ["y\n", "Y\n"] {
            assert!(confirm(&mut Cursor::new(answer), &mut Vec::new()).unwrap());
        }
    }

    #[test]
    fn matches_allowlist_against_the_entire_command() {
        let patterns = vec!["which *".to_owned()];
        assert!(matches_allowlist("which apt", &patterns));
        assert!(!matches_allowlist("echo which apt", &patterns));
    }
}
