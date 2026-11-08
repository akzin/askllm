use crate::chat::ChatClient;
use crate::config::Config;
use crate::protocol::Message;
use crate::tools::{self, ToolOutcome};
use crate::ui;
use anyhow::Result;

const TOOL_INSTRUCTIONS: &str = r#"
You can run commands on the user's machine.
Use explore only when you need to check the system before answering; it runs one read-only
command without confirmation and must stay within the user's allowlist.
Use bash for every command you propose, even when the user asks how to do something or only asks
for the command. If your answer names a command the user could run, call bash with that exact
command; never leave it as text only. The user confirms or declines, and the next step follows
after the result.
Before calling bash, write a short, natural answer in the user's language that names the exact
command in backticks. Do not write a label like "proposed command"; write it the way you would
say it. Do not ask for confirmation yourself; the CLI asks after you call bash.
Propose one chain at a time. You may chain steps with && and pipe with |, for example
`du -ah ~ | sort -rh | head`; never use ;, ||, |& or redirects. The user confirms the whole
visible chain once.
Always wrap URLs and values that contain &, ?, spaces or shell operators in quotes.
Never install or change packages to answer a question. Check whether a tool exists with explore
and a read-only command such as `command -v curl`; the bash call must be the command from your
answer, never a setup step.
After a command result, the user already sees the output above. Reply with at most one short
sentence that says whether it worked or what it means. Never repeat output values, paths, names
or versions that are already visible, and do not claim changes the output does not show. If the
output is the answer, stop.
Set follow_up to false only when the command output is the complete answer and you have nothing
to add, for example pwd, ls or cat. Set it to true for state changes, warnings, failures, or when
another step follows. On a failure the CLI continues anyway.
Keep the answer brief.
IMPORTANT: every command you propose goes through the bash tool. Never write a command as text
only; without the tool call the user cannot run it. Name the command in your answer in backticks
and then call bash with exactly that command.
Do not use em-dashes in your answer.
"#;

const DIRECT_INSTRUCTIONS: &str = r#"
Answer directly and briefly. Do not propose commands to run. Do not use em-dashes.
"#;

const MAX_TOOL_ROUNDS: usize = 6;

pub async fn ask(config: &Config, prompt: &str) -> Result<()> {
    let mut config = config.clone();
    let instructions = if config.command_execution_enabled {
        TOOL_INSTRUCTIONS
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
            let nudge = format!(
                "Your answer mentioned `{span}`. If that is a command the user can run, call the \
                 bash tool now with exactly that command. Use && for steps and | for pipelines; \
                 no ;, ||, |& or redirects."
            );
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
