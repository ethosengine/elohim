//! The projection-binding transform: canonical capability -> runtime bytes.
//! This is the "HOW" the V1 projection-binding metadata left as
//! additionalProperties. It lives here (not in eprfs-core) to keep core neutral.

use crate::canonical::CanonicalAgent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontmatterDialect {
    ClaudeAgentMd,
    CodexAgentMd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionBinding {
    pub id: String,
    pub runtime: String,
    pub source_type: String,
    pub target_pattern: String,
    pub dialect: FrontmatterDialect,
}

impl ProjectionBinding {
    pub fn claude_agent() -> Self {
        Self {
            id: "projection-binding-claude-agent".to_string(),
            runtime: "claude-code".to_string(),
            source_type: "agent".to_string(),
            target_pattern: ".claude/agents/{slug}.md".to_string(),
            dialect: FrontmatterDialect::ClaudeAgentMd,
        }
    }

    pub fn codex_agent() -> Self {
        Self {
            id: "projection-binding-codex-agent".to_string(),
            runtime: "codex".to_string(),
            source_type: "agent".to_string(),
            target_pattern: ".codex/agents/{slug}.md".to_string(),
            dialect: FrontmatterDialect::CodexAgentMd,
        }
    }

    pub fn target_path(&self, slug: &str) -> String {
        self.target_pattern.replace("{slug}", slug)
    }

    pub fn render(&self, agent: &CanonicalAgent) -> Vec<u8> {
        match self.dialect {
            FrontmatterDialect::ClaudeAgentMd => render_claude(agent),
            FrontmatterDialect::CodexAgentMd => render_codex(agent),
        }
    }
}

/// `.claude/agents/<slug>.md`: YAML frontmatter (fixed key order:
/// name, description, tools(csv), model, color, then sorted `extra`) + body.
fn render_claude(agent: &CanonicalAgent) -> Vec<u8> {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("name: {}\n", agent.slug));
    out.push_str(&format!("description: {}\n", agent.description));
    if !agent.tools.is_empty() {
        out.push_str(&format!("tools: {}\n", agent.tools.join(", ")));
    }
    if let Some(model) = &agent.model {
        out.push_str(&format!("model: {model}\n"));
    }
    if let Some(color) = &agent.color {
        out.push_str(&format!("color: {color}\n"));
    }
    for (key, value) in &agent.extra {
        // extra keys are preserved as scalar strings (round-trip fidelity for
        // any frontmatter beyond the known set).
        if let Some(scalar) = value.as_str() {
            out.push_str(&format!("{key}: {scalar}\n"));
        }
    }
    out.push_str("---\n\n");
    out.push_str(&agent.body);
    normalize(out.as_bytes()).into_bytes()
}

/// `.codex/agents/<slug>.md`: a distinct, greenfield surface from the SAME
/// source — heading + description + tools list + body. (The point is to prove
/// one canonical capability projects to two different runtime shapes.)
fn render_codex(agent: &CanonicalAgent) -> Vec<u8> {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", agent.slug));
    out.push_str(&format!("{}\n\n", agent.description));
    if !agent.tools.is_empty() {
        out.push_str("## Tools\n\n");
        for tool in &agent.tools {
            out.push_str(&format!("- {tool}\n"));
        }
        out.push('\n');
    }
    out.push_str(&agent.body);
    normalize(out.as_bytes()).into_bytes()
}

/// The normalization applied on BOTH sides of the round-trip compare:
/// strip per-line trailing whitespace; collapse to exactly one final newline.
/// Interior blank lines (e.g. the blank line between the frontmatter fence
/// and the body) are preserved — only *trailing* blank lines are stripped.
pub fn normalize(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let joined: String = text
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = joined.trim_end().to_string(); // strip trailing blank lines -> one final newline
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CanonicalAgent;

    fn sample() -> CanonicalAgent {
        CanonicalAgent::parse(
            "---\nname: code-reviewer\ndescription: Reviews the diff.\ntools: Task, Bash\nmodel: sonnet\ncolor: red\n---\n\nYou are the reviewer.\n",
        )
        .unwrap()
    }

    #[test]
    fn claude_target_path_expands_slug() {
        let b = ProjectionBinding::claude_agent();
        assert_eq!(
            b.target_path("code-reviewer"),
            ".claude/agents/code-reviewer.md"
        );
    }

    #[test]
    fn claude_render_reproduces_claude_frontmatter_shape() {
        let out = String::from_utf8(ProjectionBinding::claude_agent().render(&sample())).unwrap();
        assert!(out.starts_with("---\nname: code-reviewer\n"));
        assert!(out.contains("\ntools: Task, Bash\n"));
        assert!(out.contains("\nmodel: sonnet\n"));
        assert!(out.contains("\n---\n\nYou are the reviewer."));
    }

    #[test]
    fn codex_render_is_a_distinct_surface_from_same_source() {
        let out = String::from_utf8(ProjectionBinding::codex_agent().render(&sample())).unwrap();
        assert!(out.contains("# code-reviewer"));
        assert!(out.contains("Reviews the diff."));
        assert!(out.contains("You are the reviewer."));
        // codex path differs from claude path — one source, two surfaces
        assert_eq!(
            ProjectionBinding::codex_agent().target_path("code-reviewer"),
            ".codex/agents/code-reviewer.md"
        );
    }

    #[test]
    fn normalize_trims_trailing_ws_and_forces_single_final_newline() {
        assert_eq!(normalize(b"a  \nb\n\n\n"), "a\nb\n");
    }
}
