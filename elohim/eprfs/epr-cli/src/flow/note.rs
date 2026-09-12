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
//!
//! **Attribution** answers "whose act is this" in three arms, resolved before anything is
//! appended. A named identity (`--as agent:<role>@<model>`) is the provider outright; a resolved
//! session asks the actor sidecar who registered for it and uses that; neither leaves the note
//! attributed to the git author exactly as it always was. The two agent arms additionally carry
//! `steward:<git-author-email>` as the LAST `classified_as` slot, because the human whose key
//! signs the tree does not stop being answerable for it when an agent authors inside it — the
//! steward is a property of the commit, not of the claim, and losing it is how attribution turns
//! into deniability.
//!
//! The identity plane is never allowed to break this leg. A session that registered nothing, an
//! absent sidecar, and an unreadable one all fall through to the author-attributed arm with a
//! notice on stderr, because a note that refused to record a correction over a missing identity
//! record would lose the observation to protect the attribution. The one refusal is a MALFORMED
//! `--as`: substituting the author for an identity the caller explicitly named would mint a
//! record asserting that someone else spoke.

use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{
    parse_agent_ref, ActorStore, AgentRef, FlowEvent, FlowRecord, FlowStore, Magnitude, ReaVerb,
    SidecarActorStore, SidecarFlowStore,
};
use serde::Serialize;

use super::measures::{MeasureRef, Registry};
use super::{
    body_cid_of_file, confine_under, head_commit_provenance, rel_to_root, repo_agent,
    repo_scope_atom, short_cid, FlowError, FlowResult,
};
use std::collections::BTreeMap;

/// The unit every note is counted in.
///
/// A distinct unit string keeps notes out of every existing unit-keyed fold BY CONSTRUCTION:
/// `elohim_epr_rea::stock::count_in` filters `Magnitude::Count{unit}` by exact string match, so
/// a note can never be mistaken for an artifact, a green-run, or a token by a stock that names
/// one of those units.
pub(crate) const NOTE_UNIT: &str = "run-note";

/// Prefix on the `classified_as` slot carrying the authored body, so a reader can tell an
/// authored string from a tag or a subject at a glance (slots 0 and 1 are the established
/// tag-then-subject convention; everything after them is this leg's, and is prefixed).
pub(crate) const REASON_SLOT_PREFIX: &str = "reason:";

/// Prefix on the optional consequence slot — the second half of a failed approach.
pub(crate) const SWITCHED_TO_SLOT_PREFIX: &str = "switched-to:";

/// Prefix on the optional slot naming the correction this note CLOSES.
///
/// Closure by identity, not by date. `epr flow concerns --corrections` reads unresolved corrections
/// by asking which `run:correction` records have no LATER note carrying this slot with their exact
/// CID — so "was this fixed" is answered by an explicit act naming what it closed, never by a
/// filename's date or by a ceremony having run afterwards. Positioned AFTER `switched-to:` and
/// BEFORE `verdict:`, so a note that closes nothing emits exactly the slot vector it emitted before
/// this vocabulary existed and keeps its content address.
pub(crate) const CLOSES_SLOT_PREFIX: &str = "closes:";

/// Prefix on the optional audit-outcome slot — a `verdict` note's whole point.
///
/// Positioned AFTER `reason:`/`switched-to:` and BEFORE `steward:` (which stays last): the slot
/// vocabulary is positional and ADDITIVE, so a note that carries no verdict emits exactly the
/// slot vector it emitted before this const existed and keeps its content address.
pub(crate) const VERDICT_SLOT_PREFIX: &str = "verdict:";

/// The two admissible audit outcomes. Closed for the same reason [`NoteKind`] is: a verdict is
/// read by whoever decides whether a delivery stands, and a third spelling of "yes" would
/// partition that read.
const VERDICT_APPROVED: &str = "approved";
const VERDICT_CHANGES_REQUESTED: &str = "changes-requested";

/// Prefix on the final slot naming the human whose key signs the tree the note was written in.
///
/// LAST on purpose, and appended only on the agent-attributed arms: readers index the leading
/// slots positionally (tag, subject, reason, then the optional consequence), so a steward slot
/// inserted anywhere earlier would renumber a vocabulary other legs already read.
pub(crate) const STEWARD_SLOT_PREFIX: &str = "steward:";

/// Prefix on the slot naming the pinned measure a STRUCTURED observation was taken under.
///
/// A structured observation is the one note kind that is read arithmetically rather than by a
/// human, so its slots are a small closed vocabulary rather than prose: `measure:`, `value:`, an
/// optional `unit:`, and zero or more `env:` pairs. They sit AFTER the authored `reason:` and
/// BEFORE the trailing `verdict:`/`steward:`/attribution slots, which keeps every note that
/// carries no measure emitting exactly the slot vector it emitted before this vocabulary existed
/// — the additive discipline that preserves existing content addresses.
pub(crate) const MEASURE_SLOT_PREFIX: &str = "measure:";

/// Prefix on the slot carrying the observed magnitude, canonically formatted (see
/// [`canonical_number`]) so `8` and `8.0` are one measurement with one address rather than two.
pub(crate) const VALUE_SLOT_PREFIX: &str = "value:";

/// Prefix on the optional slot naming the unit the value is counted in.
pub(crate) const UNIT_SLOT_PREFIX: &str = "unit:";

/// Prefix on each `k=v` environment slot. Emitted in sorted key order, because the environment is
/// part of the memoization key and a key order that varied with argument order would mint two
/// addresses for one measurement.
pub(crate) const ENV_SLOT_PREFIX: &str = "env:";

/// The `env:` key naming the sampled journey's own `FlowEvent` cid (fix round 1, F1 of
/// governed-discovery station 3, task 3.3). Written by `recall::sample`'s three folds and
/// `recall::judge`'s mistaken-assertions fold onto the same journey; read by
/// `report::evaluate_rate_over_window` to group folds back into journeys instead of counting each
/// fold as its own population member. One constant so a writer and the reader can never spell the
/// key two ways.
pub(crate) const JOURNEY_ENV_KEY: &str = "journey";

