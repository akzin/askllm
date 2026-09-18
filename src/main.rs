use anyhow::{anyhow, Result};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("askllm: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let prompt = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if prompt.trim().is_empty() {
        return Err(anyhow!("usage: askllm <question>"));
    }

    let config = askllm::Config::from_file(None)?;
    askllm::ask(&config, &prompt).await
}
