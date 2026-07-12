//! The canonical `agent` capability: normalized frontmatter + markdown body.
//! Identity is the slug (permanent); fingerprint is the BlobCid.

use std::collections::BTreeMap;

use eprfs_core::BlobCid;
use serde::{Deserialize, Serialize};

use crate::error::{AgentProjectionError, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalAgent {
    pub slug: String,
    pub description: String,
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub color: Option<String>,
    pub extra: BTreeMap<String, serde_yaml::Value>,
    pub body: String,
    /// OPTIONAL lineage: the parent package CIDs this envelope was composed from.
    /// Rendered in deterministic order (sorted by CID string) in
    /// [`canonical_bytes`](CanonicalAgent::canonical_bytes). Empty by default —
    /// SUPPORT ONLY: never auto-populated from frontmatter here. Because the
    /// lineage section is appended ONLY when at least one lineage field is
    /// present, an absent-lineage envelope hashes exactly as it did before these
    /// fields existed (no existing package CID moves).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parents: Vec<BlobCid>,
    /// OPTIONAL single-parent derivation pointer. `None` by default — SUPPORT
    /// ONLY, never auto-populated here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<BlobCid>,
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
        // Normalize line endings up front so a CRLF-fenced source (`---\r\n…`) is not
        // misclassified as MissingFrontmatter; a no-op for LF input.
        let source = source.replace("\r\n", "\n");
        let source = source.as_str();
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

        let raw = parse_frontmatter(yaml)?;

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
            // Lineage is SUPPORT-ONLY: parse never captures it (no auto-population).
            parents: Vec::new(),
            derived_from: None,
        })
    }

    /// Deterministic bytes for the fingerprint: slug, description, tools (in
    /// authored order), model, color, extra (key-sorted), then body — never
    /// path-derived.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = String::new();
        out.push_str("slug:");
        out.push_str(&self.slug);
        out.push_str("\ndescription:");
        out.push_str(&self.description);
        out.push_str("\ntools:");
        // Assumes tool names are comma-free identifiers — a tool name containing a comma
        // would be ambiguous under this join (indistinguishable from two separate tools).
        out.push_str(&self.tools.join(","));
        out.push_str("\nmodel:");
        out.push_str(self.model.as_deref().unwrap_or(""));
        out.push_str("\ncolor:");
        out.push_str(self.color.as_deref().unwrap_or(""));
        out.push_str("\nextra:");
        // `extra` is a BTreeMap, so iteration order is already key-sorted —
        // deterministic without an explicit sort step.
        for (key, value) in &self.extra {
            out.push('\n');
            out.push_str(key);
            out.push('=');
            out.push_str(serde_yaml::to_string(value).unwrap_or_default().trim());
        }
        out.push_str("\nbody:\n");
        out.push_str(&self.body);
        // OPTIONAL lineage — additive by construction. A section is appended ONLY
        // when at least one lineage field is present, so an absent-lineage envelope
        // produces the exact pre-lineage bytes and its CID is unchanged. Parents
        // render in sorted (CID-string) order, so insertion order never affects the
        // fingerprint.
        if !self.parents.is_empty() || self.derived_from.is_some() {
            out.push_str("\nparents:");
            let mut sorted: Vec<String> = self.parents.iter().map(BlobCid::to_string).collect();
            sorted.sort();
            for parent in &sorted {
                out.push('\n');
                out.push_str(parent);
            }
            out.push_str("\nderived_from:");
            if let Some(derived_from) = &self.derived_from {
                out.push_str(&derived_from.to_string());
            }
        }
        out.into_bytes()
    }

    pub fn cid(&self) -> BlobCid {
        BlobCid::compute(&self.canonical_bytes())
    }
}