/// The actor sidecar, relative to the root. Its EXISTENCE is checked before it is opened, because
/// [`SidecarActorStore::open`] creates the tree — and a read path that leaves `.eprfs/` behind on
/// a repository that never had one has written a record of having looked.
const ACTOR_LOG_REL: &str = ".eprfs/status/actors.jsonl";

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
    /// A CONTROL decision — accept, defer, order a fix round, hold (VSM System 3 and System 5).
    /// The `--reason` text IS the ruling; there is no separate outcome vocabulary, because a
    /// control decision's content is precisely what cannot be enumerated in advance.
    Ruling,
    /// An AUDIT outcome — a review seat's verdict on a delivery (VSM System 3*). Unlike every
    /// other kind it carries a closed outcome value alongside its reason, because "was this
    /// accepted" must be readable without parsing prose.
    Verdict,
    /// A sidecar slot is WITHDRAWN — the declaration it carried no longer stands, and every
    /// reader should stop offering it (VSM System 3, the maintenance act).
    ///
    /// Deliberately NOT reachable from `epr flow note --kind`: a retraction is admissible only
    /// against a `DepEdge`/cite-seal slot that is the standing record of its own slot, and that
    /// eligibility check lives in [`super::retract`]. A kind that could be typed without the
    /// check would let a caller "withdraw" a commitment, an intent or a fold — records whose
    /// withdrawal has its own vocabulary — and the withdrawal would read as authoritative.
    Retraction,
}

impl NoteKind {
    /// Parse the `--kind` argument. Refuses anything outside the triad, naming the legal set.
    pub fn parse(raw: &str) -> FlowResult<Self> {
        match raw.trim() {
            "failed-approach" => Ok(NoteKind::FailedApproach),
            "correction" => Ok(NoteKind::Correction),
            "observation" => Ok(NoteKind::Observation),
            "ruling" => Ok(NoteKind::Ruling),
            "verdict" => Ok(NoteKind::Verdict),
            // Named, not merely absent. The tag EXISTS in the record vocabulary, so a reader who
            // finds `run:retraction` in the sidecar and reaches for `--kind retraction` must be
            // told where the act lives rather than that the kind is unknown — an "unknown kind"
            // refusal here would read as "retraction is not a thing", which is false.
            "retraction" => Err(FlowError::InvalidArguments(
                "retraction is not authored through `note` — it withdraws a sidecar dependency \
                 slot and is admissible only against the standing DepEdge record of that slot: \
                 `epr flow retract <edge-record-cid> --reason <text>`"
                    .into(),
            )),
            other => Err(FlowError::InvalidArguments(format!(
                "unknown note kind `{other}` — the vocabulary is closed: \
                 failed-approach|correction|observation|ruling|verdict"
            ))),
        }
    }

    /// The `classified_as` slot-0 tag this kind mints.
    pub fn tag(self) -> &'static str {
        match self {
            NoteKind::FailedApproach => "run:failed-approach",
            NoteKind::Correction => "run:correction",
            NoteKind::Observation => "run:observation",
            NoteKind::Ruling => "run:ruling",
            NoteKind::Verdict => "run:verdict",
            NoteKind::Retraction => RETRACTION_TAG,
        }
    }
}

/// The slot-0 tag every retraction carries.
///
/// A `const` rather than a literal inside [`NoteKind::tag`] because two other modules read it:
/// [`super::retract`] mints it and [`super::edges`] folds it into the sidecar plane's withdrawal
/// set. A tag string that lived in three places would drift the day one of them was edited.
pub(crate) const RETRACTION_TAG: &str = "run:retraction";

/// Resolve `--verdict` against `--kind`, refusing both mismatches rather than defaulting either.
///
/// Two refusals, and they are the same refusal seen from each side. `--kind verdict` with no
/// outcome would mint an audit record that records no outcome; `--verdict` on any other kind
/// would attach an audit outcome to a record nobody reads one from — a slot that is silently
/// ignored is worse than one that is refused, because it reads as evidence.
fn resolve_verdict(kind: NoteKind, verdict: Option<&str>) -> FlowResult<Option<String>> {
    match (kind, verdict) {
        (NoteKind::Verdict, Some(raw)) => {
            let value = non_empty(raw, "--verdict")?;
            if value == VERDICT_APPROVED || value == VERDICT_CHANGES_REQUESTED {
                Ok(Some(value.to_string()))
            } else {
                Err(FlowError::InvalidArguments(format!(
                    "unknown --verdict `{value}` — the outcome vocabulary is closed: \
                     {VERDICT_APPROVED}|{VERDICT_CHANGES_REQUESTED}"
                )))
            }
        }
        (NoteKind::Verdict, None) => Err(FlowError::InvalidArguments(format!(
            "--kind verdict needs --verdict {VERDICT_APPROVED}|{VERDICT_CHANGES_REQUESTED} — \
             an audit record that names no outcome is not a verdict"
        ))),
        (other, Some(_)) => Err(FlowError::InvalidArguments(format!(
            "--verdict belongs to --kind verdict alone; got --kind with tag `{}` — \
             a verdict slot on any other kind is read by nobody",
            other.tag()
        ))),
        (_, None) => Ok(None),
    }
}

/// Resolve `--closes` against `--kind`, refusing a malformed CID and the one kind that cannot close.
///
/// A `failed-approach` records that something did NOT work; letting it close a correction would let
/// "we tried and gave up" read as "the correction is resolved", which is exactly the inference the
/// identity-based reading exists to prevent. Everything else in the vocabulary — an observation of
/// the repair, a ruling, an audit verdict, or a follow-up correction — is a legitimate closure act.
fn resolve_closes(kind: NoteKind, closes: Option<&str>) -> FlowResult<Option<String>> {
    let Some(raw) = closes else {
        return Ok(None);
    };
    let value = non_empty(raw, "--closes")?;
    if kind == NoteKind::FailedApproach {
        return Err(FlowError::InvalidArguments(
            "--closes does not belong to --kind failed-approach — an approach that did not work \
             closes nothing; record the repair as observation|correction|ruling|verdict"
                .into(),
        ));
    }
    value.parse::<Cid>().map_err(|_| {
        FlowError::InvalidArguments(format!(
            "--closes `{value}` is not a CID — a closure names the exact correction it discharges, \
             and a prose reference closes nothing a reader can verify"
        ))
    })?;
    Ok(Some(value.to_string()))
}

