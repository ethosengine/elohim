//! `epr flow seal | reseal | hold` (spec §2–§3) — the three deliberate acts that write
//! `DepEdge` records into the `.eprfs/status/` sidecar. `seal` blesses a fresh edge,
//! `reseal` supersedes this file's stale outgoing cite-seal edges, `hold` records a
//! declared deviation. All timestamps are git-derived — never a wall-clock read.

use std::path::{Path, PathBuf};

use elohim_epr_rea::{DepEdge, EdgeStatus, FlowRecord, FlowStore, Governor, SidecarFlowStore};
use serde::Serialize;

use super::edges::{edge_verdict, governor_label, EdgeIndex, Verdict};
use super::governor::{derive_governor, load_governor_registry};
use super::{
    body_cid_of_file, confine_under, head_commit_epoch, rel_to_root, repo_agent, FlowError,
    FlowResult,
};

/// The machine-facing result of one seal/reseal/hold act (`--json` consumers read this).
#[derive(Debug, Serialize)]
pub struct SealOutcome {
    /// `seal` | `reseal` | `hold`.
    pub action: String,
    pub from: String,
    pub to: String,
    pub governor: String,
    /// Full sealed CIDv1 string — `Some` only for cite-seal (and hold) edges.
    pub sealed_cid: Option<String>,
    pub sealed_at: i64,
    /// The declared-deviation reason, for `hold`.
    pub held_reason: Option<String>,
    /// The atom CID of the appended `FlowRecord::Edge`.
    pub record_cid: String,
    /// `true` when `--governor` was absent and the governor was auto-derived (spec §3).
    pub derived: bool,
    /// The rule key that fired (`same-cargo-workspace`, `generated-ts`, …). `Some` only when
    /// `derived`.
    pub rule: Option<String>,
    /// The registry policy `id@version` consulted (e.g. `edge-governor-defaults@1`). `Some`
    /// only when `derived`.
    pub policy: Option<String>,
}

impl SealOutcome {
    pub fn render(&self) {
        let seal = self
            .sealed_cid
            .as_deref()
            .map(short)
            .unwrap_or_else(|| "—".to_string());
        match self.action.as_str() {
            "hold" => println!(
                "held    {} → {}  [{}] {}  (reason: {})",
                self.from,
                self.to,
                self.governor,
                seal,
                self.held_reason.as_deref().unwrap_or("")
            ),
            action => println!(
                "{action:<7} {} → {}  [{}] {}",
                self.from, self.to, self.governor, seal
            ),
        }
        if self.derived {
            let rule = self.rule.as_deref().unwrap_or("?");
            let policy = self.policy.as_deref().unwrap_or("?");
            println!(
                "governor derived: {} (rule {rule}, {policy}) — override with --governor",
                self.governor
            );
        }
    }
}

fn short(cid: &str) -> String {
    if cid.len() > 12 {
        format!("{}…{}", &cid[..8], &cid[cid.len() - 4..])
    } else {
        cid.to_string()
    }
}

/// `--governor` value → `Governor` (default cite-seal). Non-seal governors carry a payload
/// after the first colon (`compiler:elohim-storage`, `test:schema_contract.rs`).
fn parse_governor(spec: &str) -> FlowResult<Governor> {
    if spec == "cite-seal" {
        return Ok(Governor::CiteSeal);
    }
    let (kind, value) = spec.split_once(':').ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "governor `{spec}` must be cite-seal or <class>:<payload> \
             (compiler|codegen|schema-contract|test)"
        ))
    })?;
    let value = value.to_string();
    match kind {
        "compiler" => Ok(Governor::Compiler(value)),
        "codegen" => Ok(Governor::Codegen(value)),
        "schema-contract" => Ok(Governor::SchemaContract(value)),
        "test" => Ok(Governor::Test(value)),
        other => Err(FlowError::InvalidArguments(format!(
            "unknown governor class `{other}`"
        ))),
    }
}

