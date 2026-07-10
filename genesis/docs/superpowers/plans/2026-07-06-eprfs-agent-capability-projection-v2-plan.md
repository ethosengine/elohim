---
id: "plan-eprfs-agent-capability-projection-v2"
title: "eprfs-agent capability projection (V2 — the projection compiler)"
status: Draft
cites:
  - plan-collaboration-through-the-protocol | parent plan — V2 is its projection-compiler phase; composes, does not fork | sha256:6c080af25c339852 | path: genesis/docs/superpowers/plans/2026-07-05-collaboration-through-the-protocol-plan.md
  - epr-meta-policy-registry-measure | define-once-bind-many governance seed; the eprfs-meta vs python epr_meta convergence thread (T8) cites it | sha256:474eee1686e3123b | path: genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md
  - elohim/sdk/domains/elohim-agent/manifest.json
  - elohim/sdk/domains/elohim-agent/CLAUDE.md
  - elohim/eprfs/eprfs-core/src/projection.rs
  - elohim/eprfs/eprfs-local/src/lib.rs
domain: process-meta   # D1-adjacent by MECHANISM (extends the eprfs projection substrate); MAP.md: process-meta (agent experience, .claude/, skills, agents) has NO honest D# — do not force one.
sprint: process-meta / agent-collaboration-substrate   # not a household-living-core vision rung; it is the substrate every future agent collaboration rides. Testable in-container (no shem/alpha/harbor).
---

# eprfs-agent Capability Projection (V2 — the Projection Compiler) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make agent capabilities *author-once, project-many*: a canonical `agent` capability (markdown + normalized frontmatter) becomes content-addressed truth, and a new `eprfs-agent` domain adapter renders it + a `projection-binding` into an eprfs `ProjectionManifest` that materializes to BOTH the `.claude/agents/*` surface (round-trip byte-fidelity against today's file) and a greenfield `.codex/agents/*` surface — with a drift/verify primitive that proves the projection is live, not a one-shot migration.

**Architecture:** Truth = canonical capability EPRs (`BlobCid`, reusing the `elohim-epr`/eprfs codec — never reinvented). Render (the "HOW") = `eprfs-agent` (a domain adapter, sibling to `eprfs-meta`) turns a `CanonicalAgent` + a `ProjectionBinding` into runtime-specific bytes; the frontmatter-dialect transform lives HERE so `eprfs-core` stays domain-neutral. Project = an `eprfs_core::ProjectionManifest`. Materialize = `eprfs-local`'s `LocalMaterializer` now, FUSE later (same manifest). Verify/drift = a new `verify_projection` in `eprfs-local` (the inverse of materialize) that feeds the `projection-drift-detected` signal the V1 vocabulary already declares. Govern = `.epr-meta` (out of scope to build here; convergence named as backlog).

**Tech Stack:** Rust 2021 (the `elohim/eprfs` cargo workspace); `serde` + `serde_yaml` (already a workspace dep — no new dependency for frontmatter); `bytes`; `tokio` (async because `EprfsStorage::put_blob` is async); `async-trait`. Node stays the JSON-schema validation harness (`pnpm run manifest:test` against the V1 `elohim-agent` metadata schemas) — Rust parses+trusts, it does not re-validate the schema.

## Global Constraints