/// The positional `classified_as` slot vector for one note.
///
/// One place builds it, so the record and every reader share a single definition of the order:
/// tag, subject, `reason:`, optional `switched-to:`, optional `verdict:`, optional `steward:`
/// LAST. The order is ADDITIVE — a note that carries no verdict emits the same vector it always
/// did, which is what keeps every existing note's content address where it is.
#[allow(clippy::too_many_arguments)]
fn note_slots(
    kind: NoteKind,
    subject: &str,
    reason: &str,
    switched_to: Option<&str>,
    closes: Option<&str>,
    verdict: Option<&str>,
    steward: Option<&str>,
) -> Vec<String> {
    let mut slots = Vec::with_capacity(
        3 + usize::from(switched_to.is_some())
            + usize::from(closes.is_some())
            + usize::from(verdict.is_some())
            + usize::from(steward.is_some()),
    );
    slots.push(kind.tag().to_string());
    slots.push(subject.to_string());
    slots.push(format!("{REASON_SLOT_PREFIX}{reason}"));
    if let Some(switched) = switched_to {
        slots.push(format!("{SWITCHED_TO_SLOT_PREFIX}{switched}"));
    }
    if let Some(closed) = closes {
        slots.push(format!("{CLOSES_SLOT_PREFIX}{closed}"));
    }
    if let Some(value) = verdict {
        slots.push(format!("{VERDICT_SLOT_PREFIX}{value}"));
    }
    if let Some(steward) = steward {
        slots.push(format!("{STEWARD_SLOT_PREFIX}{steward}"));
    }
    slots
}

/// One structured measurement, resolved and validated before any store is opened.
///
/// This is the fold the native report reads. The kit's equivalent is a number written into a
/// private JSON file by whichever script computed it; the difference that matters is not the
/// storage medium but the *key*: a fold is addressed by `subject × measure@version × env`, so two
/// runs of the same measure under different conditions are two distinct records rather than one
/// overwritten file, and a report can say which one it read.
#[derive(Debug, Clone)]
pub struct Observation {
    /// The pinned measure, already checked against the registry.
    pub measure: MeasureRef,
    /// The observed magnitude.
    pub value: f64,
    /// The unit, when the caller named one or the registry declared one.
    pub unit: Option<String>,
    /// The environment this measurement was taken under. A `BTreeMap` rather than a `Vec` because
    /// the key order is part of the address and must not follow argument order.
    pub env: BTreeMap<String, String>,
}

impl Observation {
    /// The `classified_as` slots this observation contributes, in their fixed order.
    fn slots(&self) -> Vec<String> {
        let mut slots = Vec::with_capacity(2 + usize::from(self.unit.is_some()) + self.env.len());
        slots.push(format!("{MEASURE_SLOT_PREFIX}{}", self.measure));
        slots.push(format!(
            "{VALUE_SLOT_PREFIX}{}",
            canonical_number(self.value)
        ));
        if let Some(unit) = &self.unit {
            slots.push(format!("{UNIT_SLOT_PREFIX}{unit}"));
        }
        for (key, value) in &self.env {
            slots.push(format!("{ENV_SLOT_PREFIX}{key}={value}"));
        }
        slots
    }

    /// The reason text a structured observation carries when the caller authored none.
    ///
    /// Derived rather than defaulted to a constant: the reason is part of the record's address, so
    /// deriving it from the very fields that key the fold means two identical measurements mint
    /// one CID (the idempotence this leg promises) while two different ones never collide.
    fn derived_reason(&self) -> String {
        let mut reason = format!("{} = {}", self.measure, canonical_number(self.value));
        if let Some(unit) = &self.unit {
            reason.push(' ');
            reason.push_str(unit);
        }
        reason
    }
}

/// The canonical subject string naming the repository itself.
///
/// Some bounds are not about a file. Cleanup pressure, report-tier size and pending scope moves
/// are properties of the whole tree, and forcing each to nominate a stand-in file would make the
/// fold's key a lie about what was measured. `.` is the spelling every side already uses — the
/// hooks bridge writes it, a registry row declares `subject: .`, and a person types it — so one
/// constant keeps the three in agreement instead of three conventions that drift.
pub const REPO_SUBJECT: &str = ".";

/// Whether `on` names the repository root rather than something inside it.
///
/// Accepts `.`, `./`, the empty string, and an absolute path that canonicalizes to the root — the
/// same claim spelled four ways. Relative spellings are matched literally rather than
/// canonicalized, because `std::fs::canonicalize` resolves against the PROCESS cwd, not `root`,
/// and a `--root` pointing elsewhere would otherwise silently agree with whatever directory the
/// caller happened to be standing in.
pub(crate) fn is_repo_root(root: &Path, on: &str) -> bool {
    let trimmed = on.trim();
    if matches!(trimmed, "" | "." | "./") {
        return true;
    }
    if !Path::new(trimmed).is_absolute() {
        return false;
    }
    match (std::fs::canonicalize(root), std::fs::canonicalize(trimmed)) {
        (Ok(root), Ok(target)) => root == target,
        _ => false,
    }
}

/// Normalize a subject so every spelling of the repository root reads as one.
///
/// Applied on BOTH sides of the report's match — the fold's recorded subject and the bound's
/// declared one — so a row spelling it `./` and a fold spelling it `.` are not two subjects.
pub fn normalize_subject(subject: &str) -> &str {
    match subject.trim() {
        "" | "." | "./" => REPO_SUBJECT,
        other => other,
    }
}

