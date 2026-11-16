use crate::command::{capture_command, CommandResult, MAX_COMMAND_BYTES};
use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};

pub const DEFAULT_EXPLORE_ALLOWLIST: &[&str] = &[
    "dir",
    "dir *",
    "where *",
    "whoami",
    "whoami *",
    "ipconfig",
    "ipconfig /all",
    "ver",
    "vol",
    "hostname",
    "tasklist",
    "tasklist *",
    "getmac",
    "getmac *",
];

pub fn default_explore_allowlist() -> &'static [&'static str] {
    DEFAULT_EXPLORE_ALLOWLIST
}

// A speed bump, not a security boundary: the user approves the allowlist.
const BLOCKED_EXPLORE_COMMANDS: &[&str] = &[
    "attrib",
    "auditpol",
    "bash",
    "bcdedit",
    "bitsadmin",
    "bootrec",
    "busybox",
    "cacls",
    "call",
    "cd",
    "certutil",
    "chcp",
    "chdir",
    "chkdsk",
    "cleanmgr",
    "cls",
    "cmd",
    "color",
    "compact",
    "comp",
    "copy",
    "cscript",
    "date",
    "defrag",
    "del",
    "dism",
    "diskcomp",
    "diskcopy",
    "diskpart",
    "erase",
    "exit",
    "expand",
    "fc",
    "find",
    "findstr",
    "for",
    "format",
    "ftype",
    "gpupdate",
    "icacls",
    "if",
    "label",
    "logoff",
    "makecab",
    "md",
    "mkdir",
    "mklink",
    "mode",
    "more",
    "mountvol",
    "move",
    "mshta",
    "net",
    "netsh",
    "pause",
    "powercfg",
    "powershell",
    "prompt",
    "pushd",
    "pwsh",
    "rd",
    "recover",
    "reg",
    "regedit",
    "ren",
    "rename",
    "replace",
    "rmdir",
    "robocopy",
    "rundll32",
    "sc",
    "schtasks",
    "secedit",
    "set",
    "setx",
    "sfc",
    "sh",
    "shutdown",
    "sort",
    "start",
    "subst",
    "takeown",
    "taskkill",
    "time",
    "timeout",
    "title",
    "type",
    "vssadmin",
    "wbadmin",
    "wevtutil",
    "wmic",
    "wscript",
    "wsl",
    "xcopy",
    "zsh",
];

// The confirmed cmd tool accepts writes and file reads; this list blocks
// shell hosts, scripting built-ins and system-level tools.
const BLOCKED_RUN_COMMANDS: &[&str] = &[
    "auditpol",
    "bash",
    "bcdedit",
    "bitsadmin",
    "bootrec",
    "busybox",
    "cacls",
    "call",
    "certutil",
    "chcp",
    "chkdsk",
    "cleanmgr",
    "cls",
    "cmd",
    "color",
    "cscript",
    "date",
    "defrag",
    "diskcomp",
    "diskcopy",
    "diskpart",
    "dism",
    "for",
    "format",
    "ftype",
    "gpupdate",
    "icacls",
    "if",
    "label",
    "logoff",
    "mode",
    "mountvol",
    "mshta",
    "net",
    "netsh",
    "pause",
    "powercfg",
    "powershell",
    "prompt",
    "pwsh",
    "recover",
    "reg",
    "regedit",
    "rundll32",
    "sc",
    "schtasks",
    "secedit",
    "setx",
    "sfc",
    "sh",
    "shutdown",
    "start",
    "subst",
    "takeown",
    "taskkill",
    "time",
    "timeout",
    "title",
    "vssadmin",
    "wbadmin",
    "wevtutil",
    "wmic",
    "wscript",
    "wsl",
    "zsh",
];

const BLOCKED_FLAGS: &[&str] = &[
    "/s",
    "/r",
    "/p",
    "/release",
    "/release6",
    "/renew",
    "/renew6",
    "/flushdns",
    "/registerdns",
    "/setclassid",
    "/setclassid6",
];

