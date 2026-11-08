use crate::command::{
    matches_allowlist, run_attached, run_exploration, validate_command, validate_command_chain,
    DEFAULT_EXPLORE_ALLOWLIST,
};
use crate::config::Config;
use crate::protocol::ToolCall;
use crate::ui;
use anyhow::Result;

const MAX_TOOL_OUTPUT_BYTES: usize = 24 * 1024;
const TRUNCATION_MARKER: &str = "\n[... output truncated ...]\n";

pub enum ToolOutcome {
    Message(String),
    Done,
    Declined,
}

pub fn handle(config: &mut Config, call: &ToolCall, content: &str) -> Result<ToolOutcome> {
    match call.function.name.as_str() {
        "bash" => run_bash(call, content),
        "explore" => run_explore(config, call).map(ToolOutcome::Message),
        other => Ok(ToolOutcome::Message(format!("error: unknown tool {other}"))),
    }
}

fn run_bash(call: &ToolCall, content: &str) -> Result<ToolOutcome> {
    let Some(arguments) = bash_arguments(&call.function.arguments) else {
        return Ok(ToolOutcome::Message(
            "error: missing or invalid command argument".to_owned(),
        ));
    };
    let command = &arguments.command;
    if let Err(error) = validate_command_chain(command) {
        ui::blocked(&format!("{error}"));
        return Ok(ToolOutcome::Message(format!(
            "error: nothing ran. The command was blocked: {error}. \
             Call bash again with one command; use && for steps and | for pipelines, \
             and never use ;, ||, |& or redirects."
        )));
    }
    if content.trim().is_empty() || !content.contains(command.as_str()) {
        return Ok(ToolOutcome::Message(
            "error: nothing ran. Answer the user first in one or two sentences and name the \
             exact command in backticks; then call bash with that same command."
                .to_owned(),
        ));
    }

    ui::bash_start(command);
    if !ui::confirm()? {
        return Ok(ToolOutcome::Declined);
    }

    ui::bash_end(command);
    let result = match run_attached(command) {
        Ok(result) => result,
        Err(error) => {
            return Ok(ToolOutcome::Message(format!(
                "error: could not start the command: {error:#}"
            )))
        }
    };
    if !result.output.ends_with('\n') {
        ui::newline();
    }
    if result.exit_code != 0 {
        ui::command_failed(result.exit_code);
    }
    if should_stop(arguments.follow_up, result.exit_code) {
        return Ok(ToolOutcome::Done);
    }
    Ok(ToolOutcome::Message(format!(
        "exit code: {}\noutput:\n{}",
        result.exit_code,
        bounded_output(&result.output)
    )))
}

fn should_stop(follow_up: bool, exit_code: i32) -> bool {
    !follow_up && exit_code == 0
}

fn run_explore(config: &mut Config, call: &ToolCall) -> Result<String> {
    let Some(command) = command_argument(&call.function.arguments) else {
        return Ok("error: missing or invalid command argument".to_owned());
    };
    if let Err(error) = validate_command(&command) {
        return Ok(format!(
            "error: the command was blocked by the safety policy: {error}"
        ));
    }

    if !config.explore_approved {
        ui::default_allowlist(DEFAULT_EXPLORE_ALLOWLIST);
        if !ui::ask_allow_once()? {
            return Ok("the user declined read-only exploration".to_owned());
        }
        if config.explore_allow.is_empty() {
            config.explore_allow = DEFAULT_EXPLORE_ALLOWLIST
                .iter()
                .map(|allowed| (*allowed).to_owned())
                .collect();
        }
        config.explore_approved = true;
        config.save_explore_settings()?;
    }

    if !matches_allowlist(&command, &config.explore_allow) {
        if !ui::ask_add_to_allowlist(&command)? {
            return Ok(
                "error: the command is not on the allowlist and the user declined".to_owned(),
            );
        }
        config.explore_allow.push(command.clone());
        config.save_explore_settings()?;
    }

    ui::probe_start(&command);
    let result = match run_exploration(&command) {
        Ok(result) => result,
        Err(error) => return Ok(format!("error: exploration failed: {error:#}")),
    };
    ui::probe_preview_line(&result.output);
    Ok(format!(
        "exit code: {}\noutput:\n{}",
        result.exit_code,
        bounded_output(&result.output)
    ))
}

