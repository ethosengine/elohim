//! The one-graph edge index (spec §2) — merge the doc plane (`cites:` envelopes inside a
//! doc's hashed body) and the sidecar plane (`.eprfs/status/` `DepEdge` records) into a
//! single `Vec<IndexedEdge>` the walk consumes, and derive each edge's verdict.
//!
//! Two homes, one vocabulary: a doc envelope seals the SHORT-FORM fingerprint
//! (`sha256:hex16`) of the upstream body CID; a sidecar record seals the FULL CIDv1. Both
//! are the same sha2-256 digest rendered two ways (2026-07-12 convergence), so verdict
//! derivation is one rule with two comparisons. Staleness is DERIVED here, never stored:
//! only a `cite-seal` edge can go stale; every other governor is a citation of a stronger
//! system and is reported `Governed`, never entering the stale set.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{EdgeStatus, FlowStore, Governor};
use eprfs_core::BlobCid;
use serde::Serialize;

use super::registry::Registry;
use super::{
    canonical_body, cite_fingerprint, cite_path, cite_slug, cite_status, default_recipes,
    parse_frontmatter, rel_to_root, FlowResult,
};

/// Which plane an edge was projected from — decides the seal comparison (short vs full).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgePlane {
    /// A `cites:` envelope inside a doc's hashed body.
    Doc,
    /// A `DepEdge` record in the `.eprfs/status/` sidecar.
    Sidecar,
}

/// The upstream seal an edge carries, in the rendering its plane declares.
#[derive(Debug, Clone, PartialEq)]
pub enum SealForm {
    /// Doc envelope: the short-form fingerprint `sha256:hex16` to compare against.
    Short(String),
    /// Sidecar `cite-seal`: the full upstream CIDv1 to compare against.
    Full(Cid),
    /// A governed edge carries no seal.
    None,
}

/// One edge in the merged graph — plane-agnostic once indexed.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedEdge {
    /// Downstream artifact (repo-relative path).
    pub from: String,
    /// Upstream artifact (repo-relative path; the slug itself when unresolvable).
    pub to: String,
    /// The announcement — why this edge exists.
    pub desc: Option<String>,
    pub governor: Governor,
    pub seal: SealForm,
    /// `Some(reason)` when the edge is a declared deviation (`held`).
    pub held: Option<String>,
    pub plane: EdgePlane,
    /// Whether `to` resolved to a readable file at index time.
    pub target_exists: bool,
}

/// The derived health of an edge — never stored, always recomputed from the upstream body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// A `cite-seal` edge whose upstream still matches the sealed digest.
    Ok,
    /// A `cite-seal` edge whose upstream drifted from the sealed digest.
    Stale,
    /// A declared deviation (`held`) — carries the reason. Never stale.
    Held(String),
    /// A stronger system enforces this edge — carries the mechanism label. Never stale.
    Governed(String),
    /// A `cite-seal` edge whose upstream target could not be resolved/read.
    Dangling,
}

impl Verdict {
    /// The single glanceable word for a walk/status line.
    pub fn word(&self) -> &'static str {
        match self {
            Verdict::Ok => "ok",
            Verdict::Stale => "stale",
            Verdict::Held(_) => "held",
            Verdict::Governed(_) => "governed",
            Verdict::Dangling => "dangling",
        }
    }
}

/// The stable label for a governor — the rendering seal/walk/status all share.
pub fn governor_label(governor: &Governor) -> String {
    match governor {
        Governor::Compiler(unit) => format!("compiler:{unit}"),
        Governor::Codegen(pipeline) => format!("codegen:{pipeline}"),
        Governor::SchemaContract(test) => format!("schema-contract:{test}"),
        Governor::Test(id) => format!("test:{id}"),
        Governor::CiteSeal => "cite-seal".to_string(),
    }
}

