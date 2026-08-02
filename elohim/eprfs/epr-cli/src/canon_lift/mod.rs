//! `epr canon-lift` — the brit-lift PROJECTION of the `.epr-meta` concern canon.
//!
//! # What this is
//!
//! The canon lives in two sibling YAML registries — `.claude/epr-meta/policies.yaml` (enforcement
//! rows read by the resolver hook) and `.claude/epr-meta/concerns.yaml` (Precedent-shaped concern
//! rows read by the census and the SDK canon projection). Each row carries a canonical-JSON
//! `sha256` pin written by `.claude/scripts/epr-meta-pin.py`. **That live pin mechanism is
//! untouched by this module.** What this generates, beside it, is the content-addressed
//! projection the eprfs/brit substrate consumes at graduation:
//!
//! * `canon-atoms.json` — the **content plane**: one `CIDv1(dag-cbor, sha2-256)` EPR atom per
//!   row of both registries, all versions including superseded ones (lineage travels; a
//!   superseded row whose pin stopped reproducing is a lineage you can no longer verify).
//! * `declared-heads.json` — the **brit primitive**: one declared-head-over-DAG record per policy
//!   id, resolving `pinned` (the consumer declares which version applies — never recency). The
//!   identity-head spec names itself the third instance of this primitive and the dataplane's
//!   content-head election the fourth; the canon's heads are the fifth.
//! * `precedent-standing.json` — the **standing plane**: a `mishpat_integrity::Precedent`-shaped
//!   object per row that CITES its content CID. This is the two-plane split made real — the v1
//!   YAML row deliberately conflates content and standing in one object
//!   (`genesis/docs/superpowers/specs/2026-07-02-epr-meta-policy-registry-measure-design.md` §1),
//!   and the lift is what separates them, so revoking or re-scoping a policy's force never
//!   rewrites its content and two communities can grant different standing to the same
//!   content-identical policy.
//! * `canon-lift.md` — the human-readable summary, generated beside the JSON.
//!
//! # The guarantee it inherits
//!
//! `elohim/epr/tests/concern_canon_liftability.rs` (plan task P0.4) proved on a frozen fixture
//! that the exact bytes the sha256 pin already covers, wrapped as `CIDv1(dag-cbor, sha2-256)`
//! through `elohim_epr::cid::compute_cid`, carry a multihash digest byte-identical to that pin —
//! so the lift is a codec wrap of bytes that already exist, never a rewrite. **This generator is
//! that fixture generalized to every row of both registries**, and it mints through the same
//! `compute_cid`, never a second implementation. The sha256 it reports is not separately
//! computed either: it IS the CID's own multihash digest, one hash rendered two ways.
//!
//! `canonical.rs` explains why the canonicalization is hand-written rather than `serde_json`'s.
//!
//! # Consumption at graduation
//!
//! The eprfs/brit substrate consumes these artifacts when the registry graduates: the atoms
//! become blobs under `eprfs_core::BlobCid` addresses, the heads become brit declared-head
//! records, and each standing object becomes a Mishpat `Precedent` entry whose `entry_hash` is
//! its CID, citing the content CID rather than embedding the content. Nothing here writes to a
//! DHT — the projection is the handoff, and minting is the substrate's job.
//!
//! # Freshness
//!
//! The artifacts are GENERATED. Regenerate with `epr canon-lift --write` after any registry edit
//! (i.e. right after `python3 .claude/scripts/epr-meta-pin.py --write`); `epr canon-lift` alone
//! checks freshness and exits 1 when stale. `tests/canon_lift_parity.rs` enforces both freshness
//! and the pin/CID correspondence, and it runs in the eprfs pipeline's
//! `cargo nextest run --workspace --all-targets`.
//!
//! KNOWN RESIDUAL, stated rather than papered over: a canon edit that touches ONLY
//! `.claude/epr-meta/*.yaml` is inside the repo-root manifest's `ci-trigger: ignore:` set, so it
//! does not dispatch the eprfs pipeline — the staleness is caught at the NEXT eprfs build, not at
//! the edit. The check is therefore eventually-loud, not immediately-loud. Closing that properly
//! means the epr-meta audit surface calling `epr canon-lift` (owned by other work; do not wire it
//! from here), so until then: regenerate in the same commit as the pin.

pub mod canonical;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use elohim_epr::cid::compute_cid;
use serde::Serialize;
use serde_yaml::Value;

use canonical::{canonical_body, canonical_json, hex_lower, CanonError};

