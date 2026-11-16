use crate::chat::ChatClient;
use crate::config::Config;
use crate::protocol::Message;
use crate::tools::{self, ToolOutcome};
use crate::ui;
use anyhow::Result;

const TOOL_INSTRUCTIONS_POSIX: &str = r#"
You can run commands on the user's machine.
Use explore only when you need to check the system before answering; it runs one read-only
command without confirmation and must stay within the user's allowlist.
Use run for every command you propose, even when the user asks how to do something or only asks
for the command. The run tool runs bash. If your answer names a command the user could run, call
run with that exact command; never leave it as text only. The user confirms or declines, and the
next step follows after the result.
Before calling run, write a short, natural answer in the user's language that names the exact
command in backticks. Do not write a label like "proposed command"; write it the way you would
say it. Do not ask for confirmation yourself; the CLI asks after you call run.
Propose one chain at a time. You may chain steps with && and pipe with |, for example
`du -ah ~ | sort -rh | head`; never use ;, ||, |& or redirects. The user confirms the whole
visible chain once.
Always wrap URLs and values that contain &, ?, spaces or shell operators in quotes.
Never install or change packages to answer a question. Check whether a tool exists with explore
and a read-only command such as `command -v curl`; the run call must be the command from your
answer, never a setup step.
After a command result, the user already sees the output above. Reply with at most one short
sentence that says whether it worked or what it means. Never repeat output values, paths, names
or versions that are already visible, and do not claim changes the output does not show. If the
output is the answer, stop.
Set follow_up to false only when the command output is the complete answer and you have nothing
to add, for example pwd, ls or cat. Set it to true for state changes, warnings, failures, or when
another step follows. On a failure the CLI continues anyway.
Keep the answer brief.
IMPORTANT: every command you propose goes through the run tool. Never write a command as text
only; without the tool call the user cannot run it. Name the command in your answer in backticks
and then call run with exactly that command.
Do not use em-dashes in your answer.
"#;

const TOOL_INSTRUCTIONS_WINDOWS: &str = r#"
You can run commands on the user's machine.
Use explore only when you need to check the system before answering; it runs one read-only cmd
command without confirmation and must stay within the user's allowlist. Use it for quick checks
such as dir, where, whoami, ipconfig, ver, vol, hostname, tasklist and getmac. Never use pipes,
redirects, &, &&, ||, %, !, ^, ;, for, if or private paths in explore.
Use run for every command you propose, even when the user asks how to do something or only asks
for the command. The run tool runs a shell command; choose shell=cmd for one simple command, or
shell=powershell for everything else. If your answer names a command the user could run, call run
with that exact command and the chosen shell; never leave it as text only. The user confirms or
declines, and the next step follows after the result.
Before calling run, write a short, natural answer in the user's language that names the exact
command in backticks. Do not write a label like "proposed command"; write it the way you would
say it. Do not ask for confirmation yourself; the CLI asks after you call run.
Use cmd only for one simple command with no pipes, redirects, &, &&, ||, %, !, ^ or ;. Use
PowerShell 5.1 for everything else: separate steps with ; and pipe with |, for example
`Get-ChildItem | Sort-Object Length`; never use &&, ||, |& or redirects. The user confirms the
whole visible command once.
Always wrap paths and values that contain spaces or shell operators in quotes.
Never install or change packages to answer a question. Check whether a tool exists with explore
and a read-only command such as `where git`; the run call must be the command from your answer,
never a setup step.
Never use Invoke-Expression or iex, Start-Process, cmd /c, wsl, Remove-Item, del, rd, format,
diskpart, reg, schtasks or sc in PowerShell.
After a command result, the user already sees the output above. Reply with at most one short
sentence that says whether it worked or what it means. Never repeat output values, paths, names
or versions that are already visible, and do not claim changes the output does not show. If the
output is the answer, stop.
Set follow_up to false only when the command output is the complete answer and you have nothing
to add, for example Get-Location, Get-ChildItem or Get-Content. Set it to true for state changes,
warnings, failures, or when another step follows. On a failure the CLI continues anyway.
Keep the answer brief.
IMPORTANT: every command you propose goes through the run tool. Never write a command as text
only; without the tool call the user cannot run it. Name the command in your answer in backticks
and then call run with exactly that command and the chosen shell.
Do not use em-dashes.
"#;

