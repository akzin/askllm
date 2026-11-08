use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub system_prompt: String,
    pub reasoning_effort: Option<String>,
    pub command_execution_enabled: bool,
    pub explore_allow: Vec<String>,
    pub explore_approved: bool,
    pub config_path: PathBuf,
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
            reasoning_effort: file
                .reasoning_effort
                .filter(|value| !value.trim().is_empty()),
            command_execution_enabled: file.command_execution_enabled,
            explore_allow: file.explore.allow,
            explore_approved: file.explore.approved,
            config_path: path,
        })
    }

    pub fn save_explore_settings(&self) -> Result<()> {
        let contents = std::fs::read_to_string(&self.config_path).with_context(|| {
            format!("failed to read config file {}", self.config_path.display())
        })?;
        let mut document = contents
            .parse::<toml_edit::DocumentMut>()
            .with_context(|| {
                format!("failed to parse config file {}", self.config_path.display())
            })?;
        let explore = document
            .entry("explore")
            .or_insert(toml_edit::Item::Table(toml_edit::Table::new()))
            .as_table_mut()
            .context("explore config must be a TOML table")?;
        let mut allow = toml_edit::Array::new();
        for command in &self.explore_allow {
            allow.push(command.as_str());
        }
        explore["allow"] = toml_edit::value(allow);
        explore["approved"] = toml_edit::value(self.explore_approved);

        let contents = document.to_string();
        let temp_path = self.config_path.with_extension("toml.tmp");
        let permissions = std::fs::metadata(&self.config_path)
            .ok()
            .map(|metadata| metadata.permissions());
        std::fs::write(&temp_path, contents).with_context(|| {
            format!(
                "failed to write temporary config file {}",
                temp_path.display()
            )
        })?;
        if let Some(permissions) = permissions {
            std::fs::set_permissions(&temp_path, permissions).with_context(|| {
                format!("failed to preserve permissions for {}", temp_path.display())
            })?;
        }
        if let Err(error) = std::fs::rename(&temp_path, &self.config_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(error).with_context(|| {
                format!(
                    "failed to replace config file {}",
                    self.config_path.display()
                )
            });
        }
        Ok(())
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    system_prompt: Option<String>,
    #[serde(default = "default_reasoning_effort")]
    reasoning_effort: Option<String>,
    #[serde(default = "default_execution_enabled")]
    command_execution_enabled: bool,
    #[serde(default)]
    explore: ExploreConfig,
}

#[derive(Debug, Default, Deserialize)]
struct ExploreConfig {
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    approved: bool,
}

fn default_execution_enabled() -> bool {
    true
}

fn default_reasoning_effort() -> Option<String> {
    Some("none".to_owned())
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

#[cfg(test)]
mod tests {
    use super::Config;
    use std::path::PathBuf;

    fn config_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "askllm-config-test-{}-{label}.toml",
            std::process::id()
        ))
    }

    #[test]
    fn execution_defaults_to_enabled_and_explore_approval_persists() {
        let path = config_path("explore");
        std::fs::write(
            &path,
            "base_url = 'http://localhost/v1'\nmodel = 'test'\nsystem_prompt = 'brief'\n",
        )
        .unwrap();

        let mut config = Config::from_file(Some(&path)).unwrap();
        assert!(config.command_execution_enabled);
        assert_eq!(config.reasoning_effort.as_deref(), Some("none"));
        assert!(!config.explore_approved);
        config.explore_allow.push("id".to_owned());
        config.explore_approved = true;
        config.save_explore_settings().unwrap();

        let saved = Config::from_file(Some(&path)).unwrap();
        assert_eq!(saved.explore_allow, ["id"]);
        assert!(saved.explore_approved);
        assert!(!path.with_extension("toml.tmp").exists());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn saving_explore_settings_preserves_the_rest_of_the_file() {
        let path = config_path("preserve");
        std::fs::write(
            &path,
            "# top comment\nbase_url = 'http://localhost/v1'\nmodel = 'test'\nsystem_prompt = 'brief'\n\n# tooling stays\n[tooling]\nfoo = 'bar'\n",
        )
        .unwrap();

        let mut config = Config::from_file(Some(&path)).unwrap();
        config.explore_allow = vec!["id".to_owned(), "df -h".to_owned()];
        config.explore_approved = true;
        config.save_explore_settings().unwrap();
        config.save_explore_settings().unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("# top comment"));
        assert!(saved.contains("# tooling stays"));
        assert!(saved.contains("[tooling]"));
        assert!(saved.contains("foo = 'bar'"));
        assert_eq!(saved.matches("[explore]").count(), 1);

        let reloaded = Config::from_file(Some(&path)).unwrap();
        assert_eq!(reloaded.explore_allow, ["id", "df -h"]);
        assert!(reloaded.explore_approved);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn reasoning_can_be_configured_or_left_empty() {
        let path = config_path("reasoning");
        std::fs::write(
            &path,
            "base_url = 'http://localhost/v1'\nmodel = 'test'\nsystem_prompt = 'brief'\nreasoning_effort = 'medium'\n",
        )
        .unwrap();
        let config = Config::from_file(Some(&path)).unwrap();
        assert_eq!(config.reasoning_effort.as_deref(), Some("medium"));

        std::fs::write(
            &path,
            "base_url = 'http://localhost/v1'\nmodel = 'test'\nsystem_prompt = 'brief'\nreasoning_effort = ''\n",
        )
        .unwrap();
        let config = Config::from_file(Some(&path)).unwrap();
        assert_eq!(config.reasoning_effort, None);
        std::fs::remove_file(path).unwrap();
    }
}
