use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_owned(),
            content: content.to_owned(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_owned(),
            content: content.to_owned(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_owned(),
            content: String::new(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn assistant_with_tool_calls(content: &str, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_owned(),
            content: content.to_owned(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    pub fn assistant_text(content: &str) -> Self {
        Self {
            role: "assistant".to_owned(),
            content: content.to_owned(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn tool(tool_call_id: &str, content: String) -> Self {
        Self {
            role: "tool".to_owned(),
            content,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_owned()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Serialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl ChatRequest {
    pub fn streaming(model: &str, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Self {
        let tool_choice = (!tools.is_empty()).then(|| "auto".to_owned());
        Self {
            model: model.to_owned(),
            messages,
            stream: true,
            tools,
            tool_choice,
            reasoning_effort: None,
            temperature: None,
            top_p: None,
        }
    }

    pub fn with_reasoning_effort(mut self, effort: Option<String>) -> Self {
        self.reasoning_effort = effort;
        self
    }

    pub fn with_sampling(mut self, temperature: Option<f64>, top_p: Option<f64>) -> Self {
        self.temperature = temperature;
        self.top_p = top_p;
        self
    }
}

pub fn tool_definitions() -> Vec<ToolDefinition> {
    let (run_description, run_parameters, explore_description) = if cfg!(windows) {
        (
            "Run a shell command on the user's machine. The command runs only after the user \
             confirms it with y/N. Use run for anything that needs the user's approval: commands \
             outside the explore allowlist, network requests, live data, and actions that change \
             the system. You may use run during research as well as for the final command. Choose \
             shell=cmd only for one simple command; use shell=powershell for everything else, \
             including network requests. cmd has no pipes, redirects, &, &&, ||, %, !, ^ or ;. \
             PowerShell uses ; to separate steps and | to pipe; never use &&, ||, |& or redirects. \
             Set follow_up to false when the output speaks for itself.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The exact single-line command to run."
                    },
                    "follow_up": {
                        "type": "boolean",
                        "description": "Set false only when the command output is the complete answer and needs no explanation. Set true for state changes, warnings, failures, or when another step follows."
                    },
                    "shell": {
                        "type": "string",
                        "enum": ["cmd", "powershell"],
                        "default": "powershell",
                        "description": "Use cmd for one simple command; use powershell for everything else, including network requests."
                    }
                },
                "required": ["command"]
            }),
            "Run exactly one read-only cmd command for your own investigation before answering. \
             explore always uses cmd on Windows and has no shell parameter; it is not for \
             user-facing actions or network requests. Use it for simple checks like dir, where, \
             whoami, ipconfig, ver, vol, hostname, tasklist and getmac. The command must match the \
             user's allowlist and must not use pipes, redirects, &, &&, ||, %, !, ^, ; or private \
             files.",
        )
    } else {
        (
            "Run a bash command on the user's machine. The command runs only after the user \
             confirms it with y/N. Use run for anything that needs the user's approval, including \
             network requests and live data; you may use it during research as well as for the \
             final command. You may chain steps with && and pipe with |; never use ;, ||, |& or \
             redirects. Set follow_up to false when the output speaks for itself.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The exact single-line command to run."
                    },
                    "follow_up": {
                        "type": "boolean",
                        "description": "Set false only when the command output is the complete answer and needs no explanation. Set true for state changes, warnings, failures, or when another step follows."
                    }
                },
                "required": ["command"]
            }),
            "Run exactly one read-only sh command for your own investigation before answering. \
             explore always uses sh on Linux and has no shell parameter; it is not for user-facing \
             actions or network requests. The command must match the user's allowlist and must not \
             use pipes, redirects, semicolons, substitutions or private files.",
        )
    };

    vec![
        ToolDefinition {
            kind: "function".to_owned(),
            function: FunctionDefinition {
                name: "run".to_owned(),
                description: run_description.to_owned(),
                parameters: run_parameters,
            },
        },
        ToolDefinition {
            kind: "function".to_owned(),
            function: FunctionDefinition {
                name: "explore".to_owned(),
                description: explore_description.to_owned(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The exact single-line read-only command to run."
                        }
                    },
                    "required": ["command"]
                }),
            },
        },
    ]
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    #[serde(default)]
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponseMessage {
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: Delta,
}

#[derive(Debug, Default, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default, rename = "reasoning_content")]
    _reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallDelta>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ToolCallDelta {
    #[serde(default)]
    pub index: Option<usize>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub function: Option<FunctionDelta>,
}

#[derive(Debug, Default, Deserialize)]
pub struct FunctionDelta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

#[derive(Debug, Default)]
pub struct StreamEvent {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCallDelta>,
}

pub fn parse_sse_event(event: &str) -> Result<Option<StreamEvent>> {
    let data = event
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");

    if data.is_empty() || data == "[DONE]" {
        return Ok(None);
    }

    let chunk: StreamChunk = serde_json::from_str(&data).context("invalid SSE JSON event")?;
    let Some(choice) = chunk.choices.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(StreamEvent {
        content: choice.delta.content,
        tool_calls: choice.delta.tool_calls,
    }))
}

