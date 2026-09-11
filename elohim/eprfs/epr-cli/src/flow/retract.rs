//! `epr flow retract <cid> --reason <text>` — withdraw one sidecar dependency slot.
//!
//! Station six of the memory-kit replacement
//! (`genesis/docs/superpowers/plans/2026-09-10-memory-kit-replacement-finish.md`) inherited a
//! residue no verb could reach. Stations 3b and 5 deleted the Python recall suite and moved the
//! recall contract; the sealed `DepEdge` records naming the old paths stayed behind with BOTH
//! endpoints unreadable, and they head the concern page forever:
//!
//! - `epr flow cites stamp|verify` operates on **document frontmatter** and never sees a sidecar
//!   record;
//! - `epr flow hold` — the verb whose whole purpose is a declared deviation — REFUSES them
//!   (`upstream unreadable (dangling at birth)`), which is the correct gate for a NEW edge and the
//!   wrong one for an already-dangling record;
//! - `epr flow reseal` needs a readable upstream to reseal against.
//!
//! So the missing act is not "repair" and not "delete". The sidecar is append-only and the record
//! is true history: something WAS declared, and later the thing it declared about was removed. The
//! act is **withdrawal** — a later record saying the earlier declaration no longer stands.
//!
//! ## Shape
//!
//! A retraction is a `FeedbackSignal`-shaped run-note: `run:retraction`, subject = the withdrawn
//! record's CID, `reason:` = the authored withdrawal text, resource = that same record's address.
//! It is the same append every other note performs, with the same content-addressed idempotence,
//! and it adds NO new record kind — the DNA-entry-scarcity discipline applied one layer up: a new
//! social move on existing data is a new *tag*, never a new type.
//!
//! ## What is admissible, and why the gates exist
//!
//! Four refusals, all before anything is appended:
//!
//! 1. **Not a `DepEdge`.** A commitment, intent, event, process or spec is withdrawn by its own
//!    vocabulary (supersede, dismiss, close). A retraction that could name any record would be a
//!    universal eraser wearing a maintenance costume.
//! 2. **Fails `DepEdge::validate`.** `FlowStore::edges()` already skips such a record, so no
//!    reader offers it; "withdrawing" it would be a no-op that reads as work.
//! 3. **Superseded in its own slot.** `edges()` collapses to latest-per-`(from, to)`. Retracting a
//!    record that a later record already displaced changes nothing observable, so it is refused
//!    naming the standing record — the "never wire a guaranteed no-op" discipline.
//! 4. **Already retracted with a DIFFERENT reason.** Two withdrawal reasons for one slot is a
//!    ledger a reader cannot resolve. An identical re-run is admitted and dedupes to a no-op,
//!    which is what makes the verb safe to script.
//!
//! ## What reads it
//!
//! [`retracted`] folds the withdrawal set out of the record log; `super::edges::sidecar_plane`
//! consults it and drops the withdrawn slot from the merged graph, so `epr flow context --concerns`,
//! `epr flow memory recall open` (which reads through `concerns`), `walk`, `status` and `reseal`
//! all stop offering it — one filter at the index, rather than five readers each learning a rule.
//! The withdrawn records themselves stay in the sidecar untouched, and the count of them is
//! reported rather than silently subtracted.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use cid::Cid;
use elohim_epr_rea::{DepEdge, FlowRecord, FlowStore, Magnitude, SidecarFlowStore};
use serde::Serialize;

use super::edges::governor_label;
use super::note::{self, NoteActor, NoteOutcome, NOTE_UNIT, REASON_SLOT_PREFIX, RETRACTION_TAG};
use super::{short_cid, FlowError, FlowResult};

/// The withdrawal set: withdrawn record CID → the reason its retraction gave.
///
/// A `BTreeMap` keyed by the CID STRING rather than by [`Cid`] so a reader holding either
/// rendering can look a record up without re-parsing, and so the iteration order is stable for
/// rendering.
pub type Retractions = BTreeMap<String, String>;