fn tool_instructions() -> &'static str {
    if cfg!(windows) {
        TOOL_INSTRUCTIONS_WINDOWS
    } else {
        TOOL_INSTRUCTIONS_POSIX
    }
}

const DIRECT_INSTRUCTIONS: &str = r#"
Answer directly and briefly. Do not propose commands to run. Do not use em-dashes.
"#;

const MAX_TOOL_ROUNDS: usize = 12;

pub async fn ask(config: &Config, prompt: &str) -> Result<()> {
    let mut config = config.clone();
    let instructions = if config.command_execution_enabled {
        tool_instructions()
    } else {
        DIRECT_INSTRUCTIONS
    };
    let system_prompt = format!("{}\n\n{}", config.system_prompt, instructions);
    let mut messages = vec![Message::system(&system_prompt), Message::user(prompt)];
    let chat = ChatClient::new(&config)?;

    for _ in 0..MAX_TOOL_ROUNDS {
        let reply = chat.stream(&messages, ui::write_content).await?;
        if !reply.content.is_empty() && !reply.content.ends_with('\n') {
            ui::newline();
        }
        let mut tool_calls = reply.tool_calls;
        let mut nudged = false;

        if tool_calls.is_empty() {
            if !config.command_execution_enabled {
                return Ok(());
            }
            let Some(span) = first_command_span(&reply.content) else {
                return Ok(());
            };
            let nudge = if cfg!(windows) {
                format!(
                    "Your answer mentioned `{span}`. If that is a command the user can run, call \
                     the run tool now with exactly that command. Choose shell=cmd for one simple \
                     command, or shell=powershell otherwise. PowerShell 5.1: use ; for steps and \
                     | for pipelines; no &&, ||, |& or redirects. cmd: one simple command; no \
                     pipes, redirects, &, &&, ||, %, !, ^ or ;."
                )
            } else {
                format!(
                    "Your answer mentioned `{span}`. If that is a command the user can run, call \
                     the run tool now with exactly that command. Use && for steps and | for \
                     pipelines; no ;, ||, |& or redirects."
                )
            };
            messages.push(Message::assistant_text(&reply.content));
            messages.push(Message::user(&nudge));
            nudged = true;
            tool_calls = chat.tool_call(&messages).await?;
            if tool_calls.is_empty() {
                return Ok(());
            }
        }

        let assistant = if nudged {
            Message::assistant_tool_calls(tool_calls.clone())
        } else {
            Message::assistant_with_tool_calls(&reply.content, tool_calls.clone())
        };
        messages.push(assistant);
        for call in &tool_calls {
            match tools::handle(&mut config, call, &reply.content)? {
                ToolOutcome::Message(result) => messages.push(Message::tool(&call.id, result)),
                ToolOutcome::Done | ToolOutcome::Declined => return Ok(()),
            }
        }
    }

    Ok(())
}

fn first_command_span(text: &str) -> Option<&str> {
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        let end = after.find('`')?;
        let span = after[..end].trim();
        if !span.is_empty()
            && !span.contains('\n')
            && (span.starts_with("sudo ")
                || span.contains(' ')
                || matches!(span, "pwd" | "ls" | "whoami" | "id" | "tty" | "df" | "du"))
        {
            return Some(span);
        }
        rest = &after[end + 1..];
    }
    None
}

#[cfg(test)]
mod tests {
    use super::first_command_span;

    #[test]
    fn finds_a_command_span_for_the_retry() {
        assert_eq!(
            first_command_span("You can do that with `sudo apt upgrade`."),
            Some("sudo apt upgrade")
        );
        assert_eq!(
            first_command_span("Use `apt update` and then `apt upgrade`."),
            Some("apt update")
        );
        assert_eq!(
            first_command_span("The `?` and `&` block it; use `curl -s \"https://x/?a=1&b=2\"`."),
            Some("curl -s \"https://x/?a=1&b=2\"")
        );
        assert_eq!(first_command_span("That is caused by `systemd`."), None);
        assert_eq!(first_command_span("No code here."), None);
    }
}