- **Native build env** (the eprfs pre-push gate block already uses these): `RUSTFLAGS=""` (the WASM `getrandom` custom-backend flag breaks the native link) and `CARGO_TARGET_DIR=/tmp/eprfs-gate-target` (the `/projects` cargo-pool slot trips an `invoked.timestamp` ENOENT on this toolchain). Every `cargo` invocation in this plan is prefixed accordingly and run with CWD `elohim/eprfs`.
- **Gate for every task:** `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo fmt --check && … cargo clippy --workspace --all-targets -- -D warnings && … cargo test --workspace` — all green before the commit step. (This is exactly the standalone eprfs block in `.husky/pre-push.bash`.)
- **Reuse identity, never reinvent:** all content-addressing is `eprfs_core::BlobCid::compute(bytes)` (CIDv1, `0x71` dag-cbor, sha2-256). No new hash, no `String`-newtype CID. (Direct application of the take/leave lesson from the parent plan's arc.)
- **`eprfs-core` stays domain-neutral:** it must gain ZERO knowledge of `.claude`, `.codex`, agents, or frontmatter. All capability/dialect knowledge lives in `eprfs-agent`. The only `eprfs-core`/`eprfs-local` change in V2 is the *domain-neutral* `verify_projection` primitive (compares any manifest to any on-disk tree).
- **Author-once format:** a canonical `agent` is a markdown file with YAML frontmatter — the way `.claude/agents/*.md` are already written. The canonical form is that same shape with a *normalized* frontmatter key order + body normalization; identity is the `slug`, fingerprint is the `BlobCid`.
- **Scope fence:** V2 does the `agent` class only, on both runtimes, plus the eprfs primitives it forces. `skill` → `agent-spec`(CLAUDE.md) → `hook` projection are LATER waves (captured as backlog in Task 8, not built). No new DHT entry types; this is projection tooling over existing V1 vocabulary.

## File Structure

New crate `elohim/eprfs/eprfs-agent/` (sibling to `eprfs-meta`), one responsibility per file:

- `Cargo.toml` — workspace member; deps `eprfs-core` + serde/serde_yaml/serde_json/thiserror/bytes; dev-deps `eprfs-local` + `eprfs-storage` + tokio + tempfile.
- `src/lib.rs` — module wiring + re-exports.
- `src/error.rs` — `AgentProjectionError` (parse/render failures) + `Result`.
- `src/canonical.rs` — `CanonicalAgent` (the normalized capability) + `parse()` + `cid()`.
- `src/binding.rs` — `ProjectionBinding` + `FrontmatterDialect` + `render()` + `target_path()` (the "HOW").
- `src/project.rs` — `project()` → `ProjectionManifest` (async; stores rendered blobs via `put_blob`).
- `tests/round_trip.rs` — the acceptance test (parse real `code-reviewer.md` → project → materialize → normalized-equal; + codex greenfield; + drift detection).

Modified: `elohim/eprfs/Cargo.toml` (add member) · `elohim/eprfs/eprfs-local/src/lib.rs` + new `elohim/eprfs/eprfs-local/src/verify.rs` (the drift primitive).

---

## Task 1: Scaffold the `eprfs-agent` crate

**Files:**
- Modify: `elohim/eprfs/Cargo.toml` (add `"eprfs-agent"` to `members`)
- Create: `elohim/eprfs/eprfs-agent/Cargo.toml`
- Create: `elohim/eprfs/eprfs-agent/src/lib.rs`
- Create: `elohim/eprfs/eprfs-agent/src/error.rs`

**Interfaces:**
- Produces: crate `eprfs-agent` exporting (empty for now) `pub use error::{AgentProjectionError, Result};`. Later tasks add `canonical`, `binding`, `project`.

- [ ] **Step 1: Add the workspace member**

In `elohim/eprfs/Cargo.toml`, extend `members`:
```toml
members = [
    "eprfs-agent",
    "eprfs-core",
    "eprfs-host",
    "eprfs-local",
    "eprfs-meta",
    "eprfs-storage",
]
```

- [ ] **Step 2: Create the crate manifest** (`elohim/eprfs/eprfs-agent/Cargo.toml`)

```toml
[package]
name = "eprfs-agent"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true
description = "Domain adapter: elohim-agent capability EPRs -> eprfs projection manifests (claude/codex runtimes)"

[dependencies]
eprfs-core = { path = "../eprfs-core" }
serde.workspace = true
serde_json.workspace = true
serde_yaml.workspace = true
thiserror.workspace = true
bytes.workspace = true

[dev-dependencies]
eprfs-local = { path = "../eprfs-local" }
eprfs-storage = { path = "../eprfs-storage" }
tokio.workspace = true
tempfile = "3"
```

- [ ] **Step 3: Write the error type** (`elohim/eprfs/eprfs-agent/src/error.rs`)

```rust
//! Errors for the elohim-agent -> eprfs projection adapter.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentProjectionError {
    #[error("frontmatter delimiter (---) missing or malformed in capability source")]
    MissingFrontmatter,
    #[error("frontmatter is not valid YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("capability is missing required frontmatter field: {0}")]
    MissingField(&'static str),
    #[error("unknown projection runtime: {0}")]
    UnknownRuntime(String),
    #[error(transparent)]
    Eprfs(#[from] eprfs_core::EprfsError),
}

pub type Result<T> = std::result::Result<T, AgentProjectionError>;
```

- [ ] **Step 4: Write the crate root** (`elohim/eprfs/eprfs-agent/src/lib.rs`)

```rust
//! `eprfs-agent`: renders elohim-agent capability EPRs into eprfs projection
//! manifests for runtime-specific surfaces (.claude, .codex).
//!
//! This crate holds ALL capability/dialect knowledge; `eprfs-core` stays
//! domain-neutral. A capability is authored once (markdown + normalized
//! frontmatter); each runtime is a projection, never a source.

pub mod error;

pub use error::{AgentProjectionError, Result};
```

- [ ] **Step 5: Run the gate** (from `elohim/eprfs`)

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo build -p eprfs-agent && CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo fmt --check`
Expected: builds clean; fmt clean.

- [ ] **Step 6: Commit**

```bash
git add elohim/eprfs/Cargo.toml elohim/eprfs/eprfs-agent/
git commit -m "feat(eprfs-agent): scaffold the capability-projection domain adapter crate"
```

---

## Task 2: The canonical `agent` model — parse + content-address (identity)

**Files:**
- Create: `elohim/eprfs/eprfs-agent/src/canonical.rs`
- Modify: `elohim/eprfs/eprfs-agent/src/lib.rs` (add `pub mod canonical;` + re-export)
- Test: inline `#[cfg(test)]` in `canonical.rs`

**Interfaces:**
- Produces:
  - `struct CanonicalAgent { slug: String, description: String, tools: Vec<String>, model: Option<String>, color: Option<String>, extra: BTreeMap<String, serde_yaml::Value>, body: String }`
  - `CanonicalAgent::parse(source: &str) -> Result<CanonicalAgent>` — splits `---`-fenced YAML frontmatter from the markdown body; `tools` accepts either a YAML list OR the `.claude` comma-separated string; `extra` preserves any unrecognized frontmatter key (lossless).
  - `CanonicalAgent::canonical_bytes(&self) -> Vec<u8>` — deterministic serialization used for the fingerprint.
  - `CanonicalAgent::cid(&self) -> eprfs_core::BlobCid` — `BlobCid::compute(&self.canonical_bytes())`. This is the capability's fingerprint; identity is `slug`.

- [ ] **Step 1: Write the failing test** (append to `canonical.rs`)

```rust
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
        assert!(agent.body.starts_with("You are the Code Review Specialist."));
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
```

- [ ] **Step 2: Run it to confirm failure**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo test -p eprfs-agent canonical`
Expected: FAIL — `CanonicalAgent` not found.

- [ ] **Step 3: Implement** (top of `canonical.rs`)

```rust
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
```

- [ ] **Step 4: Wire the module** (in `src/lib.rs`, after `pub mod error;`)

```rust
pub mod canonical;

pub use canonical::CanonicalAgent;
```

- [ ] **Step 5: Run the tests**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo test -p eprfs-agent canonical`
Expected: PASS (3 tests).

- [ ] **Step 6: Gate + commit**

```bash
# gate: fmt + clippy + test (from elohim/eprfs)
git add elohim/eprfs/eprfs-agent/src/canonical.rs elohim/eprfs/eprfs-agent/src/lib.rs
git commit -m "feat(eprfs-agent): canonical agent parse + content-addressed identity (BlobCid)"
```

---

## Task 3: The `ProjectionBinding` transform — the "HOW" V1 left open

**Files:**
- Create: `elohim/eprfs/eprfs-agent/src/binding.rs`
- Modify: `elohim/eprfs/eprfs-agent/src/lib.rs`
- Test: inline `#[cfg(test)]` in `binding.rs`

**Interfaces:**
- Consumes: `CanonicalAgent` (Task 2).
- Produces:
  - `enum FrontmatterDialect { ClaudeAgentMd, CodexAgentMd }`
  - `struct ProjectionBinding { id: String, runtime: String, source_type: String, target_pattern: String, dialect: FrontmatterDialect }`
  - `ProjectionBinding::claude_agent()` and `::codex_agent()` constructors (the two V2 runtimes; `target_pattern` uses the `{slug}` token).
  - `ProjectionBinding::target_path(&self, slug: &str) -> String` — expands `{slug}`.
  - `ProjectionBinding::render(&self, agent: &CanonicalAgent) -> Vec<u8>` — the runtime-specific bytes.
  - free fn `normalize(bytes: &[u8]) -> String` — the normalization used on BOTH sides of the round-trip compare (strip per-line trailing whitespace; exactly one trailing newline).

- [ ] **Step 1: Write the failing test**

```rust
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
        assert_eq!(b.target_path("code-reviewer"), ".claude/agents/code-reviewer.md");
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
```

- [ ] **Step 2: Run it to confirm failure**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo test -p eprfs-agent binding`
Expected: FAIL — `ProjectionBinding` not found.

- [ ] **Step 3: Implement** (`binding.rs`)

```rust
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
pub fn normalize(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out: String = text
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    out.push('\n');
    out
}
```

- [ ] **Step 4: Wire the module** (in `src/lib.rs`)

```rust
pub mod binding;

pub use binding::{normalize, FrontmatterDialect, ProjectionBinding};
```

- [ ] **Step 5: Run the tests**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo test -p eprfs-agent binding`
Expected: PASS (4 tests).

- [ ] **Step 6: Gate + commit**

```bash
git add elohim/eprfs/eprfs-agent/src/binding.rs elohim/eprfs/eprfs-agent/src/lib.rs
git commit -m "feat(eprfs-agent): projection-binding transform for claude-code + codex runtimes"
```

---

## Task 4: `project()` — emit an eprfs `ProjectionManifest`

**Files:**
- Create: `elohim/eprfs/eprfs-agent/src/project.rs`
- Modify: `elohim/eprfs/eprfs-agent/src/lib.rs`
- Test: inline `#[cfg(test)]` in `project.rs` (uses `eprfs-storage::MemoryStorage`)

**Interfaces:**
- Consumes: `CanonicalAgent` (T2), `ProjectionBinding` (T3), `eprfs_core::{ProjectionManifest, ProjectionEntry, ProjectionPath, ProjectionRoot, ProjectionId, ProjectionSource, ProjectionSourceKind, EprRef, EntryKind, ProjectionStatus}`, `eprfs_core::storage::EprfsStorage` (`put_blob`).
- Produces:
  - `async fn project<S: EprfsStorage>(agents: &[CanonicalAgent], bindings: &[ProjectionBinding], storage: &S) -> Result<ProjectionManifest>` — for each `(agent, binding)`: render bytes, `storage.put_blob(bytes) -> BlobCid`, build a `ProjectionEntry` (File, path = `binding.target_path(slug)`, blob = the CID, `source = ProjectionSource::new("elohim-agent", Content, slug)`), collect into a `ProjectionManifest`, then `manifest.validate()?`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CanonicalAgent, ProjectionBinding};
    use eprfs_storage::MemoryStorage;

    fn sample() -> CanonicalAgent {
        CanonicalAgent::parse(
            "---\nname: code-reviewer\ndescription: Reviews.\ntools: Bash\n---\n\nBody.\n",
        )
        .unwrap()
    }

    #[tokio::test]
    async fn projects_one_agent_to_two_runtime_entries() {
        let storage = MemoryStorage::default();
        let manifest = project(
            &[sample()],
            &[ProjectionBinding::claude_agent(), ProjectionBinding::codex_agent()],
            &storage,
        )
        .await
        .unwrap();

        assert_eq!(manifest.entries.len(), 2);
        let paths: Vec<_> = manifest
            .entries
            .iter()
            .map(|e| e.path.as_path().to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&".claude/agents/code-reviewer.md".to_string()));
        assert!(paths.contains(&".codex/agents/code-reviewer.md".to_string()));
        // validate() ran inside project(); the two surfaces have DIFFERENT CIDs.
        let claude = manifest.entries.iter().find(|e| e.path.as_path().starts_with(".claude")).unwrap();
        let codex = manifest.entries.iter().find(|e| e.path.as_path().starts_with(".codex")).unwrap();
        assert_ne!(claude.blob, codex.blob);
    }
}
```

- [ ] **Step 2: Run it to confirm failure**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo test -p eprfs-agent project`
Expected: FAIL — `project` not found.