/// Parse the YAML frontmatter, tolerating the real `.claude` corpus convention
/// of an unquoted `description:` scalar that embeds `: ` colon-space sequences
/// (from the `Examples: <example>Context: … user: '…' assistant: '…'` prose
/// convention) — illegal in a strict block-context YAML plain scalar.
///
/// Strict `serde_yaml` first, so well-formed frontmatter keeps its exact prior
/// semantics (quoting, typing, key order). Only when that fails do we lift the
/// reserved free-text `description` field line-wise (verbatim) and re-parse the
/// structured remainder — which still includes nested blocks like `mcpServers:`.
/// If lifting changes nothing, the error was genuinely structural: surface it.
fn parse_frontmatter(yaml: &str) -> Result<RawFrontmatter> {
    match serde_yaml::from_str::<RawFrontmatter>(yaml) {
        Ok(raw) => Ok(raw),
        Err(strict_err) => match lift_top_level_scalar(yaml, "description") {
            Some((description, remainder)) => {
                let mut raw: RawFrontmatter = serde_yaml::from_str(&remainder)?;
                raw.description = Some(description);
                Ok(raw)
            }
            None => Err(strict_err.into()),
        },
    }
}

/// Remove the first top-level (column-0) `key: <inline value>` line, returning
/// its trimmed verbatim value and the YAML remainder with that line deleted.
/// Verbatim — not YAML-parsed — so an embedded `: ` colon-space (legal in human
/// prose, illegal in a strict block-context plain scalar) survives intact.
/// `render_claude` re-emits `description: {value}` verbatim, so the round-trip
/// contract holds.
///
/// The lift is deliberately narrow: it fires ONLY for a genuine single-physical-
/// line inline scalar, and REFUSES (returns `None`, so the caller surfaces the
/// original YAML error) when the value spans multiple lines — either a block
/// scalar (`key: |` / `key: >`, whose real value is the indented block below) or
/// a folded/wrapped plain scalar (an indented continuation line follows). Lifting
/// only the first physical line of such a value would orphan its continuation
/// onto the preceding key and YAML-fold it into a silently-corrupt value (e.g. a
/// mangled slug driving cid + the target path). A value-less `key:`, an indented
/// (nested) key, and a false-prefix key (`keyX:`) are likewise never lifted, so
/// nested structures such as `mcpServers:` are untouched.
fn lift_top_level_scalar(yaml: &str, key: &str) -> Option<(String, String)> {
    let lines: Vec<&str> = yaml.lines().collect();
    let idx = lines.iter().position(|line| {
        line.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix(':'))
            .is_some_and(|v| !v.trim().is_empty())
    })?;
    let value = lines[idx]
        .strip_prefix(key)
        .and_then(|rest| rest.strip_prefix(':'))
        .expect("position() matched this exact `key:` prefix")
        .trim();
    // A block-scalar header (`|`/`>` + optional chomping/indent modifiers): the
    // real value is the indented block below, not the literal header — do not lift.
    if is_block_scalar_header(value) {
        return None;
    }
    // An indented continuation line follows: the scalar is wrapped across lines;
    // lifting only the first would fold the orphan onto the preceding key.
    if lines
        .get(idx + 1)
        .is_some_and(|next| next.starts_with([' ', '\t']))
    {
        return None;
    }
    let mut kept = lines;
    kept.remove(idx);
    Some((value.to_string(), kept.join("\n")))
}

