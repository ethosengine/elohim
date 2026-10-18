//! Pointer-audit sweep (story 1.4d) — heal an already-torn declared row.
//!
//! ## The gap it closes
//!
//! Story 1.4a (T-1/T-2) made `adopt_local` carry a fresh conductor answer's
//! `blob_cid`/`content_size_bytes` on a FILL or a genuine head MOVE, and T7
//! carries the SAME pair when the answer names the head a row already
//! declares — but only for rows some OTHER path already selected (an
//! inbound `resolve_content_head` call, a sync apply, a reconcile signal).
//! `p2p::projection_reconcile::classify_content_gap` compares anchors, not
//! bytes, so a row whose declared head is intact but whose `blob_cid` no
//! longer names the SAME blob as that head's own notarized record reads as
//! `InSync` forever — nothing re-selects it. That is the live read from the
//! 2026-09-22 seam-smoke: one notarized head, two blobs.
//!
//! This sweep closes the gap by walking declared rows that carry a blob
//! pointer, probing THIS node's own conductor for the head they already
//! declare, and running the identical T7 heal (`pointer_heal_patch` +
//! `stamp_declared_head_mode(.., StampMode::HealCanonical, ..)`) that
//! `adopt_local` already runs — so a row this sweep touches is
//! indistinguishable, after the fact, from one `adopt_local` healed on its
//! own. Nothing here authors, declares, mints a contest, or moves a head:
//! the stamp mode is `HealCanonical`, and `pointer_heal_patch`'s condition 2
//! (exact, untrimmed equality between the row's declared head and the
//! record's head) means a genuine divergence — the row declaring one head
//! while its own conductor resolves a DIFFERENT one — is refused, not
//! adopted. That class stays entirely with `head_adoption`'s other paths.
//!
//! ## Bounded, round-robin, Category C
//!
//! Candidates are selected via
//! [`content_diesel::list_declared_blob_pointer_candidates`], a keyset
//! cursor over `(declared_head_action_hash IS NOT NULL AND blob_cid IS NOT
//! NULL)` rows — a small set by construction (1.4a's design pass sampled
//! 3,770 content items with no blob pointer at all; only app bundles carry
//! one). Each sweep visits at most `batch` rows starting after the cursor
//! the previous sweep returned, and wraps to the beginning when a page comes
//! back shorter than `batch` — the same contract as
//! `sync::projector::heal_half_rows_from_docs`. Operational (Category C):
//! nothing here is notarized, and a row this sweep skips is picked up again
//! on a later lap.
//!
//! ## What this sweep never touches: a slug a release channel owns
//!
//! An app slug bound to a release channel (`metadata.releaseChannel`, the
//! slice-2 mutual binding) is pointed at its bytes by the channel's vehicle
//! (`release_adoption::apply::AppBundleVehicle`), not by its own head. Its
//! declared head is the legacy head the pipeline once stamped, and that
//! head's record names an OLD blob — so to this sweep the row looks exactly
//! like the torn pointer it exists to heal. Alpha, 2026-10-07 21:07:51Z: the
//! vehicle moved `lamad-spa` to the new build; at 21:08:41Z this sweep
//! "refreshed" it back to the 2026-09-22 blob (size 42), and the fleet served
//! a sixteen-day-old bundle while the adoption ledger read `applied`.
//! `head_adoption`'s pre-flight already holds such a slug at step (-1); this
//! sweep is a separate entry and must hold it the same way, BEFORE it spends
//! a conductor call.

use std::sync::Arc;

use crate::conductor_admission::AdmissionClass;
use crate::db::content_diesel;
use crate::db::content_diesel::StampMode;
use crate::db::{AppContext, DbPool};
use crate::hc_client::HcClient;
use crate::services::conductor_writes;
use crate::services::head_adoption::pointer_heal_patch;
use crate::StorageError;

/// Rows visited per sweep tick. The candidate population is tiny by
/// construction (see module docs), so this is generous headroom rather than
/// a tight cap — it exists to bound a single sweep's conductor round-trips,
/// not to throttle a large corpus.
pub const POINTER_AUDIT_BATCH: i64 = 25;

