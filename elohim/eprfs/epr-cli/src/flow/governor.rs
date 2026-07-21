//! Governor auto-derivation (spec §3, `2026-07-21-sealed-contract-edges-governor-frontier-design.md`):
//! `.claude/epr-meta/governors.yaml` carries the versioned, ordered detector list; `epr flow
//! seal` runs it when `--governor` is absent, so agents hand-author only the residual
//! cross-boundary edges. Rule: **seal only the governance gap** — a detector match cites a
//! stronger mechanism (compiler, codegen byte-identity, schema-contract test) instead of
//! duplicating it with a fingerprint seal.
//!
//! Fail-soft by design: a missing/unreadable registry falls back to the built-in default
//! order with a printed note (derivation still works); an unknown rule key in a *present*
//! file is warn-and-skip, never a hard error — this sibling registry is deliberately softer
//! than `recipes.yaml`'s fail-loud validation (governors.yaml's own header explains why).

use std::path::{Path, PathBuf};

use elohim_epr_rea::Governor;
use serde::Deserialize;

use super::FlowResult;

/// One entry in the ordered detector list — the known vocabulary `governors.yaml` may name.
/// An unrecognized YAML string is warn-and-skip at load time, never a variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKey {
    /// Both endpoints `.rs` under the same cargo workspace root → `Governor::Compiler`.
    SameCargoWorkspace,
    /// A generated `.ts` (path contains `generated`/`generated-ts`) paired with its `.rs`/
    /// `.json` generator source, either direction → `Governor::Codegen`.
    GeneratedTs,
    /// A `elohim/sdk/schemas/v1/views/*.schema.json` paired with a `.rs`, either direction →
    /// `Governor::SchemaContract`.
    ViewSchema,
    /// Both endpoints doc-plane (`.md`/`.feature`) → `Governor::CiteSeal`.
    DocDoc,
    /// Everything else — the honest governance gap → `Governor::CiteSeal`.
    Residual,
}

impl RuleKey {
    fn parse(key: &str) -> Option<Self> {
        match key {
            "same-cargo-workspace" => Some(Self::SameCargoWorkspace),
            "generated-ts" => Some(Self::GeneratedTs),
            "view-schema" => Some(Self::ViewSchema),
            "doc-doc" => Some(Self::DocDoc),
            "residual" => Some(Self::Residual),
            _ => None,
        }
    }

    /// The stable rule-key string — what `derive_governor` reports as "the rule that fired".
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SameCargoWorkspace => "same-cargo-workspace",
            Self::GeneratedTs => "generated-ts",
            Self::ViewSchema => "view-schema",
            Self::DocDoc => "doc-doc",
            Self::Residual => "residual",
        }
    }
}

/// The built-in fallback order, used when the registry file is missing/unreadable — mirrors
/// `edge-governor-defaults@1`'s `derive:` list exactly (spec §3, governors.yaml header).
const DEFAULT_ORDER: [RuleKey; 5] = [
    RuleKey::SameCargoWorkspace,
    RuleKey::GeneratedTs,
    RuleKey::ViewSchema,
    RuleKey::DocDoc,
    RuleKey::Residual,
];

/// The policy id/version cited when the built-in fallback order is used.
const DEFAULT_POLICY: &str = "edge-governor-defaults@1";

/// The parsed, ready-to-run detector registry: an ordered rule list plus the `id@version`
/// policy string it was minted from (real file or built-in fallback).
#[derive(Debug, Clone)]
pub struct GovernorRegistry {
    pub derive_order: Vec<RuleKey>,
    pub policy: String,
}