/// A YAML block-scalar header: `|` or `>` followed only by chomping (`+`/`-`)
/// and/or an explicit indentation digit — e.g. `|`, `|-`, `>`, `>2+`. Signals
/// that the scalar's value is the indented block that follows.
fn is_block_scalar_header(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('|') | Some('>'))
        && chars.all(|c| c == '+' || c == '-' || c.is_ascii_digit())
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

    #[test]
    fn cid_reflects_canonical_bytes_and_changes_with_content() {
        // (a) cid() is not a constant — it's exactly BlobCid::compute over canonical_bytes().
        let agent = CanonicalAgent::parse(SAMPLE).unwrap();
        assert_eq!(agent.cid(), BlobCid::compute(&agent.canonical_bytes()));

        // (b) a body-only change produces a different cid.
        const SAMPLE_DIFFERENT_BODY: &str = "---\nname: code-reviewer\ndescription: Reviews the diff.\ntools: Task, Bash, Grep\nmodel: sonnet\ncolor: red\n---\n\nYou are a totally different specialist.\n";
        let different_body = CanonicalAgent::parse(SAMPLE_DIFFERENT_BODY).unwrap();
        assert_ne!(agent.cid(), different_body.cid());

        // (c) a description-only change also produces a different cid.
        const SAMPLE_DIFFERENT_DESCRIPTION: &str = "---\nname: code-reviewer\ndescription: A completely different description.\ntools: Task, Bash, Grep\nmodel: sonnet\ncolor: red\n---\n\nYou are the Code Review Specialist.\n";
        let different_description = CanonicalAgent::parse(SAMPLE_DIFFERENT_DESCRIPTION).unwrap();
        assert_ne!(agent.cid(), different_description.cid());
    }

    #[test]
    fn cid_changes_with_extra_frontmatter() {
        const SAMPLE_EXTRA_FOO: &str = "---\nname: code-reviewer\ndescription: Reviews the diff.\ntools: Task, Bash, Grep\nmodel: sonnet\ncolor: red\ncustom_field: foo\n---\n\nYou are the Code Review Specialist.\n";
        const SAMPLE_EXTRA_BAR: &str = "---\nname: code-reviewer\ndescription: Reviews the diff.\ntools: Task, Bash, Grep\nmodel: sonnet\ncolor: red\ncustom_field: bar\n---\n\nYou are the Code Review Specialist.\n";
        let foo = CanonicalAgent::parse(SAMPLE_EXTRA_FOO).unwrap();
        let bar = CanonicalAgent::parse(SAMPLE_EXTRA_BAR).unwrap();
        assert_ne!(foo.cid(), bar.cid());
    }

    #[test]
    fn parses_tools_as_yaml_block_list() {
        const SAMPLE_LIST: &str = "---\nname: code-reviewer\ndescription: Reviews the diff.\ntools:\n  - Task\n  - Bash\nmodel: sonnet\ncolor: red\n---\n\nBody text.\n";
        let agent = CanonicalAgent::parse(SAMPLE_LIST).unwrap();
        assert_eq!(agent.tools, vec!["Task", "Bash"]);
    }

    #[test]
    fn parses_tools_as_yaml_inline_list() {
        const SAMPLE_INLINE_LIST: &str = "---\nname: code-reviewer\ndescription: Reviews the diff.\ntools: [Task, Bash]\nmodel: sonnet\ncolor: red\n---\n\nBody text.\n";
        let agent = CanonicalAgent::parse(SAMPLE_INLINE_LIST).unwrap();
        assert_eq!(agent.tools, vec!["Task", "Bash"]);
    }

    #[test]
    fn extra_frontmatter_fields_are_preserved_losslessly() {
        const SAMPLE_WITH_EXTRA: &str = "---\nname: code-reviewer\ndescription: Reviews the diff.\ntools: Task, Bash\nmodel: sonnet\ncolor: red\ncustom_field: foo\n---\n\nBody text.\n";
        let agent = CanonicalAgent::parse(SAMPLE_WITH_EXTRA).unwrap();
        assert_eq!(
            agent.extra.get("custom_field").and_then(|v| v.as_str()),
            Some("foo")
        );
    }

    #[test]
    fn parses_crlf_fenced_frontmatter_identically_to_lf() {
        let crlf_sample = SAMPLE.replace('\n', "\r\n");
        let crlf_agent = CanonicalAgent::parse(&crlf_sample).unwrap();
        let lf_agent = CanonicalAgent::parse(SAMPLE).unwrap();
        assert_eq!(crlf_agent, lf_agent);
    }

    // The live `.claude/agents` corpus authors `description:` as one long unquoted
    // plain scalar embedding the `Examples: <example>Context: … user: '…'
    // assistant: '…'` prose convention — i.e. multiple `: ` colon-space sequences,
    // which are illegal in a strict block-context YAML plain scalar. Strict
    // `serde_yaml::from_str` over the whole frontmatter rejects every one of these
    // files ("mapping values are not allowed in this context"). The parser must
    // lift the reserved free-text field line-wise and still parse the remainder.
    const REAL_CORPUS: &str = "---\nname: code-reviewer\ndescription: Reviews the diff. Invoke when \"review my changes\". Examples: <example>Context: User finished a feature. user: 'done' assistant: 'let me review'</example>\ntools: Task, Bash\nmodel: sonnet\ncolor: red\n---\n\nYou are the reviewer.\n";

    #[test]
    fn parses_real_corpus_colon_laden_description() {
        let agent = CanonicalAgent::parse(REAL_CORPUS).unwrap();
        assert_eq!(agent.slug, "code-reviewer");
        // the embedded `: ` sequences survive verbatim in the lifted scalar
        assert!(agent
            .description
            .contains("Examples: <example>Context: User"));
        assert!(agent.description.contains("user: 'done'"));
        assert!(agent.description.contains("assistant: 'let me review'"));
        // the structured remainder still parses correctly
        assert_eq!(agent.tools, vec!["Task", "Bash"]);
        assert_eq!(agent.model.as_deref(), Some("sonnet"));
        assert_eq!(agent.color.as_deref(), Some("red"));
        assert!(agent.body.starts_with("You are the reviewer."));
    }

    #[test]
    fn lifts_description_without_disturbing_nested_extra() {
        // A real `mcpServers` agent: a colon-laden description AND a nested
        // `mcpServers` block (valid, indented YAML). Lifting `description`
        // line-wise must leave the nested structure intact for serde_yaml, so it
        // survives into `extra` as a real sequence — not dropped or flattened.
        const REAL_WITH_MCP: &str = "---\nname: after-action\ndescription: Post-incident review. Examples: <example>Context: incident. user: 'why' assistant: 'tracing'</example>\ntools: Task, Bash\nmcpServers:\n  - jenkins:\n      type: http\n      url: https://jenkins.example.com/mcp\nmodel: sonnet\ncolor: gold\n---\n\nYou are the reviewer.\n";
        let agent = CanonicalAgent::parse(REAL_WITH_MCP).unwrap();
        assert_eq!(agent.slug, "after-action");
        assert!(agent.description.contains("Examples:"));
        assert!(agent.extra.contains_key("mcpServers"));
        assert!(agent.extra.get("mcpServers").unwrap().is_sequence());
    }

    #[test]
    fn strict_parse_error_survives_when_description_is_not_the_cause() {
        // Genuinely-structural YAML garbage (not a liftable free-text scalar) must
        // still surface as a Yaml error, not be masked by the fallback path.
        const BROKEN: &str =
            "---\nname: x\ndescription: fine\n  : nonsense mapping\ntools: Task\n---\n\nbody\n";
        assert!(matches!(
            CanonicalAgent::parse(BROKEN),
            Err(AgentProjectionError::Yaml(_))
        ));
    }

    #[test]
    fn refuses_to_lift_a_wrapped_multiline_description() {
        // A colon-laden `description:` hand-wrapped across two physical lines:
        // strict YAML fails (colon-space), so the fallback fires — but lifting only
        // the FIRST physical line would orphan the indented continuation (`  three`)
        // onto the preceding `name:` key, YAML-folding it into a corrupt slug
        // (`x three`) that drives cid() and the `.claude/agents/{slug}.md` path.
        // The parser must REFUSE to lift and surface the structural error instead
        // of returning Ok with a silently-mangled slug.
        const WRAPPED: &str =
            "---\nname: x\ndescription: one: two\n  three\ntools: Task\n---\n\nbody\n";
        match CanonicalAgent::parse(WRAPPED) {
            Err(AgentProjectionError::Yaml(_)) => {} // correct: surfaced, not masked
            Ok(a) => panic!(
                "multi-line description must not silently lift: got slug={:?} description={:?}",
                a.slug, a.description
            ),
            Err(other) => panic!("expected a Yaml error, got {other:?}"),
        }
    }

    #[test]
    fn absent_lineage_canonical_bytes_and_cid_are_unchanged() {
        // GOLDEN vector. SAMPLE carries NO lineage, so its canonical_bytes MUST be
        // byte-identical to the pre-lineage output and its cid() MUST equal this
        // captured baseline. If either moves, every existing package CID moved — a
        // forbidden invariant break (the whole reason lineage is append-only).
        let agent = CanonicalAgent::parse(SAMPLE).unwrap();
        assert!(
            agent.parents.is_empty(),
            "parse must not auto-populate parents"
        );
        assert!(
            agent.derived_from.is_none(),
            "parse must not auto-populate derived_from"
        );
        let text = String::from_utf8(agent.canonical_bytes()).unwrap();
        assert!(
            !text.contains("\nparents:"),
            "absent lineage must NOT append a section"
        );
        assert!(!text.contains("\nderived_from:"));
        // Baseline cid captured from a pre-field build (single-sourced
        // BlobCid::compute over canonical_bytes).
        assert_eq!(
            agent.cid().to_string(),
            "bafyreic36hmigi34p4nf6s2l3sfpnlcuop7hlc6zzd7uee2q6ar2ekzioy"
        );
    }

    #[test]
    fn lineage_parents_are_order_independent_and_content_sensitive() {
        let base = CanonicalAgent::parse(SAMPLE).unwrap();
        let p1 = BlobCid::compute(b"parent-one");
        let p2 = BlobCid::compute(b"parent-two");

        // Same parents, different insertion order → identical canonical_bytes.
        let mut a = base.clone();
        a.parents = vec![p1.clone(), p2.clone()];
        let mut b = base.clone();
        b.parents = vec![p2.clone(), p1.clone()];
        assert_eq!(
            a.canonical_bytes(),
            b.canonical_bytes(),
            "parents must be sorted → insertion order cannot change the fingerprint"
        );
        assert_eq!(a.cid(), b.cid());

        // Adding a parent changes the cid (and differs from the no-lineage cid).
        let mut c = base.clone();
        c.parents = vec![p1.clone()];
        assert_ne!(a.cid(), c.cid(), "different parent sets → different cid");
        assert_ne!(base.cid(), c.cid(), "adding a parent moves the cid");

        // derived_from participates too.
        let mut d = base.clone();
        d.derived_from = Some(p1);
        assert_ne!(base.cid(), d.cid(), "setting derived_from moves the cid");
    }

    #[test]
    fn lineage_json_round_trips() {
        // Absent lineage: the fields are skipped in JSON (clean wire), and the
        // struct round-trips.
        let bare = CanonicalAgent::parse(SAMPLE).unwrap();
        let bare_json = serde_json::to_string(&bare).unwrap();
        assert!(
            !bare_json.contains("parents"),
            "empty parents must be skipped: {bare_json}"
        );
        assert!(
            !bare_json.contains("derivedFrom") && !bare_json.contains("derived_from"),
            "None derived_from must be skipped: {bare_json}"
        );
        let bare_back: CanonicalAgent = serde_json::from_str(&bare_json).unwrap();
        assert_eq!(bare, bare_back);

        // Present lineage: fields serialize and deserialize back identically.
        let mut with_lineage = bare.clone();
        with_lineage.parents = vec![
            BlobCid::compute(b"parent-one"),
            BlobCid::compute(b"parent-two"),
        ];
        with_lineage.derived_from = Some(BlobCid::compute(b"origin"));
        let json = serde_json::to_string(&with_lineage).unwrap();
        let back: CanonicalAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(with_lineage, back);
        assert_eq!(with_lineage.parents.len(), 2);
        assert!(back.derived_from.is_some());
    }

    #[test]
    fn valid_block_scalar_description_still_parses_via_strict_path() {
        // A block-scalar description is VALID YAML on its own: strict parsing
        // succeeds and the fallback never fires. The refuse-to-lift guard must not
        // disturb this (strict-first is unchanged code).
        const BLOCK_OK: &str =
            "---\nname: x\ndescription: |\n  Hello\n  World\ntools: Task\n---\n\nbody\n";
        let agent = CanonicalAgent::parse(BLOCK_OK).unwrap();
        assert_eq!(agent.slug, "x");
        assert!(agent.description.contains("Hello"));
        assert!(agent.description.contains("World"));
    }
}