/// Bumped when the SHAPE of the generated artifacts changes (not when the canon moves).
pub const CANON_LIFT_VERSION: u32 = 1;

/// Where the generated artifacts live — beside `concern-seam-matrix.json`, the sibling generated
/// projection of the same canon.
pub const OUT_DIR_REL: &str = ".claude/epr-meta/generated/canon-lift";

pub const ATOMS_FILE: &str = "canon-atoms.json";
pub const HEADS_FILE: &str = "declared-heads.json";
pub const STANDING_FILE: &str = "precedent-standing.json";
pub const SUMMARY_FILE: &str = "canon-lift.md";

const FIX_CMD: &str = "epr canon-lift --write";

/// The two registry homes, in the order the pin tool walks them.
const REGISTRIES: [(&str, &str, &str); 2] = [
    (
        ".claude/epr-meta/policies.yaml",
        "epr-meta-policies-version",
        "policies",
    ),
    (
        ".claude/epr-meta/concerns.yaml",
        "concern-canon-version",
        "concerns",
    ),
];

/// The registry-wide default when a row declares no `resolution_mode`. Both registries state it
/// in their headers: the `<id>@<version>` pin in a binding is REQUIRED — a declared dependency,
/// never recency. Governing artifacts resolve by pin, not by election.
const DEFAULT_RESOLUTION_MODE: &str = "pinned";

#[derive(Debug, thiserror::Error)]
pub enum CanonLiftError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("registry parse ({path}): {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("{path}: {message}")]
    Registry { path: PathBuf, message: String },
    #[error("{path} row `{row}`: {source}")]
    Canonicalization {
        path: PathBuf,
        row: String,
        #[source]
        source: CanonError,
    },
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type CanonLiftResult<T> = std::result::Result<T, CanonLiftError>;

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

/// One registry row, canonicalized and content-addressed.
#[derive(Debug, Clone)]
pub struct Row {
    pub home: &'static str,
    pub id: String,
    pub version: u64,
    pub status: String,
    pub superseded_by: Option<String>,
    /// The exact bytes the sha256 pin covers.
    pub body: Vec<u8>,
    /// `sha256:<hex>` — the CID's own multihash digest, not a second hash.
    pub sha256: String,
    pub pin: Option<String>,
    pub cid: String,
    pub raw: serde_yaml::Mapping,
}

impl Row {
    fn field(&self, key: &str) -> Option<&Value> {
        self.raw.get(Value::String(key.to_string()))
    }

    fn text(&self, key: &str) -> Option<String> {
        match self.field(key)? {
            Value::String(s) => Some(norm(s)),
            other => Some(norm(&format!("{other:?}"))),
        }
    }

    fn pin_state(&self) -> &'static str {
        match &self.pin {
            None => "unpinned",
            Some(p) if *p == self.sha256 => "verified",
            Some(_) => "mismatch",
        }
    }

    fn resolution_mode(&self) -> (String, &'static str) {
        match self.text("resolution_mode") {
            Some(mode) if !mode.is_empty() => (mode, "row"),
            _ => (DEFAULT_RESOLUTION_MODE.to_string(), "registry-default"),
        }
    }
}

/// A lineage pointer resolved to the CID it names, or explicitly marked unresolved. An
/// unresolvable pointer is reported, never dropped: a dangling lineage edge is exactly the thing
/// a supersession-carrying projection exists to make visible.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineageRef {
    /// The `superseded_by:` string exactly as the registry declares it.
    pub declared: String,
    pub id: String,
    pub version: Option<u64>,
    pub registry: Option<&'static str>,
    pub cid: Option<String>,
    pub resolved: bool,
}

/// The content plane: one EPR atom per registry row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Atom {
    pub id: String,
    pub version: u64,
    pub registry: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concern: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standing: Option<String>,
    pub resolution_mode: String,
    pub resolution_mode_source: &'static str,
    pub sha256_pin: Option<String>,
    pub sha256_computed: String,
    pub pin_state: &'static str,
    pub cid: String,
    pub body_bytes: usize,
    pub supersedes: Vec<LineageRef>,
    pub superseded_by: Option<LineageRef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadPointer {
    pub registry: &'static str,
    pub version: u64,
    pub cid: String,
    pub sha256_pin: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeadVersion {
    pub registry: &'static str,
    pub version: u64,
    pub status: String,
    pub cid: String,
    pub superseded_by: Option<String>,
}

/// The brit primitive: a declared head over the version DAG of one policy id.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Head {
    pub id: String,
    pub resolution_mode: String,
    pub resolution_mode_source: &'static str,
    pub head: HeadPointer,
    pub tip_basis: &'static str,
    /// True when more than one ACTIVE version exists for this id. Genuine competition never
    /// auto-resolves: the projection names it rather than silently picking a winner.
    pub competing: bool,
    pub versions: Vec<HeadVersion>,
}