#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
}

const MAX_EVENT_BYTES: usize = 1024 * 1024;

impl SseDecoder {
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<StreamEvent>> {
        self.buffer.extend_from_slice(bytes);
        let mut events = Vec::new();

        while let Some((position, separator)) = next_event_boundary(&self.buffer) {
            if position > MAX_EVENT_BYTES {
                anyhow::bail!("SSE event exceeded {MAX_EVENT_BYTES} bytes");
            }
            let event = String::from_utf8(self.buffer.drain(..position + separator).collect())?;
            if let Some(event) = parse_sse_event(event.trim_end())? {
                events.push(event);
            }
        }
        if self.buffer.len() > MAX_EVENT_BYTES {
            anyhow::bail!("SSE event exceeded {MAX_EVENT_BYTES} bytes");
        }

        Ok(events)
    }

    pub fn finish(&mut self) -> Result<Vec<StreamEvent>> {
        if self.buffer.is_empty() {
            return Ok(Vec::new());
        }
        if self.buffer.len() > MAX_EVENT_BYTES {
            anyhow::bail!("SSE event exceeded {MAX_EVENT_BYTES} bytes");
        }

        let event = String::from_utf8(std::mem::take(&mut self.buffer))?;
        Ok(parse_sse_event(event.trim_end())?.into_iter().collect())
    }
}

fn next_event_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    let lf = buffer
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|position| (position, 2));
    let crlf = buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| (position, 4));
    match (lf, crlf) {
        (Some(lf), Some(crlf)) => Some(if lf.0 <= crlf.0 { lf } else { crlf }),
        (Some(boundary), None) | (None, Some(boundary)) => Some(boundary),
        (None, None) => None,
    }
}

#[derive(Debug, Default)]
pub struct ToolCallAccumulator {
    calls: Vec<PartialToolCall>,
}