/// Fold every `run:retraction` note in a record log into the withdrawal set.
///
/// Latest wins on a repeat: the sidecar is in append order, so a later retraction of the same
/// record replaces the reason an earlier one recorded. (The CLI refuses to *author* a second
/// retraction with a differing reason, but this reader must still behave for a log assembled some
/// other way — a reader that panicked on a shape the writer refuses is a reader that cannot read
/// history.)
///
/// PURE — it takes the records rather than a store, so the whole withdrawal rule is unit-testable
/// without a filesystem.
pub fn retracted(records: &[(Cid, FlowRecord)]) -> Retractions {
    let mut out = Retractions::new();
    for (_, record) in records {
        let FlowRecord::Event(event) = record else {
            continue;
        };
        if !matches!(&event.quantity, Magnitude::Count { unit, .. } if unit == NOTE_UNIT) {
            continue;
        }
        if event.classified_as.first().map(String::as_str) != Some(RETRACTION_TAG) {
            continue;
        }
        // Slot 1 is the subject label, which for a retraction is the withdrawn record's CID
        // string. `resource` carries the same address; the label is used as the key because it is
        // the rendering a reader already has in hand.
        let Some(target) = event.classified_as.get(1) else {
            continue;
        };
        let reason = event
            .classified_as
            .iter()
            .find_map(|slot| slot.strip_prefix(REASON_SLOT_PREFIX))
            .unwrap_or("")
            .to_string();
        out.insert(target.clone(), reason);
    }
    out
}

/// The machine-facing result of one retraction (`--json` consumers read this).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetractOutcome {
    /// The withdrawn record's CID.
    pub record: String,
    /// The slot it declared, as the reader saw it.
    pub from: String,
    pub to: String,
    pub governor: String,
    /// The withdrawn edge's own announcement, when it carried one — what is being withdrawn, in
    /// the words the seal used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared: Option<String>,
    /// Whether each endpoint was readable at withdrawal time. Recorded because "both gone" and
    /// "upstream moved" are different stories and the reason text alone cannot be trusted to say
    /// which one this was.
    pub from_readable: bool,
    pub to_readable: bool,
    pub reason: String,
    /// The retraction note that was appended.
    pub note: NoteOutcome,
}

impl RetractOutcome {
    pub fn render(&self) {
        println!(
            "retract {} → {}  [{}]  {}",
            self.from,
            self.to,
            self.governor,
            short_cid_str(&self.record)
        );
        if let Some(declared) = &self.declared {
            println!("        declared: {declared}");
        }
        println!(
            "        endpoints: from {} · to {}",
            readable_word(self.from_readable),
            readable_word(self.to_readable)
        );
        println!("        reason: {}", self.reason);
        println!(
            "        retraction note: {}{}",
            short_cid_str(&self.note.record_cid),
            if self.note.appended {
                ""
            } else {
                "  (already recorded — no-op)"
            }
        );
    }
}

fn readable_word(readable: bool) -> &'static str {
    if readable {
        "readable"
    } else {
        "unreadable"
    }
}

fn short_cid_str(value: &str) -> String {
    match value.parse::<Cid>() {
        Ok(parsed) => short_cid(&parsed),
        Err(_) => value.to_string(),
    }
}

/// `epr flow retract <cid> --reason <text> [--as …] [--session …] [--json] [--root DIR]`.
pub fn run(root: &Path, json: bool, args: &[String]) -> FlowResult<ExitCode> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    }
    let Some(target) = args.first().filter(|a| !a.starts_with("--")) else {
        return Err(FlowError::InvalidArguments(usage()));
    };
    let rest = &args[1..];
    let reason = super::take_opt(rest, "--reason")?.ok_or_else(|| {
        FlowError::InvalidArguments(
            "retract needs --reason <text> — a withdrawal whose ground is not recorded is \
             indistinguishable from a deletion"
                .into(),
        )
    })?;
    let actor = NoteActor {
        as_ref: super::take_opt(rest, "--as")?,
        session: super::resolve_session(super::take_opt(rest, "--session")?),
    };
    let outcome = retract(root, target, &reason, &actor)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        outcome.render();
    }
    Ok(ExitCode::SUCCESS)
}

