//! **Rung 6.** The FETCH behind `VerifyInput.path` — Task 4 declared the
//! evidence and left every construction site passing `Answer::Absent`; this is
//! the site that reads it.
//!
//! # Why the fetch lives here and not in `verify`
//!
//! [`super::verify`] is pure by contract (its module docs: "this module does
//! no I/O"), which is what lets the whole floor be unit-tested against
//! fixtures with no conductor in the room. The evidence a floor check reads is
//! therefore always assembled by the caller. Every other floor input already
//! works this way — installed reality, the L2 lineage chain, the attestation
//! count — and the path is the fourth of the same shape, not a new one.
//!
//! # I1 / C5 — through THIS peer's own conductor, never a peer's word
//!
//! The commitment is read with `mishpat::get_commitment` over the peer's own
//! `HcClient` (the same bridge every other adoption read uses). A migration
//! path is authority, and authority is never taken from the party asking for
//! it: a manifest's `adoptionDiscipline.path` is a *claim* — a pointer at a
//! commitment CID — and this module turns that claim into evidence by looking
//! the commitment up locally. A release cannot ship its own permission.
//!
//! # C4 — Unreachable is never Absent
//!
//! Three outcomes, and the difference between the last two is the whole point:
//!
//! | outcome | meaning | what `verify_path` does |
//! |---|---|---|
//! | [`Answer::Present`] | the commitment is on this conductor's DHT view | checks it |
//! | [`Answer::Absent`] | the conductor answered, and answered "no such entry" | `path_not_notarized` |
//! | [`Answer::Unreachable`] | we could not ask, or could not read the answer | `conductor_unavailable` |
//!
//! A conductor bridge that is down must never read as "the elohim did not
//! notarize this path" — that would turn our own outage into a statement
//! about someone else's governance. So every failure to *ask* maps to
//! `Unreachable`, and only the conductor's own "not found" maps to `Absent`.
//!
//! # Where each field comes from
//!
//! Deliberately two sources, because they are two different facts:
//!
//! - **The commitment body** (`from_dna_hash`, `to_dna_hash`,
//!   `constitution_root`, `roster_cid`, `signatures`, `signers`,
//!   `required_signatures`) is read out of the DHT entry's `payload_json` —
//!   the notarized bytes themselves.
//! - **The roster** (`roster`) is a SECOND commitment, named by the body's
//!   `roster_cid` and fetched down the same C5 rail — see [`read_roster`].
//!
//! # What the roster check is, and is not
//!
//! The roster check is a coherence check against an author-supplied,
//! author-mintable electorate — mishpat integrity verifies no signatures and no
//! arm binds a roster to the elohim's key or root — so it raises forgery cost
//! from one commitment to two; it is not yet a trust boundary.
//! - **The lifecycle** (`state`, `revoked_at`) is DHT truth, read down the same
//!   C5 rail as the body: `mishpat::get_commitment_state_links` through this
//!   peer's own conductor. The local `mishpat_commitments` row is only a
//!   CACHE, consulted when the DHT carries no transition at all; no links and
//!   no row reads as `proposed` — fail-closed, because
//!   [`super::verify::verify_path`] establishes a path only on `"active"`.
//!
//! # Task 19 — why the lifecycle moved off the projection row
//!
//! The row is written from `CommitmentCommitted`, a post-commit signal on the
//! AUTHORING conductor, so only the peer whose elohim notarized a commitment
//! ever holds a row for it. That made two facts unreadable everywhere else:
//! activation (worked around by inferring it from the entry's own action) and,
//! far worse, REVOCATION — a `revokes-commitment` is a separate entry whose row
//! also only ever lands on its author, so an elohim could pull a migration path
//! back and every peer it was meant to stop would keep reading the path as
//! active. That is Station 7's revocation, and no amount of care on the reading
//! side could close it while the fact lived in a private index.
//!
//! The COORDINATOR side of it is closed
//! (`mishpat::commitments::author_lifecycle_link`, coordinator-only): a lineage
//! commitment records `active` on its own anchor when the validator accepts its
//! quorum, and a quorum-checked revocation records `revoked` on the anchor it
//! revokes. Both are `CommitmentByState` links on the DHT, so every peer reads
//! the same lifecycle through its own conductor rather than out of an index
//! only one peer holds. That is a move from a PRIVATE fact to a PUBLIC one —
//! it is not yet a move from a claim to a verified fact, and the next section
//! says exactly why.
//!
//! The entry-action inference this replaces is gone from the READ path. Reading
//! "active" off the fact that an entry exists made activation an inference any
//! author could produce; reading it off the link makes it an ACT the coordinator
//! only performs after the quorum check, and — unlike an inference — an act that
//! a revocation can undo. (The CACHE writer still infers from the entry action —
//! `mishpat_projection`'s own dispatch arm — and [`resolve_lifecycle`] honours
//! that row only when the links are silent.)
//!
//! # OPEN TRUST BOUNDARY, named rather than closed: gap G7
//!
//! **A `CommitmentByState` link is validated for tag SHAPE and nothing else.**
//! `mishpat_integrity::validate_create_link` checks only that the tag parses as
//! `"<state>|<signed_at>"` with both segments non-empty — it does not, and as an
//! HDI validator with no `get_links` it largely cannot cheaply, check WHO
//! authored the link or what the link's base actually is. And
//! `create_commitment_state_link` is a PUBLIC extern on the mishpat coordinator.
//!
//! So any agent on the DHT can author `revoked|<t>` on any lineage commitment's
//! anchor. This module reads it, [`lifecycle_from_links`] lets it win outright,
//! [`resolve_lifecycle`] unions it, and `verify_path` refuses `path_revoked` —
//! on every peer, permanently, because revocation is terminal and nothing
//! reopens it. **A single unauthorized link is a permanent, mesh-wide denial of
//! a migration path.** The same extern can author `active|<t>` on any anchor,
//! which the ladder reads as an activation the quorum never granted.
//!
//! This module therefore reads the links as EVIDENCE IT CANNOT YET VERIFY: they
//! are the whole mesh's word rather than one peer's private index, which is
//! strictly better than what came before, but the authorship of the statement is
//! unchecked. Do not read the paragraph above as "evidence, never a peer's
//! word" — until G7 closes, an unauthorized peer's word is exactly what a link
//! can be.
//!
//! **The fix is integrity-side and hash-moving**, which is why it is filed
//! rather than done here: `validate_create_link` must bind the link's AUTHOR to
//! the anchor —
//!
//! - `active` — only the agent who authored the Commitment at the link's base
//!   may declare it active (the coordinator already authors the link in the same
//!   call that created the entry, so the honest path already satisfies this);
//! - `revoked` — only the author of a quorum-checked `revokes-commitment` whose
//!   `target_cid` IS the link's base may declare it revoked.
//!
//! Both move the mishpat DNA hash, so G7 rides the sunset-hardening crossing
//! alongside G1 and G4 rather than landing on its own. Until it does, a path's
//! refusal is trustworthy (fail-closed is safe to over-trigger) and a path's
//! ACCEPTANCE still rests on the quorum and roster checks in `verify_path`,
//! which no link can forge.