/// Format a magnitude so that equal numbers have equal spellings.
///
/// Integral values render without a fractional part, everything else uses Rust's shortest
/// round-tripping form. Without this, `--value 8` and `--value 8.0` would be two records claiming
/// one measurement, and the dedupe promise would hold only for callers who happened to type the
/// same way.
pub(crate) fn canonical_number(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// Who the caller says is acting, as the CLI shell resolved it.
///
/// A struct rather than two more positional parameters because the two fields are one question
/// asked twice — "who is acting, and under which run" — and a caller that supplies neither is
/// asking for the unattributed behaviour this leg has always had, which [`Default`] states in one
/// word at every call site that does not care.
///
/// Both fields arrive already resolved: `session` in particular has had its environment fallback
/// applied by the shell, so [`note`] itself reads no environment and two runs given the same
/// arguments mint the same record.
#[derive(Debug, Clone, Default)]
pub struct NoteActor {
    /// A claimed identity named outright, `agent:<role>@<model>`.
    pub as_ref: Option<String>,
    /// The run whose registered claim should be consulted when no identity was named.
    pub session: Option<String>,
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
    /// The correction this note closes, present only when `--closes` named one. Omitted rather than
    /// null so every pre-closure payload is byte-identical to the one it emitted before this field
    /// existed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closes: Option<String>,
    /// The audit outcome, present only on a `verdict` note. Omitted rather than null so every
    /// pre-verdict payload is byte-identical to the one it emitted before this field existed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    /// The claimed identity this note was attributed to, absent when it stayed with the commit
    /// author. Omitted rather than null so an unattributed note's payload is byte-identical to
    /// the one it emitted before this field existed — the additive discipline
    /// `ActorClaim::definition_cid` already holds on the record side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    /// Exact registered claim used for both validation and emission, absent for direct attribution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_claim: Option<String>,
    /// The git-signing human answerable for the tree, carried only when `actor` is present. On
    /// the author-attributed arm it would merely repeat the provider, and a field that sometimes
    /// restates another is a field readers learn to ignore.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward: Option<String>,
    /// The pinned measure, present only on a structured observation. Omitted rather than null so
    /// every pre-measure payload stays byte-identical to the one it emitted before this field
    /// existed — the same additive discipline `verdict` and `actor` already hold.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measure: Option<String>,
    /// The observed magnitude, canonically formatted so the payload spells it the way the record
    /// does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The unit the value is counted in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// The environment the measurement was taken under.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
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
        if let Some(closes) = &self.closes {
            println!("        closes: {}", short_cid_str(closes));
        }
        if let Some(verdict) = &self.verdict {
            println!("        verdict: {verdict}");
        }
        if let (Some(measure), Some(value)) = (&self.measure, &self.value) {
            let unit = self
                .unit
                .as_ref()
                .map(|u| format!(" {u}"))
                .unwrap_or_default();
            println!("        measure: {measure} = {value}{unit}");
            if !self.env.is_empty() {
                let env: Vec<String> = self.env.iter().map(|(k, v)| format!("{k}={v}")).collect();
                println!("        env: {}", env.join(" "));
            }
        }
        if let Some(actor) = &self.actor {
            println!("        actor: {actor}");
        }
        if let Some(steward) = &self.steward {
            println!("        steward: {steward}");
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
    verdict: Option<&str>,
    actor: &NoteActor,
) -> FlowResult<NoteOutcome> {
    note_with_options(
        root,
        on,
        kind,
        reason,
        switched_to,
        verdict,
        actor,
        &super::acceptance::AcceptanceOptions::default(),
    )
}

/// Author a note with optional appointed acceptance metadata under the same append lock.
#[allow(clippy::too_many_arguments)]
pub fn note_with_options(
    root: &Path,
    on: &str,
    kind: &str,
    reason: &str,
    switched_to: Option<&str>,
    verdict: Option<&str>,
    actor: &NoteActor,
    options: &super::acceptance::AcceptanceOptions,
) -> FlowResult<NoteOutcome> {
    note_with_options_closing(
        root,
        on,
        kind,
        reason,
        switched_to,
        None,
        verdict,
        actor,
        options,
    )
}

/// The same append, with an explicit `--closes <correction-cid>` claim attached.
///
/// A separate entry point rather than a ninth parameter on [`note_with_options`]: every existing
/// caller keeps its signature, and the one act that carries a closure names it at the call site
/// instead of threading a `None` through the whole family.
#[allow(clippy::too_many_arguments)]
pub fn note_with_options_closing(
    root: &Path,
    on: &str,
    kind: &str,
    reason: &str,
    switched_to: Option<&str>,
    closes: Option<&str>,
    verdict: Option<&str>,
    actor: &NoteActor,
    options: &super::acceptance::AcceptanceOptions,
) -> FlowResult<NoteOutcome> {
    note_with_options_guard(
        root,
        on,
        NoteKind::parse(kind)?,
        reason,
        switched_to,
        closes,
        verdict,
        actor,
        options,
        None,
        |_| Ok(()),
    )
}

/// `epr flow note --kind observation --measure <id@version> --subject <path-or-cid> --value <n>
/// [--unit <u>] [--env k=v]…` — a STRUCTURED observation.
///
/// The same append as any other note, with one gate in front of it: the measure pin must be
/// declared in the registry, and an undeclared pin is REFUSED naming the registry path. That
/// refusal is the whole reason this arm exists rather than the caller writing a prose note that
/// happens to contain a number. The kit's thresholds were unaddressable constants; a fold minted
/// against an undeclared measure would be an unaddressable number — the same defect one layer up.
///
/// The subject IS the target: a measurement is *of* something, so there is no second `--on`.
/// `--kind` is accepted for symmetry with the prose arms but must be `observation`; a correction
/// or a verdict carrying a magnitude would be a record whose kind and content disagree.
#[allow(clippy::too_many_arguments)]
pub fn observe(
    root: &Path,
    kind: &str,
    measure_raw: &str,
    subject: &str,
    value: f64,
    unit: Option<&str>,
    env: &BTreeMap<String, String>,
    reason: Option<&str>,
    actor: &NoteActor,
    measures_path: &Path,
) -> FlowResult<NoteOutcome> {
    // Argument shape first, before the registry is read and long before the store is opened.
    let kind_parsed = NoteKind::parse(kind)?;
    if kind_parsed != NoteKind::Observation {
        return Err(FlowError::InvalidArguments(format!(
            "--measure belongs to --kind observation alone; got `{}` — \
             a magnitude on any other kind is a record whose tag and body disagree",
            kind_parsed.tag()
        )));
    }
    if !value.is_finite() {
        return Err(FlowError::InvalidArguments(
            "--value must be a finite number — NaN and infinity are not measurements".into(),
        ));
    }
    let measure = MeasureRef::parse(measure_raw, measures_path)?;

    let registry = Registry::open(measures_path)?;
    if !registry.declares(&measure) {
        let pins = registry.measure_pins();
        // A sample, not the whole registry: the refusal's job is to name WHERE the legal set lives
        // so the caller can read it, and a thirty-row wall of ids buries that path in its own
        // output. The path is the durable answer; the sample is only orientation.
        let shown: Vec<String> = pins.iter().take(6).map(ToString::to_string).collect();
        let declared = if pins.is_empty() {
            "(the registry declares no measures)".to_string()
        } else if pins.len() > shown.len() {
            format!(
                "e.g. {} … and {} more",
                shown.join(", "),
                pins.len() - shown.len()
            )
        } else {
            shown.join(", ")
        };
        return Err(FlowError::InvalidArguments(format!(
            "unknown measure `{measure}` — it is not declared in {}. Declared: {declared}",
            measures_path.display(),
        )));
    }

    // A caller-supplied unit wins; otherwise the registry's declared unit is adopted so a fold
    // carries the unit its measure was declared in without every call site restating it.
    let unit = unit
        .map(|u| non_empty(u, "--unit").map(str::to_string))
        .transpose()?
        .or_else(|| registry.declared_unit(&measure));

    let observation = Observation {
        measure,
        value,
        unit,
        env: env.clone(),
    };
    let reason = match reason {
        Some(text) => non_empty(text, "--reason")?.to_string(),
        None => observation.derived_reason(),
    };

    note_with_options_guard(
        root,
        subject,
        kind_parsed,
        &reason,
        None,
        None,
        None,
        actor,
        &super::acceptance::AcceptanceOptions::default(),
        Some(&observation),
        |_| Ok(()),
    )
}

/// A local composition may constrain the resolved attribution, but it cannot replace it.
/// The guard and event/outcome consume one snapshot, retaining its exact actor-claim CID.
#[allow(clippy::too_many_arguments)]
fn note_with_options_guard(
    root: &Path,
    on: &str,
    kind: NoteKind,
    reason: &str,
    switched_to: Option<&str>,
    closes: Option<&str>,
    verdict: Option<&str>,
    actor: &NoteActor,
    options: &super::acceptance::AcceptanceOptions,
    observation: Option<&Observation>,
    guard: impl FnOnce(&Attribution) -> FlowResult<()>,
) -> FlowResult<NoteOutcome> {
    // ── Phase 1: resolve. Nothing below this line touches the sidecar until Phase 2. ──

    // Argument shape first, so a malformed invocation never even opens the store. The KIND is
    // already parsed by the caller: every public entry point resolves it through the closed
    // `NoteKind::parse`, and the one act whose kind is not typable (`retraction`) resolves it
    // after its own eligibility check. Taking the enum here rather than a string is what makes
    // "unreachable from `--kind`" a property of the type rather than of a spelling.
    let reason = non_empty(reason, "--reason")?;
    let switched_to = switched_to
        .map(|s| non_empty(s, "--switched-to"))
        .transpose()?;
    let verdict = resolve_verdict(kind, verdict)?;
    let closes = resolve_closes(kind, closes)?;
    let named = named_identity(actor.as_ref.as_deref())?;

    let mut store = SidecarFlowStore::open(root)?.transaction()?;
    let records = store.records()?;
    // The repository-root subject is admitted on the STRUCTURED arm only. Scoping it to
    // `observation.is_some()` is what keeps every prose note's address exactly where it is: a
    // `.`-targeted prose note has always been an `UnknownResource` refusal, and quietly making it
    // resolve would mint records at an address the sidecar's history never used. A repository-wide
    // MEASUREMENT is a different thing — it has no file to name, and `repo_scope_atom` is the
    // resource the flow plane already scopes every commitment to.
    let (resource, label) = if observation.is_some() && is_repo_root(root, on) {
        (repo_scope_atom()?, REPO_SUBJECT.to_string())
    } else {
        resolve_target(root, on, &records)?
    };

    // Provenance and clock come from the same single `git log -1`, and an unattributable note is
    // refused rather than dated with a placeholder (module doc).
    let (author, occurred_at) = head_commit_provenance(root).ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "cannot date a note in `{}`: git has no HEAD commit to author it against — \
             a note is dated by the tree it was written against, never by wall clock",
            root.display()
        ))
    })?;

    let mut attribution = resolve_attribution(root, named, actor.session.as_deref(), &author);
    if options.purpose.as_deref() == Some("acceptance") {
        let (pin, identity) = actor
            .session
            .as_deref()
            .and_then(|session| claimed_for_session(root, session))
            .ok_or_else(|| {
                FlowError::InvalidArguments(
                    "acceptance requires a registered --session actor claim".into(),
                )
            })?;
        if actor
            .as_ref
            .as_deref()
            .is_some_and(|named| named.trim() != identity)
        {
            return Err(FlowError::InvalidArguments(
                "acceptance --as differs from session actor claim".into(),
            ));
        }
        attribution = Attribution {
            claim_cid: Some(pin),
            ..Attribution::claimed(identity, &author)
        };
    }

    guard(&attribution)?;

    // Tag first, subject second, authored body after, verdict next, steward last — see
    // `note_slots`, `FlowEvent::classified_as` and `STEWARD_SLOT_PREFIX`.
    let mut classified_as = note_slots(
        kind,
        &label,
        reason,
        switched_to,
        closes.as_deref(),
        verdict.as_deref(),
        None,
    );
    // Additive: a note carrying no measure extends nothing here and keeps the address it always had.
    if let Some(observation) = observation {
        classified_as.extend(observation.slots());
    }
    classified_as.extend(options.slots(root, &records, &resource, kind.tag())?);
    attribution.append_slots(&mut classified_as);

    let event = FlowEvent {
        // `Cite`, never `Produce`: a note produces no resource and discharges no promise — it
        // REFERS TO one. `Produce` would make notes count as output in every fold, and `Dismiss`
        // already means regression on the a2o verdict path.
        action: ReaVerb::Cite,
        provider: AgentRef(attribution.provider(&author)),
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

    if options.appoint.is_some() || options.purpose.is_some() {
        super::acceptance::validate_record(root, &records, &event)?;
    }
    let record = FlowRecord::Event(event);
    let record_cid = record.cid()?;
    let appended = !records.iter().any(|(cid, _)| cid == &record_cid);

    let outcome = NoteOutcome {
        kind: kind.tag().to_string(),
        on: label,
        resource: resource.to_string(),
        reason: reason.to_string(),
        switched_to: switched_to.map(str::to_string),
        closes: closes.clone(),
        verdict: verdict.clone(),
        measure: observation.map(|o| o.measure.to_string()),
        value: observation.map(|o| canonical_number(o.value)),
        unit: observation.and_then(|o| o.unit.clone()),
        env: observation.map(|o| o.env.clone()).unwrap_or_default(),
        actor: attribution.actor.clone(),
        actor_claim: attribution.claim_cid.map(|cid| cid.to_string()),
        steward: attribution.steward.clone(),
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

/// Append one `run:retraction` note against an already-checked sidecar record.
///
/// The ONLY producer of [`NoteKind::Retraction`]. It performs no eligibility check of its own —
/// [`super::retract`] owns that, because the check needs the record set folded into slots and
/// this leg deliberately knows nothing about what a `DepEdge` is. Splitting it this way keeps the
/// note plane a plane: it appends what it is told to append, and the admissibility question lives
/// with the vocabulary it is about.
///
/// `on` is the retracted record's CID string, which [`resolve_target`] resolves through its
/// known-atom arm — so the note's `resource` IS the withdrawn record's address, and a reader
/// joining the two needs no heuristic.
pub(crate) fn retraction(
    root: &Path,
    on: &str,
    reason: &str,
    actor: &NoteActor,
) -> FlowResult<NoteOutcome> {
    note_with_options_guard(
        root,
        on,
        NoteKind::Retraction,
        reason,
        None,
        None,
        None,
        actor,
        &super::acceptance::AcceptanceOptions::default(),
        None,
        |_| Ok(()),
    )
}

/// Native local act policy: require a registered session snapshot and optionally
/// match an authored identity. This is claimed attribution, not authentication.
pub(crate) fn note_for_session(
    root: &Path,
    on: &str,
    kind: &str,
    reason: &str,
    session: &str,
    expected_author: Option<&str>,
) -> FlowResult<NoteOutcome> {
    note_with_options_guard(
        root,
        on,
        NoteKind::parse(kind)?,
        reason,
        None,
        None,
        None,
        &NoteActor {
            as_ref: None,
            session: Some(session.into()),
        },
        &super::acceptance::AcceptanceOptions::default(),
        None,
        |attribution| {
            if attribution.claim_cid.is_none() {
                return Err(FlowError::InvalidArguments(
                    "session has no registered actor claim".into(),
                ));
            }
            if expected_author
                .is_some_and(|expected| attribution.actor.as_deref() != Some(expected))
            {
                return Err(FlowError::InvalidArguments(
                    "author differs from registered session claim".into(),
                ));
            }
            Ok(())
        },
    )
}

/// Reject a blank flag value, naming the flag. An empty `--reason` is the one refusal this leg
/// makes before it opens the store at all: a note with nothing in it is not an observation.
pub(crate) fn non_empty<'a>(value: &'a str, flag: &str) -> FlowResult<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FlowError::InvalidArguments(format!(
            "{flag} needs text — an empty note records nothing"
        )));
    }
    Ok(trimmed)
}