/// Resolve a CLI path argument to a repo-relative path under `root` — confined (fix: path
/// confinement): a `--on`/`<file>` argument that resolves outside `root` (`../outside.md`,
/// an absolute path elsewhere) is a clear error, never a silent write outside the tree.
fn rel_under(root: &Path, arg: &str) -> FlowResult<String> {
    let canonical_root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let abs = if Path::new(arg).is_absolute() {
        PathBuf::from(arg)
    } else {
        canonical_root.join(arg)
    };
    let confined = confine_under(&canonical_root, &abs)?;
    Ok(rel_to_root(&canonical_root, &confined))
}

/// The full upstream body CIDv1 required by a cite-seal edge; an unreadable upstream is a
/// dangling-at-birth error (spec §3 — a cite-seal must seal something).
fn seal_upstream(root: &Path, to: &str) -> FlowResult<cid::Cid> {
    body_cid_of_file(&root.join(to)).ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "cannot cite-seal `{to}`: upstream unreadable (dangling at birth)"
        ))
    })
}

/// `epr flow seal <file> --on <upstream> [--governor …] [--desc …]`. When `governor_spec` is
/// `None` (no `--governor` on the CLI), the governor is AUTO-DERIVED from
/// `.claude/epr-meta/governors.yaml` (spec §3) — an explicit `--governor` is a binding
/// override that bypasses derivation entirely (no registry read).
pub fn seal(
    root: &Path,
    file: &str,
    on: &str,
    governor_spec: Option<&str>,
    desc: Option<String>,
) -> FlowResult<SealOutcome> {
    let from = rel_under(root, file)?;
    let to = rel_under(root, on)?;

    let (governor, derivation) = match governor_spec {
        Some(spec) => (parse_governor(spec)?, None),
        None => {
            let registry = load_governor_registry(root)?;
            let (governor, rule) = derive_governor(root, &from, &to, &registry);
            let policy = registry.policy.clone();
            (governor, Some((rule, policy)))
        }
    };

    // Only a cite-seal edge seals a CID; every governed edge records none (the invariant
    // `sealed_cid.is_some() ⇔ CiteSeal` is enforced by `DepEdge::new`). A governed edge still
    // needs a real upstream target — pointing a compiler/codegen/schema-contract/test
    // citation at a nonexistent file is a user error, just one that carries no seal.
    let sealed_cid = if matches!(governor, Governor::CiteSeal) {
        Some(seal_upstream(root, &to)?)
    } else {
        if !root.join(&to).is_file() {
            return Err(FlowError::InvalidArguments(format!(
                "cannot seal `{to}` as governor `{}`: upstream target does not exist \
                 (a governed edge to a nonexistent target is still a user error)",
                governor_label(&governor)
            )));
        }
        None
    };
    let sealed_at = head_commit_epoch(root).unwrap_or(0);

    let edge = DepEdge::new(
        from,
        to,
        desc,
        governor,
        sealed_cid,
        repo_agent(),
        sealed_at,
        None,
    )?;
    let mut outcome = append_edge(root, "seal", edge)?;
    if let Some((rule, policy)) = derivation {
        outcome.derived = true;
        outcome.rule = Some(rule.to_string());
        outcome.policy = Some(policy);
    }
    Ok(outcome)
}

