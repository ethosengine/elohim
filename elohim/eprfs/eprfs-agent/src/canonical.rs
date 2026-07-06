//! The canonical `agent` capability: normalized frontmatter + markdown body.
//! Identity is the slug (permanent); fingerprint is the BlobCid.

use std::collections::BTreeMap;

use eprfs_core::BlobCid;
use serde::Deserialize;

use crate::error::{AgentProjectionError, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalAgent {
    pub slug: String,
    pub description: String,
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub color: Option<String>,
    pub extra: BTreeMap<String, serde_yaml::Value>,
    pub body: String,
}

/// Raw frontmatter as authored (before normalization).
#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    tools: serde_yaml::Value,
    model: Option<String>,
    color: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_yaml::Value>,
}

impl CanonicalAgent {
    pub fn parse(source: &str) -> Result<CanonicalAgent> {
        // Frontmatter is fenced by a leading `---\n` and a closing `\n---`.
        let rest = source
            .strip_prefix("---\n")
            .ok_or(AgentProjectionError::MissingFrontmatter)?;
        let end = rest
            .find("\n---")
            .ok_or(AgentProjectionError::MissingFrontmatter)?;
        let yaml = &rest[..end];
        // Body is everything after the closing fence's line.
        let after = &rest[end + 4..]; // skip "\n---"
        let body = after.strip_prefix('\n').unwrap_or(after);
        let body = body.strip_prefix('\n').unwrap_or(body).to_string();

        let raw: RawFrontmatter = serde_yaml::from_str(yaml)?;

        let slug = raw.name.ok_or(AgentProjectionError::MissingField("name"))?;
        let description = raw
            .description
            .ok_or(AgentProjectionError::MissingField("description"))?;
        let tools = parse_tools(&raw.tools);

        Ok(CanonicalAgent {
            slug,
            description,
            tools,
            model: raw.model,
            color: raw.color,
            extra: raw.extra,
            body,
        })
    }

    /// Deterministic bytes for the fingerprint: slug, description, tools (in
    /// authored order), model, color, then body — never path-derived.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = String::new();
        out.push_str("slug:");
        out.push_str(&self.slug);
        out.push_str("\ndescription:");
        out.push_str(&self.description);
        out.push_str("\ntools:");
        out.push_str(&self.tools.join(","));
        out.push_str("\nmodel:");
        out.push_str(self.model.as_deref().unwrap_or(""));
        out.push_str("\ncolor:");
        out.push_str(self.color.as_deref().unwrap_or(""));
        out.push_str("\nbody:\n");
        out.push_str(&self.body);
        out.into_bytes()
    }

    pub fn cid(&self) -> BlobCid {
        BlobCid::compute(&self.canonical_bytes())
    }
}

/// `.claude` writes tools as a comma-separated string; a YAML list is also accepted.
fn parse_tools(value: &serde_yaml::Value) -> Vec<String> {
    match value {
        serde_yaml::Value::String(s) => s
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect(),
        serde_yaml::Value::Sequence(items) => items
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
            .filter(|t| !t.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\nname: code-reviewer\ndescription: Reviews the diff.\ntools: Task, Bash, Grep\nmodel: sonnet\ncolor: red\n---\n\nYou are the Code Review Specialist.\n";

    #[test]
    fn parses_claude_agent_frontmatter_and_body() {
        let agent = CanonicalAgent::parse(SAMPLE).unwrap();
        assert_eq!(agent.slug, "code-reviewer");
        assert_eq!(agent.description, "Reviews the diff.");
        assert_eq!(agent.tools, vec!["Task", "Bash", "Grep"]);
        assert_eq!(agent.model.as_deref(), Some("sonnet"));
        assert_eq!(agent.color.as_deref(), Some("red"));
        assert!(agent
            .body
            .starts_with("You are the Code Review Specialist."));
    }

    #[test]
    fn cid_is_stable_across_parse_roundtrip() {
        let a = CanonicalAgent::parse(SAMPLE).unwrap();
        let b = CanonicalAgent::parse(SAMPLE).unwrap();
        assert_eq!(a.cid(), b.cid());
    }

    #[test]
    fn missing_frontmatter_is_an_error() {
        assert!(matches!(
            CanonicalAgent::parse("no frontmatter here"),
            Err(AgentProjectionError::MissingFrontmatter)
        ));
    }
}
