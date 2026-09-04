//! `epr flow claim` — an actor TAKES one intent (spec §5.1,
//! `genesis/docs/superpowers/specs/2026-09-05-valueflow-authoring-surface-design.md`).
//!
//! The projection already derives one `Intent` per gap item, and `fulfill` already discharges
//! a2o scenario commitments from a sprint report. Between those two there was nothing: no verb
//! by which a named actor took a task, so task-level work had no plan-level record and
//! therefore no drain path. This is that verb.
//!
//! **The claim is the promise, not the work.** It mints exactly one `Commitment` in
//! `CommitmentState::Active` whose `satisfies` names the intent, whose `provider` is the
//! resolved actor, and whose `in_scope_of` is COPIED from the intent so `walk`'s scoped-intents
//! contract does not move. The brief, when one is given, is carried as the content address of
//! the brief document rather than as its text: the brief IS the claim, and an address is the
//! only reference that stays true when the file is edited afterwards.
//!
//! **Never guess which task an author meant.** `--on` resolves in three shapes — an intent's
//! content address, a gap id, or a repository path — and the path arm REFUSES when it resolves
//! to more than one intent, naming every candidate. A claim minted against the wrong intent is
//! a promise made in someone else's name.
//!
//! **A duplicate is a refusal, not a second promise.** An active, undischarged commitment
//! already satisfying the intent stops the claim and names the incumbent — including the
//! `tool:decompose-claim` commitments the Python decompose step still mints for items in the
//! CLAIMED state. `--supersede` mints anyway and reports what it superseded, so taking a task
//! from another actor is possible but never silent.
//!
//! Identity is the atom address, as everywhere on this path: re-running the SAME claim against
//! the SAME tree appends nothing and reports `appended: false`, and that check runs BEFORE the
//! duplicate refusal so an idempotent re-run is never mistaken for a second claimant.

use std::path::{Path, PathBuf};

use cid::Cid;
use elohim_epr_rea::{
    AgentRef, Commitment, CommitmentState, FlowRecord, FlowStore, Intent, ReaVerb, ResourceSpec,
    SidecarFlowStore,
};
use serde::Serialize;

use super::note::{named_identity, non_empty, resolve_attribution, NoteActor, STEWARD_SLOT_PREFIX};
use super::registers;
use super::{
    body_cid_of_file, confine_under, head_commit_provenance, rel_to_root, repo_agent, short_cid,
    FlowError, FlowResult,
};

/// Slot-0 tag on a claimed commitment — the same `gap:<state>` vocabulary the projection uses
/// for gap items, so a claim reads as the same kind of thing the decompose step mints.
const CLAIM_TAG: &str = "gap:claimed";

/// Prefix on the optional slot carrying the brief's canonical body address.
const BRIEF_SLOT_PREFIX: &str = "brief:";

/// Prefix on the optional slot naming the habit this work is accounted to.
///
/// A habit is a SCOPE in REA terms, but `in_scope_of` stays the plan document on purpose (spec
/// §3): moving it would silently re-home every scoped-intents read. The habit is carried as a
/// classification slot instead, which is additive and reads back exactly as well.
const HABIT_SLOT_PREFIX: &str = "habit:";

/// What the caller asked for, already split out of argv.
pub struct ClaimRequest<'a> {
    pub on: &'a str,
    pub brief: Option<&'a str>,
    pub serves: Option<&'a str>,
    pub supersede: bool,
    pub actor: &'a NoteActor,
}

/// The machine-facing result of one `claim` act (`--json` consumers read this).
#[derive(Debug, Serialize)]
pub struct ClaimOutcome {
    /// The intent this commitment satisfies.
    pub intent: String,
    /// The intent's own subject slot — the gap id, when it has one.
    pub gap_id: String,
    pub commitment_cid: String,
    /// Who promised: the claimed identity when one was named, else the commit author.
    pub provider: String,
    /// The git-signing human answerable for the tree, carried only on the agent arms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steward: Option<String>,
    /// The brief's canonical body address, when `--brief` named one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brief: Option<String>,
    /// The habit this claim is accounted to, when `--serves` named one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub habit: Option<String>,
    /// Git HEAD's author date — the tree the promise was made against.
    pub valid_from: String,
    /// `false` when this exact commitment was already in the sidecar and the append was a no-op.
    pub appended: bool,
    /// The incumbent commitment(s) this claim took the intent from, under `--supersede`. Empty
    /// on an ordinary claim; a duplicate without `--supersede` never gets this far.
    pub superseded_by: Vec<String>,
}