use std::sync::Arc;

use seam_contracts::Answer;

use super::{ArtifactClass, PathEvidence, ReleaseManifest, RosterEvidence};
use crate::db::DbPool;
use crate::hc_client::HcClient;
use crate::services::conductor_writes::CommitmentStateLink;

/// The lifecycle state a commitment is read as when its projection row has not
/// landed (or could not be read). Fail-closed: `verify_path` establishes a path
/// only on `"active"`, so an unknown lifecycle refuses rather than adopts.
const UNPROJECTED_STATE: &str = "proposed";

/// The lifecycle state a `CommitmentByState` link records for an activation.
/// The one value [`super::verify::verify_path`] establishes a path on.
const ACTIVE_STATE: &str = "active";

/// The lifecycle state a quorum-checked revocation records ON THE ANCHOR IT
/// REVOKES. Terminal: nothing reopens a revoked path (epic Station 8).
const REVOKED_STATE: &str = "revoked";

/// The signature count a path's discipline requires when the commitment body
/// does not declare one. One is the floor, never zero — a `required_signatures`
/// of zero would make the quorum check in `verify_path` vacuous, which is the
/// one value a defaulting rule must not be able to produce.
pub(super) const DEFAULT_REQUIRED_SIGNATURES: usize = 1;

/// Fetch the evidence for a manifest's `adoptionDiscipline.path`.
///
/// Returns [`Answer::Absent`] without touching the conductor for any artifact
/// class but [`ArtifactClass::HappLineage`] — `verify_path` is a no-op for
/// those and never consults the value, so paying for a zome call would be
/// work with no reader.
pub async fn fetch_path_evidence(
    hc: Option<&Arc<HcClient>>,
    db: Option<&DbPool>,
    manifest: &ReleaseManifest,
) -> Answer<PathEvidence> {
    if manifest.artifact_class != ArtifactClass::HappLineage {
        return Answer::Absent;
    }
    // A `happ-lineage` manifest with no path is a schema violation, and
    // `verify_path` says so (`manifest_schema_invalid`) without needing
    // evidence. Absent is the honest input: there is no commitment named to
    // go and read.
    let Some(path) = manifest.adoption_discipline.path.as_ref() else {
        return Answer::Absent;
    };
    let cid = path.commitment_cid.as_str();

    // The path names a commitment that is not an ADDRESS. `mishpat::get_commitment`
    // would answer with a GUEST ERROR (`EntryHash::try_from`), which reads as
    // `conductor_unavailable` — our outage — when the honest finding is that the
    // MANIFEST named a commitment that cannot exist. Same rule as the roster's
    // (see [`read_roster`]), applied to the sibling read: an unaddressable cid is
    // an observed absence, `path_not_notarized`, and no gossip will change it.
    if !is_addressable_cid(cid) {
        tracing::debug!(
            commitment_cid = %cid,
            "release-adoption: manifest names a path commitment that is not an address — \
             absent, never an outage"
        );
        return Answer::Absent;
    }

    // No bridge at all — we could not ask. NEVER Absent (C4).
    let Some(hc) = hc else {
        tracing::debug!(
            commitment_cid = %cid,
            "release-adoption: no conductor bridge to read path evidence through — unreachable, \
             which establishes nothing about the commitment"
        );
        return Answer::Unreachable;
    };

    let out = match crate::services::conductor_writes::get_commitment(hc, cid).await {
        Ok(Some(out)) => out,
        // The conductor ANSWERED, and answered "not on my DHT view". That is
        // an observed absence, and `verify_path` reports it as
        // `path_not_notarized` — a refusal that self-heals the moment the
        // commitment gossips to this peer.
        Ok(None) => {
            tracing::debug!(
                commitment_cid = %cid,
                "release-adoption: path commitment is not on this conductor's DHT view yet"
            );
            return Answer::Absent;
        }
        Err(e) => {
            tracing::debug!(
                commitment_cid = %cid,
                error = %e,
                "release-adoption: path commitment unreadable — unreachable, never absence"
            );
            return Answer::Unreachable;
        }
    };

    let payload: serde_json::Value = match serde_json::from_str(&out.payload_json) {
        Ok(v) => v,
        // We reached the commitment but cannot read what it says. That is a
        // failure to READ, not an absence and not a mismatch — treating it as
        // either would put a fabricated fact in front of the floor.
        Err(e) => {
            tracing::warn!(
                commitment_cid = %cid,
                error = %e,
                "release-adoption: path commitment payload_json does not parse — unreadable"
            );
            return Answer::Unreachable;
        }
    };

    // **Task 16.** The roster the body NAMES, read through this peer's own
    // conductor — the second C5 read, and the one that decides whose
    // signature counts. An unreadable roster is `Unreachable` (never a pass,
    // never an off-roster acceptance); a roster the conductor answered "no
    // such entry" for is `None`, which `verify_path` refuses as
    // `quorum_unmet`.
    let roster_cid = string_field(&payload, "roster_cid");
    let roster = match read_roster(Some(hc), &roster_cid).await {
        Ok(roster) => roster,
        Err(()) => return Answer::Unreachable,
    };

    // ── The lifecycle, off the DHT (Task 19) ────────────────────────────────
    // The transitions this peer's own conductor can see on the commitment's
    // anchor. An unreadable read is `Unreachable` — never a state we made up.
    //
    // OPEN TRUST BOUNDARY, named rather than closed this round — GAP G7, and
    // the reason the module docs stop short of calling these links verified.
    // `mishpat_integrity::validate_create_link` validates a `CommitmentByState`
    // link's TAG SHAPE only (`"<state>|<signed_at>"`, both segments non-empty);
    // it binds neither the link's author nor its base. And
    // `create_commitment_state_link` is a PUBLIC coordinator extern. So any
    // agent on the DHT can author `revoked|<t>` on any lineage anchor — and
    // because [`lifecycle_from_links`] lets a revocation win outright,
    // [`resolve_lifecycle`] unions it, and `verify_path` checks `revoked_at`
    // first, ONE unauthorized link is a permanent, mesh-wide, unreopenable
    // denial of that migration path. The same extern can author `active|<t>` on
    // any anchor, which reads as an activation the quorum never granted.
    //
    // What is read below is therefore EVIDENCE THIS PEER CANNOT YET VERIFY:
    // public rather than private (the whole point of Task 19), but unauthored.
    // The seam that closes it is integrity-side and HASH-MOVING — bind the link
    // author to the anchor: `active` only from the agent who authored the
    // Commitment at the base, `revoked` only from the author of a quorum-checked
    // `revokes-commitment` whose `target_cid` is that base. Filed as gap G7,
    // riding the sunset-hardening crossing with G1 and G4.
    let links = match read_state_links(hc, cid).await {
        Ok(links) => links,
        Err(()) => return Answer::Unreachable,
    };
    // The projection row, still read — but only as a CACHE now, and still
    // subject to the same C4 line: a row we could not READ is unreachable.
    let Some(row) = lifecycle(db, cid).await else {
        return Answer::Unreachable;
    };
    let (state, revoked_at) = resolve_lifecycle(&links, row);
    tracing::debug!(
        commitment_cid = %cid,
        state = %state,
        revoked = revoked_at.is_some(),
        state_links = links.len(),
        "release-adoption: path lifecycle resolved from this peer's own DHT view"
    );

    Answer::Present(evidence_from(
        // The commitment's CID is its ENTRY hash, never its action hash —
        // returning the wrong one here would fail `verify_path`'s
        // commitment-identity check on every legitimate path.
        &format!("{}", out.entry_hash),
        &payload,
        state,
        revoked_at,
        roster,
    ))
}