/// Cadence between sweeps — same order of magnitude as the sibling
/// `custody_rotation` tick (`ROTATION_TICK_SECONDS`); this sweep is cheaper
/// per candidate (one resolve + at most one stamp) but must never compete
/// with interactive traffic on the SAME admission lane, hence
/// [`SWEEP_PROBE_CLASS`] below.
pub const POINTER_AUDIT_TICK_SECONDS: u64 = 300;

/// Shared fixture heads for the two test modules below (`tests` and
/// `candidate_query_tests`) — hoisted to module scope so both can reach them
/// via `super::*` without duplicating the literals.
#[cfg(test)]
const HEAD_A: &str = "uhCkkHEADA";
#[cfg(test)]
const HEAD_B: &str = "uhCkkHEADB";

/// BACKGROUND, explicitly — not inherited. A plain resolve defaults to
/// `AdmissionClass::Interactive` (`conductor_writes::call_resolve_content_head`);
/// this sweep is a background loop nobody is waiting on, and borrowing the
/// interactive lane is how a person's read gets starved (the same reasoning
/// `head_adoption::SWEEP_PROBE_CLASS` documents for the reanchor sweep's own
/// probe — that constant is private to its module, so this is a sibling
/// value, not a re-export).
const SWEEP_PROBE_CLASS: AdmissionClass = AdmissionClass::Background;

/// Rows this process has already found IN STEP, each against the exact pair
/// that verdict was reached on: the head the row declares and the blob it
/// points at.
///
/// WHY THIS IS SOUND AND NOT A TTL. The audit compares a row's blob pointer
/// with the record at the row's OWN declared head, and a record at an action
/// hash is immutable. So "this row, declaring head H and pointing at blob B, is
/// in step with H's record" stays true for as long as the row still declares H
/// and still points at B — there is nothing on the conductor that could change
/// it. A tear can only be made by a write to the row, and any such write
/// changes the pair and so misses this memory. Before it, every declared row
/// was re-asked every lap forever: 106 rows on a household peer, one conductor
/// round trip each per ~20 minutes, to re-learn a fact that cannot change.
///
/// Only that one verdict is remembered. `NotCanonical`, `Unreadable`, `Error`
/// and the divergent-head flavour of `InStep` (the row declares a head other
/// than the one the conductor resolves) are all answers that CAN change with
/// no write to the row, and are asked again every lap exactly as before.
///
/// In memory on purpose: a restart re-audits every row once, which is the
/// audit's whole cost paid one time per process rather than one time per lap.
#[derive(Debug, Default)]
pub struct SettledPointers {
    rows: std::sync::Mutex<std::collections::HashMap<String, (String, String)>>,
}

impl SettledPointers {
    /// Was `id` found in step while declaring exactly `declared` and pointing
    /// at exactly `blob`?
    pub fn is_settled(&self, id: &str, declared: &str, blob: &str) -> bool {
        self.rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .is_some_and(|(d, b)| d == declared && b == blob)
    }

    /// Remember that `id` is in step at `(declared, blob)`.
    pub fn settle(&self, id: &str, declared: &str, blob: &str) {
        self.rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.to_string(), (declared.to_string(), blob.to_string()));
    }

    /// Drop every remembered row not in `live` — a row that left the candidate
    /// set must not hold its entry for the life of the process.
    pub fn retain_only(&self, live: &std::collections::HashSet<&str>) {
        self.rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|id, _| live.contains(id.as_str()));
    }

    pub fn len(&self) -> usize {
        self.rows.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The process-wide memory the live sweep uses.
pub fn settled_pointers() -> &'static SettledPointers {
    static SETTLED: std::sync::OnceLock<SettledPointers> = std::sync::OnceLock::new();
    SETTLED.get_or_init(SettledPointers::default)
}

/// May an in-step verdict be remembered? Only the TRUE in-step: a canonical
/// answer for exactly the head the row declares. The other answers `decide`
/// folds into `InStep` can change without the row changing.
pub(crate) fn verdict_is_rememberable(
    outcome: crate::metrics::PointerAuditOutcome,
    head_canonical: bool,
    local_declared: Option<&str>,
    head_action_hash: &str,
) -> bool {
    matches!(outcome, crate::metrics::PointerAuditOutcome::InStep)
        && head_canonical
        && local_declared == Some(head_action_hash)
}