fn usage() -> String {
    "usage: epr flow retract <edge-record-cid> --reason <text> [--as agent:<role>@<model>] \
     [--session <id>] [--json] [--root DIR]\n  \
     (withdraw ONE sidecar DepEdge / cite-seal slot: appends a `run:retraction` run-note naming \
     the record, so every edge reader stops offering the slot while the original record stays in \
     the append-only sidecar. Refused for any record that is not a DepEdge, for a record already \
     superseded in its own slot, and for a second retraction carrying a different reason)"
        .to_string()
}

/// Withdraw one sidecar dependency slot. Resolve-everything-then-append, like every other write
/// leg in this family: the four refusals below all happen before the store is opened for writing.
pub fn retract(
    root: &Path,
    target: &str,
    reason: &str,
    actor: &NoteActor,
) -> FlowResult<RetractOutcome> {
    let cid: Cid = target.parse().map_err(|_| {
        FlowError::InvalidArguments(format!(
            "`{target}` is not a CID — a retraction names the exact record it withdraws, and a \
             path or a slug names a slot rather than the record standing in it"
        ))
    })?;

    // Asking a question must not create a store (the discipline `concerns` already holds).
    if !root.join(".eprfs/status/flows.jsonl").exists() {
        return Err(FlowError::InvalidArguments(format!(
            "no flows sidecar under `{}` — there is no record to withdraw",
            root.display()
        )));
    }
    let records = SidecarFlowStore::open(root)?.records()?;

    // ── Gate 1: the record exists, and it is a DepEdge. ──
    let Some((_, record)) = records.iter().find(|(known, _)| known == &cid) else {
        return Err(FlowError::UnknownResource(format!(
            "{target} (no record in this sidecar carries that address)"
        )));
    };
    let FlowRecord::Edge(edge) = record else {
        return Err(FlowError::InvalidArguments(format!(
            "record {target} is a {} — retraction withdraws a sidecar DEPENDENCY slot \
             (a `DepEdge`, i.e. an `epr flow seal`/`hold` record). A commitment, intent, event, \
             process or spec is withdrawn by its own vocabulary, not by this verb",
            record_kind(record)
        )));
    };

    // ── Gate 2: the record is one a reader actually surfaces. ──
    if edge.validate().is_err() {
        return Err(FlowError::InvalidArguments(format!(
            "record {target} fails DepEdge validation, so `FlowStore::edges()` already skips it \
             and no reader offers it — withdrawing it would change nothing"
        )));
    }

    // ── Gate 3: it is the STANDING record of its slot. ──
    if let Some((winner_cid, _)) = standing(&records, edge) {
        if winner_cid != cid {
            return Err(FlowError::InvalidArguments(format!(
                "record {target} is superseded in its slot `{} → {}` by {} — retract the \
                 standing record, or nothing an edge reader shows would change",
                edge.from, edge.to, winner_cid
            )));
        }
    }

    // ── Gate 4: no conflicting prior withdrawal. ──
    let already = retracted(&records);
    if let Some(previous) = already.get(target) {
        if previous != reason {
            return Err(FlowError::InvalidArguments(format!(
                "record {target} is already retracted with a different reason: `{previous}`. \
                 Two withdrawal reasons for one slot is a ledger a reader cannot resolve — \
                 append the correction as `epr flow note --kind correction` instead, or re-run \
                 with the recorded reason verbatim"
            )));
        }
    }

    let note = note::retraction(root, target, reason, actor)?;
    Ok(RetractOutcome {
        record: target.to_string(),
        from: edge.from.clone(),
        to: edge.to.clone(),
        governor: governor_label(&edge.governor),
        declared: edge.desc.clone(),
        from_readable: root.join(&edge.from).is_file(),
        to_readable: root.join(&edge.to).is_file(),
        reason: reason.to_string(),
        note,
    })
}