/// `epr flow reseal <file> [--on <upstream>] [--all-stale]` — the deliberate re-bless.
/// Supersedes SIDECAR cite-seal edges only (doc `cites:` are cite-gen-managed and stay
/// untouched). With `--on`, reseals that one edge; with `--all-stale`, every stale outgoing
/// cite-seal sidecar edge of `file`.
pub fn reseal(
    root: &Path,
    file: &str,
    on: Option<&str>,
    all_stale: bool,
) -> FlowResult<Vec<SealOutcome>> {
    let from = rel_under(root, file)?;
    let on_rel = on.map(|target| rel_under(root, target)).transpose()?;
    let store = SidecarFlowStore::open(root)?;
    let index = EdgeIndex::build(root, &store)?;

    // Candidate slots: this file's outgoing sidecar cite-seal edges (skip governed/held/doc).
    // Both selection modes gate on staleness (fix: `--on` on a currently-Ok edge must error,
    // never silently re-append) — `--on` narrows to one slot, `--all-stale` takes every one.
    let candidates: Vec<(String, Option<String>)> = index
        .outgoing(&from)
        .filter(|e| {
            e.plane == super::edges::EdgePlane::Sidecar
                && matches!(e.governor, Governor::CiteSeal)
                && e.held.is_none()
        })
        .filter(|e| match &on_rel {
            Some(target) => &e.to == target && matches!(edge_verdict(root, e), Verdict::Stale),
            None => all_stale && matches!(edge_verdict(root, e), Verdict::Stale),
        })
        .map(|e| (e.to.clone(), e.desc.clone()))
        .collect();

    if candidates.is_empty() {
        let hint = match &on_rel {
            Some(target) => {
                let slot_exists = index.outgoing(&from).any(|e| {
                    e.plane == super::edges::EdgePlane::Sidecar
                        && matches!(e.governor, Governor::CiteSeal)
                        && e.held.is_none()
                        && &e.to == target
                });
                if slot_exists {
                    format!("edge {from} → {target} is not stale; nothing to reseal")
                } else {
                    format!("no sidecar cite-seal edge {from} → {target}")
                }
            }
            None => {
                format!("no stale outgoing cite-seal edge for {from} (use --on or --all-stale)")
            }
        };
        return Err(FlowError::InvalidArguments(hint));
    }

    let sealed_at = head_commit_epoch(root).unwrap_or(0);

    // Phase 1 — resolve EVERY candidate's upstream seal before appending anything. A
    // resolve failure on one candidate must not leave earlier candidates already appended
    // (fix: no partial-write window) — collect every failure, then append nothing at all.
    let mut resolved = Vec::with_capacity(candidates.len());
    let mut failures = Vec::new();
    for (to, desc) in candidates {
        match seal_upstream(root, &to) {
            Ok(sealed_cid) => resolved.push((to, desc, sealed_cid)),
            Err(err) => failures.push(format!("{to}: {err}")),
        }
    }
    if !failures.is_empty() {
        return Err(FlowError::InvalidArguments(format!(
            "reseal aborted, nothing appended — {} upstream(s) failed to resolve: {}",
            failures.len(),
            failures.join("; ")
        )));
    }

    // Phase 2 — every candidate resolved; append.
    let mut store = store;
    let mut outcomes = Vec::with_capacity(resolved.len());
    for (to, desc, sealed_cid) in resolved {
        let edge = DepEdge::new(
            from.clone(),
            to,
            desc,
            Governor::CiteSeal,
            Some(sealed_cid),
            repo_agent(),
            sealed_at,
            None,
        )?;
        outcomes.push(append_edge_to(&mut store, "reseal", edge)?);
    }
    Ok(outcomes)
}

/// `epr flow hold <file> --on <upstream> --reason <text> [--valid-from <iso8601>]` — record
/// a declared deviation. Hold applies to the cite-seal class (only that class goes stale);
/// it still seals the current upstream CID so the invariant holds, then marks it `Held`.
pub fn hold(
    root: &Path,
    file: &str,
    on: &str,
    reason: String,
    valid_from: Option<&str>,
) -> FlowResult<SealOutcome> {
    let from = rel_under(root, file)?;
    let to = rel_under(root, on)?;
    let sealed_cid = seal_upstream(root, &to)?;
    let sealed_at = head_commit_epoch(root).unwrap_or(0);
    let valid_from = match valid_from {
        Some(iso) => iso8601_to_epoch(iso)?,
        None => sealed_at,
    };

    let edge = DepEdge::new(
        from,
        to,
        None,
        Governor::CiteSeal,
        Some(sealed_cid),
        repo_agent(),
        sealed_at,
        Some(EdgeStatus::Held {
            reason: reason.clone(),
            valid_from,
            superseded_by: None,
        }),
    )?;
    let mut outcome = append_edge(root, "hold", edge)?;
    outcome.held_reason = Some(reason);
    Ok(outcome)
}

fn append_edge(root: &Path, action: &str, edge: DepEdge) -> FlowResult<SealOutcome> {
    let mut store = SidecarFlowStore::open(root)?;
    append_edge_to(&mut store, action, edge)
}