/// The `mishpat_integrity::Precedent` field shape, mirrored exactly (snake_case field names kept
/// so the correspondence is checkable by eye against the zome).
///
/// `citations` and the three timestamps are `null` on purpose: the registries carry no citation
/// measure and no clock, and those values are minted by the notarizing action at DHT residency.
/// Emitting `0` / a generation timestamp would be a fabricated notarization — and would also make
/// the artifact non-deterministic, which the freshness check depends on.
#[derive(Debug, Clone, Serialize)]
pub struct Precedent {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub full_reasoning: String,
    pub binding: String,
    pub scope_json: String,
    pub citations: Option<u32>,
    pub status: String,
    pub established_by: String,
    pub established_at: Option<String>,
    pub superseded_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub metadata_json: String,
}

/// The standing plane: a Precedent citing a content CID. The citation lives in exactly ONE place
/// (`citesContentCid`); it is deliberately NOT duplicated into `metadata_json`, because two homes
/// for one fact is the duplication the two-plane split exists to prevent. At DHT residency the
/// citation becomes a link from the Precedent entry to the content atom.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Standing {
    pub cites_content_cid: String,
    pub cites_content_sha256: String,
    pub registry: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standing_shape: Option<String>,
    pub precedent: Precedent,
}

/// Everything the generator derived, plus the exact bytes of every artifact.
#[derive(Debug, Clone)]
pub struct Projection {
    pub atoms: Vec<Atom>,
    pub heads: Vec<Head>,
    pub standings: Vec<Standing>,
    pub errors: Vec<String>,
    /// `(relative file name, exact contents)` — the committed artifact set.
    pub files: Vec<(&'static str, String)>,
}

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AtomsDoc<'a> {
    #[serde(rename = "$comment")]
    comment: String,
    canon_lift_version: u32,
    generated_from: Vec<&'static str>,
    plane: &'static str,
    cid_codec: &'static str,
    cid_multihash: &'static str,
    atoms: &'a [Atom],
    errors: &'a [String],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HeadsDoc<'a> {
    #[serde(rename = "$comment")]
    comment: String,
    canon_lift_version: u32,
    generated_from: Vec<&'static str>,
    primitive: &'static str,
    heads: &'a [Head],
    errors: &'a [String],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StandingDoc<'a> {
    #[serde(rename = "$comment")]
    comment: String,
    canon_lift_version: u32,
    generated_from: Vec<&'static str>,
    plane: &'static str,
    mirrors: &'static str,
    standings: &'a [Standing],
    errors: &'a [String],
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn norm(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            pending_space = !out.is_empty();
        } else {
            if pending_space {
                out.push(' ');
            }
            pending_space = false;
            out.push(c);
        }
    }
    out
}

fn scalar_text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.trim().to_string()),
        Value::Null => None,
        other => Some(format!("{other:?}")),
    }
}