/// One sweep's outcome tally, by [`crate::metrics::PointerAuditOutcome`]
/// label — returned for logging/tests, mirrored into the metric via
/// [`crate::metrics::inc_pointer_audit`] as each row is decided.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PointerAuditStats {
    pub scanned: usize,
    pub healed: usize,
    pub in_step: usize,
    pub not_canonical: usize,
    pub unreadable: usize,
    pub error: usize,
    pub held_by_release_channel: usize,
    /// Rows not asked about because they are remembered in step at the pair
    /// they still carry. No conductor call was made for these.
    pub settled: usize,
}

/// One sweep batch: select up to `batch` candidates after `after`, probe
/// each against this node's own conductor, heal a drifted pointer via the
/// T7 path when the conditions hold, and leave everything else untouched.
///
/// Returns the tally and the next cursor — `None` when this page came back
/// shorter than `batch` (the candidate set is exhausted; the caller wraps to
/// the beginning on the next tick).
pub async fn run_once(
    pool: &DbPool,
    hc: &Arc<HcClient>,
    ctx: &AppContext,
    after: Option<&str>,
    batch: i64,
) -> Result<(PointerAuditStats, Option<String>), StorageError> {
    let ids = {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("pointer_audit: db conn: {e}")))?;
        content_diesel::list_declared_blob_pointer_candidates(&mut conn, ctx, batch, after)?
    };

    let settled = settled_pointers();
    let mut stats = PointerAuditStats::default();
    for id in &ids {
        stats.scanned += 1;
        // Two local reads, no conductor call: is this row still exactly the
        // pair it was found in step at? A read that fails settles nothing and
        // the row is asked about as before.
        if row_is_still_settled(pool, ctx, id, settled) {
            stats.settled += 1;
            crate::metrics::inc_pointer_audit_settled();
            continue;
        }
        let outcome = heal_one(pool, hc, ctx, id, settled).await;
        record(&mut stats, outcome);
        crate::metrics::inc_pointer_audit(outcome);
    }

    let exhausted = (ids.len() as i64) < batch.max(1);
    // A FIRST page that is also the last is the whole candidate set: anything
    // remembered and not in it has left the set.
    if exhausted && after.is_none() {
        settled.retain_only(&ids.iter().map(String::as_str).collect());
    }
    let cursor = if exhausted { None } else { ids.last().cloned() };
    Ok((stats, cursor))
}

fn record(stats: &mut PointerAuditStats, outcome: crate::metrics::PointerAuditOutcome) {
    use crate::metrics::PointerAuditOutcome::*;
    match outcome {
        Healed => stats.healed += 1,
        InStep => stats.in_step += 1,
        NotCanonical => stats.not_canonical += 1,
        Unreadable => stats.unreadable += 1,
        Error => stats.error += 1,
        HeldByReleaseChannel => stats.held_by_release_channel += 1,
    }
}

/// What this sweep should do for ONE candidate, given already-resolved
/// inputs. Pure and total: [`heal_one`] is a thin async wrapper that
/// resolves the conductor answer and the two local reads, calls this, and
/// then writes IF (and only if) it says to.
///
/// `head_canonical=false` is decided HERE, ahead of
/// [`pointer_heal_patch`]'s own identical check, so the sweep can count a
/// non-canonical answer on its own
/// [`crate::metrics::PointerAuditOutcome::NotCanonical`] label instead of
/// folding it into the general no-op bucket `pointer_heal_patch`'s `None`
/// would otherwise produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PointerAuditDecision {
    /// Write this pointer + size via the guarded T7 stamp.
    Heal(String, i32),
    /// Nothing to write; count under this label.
    Skip(crate::metrics::PointerAuditOutcome),
}

