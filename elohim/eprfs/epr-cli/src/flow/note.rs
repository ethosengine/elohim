//! `epr flow note` — the run-scale WRITE leg into the EPR plane
//! (`genesis/docs/superpowers/specs/2026-08-13-run-plane-projection-observation-events-design.md`
//! §3; harness-borrows plan Task 1).
//!
//! Every other leg in this family derives records from the tree. This one is the only leg that
//! accepts an *authored* observation: a mid-run correction, a failed approach and what it was
//! switched to, or a plain observation. The seam it fills is precise — an operator's correction
//! lands in the conversation, compaction is lossy summarization of exactly that region, and
//! nothing re-injects it afterwards because the durable plane was never told. A note is how the
//! correction becomes a record instead of a transcript artifact.
//!
//! **A note annotates; it never discharges.** `fulfills` and `satisfies` stay empty, and that is
//! the load-bearing rule of this leg rather than a stylistic preference: the discharged set in
//! `walk` and `fulfill` is derived as *any event whose `fulfills` names the commitment*, so a
//! note that populated `fulfills` would mark its own commitment fulfilled and silently retire
//! live work. Association is carried by `in_scope_of` plus `classified_as`. (The same discipline
//! is already visible upstream: `Dismiss` events carry an empty `fulfills` precisely so a red run
//! cannot discharge anything.)
//!
//! **Identity and idempotence** are the crate's usual atom CID: a note's fields fully determine
//! its address, so appending a byte-identical note twice is a true no-op — one record, not two
//! rows — while two notes differing only in their reason text are two distinct records.
//!
//! **Dating** resolves spec open question Q1 for v1: `occurred_at` is the **git HEAD commit's
//! author date**, the way every other timestamp on this path is history-derived and never
//! `now()`. A note is dated by the tree it was authored against. The accepted coarseness is that
//! notes authored against the same head share a timestamp; intra-session ordering is carried by
//! sidecar append order, and CID dedup is unaffected because distinct reasons give distinct CIDs.
//! Unlike `project` — which derives thousands of records and treats one unreadable history as
//! honest absence for that record — this leg **refuses** rather than emitting an empty
//! `occurred_at`: a single deliberate authored act dated with an empty string is invisible to
//! every window fold and unorderable against its siblings.

use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{
    AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb, SidecarFlowStore,
};
use serde::Serialize;

use super::{
    body_cid_of_file, confine_under, head_commit_provenance, rel_to_root, repo_agent,
    repo_scope_atom, short_cid, FlowError, FlowResult,
};

/// The unit every note is counted in.
///
/// A distinct unit string keeps notes out of every existing unit-keyed fold BY CONSTRUCTION:
/// `elohim_epr_rea::stock::count_in` filters `Magnitude::Count{unit}` by exact string match, so
/// a note can never be mistaken for an artifact, a green-run, or a token by a stock that names
/// one of those units.
const NOTE_UNIT: &str = "run-note";

/// Prefix on the `classified_as` slot carrying the authored body, so a reader can tell an
/// authored string from a tag or a subject at a glance (slots 0 and 1 are the established
/// tag-then-subject convention; everything after them is this leg's, and is prefixed).
const REASON_SLOT_PREFIX: &str = "reason:";

/// Prefix on the optional consequence slot — the second half of a failed approach.
const SWITCHED_TO_SLOT_PREFIX: &str = "switched-to:";

/// The closed triad of run-scale observation kinds.
///
/// Closed on purpose. The vocabulary is the semantics of every downstream read — the projection
/// re-finds notes by these tags, and a future stock fold decides what it counts by them — so a
/// fourth kind is a spec amendment, not a CLI argument. An unrecognised `--kind` is REFUSED here
/// rather than defaulted to `observation`: a defaulted classification is a record that lies about
/// what it is, and it lies quietly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    /// Something was tried and did not work. Pairs with `--switched-to` to carry the consequence,
    /// so a later session inherits the answer without re-reading the failure.
    FailedApproach,
    /// A decision was reversed mid-run — the algedonic channel, written down.
    Correction,
    /// Something worth knowing that reverses nothing.
    Observation,
}