pub fn validate_explore(command: &str) -> Result<()> {
    if command.len() > MAX_COMMAND_BYTES {
        return Err(anyhow!("excessively long command"));
    }
    let tokens = tokenize(command)?;
    let program = normalized_program(&tokens)?;
    if BLOCKED_EXPLORE_COMMANDS.contains(&program.as_str()) {
        return Err(anyhow!("command is blocked by the safety policy"));
    }
    for argument in &tokens[1..] {
        let flag = argument.to_ascii_lowercase();
        if BLOCKED_FLAGS.contains(&flag.as_str()) {
            return Err(anyhow!(
                "recursive, interactive or state-changing flags are not allowed"
            ));
        }
    }
    if is_private_path_request(command) {
        return Err(anyhow!("commands involving private files are blocked"));
    }
    Ok(())
}

fn normalized_program(tokens: &[String]) -> Result<String> {
    let raw_program = tokens[0].trim_matches('"');
    let mut program = raw_program
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if program.is_empty() {
        return Err(anyhow!("could not identify the command"));
    }
    if has_blocked_extension(&program) || has_blocked_extension(raw_program) {
        return Err(anyhow!("script execution is not allowed"));
    }
    if let Some(stripped) = program.strip_suffix(".exe") {
        program = stripped.to_owned();
    }
    if let Some(stripped) = program.strip_suffix(".com") {
        program = stripped.to_owned();
    }
    if has_blocked_extension(&program) {
        return Err(anyhow!("script execution is not allowed"));
    }
    Ok(program)
}

pub fn validate_run(command: &str) -> Result<()> {
    if command.len() > MAX_COMMAND_BYTES {
        return Err(anyhow!("excessively long command"));
    }
    let tokens = tokenize(command)?;
    let program = normalized_program(&tokens)?;
    if BLOCKED_RUN_COMMANDS.contains(&program.as_str()) {
        return Err(anyhow!("command is blocked by the safety policy"));
    }
    if is_private_path_request(command) {
        return Err(anyhow!("commands involving private files are blocked"));
    }
    Ok(())
}

fn has_blocked_extension(program: &str) -> bool {
    [".bat", ".cmd", ".com", ".ps1", ".vbs", ".js", ".wsf"]
        .iter()
        .any(|extension| program.ends_with(extension))
}

