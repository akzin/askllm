use crate::config::Config;
use crate::protocol::{
    tool_definitions, ChatRequest, ChatResponse, Message, SseDecoder, ToolCall,
    ToolCallAccumulator, ToolDefinition,
};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub struct ChatClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
    reasoning_effort: Option<String>,
    tools_enabled: bool,
}

pub struct StreamReply {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

impl ChatClient {
    pub fn new(config: &Config) -> Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("failed to build the HTTP client")?;
        Ok(Self {
            http,
            base_url: config.base_url.trim_end_matches('/').to_owned(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            reasoning_effort: config.reasoning_effort.clone(),
            tools_enabled: config.command_execution_enabled,
        })
    }

    fn request(
        &self,
        messages: &[Message],
        tools: Vec<ToolDefinition>,
        stream: bool,
    ) -> ChatRequest {
        let mut request = ChatRequest::streaming(&self.model, messages.to_vec(), tools)
            .with_reasoning_effort(self.reasoning_effort.clone());
        request.stream = stream;
        request
    }

    fn post(&self, request: &ChatRequest) -> reqwest::RequestBuilder {
        let mut builder = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .header(CONTENT_TYPE, "application/json")
            .json(request);
        if let Some(api_key) = &self.api_key {
            builder = builder.header(AUTHORIZATION, "Bearer ".to_owned() + api_key);
        }
        builder
    }

    pub async fn stream<F>(&self, messages: &[Message], mut on_content: F) -> Result<StreamReply>
    where
        F: FnMut(&str) -> Result<()>,
    {
        let tools = if self.tools_enabled {
            tool_definitions()
        } else {
            Vec::new()
        };
        let request = self.request(messages, tools, true);
        let response = self.post(&request).send().await?.error_for_status()?;
        let mut stream = response.bytes_stream();
        let mut decoder = SseDecoder::default();
        let mut calls = ToolCallAccumulator::default();
        let mut answer = String::new();

        while let Some(chunk) = stream.next().await {
            for event in decoder.feed(&chunk?)? {
                if let Some(text) = event.content {
                    on_content(&text)?;
                    answer.push_str(&text);
                }
                calls.apply(event.tool_calls);
            }
        }
        for event in decoder.finish()? {
            if let Some(text) = event.content {
                on_content(&text)?;
                answer.push_str(&text);
            }
            calls.apply(event.tool_calls);
        }

        Ok(StreamReply {
            content: answer,
            tool_calls: calls.finish(),
        })
    }

    pub async fn tool_call(&self, messages: &[Message]) -> Result<Vec<ToolCall>> {
        let request = self.request(messages, tool_definitions(), false);
        let response: ChatResponse = self
            .post(&request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(response
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.tool_calls)
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::ChatClient;
    use crate::config::Config;
    use crate::protocol::Message;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::path::PathBuf;
    use std::thread::{self, JoinHandle};

    fn test_config(address: SocketAddr) -> Config {
        Config {
            base_url: format!("http://{address}"),
            model: "test".to_owned(),
            api_key: None,
            system_prompt: "test".to_owned(),
            reasoning_effort: None,
            command_execution_enabled: false,
            explore_allow: Vec::new(),
            explore_approved: false,
            config_path: PathBuf::new(),
        }
    }

    fn read_request(stream: &mut TcpStream) {
        let mut data = Vec::new();
        let mut buffer = [0; 1024];
        loop {
            let count = stream.read(&mut buffer).unwrap();
            if count == 0 {
                return;
            }
            data.extend_from_slice(&buffer[..count]);
            if let Some(header_end) = data.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&data[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                if data.len() >= header_end + 4 + content_length {
                    return;
                }
            }
        }
    }

    fn fake_sse_server(body: &'static str) -> (SocketAddr, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_request(&mut stream);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        });
        (address, handle)
    }

    #[tokio::test]
    async fn streams_content_from_an_sse_server() {
        let (address, handle) = fake_sse_server(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"world\"}}]}\n\ndata: [DONE]\n\n",
        );
        let client = ChatClient::new(&test_config(address)).unwrap();
        let mut streamed = String::new();
        let reply = client
            .stream(&[Message::user("hi")], |text| {
                streamed.push_str(text);
                Ok(())
            })
            .await
            .unwrap();
        handle.join().unwrap();

        assert_eq!(streamed, "hello world");
        assert_eq!(reply.content, "hello world");
        assert!(reply.tool_calls.is_empty());
    }

    #[tokio::test]
    async fn assembles_tool_calls_from_an_sse_server() {
        let body = r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"bash","arguments":"{\"command\":\"pwd\"}"}}]}}]}

data: [DONE]

"#;
        let (address, handle) = fake_sse_server(body);
        let client = ChatClient::new(&test_config(address)).unwrap();
        let reply = client
            .stream(&[Message::user("hi")], |_| Ok(()))
            .await
            .unwrap();
        handle.join().unwrap();

        assert_eq!(reply.tool_calls.len(), 1);
        assert_eq!(reply.tool_calls[0].function.name, "bash");
        assert_eq!(
            reply.tool_calls[0].function.arguments,
            r#"{"command":"pwd"}"#
        );
    }
}
