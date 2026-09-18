use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl ChatRequest {
    pub fn streaming(model: &str, system_prompt: &str, user_prompt: &str) -> Self {
        Self {
            model: model.to_owned(),
            messages: vec![
                Message {
                    role: "system".to_owned(),
                    content: system_prompt.to_owned(),
                },
                Message {
                    role: "user".to_owned(),
                    content: user_prompt.to_owned(),
                },
            ],
            stream: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

pub fn parse_sse_event(event: &str) -> Result<Option<String>> {
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
    Ok(chunk
        .choices
        .first()
        .and_then(|choice| choice.delta.content.clone()))
}

#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
}

impl SseDecoder {
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Option<String>>> {
        self.buffer.extend_from_slice(bytes);
        let mut events = Vec::new();

        while let Some(position) = self.buffer.windows(2).position(|window| window == b"\n\n") {
            let event = String::from_utf8(self.buffer.drain(..position + 2).collect())?;
            events.push(parse_sse_event(event.trim_end())?);
        }

        Ok(events)
    }

    pub fn finish(&mut self) -> Result<Vec<Option<String>>> {
        if self.buffer.is_empty() {
            return Ok(Vec::new());
        }

        let event = String::from_utf8(std::mem::take(&mut self.buffer))?;
        Ok(vec![parse_sse_event(event.trim_end())?])
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_sse_event, ChatRequest, SseDecoder};

    #[test]
    fn builds_streaming_request() {
        let request = ChatRequest::streaming("model", "system", "question");
        assert!(request.stream);
        assert_eq!(request.messages[0].role, "system");
        assert_eq!(request.messages[1].content, "question");
    }

    #[test]
    fn parses_content_and_done_events() {
        let event = r#"data: {"choices":[{"delta":{"content":"sudo snap refresh"}}]}"#;
        assert_eq!(
            parse_sse_event(event).unwrap().as_deref(),
            Some("sudo snap refresh")
        );
        assert_eq!(parse_sse_event("data: [DONE]").unwrap(), None);
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
        assert_eq!(events, vec![Some("sudo snap refresh".to_owned()), None]);
    }
}