impl NoteKind {
    /// Parse the `--kind` argument. Refuses anything outside the triad, naming the legal set.
    pub fn parse(raw: &str) -> FlowResult<Self> {
        match raw.trim() {
            "failed-approach" => Ok(NoteKind::FailedApproach),
            "correction" => Ok(NoteKind::Correction),
            "observation" => Ok(NoteKind::Observation),
            other => Err(FlowError::InvalidArguments(format!(
                "unknown note kind `{other}` — the vocabulary is closed: \
                 failed-approach|correction|observation"
            ))),
        }
    }

    /// The `classified_as` slot-0 tag this kind mints.
    pub fn tag(self) -> &'static str {
        match self {
            NoteKind::FailedApproach => "run:failed-approach",
            NoteKind::Correction => "run:correction",
            NoteKind::Observation => "run:observation",
        }
    }
}

/// The machine-facing result of one `note` act (`--json` consumers read this).
#[derive(Debug, Serialize)]
pub struct NoteOutcome {
    /// The slot-0 tag (`run:correction`, …).
    pub kind: String,
    /// The resolved target's label — its repo-relative path, or the CID string when `--on` named
    /// an atom directly.
    pub on: String,
    /// The resolved target's full CIDv1 string.
    pub resource: String,
    pub reason: String,
    pub switched_to: Option<String>,
    /// Git HEAD's author date (RFC3339) — the tree the note was authored against.
    pub occurred_at: String,
    /// The atom CID of the `FlowRecord::Event`.
    pub record_cid: String,
    /// `false` when this exact note was already in the sidecar and the append was a no-op.
    pub appended: bool,
}

impl NoteOutcome {
    pub fn render(&self) {
        println!(
            "note    {} → {}  {}",
            self.kind,
            self.on,
            short_cid_str(&self.record_cid)
        );
        println!("        reason: {}", self.reason);
        if let Some(switched) = &self.switched_to {
            println!("        switched to: {switched}");
        }
        if !self.appended {
            println!("        (already recorded — no-op)");
        }
    }
}

fn short_cid_str(cid: &str) -> String {
    match cid.parse::<Cid>() {
        Ok(parsed) => short_cid(&parsed),
        Err(_) => cid.to_string(),
    }
}

