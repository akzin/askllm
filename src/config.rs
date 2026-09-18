use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub system_prompt: String,
}

impl Config {
    pub fn from_file(path: Option<&Path>) -> Result<Self> {
        let path = path.map(PathBuf::from).unwrap_or(default_config_path()?);
        let file = load_file_config(&path)?;
        let model = file
            .model
            .filter(|value| !value.trim().is_empty())
            .context("missing model in config")?;
        let base_url = file
            .base_url
            .filter(|value| !value.trim().is_empty())
            .context("missing base_url in config")?;
        let system_prompt = file
            .system_prompt
            .filter(|value| !value.trim().is_empty())
            .context("missing system_prompt in config")?;

        Ok(Self {
            base_url,
            model,
            api_key: file.api_key.filter(|value| !value.is_empty()),
            system_prompt,
        })
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    system_prompt: Option<String>,
}

fn default_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("could not determine user config directory")?;
    Ok(config_dir.join("askllm").join("config.toml"))
}

fn load_file_config(path: &Path) -> Result<FileConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file {}", path.display()))
}