fn append_edge_to(
    store: &mut SidecarFlowStore,
    action: &str,
    edge: DepEdge,
) -> FlowResult<SealOutcome> {
    let outcome = SealOutcome {
        action: action.to_string(),
        from: edge.from.clone(),
        to: edge.to.clone(),
        governor: governor_label(&edge.governor),
        sealed_cid: edge.sealed_cid.map(|c| c.to_string()),
        sealed_at: edge.sealed_at,
        held_reason: edge
            .status
            .as_ref()
            .map(|EdgeStatus::Held { reason, .. }| reason.clone()),
        record_cid: String::new(),
        derived: false,
        rule: None,
        policy: None,
    };
    let record_cid = store.append(FlowRecord::Edge(edge))?;
    Ok(SealOutcome {
        record_cid: record_cid.to_string(),
        ..outcome
    })
}

/// Parse an ISO-8601 date/datetime (`YYYY-MM-DD` or `YYYY-MM-DDThh:mm:ss[Z]`, UTC) into a
/// Unix epoch. Offset suffixes beyond `Z` are ignored (treated as UTC) — enough for the
/// `--valid-from` override; the default path never touches this (it uses the git epoch).
fn iso8601_to_epoch(s: &str) -> FlowResult<i64> {
    let s = s.trim();
    let (date, time) = match s.split_once(['T', ' ']) {
        Some((d, t)) => (d, t),
        None => (s, ""),
    };
    let mut dp = date.split('-');
    let year = parse_field(dp.next(), "year", s)?;
    let month = parse_field(dp.next(), "month", s)?;
    let day = parse_field(dp.next(), "day", s)?;

    let (h, mi, se) = if time.is_empty() {
        (0, 0, 0)
    } else {
        // Strip a trailing `Z` or a `±hh:mm` offset — treat the wall time as UTC.
        let core = time.trim_end_matches('Z');
        let core = core
            .split_once(['+'])
            .map(|(t, _)| t)
            .unwrap_or(core)
            .trim();
        let mut tp = core.split(':');
        (
            parse_field(tp.next(), "hour", s)?,
            parse_field(tp.next(), "minute", s).unwrap_or(0),
            parse_field(tp.next(), "second", s).unwrap_or(0),
        )
    };
    Ok(days_from_civil(year, month, day) * 86_400 + h * 3600 + mi * 60 + se)
}

fn parse_field(part: Option<&str>, name: &str, whole: &str) -> FlowResult<i64> {
    part.and_then(|p| p.trim().parse::<i64>().ok())
        .ok_or_else(|| FlowError::InvalidArguments(format!("bad {name} in --valid-from `{whole}`")))
}

/// Days since the Unix epoch for a civil date (Howard Hinnant's algorithm; proleptic
/// Gregorian, valid for any year). No wall-clock, no external date dependency.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_governor_specs() {
        assert_eq!(parse_governor("cite-seal").unwrap(), Governor::CiteSeal);
        assert_eq!(
            parse_governor("compiler:elohim-storage").unwrap(),
            Governor::Compiler("elohim-storage".into())
        );
        assert_eq!(
            parse_governor("test:schema_contract.rs").unwrap(),
            Governor::Test("schema_contract.rs".into())
        );
        assert!(parse_governor("bogus").is_err());
        assert!(parse_governor("weird:x").is_err());
    }

    #[test]
    fn iso8601_epoch_golden() {
        // 1970-01-01 is the epoch.
        assert_eq!(iso8601_to_epoch("1970-01-01").unwrap(), 0);
        // A known UTC instant: 2026-07-21T00:00:00Z.
        assert_eq!(
            iso8601_to_epoch("2026-07-21T00:00:00Z").unwrap(),
            1_784_592_000
        );
        // Date-only defaults to midnight UTC.
        assert_eq!(iso8601_to_epoch("2026-07-21").unwrap(), 1_784_592_000);
        assert!(iso8601_to_epoch("not-a-date").is_err());
    }
}