/// The resolved answer to "whose act is this note, and whose tree was it written in".
pub(crate) struct Attribution {
    /// The exact existing claim consulted at authoring; direct attribution has no claim pin.
    claim_cid: Option<Cid>,
    /// The claimed identity the note is attributed to; `None` leaves it with the commit author.
    pub(crate) actor: Option<String>,
    /// The git-signing human, carried on the agent arms only.
    pub(crate) steward: Option<String>,
}

impl Attribution {
    /// Keep descriptive slots stable and the steward last across all authoring verbs.
    pub(crate) fn append_slots(&self, slots: &mut Vec<String>) {
        if let Some(cid) = self.claim_cid {
            slots.push(format!("actor-claim:{cid}"));
        }
        if let Some(steward) = &self.steward {
            slots.push(format!("{STEWARD_SLOT_PREFIX}{steward}"));
        }
    }

    /// The `provider` slot: the claimed identity when there is one, the commit author otherwise.
    /// One place decides this, so the record and the outcome can never disagree about who acted.
    pub(crate) fn provider(&self, author: &str) -> String {
        self.actor.clone().unwrap_or_else(|| author.to_string())
    }

    /// The unattributed arm — the leg's original behaviour, named so the three arms read as three.
    fn authored() -> Self {
        Self {
            claim_cid: None,
            actor: None,
            steward: None,
        }
    }