pub(crate) fn decide(
    bound_to_release_channel: bool,
    head_canonical: bool,
    local_declared: Option<&str>,
    head_action_hash: &str,
    row_blob_hash: Option<&str>,
    record_blob_cid: Option<&str>,
    record_size_bytes: Option<u64>,
) -> PointerAuditDecision {
    // (-1) A slug a release channel owns is never this sweep's to heal,
    // whatever its own head's record says — see the module docs.
    if bound_to_release_channel {
        return PointerAuditDecision::Skip(
            crate::metrics::PointerAuditOutcome::HeldByReleaseChannel,
        );
    }
    if !head_canonical {
        return PointerAuditDecision::Skip(crate::metrics::PointerAuditOutcome::NotCanonical);
    }
    match pointer_heal_patch(
        head_canonical,
        local_declared,
        head_action_hash,
        row_blob_hash,
        record_blob_cid,
        record_size_bytes,
    ) {
        // Covers: the row already carries this exact record's pointer (truly
        // in step), the record names an empty/absent pointer, or the row's
        // declared head is not (or no longer) EXACTLY this record's head —
        // a genuine head divergence, which stays with `head_adoption`'s
        // other paths and is never touched here.
        None => PointerAuditDecision::Skip(crate::metrics::PointerAuditOutcome::InStep),
        Some((cid, size)) => PointerAuditDecision::Heal(cid, size),
    }
}

/// Does `id`'s row still carry the pair it was remembered in step at?
fn row_is_still_settled(
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    settled: &SettledPointers,
) -> bool {
    let Ok(mut conn) = pool.get() else {
        return false;
    };
    let Ok(Some(declared)) = content_diesel::declared_head_for(&mut conn, ctx, id) else {
        return false;
    };
    // A pointer that cannot be read, or is absent, settles nothing.
    let Ok(Some(blob)) = content_diesel::blob_hash_for(&mut conn, ctx, id) else {
        return false;
    };
    settled.is_settled(id, &declared, &blob)
}