/// `epr flow note --on <commitment-cid-or-path> --kind <kind> --reason <text> [--switched-to <text>]`.
///
/// Two phases, mirroring `reseal`'s failure-safety idiom: **Phase 1 resolves everything and
/// appends nothing** — the kind, the authored body, the target, the provenance, the scope, and
/// the record's own address — and **Phase 2 performs the single append** only once every
/// resolution has succeeded. A note that half-wrote would be worse than one that refused, and
/// this leg's whole value is that the record it leaves can be trusted.
pub fn note(
    root: &Path,
    on: &str,
    kind: &str,
    reason: &str,
    switched_to: Option<&str>,
) -> FlowResult<NoteOutcome> {
    // ── Phase 1: resolve. Nothing below this line touches the sidecar until Phase 2. ──

    // Argument shape first, so a malformed invocation never even opens the store.
    let kind = NoteKind::parse(kind)?;
    let reason = non_empty(reason, "--reason")?;
    let switched_to = switched_to
        .map(|s| non_empty(s, "--switched-to"))
        .transpose()?;

    let mut store = SidecarFlowStore::open(root)?;
    let records = store.records()?;
    let (resource, label) = resolve_target(root, on, &records)?;

    // Provenance and clock come from the same single `git log -1`, and an unattributable note is
    // refused rather than dated with a placeholder (module doc).
    let (author, occurred_at) = head_commit_provenance(root).ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "cannot date a note in `{}`: git has no HEAD commit to author it against — \
             a note is dated by the tree it was written against, never by wall clock",
            root.display()
        ))
    })?;

    // Tag first, subject second, authored body after — see `FlowEvent::classified_as`.
    let mut classified_as = Vec::with_capacity(3 + usize::from(switched_to.is_some()));
    classified_as.push(kind.tag().to_string());
    classified_as.push(label.clone());
    classified_as.push(format!("{REASON_SLOT_PREFIX}{reason}"));
    if let Some(switched) = &switched_to {
        classified_as.push(format!("{SWITCHED_TO_SLOT_PREFIX}{switched}"));
    }

    let event = FlowEvent {
        // `Cite`, never `Produce`: a note produces no resource and discharges no promise — it
        // REFERS TO one. `Produce` would make notes count as output in every fold, and `Dismiss`
        // already means regression on the a2o verdict path.
        action: ReaVerb::Cite,
        provider: AgentRef(author),
        receiver: repo_agent(),
        resource,
        quantity: Magnitude::Count {
            value: 1.0,
            unit: NOTE_UNIT.to_string(),
        },
        // A note belongs to no recipe run: inventing a process would assert a stage that never
        // ran.
        process: None,
        in_scope_of: repo_scope_atom()?,
        fulfills: Vec::new(),
        satisfies: Vec::new(),
        classified_as,
        occurred_at: occurred_at.clone(),
    };

    let record = FlowRecord::Event(event);
    let record_cid = record.cid()?;
    let appended = !records.iter().any(|(cid, _)| cid == &record_cid);

    let outcome = NoteOutcome {
        kind: kind.tag().to_string(),
        on: label,
        resource: resource.to_string(),
        reason: reason.to_string(),
        switched_to: switched_to.map(str::to_string),
        occurred_at,
        record_cid: record_cid.to_string(),
        appended,
    };

    // ── Phase 2: append. One record, or none. ──
    if appended {
        store.append(record)?;
    }
    Ok(outcome)
}

/// Reject a blank flag value, naming the flag. An empty `--reason` is the one refusal this leg
/// makes before it opens the store at all: a note with nothing in it is not an observation.
fn non_empty<'a>(value: &'a str, flag: &str) -> FlowResult<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FlowError::InvalidArguments(format!(
            "{flag} needs text — an empty note records nothing"
        )));
    }
    Ok(trimmed)
}

/// Resolve `--on` to `(resource CID, human label)`.
///
/// Two admissible shapes, and no third: a **CIDv1 string**, which must already be an atom this
/// sidecar knows, or a **repo-relative path**, whose canonical body CID is computed the way every
/// other resource identity on this path is. Both refuse with [`FlowError::UnknownResource`]
/// rather than minting an orphan — a note pointing at nothing is worse than no note, because it
/// reads as evidence.
///
/// The CID arm deliberately verifies *membership* instead of accepting any well-formed CID:
/// `atom_cid` will happily parse an address for something that was never recorded, and a note
/// against it would be unreachable from every walk that starts at a real record.
fn resolve_target(
    root: &Path,
    on: &str,
    records: &[(Cid, FlowRecord)],
) -> FlowResult<(Cid, String)> {
    if let Ok(cid) = on.parse::<Cid>() {
        if is_known_atom(&cid, records) {
            return Ok((cid, on.to_string()));
        }
        return Err(FlowError::UnknownResource(format!(
            "{on} (a well-formed CID, but no record in this sidecar mints or names it)"
        )));
    }

    let canonical_root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let abs = if Path::new(on).is_absolute() {
        PathBuf::from(on)
    } else {
        canonical_root.join(on)
    };
    let confined = confine_under(&canonical_root, &abs)?;
    let cid =
        body_cid_of_file(&confined).ok_or_else(|| FlowError::UnknownResource(on.to_string()))?;
    Ok((cid, rel_to_root(&canonical_root, &confined)))
}