    /// An agent arm: the identity provides, and the tree's signer is kept beside it. The two are
    /// set together because they are the same claim — "this agent acted, in this human's tree" —
    /// and an agent-provided note with no steward is precisely the deniable shape.
    fn claimed(identity: String, author: &str) -> Self {
        Self {
            claim_cid: None,
            actor: Some(identity),
            steward: Some(author.to_string()),
        }
    }
}

/// Validate a `--as` value without resolving anything else.
///
/// Separated from [`resolve_attribution`] because it is the one attribution failure that REFUSES,
/// and it must refuse at the same early gate as `--kind` and `--reason` — before the store is
/// opened. The refusal is `elohim-epr-rea`'s own, verbatim: one parser owns the shape, so the CLI
/// cannot accept an identity the store would reject, nor the reverse.
pub(crate) fn named_identity(as_ref: Option<&str>) -> FlowResult<Option<String>> {
    match as_ref {
        Some(raw) => {
            let trimmed = non_empty(raw, "--as")?;
            parse_agent_ref(trimmed)?;
            Ok(Some(trimmed.to_string()))
        }
        None => Ok(None),
    }
}

/// The three arms, in priority order: a named identity, then a session's registered claim, then
/// the commit author.
///
/// Every failure on the session arm — no sidecar, no claim, an unreadable or tampered log — lands
/// on the author arm with a notice on stderr rather than an error. The note is the durable thing
/// here; the attribution is an enrichment of it, and an enrichment that can veto its subject is a
/// dependency in the wrong direction.
pub(crate) fn resolve_attribution(
    root: &Path,
    named: Option<String>,
    session: Option<&str>,
    author: &str,
) -> Attribution {
    match (named, session) {
        (Some(identity), _) => Attribution::claimed(identity, author),
        (None, Some(session)) => match claimed_for_session(root, session) {
            Some((cid, identity)) => Attribution {
                claim_cid: Some(cid),
                ..Attribution::claimed(identity, author)
            },
            None => Attribution::authored(),
        },
        (None, None) => Attribution::authored(),
    }
}

