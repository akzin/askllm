use crate::command::{capture_command, CommandResult, MAX_COMMAND_BYTES};
use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};

// A speed bump, not a security boundary: every bash command is confirmed.
const BLOCKED_TOKENS: &[&str] = &[
    "add-type",
    "add-content",
    "ac",
    "auditpol",
    "bash",
    "bcdedit",
    "bitsadmin",
    "certutil",
    "chkdsk",
    "cleanmgr",
    "clear-content",
    "clear-disk",
    "clear-recyclebin",
    "clc",
    "cmd",
    "cscript",
    "defrag",
    "del",
    "dism",
    "diskpart",
    "enter-pssession",
    "erase",
    "format",
    "format-volume",
    "icacls",
    "icm",
    "iex",
    "ii",
    "import-module",
    "install-module",
    "initialize-disk",
    "invoke-command",
    "invoke-expression",
    "invoke-item",
    "kill",
    "logoff",
    "mshta",
    "net",
    "netsh",
    "new-itemproperty",
    "new-object",
    "new-partition",
    "new-pssession",
    "new-service",
    "out-file",
    "powercfg",
    "powershell",
    "pwsh",
    "rd",
    "reg",
    "regedit",
    "register-scheduledtask",
    "remove-partition",
    "resize-partition",
    "restart-computer",
    "rm",
    "rmdir",
    "ri",
    "robocopy",
    "rundll32",
    "saps",
    "sc",
    "schtasks",
    "secedit",
    "set-alias",
    "set-content",
    "set-disk",
    "set-executionpolicy",
    "set-itemproperty",
    "set-partition",
    "set-service",
    "set-variable",
    "set-volume",
    "sfc",
    "sh",
    "shutdown",
    "spps",
    "spsv",
    "start",
    "start-job",
    "start-process",
    "stop-computer",
    "stop-process",
    "stop-service",
    "takeown",
    "taskkill",
    "vssadmin",
    "wbadmin",
    "wevtutil",
    "wmic",
    "wscript",
    "wsl",
    "xcopy",
];

const BLOCKED_PREFIXES: &[&str] = &["remove-", "restart-", "stop-"];

pub fn validate_run(command: &str) -> Result<()> {
    if command.len() > MAX_COMMAND_BYTES {
        return Err(anyhow!("excessively long command"));
    }
    if command.chars().any(char::is_control) {
        return Err(anyhow!("control characters are not allowed"));
    }
    reject_operators(command)?;
    if is_private_path_request(command) {
        return Err(anyhow!("commands involving private files are blocked"));
    }
    for token in command_tokens(command) {
        let token = token.replace('`', "");
        let token = token.trim_matches(|character| matches!(character, '\'' | '"' | '$'));
        let token = token.rsplit('\\').next().unwrap_or(token);
        let token = token
            .strip_suffix(".exe")
            .or_else(|| token.strip_suffix(".com"))
            .unwrap_or(token);
        let token = token.to_ascii_lowercase();
        if token.is_empty() {
            continue;
        }
        if BLOCKED_TOKENS.contains(&token.as_str())
            || BLOCKED_PREFIXES
                .iter()
                .any(|prefix| token.starts_with(prefix))
        {
            return Err(anyhow!("command is blocked by the safety policy"));
        }
    }
    Ok(())
}

fn reject_operators(command: &str) -> Result<()> {
    let characters = command.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut quote = None;

    while index < characters.len() {
        let character = characters[index];
        if let Some(current) = quote {
            if current == '\'' {
                if character == '\'' {
                    if characters.get(index + 1) == Some(&'\'') {
                        index += 2;
                        continue;
                    }
                    quote = None;
                }
            } else if character == '`' {
                index += 2;
                continue;
            } else if character == '"' {
                quote = None;
            }
            index += 1;
            continue;
        }

        match character {
            '\'' | '"' => quote = Some(character),
            '&' => {
                if characters.get(index + 1) == Some(&'&') {
                    return Err(anyhow!("&& is not allowed; use ; to separate steps"));
                }
                return Err(anyhow!("the call operator & is not allowed"));
            }
            '|' => {
                if characters.get(index + 1) == Some(&'|') {
                    return Err(anyhow!("|| is not allowed; use ; to separate steps"));
                }
                if characters.get(index + 1) == Some(&'&') {
                    return Err(anyhow!("|& is not allowed"));
                }
            }
            '>' | '<' => return Err(anyhow!("redirection is not allowed")),
            _ => {}
        }
        index += 1;
    }

    Ok(())
}