/// Derive an edge's verdict from its declaration and the freshly-recomputed upstream body
/// CID (`current`, `None` when the upstream is unresolvable). PURE — no IO, so the whole
/// verdict rule is unit-testable in isolation (spec §2):
/// - governor ≠ cite-seal → `Governed` (never stale);
/// - `held` → `Held` (stays held even when stale-by-CID);
/// - unresolvable upstream → `Dangling`;
/// - else compare the sealed digest to `current` — full CID (sidecar) or short form (doc).
pub fn derive_verdict(edge: &IndexedEdge, current: Option<&BlobCid>) -> Verdict {
    if !matches!(edge.governor, Governor::CiteSeal) {
        return Verdict::Governed(governor_label(&edge.governor));
    }
    if let Some(reason) = &edge.held {
        return Verdict::Held(reason.clone());
    }
    let Some(current) = current else {
        return Verdict::Dangling;
    };
    let conformant = match &edge.seal {
        SealForm::Full(sealed) => current.as_cid() == sealed,
        SealForm::Short(sealed) => &current.short_fingerprint() == sealed,
        // A cite-seal edge with no seal is malformed — treat as dangling rather than panic.
        SealForm::None => return Verdict::Dangling,
    };
    if conformant {
        Verdict::Ok
    } else {
        Verdict::Stale
    }
}

/// Recompute the upstream body CID for an edge, or `None` when the target is unreadable.
pub fn recompute_upstream(root: &Path, edge: &IndexedEdge) -> Option<BlobCid> {
    let text = std::fs::read_to_string(root.join(&edge.to)).ok()?;
    Some(BlobCid::compute_raw(canonical_body(&text).as_bytes()))
}

/// The verdict of an edge against the tree at `root` — recomputes the upstream only when a
/// verdict actually depends on it (cite-seal, not held).
pub fn edge_verdict(root: &Path, edge: &IndexedEdge) -> Verdict {
    if !matches!(edge.governor, Governor::CiteSeal) {
        return Verdict::Governed(governor_label(&edge.governor));
    }
    if let Some(reason) = &edge.held {
        return Verdict::Held(reason.clone());
    }
    let current = recompute_upstream(root, edge);
    derive_verdict(edge, current.as_ref())
}

/// The merged edge graph over both planes.
#[derive(Debug, Default)]
pub struct EdgeIndex {
    pub edges: Vec<IndexedEdge>,
}

impl EdgeIndex {
    /// Merge the doc plane (`cites:` envelopes) and the sidecar plane (`store.edges()`)
    /// into one graph. A missing/unparseable recipe registry yields an empty doc plane
    /// (the sidecar plane still projects) — the index degrades, never errors, on it.
    pub fn build(root: &Path, store: &impl FlowStore) -> FlowResult<Self> {
        let mut edges = doc_plane(root);
        edges.extend(sidecar_plane(store)?);
        Ok(Self { edges })
    }

    /// Edges `from` this path — what it depends on.
    pub fn outgoing<'a>(&'a self, path: &str) -> impl Iterator<Item = &'a IndexedEdge> {
        let path = path.to_string();
        self.edges.iter().filter(move |e| e.from == path)
    }

    /// Edges `to` this path — who depends on it.
    pub fn incoming<'a>(&'a self, path: &str) -> impl Iterator<Item = &'a IndexedEdge> {
        let path = path.to_string();
        self.edges.iter().filter(move |e| e.to == path)
    }
}

/// Project the sidecar `DepEdge` records (latest-per-slot) into `IndexedEdge`s.
fn sidecar_plane(store: &impl FlowStore) -> FlowResult<Vec<IndexedEdge>> {
    let mut out = Vec::new();
    for (_, edge) in store.edges()? {
        let held = edge
            .status
            .as_ref()
            .map(|EdgeStatus::Held { reason, .. }| reason.clone());
        let seal = match edge.sealed_cid {
            Some(cid) => SealForm::Full(cid),
            None => SealForm::None,
        };
        out.push(IndexedEdge {
            from: edge.from,
            to: edge.to,
            desc: edge.desc,
            governor: edge.governor,
            seal,
            held,
            plane: EdgePlane::Sidecar,
            // Sidecar edges are path-addressed; resolvability is recomputed at verdict time.
            target_exists: true,
        });
    }
    Ok(out)
}

