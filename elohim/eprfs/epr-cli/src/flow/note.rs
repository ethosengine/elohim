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
pub(crate) const NOTE_UNIT: &str = "run-note";

/// Prefix on the `classified_as` slot carrying the authored body, so a reader can tell an
/// authored string from a tag or a subject at a glance (slots 0 and 1 are the established
/// tag-then-subject convention; everything after them is this leg's, and is prefixed).
pub(crate) const REASON_SLOT_PREFIX: &str = "reason:";

/// Prefix on the optional consequence slot — the second half of a failed approach.
pub(crate) const SWITCHED_TO_SLOT_PREFIX: &str = "switched-to:";

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
        }
    }
}

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

/// The positional `classified_as` slot vector for one note.
///
/// One place builds it, so the record and every reader share a single definition of the order:
/// tag, subject, `reason:`, optional `switched-to:`, optional `verdict:`, optional `steward:`
/// LAST. The order is ADDITIVE — a note that carries no verdict emits the same vector it always
/// did, which is what keeps every existing note's content address where it is.
fn note_slots(
    kind: NoteKind,
    subject: &str,
    reason: &str,
    switched_to: Option<&str>,
    verdict: Option<&str>,
    steward: Option<&str>,
) -> Vec<String> {
    let mut slots = Vec::with_capacity(
        3 + usize::from(switched_to.is_some())
            + usize::from(verdict.is_some())
            + usize::from(steward.is_some()),
    );
    slots.push(kind.tag().to_string());
    slots.push(subject.to_string());
    slots.push(format!("{REASON_SLOT_PREFIX}{reason}"));
    if let Some(switched) = switched_to {
        slots.push(format!("{SWITCHED_TO_SLOT_PREFIX}{switched}"));
    }
    if let Some(value) = verdict {
        slots.push(format!("{VERDICT_SLOT_PREFIX}{value}"));
    }
    if let Some(steward) = steward {
        slots.push(format!("{STEWARD_SLOT_PREFIX}{steward}"));
    }
    slots
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
    /// The git-signing human answerable for the tree, carried only when `actor` is present. On
    /// the author-attributed arm it would merely repeat the provider, and a field that sometimes
    /// restates another is a field readers learn to ignore.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward: Option<String>,
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
        if let Some(verdict) = &self.verdict {
            println!("        verdict: {verdict}");
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
    // ── Phase 1: resolve. Nothing below this line touches the sidecar until Phase 2. ──

    // Argument shape first, so a malformed invocation never even opens the store.
    let kind = NoteKind::parse(kind)?;
    let reason = non_empty(reason, "--reason")?;
    let switched_to = switched_to
        .map(|s| non_empty(s, "--switched-to"))
        .transpose()?;
    let verdict = resolve_verdict(kind, verdict)?;
    let named = named_identity(actor.as_ref.as_deref())?;

    let mut store = SidecarFlowStore::open(root)?.transaction()?;
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

    let attribution = resolve_attribution(root, named, actor.session.as_deref(), &author);

    // Tag first, subject second, authored body after, verdict next, steward last — see
    // `note_slots`, `FlowEvent::classified_as` and `STEWARD_SLOT_PREFIX`.
    let mut classified_as = note_slots(kind, &label, reason, switched_to, verdict.as_deref(), None);
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

    let record = FlowRecord::Event(event);
    let record_cid = record.cid()?;
    let appended = !records.iter().any(|(cid, _)| cid == &record_cid);

    let outcome = NoteOutcome {
        kind: kind.tag().to_string(),
        on: label,
        resource: resource.to_string(),
        reason: reason.to_string(),
        switched_to: switched_to.map(str::to_string),
        verdict: verdict.clone(),
        actor: attribution.actor.clone(),
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
}