fn command_argument(arguments: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Arguments {
        command: String,
    }

    let arguments: Arguments = serde_json::from_str(arguments).ok()?;
    let command = arguments.command.trim().to_owned();
    if command.is_empty() {
        None
    } else {
        Some(command)
    }
}

#[derive(serde::Deserialize)]
struct BashArguments {
    command: String,
    #[serde(default = "follow_up_default")]
    follow_up: bool,
}

fn follow_up_default() -> bool {
    true
}

fn bash_arguments(arguments: &str) -> Option<BashArguments> {
    let arguments: BashArguments = serde_json::from_str(arguments).ok()?;
    let command = arguments.command.trim().to_owned();
    if command.is_empty() {
        None
    } else {
        Some(BashArguments {
            command,
            follow_up: arguments.follow_up,
        })
    }
}

fn bounded_output(output: &str) -> String {
    if output.is_empty() {
        return "(no output)".to_owned();
    }
    if output.len() <= MAX_TOOL_OUTPUT_BYTES {
        return output.to_owned();
    }

    let remaining = MAX_TOOL_OUTPUT_BYTES - TRUNCATION_MARKER.len();
    let head_bytes = remaining / 2;
    let tail_bytes = remaining - head_bytes;
    let head_end = floor_char_boundary(output, head_bytes);
    let tail_start = ceil_char_boundary(output, output.len() - tail_bytes);
    format!(
        "{}{}{}",
        &output[..head_end],
        TRUNCATION_MARKER,
        &output[tail_start..]
    )
}

fn floor_char_boundary(value: &str, mut index: usize) -> usize {
    while index > 0 && !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(value: &str, mut index: usize) -> usize {
    while index < value.len() && !value.is_char_boundary(index) {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::{
        bash_arguments, bounded_output, command_argument, should_stop, MAX_TOOL_OUTPUT_BYTES,
    };

    #[test]
    fn parses_the_command_argument() {
        assert_eq!(
            command_argument(r#"{"command":"pwd"}"#).as_deref(),
            Some("pwd")
        );
        assert_eq!(command_argument(r#"{"command":"   "}"#), None);
        assert_eq!(command_argument("not json"), None);
    }

    #[test]
    fn parses_follow_up_from_bash_arguments() {
        let arguments = bash_arguments(r#"{"command":"pwd"}"#).unwrap();
        assert_eq!(arguments.command, "pwd");
        assert!(arguments.follow_up);

        let arguments = bash_arguments(r#"{"command":"pwd","follow_up":false}"#).unwrap();
        assert!(!arguments.follow_up);

        assert!(bash_arguments(r#"{"command":"   "}"#).is_none());
        assert!(bash_arguments("not json").is_none());
    }

    #[test]
    fn stops_only_after_a_successful_silent_command() {
        assert!(should_stop(false, 0));
        assert!(!should_stop(false, 1));
        assert!(!should_stop(true, 0));
        assert!(!should_stop(true, 1));
    }

    #[test]
    fn keeps_short_output_untouched() {
        assert_eq!(bounded_output("line\n"), "line\n");
        assert_eq!(bounded_output(""), "(no output)");
    }

    #[test]
    fn bounds_long_output_from_both_ends() {
        let output = "a".repeat(MAX_TOOL_OUTPUT_BYTES) + "END";
        let bounded = bounded_output(&output);
        assert!(bounded.contains("[... output truncated ...]"));
        assert!(bounded.ends_with("END"));
        assert!(bounded.len() <= MAX_TOOL_OUTPUT_BYTES);
        assert!(bounded.len() < output.len());
    }
}