/// Project the doc corpus's sealed `cites:` envelopes into `IndexedEdge`s. Only envelopes
/// carrying a `sha256:` fingerprint are indexed — a bare/legacy path cite is an UNSEALED
/// reference, out of the seal-aware walk's scope for this slice.
fn doc_plane(root: &Path) -> Vec<IndexedEdge> {
    let Some(registry) = load_registry(root) else {
        return Vec::new();
    };
    let corpus = doc_corpus(root, &registry);
    let id_index = build_id_index(&corpus);

    let mut out = Vec::new();
    for (rel, abs) in &corpus {
        let Ok(text) = std::fs::read_to_string(abs) else {
            continue;
        };
        let fm = parse_frontmatter(&text);
        for entry in fm.list("cites") {
            let Some(fingerprint) = cite_fingerprint(entry) else {
                continue; // unsealed reference — not part of the seal-aware graph
            };
            // Resolve the upstream: `path:` locator first, then slug via the id index.
            let (to, target_exists) = resolve_target(root, entry, &id_index);
            let held = cite_status(entry)
                .filter(|s| s.to_lowercase().contains("held"))
                .map(|s| s.trim().to_string());
            out.push(IndexedEdge {
                from: rel.clone(),
                to,
                desc: cite_desc(entry),
                governor: Governor::CiteSeal,
                seal: SealForm::Short(fingerprint),
                held,
                plane: EdgePlane::Doc,
                target_exists,
            });
        }
    }
    out
}

/// Resolve an envelope's upstream to `(repo-relative path, exists)`: `path:` locator wins,
/// then the slug against the corpus id index; unresolved falls back to the slug itself.
fn resolve_target(root: &Path, entry: &str, id_index: &BTreeMap<String, String>) -> (String, bool) {
    if let Some(path) = cite_path(entry) {
        let exists = root.join(&path).is_file();
        return (path, exists);
    }
    if let Some(slug) = cite_slug(entry) {
        if let Some(path) = id_index.get(&slug) {
            let exists = root.join(path).is_file();
            return (path.clone(), exists);
        }
        return (slug, false);
    }
    (String::new(), false)
}

/// The envelope's `desc` segment (first non-keyed segment after the slug).
fn cite_desc(entry: &str) -> Option<String> {
    let mut segments = entry.split('|').map(str::trim);
    let _slug = segments.next();
    for seg in segments {
        if seg.starts_with("sha256:") || seg.starts_with("status:") || seg.starts_with("path:") {
            continue;
        }
        if !seg.is_empty() {
            return Some(seg.to_string());
        }
    }
    None
}

/// Doc stages that carry frontmatter + `cites:` — the seal-aware corpus.
const DOC_STAGES: &[&str] = &["manifesto", "epic", "architecture-seed", "spec", "plan"];

fn load_registry(root: &Path) -> Option<Registry> {
    Registry::load(&default_recipes(root)).ok()
}

/// Every doc file under the doc stages, deduped, as `(rel, abs)` pairs.
fn doc_corpus(root: &Path, registry: &Registry) -> Vec<(String, PathBuf)> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for recipe in &registry.recipes {
        for stage_name in DOC_STAGES {
            let Some(stage) = recipe.stage(stage_name) else {
                continue;
            };
            for pattern in &stage.paths {
                let full = root.join(pattern);
                let Some(pat) = full.to_str() else {
                    continue;
                };
                let Ok(paths) = glob::glob(pat) else {
                    continue;
                };
                for entry in paths.flatten() {
                    if entry.is_file() && seen.insert(entry.clone()) {
                        out.push((rel_to_root(root, &entry), entry));
                    }
                }
            }
        }
    }
    out
}

