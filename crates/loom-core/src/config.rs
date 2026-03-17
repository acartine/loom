use serde::Deserialize;
use std::path::Path;

use crate::error::{LoomError, LoomResult};

#[derive(Debug, Clone, Deserialize)]
pub struct LoomConfig {
    pub workflow: WorkflowConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowConfig {
    pub name: String,
    pub version: u32,
    #[serde(default = "default_entry")]
    pub entry: String,
    pub default_profile: Option<String>,
}

fn default_entry() -> String {
    "workflow.loom".to_string()
}

pub fn load_config(path: &Path) -> LoomResult<LoomConfig> {
    let content = std::fs::read_to_string(path)?;
    parse_config(&content)
}

pub fn parse_config(content: &str) -> LoomResult<LoomConfig> {
    toml::from_str(content).map_err(|e| LoomError::Toml(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let content = r#"
[workflow]
name = "knots_sdlc"
version = 1
entry = "workflow.loom"
default_profile = "autopilot"
"#;
        let config = parse_config(content).unwrap();
        assert_eq!(config.workflow.name, "knots_sdlc");
        assert_eq!(config.workflow.version, 1);
        assert_eq!(
            config.workflow.default_profile,
            Some("autopilot".to_string())
        );
    }

    #[test]
    fn test_load_config() {
        let path = Path::new("../../tests/fixtures/knots_sdlc/loom.toml");
        let config = load_config(path).unwrap();
        assert_eq!(config.workflow.name, "knots_sdlc");
    }
}