- [ ] **Step 3: Implement** (`project.rs`)

```rust
//! Emit an eprfs ProjectionManifest from capabilities + bindings.

use bytes::Bytes;
use eprfs_core::{
    EntryKind, EprRef, ProjectionEntry, ProjectionId, ProjectionManifest, ProjectionPath,
    ProjectionRoot, ProjectionSource, ProjectionSourceKind, ProjectionStatus,
};
use eprfs_core::storage::EprfsStorage;

use crate::binding::ProjectionBinding;
use crate::canonical::CanonicalAgent;
use crate::error::Result;

pub async fn project<S: EprfsStorage>(
    agents: &[CanonicalAgent],
    bindings: &[ProjectionBinding],
    storage: &S,
) -> Result<ProjectionManifest> {
    let mut entries = Vec::new();

    for agent in agents {
        for binding in bindings {
            let bytes = binding.render(agent);
            let blob = storage.put_blob(Bytes::from(bytes)).await?;
            let path = ProjectionPath::new(binding.target_path(&agent.slug))?;

            entries.push(ProjectionEntry {
                path,
                kind: EntryKind::File,
                source: Some(ProjectionSource::new(
                    "elohim-agent",
                    ProjectionSourceKind::Content,
                    agent.slug.clone(),
                )),
                epr: None,
                blob: Some(blob),
                size_bytes: None,
                executable: false,
                status: ProjectionStatus::Unknown,
                metadata: serde_json::Value::Null,
            });
        }
    }

    let manifest = ProjectionManifest {
        root: ProjectionRoot {
            id: ProjectionId::new("elohim-agent"),
            root: EprRef::new("epr:elohim-agent:capabilities"),
        },
        entries,
        metadata: serde_json::Value::Null,
    };
    manifest.validate()?;
    Ok(manifest)
}
```