/// Map each doc's frontmatter `id:` to its repo-relative path (for slug resolution).
fn build_id_index(corpus: &[(String, PathBuf)]) -> BTreeMap<String, String> {
    let mut index = BTreeMap::new();
    for (rel, abs) in corpus {
        if let Ok(text) = std::fs::read_to_string(abs) {
            if let Some(id) = parse_frontmatter(&text).get("id") {
                index.entry(id.to_string()).or_insert_with(|| rel.clone());
            }
        }
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(body: &str) -> BlobCid {
        BlobCid::compute_raw(canonical_body(body).as_bytes())
    }

    fn doc_edge(seal: SealForm, held: Option<String>) -> IndexedEdge {
        IndexedEdge {
            from: "docs/downstream.md".into(),
            to: "docs/upstream.md".into(),
            desc: Some("downstream conforms to upstream".into()),
            governor: Governor::CiteSeal,
            seal,
            held,
            plane: EdgePlane::Doc,
            target_exists: true,
        }
    }

    // ── Pure verdict derivation ──────────────────────────────────────────────────

    #[test]
    fn doc_envelope_matching_short_form_is_ok() {
        let upstream = blob("the upstream body");
        let edge = doc_edge(SealForm::Short(upstream.short_fingerprint()), None);
        assert_eq!(derive_verdict(&edge, Some(&upstream)), Verdict::Ok);
    }

    #[test]
    fn doc_envelope_drifted_upstream_is_stale() {
        let sealed = blob("the upstream body at seal time");
        let drifted = blob("the upstream body AFTER an edit");
        let edge = doc_edge(SealForm::Short(sealed.short_fingerprint()), None);
        assert_eq!(derive_verdict(&edge, Some(&drifted)), Verdict::Stale);
    }

    #[test]
    fn sidecar_full_cid_equality_is_ok_then_stale() {
        let sealed = blob("v1");
        let edge = IndexedEdge {
            from: "app/foo.ts".into(),
            to: "spec/bar.md".into(),
            desc: None,
            governor: Governor::CiteSeal,
            seal: SealForm::Full(*sealed.as_cid()),
            held: None,
            plane: EdgePlane::Sidecar,
            target_exists: true,
        };
        assert_eq!(derive_verdict(&edge, Some(&sealed)), Verdict::Ok);
        let drifted = blob("v2");
        assert_eq!(derive_verdict(&edge, Some(&drifted)), Verdict::Stale);
    }

    #[test]
    fn held_edge_stays_held_even_when_stale_by_cid() {
        let sealed = blob("v1");
        let drifted = blob("v2"); // upstream has moved on — would be Stale if not held
        let edge = doc_edge(
            SealForm::Short(sealed.short_fingerprint()),
            Some("read-legacy, write-canonical — transitional".into()),
        );
        match derive_verdict(&edge, Some(&drifted)) {
            Verdict::Held(reason) => assert!(reason.contains("transitional")),
            other => panic!("held edge must stay Held, got {other:?}"),
        }
    }

    #[test]
    fn governed_edge_is_never_stale_under_any_upstream() {
        let drifted = blob("anything at all");
        for governor in [
            Governor::Compiler("cargo:elohim-storage".into()),
            Governor::Codegen("ts-rs".into()),
            Governor::SchemaContract("schema_contract.rs".into()),
            Governor::Test("cargo test export_bindings".into()),
        ] {
            let edge = IndexedEdge {
                from: "a".into(),
                to: "b".into(),
                desc: None,
                governor: governor.clone(),
                seal: SealForm::None,
                held: None,
                plane: EdgePlane::Sidecar,
                target_exists: true,
            };
            // Never stale — even with a mismatched upstream, and even with no upstream at all.
            assert!(matches!(
                derive_verdict(&edge, Some(&drifted)),
                Verdict::Governed(_)
            ));
            assert!(matches!(derive_verdict(&edge, None), Verdict::Governed(_)));
        }
    }

    #[test]
    fn unresolvable_upstream_is_dangling() {
        let edge = doc_edge(SealForm::Short("sha256:0000000000000000".into()), None);
        assert_eq!(derive_verdict(&edge, None), Verdict::Dangling);
    }

    #[test]
    fn cite_desc_extracts_the_first_unkeyed_segment() {
        let entry = "my-slug | the human description | sha256:1234567890abcdef | path: docs/x.md";
        assert_eq!(cite_desc(entry).as_deref(), Some("the human description"));
    }
}