impl ClaimOutcome {
    pub fn render(&self) {
        println!(
            "claim   {} → {}  {}",
            self.provider,
            self.gap_id,
            short_cid_str(&self.commitment_cid)
        );
        println!("        intent: {}", short_cid_str(&self.intent));
        if let Some(brief) = &self.brief {
            println!("        brief: {}", short_cid_str(brief));
        }
        if let Some(habit) = &self.habit {
            println!("        serves habit: {habit}");
        }
        if let Some(steward) = &self.steward {
            println!("        steward: {steward}");
        }
        for superseded in &self.superseded_by {
            println!("        superseded: {}", short_cid_str(superseded));
        }
        if !self.appended {
            println!("        (already claimed on this tree — no-op)");
        }
    }
}

fn short_cid_str(cid: &str) -> String {
    match cid.parse::<Cid>() {
        Ok(parsed) => short_cid(&parsed),
        Err(_) => cid.to_string(),
    }
}

/// `epr flow claim --on <intent-cid|gap-id|path> [--as …] [--brief …] [--serves …]`.
///
/// Two phases, the idiom `note` and `reseal` already use: **Phase 1 resolves everything and
/// appends nothing**, **Phase 2 performs the single append**. Every refusal — an unresolvable
/// target, an ambiguous path, an unknown habit, an unreadable brief, a standing incumbent —
/// happens in Phase 1, so a refused claim leaves the sidecar byte-identical.
pub fn claim(root: &Path, request: &ClaimRequest) -> FlowResult<ClaimOutcome> {
    // ── Phase 1: resolve. Nothing below this line touches the sidecar until Phase 2. ──
    let on = non_empty(request.on, "--on")?;
    let named = named_identity(request.actor.as_ref.as_deref())?;

    // `--serves` is checked against the GENERATED register before anything else is resolved:
    // an unknown habit id is a typo in the accounting, and accounting work to a habit that
    // does not exist is worse than not accounting it at all.
    let habit = match request.serves {
        Some(raw) => {
            let id = non_empty(raw, "--serves")?;
            let habits = registers::read_habits(root)?;
            if !habits.iter().any(|habit| habit.id == id) {
                return Err(FlowError::InvalidArguments(format!(
                    "unknown habit `{id}` — the register {} declares no such id; \
                     a habit is DECLARED in its `.epr-meta` atom and projected there, \
                     never invented at claim time",
                    registers::HABITS_REGISTER_REL
                )));
            }
            Some(id.to_string())
        }
        None => None,
    };

    let mut store = SidecarFlowStore::open(root)?;
    let records = store.records()?;
    let (intent_cid, intent) = resolve_intent(root, on, &records)?;

    let brief = match request.brief {
        Some(raw) => {
            let path = non_empty(raw, "--brief")?;
            Some(brief_address(root, path)?)
        }
        None => None,
    };

    // Provenance and clock from one `git log -1`, exactly as `note` sources them: a promise is
    // dated by the tree it was made against, never by a wall clock.
    let (author, occurred_at) = head_commit_provenance(root).ok_or_else(|| {
        FlowError::InvalidArguments(format!(
            "cannot date a claim in `{}`: git has no HEAD commit to make the promise against",
            root.display()
        ))
    })?;
    let attribution = resolve_attribution(root, named, request.actor.session.as_deref(), &author);

    let gap_id = intent
        .resource_spec
        .classified_as
        .get(1)
        .cloned()
        .unwrap_or_else(|| short_cid(&intent_cid));

    let mut classified_as = Vec::with_capacity(4);
    classified_as.push(CLAIM_TAG.to_string());
    classified_as.push(gap_id.clone());
    if let Some(brief) = &brief {
        classified_as.push(format!("{BRIEF_SLOT_PREFIX}{brief}"));
    }
    if let Some(habit) = &habit {
        classified_as.push(format!("{HABIT_SLOT_PREFIX}{habit}"));
    }
    if let Some(steward) = &attribution.steward {
        classified_as.push(format!("{STEWARD_SLOT_PREFIX}{steward}"));
    }

    let commitment = Commitment {
        action: ReaVerb::Produce,
        provider: AgentRef(attribution.provider(&author)),
        receiver: repo_agent(),
        resource_spec: ResourceSpec {
            classified_as,
            quantity: None,
        },
        // COPIED from the intent, never re-derived: `walk` reads scoped intents and scoped
        // commitments through the same `in_scope_of`, so a claim that re-homed itself would
        // disappear from the very frontier that raised it.
        in_scope_of: intent.in_scope_of,
        valid_from: Some(occurred_at.clone()),
        valid_until: None,
        state: CommitmentState::Active,
        satisfies: vec![intent_cid],
        bound: None,
    };

    let record = FlowRecord::Commitment(commitment);
    let commitment_cid = record.cid()?;
    let appended = !records.iter().any(|(cid, _)| cid == &commitment_cid);

    // The idempotence check runs FIRST. Re-running the same claim against the same tree must
    // read as the no-op it is, never as a second claimant colliding with itself.
    let mut superseded_by = Vec::new();
    if appended {
        let incumbents = standing_claims(&records, &intent_cid);
        if !incumbents.is_empty() {
            if !request.supersede {
                let named: Vec<String> = incumbents
                    .iter()
                    .map(|(cid, provider)| format!("{provider} ({})", short_cid(cid)))
                    .collect();
                return Err(FlowError::InvalidArguments(format!(
                    "intent `{gap_id}` is already claimed by {} — pass --supersede to take it \
                     over, which records what it superseded",
                    named.join(", ")
                )));
            }
            superseded_by = incumbents.iter().map(|(cid, _)| cid.to_string()).collect();
        }
    }

    let outcome = ClaimOutcome {
        intent: intent_cid.to_string(),
        gap_id,
        commitment_cid: commitment_cid.to_string(),
        provider: attribution.provider(&author),
        steward: attribution.steward.clone(),
        brief: brief.map(|cid| cid.to_string()),
        habit,
        valid_from: occurred_at,
        appended,
        superseded_by,
    };

    // ── Phase 2: append. One record, or none. ──
    if appended {
        store.append(record)?;
    }
    Ok(outcome)
}