/// Load one registry into content-addressed rows. Every row is minted; a row whose pin does not
/// reproduce is still minted and flagged `mismatch` rather than dropped — dropping it would make
/// a tampered row invisible, which is the failure mode the pin exists to prevent.
fn load_registry(
    root: &Path,
    rel: &'static str,
    version_key: &str,
    rows_key: &str,
) -> CanonLiftResult<Vec<Row>> {
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path).map_err(|source| CanonLiftError::Read {
        path: path.clone(),
        source,
    })?;
    let doc: Value = serde_yaml::from_str(&text).map_err(|source| CanonLiftError::Yaml {
        path: path.clone(),
        source,
    })?;
    let Value::Mapping(map) = &doc else {
        return Err(CanonLiftError::Registry {
            path,
            message: "registry is not a mapping".into(),
        });
    };
    match map.get(Value::String(version_key.to_string())) {
        Some(Value::Number(n)) if n.as_u64() == Some(1) => {}
        _ => {
            return Err(CanonLiftError::Registry {
                path,
                message: format!("missing/invalid `{version_key}` (must be 1)"),
            })
        }
    }
    let Some(Value::Sequence(rows)) = map.get(Value::String(rows_key.to_string())) else {
        return Err(CanonLiftError::Registry {
            path,
            message: format!("no `{rows_key}` list"),
        });
    };

    // `rel` is `&'static str`, so its basename is too — the short home name the registries use
    // in cross-home `superseded_by: <file>#<id>@<n>` pointers.
    let home: &'static str = rel.rsplit('/').next().unwrap_or(rel);

    let mut out = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let Value::Mapping(row) = row else {
            return Err(CanonLiftError::Registry {
                path,
                message: format!("{rows_key}[{i}] is not a mapping"),
            });
        };
        let id = scalar_text(row.get(Value::String("id".into()))).ok_or_else(|| {
            CanonLiftError::Registry {
                path: path.clone(),
                message: format!("{rows_key}[{i}] has no `id`"),
            }
        })?;
        let version = match row.get(Value::String("version".into())) {
            Some(Value::Number(n)) => n.as_u64().ok_or_else(|| CanonLiftError::Registry {
                path: path.clone(),
                message: format!("{rows_key}[{i}] `{id}` has a non-integer `version`"),
            })?,
            _ => {
                return Err(CanonLiftError::Registry {
                    path: path.clone(),
                    message: format!("{rows_key}[{i}] `{id}` needs an integer `version`"),
                })
            }
        };
        let body = canonical_body(row).map_err(|source| CanonLiftError::Canonicalization {
            path: path.clone(),
            row: format!("{id}@{version}"),
            source,
        })?;
        // ONE hash, two renderings: the CID's multihash digest IS the sha256 the pin declares.
        let cid = compute_cid(&body);
        let sha256 = format!("sha256:{}", hex_lower(cid.hash().digest()));
        out.push(Row {
            home,
            id,
            version,
            status: scalar_text(row.get(Value::String("status".into())))
                .unwrap_or_else(|| "active".into()),
            superseded_by: scalar_text(row.get(Value::String("superseded_by".into()))),
            body,
            sha256,
            pin: scalar_text(row.get(Value::String("contentHash".into()))),
            cid: cid.to_string(),
            raw: row.clone(),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Lineage
// ---------------------------------------------------------------------------

/// Parse a `superseded_by:` value: `<id>@<n>` (same home) or `<file>#<id>@<n>` (across homes —
/// the C6a graduation shape). Mirrors `_SUPERSEDED_BY` in `concern_canon_projection.py`.
fn parse_ref(raw: &str) -> Option<(Option<String>, String, u64)> {
    let raw = raw.trim();
    let (home, rest) = match raw.split_once('#') {
        Some((h, r)) => (Some(h.to_string()), r),
        None => (None, raw),
    };
    let (id, version) = rest.rsplit_once('@')?;
    if id.is_empty() {
        return None;
    }
    Some((home, id.to_string(), version.parse().ok()?))
}

/// Highest version wins; `policies.yaml` breaks a tie (an enforcement row is the stronger home —
/// the same rule `concern_canon_projection.py` uses for its tip selection).
fn pick_tip<'a>(set: &[&'a Row]) -> Option<&'a Row> {
    set.iter()
        .copied()
        .max_by_key(|r| (r.version, r.home == "policies.yaml"))
}