/// The record currently standing in `edge`'s `(from, to)` slot, by the same fold
/// [`FlowStore::edges`] performs: forward scan, `sealed_at >=` wins, so a later append takes a tie.
///
/// Re-derived here rather than calling `edges()` because this leg needs the WINNER'S CID for a
/// specific slot and `edges()` yields the whole collapsed graph; the two must agree, so the tie
/// rule is transcribed verbatim and pinned by `retract_refuses_a_superseded_record` in
/// `tests/flow_retract.rs`.
fn standing<'a>(records: &'a [(Cid, FlowRecord)], edge: &DepEdge) -> Option<(Cid, &'a DepEdge)> {
    let mut winner: Option<(Cid, &DepEdge)> = None;
    for (cid, record) in records {
        let FlowRecord::Edge(candidate) = record else {
            continue;
        };
        if candidate.from != edge.from || candidate.to != edge.to {
            continue;
        }
        if candidate.validate().is_err() {
            continue;
        }
        match &winner {
            Some((_, standing)) if candidate.sealed_at < standing.sealed_at => {}
            _ => winner = Some((*cid, candidate)),
        }
    }
    winner
}

/// The record's kind, for a refusal that names what the caller actually pointed at.
fn record_kind(record: &FlowRecord) -> &'static str {
    match record {
        FlowRecord::Event(_) => "run event (a note, a fulfilment, an observation)",
        FlowRecord::Commitment(_) => "commitment",
        FlowRecord::Intent(_) => "intent",
        FlowRecord::Process(_) => "process",
        FlowRecord::Spec(_) => "spec",
        FlowRecord::Edge(_) => "dependency edge",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr_rea::{AgentRef, FlowEvent, ReaVerb};

    fn note_event(tag: &str, subject: &str, reason: &str) -> FlowRecord {
        FlowRecord::Event(FlowEvent {
            action: ReaVerb::Cite,
            provider: AgentRef("agent:test".into()),
            receiver: AgentRef("repo:test".into()),
            resource: *eprfs_core::BlobCid::compute_raw(subject.as_bytes()).as_cid(),
            quantity: Magnitude::Count {
                value: 1.0,
                unit: NOTE_UNIT.to_string(),
            },
            process: None,
            in_scope_of: *eprfs_core::BlobCid::compute_raw(b"scope").as_cid(),
            fulfills: Vec::new(),
            satisfies: Vec::new(),
            classified_as: vec![
                tag.to_string(),
                subject.to_string(),
                format!("{REASON_SLOT_PREFIX}{reason}"),
            ],
            occurred_at: "2026-09-10T00:00:00Z".into(),
        })
    }

    fn with_cid(record: FlowRecord) -> (Cid, FlowRecord) {
        (record.cid().unwrap(), record)
    }

    #[test]
    fn only_retraction_tagged_notes_join_the_withdrawal_set() {
        let records = vec![
            with_cid(note_event(RETRACTION_TAG, "bafy-one", "endpoints deleted")),
            with_cid(note_event("run:correction", "bafy-two", "not a withdrawal")),
            with_cid(note_event("run:observation", "bafy-three", "a measurement")),
        ];
        let folded = retracted(&records);
        assert_eq!(folded.len(), 1, "only the retraction tag withdraws");
        assert_eq!(folded.get("bafy-one").unwrap(), "endpoints deleted");
    }

    #[test]
    fn a_later_retraction_of_the_same_record_replaces_the_earlier_reason() {
        let records = vec![
            with_cid(note_event(RETRACTION_TAG, "bafy-one", "first ground")),
            with_cid(note_event(RETRACTION_TAG, "bafy-one", "second ground")),
        ];
        assert_eq!(
            retracted(&records).get("bafy-one").unwrap(),
            "second ground"
        );
    }

    #[test]
    fn a_retraction_with_no_reason_slot_still_withdraws() {
        // Withdrawal is carried by the TAG. A reader that required the reason slot would leave a
        // hand-assembled record silently standing — the failure mode this whole module exists to
        // end, one layer down.
        let mut record = note_event(RETRACTION_TAG, "bafy-one", "ignored");
        if let FlowRecord::Event(event) = &mut record {
            event.classified_as.truncate(2);
        }
        let folded = retracted(&[with_cid(record)]);
        assert_eq!(folded.get("bafy-one").map(String::as_str), Some(""));
    }
}