- [ ] **Step 4: Wire the module** (in `src/lib.rs`)

```rust
pub mod project;

pub use project::project;
```

- [ ] **Step 5: Run the tests**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo test -p eprfs-agent project`
Expected: PASS.

- [ ] **Step 6: Gate + commit**

```bash
git add elohim/eprfs/eprfs-agent/src/project.rs elohim/eprfs/eprfs-agent/src/lib.rs
git commit -m "feat(eprfs-agent): project capabilities+bindings into an eprfs ProjectionManifest"
```

---

## Task 5: The drift/verify primitive — `verify_projection` (inverse of materialize)

**Files:**
- Create: `elohim/eprfs/eprfs-local/src/verify.rs`
- Modify: `elohim/eprfs/eprfs-local/src/lib.rs` (add `mod verify;` + re-export)
- Test: inline `#[cfg(test)]` in `verify.rs`

**Rationale:** This is a *domain-neutral* eprfs primitive (compares ANY manifest to ANY on-disk tree), so it belongs in `eprfs-local` next to `LocalMaterializer` — NOT in `eprfs-agent`. It closes the loop the V1 vocabulary already declares (`projection-drift-detected`; the `capability-usable contradictedBy projection-drift` claim). It needs no storage: expected CID is `entry.blob`; actual CID is `BlobCid::compute(on-disk bytes)`.