fn resolve_ref(rows: &[Row], declared: &str) -> LineageRef {
    match parse_ref(declared) {
        None => LineageRef {
            declared: declared.to_string(),
            id: declared.to_string(),
            version: None,
            registry: None,
            cid: None,
            resolved: false,
        },
        Some((home, id, version)) => {
            let target = rows.iter().find(|r| {
                r.id == id
                    && r.version == version
                    && home.as_deref().map(|h| h == r.home).unwrap_or(true)
            });
            LineageRef {
                declared: declared.to_string(),
                id,
                version: Some(version),
                registry: target.map(|r| r.home),
                cid: target.map(|r| r.cid.clone()),
                resolved: target.is_some(),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Projection
// ---------------------------------------------------------------------------

pub fn project(root: &Path) -> CanonLiftResult<Projection> {
    let mut rows = Vec::new();
    for (rel, version_key, rows_key) in REGISTRIES {
        rows.extend(load_registry(root, rel, version_key, rows_key)?);
    }
    // Lineage siblings sit adjacent; the ordering is registry-order-independent so a row moving
    // within its file never churns the artifact.
    rows.sort_by(|a, b| {
        (a.id.as_str(), a.version, a.home).cmp(&(b.id.as_str(), b.version, b.home))
    });

    let mut errors: Vec<String> = Vec::new();

    // ── content plane ──────────────────────────────────────────────────────────────────────
    let mut atoms = Vec::with_capacity(rows.len());
    for row in &rows {
        let superseded_by = row.superseded_by.as_deref().map(|d| resolve_ref(&rows, d));
        if let Some(r) = &superseded_by {
            if !r.resolved {
                errors.push(format!(
                    "{}#{}@{} declares `superseded_by: {}`, which resolves to no row in either \
                     home — a dangling lineage pointer",
                    row.home, row.id, row.version, r.declared
                ));
            }
        }
        let supersedes: Vec<LineageRef> = rows
            .iter()
            .filter(|other| {
                other
                    .superseded_by
                    .as_deref()
                    .and_then(parse_ref)
                    .map(|(home, id, version)| {
                        id == row.id
                            && version == row.version
                            && home.as_deref().map(|h| h == row.home).unwrap_or(true)
                    })
                    .unwrap_or(false)
            })
            .map(|other| LineageRef {
                declared: format!("{}#{}@{}", other.home, other.id, other.version),
                id: other.id.clone(),
                version: Some(other.version),
                registry: Some(other.home),
                cid: Some(other.cid.clone()),
                resolved: true,
            })
            .collect();

        if row.pin_state() == "mismatch" {
            errors.push(format!(
                "{}#{}@{} pin MISMATCH: registry pins {}, canonical body hashes to {} — the lift \
                 would mint an address the registry does not vouch for. Re-run \
                 `python3 .claude/scripts/epr-meta-pin.py --write`.",
                row.home,
                row.id,
                row.version,
                row.pin.as_deref().unwrap_or("-"),
                row.sha256
            ));
        }
        if row.pin_state() == "unpinned" {
            errors.push(format!(
                "{}#{}@{} carries no `contentHash` — its CID is computable but nothing in the \
                 registry vouches for the bytes (C5: unverifiable evidence confers nothing).",
                row.home, row.id, row.version
            ));
        }

        let (resolution_mode, resolution_mode_source) = row.resolution_mode();
        atoms.push(Atom {
            id: row.id.clone(),
            version: row.version,
            registry: row.home,
            concern: row.text("concern"),
            status: row.status.clone(),
            standing: row.text("standing"),
            resolution_mode,
            resolution_mode_source,
            sha256_pin: row.pin.clone(),
            sha256_computed: row.sha256.clone(),
            pin_state: row.pin_state(),
            cid: row.cid.clone(),
            body_bytes: row.body.len(),
            supersedes,
            superseded_by,
        });
    }

    // ── declared heads (one per policy id, across both homes) ──────────────────────────────
    let mut by_id: BTreeMap<&str, Vec<&Row>> = BTreeMap::new();
    for row in &rows {
        by_id.entry(row.id.as_str()).or_default().push(row);
    }
    let mut heads = Vec::with_capacity(by_id.len());
    for (id, versions) in &by_id {
        let active: Vec<&Row> = versions
            .iter()
            .copied()
            .filter(|r| r.status != "superseded")
            .collect();
        let (tip, tip_basis, competing) = match active.len() {
            1 => (pick_tip(&active), "sole-active", false),
            0 => {
                errors.push(format!(
                    "`{id}` has no ACTIVE version in either home — every version is superseded, \
                     so the declared head falls back to the highest version"
                ));
                (pick_tip(versions), "all-superseded-highest-version", false)
            }
            _ => {
                errors.push(format!(
                    "`{id}` has {} ACTIVE versions across the two homes — genuine competition \
                     never auto-resolves; the head below is a deterministic fallback, not an \
                     arbitration",
                    active.len()
                ));
                (pick_tip(&active), "multiple-active-highest-version", true)
            }
        };
        let Some(tip) = tip else { continue };
        let (resolution_mode, resolution_mode_source) = tip.resolution_mode();
        heads.push(Head {
            id: (*id).to_string(),
            resolution_mode,
            resolution_mode_source,
            head: HeadPointer {
                registry: tip.home,
                version: tip.version,
                cid: tip.cid.clone(),
                sha256_pin: tip.pin.clone(),
            },
            tip_basis,
            competing,
            versions: versions
                .iter()
                .map(|r| HeadVersion {
                    registry: r.home,
                    version: r.version,
                    status: r.status.clone(),
                    cid: r.cid.clone(),
                    superseded_by: r.superseded_by.clone(),
                })
                .collect(),
        });
    }

    // ── standing plane ─────────────────────────────────────────────────────────────────────
    let mut standings = Vec::with_capacity(rows.len());
    for row in &rows {
        standings.push(Standing {
            cites_content_cid: row.cid.clone(),
            cites_content_sha256: row.sha256.clone(),
            registry: row.home,
            standing_shape: row.text("standing"),
            precedent: precedent_for(row)?,
        });
    }

    errors.sort();
    errors.dedup();

    let files = render_files(&atoms, &heads, &standings, &errors)?;
    Ok(Projection {
        atoms,
        heads,
        standings,
        errors,
        files,
    })
}

/// Build the Precedent-shaped standing object for one row.
fn precedent_for(row: &Row) -> CanonLiftResult<Precedent> {
    let mut scope = serde_yaml::Mapping::new();
    if let Some(v) = row.field("scope") {
        scope.insert(Value::String("scope".into()), v.clone());
    }
    if let Some(prose) = row.text("applies_to") {
        scope.insert(Value::String("appliesTo".into()), Value::String(prose));
    }

    // Non-CID standing metadata only. The content citation lives in `citesContentCid`.
    let mut meta = serde_yaml::Mapping::new();
    meta.insert(
        Value::String("registry".into()),
        Value::String(row.home.to_string()),
    );
    meta.insert(
        Value::String("version".into()),
        Value::Number(row.version.into()),
    );
    let (resolution_mode, _) = row.resolution_mode();
    meta.insert(
        Value::String("resolutionMode".into()),
        Value::String(resolution_mode),
    );
    for (yaml_key, json_key) in [
        ("concern", "concern"),
        ("standing", "standing"),
        ("probe_home", "probeHome"),
        ("severity_class", "severityClass"),
        ("dispatch-agent", "dispatchAgent"),
        ("findings_namespace", "findingsNamespace"),
        ("class", "enforcementClass"),
        ("validator", "validator"),
    ] {
        if let Some(v) = row.text(yaml_key) {
            meta.insert(Value::String(json_key.into()), Value::String(v));
        }
    }
    if let Some(v) = row.field("recurrence") {
        meta.insert(Value::String("recurrence".into()), v.clone());
    }

    let canon = |m: serde_yaml::Mapping| -> CanonLiftResult<String> {
        canonical_json(&Value::Mapping(m)).map_err(|source| CanonLiftError::Canonicalization {
            path: PathBuf::from(row.home),
            row: format!("{}@{}", row.id, row.version),
            source,
        })
    };

    Ok(Precedent {
        id: format!("{}@{}", row.id, row.version),
        title: row.text("title").unwrap_or_default(),
        // A concerns row states its rule in `statement`; a policies row carries only `title`.
        summary: row
            .text("statement")
            .filter(|s| !s.is_empty())
            .or_else(|| row.text("title"))
            .unwrap_or_default(),
        full_reasoning: row.text("why").unwrap_or_default(),
        binding: row.text("binding").unwrap_or_default(),
        scope_json: canon(scope)?,
        citations: None,
        status: row.status.clone(),
        established_by: row.text("established_by").unwrap_or_default(),
        established_at: None,
        superseded_by: row.superseded_by.clone(),
        created_at: None,
        updated_at: None,
        metadata_json: canon(meta)?,
    })
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn comment(what: &str) -> String {
    format!(
        "GENERATED by `{FIX_CMD}` (elohim/eprfs/epr-cli/src/canon_lift/) — DO NOT HAND-EDIT. \
         {what} Source of truth is the two registry homes (.claude/epr-meta/policies.yaml + \
         .claude/epr-meta/concerns.yaml) and their live canonical-JSON sha256 pins, which this \
         projection does not touch. Every CID is minted through elohim_epr::cid::compute_cid over \
         the exact bytes the pin covers, so the multihash digest and the pin are the same 32 \
         bytes (the P0.4 liftability gate, elohim/epr/tests/concern_canon_liftability.rs). The \
         eprfs/brit substrate consumes these artifacts at graduation; nothing here writes to a \
         DHT."
    )
}

fn json_doc<T: Serialize>(doc: &T) -> CanonLiftResult<String> {
    Ok(serde_json::to_string_pretty(doc)? + "\n")
}

fn render_files(
    atoms: &[Atom],
    heads: &[Head],
    standings: &[Standing],
    errors: &[String],
) -> CanonLiftResult<Vec<(&'static str, String)>> {
    let sources = vec![REGISTRIES[0].0, REGISTRIES[1].0];
    let atoms_doc = AtomsDoc {
        comment: comment(
            "The CONTENT plane: one CIDv1(dag-cbor, sha2-256) EPR atom per registry row, all \
             versions including superseded ones (lineage travels).",
        ),
        canon_lift_version: CANON_LIFT_VERSION,
        generated_from: sources.clone(),
        plane: "content",
        cid_codec: "dag-cbor (0x71)",
        cid_multihash: "sha2-256 (0x12)",
        atoms,
        errors,
    };
    let heads_doc = HeadsDoc {
        comment: comment(
            "The declared-head-over-DAG primitive, one record per policy id. Resolution mode is \
             `pinned`: which version binds a consumer is a DECLARED dependency, never recency, so \
             a new version moves nothing until each binding re-declares.",
        ),
        canon_lift_version: CANON_LIFT_VERSION,
        generated_from: sources.clone(),
        primitive: "declared-head-over-DAG",
        heads,
        errors,
    };
    let standing_doc = StandingDoc {
        comment: comment(
            "The STANDING plane: a Precedent-shaped object per row that CITES its content CID \
             rather than embedding the content. `citations` and the timestamps are null because \
             the registries carry no citation measure and no clock — those are minted by the \
             notarizing action at DHT residency.",
        ),
        canon_lift_version: CANON_LIFT_VERSION,
        generated_from: sources,
        plane: "standing",
        mirrors: "elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs::Precedent",
        standings,
        errors,
    };
    Ok(vec![
        (ATOMS_FILE, json_doc(&atoms_doc)?),
        (HEADS_FILE, json_doc(&heads_doc)?),
        (STANDING_FILE, json_doc(&standing_doc)?),
        (SUMMARY_FILE, render_summary(atoms, heads, errors)),
    ])
}

fn render_summary(atoms: &[Atom], heads: &[Head], errors: &[String]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<!-- GENERATED by `{FIX_CMD}` — DO NOT HAND-EDIT. -->\n\n# Concern canon — brit lift\n\n"
    ));
    let verified = atoms.iter().filter(|a| a.pin_state == "verified").count();
    out.push_str(&format!(
        "**{} rows across both registry homes -> {} CIDv1(dag-cbor, sha2-256) atoms** \
         ({verified} pin-verified, {} declared heads).\n\n",
        atoms.len(),
        atoms.len(),
        heads.len()
    ));
    out.push_str(
        "The content plane (`canon-atoms.json`) and the standing plane \
         (`precedent-standing.json`) are what the v1 YAML row deliberately conflates; \
         `declared-heads.json` carries the brit declared-head-over-DAG primitive at resolution \
         mode `pinned`. Every CID is a codec wrap of the exact bytes the registry's own `sha256` \
         pin already covers — the digests below are the same 32 bytes, rendered twice.\n\n",
    );
    out.push_str(
        "## Declared heads\n\nThe `sha256` column is the registry's own live pin; the `CID` is \
         that same digest wearing a codec. Standing for each version is a separate object in \
         `precedent-standing.json` citing the content CID — a version can lose its force without \
         its content changing an address.\n\n\
         | Policy id | Head | Content sha256 pin | Content CID |\n|---|---|---|---|\n",
    );
    for head in heads {
        out.push_str(&format!(
            "| `{}` | `{}@{}` | `{}` | `{}` |\n",
            head.id,
            head.head.registry,
            head.head.version,
            head.head.sha256_pin.as_deref().unwrap_or("(unpinned)"),
            head.head.cid
        ));
    }
    out.push_str("\n## Lineage (superseded versions travel)\n\n");
    let lineage: Vec<&Atom> = atoms
        .iter()
        .filter(|a| a.superseded_by.is_some() || !a.supersedes.is_empty())
        .collect();
    if lineage.is_empty() {
        out.push_str("No supersession edges declared.\n");
    } else {
        out.push_str("| Row | Status | Supersedes | Superseded by |\n|---|---|---|---|\n");
        for atom in lineage {
            let sup = atom
                .supersedes
                .iter()
                .map(|r| format!("`{}`", r.declared))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "| `{}#{}@{}` | {} | {} | {} |\n",
                atom.registry,
                atom.id,
                atom.version,
                atom.status,
                if sup.is_empty() { "—".into() } else { sup },
                atom.superseded_by
                    .as_ref()
                    .map(|r| format!(
                        "`{}`{}",
                        r.declared,
                        if r.resolved { "" } else { " ⛔ UNRESOLVED" }
                    ))
                    .unwrap_or_else(|| "—".into())
            ));
        }
    }
    out.push_str("\n## Findings\n\n");
    if errors.is_empty() {
        out.push_str(
            "None — every row is pinned, every pin reproduces, every lineage pointer resolves.\n",
        );
    } else {
        for e in errors {
            out.push_str(&format!("- ⛔ {e}\n"));
        }
    }
    out.push_str(&format!(
        "\n---\n\nRegenerate after any registry edit (right after \
         `python3 .claude/scripts/epr-meta-pin.py --write`): `{FIX_CMD}`. \
         `epr canon-lift` alone checks freshness and exits 1 when stale.\n"
    ));
    out
}

// ---------------------------------------------------------------------------
// Write / check
// ---------------------------------------------------------------------------

/// Write the artifact set into `dir` (created if needed), atomically per file.
pub fn write_to(projection: &Projection, dir: &Path) -> CanonLiftResult<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut written = Vec::new();
    for (name, text) in &projection.files {
        let path = dir.join(name);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &path)?;
        written.push(path);
    }
    Ok(written)
}