/// Who registered for `session`, or `None` with one line on stderr saying why.
///
/// The sidecar is only opened once its log is known to exist: [`SidecarActorStore::open`] creates
/// the tree, and this leg must be able to ask the question in a repository that has never had an
/// identity plane without leaving one behind.
///
/// The notice names the session on purpose. "Not attributed to an agent" is indistinguishable
/// from "attributed to the wrong one" in the record itself, so the only place a caller can learn
/// that its session was not found is at the moment it was not found.
fn claimed_for_session(root: &Path, session: &str) -> Option<(Cid, String)> {
    if !root.join(ACTOR_LOG_REL).exists() {
        eprintln!(
            "note: session `{session}` has no actor sidecar — \
             the note stays attributed to the commit author"
        );
        return None;
    }
    match SidecarActorStore::open(root).and_then(|store| store.current_for(session)) {
        Ok(Some((cid, claim))) => Some((cid, claim.claimed.0)),
        Ok(None) => {
            eprintln!(
                "note: session `{session}` registered no actor claim — \
                 the note stays attributed to the commit author"
            );
            None
        }
        Err(error) => {
            eprintln!(
                "note: session `{session}` could not be read from the actor sidecar ({error}) — \
                 the note stays attributed to the commit author"
            );
            None
        }
    }
}

/// Resolve `--on` to `(resource CID, human label)`.
///
/// THREE admissible shapes, the same three `claim` and `fulfill` accept, tried in order of
/// decreasing precision. A **CIDv1 string**, which must already be an atom this sidecar knows. A
/// **gap id**, which resolves to the newest commitment carrying it and, failing that, to the
/// intent that raised it — so a note about a task lands on the promise, not on the whole plan.
/// A **repo-relative path**, whose canonical body CID is computed the way every other resource
/// identity on this path is. All three refuse with [`FlowError::UnknownResource`] rather than
/// minting an orphan — a note pointing at nothing is worse than no note, because it reads as
/// evidence.
///
/// The gap-id arm exists because it was MISSING: `claim` and `fulfill` learned it and `note` did
/// not, so the one verb the three non-discharging statuses are told to use could not name the
/// item it was blocked on. A vocabulary that refuses the record it recommends is not a
/// vocabulary.
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

    // A gap id: the newest commitment carrying it — the live promise — else the intent that
    // raised it, which is all that exists before anyone claims it. "Newest" is sidecar append
    // order, the same rule `claim`'s gap-id arm holds, so a superseded claim never wins.
    if let Some(cid) = gap_id_target(on, records) {
        return Ok((cid, on.to_string()));
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
    let cid = body_cid_of_file(&confined).ok_or_else(|| {
        FlowError::UnknownResource(format!(
            "{on} (not an atom address this sidecar knows, not a gap id any commitment or \
             intent carries, and not a readable path)"
        ))
    })?;
    Ok((cid, rel_to_root(&canonical_root, &confined)))
}