/// Does any record in the sidecar mint or name `cid`? Short-circuits on the first hit and builds
/// no index — a note resolves exactly one target, so an owned `HashSet` over 4,000+ records would
/// cost more than the scan it replaces.
fn is_known_atom(cid: &Cid, records: &[(Cid, FlowRecord)]) -> bool {
    records.iter().any(|(record_cid, record)| {
        record_cid == cid
            || match record {
                FlowRecord::Event(e) => &e.resource == cid || &e.in_scope_of == cid,
                FlowRecord::Commitment(c) => &c.in_scope_of == cid,
                FlowRecord::Intent(i) => &i.in_scope_of == cid,
                FlowRecord::Process(p) => {
                    &p.in_scope_of == cid || p.inputs.contains(cid) || p.outputs.contains(cid)
                }
                FlowRecord::Spec(_) | FlowRecord::Edge(_) => false,
            }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr_rea::atom_cid;

    #[test]
    fn the_kind_vocabulary_is_closed_and_refuses_a_fourth() {
        assert_eq!(
            NoteKind::parse("failed-approach").unwrap(),
            NoteKind::FailedApproach
        );
        assert_eq!(NoteKind::parse("correction").unwrap(), NoteKind::Correction);
        assert_eq!(
            NoteKind::parse(" observation ").unwrap(),
            NoteKind::Observation
        );
        let err = NoteKind::parse("regression")
            .expect_err("a fourth kind must be refused, never defaulted to observation");
        assert!(matches!(err, FlowError::InvalidArguments(_)));
        assert!(
            err.to_string().contains("failed-approach"),
            "the refusal must name the legal set; got: {err}"
        );
        assert!(NoteKind::parse("").is_err());
    }

    #[test]
    fn every_kind_mints_a_distinct_run_prefixed_tag() {
        let tags = [
            NoteKind::FailedApproach.tag(),
            NoteKind::Correction.tag(),
            NoteKind::Observation.tag(),
        ];
        assert!(tags.iter().all(|t| t.starts_with("run:")));
        let mut sorted = tags;
        sorted.sort_unstable();
        sorted
            .windows(2)
            .for_each(|w| assert_ne!(w[0], w[1], "two kinds must never share a tag"));
    }

    #[test]
    fn a_blank_reason_is_refused_and_names_the_flag() {
        let err = non_empty("   \n ", "--reason").expect_err("whitespace is not an observation");
        assert!(err.to_string().contains("--reason"));
        assert_eq!(non_empty("  kept  ", "--reason").unwrap(), "kept");
    }

    /// Pinned so canonical dag-cbor encoding of a note `FlowEvent` can never silently drift
    /// (mirrors `fulfill::tests::fulfillment_event_cid_is_stable`). Every field is a literal, so
    /// this golden is independent of git, the clock, and the tree.
    #[test]
    fn note_event_cid_is_stable() {
        let event = FlowEvent {
            action: ReaVerb::Cite,
            provider: AgentRef("author@example.test".to_string()),
            receiver: AgentRef("repo:ethosengine/elohim".to_string()),
            resource: atom_cid(&"golden-note-target".to_string()).expect("cid"),
            quantity: Magnitude::Count {
                value: 1.0,
                unit: NOTE_UNIT.to_string(),
            },
            process: None,
            in_scope_of: atom_cid(&"golden-repo-scope".to_string()).expect("cid"),
            fulfills: Vec::new(),
            satisfies: Vec::new(),
            classified_as: vec![
                "run:failed-approach".to_string(),
                "genesis/plan.md".to_string(),
                "reason:Tried Tsit5, the system is too stiff".to_string(),
                "switched-to:Kvaerno5".to_string(),
            ],
            occurred_at: "2026-08-13T00:00:00Z".to_string(),
        };
        let cid = atom_cid(&event).expect("cid");
        assert_eq!(
            cid.to_string(),
            "bafyreibzhpdchthmt3zlnhjjjsll6ji6p6fmhqlarcoymuhiprzof75jbq"
        );
    }
}