impl GovernorRegistry {
    fn fallback() -> Self {
        Self {
            derive_order: DEFAULT_ORDER.to_vec(),
            policy: DEFAULT_POLICY.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// YAML shape (mirrors registry.rs's serde-yaml convention)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GovernorsYaml {
    #[allow(dead_code)]
    #[serde(rename = "edge-governors-version")]
    edge_governors_version: u32,
    policies: Vec<GovernorPolicy>,
}

#[derive(Debug, Deserialize)]
struct GovernorPolicy {
    id: String,
    version: u32,
    #[allow(dead_code)]
    title: String,
    status: String,
    #[allow(dead_code)]
    established_by: String,
    #[serde(default)]
    derive: Vec<String>,
    #[allow(dead_code)]
    #[serde(default)]
    why: String,
}

/// Load `.claude/epr-meta/governors.yaml`, resolve its active `edge-governor-defaults` row,
/// and return the ordered detector list. Fail-soft: a missing file, unreadable/malformed
/// YAML, or the absence of any `active` policy all fall back to [`DEFAULT_ORDER`] with a
/// printed note — derivation still works. A file that IS present but names an unknown rule
/// key is warn-and-skip: the key is dropped, the rest of the (still-valid) order is kept.
pub fn load_governor_registry(root: &Path) -> FlowResult<GovernorRegistry> {
    let path = root.join(".claude/epr-meta/governors.yaml");

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            println!(
                "note: governor registry not found at {} — using built-in default order ({DEFAULT_POLICY})",
                path.display()
            );
            return Ok(GovernorRegistry::fallback());
        }
    };

    let doc: GovernorsYaml = match serde_yaml::from_str(&text) {
        Ok(doc) => doc,
        Err(error) => {
            println!(
                "note: governor registry at {} is unreadable ({error}) — using built-in default order ({DEFAULT_POLICY})",
                path.display()
            );
            return Ok(GovernorRegistry::fallback());
        }
    };

    let Some(active) = doc
        .policies
        .iter()
        .find(|p| p.id == "edge-governor-defaults" && p.status == "active")
        .or_else(|| doc.policies.iter().find(|p| p.status == "active"))
    else {
        println!(
            "note: no active policy in {} — using built-in default order ({DEFAULT_POLICY})",
            path.display()
        );
        return Ok(GovernorRegistry::fallback());
    };

    let mut derive_order = Vec::with_capacity(active.derive.len());
    for key in &active.derive {
        match RuleKey::parse(key) {
            Some(rule) => derive_order.push(rule),
            None => println!(
                "warning: unknown governor rule key `{key}` in {} — skipping",
                path.display()
            ),
        }
    }

    Ok(GovernorRegistry {
        derive_order,
        policy: format!("{}@{}", active.id, active.version),
    })
}

// ---------------------------------------------------------------------------
// Detectors
// ---------------------------------------------------------------------------

/// Nearest ancestor Cargo.toml with a `[workspace]` table under `root`; if none, the nearest
/// ancestor Cargo.toml dir at all (a standalone crate root). `None` when no Cargo.toml is
/// found on the path up to and including `root` (repo has no root monorepo workspace —
/// nested single-crate workspaces are the norm, so most crates resolve to themselves here).
fn cargo_workspace_root(root: &Path, rel: &str) -> Option<PathBuf> {
    let file_abs = root.join(rel);
    let mut dir = file_abs.parent()?.to_path_buf();
    let mut standalone: Option<PathBuf> = None;
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(contents) = std::fs::read_to_string(&manifest) {
                if has_workspace_table(&contents) {
                    return Some(dir);
                }
                if standalone.is_none() {
                    standalone = Some(dir.clone());
                }
            }
        }
        if dir == root {
            break;
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => break,
        }
    }
    standalone
}

/// Whether `contents` declares a `[workspace]` table — a plain line scan (no toml parser
/// dependency), matching the section header exactly so `[workspace.package]` etc. don't
/// false-positive.
fn has_workspace_table(contents: &str) -> bool {
    contents.lines().any(|line| line.trim() == "[workspace]")
}

/// `same-cargo-workspace`: both endpoints `.rs`, resolving to the same workspace root.
fn same_cargo_workspace(root: &Path, from: &str, to: &str) -> Option<Governor> {
    if !(from.ends_with(".rs") && to.ends_with(".rs")) {
        return None;
    }
    let from_root = cargo_workspace_root(root, from)?;
    let to_root = cargo_workspace_root(root, to)?;
    if from_root != to_root {
        return None;
    }
    let name = from_root.file_name()?.to_str()?.to_string();
    Some(Governor::Compiler(name))
}

/// The repo-relative directory ending at (and including) a `generated`/`generated-ts` path
/// component, or `None` if `path` has no such component.
fn generated_dir(path: &str) -> Option<String> {
    let comps: Vec<&str> = path.split('/').collect();
    comps
        .iter()
        .position(|c| *c == "generated" || *c == "generated-ts")
        .map(|i| comps[..=i].join("/"))
}