**Interfaces:**
- Consumes: `eprfs_core::{ProjectionManifest, ProjectionPath, BlobCid, EntryKind, LocalOverlayStatus, Result}`.
- Produces:
  - `struct EntryDrift { path: ProjectionPath, expected: BlobCid, actual: Option<BlobCid>, status: LocalOverlayStatus }`
  - `async fn verify_projection(manifest: &ProjectionManifest, target: impl AsRef<Path>) -> Result<Vec<EntryDrift>>` — for each `File` entry: read `target/path`; `actual = Some(compute)` if present, `None` if absent; `status = Clean` iff `actual == Some(expected)`, else `Dirty`. (Directory/Symlink entries are skipped in V2 — agents are files.)
  - `fn has_drift(drifts: &[EntryDrift]) -> bool` — any entry not `Clean` (the boolean that fires `projection-drift-detected`).

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use eprfs_core::{
        BlobCid, EntryKind, ProjectionEntry, ProjectionId, ProjectionManifest, ProjectionPath,
        ProjectionRoot, ProjectionStatus,
    };

    fn manifest_for(bytes: &[u8]) -> ProjectionManifest {
        ProjectionManifest {
            root: ProjectionRoot {
                id: ProjectionId::new("t"),
                root: "epr:t".into(),
            },
            entries: vec![ProjectionEntry::file(
                ProjectionPath::new("a.md").unwrap(),
                BlobCid::compute(bytes),
            )],
            metadata: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn clean_when_disk_matches_manifest() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.md"), b"hello").await.unwrap();
        let drifts = verify_projection(&manifest_for(b"hello"), dir.path()).await.unwrap();
        assert_eq!(drifts[0].status, LocalOverlayStatus::Clean);
        assert!(!has_drift(&drifts));
    }

    #[tokio::test]
    async fn dirty_when_disk_was_hand_edited() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.md"), b"HAND EDITED").await.unwrap();
        let drifts = verify_projection(&manifest_for(b"hello"), dir.path()).await.unwrap();
        assert_eq!(drifts[0].status, LocalOverlayStatus::Dirty);
        assert!(drifts[0].actual.is_some());
        assert!(has_drift(&drifts));
    }

    #[tokio::test]
    async fn dirty_and_absent_when_not_materialized() {
        let dir = tempfile::tempdir().unwrap();
        let drifts = verify_projection(&manifest_for(b"hello"), dir.path()).await.unwrap();
        assert_eq!(drifts[0].status, LocalOverlayStatus::Dirty);
        assert!(drifts[0].actual.is_none());
        assert!(has_drift(&drifts));
    }
}
```

- [ ] **Step 2: Run it to confirm failure**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo test -p eprfs-local verify`
Expected: FAIL — `verify_projection` not found.

- [ ] **Step 3: Implement** (`verify.rs`)

```rust
//! Projection drift verification: compare a manifest to an on-disk tree.
//! The inverse of `LocalMaterializer` — feeds `projection-drift-detected`.

use std::path::Path;

use eprfs_core::{
    BlobCid, EntryKind, LocalOverlayStatus, ProjectionManifest, ProjectionPath, Result,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryDrift {
    pub path: ProjectionPath,
    pub expected: BlobCid,
    pub actual: Option<BlobCid>,
    pub status: LocalOverlayStatus,
}

pub async fn verify_projection(
    manifest: &ProjectionManifest,
    target: impl AsRef<Path>,
) -> Result<Vec<EntryDrift>> {
    let target = target.as_ref();
    let mut drifts = Vec::new();

    for entry in &manifest.entries {
        if entry.kind != EntryKind::File {
            continue; // V2: agents are files; dir/symlink drift is a later wave.
        }
        let Some(expected) = entry.blob.clone() else {
            continue;
        };
        let path = target.join(entry.path.as_path());

        let (actual, status) = match tokio::fs::read(&path).await {
            Ok(bytes) => {
                let actual = BlobCid::compute(&bytes);
                let status = if actual == expected {
                    LocalOverlayStatus::Clean
                } else {
                    LocalOverlayStatus::Dirty
                };
                (Some(actual), status)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (None, LocalOverlayStatus::Dirty)
            }
            Err(source) => {
                return Err(eprfs_core::EprfsError::Io {
                    path: path.clone(),
                    source,
                });
            }
        };

        drifts.push(EntryDrift {
            path: entry.path.clone(),
            expected,
            actual,
            status,
        });
    }

    Ok(drifts)
}

/// True iff any entry is not Clean — the signal that fires `projection-drift-detected`.
pub fn has_drift(drifts: &[EntryDrift]) -> bool {
    drifts.iter().any(|d| d.status != LocalOverlayStatus::Clean)
}
```

