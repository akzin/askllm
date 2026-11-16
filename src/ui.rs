use crate::command::{ask_yes_no, confirm as ask_confirmation};
use crate::style;
use anyhow::Result;
use std::io::{self, Write};

pub fn write_content(text: &str) -> Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

pub fn newline() {
    println!();
}

pub fn probe_start(command: &str) {
    println!("{}", style::dim(&format!("▸ probing  {command}")));
}

pub fn probe_preview_line(output: &str) {
    println!("{}", style::dim(&format!("  → {}", probe_preview(output))));
}

pub fn command_start(shell: &str, command: &str) {
    println!("{}", style::dim(&format!("▸ {shell}\n  {command}")));
}

pub fn blocked(error: &str) {
    eprintln!("{}", style::dim(&format!("Execution blocked: {error}")));
}

pub fn command_failed(exit_code: i32) {
    eprintln!("Command failed (exit code {exit_code}).");
}

pub fn default_allowlist(allowlist: &[&str]) {
    eprintln!("{}", style::dim("Default read-only exploration commands:"));
    for command in allowlist {
        eprintln!("{}", style::dim(&format!("  {command}")));
    }
}

pub fn confirm() -> Result<bool> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    ask_confirmation(&mut stdin, &mut stdout)
}

pub fn ask_allow_once() -> Result<bool> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    ask_yes_no(
        &mut stdin,
        &mut stdout,
        "Allow once and save to the configuration? [yN] ",
    )
}

pub fn ask_add_to_allowlist(command: &str) -> Result<bool> {
    eprintln!(
        "{}",
        style::dim(&format!("Add to allowlist? {command} [yN]"))
    );
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    ask_yes_no(&mut stdin, &mut stdout, "")
}

pub fn probe_preview(output: &str) -> String {
    let line = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no output");
    let cleaned = line
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();
    let mut preview = cleaned.chars().take(200).collect::<String>();
    if cleaned.chars().count() > 200 {
        preview.push_str("...");
    }
    if preview.is_empty() {
        "no output".to_owned()
    } else {
        preview
    }
}

#[cfg(test)]
mod tests {
    use super::probe_preview;

    #[test]
    fn probe_preview_shows_the_first_useful_line() {
        assert_eq!(
            probe_preview("\n  /usr/bin/curl \nand more\n"),
            "/usr/bin/curl"
        );
        assert_eq!(probe_preview(""), "no output");
        assert_eq!(probe_preview("   \n\t\n"), "no output");
    }
}
