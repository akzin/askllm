use crate::config::Config;
use crate::protocol::{ChatRequest, SseDecoder};
use anyhow::Result;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use std::io::{self, Write};

pub async fn ask(config: &Config, prompt: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let request = ChatRequest::streaming(&config.model, &config.system_prompt, prompt);

    let mut builder = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .json(&request);
    if let Some(api_key) = &config.api_key {
        builder = builder.header(AUTHORIZATION, format!("Bearer {api_key}"));
    }

    let response = builder.send().await?.error_for_status()?;
    let stdout = io::stdout();
    write_sse_stream(response.bytes_stream(), stdout.lock()).await
}

pub async fn write_sse_stream<S, W>(mut stream: S, mut output: W) -> Result<()>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin,
    W: Write,
{
    let mut decoder = SseDecoder::default();

    while let Some(chunk) = stream.next().await {
        for content in decoder.feed(&chunk?)?.into_iter().flatten() {
            write!(output, "{content}")?;
            output.flush()?;
        }
    }

    for content in decoder.finish()?.into_iter().flatten() {
        write!(output, "{content}")?;
    }
    writeln!(output)?;
    Ok(())
}