#[derive(Debug, Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl ToolCallAccumulator {
    pub fn apply(&mut self, deltas: Vec<ToolCallDelta>) {
        for delta in deltas {
            let index = delta.index.unwrap_or(0);
            while self.calls.len() <= index {
                self.calls.push(PartialToolCall::default());
            }
            let call = &mut self.calls[index];
            if let Some(id) = delta.id {
                if call.id.is_empty() {
                    call.id = id;
                }
            }
            if let Some(function) = delta.function {
                if let Some(name) = function.name {
                    call.name.push_str(&name);
                }
                if let Some(arguments) = function.arguments {
                    call.arguments.push_str(&arguments);
                }
            }
        }
    }

    pub fn finish(self) -> Vec<ToolCall> {
        self.calls
            .into_iter()
            .filter(|call| !call.name.is_empty())
            .map(|call| ToolCall {
                id: call.id,
                kind: "function".to_owned(),
                function: FunctionCall {
                    name: call.name,
                    arguments: call.arguments,
                },
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_sse_event, tool_definitions, ChatRequest, Message, SseDecoder, ToolCallAccumulator,
        MAX_EVENT_BYTES,
    };

    fn feed_all(decoder: &mut SseDecoder, text: &str) -> Vec<super::StreamEvent> {
        decoder.feed(text.as_bytes()).unwrap()
    }

    #[test]
    fn builds_streaming_request_with_tools() {
        let request =
            ChatRequest::streaming("model", vec![Message::user("question")], tool_definitions())
                .with_reasoning_effort(Some("none".to_owned()));
        assert!(request.stream);
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.tools.len(), 2);
        assert_eq!(request.tools[0].function.name, "run");
        assert_eq!(request.tool_choice.as_deref(), Some("auto"));
        assert_eq!(request.reasoning_effort.as_deref(), Some("none"));
        assert_eq!(request.temperature, None);
        assert_eq!(request.top_p, None);
    }

    #[test]
    fn omits_sampling_parameters_when_unset() {
        let request =
            ChatRequest::streaming("model", vec![Message::user("question")], tool_definitions());
        let value = serde_json::to_value(&request).unwrap();
        assert!(value.get("temperature").is_none());
        assert!(value.get("top_p").is_none());
    }

    #[test]
    fn includes_sampling_parameters_when_set() {
        let request =
            ChatRequest::streaming("model", vec![Message::user("question")], tool_definitions())
                .with_sampling(Some(0.0), Some(1.0));
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["temperature"].as_f64(), Some(0.0));
        assert_eq!(value["top_p"].as_f64(), Some(1.0));
    }

    #[test]
    fn run_tool_documents_follow_up() {
        let schema = tool_definitions()[0].function.parameters.to_string();
        assert!(schema.contains("follow_up"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_run_tool_exposes_shell_choice() {
        let schema = tool_definitions()[0].function.parameters.to_string();
        assert!(schema.contains("shell"));
        assert!(schema.contains("cmd"));
        assert!(schema.contains("powershell"));
    }

    #[test]
    fn handles_crlf_events() {
        let mut decoder = SseDecoder::default();
        let events = decoder
            .feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\r\n\r\n")
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].content.as_deref(), Some("hi"));
    }

    #[test]
    fn handles_crlf_split_across_chunks() {
        let mut decoder = SseDecoder::default();
        assert!(decoder
            .feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\r\n")
            .unwrap()
            .is_empty());
        let events = decoder.feed(b"\r\n").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].content.as_deref(), Some("hi"));
    }

    #[test]
    fn rejects_oversized_events() {
        let mut decoder = SseDecoder::default();
        let data = vec![b'a'; MAX_EVENT_BYTES + 1];
        assert!(decoder.feed(&data).is_err());
    }

    #[test]
    fn parses_content_and_done_events() {
        let event = r#"data: {"choices":[{"delta":{"content":"sudo snap refresh"}}]}"#;
        let parsed = parse_sse_event(event).unwrap().unwrap();
        assert_eq!(parsed.content.as_deref(), Some("sudo snap refresh"));
        assert!(parse_sse_event("data: [DONE]").unwrap().is_none());
    }

    #[test]
    fn ignores_reasoning_content() {
        let event =
            r#"data: {"choices":[{"delta":{"reasoning_content":"thinking","content":null}}]}"#;
        let parsed = parse_sse_event(event).unwrap().unwrap();
        assert!(parsed.content.is_none());
    }

    #[test]
    fn handles_events_split_across_chunks() {
        let mut decoder = SseDecoder::default();
        assert!(decoder
            .feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"sudo ")
            .unwrap()
            .is_empty());
        let events = decoder
            .feed(b"snap refresh\"}}]}\n\ndata: [DONE]\n\n")
            .unwrap();
        let contents = events
            .into_iter()
            .filter_map(|event| event.content)
            .collect::<Vec<_>>();
        assert_eq!(contents, vec!["sudo snap refresh".to_owned()]);
    }

    #[test]
    fn assembles_streamed_tool_calls() {
        let mut decoder = SseDecoder::default();
        let first = feed_all(
            &mut decoder,
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"run\",\"arguments\":\"{\\\"com\"}}]},\"finish_reason\":null}]}\n\n",
        );
        let second = feed_all(
            &mut decoder,
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"mand\\\":\\\"pwd\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\n",
        );

        let mut accumulator = ToolCallAccumulator::default();
        for event in first.into_iter().chain(second) {
            accumulator.apply(event.tool_calls);
        }

        let calls = accumulator.finish();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].function.name, "run");
        assert_eq!(calls[0].function.arguments, r#"{"command":"pwd"}"#);
    }

    #[test]
    fn serializes_tool_messages_for_the_api() {
        let message = Message::tool("call_1", "ok".to_owned());
        let value = serde_json::to_value(&message).unwrap();
        assert_eq!(value["role"], "tool");
        assert_eq!(value["tool_call_id"], "call_1");
        assert!(value.get("tool_calls").is_none());
    }
}