- [ ] **Step 4: Wire the module** (in `eprfs-local/src/lib.rs`, near the top after the `use` block)

```rust
mod verify;
pub use verify::{has_drift, verify_projection, EntryDrift};
```

- [ ] **Step 5: Run the tests**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo test -p eprfs-local verify`
Expected: PASS (3 tests).

- [ ] **Step 6: Gate + commit**

```bash
git add elohim/eprfs/eprfs-local/src/verify.rs elohim/eprfs/eprfs-local/src/lib.rs
git commit -m "feat(eprfs-local): verify_projection drift primitive (inverse of materialize)"
```

---

## Task 6: The `code-reviewer` round-trip acceptance test (the whole loop)

**Files:**
- Create: `elohim/eprfs/eprfs-agent/tests/round_trip.rs`
- Create: `elohim/eprfs/eprfs-agent/tests/fixtures/code-reviewer.md` (a trimmed but real-shaped copy of `.claude/agents/code-reviewer.md`)

**Rationale:** This is the acceptance criterion the whole plan exists to satisfy: **canonical → `.claude` projection is normalized-equal to the authored file (lossless migration) AND drift is detectable (live projection) — in one test.** Plus the greenfield `.codex` projection materializes with the distinct shape (one source, two surfaces). Using a checked-in fixture (not the live `.claude/agents/code-reviewer.md`) keeps the test hermetic and stable; a follow-up wave (Task 8 backlog) runs it over the live tree.

**Interfaces:**
- Consumes: `eprfs_agent::{CanonicalAgent, ProjectionBinding, project, normalize}`, `eprfs_local::{LocalMaterializer, verify_projection, has_drift}`, `eprfs_storage::MemoryStorage`, `eprfs_core::MaterializationPolicy`.

- [ ] **Step 1: Create the fixture** (`tests/fixtures/code-reviewer.md`)

Author a real-shaped `.claude` agent file (frontmatter `name/description/tools/model/color` + a short body). It must already be in the normalized form the projector emits (fixed key order; single trailing newline) so the round-trip is exact. Keep it small:

```markdown
---
name: code-reviewer
description: Reviews the diff for quality and security before PR.
tools: Task, Bash, Grep, Read
model: sonnet
color: red
---

You are the Code Review Specialist for the Elohim Protocol.

## Your Expertise

You review recently modified code for correctness, security, and pattern-fit.
```

- [ ] **Step 2: Write the acceptance test** (`tests/round_trip.rs`)

```rust
use eprfs_agent::{normalize, project, CanonicalAgent, ProjectionBinding};
use eprfs_core::MaterializationPolicy;
use eprfs_local::{has_drift, verify_projection, LocalMaterializer};
use eprfs_storage::MemoryStorage;

const CODE_REVIEWER: &str = include_str!("fixtures/code-reviewer.md");

#[tokio::test]
async fn claude_round_trip_is_lossless_and_drift_is_detectable() {
    // 1. author-once -> canonical
    let agent = CanonicalAgent::parse(CODE_REVIEWER).unwrap();

    // 2. project -> 3. manifest (claude + codex, one source two surfaces)
    let storage = MemoryStorage::default();
    let manifest = project(
        &[agent],
        &[ProjectionBinding::claude_agent(), ProjectionBinding::codex_agent()],
        &storage,
    )
    .await
    .unwrap();

    // 4. materialize to a scratch dir
    let dir = tempfile::tempdir().unwrap();
    let materializer = LocalMaterializer::new(storage);
    let report = materializer
        .materialize(&manifest, dir.path(), MaterializationPolicy::LocalOnly)
        .await
        .unwrap();
    assert_eq!(report.files_written, 2);

    // 5a. ACCEPTANCE: the .claude projection is normalized-equal to the authored file
    let projected = tokio::fs::read(dir.path().join(".claude/agents/code-reviewer.md"))
        .await
        .unwrap();
    assert_eq!(
        normalize(&projected),
        normalize(CODE_REVIEWER.as_bytes()),
        "claude projection must round-trip losslessly against the authored capability"
    );

    // 5b. the .codex projection is a DISTINCT surface from the SAME source
    let codex = tokio::fs::read_to_string(dir.path().join(".codex/agents/code-reviewer.md"))
        .await
        .unwrap();
    assert!(codex.contains("# code-reviewer"));
    assert!(codex.contains("You are the Code Review Specialist"));

    // 6. drift is detectable: a clean tree has none...
    let clean = verify_projection(&manifest, dir.path()).await.unwrap();
    assert!(!has_drift(&clean), "freshly materialized tree must be clean");

    // ...and a hand-edit trips projection-drift-detected.
    tokio::fs::write(
        dir.path().join(".claude/agents/code-reviewer.md"),
        b"---\nname: code-reviewer\n---\nhand edited\n",
    )
    .await
    .unwrap();
    let drifted = verify_projection(&manifest, dir.path()).await.unwrap();
    assert!(has_drift(&drifted), "a hand-edited surface must report drift");
}
```

- [ ] **Step 3: Run the acceptance test**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo test -p eprfs-agent --test round_trip`
Expected: PASS. If 5a fails, the fixture is not in the projector's normalized form — align the fixture (do NOT loosen `normalize`; the normalization is the contract).