/// `generated-ts`: one endpoint a generated `.ts`, the other its `.rs`/`.json` source —
/// either direction.
fn generated_ts(from: &str, to: &str) -> Option<Governor> {
    let is_generator_source = |p: &str| p.ends_with(".rs") || p.ends_with(".json");
    if from.ends_with(".ts") && is_generator_source(to) {
        if let Some(dir) = generated_dir(from) {
            return Some(Governor::Codegen(dir));
        }
    }
    if to.ends_with(".ts") && is_generator_source(from) {
        if let Some(dir) = generated_dir(to) {
            return Some(Governor::Codegen(dir));
        }
    }
    None
}

/// A `elohim/sdk/schemas/v1/views/*.schema.json` view schema path.
fn is_view_schema(path: &str) -> bool {
    path.starts_with("elohim/sdk/schemas/v1/views/") && path.ends_with(".schema.json")
}

/// `view-schema`: one endpoint a view schema, the other `.rs` — either direction.
fn view_schema(from: &str, to: &str) -> Option<Governor> {
    let contract =
        || Governor::SchemaContract("elohim/elohim-storage/tests/schema_contract.rs".to_string());
    if is_view_schema(from) && to.ends_with(".rs") {
        return Some(contract());
    }
    if is_view_schema(to) && from.ends_with(".rs") {
        return Some(contract());
    }
    None
}

/// A doc-plane artifact (`.md`/`.feature`).
fn is_doc(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".feature")
}

/// `doc-doc`: both endpoints doc-plane.
fn doc_doc(from: &str, to: &str) -> Option<Governor> {
    if is_doc(from) && is_doc(to) {
        Some(Governor::CiteSeal)
    } else {
        None
    }
}

/// Run one named detector.
fn apply_rule(root: &Path, from: &str, to: &str, rule: RuleKey) -> Option<Governor> {
    match rule {
        RuleKey::SameCargoWorkspace => same_cargo_workspace(root, from, to),
        RuleKey::GeneratedTs => generated_ts(from, to),
        RuleKey::ViewSchema => view_schema(from, to),
        RuleKey::DocDoc => doc_doc(from, to),
        RuleKey::Residual => Some(Governor::CiteSeal),
    }
}