fn command_tokens(command: &str) -> Vec<String> {
    let characters = command.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut index = 0;

    while index < characters.len() {
        let character = characters[index];
        if let Some(current_quote) = quote {
            current.push(character);
            if current_quote == '\'' {
                if character == '\'' {
                    if characters.get(index + 1) == Some(&'\'') {
                        current.push('\'');
                        index += 2;
                        continue;
                    }
                    quote = None;
                }
            } else if character == '`' {
                if let Some(escaped) = characters.get(index + 1) {
                    current.push(*escaped);
                    index += 2;
                    continue;
                }
            } else if character == '"' {
                quote = None;
            }
            index += 1;
            continue;
        }

        match character {
            '\'' | '"' => {
                quote = Some(character);
                current.push(character);
            }
            ';' | '|' | '(' | ')' | '{' | '}' => {
                push_first_token(&mut tokens, &mut current);
            }
            _ => current.push(character),
        }
        index += 1;
    }

    push_first_token(&mut tokens, &mut current);
    tokens
}

fn push_first_token(tokens: &mut Vec<String>, current: &mut String) {
    if let Some(token) = current.split_whitespace().next() {
        tokens.push(token.to_owned());
    }
    current.clear();
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
    let script = format!(
        "$ProgressPreference='SilentlyContinue'; \
         [Console]::OutputEncoding=[System.Text.Encoding]::UTF8; {command}"
    );
    let encoded = base64_utf16le(&script);
    let mut process = Command::new("powershell.exe");
    process
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-EncodedCommand")
        .arg(encoded);
    capture_command(process, Stdio::inherit(), true)
}

fn base64_utf16le(text: &str) -> String {
    let bytes = text
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    base64_encode(&bytes)
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let first = chunk[0] as u32;
        let second = chunk.get(1).copied().unwrap_or(0) as u32;
        let third = chunk.get(2).copied().unwrap_or(0) as u32;
        let combined = (first << 16) | (second << 8) | third;

        encoded.push(TABLE[((combined >> 18) & 63) as usize] as char);
        encoded.push(TABLE[((combined >> 12) & 63) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            TABLE[((combined >> 6) & 63) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            TABLE[(combined & 63) as usize] as char
        } else {
            '='
        });
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::{base64_utf16le, run_attached, validate_run};

    #[test]
    fn allows_powershell_syntax() {
        assert!(validate_run("Get-Date").is_ok());
        assert!(validate_run("Get-ChildItem | Sort-Object Name").is_ok());
        assert!(validate_run("Write-Output hi; Get-Date").is_ok());
        assert!(validate_run("Get-ChildItem -Path \"C:\\Program Files\"").is_ok());
        assert!(validate_run("$items = Get-ChildItem; $items.Count").is_ok());
        assert!(validate_run("Get-ChildItem C:\\format").is_ok());
        assert!(validate_run("Write-Output \"foo; Remove-Item bar\"").is_ok());
    }

    #[test]
    fn rejects_chains_and_redirection() {
        assert!(validate_run("Get-Date && Get-Date").is_err());
        assert!(validate_run("Get-Date || Get-Date").is_err());
        assert!(validate_run("Get-Date |& Get-Date").is_err());
        assert!(validate_run("Get-Date > out.txt").is_err());
        assert!(validate_run("Get-Content < in.txt").is_err());
        assert!(validate_run("& Get-Date").is_err());
    }

    #[test]
    fn rejects_dangerous_commands_and_aliases() {
        assert!(validate_run("iex 'Get-Date'").is_err());
        assert!(validate_run("Invoke-Expression 'Get-Date'").is_err());
        assert!(validate_run("Remove-Item foo").is_err());
        assert!(validate_run("rm foo").is_err());
        assert!(validate_run("del foo").is_err());
        assert!(validate_run("Start-Process notepad").is_err());
        assert!(validate_run("cmd /c dir").is_err());
        assert!(validate_run("powershell -Command dir").is_err());
        assert!(validate_run("wsl ls").is_err());
        assert!(validate_run("format D:").is_err());
        assert!(validate_run("format.com D:").is_err());
        assert!(validate_run("Stop-Process -Name foo").is_err());
        assert!(validate_run("Get-ChildItem | Remove-Item").is_err());
        assert!(validate_run("ForEach-Object { Remove-Item $_ }").is_err());
        assert!(validate_run("Set-Alias x Remove-Item").is_err());
    }

    #[test]
    fn rejects_private_paths() {
        assert!(validate_run("Get-Content C:\\Users\\me\\.ssh\\id_rsa").is_err());
        assert!(validate_run("Get-Content C:\\Users\\me\\.aws\\credentials").is_err());
        assert!(validate_run("Get-Content .env").is_err());
    }

    #[test]
    fn encodes_powershell_commands_as_utf16le_base64() {
        assert_eq!(base64_utf16le("exit"), "ZQB4AGkAdAA=");
        assert_eq!(base64_utf16le(""), "");
    }

    #[test]
    fn runs_a_simple_command() {
        let result = run_attached("Write-Output askllm").unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("askllm"));
        assert!(!result.output.contains("CLIXML"));
    }

    #[test]
    fn preserves_utf8_output() {
        let result = run_attached("Write-Output 'café'").unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("café"));
    }
}