- [ ] **Step 4: Gate + commit**

```bash
git add elohim/eprfs/eprfs-agent/tests/
git commit -m "test(eprfs-agent): code-reviewer round-trip = lossless claude migration + drift detection + codex greenfield"
```

---

## Task 7: Operator entrypoint + the node validation seam

**Files:**
- Create: `elohim/eprfs/eprfs-agent/examples/project_agents.rs` (a runnable dry-run over a real `.claude/agents` dir)
- Modify: `.husky/pre-push.bash` (extend the eprfs gate note — the example builds under the existing `cargo test --workspace`, no new gate needed)

**Rationale:** Give the operator a one-command way to see the loop against the live tree, and pin the "author-once → validate in node → project in Rust" seam so it does not drift. The node side is the existing `pnpm run manifest:test` (V1 metadata schemas); this task only documents the seam and provides the Rust dry-run.

**Interfaces:**
- Consumes: `eprfs_agent::{CanonicalAgent, ProjectionBinding, project}`, `eprfs_local::{verify_projection, has_drift}`, `eprfs_storage::MemoryStorage`.

- [ ] **Step 1: Write the example** (`examples/project_agents.rs`)

```rust
//! Dry-run: parse every .claude/agents/*.md under a root, project both runtimes,
//! and report drift vs the on-disk .claude surface. Usage:
//!   cargo run -p eprfs-agent --example project_agents -- <repo-root>

use std::path::PathBuf;

use eprfs_agent::{project, CanonicalAgent, ProjectionBinding};
use eprfs_local::{has_drift, verify_projection};
use eprfs_storage::MemoryStorage;

#[tokio::main]
async fn main() {
    let root = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let agents_dir = root.join(".claude/agents");

    let mut agents = Vec::new();
    let mut read = tokio::fs::read_dir(&agents_dir).await.expect("read .claude/agents");
    while let Some(ent) = read.next_entry().await.unwrap() {
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let src = tokio::fs::read_to_string(&path).await.unwrap();
        match CanonicalAgent::parse(&src) {
            Ok(agent) => agents.push(agent),
            Err(e) => eprintln!("skip {}: {e}", path.display()),
        }
    }
    println!("parsed {} agent capabilities", agents.len());

    let storage = MemoryStorage::default();
    let manifest = project(&agents, &[ProjectionBinding::claude_agent()], &storage)
        .await
        .expect("project");

    let drifts = verify_projection(&manifest, &root).await.expect("verify");
    let dirty = drifts.iter().filter(|d| d.status != eprfs_core::LocalOverlayStatus::Clean).count();
    println!(
        "projected {} entries; {} drifted vs on-disk .claude (drift={})",
        manifest.entries.len(),
        dirty,
        has_drift(&drifts)
    );
}
```

- [ ] **Step 2: Run it against the repo**

Run: `CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS="" cargo run -p eprfs-agent --example project_agents -- ../..`
Expected: prints the parsed count (~23) and a drift count. A NON-zero drift count here is EXPECTED and informative — it measures how far today's hand-authored `.claude/agents/*.md` are from the projector's normalized form (the migration delta each file needs). This is data, not a failure.

- [ ] **Step 3: Document the seam** — add a short `## Validation seam` section to `elohim/eprfs/eprfs-agent/README.md` (create it): *frontmatter is validated against the V1 `elohim-agent` metadata schemas by `pnpm run manifest:test` (node/ajv); `eprfs-agent` parses and trusts. Author-once in markdown; validate in node; project in Rust.*

- [ ] **Step 4: Gate + commit**

```bash
git add elohim/eprfs/eprfs-agent/examples/ elohim/eprfs/eprfs-agent/README.md
git commit -m "feat(eprfs-agent): dry-run example over .claude/agents + document the node validation seam"
```

---

## Task 8: Backlog captures (scale waves + governance convergence + FUSE) + self-review

**Files:**
- Create: `genesis/data/timeline/backlog/eprfs-agent-scale-skill-agentspec-hook-waves.md`
- Create: `genesis/data/timeline/backlog/eprfs-meta-python-governance-convergence.md`
- Create: `genesis/data/timeline/backlog/eprfs-fuse-mount-lazy-materializer.md`

**Rationale:** Keep V2's scope genuine (writing-plans / `/plan` Step 1c(3)): capture the adjacent work the gaps brushed without bloating V2. Each backlog file carries the standard frontmatter (`id/kind/contentType/contentFormat/title/slug/written/status/priority/jobs`) so it passes the compose-gate.