/// Run `registry`'s detectors, in order, over `(from_rel, to_rel)`; the first match wins.
/// Returns the derived governor plus the rule key that fired. Falls back to `residual` (cite-
/// seal) even if the registry's own list somehow omits it — the honest governance gap is
/// always representable.
pub fn derive_governor(
    root: &Path,
    from_rel: &str,
    to_rel: &str,
    registry: &GovernorRegistry,
) -> (Governor, &'static str) {
    for rule in &registry.derive_order {
        if let Some(governor) = apply_rule(root, from_rel, to_rel, *rule) {
            return (governor, rule.as_str());
        }
    }
    (Governor::CiteSeal, RuleKey::Residual.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, contents: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    // ── same-cargo-workspace ─────────────────────────────────────────────────────

    #[test]
    fn same_cargo_workspace_pair_matches_and_derives_compiler() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write(
            root,
            "Cargo.toml",
            "[workspace]\nmembers = [\"crate_a\", \"crate_b\"]\n",
        );
        write(
            root,
            "crate_a/Cargo.toml",
            "[package]\nname = \"crate_a\"\n",
        );
        write(root, "crate_a/src/lib.rs", "// a\n");
        write(
            root,
            "crate_b/Cargo.toml",
            "[package]\nname = \"crate_b\"\n",
        );
        write(root, "crate_b/src/lib.rs", "// b\n");

        let expected_name = root.file_name().unwrap().to_str().unwrap().to_string();
        assert_eq!(
            same_cargo_workspace(root, "crate_a/src/lib.rs", "crate_b/src/lib.rs"),
            Some(Governor::Compiler(expected_name))
        );
    }

    #[test]
    fn two_standalone_crates_do_not_match_same_cargo_workspace() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write(
            root,
            "crate_a/Cargo.toml",
            "[package]\nname = \"crate_a\"\n",
        );
        write(root, "crate_a/src/lib.rs", "// a\n");
        write(
            root,
            "crate_b/Cargo.toml",
            "[package]\nname = \"crate_b\"\n",
        );
        write(root, "crate_b/src/lib.rs", "// b\n");

        // No shared workspace root exists — the repo's own norm (no root monorepo
        // workspace; nested single-crate workspaces).
        assert_eq!(
            same_cargo_workspace(root, "crate_a/src/lib.rs", "crate_b/src/lib.rs"),
            None
        );
    }

    #[test]
    fn single_crate_workspace_pattern_is_detected_via_own_workspace_table() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write(
            root,
            "onlycrate/Cargo.toml",
            "[package]\nname = \"onlycrate\"\n\n[workspace]\n",
        );
        write(root, "onlycrate/src/a.rs", "// a\n");
        write(root, "onlycrate/src/b.rs", "// b\n");

        assert_eq!(
            same_cargo_workspace(root, "onlycrate/src/a.rs", "onlycrate/src/b.rs"),
            Some(Governor::Compiler("onlycrate".into()))
        );
    }

    #[test]
    fn same_cargo_workspace_requires_both_endpoints_rust() {
        assert_eq!(
            same_cargo_workspace(Path::new("/tmp"), "a.rs", "a.ts"),
            None
        );
    }

    // ── generated-ts ─────────────────────────────────────────────────────────────

    #[test]
    fn generated_ts_matches_ts_generated_to_rs() {
        assert_eq!(
            generated_ts(
                "app/elohim-app/src/app/generated/manifest-types.ts",
                "elohim/elohim-storage/src/views.rs",
            ),
            Some(Governor::Codegen("app/elohim-app/src/app/generated".into()))
        );
    }

    #[test]
    fn generated_ts_matches_rs_to_ts_generated_reverse_direction() {
        assert_eq!(
            generated_ts(
                "elohim/elohim-storage/src/views.rs",
                "elohim/sdk/storage-client-ts/src/generated/index.ts",
            ),
            Some(Governor::Codegen(
                "elohim/sdk/storage-client-ts/src/generated".into()
            ))
        );
    }

    #[test]
    fn generated_ts_recognizes_generated_dash_ts_component_paired_with_json() {
        assert_eq!(
            generated_ts("sdk/foo/generated-ts/bar.ts", "sdk/foo/source.json"),
            Some(Governor::Codegen("sdk/foo/generated-ts".into()))
        );
    }

    #[test]
    fn generated_ts_does_not_match_a_ts_file_outside_a_generated_dir() {
        assert_eq!(
            generated_ts(
                "app/src/hand-written.ts",
                "elohim/elohim-storage/src/views.rs"
            ),
            None
        );
    }

    // ── view-schema ──────────────────────────────────────────────────────────────

    #[test]
    fn view_schema_matches_rs_to_schema() {
        assert_eq!(
            view_schema(
                "elohim/elohim-storage/src/views.rs",
                "elohim/sdk/schemas/v1/views/resilience.schema.json",
            ),
            Some(Governor::SchemaContract(
                "elohim/elohim-storage/tests/schema_contract.rs".into()
            ))
        );
    }

    #[test]
    fn view_schema_matches_reverse_direction() {
        assert!(matches!(
            view_schema(
                "elohim/sdk/schemas/v1/views/resilience.schema.json",
                "elohim/elohim-storage/src/views.rs",
            ),
            Some(Governor::SchemaContract(_))
        ));
    }

    #[test]
    fn view_schema_does_not_match_a_non_views_schema_json() {
        assert_eq!(
            view_schema(
                "elohim/elohim-storage/src/views.rs",
                "elohim/sdk/schemas/v1/enums/reach.schema.json",
            ),
            None
        );
    }

    // ── doc-doc ──────────────────────────────────────────────────────────────────

    #[test]
    fn doc_doc_matches_md_and_feature_either_way() {
        assert_eq!(
            doc_doc("docs/a.md", "docs/b.feature"),
            Some(Governor::CiteSeal)
        );
        assert_eq!(
            doc_doc("docs/a.feature", "docs/b.md"),
            Some(Governor::CiteSeal)
        );
    }

    #[test]
    fn doc_doc_does_not_match_a_doc_and_code_pair() {
        assert_eq!(doc_doc("docs/a.md", "src/b.rs"), None);
    }

    // ── residual ─────────────────────────────────────────────────────────────────

    #[test]
    fn residual_matches_anything() {
        assert_eq!(
            apply_rule(Path::new("/tmp"), "x.txt", "y.txt", RuleKey::Residual),
            Some(Governor::CiteSeal)
        );
    }

    // ── ordering ─────────────────────────────────────────────────────────────────

    #[test]
    fn derive_governor_only_consults_registry_declared_rules_in_order() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write(
            root,
            "Cargo.toml",
            "[workspace]\nmembers = [\"a\", \"b\"]\n",
        );
        write(root, "a/Cargo.toml", "[package]\nname = \"a\"\n");
        write(root, "a/src/lib.rs", "// a\n");
        write(root, "b/Cargo.toml", "[package]\nname = \"b\"\n");
        write(root, "b/src/lib.rs", "// b\n");

        let full = GovernorRegistry {
            derive_order: DEFAULT_ORDER.to_vec(),
            policy: DEFAULT_POLICY.into(),
        };
        let (governor, rule) = derive_governor(root, "a/src/lib.rs", "b/src/lib.rs", &full);
        assert_eq!(rule, "same-cargo-workspace");
        assert!(matches!(governor, Governor::Compiler(_)));

        // A registry that never consults same-cargo-workspace must fall straight to residual,
        // even though the pair WOULD match it — proves dispatch is registry-order-driven, not
        // "try every detector anyway".
        let residual_only = GovernorRegistry {
            derive_order: vec![RuleKey::Residual],
            policy: DEFAULT_POLICY.into(),
        };
        let (governor, rule) =
            derive_governor(root, "a/src/lib.rs", "b/src/lib.rs", &residual_only);
        assert_eq!(rule, "residual");
        assert_eq!(governor, Governor::CiteSeal);
    }

    // ── registry loading: fail-soft + warn-and-skip ─────────────────────────────

    #[test]
    fn missing_registry_file_fails_soft_to_default_order() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let registry = load_governor_registry(root).expect("fails soft, never errors");
        assert_eq!(registry.policy, DEFAULT_POLICY);
        assert_eq!(registry.derive_order, DEFAULT_ORDER.to_vec());
    }

    #[test]
    fn unknown_rule_key_in_present_registry_warns_and_skips() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write(
            root,
            ".claude/epr-meta/governors.yaml",
            "edge-governors-version: 1\n\
             policies:\n\
             \x20\x20- id: edge-governor-defaults\n\
             \x20\x20\x20\x20version: 1\n\
             \x20\x20\x20\x20title: test fixture\n\
             \x20\x20\x20\x20status: active\n\
             \x20\x20\x20\x20established_by: test\n\
             \x20\x20\x20\x20derive:\n\
             \x20\x20\x20\x20\x20\x20- same-cargo-workspace\n\
             \x20\x20\x20\x20\x20\x20- bogus-rule\n\
             \x20\x20\x20\x20\x20\x20- doc-doc\n\
             \x20\x20\x20\x20\x20\x20- residual\n\
             \x20\x20\x20\x20why: test fixture\n",
        );
        let registry = load_governor_registry(root).expect("loads");
        assert_eq!(registry.policy, "edge-governor-defaults@1");
        assert_eq!(
            registry.derive_order,
            vec![
                RuleKey::SameCargoWorkspace,
                RuleKey::DocDoc,
                RuleKey::Residual
            ]
        );
    }

    // ── dogfood: the real repo registry ─────────────────────────────────────────

    #[test]
    fn dogfood_real_repo_registry_has_five_known_rules_in_order() {
        let Some(root) = find_repo_root() else {
            eprintln!("skipping dogfood test: not running inside the elohim repo tree");
            return;
        };
        let registry = load_governor_registry(&root).expect("loads");
        assert_eq!(registry.policy, "edge-governor-defaults@1");
        assert_eq!(registry.derive_order, DEFAULT_ORDER.to_vec());
    }

    /// Walk up from the crate manifest dir looking for the repo root marker
    /// (`.claude/epr-meta/governors.yaml`); `None` when not running inside the tree (a
    /// packaging/extraction context) — the dogfood test skips gracefully rather than failing.
    fn find_repo_root() -> Option<PathBuf> {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        loop {
            if dir.join(".claude/epr-meta/governors.yaml").is_file() {
                return Some(dir);
            }
            if !dir.pop() {
                return None;
            }
        }
    }
}