/// The roster a path names, read through THIS peer's own conductor.
///
/// **Task 16 / epic §4.1.** A `migrates-lineage` body carries both the
/// signatures and the `roster_cid` they are supposed to be drawn from, and
/// until this read existed nothing anywhere compared the two: a commitment
/// notarized by a household peer whose key is on no roster at all was accepted
/// by every peer on the mesh (measured, Station 10, `cucumber-stations-mvp-r14`).
/// The signers are the body's own claim; the ROSTER is not — it is looked up
/// locally, exactly as the path commitment itself is, so a release still
/// cannot ship its own permission.
///
/// The roster check is a coherence check against an author-supplied,
/// author-mintable electorate — mishpat integrity verifies no signatures and no
/// arm binds a roster to the elohim's key or root — so it raises forgery cost
/// from one commitment to two; it is not yet a trust boundary.
///
/// Four outcomes, and the C4 line runs above the last one:
///
/// | outcome | meaning | what the caller does |
/// |---|---|---|
/// | `Ok(Read{..})` | the roster is on this conductor's DHT view | `verify_path` counts against it, and checks its root |
/// | `Ok(Unaddressable)` | the body named no roster, or one that is not an address | `quorum_unmet`, TERMINAL — no gossip resolves a non-address |
/// | `Ok(NotFound)` | the conductor answered "no such entry" | `quorum_unmet`, may still gossip here |
/// | `Err(())` | we could not ASK, or could not READ the answer | the whole evidence is `Unreachable` → `conductor_unavailable` |
///
/// The roster body is read for `members` and its own `constitution_root`. This
/// side deliberately does NOT re-verify the roster's predecessor CHAIN back to
/// that root (epic §4.1's full rule): that is the integrity-side arm, and it is
/// hash-moving on mishpat.
async fn read_roster(hc: Option<&Arc<HcClient>>, roster_cid: &str) -> Result<RosterEvidence, ()> {
    // A body naming no roster, or naming one that is not even an ADDRESS, names
    // no members. Neither is an outage — the commitment really does say nothing
    // this peer can go and look at — so both are an ANSWER, and `verify_path`
    // refuses `quorum_unmet` rather than passing an uncheckable quorum.
    //
    // The shape check is not decoration. `mishpat::get_commitment` does
    // `EntryHash::try_from(cid)` and returns a GUEST ERROR for anything that is
    // not a base64 entry hash, which arrives here as `Err` and would be read as
    // `conductor_unavailable` — our outage — when what actually happened is that
    // the notarized body named a roster that cannot exist. Checking the shape
    // locally keeps that a statement about the COMMITMENT, and costs no round
    // trip.
    if roster_cid.is_empty() || !is_addressable_cid(roster_cid) {
        tracing::debug!(
            roster_cid = %roster_cid,
            "release-adoption: path names no addressable roster — an absent roster, never an outage"
        );
        return Ok(RosterEvidence::Unaddressable);
    }
    // Unreachable by construction: `fetch_path_evidence` already returned on a
    // missing bridge before it got here, so this arm is belt-and-braces for any
    // future caller — and it must still be Err, never an answer.
    let Some(hc) = hc else {
        return Err(());
    };
    match crate::services::conductor_writes::get_commitment(hc, roster_cid).await {
        Ok(Some(out)) => match serde_json::from_str::<serde_json::Value>(&out.payload_json) {
            Ok(body) => Ok(RosterEvidence::Read {
                members: string_array_field(&body, "members"),
                // The root the ROSTER declares itself under. Empty or absent is
                // `None` — "this roster declares no root" — which `verify_path`
                // refuses against a path that DOES declare one: an electorate
                // that names no constitution is not this constitution's.
                constitution_root: Some(string_field(&body, "constitution_root"))
                    .filter(|root| !root.is_empty()),
            }),
            Err(e) => {
                tracing::warn!(
                    roster_cid = %roster_cid,
                    error = %e,
                    "release-adoption: roster commitment payload_json does not parse — unreadable, \
                     never an empty roster"
                );
                Err(())
            }
        },
        // The conductor ANSWERED: the roster is not on its DHT view. That is an
        // observed absence, and it refuses — self-healing the moment the roster
        // gossips to this peer.
        Ok(None) => {
            tracing::debug!(
                roster_cid = %roster_cid,
                "release-adoption: path roster is not on this conductor's DHT view yet"
            );
            Ok(RosterEvidence::NotFound)
        }
        Err(e) => {
            tracing::debug!(
                roster_cid = %roster_cid,
                error = %e,
                "release-adoption: path roster unreadable — unreachable, never absence"
            );
            Err(())
        }
    }
}

/// Build the evidence from a commitment body plus its projected lifecycle.
///
/// Pure, so the parsing rule is unit-testable against a payload fixture with
/// no conductor and no pool. A field the body omits becomes an empty string
/// rather than an error: `verify_path` then reports the precise crossing
/// mismatch (`path X names →, release is A→B`), which tells an operator far
/// more than "the payload was malformed" would.
pub fn evidence_from(
    commitment_cid: &str,
    payload: &serde_json::Value,
    state: String,
    revoked_at: Option<String>,
    roster: RosterEvidence,
) -> PathEvidence {
    PathEvidence {
        commitment_cid: commitment_cid.to_string(),
        state,
        revoked_at,
        from_dna_hash: string_field(payload, "from_dna_hash"),
        to_dna_hash: string_field(payload, "to_dna_hash"),
        constitution_root: string_field(payload, "constitution_root"),
        roster_cid: string_field(payload, "roster_cid"),
        // WHO signed, as the body renders them — the roster check's input.
        // Read off each element's `agent`, which is the field
        // `mishpat::validate_lineage_signatures` verifies the signature
        // against; an element without one contributes no signer, so it can
        // never be counted toward a roster quorum.
        signers: signer_agents(payload),
        roster,
        // The COUNT of signatures the commitment carries — read as the length
        // of the array, so a body that lists three signers cannot claim four.
        signatures: payload
            .get("signatures")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0),
        // FLOORED, never taken verbatim. An absent field defaults to the
        // floor, and so does an EXPLICIT zero: `0 of 0` satisfies every
        // comparison in `verify_path`, so a body declaring
        // `required_signatures: 0` would otherwise mint a path that needs no
        // signature at all — the one value a quorum rule must not be able to
        // express. `.max()` rather than a rejection, because a body we cannot
        // read as stricter is read as the floor, never as unconstrained.
        required_signatures: payload
            .get("required_signatures")
            .or_else(|| payload.get("requiredSignatures"))
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_REQUIRED_SIGNATURES)
            .max(DEFAULT_REQUIRED_SIGNATURES),
    }
}

