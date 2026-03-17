use indexmap::IndexMap;
use regex::Regex;
use serde::Deserialize;
use std::path::Path;

use crate::error::{LoomError, LoomResult};

/// Parsed prompt file
#[derive(Debug, Clone)]
pub struct PromptFile {
    pub accept: Vec<String>,
    pub success: IndexMap<String, String>,
    pub failure: IndexMap<String, String>,
    pub params: IndexMap<String, ParamDef>,
    pub body: String,
    /// Parameters referenced in the body via {{ name }}
    pub body_params: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ParamDef {
    #[serde(rename = "type")]
    pub param_type: ParamType,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    String,
    Int,
    Bool,
    Enum,
}

#[derive(Debug, Deserialize)]
struct PromptFrontmatter {
    #[serde(default)]
    accept: Vec<String>,
    #[serde(default)]
    success: IndexMap<String, String>,
    #[serde(default)]
    failure: IndexMap<String, String>,
    #[serde(default)]
    params: IndexMap<String, ParamDef>,
}

/// Parse a prompt markdown file with YAML frontmatter
pub fn parse_prompt(content: &str) -> LoomResult<PromptFile> {
    let (frontmatter_str, body) = split_frontmatter(content)?;
    let frontmatter: PromptFrontmatter =
        serde_yaml::from_str(&frontmatter_str).map_err(|e| LoomError::Yaml(e.to_string()))?;

    let body_params = extract_body_params(&body);

    Ok(PromptFile {
        accept: frontmatter.accept,
        success: frontmatter.success,
        failure: frontmatter.failure,
        params: frontmatter.params,
        body,
        body_params,
    })
}

/// Load and parse a prompt file from disk
pub fn load_prompt(path: &Path) -> LoomResult<PromptFile> {
    let content = std::fs::read_to_string(path)?;
    parse_prompt(&content)
}

fn split_frontmatter(content: &str) -> LoomResult<(String, String)> {
    let content = content.trim();
    if !content.starts_with("---") {
        return Err(LoomError::Parse {
            message: "prompt file must start with YAML frontmatter (---)".to_string(),
        });
    }

    let rest = &content[3..];
    let end = rest.find("---").ok_or(LoomError::Parse {
        message: "unterminated YAML frontmatter".to_string(),
    })?;

    let frontmatter = rest[..end].to_string();
    let body = rest[end + 3..].trim().to_string();
    Ok((frontmatter, body))
}

fn extract_body_params(body: &str) -> Vec<String> {
    let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
    let mut params: Vec<String> = re.captures_iter(body).map(|c| c[1].to_string()).collect();
    params.sort();
    params.dedup();
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_prompt() {
        let content = r#"---
accept:
  - Thing works
success:
  done: target_state
failure:
  fail: other_state
params:
  name:
    type: string
    description: The name
---

Do the {{ name }} thing.
"#;
        let prompt = parse_prompt(content).unwrap();
        assert_eq!(prompt.accept.len(), 1);
        assert_eq!(prompt.success.len(), 1);
        assert_eq!(prompt.failure.len(), 1);
        assert_eq!(prompt.params.len(), 1);
        assert_eq!(prompt.body_params, vec!["name"]);
    }

    #[test]
    fn test_parse_empty_params() {
        let content = r#"---
accept: []
success:
  done: target
failure: {}
params: {}
---

Body text.
"#;
        let prompt = parse_prompt(content).unwrap();
        assert!(prompt.params.is_empty());
        assert!(prompt.body_params.is_empty());
    }

    #[test]
    fn test_load_planning_prompt() {
        let path = Path::new("../../tests/fixtures/knots_sdlc/prompts/planning.md");
        let prompt = load_prompt(path).unwrap();
        assert_eq!(prompt.success.len(), 1);
        assert_eq!(prompt.failure.len(), 2);
        assert_eq!(prompt.params.len(), 1);
        assert!(prompt.params.contains_key("complexity"));
    }
}