/// Byte-compare the committed artifacts against a fresh render. Returns the staleness problems.
pub fn check(projection: &Projection, dir: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    for (name, want) in &projection.files {
        let path = dir.join(name);
        match std::fs::read_to_string(&path) {
            Err(_) => problems.push(format!("{OUT_DIR_REL}/{name} is MISSING — run `{FIX_CMD}`")),
            Ok(have) if have != *want => problems.push(format!(
                "{OUT_DIR_REL}/{name} is STALE ({}B on disk vs {}B regenerated) — the canon moved \
                 and its brit-lift projection did not. Run `{FIX_CMD}`.",
                have.len(),
                want.len()
            )),
            Ok(_) => {}
        }
    }
    problems
}

// ---------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------

pub fn run(args: &[String]) -> CanonLiftResult<ExitCode> {
    let mut root = PathBuf::from(".");
    let mut out: Option<PathBuf> = None;
    let mut write = false;
    let mut json = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--write" => {
                write = true;
                i += 1;
            }
            "--check" => {
                write = false;
                i += 1;
            }
            "--json" => {
                json = true;
                i += 1;
            }
            "--root" => {
                root = PathBuf::from(args.get(i + 1).ok_or_else(|| {
                    CanonLiftError::InvalidArguments("--root needs a path".into())
                })?);
                i += 2;
            }
            "--out" => {
                out = Some(PathBuf::from(args.get(i + 1).ok_or_else(|| {
                    CanonLiftError::InvalidArguments("--out needs a path".into())
                })?));
                i += 2;
            }
            other => {
                return Err(CanonLiftError::InvalidArguments(format!(
                    "unknown canon-lift argument `{other}`\n{}",
                    usage()
                )))
            }
        }
    }
    let root = std::fs::canonicalize(&root).unwrap_or(root);
    let dir = out.unwrap_or_else(|| root.join(OUT_DIR_REL));

    let projection = project(&root)?;
    if write {
        let written = write_to(&projection, &dir)?;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "wrote": written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                    "atoms": projection.atoms.len(),
                    "heads": projection.heads.len(),
                    "errors": projection.errors,
                }))?
            );
        } else {
            for path in &written {
                println!("wrote {}", path.display());
            }
            println!(
                "canon-lift: {} atoms · {} declared heads · {} standing records",
                projection.atoms.len(),
                projection.heads.len(),
                projection.standings.len()
            );
            for e in &projection.errors {
                println!("  ⛔ {e}");
            }
        }
        return Ok(if projection.errors.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }

    let problems = check(&projection, &dir);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "fresh": problems.is_empty(),
                "problems": problems,
                "atoms": projection.atoms.len(),
                "heads": projection.heads.len(),
                "errors": projection.errors,
            }))?
        );
    } else if problems.is_empty() && projection.errors.is_empty() {
        println!(
            "canon-lift: fresh ✅ ({} atoms, {} declared heads, both homes)",
            projection.atoms.len(),
            projection.heads.len()
        );
    } else {
        for p in problems.iter().chain(projection.errors.iter()) {
            println!("  ⛔ {p}");
        }
    }
    Ok(if problems.is_empty() && projection.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

pub fn usage() -> String {
    format!(
        "usage: epr canon-lift [--check | --write] [--json] [--root DIR] [--out DIR]\n  \
         projects .claude/epr-meta/{{policies,concerns}}.yaml into {OUT_DIR_REL}/ as \
         CIDv1 EPR atoms + declared heads + Precedent standing"
    )
}