/// Resolve, read, [`decide`], and — only if the decision says to — heal ONE
/// candidate row. Every write is over the SAME guarded stamp
/// `adopt_local`'s T7 branch uses: this function never constructs a patch
/// outside [`decide`]'s `Heal` variant, and never passes a `StampMode` other
/// than [`StampMode::HealCanonical`].
async fn heal_one(
    pool: &DbPool,
    hc: &Arc<HcClient>,
    ctx: &AppContext,
    id: &str,
    settled: &SettledPointers,
) -> crate::metrics::PointerAuditOutcome {
    use crate::metrics::PointerAuditOutcome;

    // (-1) Who owns this row's pointer? Asked FIRST, off the local row alone,
    // so a slug a release channel owns costs this sweep no conductor call at
    // all — the same hold `head_adoption`'s pre-flight applies, counted on
    // the same `elohim_content_adopt_held_total{arm}` series.
    {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(content_id = %id, error = %e, "pointer_audit: db conn for binding read");
                return PointerAuditOutcome::Error;
            }
        };
        match crate::services::head_adoption::row_release_channel(&mut conn, ctx, id) {
            Ok(Some(channel)) => {
                tracing::debug!(
                    content_id = %id, channel = %channel,
                    "pointer_audit: held — the slug is bound to a release channel, which owns \
                     its serving pointer; its own head's record is not consulted"
                );
                crate::services::head_adoption::note_held_bound_to_release_channel();
                return PointerAuditOutcome::HeldByReleaseChannel;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(content_id = %id, error = %e, "pointer_audit: binding read failed");
                return PointerAuditOutcome::Error;
            }
        }
    }

    // Own-conductor resolve, BACKGROUND-classed — never the row's declared
    // head compared against a peer, never a network hint. `Err` and `Ok(None)`
    // are both non-authoritative answers: nothing to heal from, and nothing
    // that could license a write.
    let head = match conductor_writes::call_resolve_content_head_classed(hc, id, SWEEP_PROBE_CLASS)
        .await
    {
        Ok(Some(head)) => head,
        Ok(None) => {
            tracing::debug!(
                content_id = %id,
                "pointer_audit: own conductor holds no head record for this id — skipping"
            );
            return PointerAuditOutcome::Unreadable;
        }
        Err(e) => {
            tracing::debug!(
                content_id = %id, error = %e,
                "pointer_audit: own-conductor resolve failed — skipping this sweep"
            );
            return PointerAuditOutcome::Error;
        }
    };

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(content_id = %id, error = %e, "pointer_audit: db conn for local read");
            return PointerAuditOutcome::Error;
        }
    };
    let local_declared = match content_diesel::declared_head_for(&mut conn, ctx, id) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(content_id = %id, error = %e, "pointer_audit: declared-head read failed");
            return PointerAuditOutcome::Error;
        }
    };
    let row_blob_hash = match content_diesel::blob_hash_for(&mut conn, ctx, id) {
        Ok(v) => v,
        Err(e) => {
            // A pointer we cannot read is a pointer we do not heal — same
            // rule `adopt_local`'s T7 branch applies. Not a hard error: the
            // head answer was fine, only the local pointer read failed.
            tracing::debug!(
                content_id = %id, error = %e,
                "pointer_audit: could not read the row's blob pointer — leaving it as is"
            );
            None
        }
    };

    let (healed_cid, size_bytes) = match decide(
        false,
        head.canonical,
        local_declared.as_deref(),
        head.head_action_hash.as_str(),
        row_blob_hash.as_deref(),
        head.content.blob_cid.as_deref(),
        head.content.content_size_bytes,
    ) {
        PointerAuditDecision::Skip(outcome) => {
            if verdict_is_rememberable(
                outcome,
                head.canonical,
                local_declared.as_deref(),
                head.head_action_hash.as_str(),
            ) {
                // The pair read AFTER the resolve, i.e. the pair `decide` was
                // given: exactly what the verdict is
                // about. A blob pointer that could not be read is `None` here
                // and remembers nothing.
                if let (Some(declared), Some(blob)) =
                    (local_declared.as_deref(), row_blob_hash.as_deref())
                {
                    settled.settle(id, declared, blob);
                }
            }
            return outcome;
        }
        PointerAuditDecision::Heal(cid, size) => (cid, size),
    };

    tracing::warn!(
        target: "elohim_storage::head_adoption",
        content_id = %id,
        head = %head.head_action_hash,
        from = ?row_blob_hash,
        to = %healed_cid,
        size_bytes = size_bytes,
        "pointer-heal (T7, pointer_audit sweep): the own conductor's record for the head this \
         row ALREADY declares names a different blob — refreshing the pointer and its length \
         together; the declared head and dht_anchor_hash do not move"
    );

    let patch = content_diesel::ContentProjectionPatch {
        blob_cid: Some(healed_cid),
        content_size_bytes: Some(size_bytes),
        ..Default::default()
    };
    match content_diesel::stamp_declared_head_mode(
        &mut conn,
        ctx,
        id,
        head.head_action_hash.as_str(),
        Some(head.declared_at),
        Some(patch),
        // NEVER `Declare`: this sweep must only refresh a declared row's
        // pointer through the same guarded path `adopt_local`'s T7 branch
        // uses. `HealCanonical` is the mode that both licenses a same-head
        // value-field refresh AND refuses to move an already-declared head
        // without proof of forward ordering — see `pointer_heal_patch`'s own
        // condition 2, which this call's `head_action_hash` argument already
        // satisfied by construction (it is the SAME hash `local_declared`
        // was checked against).
        StampMode::HealCanonical,
        head.canonical_ordering(),
    ) {
        Ok(_) => PointerAuditOutcome::Healed,
        Err(e) => {
            tracing::warn!(content_id = %id, error = %e, "pointer_audit: stamp failed");
            PointerAuditOutcome::Error
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_settled_row_is_remembered_only_at_the_exact_pair_it_was_judged_on() {
        let settled = SettledPointers::default();
        assert!(
            !settled.is_settled("row", HEAD_A, "blob-1"),
            "nothing remembered yet"
        );
        settled.settle("row", HEAD_A, "blob-1");
        assert!(settled.is_settled("row", HEAD_A, "blob-1"));
        // THE TEAR THE AUDIT EXISTS FOR: the row's pointer was rewritten under
        // the same declared head. The memory must miss, so the row is asked.
        assert!(!settled.is_settled("row", HEAD_A, "blob-2"));
        // The row's head moved: a different record, never judged.
        assert!(!settled.is_settled("row", HEAD_B, "blob-1"));
        assert!(!settled.is_settled("other", HEAD_A, "blob-1"));
        // A later verdict replaces the pair rather than accumulating.
        settled.settle("row", HEAD_B, "blob-2");
        assert!(!settled.is_settled("row", HEAD_A, "blob-1"));
        assert_eq!(settled.len(), 1);
    }

    #[test]
    fn only_a_canonical_answer_for_the_declared_head_is_rememberable() {
        use crate::metrics::PointerAuditOutcome::*;
        assert!(verdict_is_rememberable(InStep, true, Some(HEAD_A), HEAD_A));
        // `decide` also answers InStep when the row declares a head OTHER than
        // the one resolved. That can change with no write to the row (the
        // conductor's head moves again), so it is never remembered.
        assert!(!verdict_is_rememberable(InStep, true, Some(HEAD_A), HEAD_B));
        assert!(!verdict_is_rememberable(InStep, true, None, HEAD_A));
        assert!(!verdict_is_rememberable(
            InStep,
            false,
            Some(HEAD_A),
            HEAD_A
        ));
        for outcome in [
            Healed,
            NotCanonical,
            Unreadable,
            Error,
            HeldByReleaseChannel,
        ] {
            assert!(
                !verdict_is_rememberable(outcome, true, Some(HEAD_A), HEAD_A),
                "{outcome:?} must be asked again every lap"
            );
        }
    }

    #[test]
    fn rows_that_left_the_candidate_set_are_forgotten() {
        let settled = SettledPointers::default();
        settled.settle("kept", HEAD_A, "b");
        settled.settle("gone", HEAD_A, "b");
        settled.retain_only(&["kept"].into_iter().collect());
        assert!(settled.is_settled("kept", HEAD_A, "b"));
        assert!(!settled.is_settled("gone", HEAD_A, "b"));
        assert!(!settled.is_empty());
    }

    #[test]
    fn the_sweep_checks_its_memory_before_it_asks_the_conductor() {
        let src = include_str!("pointer_audit.rs");
        let body = src
            .split("pub async fn run_once(")
            .nth(1)
            .expect("run_once");
        let check = body
            .find("row_is_still_settled(pool, ctx, id, settled)")
            .expect("check");
        let ask = body
            .find("heal_one(pool, hc, ctx, id, settled).await")
            .expect("ask");
        assert!(check < ask);
        assert!(body[check..ask].contains("continue;"));
    }

    #[test]
    fn stats_fold_every_outcome_into_its_own_counter() {
        let mut stats = PointerAuditStats::default();
        for outcome in [
            crate::metrics::PointerAuditOutcome::Healed,
            crate::metrics::PointerAuditOutcome::InStep,
            crate::metrics::PointerAuditOutcome::NotCanonical,
            crate::metrics::PointerAuditOutcome::Unreadable,
            crate::metrics::PointerAuditOutcome::Error,
            crate::metrics::PointerAuditOutcome::HeldByReleaseChannel,
        ] {
            record(&mut stats, outcome);
        }
        assert_eq!(stats.healed, 1);
        assert_eq!(stats.in_step, 1);
        assert_eq!(stats.not_canonical, 1);
        assert_eq!(stats.unreadable, 1);
        assert_eq!(stats.error, 1);
        assert_eq!(stats.held_by_release_channel, 1);
    }

    /// LIVE RED, alpha 2026-10-07: a slug bound to a release channel declares
    /// the legacy head the pipeline once stamped, and that head's record names
    /// the OLD blob — everything else here says "heal". The vehicle moved the
    /// pointer to the release's bytes at 21:07:51Z; this sweep put the old
    /// blob back at 21:08:41Z. A bound slug is held before anything else is
    /// weighed, and counted on its own label.
    #[test]
    fn a_slug_a_release_channel_owns_is_held_whatever_its_own_head_says() {
        assert_eq!(
            decide(
                true,
                true,
                Some(HEAD_A),
                HEAD_A,
                Some("sha256-released-by-the-vehicle"),
                Some("sha256-named-by-the-legacy-head"),
                Some(42),
            ),
            PointerAuditDecision::Skip(crate::metrics::PointerAuditOutcome::HeldByReleaseChannel)
        );
    }

    #[test]
    fn batch_and_cadence_are_bounded_and_nonzero() {
        const {
            assert!(
                POINTER_AUDIT_BATCH > 0,
                "must bound conductor load per sweep"
            );
            assert!(POINTER_AUDIT_TICK_SECONDS > 0, "must not busy-loop");
        }
    }

    #[test]
    fn sweep_probe_class_is_background_never_interactive() {
        // The whole reason this sweep gets its own const rather than reusing
        // `call_resolve_content_head`'s default: a background sweep must
        // never queue on the interactive admission lane a person's read is
        // standing in.
        assert_eq!(SWEEP_PROBE_CLASS, AdmissionClass::Background);
    }

    /// LIVE RED, story 1.4d: a row declares head A and its own conductor's
    /// record for head A names a DIFFERENT blob (the 2026-09-22 seam-smoke
    /// tear). This is what the sweep exists to heal — the pointer AND its
    /// size move together, and nothing about the decision touches the head.
    #[test]
    fn a_torn_declared_row_is_healed_pointer_and_size_move_together() {
        assert_eq!(
            decide(
                false,
                true,
                Some(HEAD_A),
                HEAD_A,
                Some("sha256-stale"),
                Some("sha256-fresh"),
                Some(4096),
            ),
            PointerAuditDecision::Heal("sha256-fresh".to_string(), 4096)
        );
    }

    /// A row whose own conductor's record names a DIFFERENT head than the
    /// row declares is a genuine head divergence — `head_adoption`'s other
    /// paths own that, not this sweep. `pointer_heal_patch`'s condition 2
    /// refuses it, and the sweep must fold that refusal into `InStep`
    /// (nothing to do here), never into a heal.
    #[test]
    fn a_row_whose_record_names_a_different_head_is_untouched() {
        assert_eq!(
            decide(
                false,
                true,
                Some(HEAD_A),
                HEAD_B,
                Some("sha256-stale"),
                Some("sha256-fresh"),
                Some(4096),
            ),
            PointerAuditDecision::Skip(crate::metrics::PointerAuditOutcome::InStep)
        );
    }

    /// A non-canonical (root-author fallback) answer must never license a
    /// write, however well everything else lines up — and it is counted on
    /// its own label rather than the general no-op bucket.
    #[test]
    fn a_non_canonical_answer_is_untouched_and_counted_separately() {
        assert_eq!(
            decide(
                false,
                false,
                Some(HEAD_A),
                HEAD_A,
                Some("sha256-stale"),
                Some("sha256-fresh"),
                Some(4096),
            ),
            PointerAuditDecision::Skip(crate::metrics::PointerAuditOutcome::NotCanonical)
        );
    }

    /// An already-converged row (record names the pointer the row already
    /// carries) writes nothing — a converged corpus heals zero pointers per
    /// sweep, same contract as `pointer_heal_patch` itself.
    #[test]
    fn an_already_converged_row_is_in_step() {
        assert_eq!(
            decide(
                false,
                true,
                Some(HEAD_A),
                HEAD_A,
                Some("sha256-fresh"),
                Some("sha256-fresh"),
                Some(4096),
            ),
            PointerAuditDecision::Skip(crate::metrics::PointerAuditOutcome::InStep)
        );
    }
}

/// DB-level tests for [`content_diesel::list_declared_blob_pointer_candidates`]:
/// candidate selection and the round-robin keyset cursor. A separate `mod`
/// (rather than `tests` above) because it needs its own in-memory schema,
/// mirroring `db::content_diesel::tests::setup_test_db` — that helper is
/// private to its own module and cannot be reused across the crate boundary.
#[cfg(test)]
mod candidate_query_tests {
    use super::*;
    use diesel::sql_types::{Nullable, Text};
    use diesel::sqlite::SqliteConnection;
    use diesel::{Connection, RunQueryDsl};

    fn setup_test_db() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory database");
        diesel::sql_query(
            r#"
            CREATE TABLE content (
                id TEXT PRIMARY KEY NOT NULL,
                h_app_id TEXT NOT NULL DEFAULT 'lamad',
                title TEXT NOT NULL,
                description TEXT,
                content_type TEXT NOT NULL DEFAULT 'concept',
                content_format TEXT NOT NULL DEFAULT 'markdown',
                content_body TEXT,
                blob_hash TEXT,
                server_blob_hash TEXT,
                blob_cid TEXT,
                content_size_bytes INTEGER,
                metadata_json TEXT,
                reach TEXT NOT NULL DEFAULT 'public',
                validation_status TEXT NOT NULL DEFAULT 'valid',
                created_by TEXT,
                dht_anchor_hash TEXT,
                p2p_published_at TEXT,
                crdt_converged_at TEXT,
                declared_head_action_hash TEXT,
                declared_head_at BIGINT,
                canonical_declared_at BIGINT,
                canonical_earned INTEGER,
                canonical_link_hash TEXT,
                dht_anchor_state TEXT,
                dht_anchor_checked_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&mut conn)
        .expect("Failed to create content table");
        conn
    }

    fn insert_row(
        conn: &mut SqliteConnection,
        id: &str,
        declared_head: Option<&str>,
        blob_cid: Option<&str>,
    ) {
        diesel::sql_query(
            "INSERT INTO content (id, h_app_id, title, declared_head_action_hash, blob_cid) \
             VALUES (?, 'lamad', ?, ?, ?)",
        )
        .bind::<Text, _>(id)
        .bind::<Text, _>(id)
        .bind::<Nullable<Text>, _>(declared_head)
        .bind::<Nullable<Text>, _>(blob_cid)
        .execute(conn)
        .expect("insert row");
    }

    /// A row with no blob pointer is never a candidate, whether or not it
    /// declares a head — and an undeclared row with a pointer is not a
    /// candidate either (nothing this sweep could exactly-equality-match
    /// against). Only the declared ∩ pointer-bearing intersection qualifies.
    #[test]
    fn a_row_with_null_blob_cid_is_never_a_candidate() {
        let mut conn = setup_test_db();
        let ctx = AppContext::default_lamad();
        insert_row(&mut conn, "declared-no-pointer", Some(HEAD_A), None);
        insert_row(
            &mut conn,
            "declared-with-pointer",
            Some(HEAD_A),
            Some("bafkrei-fresh"),
        );
        insert_row(
            &mut conn,
            "undeclared-with-pointer",
            None,
            Some("bafkrei-fresh"),
        );

        let candidates =
            content_diesel::list_declared_blob_pointer_candidates(&mut conn, &ctx, 10, None)
                .expect("query succeeds");
        assert_eq!(candidates, vec!["declared-with-pointer".to_string()]);
    }

    /// The round-robin cursor: a bounded page per sweep, resumed strictly
    /// after the last id, wrapping to the beginning once the page comes back
    /// shorter than the batch — so every candidate is visited over
    /// successive sweeps rather than the same head of the id space forever.
    #[test]
    fn the_cursor_visits_every_row_across_sweeps_then_wraps() {
        let mut conn = setup_test_db();
        let ctx = AppContext::default_lamad();
        for i in 0..5 {
            insert_row(
                &mut conn,
                &format!("row-{i}"),
                Some(HEAD_A),
                Some("bafkrei-fresh"),
            );
        }

        let mut cursor: Option<String> = None;
        let mut visited: Vec<String> = Vec::new();
        for _ in 0..3 {
            let page = content_diesel::list_declared_blob_pointer_candidates(
                &mut conn,
                &ctx,
                2,
                cursor.as_deref(),
            )
            .expect("query succeeds");
            let exhausted = page.len() < 2;
            visited.extend(page.iter().cloned());
            cursor = if exhausted {
                None
            } else {
                page.last().cloned()
            };
        }

        assert_eq!(
            visited,
            vec!["row-0", "row-1", "row-2", "row-3", "row-4"],
            "every candidate visited exactly once across the sweeps"
        );
        assert!(
            cursor.is_none(),
            "the set is exhausted — the caller wraps to the beginning on the next tick"
        );
    }
}