fn tokenize(command: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for character in command.chars() {
        if character.is_control() {
            return Err(anyhow!("control characters are not allowed"));
        }
        match character {
            '"' => {
                in_quotes = !in_quotes;
                current.push(character);
            }
            '%' | '!' | '^' | ';' | '`' => {
                return Err(anyhow!(
                    "shell expansion and shell operators are not allowed"
                ));
            }
            '&' | '|' | '>' | '<' | '(' | ')' if !in_quotes => {
                return Err(anyhow!("shell operators are not allowed"));
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(character),
        }
    }

    if in_quotes {
        return Err(anyhow!("unterminated quote"));
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    if tokens.is_empty() {
        return Err(anyhow!("empty command"));
    }
    Ok(tokens)
}

fn is_private_path_request(command: &str) -> bool {
    let normalized = command.to_ascii_lowercase().replace('\\', "/");
    [
        ".ssh",
        ".aws",
        ".azure",
        ".kube",
        ".docker",
        ".git-credentials",
        ".npmrc",
        ".pypirc",
        ".env",
        ".bash_history",
        ".zsh_history",
        "ntuser.dat",
        "appdata/roaming/microsoft/credentials",
        "appdata/local/microsoft/credentials",
        "/config/sam",
        "/config/system",
        "/config/security",
    ]
    .iter()
    .any(|private_path| normalized.contains(private_path))
}

pub fn run_attached(command: &str) -> Result<CommandResult> {
    validate_run(command)?;
    let mut process = Command::new("cmd.exe");
    process.arg("/C").arg(command);
    capture_command(process, Stdio::inherit(), true)
}

pub fn run_exploration(command: &str) -> Result<CommandResult> {
    validate_explore(command)?;
    let mut process = Command::new("cmd.exe");
    process.arg("/C").arg(command);
    capture_command(process, Stdio::null(), false)
}

#[cfg(test)]
mod tests {
    use super::{run_attached, run_exploration, validate_explore, validate_run};

    #[test]
    fn allows_simple_read_only_commands() {
        assert!(validate_explore("dir").is_ok());
        assert!(validate_explore("dir /a").is_ok());
        assert!(validate_explore("dir \"C:\\Program Files\"").is_ok());
        assert!(validate_explore("where git").is_ok());
        assert!(validate_explore("whoami /groups").is_ok());
        assert!(validate_explore("ipconfig /all").is_ok());
        assert!(validate_explore("ver").is_ok());
        assert!(validate_explore("hostname").is_ok());
        assert!(validate_explore("tasklist").is_ok());
        assert!(validate_explore("getmac").is_ok());
    }

    #[test]
    fn run_allows_writes_and_file_reads() {
        assert!(validate_run("del foo").is_ok());
        assert!(validate_run("copy a b").is_ok());
        assert!(validate_run("type readme.txt").is_ok());
        assert!(validate_run("echo hello").is_ok());
        assert!(validate_run("dir /s").is_ok());
    }

    #[test]
    fn run_rejects_shell_hosts_and_system_tools() {
        assert!(validate_run("cmd /c dir").is_err());
        assert!(validate_run("cmd.exe /c dir").is_err());
        assert!(validate_run("powershell -Command dir").is_err());
        assert!(validate_run("bash -c dir").is_err());
        assert!(validate_run("wsl ls").is_err());
        assert!(validate_run("format.com D:").is_err());
        assert!(validate_run("reg query HKLM").is_err());
        assert!(validate_run("start notepad").is_err());
        assert!(validate_run("script.bat").is_err());
        assert!(validate_run("script.bat.exe").is_err());
    }

    #[test]
    fn run_rejects_operators_and_expansion() {
        assert!(validate_run("del foo & echo done").is_err());
        assert!(validate_run("del foo && echo done").is_err());
        assert!(validate_run("dir | more").is_err());
        assert!(validate_run("dir > out.txt").is_err());
        assert!(validate_run("del %USERPROFILE%\\foo").is_err());
        assert!(validate_run("del !USERPROFILE!\\foo").is_err());
        assert!(validate_run("dir; del foo").is_err());
    }

    #[test]
    fn rejects_shell_operators_and_expansion() {
        assert!(validate_explore("dir & del foo").is_err());
        assert!(validate_explore("dir && del foo").is_err());
        assert!(validate_explore("dir | more").is_err());
        assert!(validate_explore("dir > out.txt").is_err());
        assert!(validate_explore("dir < in.txt").is_err());
        assert!(validate_explore("dir %USERPROFILE%").is_err());
        assert!(validate_explore("dir !USERPROFILE!").is_err());
        assert!(validate_explore("dir ^& del foo").is_err());
        assert!(validate_explore("dir; del foo").is_err());
        assert!(validate_explore("dir\n").is_err());
    }

    #[test]
    fn rejects_dangerous_and_state_changing_commands() {
        assert!(validate_explore("del foo").is_err());
        assert!(validate_explore("erase foo").is_err());
        assert!(validate_explore("copy a b").is_err());
        assert!(validate_explore("move a b").is_err());
        assert!(validate_explore("type secret.txt").is_err());
        assert!(validate_explore("cmd /c dir").is_err());
        assert!(validate_explore("cmd.exe /c dir").is_err());
        assert!(validate_explore("powershell -Command dir").is_err());
        assert!(validate_explore("format.com D:").is_err());
        assert!(validate_explore("start notepad").is_err());
        assert!(validate_explore("script.bat").is_err());
        assert!(validate_explore("script.bat.exe").is_err());
    }

    #[test]
    fn rejects_recursive_and_state_changing_flags() {
        assert!(validate_explore("dir /s").is_err());
        assert!(validate_explore("dir /r").is_err());
        assert!(validate_explore("ipconfig /release").is_err());
        assert!(validate_explore("ipconfig /renew").is_err());
        assert!(validate_explore("ipconfig /flushdns").is_err());
    }

    #[test]
    fn rejects_private_paths() {
        assert!(validate_explore("dir C:\\Users\\me\\.ssh").is_err());
        assert!(validate_explore("dir C:\\Users\\me\\.aws").is_err());
        assert!(
            validate_explore("dir C:\\Users\\me\\AppData\\Roaming\\Microsoft\\Credentials")
                .is_err()
        );
        assert!(validate_explore("dir C:\\Windows\\System32\\config\\SAM").is_err());
    }

    #[test]
    fn runs_a_simple_command() {
        let result = run_exploration("ver").unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(!result.output.is_empty());
    }

    #[test]
    fn runs_a_confirmed_command() {
        let result = run_attached("echo askllm").unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("askllm"));
    }
}