/// The address a gap id names: the newest commitment carrying it in `classified_as`, else the
/// intent whose slot-1 subject equals it.
///
/// Commitments are preferred over intents on purpose. A note about a task in flight is about the
/// PROMISE — that is where a verdict, a correction and a blocked observation all belong, and it
/// is what makes them roll up onto the plan's screen with a `via` marker instead of appearing as
/// unattributed remarks on the document.
fn gap_id_target(on: &str, records: &[(Cid, FlowRecord)]) -> Option<Cid> {
    records
        .iter()
        .rev()
        .find_map(|(cid, record)| match record {
            FlowRecord::Commitment(c) if c.resource_spec.classified_as.iter().any(|s| s == on) => {
                Some(*cid)
            }
            _ => None,
        })
        .or_else(|| {
            records.iter().rev().find_map(|(cid, record)| match record {
                FlowRecord::Intent(i)
                    if i.resource_spec.classified_as.get(1).map(String::as_str) == Some(on) =>
                {
                    Some(*cid)
                }
                _ => None,
            })
        })
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

    #[test]
    fn a_malformed_identity_is_refused_rather_than_replaced_by_the_author() {
        // The refusal is `elohim-epr-rea`'s, so the CLI cannot accept a shape the store would
        // reject. What matters here is that it refuses AT ALL: falling back to the git author
        // would mint a record asserting that a different actor spoke.
        let err = named_identity(Some("scribe@opus-5"))
            .expect_err("a bare role@model is not a claimed identity");
        assert!(
            err.to_string().contains("agent:<role>@<model>"),
            "the refusal must name the legal shape; got: {err}"
        );
        assert!(named_identity(Some("   ")).is_err(), "--as needs a value");
        assert_eq!(
            named_identity(Some(" agent:rust-architect@opus-5 ")).unwrap(),
            Some("agent:rust-architect@opus-5".to_string())
        );
        assert_eq!(named_identity(None).unwrap(), None);
    }

    #[test]
    fn the_author_arm_carries_no_steward_and_the_agent_arms_always_do() {
        let authored = Attribution::authored();
        assert_eq!(
            authored.provider("author@example.test"),
            "author@example.test"
        );
        assert!(
            authored.steward.is_none(),
            "a steward slot on the author arm would only repeat the provider"
        );

        let claimed = Attribution::claimed("agent:scribe@opus-5".into(), "author@example.test");
        assert_eq!(
            claimed.provider("author@example.test"),
            "agent:scribe@opus-5"
        );
        assert_eq!(claimed.steward.as_deref(), Some("author@example.test"));
    }

    #[test]
    fn ruling_and_verdict_join_the_closed_vocabulary_with_distinct_run_tags() {
        assert_eq!(NoteKind::parse("ruling").unwrap(), NoteKind::Ruling);
        assert_eq!(NoteKind::parse(" verdict ").unwrap(), NoteKind::Verdict);
        assert_eq!(NoteKind::Ruling.tag(), "run:ruling");
        assert_eq!(NoteKind::Verdict.tag(), "run:verdict");
        let err = NoteKind::parse("adjudication").expect_err("the vocabulary stays closed");
        assert!(
            err.to_string().contains("ruling") && err.to_string().contains("verdict"),
            "the refusal must name the whole legal set; got: {err}"
        );
    }

    #[test]
    fn a_verdict_kind_requires_the_flag_and_the_flag_requires_the_verdict_kind() {
        // The two accepted values, and nothing else.
        assert_eq!(
            resolve_verdict(NoteKind::Verdict, Some(" approved ")).unwrap(),
            Some("approved".to_string())
        );
        assert_eq!(
            resolve_verdict(NoteKind::Verdict, Some("changes-requested")).unwrap(),
            Some("changes-requested".to_string())
        );
        let bad = resolve_verdict(NoteKind::Verdict, Some("lgtm"))
            .expect_err("a third verdict value is refused, never defaulted");
        assert!(
            bad.to_string().contains("approved") && bad.to_string().contains("changes-requested"),
            "the refusal must name both legal values; got: {bad}"
        );

        // `--kind verdict` with no `--verdict` is refused, naming the flag and the kind.
        let missing = resolve_verdict(NoteKind::Verdict, None)
            .expect_err("an audit outcome with no outcome is not a verdict");
        assert!(missing.to_string().contains("--verdict"));
        assert!(missing.to_string().contains("verdict"));

        // `--verdict` on any other kind is refused rather than silently carried.
        for kind in [
            NoteKind::FailedApproach,
            NoteKind::Correction,
            NoteKind::Observation,
            NoteKind::Ruling,
        ] {
            let err = resolve_verdict(kind, Some("approved"))
                .expect_err("--verdict belongs to --kind verdict alone");
            assert!(err.to_string().contains("--verdict"));
            assert!(
                err.to_string().contains(kind.tag()),
                "the refusal must name the kind it was given; got: {err}"
            );
            assert_eq!(resolve_verdict(kind, None).unwrap(), None);
        }
    }

    #[test]
    fn the_verdict_slot_is_positional_and_additive_after_reason_before_steward() {
        let slots = note_slots(
            NoteKind::Verdict,
            "genesis/plan.md",
            "the gate line is present and the diff conforms",
            Some("a different approach"),
            None,
            Some("approved"),
            Some("author@example.test"),
        );
        assert_eq!(
            slots,
            vec![
                "run:verdict".to_string(),
                "genesis/plan.md".to_string(),
                "reason:the gate line is present and the diff conforms".to_string(),
                "switched-to:a different approach".to_string(),
                "verdict:approved".to_string(),
                "steward:author@example.test".to_string(),
            ]
        );
    }

    /// The closure slot sits between `switched-to:` and `verdict:`, and a note that closes nothing
    /// emits no slot for it at all — which is what keeps every existing note's address where it is.
    #[test]
    fn the_closes_slot_is_positional_and_additive_between_switched_to_and_verdict() {
        assert_eq!(
            note_slots(
                NoteKind::Verdict,
                "genesis/plan.md",
                "the repair conforms",
                Some("a different approach"),
                Some("bafyreiabc"),
                Some("approved"),
                Some("author@example.test"),
            ),
            vec![
                "run:verdict".to_string(),
                "genesis/plan.md".to_string(),
                "reason:the repair conforms".to_string(),
                "switched-to:a different approach".to_string(),
                "closes:bafyreiabc".to_string(),
                "verdict:approved".to_string(),
                "steward:author@example.test".to_string(),
            ]
        );
    }

    /// `--closes` is refused on the one kind that cannot discharge anything, and refused outright
    /// when its target is not an address a reader can verify.
    #[test]
    fn a_closure_target_must_be_a_cid_and_never_a_failed_approach() {
        let cid = super::super::body_cid("some correction").to_string();
        assert_eq!(
            resolve_closes(NoteKind::Observation, Some(&cid)).unwrap(),
            Some(cid.clone())
        );
        assert_eq!(resolve_closes(NoteKind::Correction, None).unwrap(), None);
        assert!(resolve_closes(NoteKind::FailedApproach, Some(&cid)).is_err());
        assert!(resolve_closes(NoteKind::Observation, Some("2026-09-10")).is_err());
        assert!(resolve_closes(NoteKind::Observation, Some("  ")).is_err());
    }

    /// The additive discipline, pinned: with no verdict the slot vector is BYTE-IDENTICAL to
    /// the one the pre-verdict encoder emitted, so every existing note keeps its address.
    #[test]
    fn a_note_without_a_verdict_keeps_its_pre_verdict_slots() {
        assert_eq!(
            note_slots(
                NoteKind::FailedApproach,
                "genesis/plan.md",
                "Tried Tsit5, the system is too stiff",
                Some("Kvaerno5"),
                None,
                None,
                None,
            ),
            vec![
                "run:failed-approach".to_string(),
                "genesis/plan.md".to_string(),
                "reason:Tried Tsit5, the system is too stiff".to_string(),
                "switched-to:Kvaerno5".to_string(),
            ]
        );
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
    #[test]
    fn guarded_note_emits_the_validated_actor_snapshot_after_session_switch() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("assertion.md"), "A qualified assertion").unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["add", "assertion.md"],
            vec!["commit", "-qm", "fixture"],
        ] {
            let result = crate::process::build_command("git", &args, root, &[])
                .env("GIT_AUTHOR_NAME", "Fixture")
                .env("GIT_COMMITTER_NAME", "Fixture")
                .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
                .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
                .output()
                .unwrap();
            assert!(result.status.success());
        }
        crate::actor::claim(root, "agent:investigator@fixture", "switching").unwrap();
        let original = SidecarActorStore::open(root)
            .unwrap()
            .current_for("switching")
            .unwrap()
            .unwrap();
        let outcome = note_with_options_guard(
            root,
            "assertion.md",
            NoteKind::Observation,
            "Validated before switch",
            None,
            None,
            None,
            &NoteActor {
                as_ref: None,
                session: Some("switching".into()),
            },
            &super::super::acceptance::AcceptanceOptions::default(),
            None,
            |resolved| {
                assert_eq!(
                    resolved.actor.as_deref(),
                    Some("agent:investigator@fixture")
                );
                assert_eq!(resolved.claim_cid, Some(original.0));
                // Deterministic interleaving at the old race: another persona registers
                // after validation but before event construction and append.
                crate::actor::claim(root, "agent:reviewer@fixture", "switching").unwrap();
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(outcome.actor.as_deref(), Some("agent:investigator@fixture"));
        assert_eq!(outcome.actor_claim, Some(original.0.to_string()));
        let records = SidecarFlowStore::open(root).unwrap().records().unwrap();
        let FlowRecord::Event(event) = &records[0].1 else {
            panic!("event expected")
        };
        assert_eq!(event.provider.0, "agent:investigator@fixture");
        assert!(event
            .classified_as
            .contains(&format!("actor-claim:{}", original.0)));
        assert!(note_for_session(
            root,
            "assertion.md",
            "observation",
            "Wrong expected persona",
            "switching",
            Some("agent:investigator@fixture")
        )
        .is_err());
        assert_eq!(
            SidecarFlowStore::open(root)
                .unwrap()
                .records()
                .unwrap()
                .len(),
            1
        );
    }
}
