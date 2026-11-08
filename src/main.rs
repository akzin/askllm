use anyhow::Result;

#[tokio::main]
async fn main() {
    let prompt = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if prompt.trim().is_empty() {
        eprintln!("askllm: usage: askllm <question>");
        std::process::exit(2);
    }
    if let Err(error) = run(&prompt).await {
        eprintln!("askllm: {error:#}");
        std::process::exit(1);
    }
}

async fn run(prompt: &str) -> Result<()> {
    let config = askllm::Config::from_file(None)?;
    askllm::ask(&config, prompt).await
}