/// The brief's canonical body address, refusing an unreadable path rather than dropping the
/// slot: a claim whose brief silently vanished would read as a claim made without one.
fn brief_address(root: &Path, rel: &str) -> FlowResult<Cid> {
    let canonical_root = std::fs::canonicalize(root).map_err(|source| FlowError::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let abs = if Path::new(rel).is_absolute() {
        PathBuf::from(rel)
    } else {
        canonical_root.join(rel)
    };
    let confined = confine_under(&canonical_root, &abs)?;
    body_cid_of_file(&confined).ok_or_else(|| {
        FlowError::UnknownResource(format!("{rel} (named by --brief, but it cannot be read)"))
    })
}

/// Resolve `--on` to `(intent cid, intent)` in the spec's three shapes, in order.
///
/// The shapes are tried in order of decreasing precision — an address names exactly one thing,
/// a gap id names one item of one plan, a path names a whole document — and the least precise
/// shape is the only one that can be ambiguous, which is why it is the only one that refuses on
/// ambiguity rather than choosing.
fn resolve_intent(
    root: &Path,
    on: &str,
    records: &[(Cid, FlowRecord)],
) -> FlowResult<(Cid, Intent)> {
    // (1) A content address. It must be an Intent this sidecar actually holds: `atom_cid` will
    // happily parse an address for something never recorded, and claiming against it would
    // mint a promise unreachable from every walk that starts at a real record.
    if let Ok(cid) = on.parse::<Cid>() {
        for (record_cid, record) in records {
            if record_cid == &cid {
                return match record {
                    FlowRecord::Intent(intent) => Ok((cid, intent.clone())),
                    other => Err(FlowError::InvalidArguments(format!(
                        "{on} is a {} in this sidecar, not an Intent — a claim answers an intent",
                        record_kind(other)
                    ))),
                };
            }
        }
        return Err(FlowError::UnknownResource(format!(
            "{on} (a well-formed CID, but no record in this sidecar mints it)"
        )));
    }

    // (2) A gap id, matched against the intent's slot-1 subject. One gap item can carry more
    // than one intent over its life (its `gap:<state>` slot-0 tag moves as the item moves), so
    // the LAST one in sidecar append order — the most recent projection — is the live one.
    let by_gap_id = records.iter().rev().find_map(|(cid, record)| match record {
        FlowRecord::Intent(intent)
            if intent
                .resource_spec
                .classified_as
                .get(1)
                .map(String::as_str)
                == Some(on) =>
        {
            Some((*cid, intent.clone()))
        }
        _ => None,
    });
    if let Some(found) = by_gap_id {
        return Ok(found);
    }

    // (3) A repository path: its canonical body address, then the intents scoped by it.
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
    let scope = body_cid_of_file(&confined).ok_or_else(|| {
        FlowError::UnknownResource(format!(
            "{on} (not a known intent address, not a gap id, and not a readable path)"
        ))
    })?;
    let scoped: Vec<(Cid, &Intent)> = records
        .iter()
        .filter_map(|(cid, record)| match record {
            FlowRecord::Intent(intent) if intent.in_scope_of == scope => Some((*cid, intent)),
            _ => None,
        })
        .collect();
    match scoped.len() {
        0 => Err(FlowError::UnknownResource(format!(
            "{} scopes no intent — run `epr flow project` first, or name a gap id",
            rel_to_root(&canonical_root, &confined)
        ))),
        1 => Ok((scoped[0].0, scoped[0].1.clone())),
        _ => {
            let candidates: Vec<String> = scoped
                .iter()
                .map(|(cid, intent)| {
                    format!(
                        "{} ({})",
                        intent
                            .resource_spec
                            .classified_as
                            .get(1)
                            .cloned()
                            .unwrap_or_else(|| "?".to_string()),
                        short_cid(cid)
                    )
                })
                .collect();
            Err(FlowError::InvalidArguments(format!(
                "{} scopes {} intents — name the one you mean by its gap id or address: {}",
                rel_to_root(&canonical_root, &confined),
                candidates.len(),
                candidates.join(", ")
            )))
        }
    }
}

/// The active, undischarged commitments already satisfying `intent_cid`, with their providers.
///
/// "Undischarged" is the crate's one derivation of it — any event whose `fulfills` names the
/// commitment — so a claim, a fulfilment and a walk can never disagree about what is still open.
fn standing_claims(records: &[(Cid, FlowRecord)], intent_cid: &Cid) -> Vec<(Cid, String)> {
    let discharged: std::collections::HashSet<Cid> = records
        .iter()
        .filter_map(|(_, record)| match record {
            FlowRecord::Event(e) => Some(e.fulfills.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    records
        .iter()
        .filter_map(|(cid, record)| match record {
            FlowRecord::Commitment(c)
                if c.state == CommitmentState::Active
                    && c.satisfies.contains(intent_cid)
                    && !discharged.contains(cid) =>
            {
                Some((*cid, c.provider.0.clone()))
            }
            _ => None,
        })
        .collect()
}

fn record_kind(record: &FlowRecord) -> &'static str {
    match record {
        FlowRecord::Intent(_) => "Intent",
        FlowRecord::Commitment(_) => "Commitment",
        FlowRecord::Event(_) => "Event",
        FlowRecord::Process(_) => "Process",
        FlowRecord::Spec(_) => "Spec",
        FlowRecord::Edge(_) => "Edge",
    }
}