/// A string field from the commitment body, accepting either the snake_case
/// spelling the mishpat payloads use or the camelCase one the manifest schema
/// uses. Absent → empty string (see [`evidence_from`]).
fn string_field(payload: &serde_json::Value, snake: &str) -> String {
    let camel = to_lower_camel(snake);
    payload
        .get(snake)
        .or_else(|| payload.get(camel.as_str()))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Whether a string is an entry hash this conductor could actually be asked
/// about — the same `EntryHash::try_from` the mishpat coordinator applies,
/// run locally so a malformed cid never has to be told apart from an outage
/// after the fact.
fn is_addressable_cid(cid: &str) -> bool {
    holochain_types::prelude::EntryHash::try_from(cid.to_string()).is_ok()
}

/// A string ARRAY field from a commitment body, accepting either spelling —
/// the same snake/camel tolerance [`string_field`] applies, for the same
/// reason. A non-array, or an element that is not a string, contributes
/// nothing rather than erroring: an unparseable member can never be matched
/// by a signer, so it fails closed.
fn string_array_field(payload: &serde_json::Value, snake: &str) -> Vec<String> {
    let camel = to_lower_camel(snake);
    payload
        .get(snake)
        .or_else(|| payload.get(camel.as_str()))
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The agents named by a body's `signatures: [{agent, signature}]` array.
///
/// A body that spells its signatures as bare strings (the shape some older
/// fixtures use) is read as those strings being the agents, so a roster check
/// still has something to compare rather than silently seeing zero signers and
/// falling through to the count rule.
fn signer_agents(payload: &serde_json::Value) -> Vec<String> {
    payload
        .get("signatures")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    s.get("agent")
                        .and_then(|v| v.as_str())
                        .or_else(|| s.as_str())
                })
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// `from_dna_hash` → `fromDnaHash`. Small enough to keep local; the alternative
/// is a dependency on a case crate for four field names.
fn to_lower_camel(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut upper_next = false;
    for ch in snake.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Read every `CommitmentByState` transition on a commitment's anchor through
/// THIS peer's own conductor.
///
/// The C5 rail for LIFECYCLE, and it draws the same C4 line the body read does:
/// an empty `Vec` is the conductor ANSWERING that its DHT view carries no
/// transition (the caller falls back to the cache, then to `proposed`); an
/// `Err(())` is a failure to ASK or to READ, which makes the whole evidence
/// [`Answer::Unreachable`] and the refusal `conductor_unavailable`.
async fn read_state_links(hc: &Arc<HcClient>, cid: &str) -> Result<Vec<CommitmentStateLink>, ()> {
    match crate::services::conductor_writes::get_commitment_state_links(hc, cid).await {
        Ok(links) => Ok(links),
        Err(e) => {
            tracing::debug!(
                commitment_cid = %cid,
                error = %e,
                "release-adoption: path lifecycle links unreadable — unreachable, never a state"
            );
            Err(())
        }
    }
}

/// The lifecycle the DHT declares, or `None` when it declares none.
///
/// # Not a timeline replay, a fail-closed ladder
///
/// `signed_at` is caller-supplied and its format is not pinned (the mishpat
/// coordinator's own doc allows ISO-8601 *or* epoch seconds), so ordering the
/// links by it would be guessing. It does not need to be ordered:
///
/// 1. **Any `revoked` link wins outright.** Revocation is terminal — nothing
///    reopens a revoked path (epic Station 8) — so whichever order the links
///    arrive in, the answer is the same. `revoked_at` is the smallest declared
///    signing time among them, which makes the value deterministic rather than
///    dependent on DHT link order.
/// 2. Otherwise any `active` link makes it active.
/// 3. Otherwise the smallest state string present, deterministically — some
///    transition this reader does not know, which `verify_path` refuses anyway
///    because it is not `"active"`.
///
/// Links carrying no usable state at all read as `None`, and the caller falls
/// back to the cache exactly as it does for an empty answer.
fn lifecycle_from_links(links: &[CommitmentStateLink]) -> Option<(String, Option<String>)> {
    let revoked_at = links
        .iter()
        .filter(|l| l.state == REVOKED_STATE)
        .map(|l| l.signed_at.clone())
        .filter(|at| !at.is_empty())
        .min();
    if links.iter().any(|l| l.state == REVOKED_STATE) {
        // A revocation with an unreadable time is still a revocation: fall back
        // to the state string itself rather than dropping the refusal, because
        // `verify_path` checks `revoked_at` before it checks `state`.
        return Some((
            REVOKED_STATE.to_string(),
            Some(revoked_at.unwrap_or_else(|| REVOKED_STATE.to_string())),
        ));
    }
    if links.iter().any(|l| l.state == ACTIVE_STATE) {
        return Some((ACTIVE_STATE.to_string(), None));
    }
    links
        .iter()
        .map(|l| l.state.clone())
        .filter(|s| !s.is_empty())
        .min()
        .map(|s| (s, None))
}

/// Join DHT truth with the local cache.
///
/// `state` is the DHT's when the DHT says anything at all; the row answers only
/// when it does not, and `proposed` when neither does (fail-closed — a state we
/// cannot establish is never `active`).
///
/// `revoked_at` is the UNION of the two, and deliberately so. Revocation is
/// terminal and monotone, so a revocation observed from either source revokes:
/// honouring the row here can only ever ADD a refusal, never mint a permission,
/// and it keeps the author of a revocation — the one peer whose row knows about
/// it — refusing even against a stale `active` link. The precedence is the DHT's
/// when both speak.
fn resolve_lifecycle(
    links: &[CommitmentStateLink],
    row: Option<(String, Option<String>)>,
) -> (String, Option<String>) {
    let (row_state, row_revoked_at) = match row {
        Some((state, at)) => (Some(state), at),
        None => (None, None),
    };
    let (state, link_revoked_at) = match lifecycle_from_links(links) {
        Some(pair) => pair,
        None => (
            row_state.unwrap_or_else(|| UNPROJECTED_STATE.to_string()),
            None,
        ),
    };
    (state, link_revoked_at.or(row_revoked_at))
}

/// The commitment's projected lifecycle — the CACHE, since Task 19.
///
/// Three answers, and the two nested layers are both load-bearing:
/// `None` — the projection could not be READ at all (no pool, a checkout that
/// timed out, a query error, a panicked blocking task); the caller answers
/// `Unreachable`. `Some(None)` — the projection ANSWERED and holds no row for
/// this cid. `Some(Some((state, revoked_at)))` — the row, verbatim.
///
/// The distinction is the same C4 line the conductor read draws, applied to the
/// second source. An **absent row** is an answer — this peer holds no
/// projection for the commitment, which for a lineage path is every peer but
/// its author — and [`resolve_lifecycle`] decides what that answer means now
/// that the DHT carries the fact directly. A **failed read** — no pool
/// configured, a pool checkout that timed out, a query error, a blocking task
/// that panicked — is not an answer, and returning `proposed` for it would let
/// a busy sqlite read as "the elohim did not notarize this path". Those all
/// come back `None`, and the caller answers [`Answer::Unreachable`].
#[allow(clippy::option_option)]
async fn lifecycle(db: Option<&DbPool>, cid: &str) -> Option<Option<(String, Option<String>)>> {
    let pool = db.cloned()?;
    let cid = cid.to_string();
    // The pool checkout + diesel query are blocking; offload exactly as
    // `ProjectionCommitmentFetcher::fetch` does rather than stalling a runtime
    // worker on a sqlite read.
    let joined = tokio::task::spawn_blocking(
        move || -> Result<Option<crate::db::models::MishpatCommitment>, String> {
            let mut conn = pool.get().map_err(|e| format!("db pool: {e}"))?;
            crate::db::mishpat_commitments::get_by_cid(&mut conn, &cid)
                .map_err(|e| format!("query: {e}"))
        },
    )
    .await;
    match joined {
        Ok(Ok(Some(row))) => Some(Some((row.state, row.revoked_at))),
        // The projection answered: no such row here yet.
        Ok(Ok(None)) => Some(None),
        Ok(Err(e)) => {
            tracing::warn!(
                error = %e,
                "release-adoption: path lifecycle unreadable — unreachable, never a fabricated state"
            );
            None
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "release-adoption: path lifecycle read panicked/aborted — unreachable"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The roster [`body`] names, as this peer read it back off its own
    /// conductor: the three signers of the fixture, and nobody else.
    fn roster() -> RosterEvidence {
        RosterEvidence::Read {
            members: vec![
                "uhCAkSignerOne".to_string(),
                "uhCAkSignerTwo".to_string(),
                "uhCAkSignerThree".to_string(),
            ],
            constitution_root: Some("bafyLineageConstitutionRoot".to_string()),
        }
    }

    /// A `CommitmentByState` transition as this peer read it back off its own
    /// conductor.
    fn link(state: &str, signed_at: &str) -> CommitmentStateLink {
        CommitmentStateLink {
            state: state.to_string(),
            signed_at: signed_at.to_string(),
            event_hash: "uhCkkLifecycleEvent".to_string(),
        }
    }

    /// The measured Station-2 defect on the two peers that did NOT notarize:
    /// only the AUTHOR's storage projects a `mishpat_commitments` row (the
    /// `CommitmentCommitted` signal is a post-commit hook on the author's own
    /// conductor), so every other peer read the same notarized commitment off
    /// its own DHT view and found no row — and "no row" was refused as
    /// `proposed`.
    ///
    /// Task 19 closes it where the fact belongs: the mishpat coordinator
    /// authors an `active` link on the commitment's own anchor when it accepts
    /// the quorum, so every peer reads the activation off the DHT, with no row
    /// and no inference.
    #[test]
    fn an_active_link_is_read_by_a_peer_that_holds_no_row() {
        assert_eq!(
            resolve_lifecycle(&[link(ACTIVE_STATE, "2026-09-04T00:00:00Z")], None),
            ("active".to_string(), None),
            "a peer with no projection row must still read the DHT's activation"
        );
    }

    /// **The Task 19 deliverable, at the reading end.** A revocation authored
    /// by ANOTHER peer reaches this one as a `revoked` link on the path's
    /// anchor — no row and no signal needed — and `verify_path` refuses
    /// `path_revoked` because it checks `revoked_at` before anything else.
    ///
    /// It is a PUBLIC statement, not yet a verified one: nothing binds a link's
    /// author to its anchor (gap G7, at the read site), so this arm is also the
    /// arm an unauthorized `revoked|<t>` would ride to a permanent denial. The
    /// asymmetry is deliberate for now — over-refusing a path is safe, and a
    /// path's ACCEPTANCE still rests on the quorum and roster checks no link can
    /// forge.
    #[test]
    fn a_revocation_authored_elsewhere_is_read_as_revoked_here() {
        let links = [
            link(ACTIVE_STATE, "2026-09-04T00:00:00Z"),
            link(REVOKED_STATE, "2026-09-05T00:00:00Z"),
        ];
        assert_eq!(
            resolve_lifecycle(&links, None),
            (
                "revoked".to_string(),
                Some("2026-09-05T00:00:00Z".to_string())
            ),
            "a revoked link must win outright over the activation it pulls back"
        );
        // …and in the other link order, because the ladder does not replay a
        // timeline it cannot order (see `lifecycle_from_links`).
        let reversed = [links[1].clone(), links[0].clone()];
        assert_eq!(
            resolve_lifecycle(&reversed, None),
            resolve_lifecycle(&links, None)
        );
    }

    /// A revoked link whose signing time is unreadable is still a revocation —
    /// `verify_path` reads `revoked_at.is_some()`, so dropping the time would
    /// drop the refusal.
    #[test]
    fn a_revocation_with_no_readable_time_still_refuses() {
        let (state, revoked_at) = resolve_lifecycle(&[link(REVOKED_STATE, "")], None);
        assert_eq!(state, "revoked");
        assert!(
            revoked_at.is_some(),
            "an unreadable revocation time must never turn a revocation into a pass"
        );
    }

    /// The row is a CACHE, and only that: when the DHT says anything at all,
    /// the DHT's state is the answer.
    #[test]
    fn the_dht_outranks_the_local_row() {
        assert_eq!(
            resolve_lifecycle(
                &[link(ACTIVE_STATE, "2026-09-04T00:00:00Z")],
                Some(("proposed".to_string(), None))
            )
            .0,
            "active"
        );
    }

    /// …but revocation is TERMINAL and MONOTONE, so it is a union rather than
    /// a precedence: a revocation the author's own row knows about still
    /// refuses, even against a stale `active` link. Honouring the cache here
    /// can only ever add a refusal, never mint a permission.
    #[test]
    fn a_row_revocation_still_refuses_against_an_active_link() {
        let (state, revoked_at) = resolve_lifecycle(
            &[link(ACTIVE_STATE, "2026-09-04T00:00:00Z")],
            Some((
                "revoked".to_string(),
                Some("2026-09-05T00:00:00Z".to_string()),
            )),
        );
        assert_eq!(state, "active", "the DHT still owns the state");
        assert_eq!(
            revoked_at,
            Some("2026-09-05T00:00:00Z".to_string()),
            "and the revocation the cache holds still refuses the path"
        );
    }

    /// No links and no row is `proposed` — fail-closed. The entry-action
    /// inference that used to fill this gap is gone on purpose: it made
    /// activation something an author could produce by writing an entry,
    /// which is exactly what a revocation then could not undo.
    #[test]
    fn no_links_and_no_row_is_proposed() {
        assert_eq!(
            resolve_lifecycle(&[], None),
            (UNPROJECTED_STATE.to_string(), None)
        );
        assert_ne!(resolve_lifecycle(&[], None).0, ACTIVE_STATE);
    }

    /// With no links, the row answers — the cache is still read, it is just no
    /// longer the authority.
    #[test]
    fn with_no_links_the_row_answers() {
        assert_eq!(
            resolve_lifecycle(
                &[],
                Some((
                    "revoked".to_string(),
                    Some("2026-09-05T00:00:00Z".to_string())
                ))
            ),
            (
                "revoked".to_string(),
                Some("2026-09-05T00:00:00Z".to_string())
            )
        );
    }

    /// A transition this reader does not know reads as itself, deterministically
    /// — and `verify_path` refuses it, because it is not `"active"`.
    #[test]
    fn an_unknown_transition_is_deterministic_and_refuses() {
        let links = [link("zeta", "t2"), link("alpha", "t1")];
        assert_eq!(resolve_lifecycle(&links, None), ("alpha".to_string(), None));
        assert_ne!(resolve_lifecycle(&links, None).0, ACTIVE_STATE);
    }

    /// A migrates-lineage commitment body in the shape the NOTARIZING side
    /// actually writes — every field
    /// `mishpat::commitments::validate_migrates_lineage` requires, with
    /// `signatures` as `[{agent, signature}]` OBJECTS (that validator's
    /// `validate_lineage_signatures` reads `.agent` and `.signature` off each
    /// element and verifies over `signing_payload_cid`).
    ///
    /// This side only takes `.len()` of that array, which is correct — the
    /// signatures were verified in-wasm at commit time and re-verifying a
    /// notarized entry here would be re-deriving authority the DHT already
    /// established. But the FIXTURE must be the real shape, or the parse is
    /// pinned against a body no conductor will ever return.
    fn body() -> serde_json::Value {
        serde_json::json!({
            "action": "migrates-lineage",
            "role": "node_registry",
            "from_dna_hash": "uhC0kINSTALLED",
            "to_dna_hash": "uhC0kV2NODEREG",
            "release_cid": "uhCkkLineageReleaseHead",
            "constitution_root": "bafyLineageConstitutionRoot",
            "roster_cid": "bafyProgenitorRoster",
            "signing_payload_cid": "bafySigningPayload",
            "signatures": [
                { "agent": "uhCAkSignerOne", "signature": "c2lnLW9uZQ==" },
                { "agent": "uhCAkSignerTwo", "signature": "c2lnLXR3bw==" },
                { "agent": "uhCAkSignerThree", "signature": "c2lnLXRocmVl" },
            ],
            "required_signatures": 3,
            "evidence": { "soakSecs": 900 },
            "window": {
                "opens_at": "2026-09-04T00:00:00Z",
                "revert_until": "2026-09-11T00:00:00Z",
            },
        })
    }

    /// The body's fields land where `verify_path` reads them, and `signatures`
    /// is the ARRAY LENGTH — a body listing three signers must never be able
    /// to report a different number.
    #[test]
    fn a_payload_fixture_parses_into_the_evidence_verify_path_reads() {
        let ev = evidence_from(
            "uhCEkPathCommitment",
            &body(),
            "active".to_string(),
            None,
            roster(),
        );
        assert_eq!(ev.commitment_cid, "uhCEkPathCommitment");
        assert_eq!(ev.from_dna_hash, "uhC0kINSTALLED");
        assert_eq!(ev.to_dna_hash, "uhC0kV2NODEREG");
        assert_eq!(ev.constitution_root, "bafyLineageConstitutionRoot");
        assert_eq!(ev.signatures, 3);
        assert_eq!(ev.required_signatures, 3);
        assert_eq!(ev.state, "active");
        assert!(ev.revoked_at.is_none());
        // **Task 16.** WHO signed, off each element's `agent` — the field the
        // notarizing validator itself verifies the signature against. The
        // roster the body NAMES is carried too, so a refusal can point at it.
        assert_eq!(ev.roster_cid, "bafyProgenitorRoster");
        assert_eq!(
            ev.signers,
            vec!["uhCAkSignerOne", "uhCAkSignerTwo", "uhCAkSignerThree"]
        );
        assert_eq!(ev.signers.len(), ev.signatures);
        assert_eq!(ev.roster, roster());
    }

    /// **Task 16, the parse half.** A signature element with no readable
    /// `agent` contributes NO signer while still counting toward
    /// `signatures` — which is exactly the gap `verify_path`'s
    /// count-only-roster-members rule closes: the headcount says three, the
    /// roster can back one, and the path is refused.
    #[test]
    fn a_signature_without_a_readable_agent_names_no_signer() {
        let body = serde_json::json!({
            "roster_cid": "bafyProgenitorRoster",
            "signatures": [
                { "agent": "uhCAkSignerOne", "signature": "c2ln" },
                { "signature": "c2ln" },
                { "agent": "", "signature": "c2ln" },
            ],
            "required_signatures": 3,
        });
        let ev = evidence_from("uhCEkX", &body, "active".to_string(), None, roster());
        assert_eq!(ev.signatures, 3, "the array length is unchanged");
        assert_eq!(
            ev.signers,
            vec!["uhCAkSignerOne"],
            "an element with no readable agent can never be counted onto a roster"
        );
    }

    /// A body naming no ADDRESSABLE roster reads as `roster_members: None`
    /// WITHOUT a conductor round-trip — asserted by passing no bridge, which
    /// would answer `Err` (→ `Unreachable`) if the cid were dialled. The
    /// refusal that `None` produces is `verify_path`'s `quorum_unmet`.
    ///
    /// The second case is the one that matters in practice: `roster_cid` is a
    /// free string in the notarized body, and
    /// `mishpat::get_commitment`'s `EntryHash::try_from` turns anything that
    /// is not a base64 entry hash into a guest ERROR. Read naively that would
    /// arrive as `conductor_unavailable` — OUR outage — when the honest
    /// finding is that the commitment named a roster that cannot exist. The
    /// live a2o Station 10 fixture names exactly such a roster
    /// (`a2o-fixture-bootstrap-steward-roster`), so this is the difference
    /// between the story measuring `quorum_unmet` and it measuring a
    /// substrate failure.
    #[tokio::test]
    async fn a_body_naming_no_addressable_roster_is_an_absent_roster_not_an_outage() {
        assert_eq!(
            read_roster(None, "").await,
            Ok(RosterEvidence::Unaddressable),
            "no roster named is an ANSWER, and never a dial"
        );
        assert_eq!(
            read_roster(None, "a2o-fixture-bootstrap-steward-roster").await,
            Ok(RosterEvidence::Unaddressable),
            "a roster cid that is not an ADDRESS is a fact about the commitment, never an outage"
        );
        assert!(!is_addressable_cid("a2o-fixture-bootstrap-steward-roster"));
        assert!(!is_addressable_cid("bafyProgenitorRoster"));

        // …whereas a roster named by a REAL entry hash, with no bridge to ask
        // through, is an outage — and must never read as an absent roster (C4).
        let real = holochain_types::prelude::EntryHash::from_raw_32(vec![0x5A; 32]).to_string();
        assert!(is_addressable_cid(&real));
        assert_eq!(
            read_roster(None, &real).await,
            Err(()),
            "a roster we could not ASK about is unreachable, never absence"
        );
    }

    /// **Hardening 4 — the SIBLING read.** The same rule applied to the path's
    /// own `commitmentCid`. `mishpat::get_commitment` guest-errors on a
    /// non-entry-hash, which would read as `conductor_unavailable` — our
    /// outage — when the honest finding is that the MANIFEST named a
    /// commitment that cannot exist. Asserted with no bridge, which would
    /// answer `Unreachable` if the cid were dialled.
    #[tokio::test]
    async fn an_unaddressable_path_commitment_is_absent_not_unreachable() {
        let mut m = super::super::test_support::lineage_manifest();
        m.adoption_discipline.path = Some(crate::services::release_attestation::PathRef {
            commitment_cid: "not-an-entry-hash-at-all".to_string(),
        });
        assert!(
            matches!(fetch_path_evidence(None, None, &m).await, Answer::Absent),
            "a path commitment that is not an address is path_not_notarized, never our outage"
        );

        // The control: a REAL entry hash with no bridge stays Unreachable, so
        // what is pinned is the addressability rule and not a blanket Absent.
        m.adoption_discipline.path = Some(crate::services::release_attestation::PathRef {
            commitment_cid: holochain_types::prelude::EntryHash::from_raw_32(vec![0x11; 32])
                .to_string(),
        });
        assert!(matches!(
            fetch_path_evidence(None, None, &m).await,
            Answer::Unreachable
        ));
    }

    /// **Hardening 1, the parse half.** The roster's OWN
    /// `constitution_root` is read alongside its members — `verify_path`
    /// refuses a roster minted under a different constitution, and it can only
    /// do that if this side carries the root. An empty or absent root is
    /// `None` ("declares no root"), never an empty string masquerading as one.
    #[test]
    fn a_roster_body_carries_the_root_it_declares_itself_under() {
        let body = serde_json::json!({
            "members": ["uhCAkSignerOne"],
            "constitution_root": "bafyLineageConstitutionRoot",
        });
        assert_eq!(
            string_field(&body, "constitution_root"),
            "bafyLineageConstitutionRoot"
        );
        for empty in [
            serde_json::json!({}),
            serde_json::json!({"constitution_root": ""}),
        ] {
            assert!(
                Some(string_field(&empty, "constitution_root"))
                    .filter(|r| !r.is_empty())
                    .is_none(),
                "a roster declaring no root is None, never an empty-string root"
            );
        }
    }

    /// **Hardening 3, at the parse site.** An EXPLICIT `required_signatures: 0`
    /// floors to [`DEFAULT_REQUIRED_SIGNATURES`] — `0 of 0` satisfies every
    /// comparison in `verify_path`, so a zero would mint a path needing no
    /// signature at all. `.max()` rather than a rejection: a body we cannot
    /// read as stricter is read as the floor, never as unconstrained.
    #[test]
    fn an_explicit_zero_quorum_floors_rather_than_going_vacuous() {
        let zero = serde_json::json!({ "signatures": [], "required_signatures": 0 });
        let ev = evidence_from("uhCEkX", &zero, "active".to_string(), None, roster());
        assert_eq!(ev.required_signatures, DEFAULT_REQUIRED_SIGNATURES);
        assert!(ev.required_signatures > 0);
        assert!(
            ev.signatures < ev.required_signatures,
            "a zero-quorum body must still fail the count check"
        );
        // A stated quorum ABOVE the floor is untouched — the floor raises, it
        // never caps.
        let three = serde_json::json!({ "signatures": [], "required_signatures": 3 });
        assert_eq!(
            evidence_from("uhCEkX", &three, "active".to_string(), None, roster())
                .required_signatures,
            3
        );
    }

    /// The lifecycle [`resolve_lifecycle`] settled — not the body's — decides
    /// `state` and `revoked_at`. A revoked commitment whose BODY still reads
    /// active must come through as revoked, because revocation is a lifecycle
    /// fact that lands after the entry was written and can never be inside it.
    #[test]
    fn the_lifecycle_is_supplied_by_the_caller_never_read_off_the_body() {
        let ev = evidence_from(
            "uhCEkPathCommitment",
            &body(),
            "active".to_string(),
            Some("2026-09-04T10:00:00Z".to_string()),
            roster(),
        );
        assert_eq!(ev.revoked_at.as_deref(), Some("2026-09-04T10:00:00Z"));
        // And the fail-closed default a missing row produces.
        let unprojected = evidence_from(
            "uhCEkPathCommitment",
            &body(),
            UNPROJECTED_STATE.to_string(),
            None,
            roster(),
        );
        assert_eq!(unprojected.state, "proposed");
        assert_ne!(
            unprojected.state, "active",
            "an unprojected row must never read as the one state that establishes a path"
        );
    }

    /// A body missing every field yields empty crossings and the quorum FLOOR
    /// — never a vacuous `0 of 0` that would pass the quorum check.
    #[test]
    fn an_empty_body_defaults_to_the_quorum_floor_never_a_vacuous_pass() {
        let ev = evidence_from(
            "uhCEkPathCommitment",
            &serde_json::json!({}),
            "active".to_string(),
            None,
            roster(),
        );
        assert_eq!(ev.signatures, 0);
        // The SAME default the notarizing side applies
        // (`validate_lineage_signatures`: `None => 1usize`), so the two sides
        // cannot disagree about what quorum an unstated k-of-n means.
        assert_eq!(ev.required_signatures, DEFAULT_REQUIRED_SIGNATURES);
        assert_eq!(DEFAULT_REQUIRED_SIGNATURES, 1);
        assert!(ev.required_signatures > 0);
        assert!(ev.signatures < ev.required_signatures, "must fail quorum");
        assert!(ev.from_dna_hash.is_empty());
    }

    /// camelCase bodies read identically — the manifest schema spells these
    /// fields one way and the mishpat payloads the other, and a path must not
    /// depend on which side authored the commitment.
    #[test]
    fn camel_case_bodies_read_the_same_as_snake_case_ones() {
        let camel = serde_json::json!({
            "fromDnaHash": "uhC0kINSTALLED",
            "toDnaHash": "uhC0kV2NODEREG",
            "constitutionRoot": "bafyRoot",
            "signatures": ["a"],
            "requiredSignatures": 1,
        });
        let ev = evidence_from("uhCEkX", &camel, "active".to_string(), None, roster());
        assert_eq!(ev.from_dna_hash, "uhC0kINSTALLED");
        assert_eq!(ev.to_dna_hash, "uhC0kV2NODEREG");
        assert_eq!(ev.constitution_root, "bafyRoot");
        assert_eq!(ev.signatures, 1);
        assert_eq!(ev.required_signatures, 1);
        assert_eq!(to_lower_camel("from_dna_hash"), "fromDnaHash");
    }

    /// The roster body's `members` parse, which is the whole of what this side
    /// reads off the roster commitment. A non-string or empty member is
    /// DROPPED rather than erroring — an unparseable member can never be
    /// matched by a signer, so dropping it fails closed, while erroring would
    /// turn one malformed entry into a `conductor_unavailable` on an otherwise
    /// readable roster.
    #[test]
    fn a_roster_body_parses_its_members_and_drops_the_unreadable_ones() {
        let roster_body = serde_json::json!({
            "action": "declares-roster",
            "constitution_root": "bafyLineageConstitutionRoot",
            "members": ["uhCAkSignerOne", 7, "", "uhCAkSignerTwo", null],
        });
        assert_eq!(
            string_array_field(&roster_body, "members"),
            vec!["uhCAkSignerOne", "uhCAkSignerTwo"]
        );
        // A roster body with no members list at all is an EMPTY roster — read
        // as an answer, and one no signer can be on, so every path under it
        // refuses.
        assert!(string_array_field(&serde_json::json!({}), "members").is_empty());
    }

    /// Every class but `happ-lineage` is Absent WITHOUT a conductor round-trip
    /// — asserted by passing no bridge at all: a class that consulted the
    /// conductor would answer `Unreachable` here instead.
    #[tokio::test]
    async fn a_non_lineage_class_never_pays_for_a_conductor_call() {
        for class in [
            ArtifactClass::CoordinatorBundle,
            ArtifactClass::ConfigEpr,
            ArtifactClass::StorageBinary,
            ArtifactClass::HappBundle,
        ] {
            let mut m = super::super::test_support::lineage_manifest();
            m.artifact_class = class;
            let answer = fetch_path_evidence(None, None, &m).await;
            assert!(
                matches!(answer, Answer::Absent),
                "{} must be Absent without asking the conductor",
                class.label()
            );
        }
    }

    /// A `happ-lineage` release with no bridge to ask through is UNREACHABLE,
    /// never Absent — our outage is never a statement about the elohim's
    /// governance (C4).
    #[tokio::test]
    async fn no_bridge_is_unreachable_never_absent() {
        let m = super::super::test_support::lineage_manifest();
        assert!(matches!(
            fetch_path_evidence(None, None, &m).await,
            Answer::Unreachable
        ));
    }

    /// …and a `happ-lineage` release that names NO path needs no evidence at
    /// all: `verify_path` refuses it on the schema, so there is nothing to go
    /// and read.
    #[tokio::test]
    async fn a_lineage_release_naming_no_path_is_absent_not_unreachable() {
        let mut m = super::super::test_support::lineage_manifest();
        m.adoption_discipline.path = None;
        assert!(matches!(
            fetch_path_evidence(None, None, &m).await,
            Answer::Absent
        ));
    }

    /// **C4 on the SECOND source.** The projection supplies `state` and
    /// `revoked_at`, and the three outcomes must stay distinct:
    ///
    /// - **No pool configured** → `None` → the fetch answers `Unreachable`. We
    ///   could not READ the lifecycle, and a fabricated `proposed` would let an
    ///   unconfigured (or busy, or broken) sqlite read as "the elohim did not
    ///   notarize this path".
    /// - **Row absent, pool present** → an ANSWER: `proposed`, which refuses on
    ///   state and self-heals the moment the projection lands.
    /// - **Row revoked** → the revocation carried through verbatim.
    ///
    /// Run against a real in-memory pool, so what is asserted is the query
    /// path's own behaviour rather than a hand-built tuple.
    #[tokio::test]
    async fn an_unreadable_lifecycle_is_unreachable_and_an_absent_row_is_proposed() {
        use diesel::r2d2::{ConnectionManager, Pool};
        use diesel::sqlite::SqliteConnection;

        // (1) No pool at all — unreadable, never a state.
        assert!(
            lifecycle(None, "uhCEkPathCommitment").await.is_none(),
            "an unconfigured pool is an unreadable lifecycle, never a fabricated state"
        );
        // …and the fetch boundary turns that into Unreachable rather than the
        // Absent that would read as "not notarized". A manifest is needed here
        // only to reach the lifecycle read at all.
        let m = super::super::test_support::lineage_manifest();
        assert!(matches!(
            fetch_path_evidence(None, None, &m).await,
            Answer::Unreachable
        ));

        let url = format!(
            "file:path_evidence_lifecycle_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool: DbPool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        crate::db::run_migrations(&pool).expect("migrations");

        // (2) Pool present, row absent — the projection ANSWERED "no such row",
        // which is `Some(None)` and NOT the same value as "could not read".
        let answered = lifecycle(Some(&pool), "uhCEkPathCommitment")
            .await
            .expect("a reachable projection always answers");
        assert!(
            answered.is_none(),
            "an absent row is an ANSWER of no-row, not a fabricated lifecycle"
        );
        // What that no-row answer means is the caller's decision, per action
        // class. A class with a real acceptance step still refuses.
        let ev = evidence_from(
            "uhCEkPathCommitment",
            &body(),
            UNPROJECTED_STATE.to_string(),
            None,
            roster(),
        );
        assert_eq!(ev.state, "proposed");
        assert_ne!(
            ev.state, "active",
            "an unprojected row must never read as the one state that establishes a path"
        );

        // (3) A projected, REVOKED row — the revocation is carried, and it is
        // read off the projection rather than off the DHT body (which still
        // says nothing about revocation: it cannot, it was written first).
        {
            let mut conn = pool.get().expect("conn");
            crate::db::mishpat_commitments::upsert_with_anchor(
                &mut conn,
                crate::db::models::NewMishpatCommitment {
                    cid: "uhCEkPathCommitment".to_string(),
                    action: "migrates-lineage".to_string(),
                    scope: "migrates-lineage".to_string(),
                    provider: "uhCAkSignerOne".to_string(),
                    recipient: "uhCAkSignerTwo".to_string(),
                    bounds_json: "{}".to_string(),
                    valid_from: "2026-09-04T00:00:00Z".to_string(),
                    valid_until: "2026-09-11T00:00:00Z".to_string(),
                    revoked_at: Some("2026-09-04T10:00:00Z".to_string()),
                    state: "active".to_string(),
                    dht_anchor_hash: Some("uhCkkPathCommitmentAction".to_string()),
                },
            )
            .expect("upsert");
        }
        let (state, revoked) = lifecycle(Some(&pool), "uhCEkPathCommitment")
            .await
            .expect("a reachable projection always answers")
            .expect("the row is present now");
        let ev = evidence_from("uhCEkPathCommitment", &body(), state, revoked, roster());
        assert_eq!(ev.state, "active");
        assert_eq!(
            ev.revoked_at.as_deref(),
            Some("2026-09-04T10:00:00Z"),
            "verify_path refuses `path_revoked` on this, terminally and independent of state"
        );
    }
}