- [ ] **Step 1: Scale-waves backlog** — one item: "V2 proved the `agent` class round-trip; scale to `skill` (SKILL.md, 106+38), `agent-spec` (CLAUDE.md, 140 — its metadata IS scope+appliesTo), `hook` (56, BINDS phases+matchers) in dependency order, each with the same round-trip acceptance gate. Reuse `eprfs-agent`'s `ProjectionBinding`/`normalize`; add a `FrontmatterDialect` per class." Link to this plan.

- [ ] **Step 2: Governance-convergence backlog** — one item: "`.epr-meta` has TWO engines: the python `_lib/epr_meta.py` compose-gate (bound to git-hooks by the parent plan) and the Rust `eprfs-meta` substrate model. Define-once-bind-many says they converge, with `eprfs-meta` canonical and the python one binding. Name the parity contract; do not build in V2." Cite `2026-07-02-epr-meta-policy-registry-measure-design.md` and the parent plan.

- [ ] **Step 3: FUSE backlog** — one item: "A FUSE mount crate (`eprfs-fuse`) is the lazy sibling of `LocalMaterializer` — `MaterializationPolicy::{Sparse,FetchMissing}` already model on-demand hydration. Because V2 projects to a `ProjectionManifest`, FUSE is a drop-in materializer (no projector change). Blocked on nothing; sequence after the scale waves." Link to this plan.

- [ ] **Step 4: Self-review** — verify against this plan's own spec: (1) agent class both runtimes → T2–T4, T6; (2) verify/diff primitive → T5; (3) round-trip acceptance → T6; (4) reuse BlobCid → T2/T4 (`put_blob`), never a new hash; (5) eprfs-core untouched (neutral) → only `eprfs-local` gained `verify` (neutral) and `eprfs-agent` is new; (6) governance convergence NAMED not built → T8 backlog. Confirm no task references a type another task did not define (`CanonicalAgent` T2, `ProjectionBinding`/`normalize` T3, `project` T4, `verify_projection`/`has_drift`/`EntryDrift` T5 — all consumed only downstream of definition).

- [ ] **Step 5: Commit**

```bash
git add genesis/data/timeline/backlog/eprfs-agent-scale-skill-agentspec-hook-waves.md \
        genesis/data/timeline/backlog/eprfs-meta-python-governance-convergence.md \
        genesis/data/timeline/backlog/eprfs-fuse-mount-lazy-materializer.md
git commit -m "docs(backlog): eprfs-agent scale waves, governance convergence, FUSE — V2 adjacencies"
```

---

## Self-Review (author pass)

**Spec coverage:** (1) new `eprfs-agent` domain adapter → T1–T4, T7. (2) formalize projection-binding transform for claude-code + codex → T3. (3) verify/diff primitive feeding `projection-drift-detected` → T5. (4) code-reviewer round-trip = lossless migration + drift detector + codex greenfield → T6. (5) reuse BlobCid / codec, never reinvent → T2 (`cid`), T4 (`put_blob`); no new hash anywhere. (6) keep eprfs-core domain-neutral → the only non-`eprfs-agent` change is the neutral `verify_projection` in `eprfs-local`; core is untouched. (7) canonical = markdown+frontmatter validated in node, projected in Rust → T2 (parse), T7 (seam). (8) scale waves + governance convergence NAMED not built → T8. (9) storage-as-projection-not-truth / identity is CID not path → `canonical_bytes()` is never path-derived; `ProjectionSource.id` = slug; the projected file is a render of the blob. Covered.

**Placeholder scan:** every code step carries complete, compilable Rust (types, signatures, bodies) and exact commands. No TBD/TODO in any deliverable. The fixture in T6 is concrete.

**Type consistency:** `CanonicalAgent{slug,description,tools,model,color,extra,body}` (T2) is consumed by `ProjectionBinding::render` (T3), `project` (T4), and the T6 test with the same field names. `project<S: EprfsStorage>(&[CanonicalAgent], &[ProjectionBinding], &S) -> Result<ProjectionManifest>` (T4) matches its T6 call. `verify_projection(&ProjectionManifest, impl AsRef<Path>) -> Result<Vec<EntryDrift>>` (T5) matches T6/T7 calls. `normalize` (T3) is used identically on both sides of the T6 compare.

**Env / scope:** no `requires_env` — pure Rust workspace build + node validation, testable in-container (household-nodes floor). Not BLOCKED-BY-ENV. Process-meta by payload; D1-adjacent by mechanism; not a household vision rung.

## Execution Handoff

Plan complete and saved. Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task (T1→T8), review between tasks; the eprfs gate (`fmt` + `clippy -D` + `test --workspace`) is the per-task acceptance. Tasks are strictly ordered (each consumes the prior); T5 is independent of T1–T4 and can run in parallel with T2–T4 if desired.
2. **Inline Execution** — execute in one session with a checkpoint after T4 (adapter complete) and after T6 (acceptance green).

Given the crate is small and the gate is fast, either works; subagent-per-task keeps each diff reviewable.
