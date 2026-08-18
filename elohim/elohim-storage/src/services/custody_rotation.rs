//! Custody-commitment ROTATION — author a successor pledge when the bytes a
//! custody promise names stop being the bytes the content points at.
//!
//! ## The missing station
//!
//! Every other station in the custody chain is wired. The manifest re-stamps
//! itself when a blob rotates ([`crate::services::shard_manifest_backfill`]
//! Part B); self-held possession is recorded only when the local pantry really
//! holds the bytes ([`crate::services::self_stewardship`]); the fold classifies
//! a (holder, blob) pair as `Stocked` when an ACTIVE `custody-blob` commitment
//! names that blob AND locally-witnessed evidence exists
//! ([`crate::services::custody_facing::load_custody_observation_relation`]);
//! and the gauge publishes the counts. What nothing did was **author a
//! successor commitment naming the CURRENT blob when the old one rotated
//! underneath the pledge**.
//!
//! The seeder says so in its own words (`genesis/seeder/src/seed-commitments.ts`):
//! "a server-bundle re-upload mints a new serverBlobHash and therefore a NEW
//! commitment id; the old pledge is not superseded automatically (that reconcile
//! pass remains out of scope)". This module is that reconcile pass.
//!
//! ## The shape
//!
//! - [`select_rotation_candidates`] is the pure detection half: it reads, it
//!   decides, it writes nothing. Every examined commitment produces exactly one
//!   counted outcome.
//! - [`run_rotation_pass`] is the acting half: gate on local possession, author
//!   the successor through the injected [`RotationAuthor`], then retire the
//!   predecessor through the projection-layer supersession ceremony
//!   ([`crate::db::rea_commitments::mark_superseded`]).
//! - [`ConductorRotationAuthor`] is the production author: the successor goes
//!   through the NOTARIZED path
//!   ([`crate::services::rea_commitment_service::ReaCommitmentService::create`]
//!   → `conductor_writes::call_create_rea_commitment`), never a local SQL
//!   insert.
//!
//! ## Honest absence (C4)
//!
//! A commitment whose `metadata_json` carries no resolvable `contentId` is
//! SKIPPED and counted — never matched to a content row by guesswork. There is
//! no fallback that infers "which content did this pledge mean"; a wrong guess
//! would author custody of the wrong bytes, which is worse than a visible gap.
//!
//! ## The artifact role is load-bearing
//!
//! A custody pledge can name either of a content's two artifacts: the browser
//! bundle (`content.blob_hash`) or the SSR server bundle
//! (`content.server_blob_hash`), discriminated by `metadata.artifactRole`
//! (`"ssr-server"`). Comparing an ssr-server pledge against `blob_hash` would
//! read as divergent every single tick and rotate it onto the WRONG artifact,
//! so the role selects the column — and the successor carries the role forward
//! so the next rotation still knows which artifact it pledges.
//!
//! Category A/projection: the successor is a notarized DHT commitment; the
//! predecessor's `superseded` state is projection-layer current-view
//! materialization (see [`crate::db::rea_commitments::SUPERSEDED_STATE`]).

use diesel::prelude::*;
use diesel::SqliteConnection;

use crate::db::context::AppContext;
use crate::db::models::ReaCommitment;
use crate::error::StorageError;
use crate::metrics::CustodyRotationSkip;
use crate::reconcile::custody::LocalBlobStore;

/// `eq_any` chunk size — mirrors [`crate::services::custody_facing`]'s reasoning
/// (stay well under `SQLITE_MAX_VARIABLE_NUMBER`).
const IN_CHUNK: usize = 400;

/// Cadence of the rotation tick. A blob rotates on deploy, not on traffic, so a
/// five-minute reconcile is comfortably faster than the thing it watches while
/// costing one indexed query per tick on a node with no drift.
pub const ROTATION_TICK_SECONDS: u64 = 300;

/// Which of a content row's two blob columns a custody pledge names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactRole {
    /// The browser bundle — `content.blob_hash`. The default when a pledge
    /// records no role (every pre-role commitment).
    Content,
    /// The SSR server bundle — `content.server_blob_hash`.
    SsrServer,
}

impl ArtifactRole {
    /// The seeder's wire value for this role (`metadata.artifactRole`).
    pub fn as_str(self) -> &'static str {
        match self {
            ArtifactRole::Content => "content",
            ArtifactRole::SsrServer => "ssr-server",
        }
    }

    /// Parse a recorded `artifactRole`. Anything that is not the explicit
    /// server-bundle marker is the browser bundle — including absent, which is
    /// what every pre-role pledge carries.
    fn from_metadata(raw: Option<&str>) -> Self {
        match raw {
            Some("ssr-server") => ArtifactRole::SsrServer,
            _ => ArtifactRole::Content,
        }
    }
}

/// One custody pledge that has drifted off the bytes its content now points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationCandidate {
    /// The predecessor commitment to retire once the successor is notarized.
    pub old_commitment_id: String,
    /// `metadata_json.contentId` — the content whose blob rotated.
    pub content_id: String,
    /// Preserved verbatim from the predecessor (this node, gated by the
    /// caller's `self_agent_cids`).
    pub provider: String,
    /// Preserved verbatim from the predecessor — rotation re-pledges the SAME
    /// relationship over new bytes; it never re-points custody at a new party.
    pub receiver: String,
    /// The blob the content points at RIGHT NOW, for this artifact role.
    pub current_blob_hash: String,
    /// Every classification the predecessor named — the markers this rotation
    /// supersedes. Kept whole (not just the first) so the audit trail records
    /// what was actually promised.
    pub superseded_blob_markers: Vec<String>,
    /// Which artifact the pledge names; carried onto the successor.
    pub artifact_role: ArtifactRole,
    /// Deterministic id of the successor — `(provider, receiver, new blob)`.
    pub successor_id: String,
    /// `false` when the successor already exists (a prior pass authored it but
    /// failed to supersede the predecessor): the pass skips authoring and only
    /// converges the supersession, so that residual state cannot park forever
    /// (the C3 liveness leg).
    pub author_needed: bool,
}

/// Authors the successor commitment for one candidate.
///
/// A seam, not a convenience: the production impl needs a live conductor, so
/// [`run_rotation_pass`] stays unit-testable by injecting a double. Takes the
/// connection because a notarized create projects eagerly (and the successor
/// must be ACTIVATED before the custody fold can see it — see
/// [`ConductorRotationAuthor`]).
pub trait RotationAuthor: Send + Sync {
    fn author_rotation_successor(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        candidate: &RotationCandidate,
    ) -> Result<(), StorageError>;
}

/// What one rotation pass did. Every field is a counted decision outcome.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RotationOutcome {
    /// Active self-provided `custody-blob` rows examined.
    pub examined: u32,
    /// Successors authored AND predecessors superseded.
    pub rotated: u32,
    /// Candidates whose current bytes are not in the local pantry.
    pub bytes_absent: u32,
    /// Authoring calls that failed (retried next tick).
    pub author_failed: u32,
    /// Supersession marks that failed after a successful author.
    pub supersede_failed: u32,
    /// Residual states converged: the successor already existed (authored on a
    /// prior pass whose supersession failed) and this pass retired the
    /// predecessor without authoring anything.
    pub converged: u32,
}

/// Build the successor's `metadata_json`.
///
/// `origin` marks the producer, `supersedes` makes the chain walkable via
/// `GET /api/v1/commitments/{id}` (the same steward-authored pointer the
/// project-epr re-grant ceremony uses), `contentId` keeps the pledge
/// re-resolvable by the NEXT rotation, and `artifactRole` keeps it pointed at
/// the right one of the content's two artifacts.
fn successor_metadata(candidate: &RotationCandidate) -> String {
    serde_json::json!({
        "origin": "rotation",
        "supersedes": candidate.old_commitment_id,
        "contentId": candidate.content_id,
        "blobMarker": candidate.current_blob_hash,
        "artifactRole": candidate.artifact_role.as_str(),
    })
    .to_string()
}

/// Read `metadata_json.contentId` + `metadata_json.artifactRole`.
///
/// `None` when the metadata is absent, unparseable, not an object, or carries
/// no string `contentId` — all four are the same honest answer: *we do not know
/// which content this pledge meant*, and we will not guess.
fn content_binding(commitment: &ReaCommitment) -> Option<(String, ArtifactRole)> {
    let raw = commitment.metadata_json.as_deref()?;
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let object = value.as_object()?;
    let content_id = object.get("contentId")?.as_str()?;
    if content_id.is_empty() {
        return None;
    }
    let role = ArtifactRole::from_metadata(object.get("artifactRole").and_then(|v| v.as_str()));
    Some((content_id.to_string(), role))
}

/// The blob a content row points at RIGHT NOW for `role`, or `None` when the
/// row is absent or the column is NULL/empty.
fn current_blob_for(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    content_id: &str,
    role: ArtifactRole,
) -> Result<Option<String>, StorageError> {
    use crate::db::diesel_schema::content::dsl as c;

    let row: Option<(Option<String>, Option<String>)> = c::content
        .filter(c::h_app_id.eq(h_app_id))
        .filter(c::id.eq(content_id))
        .select((c::blob_hash, c::server_blob_hash))
        .first::<(Option<String>, Option<String>)>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("custody rotation: content lookup: {e}")))?;

    let Some((blob_hash, server_blob_hash)) = row else {
        return Ok(None);
    };
    let picked = match role {
        ArtifactRole::Content => blob_hash,
        ArtifactRole::SsrServer => server_blob_hash,
    };
    Ok(picked.filter(|h| !h.trim().is_empty()))
}

/// The `state` of the commitment row with this id, if the row exists.
///
/// State matters, not bare existence: authoring is create-then-activate, and a
/// crash between the two leaves a `created` row the custody fold cannot see.
/// Treating that row as an authored successor would retire the predecessor
/// while the replacement promise never takes effect — a terminal strand.
fn successor_state(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    id: &str,
) -> Result<Option<String>, StorageError> {
    use crate::db::diesel_schema::rea_commitments::dsl as rc;

    rc::rea_commitments
        .filter(rc::h_app_id.eq(h_app_id))
        .filter(rc::id.eq(id))
        .select(rc::state)
        .first::<String>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("custody rotation: successor lookup: {e}")))
}

/// Detect the active self-provided `custody-blob` pledges whose named bytes are
/// no longer the bytes their content points at.
///
/// Pure in the sense that matters: it reads and decides, and mutates nothing but
/// the outcome counters. Every examined row lands in exactly one bucket —
/// candidate, or a counted [`CustodyRotationSkip`].
///
/// A pledge is a candidate iff ALL of:
/// 1. `state = 'active'`, `action = 'custody-blob'`, `provider ∈ self_agent_cids`;
/// 2. its metadata resolves a `contentId` (else `NoContentId`);
/// 3. that content row exists in this scope and its blob column for the pledge's
///    artifact role is non-empty (else `ContentBlobAbsent`);
/// 4. that current blob appears in NONE of the pledge's `resource_classified_as`
///    entries (else `NotDivergent` — the promise is already current);
/// 5. when the deterministic successor id for `(provider, receiver, current
///    blob)` already exists **active**, the candidate is still emitted with
///    `author_needed: false` (and `SuccessorExists` counted): a prior pass
///    authored the successor but failed to supersede the predecessor, and that
///    residual must converge rather than park forever (C3). Re-running mints
///    nothing either way (C6b) — the healthy fully-rotated state never reaches
///    this arm because a superseded predecessor is not `active`. A successor
///    row stuck in `created` (activate failed mid-authoring) keeps
///    `author_needed: true`: the idempotent author finishes the activation
///    rather than minting a duplicate, and the predecessor is never retired
///    behind a successor the custody fold cannot see.
pub fn select_rotation_candidates(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    self_agent_cids: &[String],
) -> Result<Vec<RotationCandidate>, StorageError> {
    use crate::db::diesel_schema::rea_commitments::dsl as rc;

    if self_agent_cids.is_empty() {
        return Ok(Vec::new());
    }

    let mut rows: Vec<ReaCommitment> = Vec::new();
    for chunk in self_agent_cids.chunks(IN_CHUNK) {
        let mut got: Vec<ReaCommitment> = rc::rea_commitments
            .filter(rc::h_app_id.eq(h_app_id))
            .filter(rc::action.eq("custody-blob"))
            .filter(rc::state.eq("active"))
            .filter(rc::provider.eq_any(chunk))
            .order_by(rc::id.asc())
            .load::<ReaCommitment>(conn)
            .map_err(|e| {
                StorageError::Database(format!("custody rotation: load custody-blob rows: {e}"))
            })?;
        rows.append(&mut got);
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows.dedup_by(|a, b| a.id == b.id);

    let mut candidates = Vec::new();
    for commitment in rows {
        let Some((content_id, artifact_role)) = content_binding(&commitment) else {
            crate::metrics::inc_custody_rotation_skipped(CustodyRotationSkip::NoContentId);
            tracing::debug!(
                target: "custody_rotation",
                commitment = %commitment.id,
                "custody pledge carries no resolvable contentId — skipped (never guessed)"
            );
            continue;
        };

        let Some(current_blob) = current_blob_for(conn, h_app_id, &content_id, artifact_role)?
        else {
            crate::metrics::inc_custody_rotation_skipped(CustodyRotationSkip::ContentBlobAbsent);
            continue;
        };

        let markers = commitment.classifications();
        if markers.iter().any(|m| m == &current_blob) {
            crate::metrics::inc_custody_rotation_skipped(CustodyRotationSkip::NotDivergent);
            continue;
        }

        let successor_id = crate::services::rea_commitment_service::deterministic_custody_id(
            &commitment.provider,
            &commitment.receiver,
            &current_blob,
        );
        let author_needed = match successor_state(conn, h_app_id, &successor_id)?.as_deref() {
            Some("active") => {
                // Successor authored AND activated (a prior pass's supersession
                // failed): still a candidate, but only the supersession remains
                // to converge.
                crate::metrics::inc_custody_rotation_skipped(CustodyRotationSkip::SuccessorExists);
                false
            }
            // A `created`-stuck row (activate failed mid-authoring) is NOT an
            // authored successor — route it back through the author, whose
            // create-if-absent-then-activate contract finishes the half-done
            // authoring instead of minting a duplicate.
            Some(_) | None => true,
        };

        candidates.push(RotationCandidate {
            old_commitment_id: commitment.id.clone(),
            content_id,
            provider: commitment.provider.clone(),
            receiver: commitment.receiver.clone(),
            current_blob_hash: current_blob,
            superseded_blob_markers: markers,
            artifact_role,
            successor_id,
            author_needed,
        });
    }

    Ok(candidates)
}

/// Run one rotation pass: detect, author, supersede.
///
/// Safe no-op when `self_agent_cids` is empty, when nothing has diverged, or
/// when this node does not hold the current bytes — a node never pledges
/// custody of bytes it cannot serve (the same rule
/// [`crate::services::self_stewardship`] applies to evidence).
///
/// Bounded (C6a): one pass over the node's own active custody pledges, no retry
/// ladder — a failed author or supersede is counted and retried on the next
/// tick.
pub fn run_rotation_pass(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    self_agent_cids: &[String],
    local_store: &dyn LocalBlobStore,
    author: &dyn RotationAuthor,
) -> Result<RotationOutcome, StorageError> {
    let candidates = select_rotation_candidates(conn, &ctx.h_app_id, self_agent_cids)?;
    let mut outcome = RotationOutcome {
        examined: candidates.len() as u32,
        ..Default::default()
    };

    for candidate in candidates {
        // The bytes gate guards AUTHORING a fresh pledge; converging a
        // supersession whose successor is already notarized has no byte
        // precondition — the promise over the new bytes already stands.
        if candidate.author_needed {
            if !local_store.has(&candidate.current_blob_hash) {
                outcome.bytes_absent += 1;
                crate::metrics::inc_custody_rotation_skipped(CustodyRotationSkip::BytesAbsent);
                tracing::debug!(
                    target: "custody_rotation",
                    content_id = %candidate.content_id,
                    blob = %candidate.current_blob_hash,
                    "custody rotation deferred: local pantry does not hold the current bytes \
                     (never pledge custody of bytes we cannot serve)"
                );
                continue;
            }

            if let Err(e) = author.author_rotation_successor(conn, ctx, &candidate) {
                outcome.author_failed += 1;
                crate::metrics::inc_custody_rotation_skipped(CustodyRotationSkip::AuthorFailed);
                tracing::warn!(
                    target: "custody_rotation",
                    error = %e,
                    content_id = %candidate.content_id,
                    successor = %candidate.successor_id,
                    "custody rotation: successor authoring failed (retry next tick)"
                );
                continue;
            }
        }

        // Projection-layer supersession: both rows stay notarized DHT entries;
        // `superseded` is the current-view materialization. `mark_superseded`
        // re-checks the not-already-superseded invariant inside its own
        // transaction (first-write-wins) and is idempotent.
        if let Err(e) =
            crate::db::rea_commitments::mark_superseded(conn, ctx, &candidate.old_commitment_id)
        {
            outcome.supersede_failed += 1;
            crate::metrics::inc_custody_rotation_skipped(CustodyRotationSkip::SupersedeFailed);
            tracing::warn!(
                target: "custody_rotation",
                error = %e,
                predecessor = %candidate.old_commitment_id,
                "custody rotation: successor authored but predecessor not superseded"
            );
            continue;
        }

        if candidate.author_needed {
            outcome.rotated += 1;
            crate::metrics::inc_custody_rotation_authored();
            tracing::info!(
                target: "custody_rotation",
                content_id = %candidate.content_id,
                predecessor = %candidate.old_commitment_id,
                successor = %candidate.successor_id,
                blob = %candidate.current_blob_hash,
                superseded_markers = ?candidate.superseded_blob_markers,
                "custody rotation: successor pledge authored, predecessor superseded"
            );
        } else {
            outcome.converged += 1;
            tracing::info!(
                target: "custody_rotation",
                content_id = %candidate.content_id,
                predecessor = %candidate.old_commitment_id,
                successor = %candidate.successor_id,
                "custody rotation: residual converged — successor already notarized, \
                 predecessor now superseded"
            );
        }
    }

    Ok(outcome)
}

/// Production [`RotationAuthor`]: notarizes the successor through the conductor
/// and ACTIVATES it.
///
/// ## Why activation is not optional
///
/// The DNA's `create_rea_commitment` coordinator stamps every fresh commitment
/// `state: "created"`
/// (`holochain/dna/elohim/zomes/content_store/src/lib.rs`). The custody fold
/// only reads `state = 'active'` rows
/// ([`crate::services::custody_facing`]'s commitment plane), so a successor that
/// is authored and left alone changes NOTHING the gauge can see. This mirrors
/// [`crate::services::rea_commitment_service::ReaCommitmentService::ensure_active_self_custody`],
/// which has always done create-then-activate for exactly this reason.
///
/// ## Sync trait over async conductor calls
///
/// Same bridge as
/// [`crate::services::salvage_commitment_author::SalvageCommitmentAuthor`]:
/// `block_in_place` + `Handle::block_on`, so the detection pass stays sync and
/// unit-testable. Must be called from a tokio runtime.
pub struct ConductorRotationAuthor {
    hc: std::sync::Arc<crate::hc_client::HcClient>,
}

impl ConductorRotationAuthor {
    pub fn new(hc: std::sync::Arc<crate::hc_client::HcClient>) -> Self {
        Self { hc }
    }
}

impl RotationAuthor for ConductorRotationAuthor {
    fn author_rotation_successor(
        &self,
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        candidate: &RotationCandidate,
    ) -> Result<(), StorageError> {
        use crate::db::rea_commitments::{CreateReaCommitmentInput, UpdateReaCommitmentState};
        use crate::services::rea_commitment_service::ReaCommitmentService;

        let input = CreateReaCommitmentInput {
            id: Some(candidate.successor_id.clone()),
            action: "custody-blob".to_string(),
            provider: candidate.provider.clone(),
            receiver: candidate.receiver.clone(),
            resource_conforms_to: Some("blob".to_string()),
            resource_classified_as: Some(candidate.current_blob_hash.clone()),
            resource_quantity_unit: Some("B".to_string()),
            note: Some(format!(
                "rotation: {} re-pledges custody of {} for {} (supersedes {})",
                candidate.provider,
                candidate.current_blob_hash,
                candidate.receiver,
                candidate.old_commitment_id
            )),
            metadata_json: Some(successor_metadata(candidate)),
            ..Default::default()
        };

        let handle = tokio::runtime::Handle::try_current().map_err(|_| {
            StorageError::Internal(
                "custody rotation author: no tokio runtime handle (must be called from an \
                 async context)"
                    .to_string(),
            )
        })?;
        let hc = self.hc.clone();
        let successor_id = candidate.successor_id.clone();

        // Idempotent by state, so a crash between create and activate converges
        // on the next tick instead of stranding a `created` row forever (the
        // detector routes such rows back here with `author_needed: true`).
        let existing = successor_state(conn, &ctx.h_app_id, &successor_id)?;

        tokio::task::block_in_place(|| {
            handle.block_on(async move {
                if existing.is_none() {
                    // custody-blob is a CONDUCTOR_SOFT_ACTION: with a bridge
                    // present this routes to create_via_conductor →
                    // conductor_writes::call_create_rea_commitment (NOTARIZED),
                    // and eagerly projects the row with its dht_anchor_hash.
                    ReaCommitmentService::create(conn, ctx, input, None, Some(&hc)).await?;
                }
                if existing.as_deref() != Some("active") {
                    let activate = UpdateReaCommitmentState {
                        state: "active".to_string(),
                        finished: None,
                        metadata_json: None,
                    };
                    ReaCommitmentService::update_state(
                        conn,
                        ctx,
                        &successor_id,
                        &activate,
                        None,
                        Some(&hc),
                    )
                    .await?;
                }
                Ok::<(), StorageError>(())
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{NewContent, NewReaCommitment, NewShardManifest};
    use crate::metrics::CustodyRotationSkip;
    use crate::test_util::test_pool;

    const APP: &str = "lamad";
    const SELF: &str = "uhCAkSelfAgent";
    const STEWARD: &str = "uhCAkStewardAgent";
    /// The bytes the content points at now.
    const NEW_BLOB: &str = "sha256-93ecnewblob";
    /// The bytes the seeded pledge still names.
    const OLD_BLOB: &str = "sha256-7ce8oldblob";

    fn ctx() -> AppContext {
        AppContext::new(APP)
    }

    fn selves() -> Vec<String> {
        vec![SELF.to_string()]
    }

    fn skip_count(reason: CustodyRotationSkip) -> u64 {
        crate::metrics::custody_rotation_skipped_count(reason)
    }

    /// The skip counters are process-global, and by design EVERY examined row
    /// lands in some counted bucket — so a delta-window assertion is only sound
    /// while no concurrently-running test bumps counters. Any test that opens a
    /// counter window, or that runs detection at all, holds this guard.
    static COUNTER_WINDOW: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn counter_window() -> std::sync::MutexGuard<'static, ()> {
        COUNTER_WINDOW.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A `LocalBlobStore` fake holding an explicit hash list.
    struct FakeStore(Vec<String>);
    impl LocalBlobStore for FakeStore {
        fn has(&self, hash: &str) -> bool {
            self.0.iter().any(|h| h == hash)
        }
    }

    fn holding_new_blob() -> FakeStore {
        FakeStore(vec![NEW_BLOB.to_string()])
    }

    /// Records what it was asked to author and projects an ACTIVE successor row
    /// — the end state the conductor author reaches (notarize + activate).
    #[derive(Default)]
    struct RecordingAuthor {
        authored: std::sync::Mutex<Vec<RotationCandidate>>,
        fail: bool,
    }
    impl RotationAuthor for RecordingAuthor {
        fn author_rotation_successor(
            &self,
            conn: &mut SqliteConnection,
            ctx: &AppContext,
            candidate: &RotationCandidate,
        ) -> Result<(), StorageError> {
            if self.fail {
                return Err(StorageError::Internal("author refused".into()));
            }
            self.authored.lock().unwrap().push(candidate.clone());
            // Mirror the conductor author's idempotent contract: create only if
            // absent, then activate whatever state the row is in.
            if successor_state(conn, &ctx.h_app_id, &candidate.successor_id)
                .expect("successor lookup")
                .is_none()
            {
                insert_commitment(
                    conn,
                    &candidate.successor_id,
                    &candidate.provider,
                    &candidate.receiver,
                    &candidate.current_blob_hash,
                    "active",
                    Some(&successor_metadata(candidate)),
                    &ctx.h_app_id,
                );
            } else {
                use crate::db::diesel_schema::rea_commitments::dsl as rc;
                diesel::update(
                    rc::rea_commitments
                        .filter(rc::h_app_id.eq(&ctx.h_app_id))
                        .filter(rc::id.eq(&candidate.successor_id)),
                )
                .set(rc::state.eq("active"))
                .execute(conn)
                .expect("activate successor");
            }
            Ok(())
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_commitment(
        conn: &mut SqliteConnection,
        id: &str,
        provider: &str,
        receiver: &str,
        blob: &str,
        state: &str,
        metadata_json: Option<&str>,
        h_app_id: &str,
    ) {
        use crate::db::diesel_schema::rea_commitments;
        diesel::insert_into(rea_commitments::table)
            .values(&NewReaCommitment {
                id,
                h_app_id,
                action: "custody-blob",
                provider,
                receiver,
                resource_conforms_to: Some("blob"),
                resource_classified_as: Some(blob),
                resource_quantity_value: None,
                resource_quantity_unit: Some("B"),
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_beginning: None,
                has_end: None,
                due: None,
                clause_of: None,
                in_scope_of: None,
                medium_of_exchange_id: None,
                state,
                finished: 0,
                note: None,
                metadata_json,
                dht_anchor_hash: Some("uhCkk-anchor"),
            })
            .execute(conn)
            .expect("insert commitment");
    }

    /// The seeded shape: an ACTIVE pledge naming `OLD_BLOB` for `content_id`.
    fn seed_stale_pledge(conn: &mut SqliteConnection, id: &str, content_id: &str) {
        let metadata = serde_json::json!({
            "seedGeneration": "genesis",
            "blobHash": OLD_BLOB,
            "contentId": content_id,
        })
        .to_string();
        insert_commitment(
            conn,
            id,
            SELF,
            STEWARD,
            OLD_BLOB,
            "active",
            Some(&metadata),
            APP,
        );
    }

    fn insert_content(
        conn: &mut SqliteConnection,
        id: &str,
        blob_hash: Option<&str>,
        server_blob_hash: Option<&str>,
    ) {
        use crate::db::diesel_schema::content;
        diesel::insert_into(content::table)
            .values(&NewContent {
                id,
                h_app_id: APP,
                title: id,
                description: None,
                content_type: "concept",
                content_format: "html5-app",
                blob_hash,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: "commons",
                created_by: None,
                content_body: None,
                dht_anchor_hash: None,
                server_blob_hash,
            })
            .execute(conn)
            .expect("insert content");
    }

    fn state_of(conn: &mut SqliteConnection, id: &str) -> String {
        crate::db::rea_commitments::get_commitment(conn, &ctx(), id)
            .expect("load")
            .expect("row present")
            .state
    }

    // ---- the counted vocabulary --------------------------------------------

    /// Label strings are a dashboard contract (C8): re-pin deliberately, and
    /// migrate any panel keyed on the old string in the same change.
    #[test]
    fn rotation_skip_labels_are_stable() {
        seam_contracts::assert_reason_labels_stable::<CustodyRotationSkip>(&[
            "no_content_id",
            "content_blob_absent",
            "not_divergent",
            "successor_exists",
            "bytes_absent",
            "author_failed",
            "supersede_failed",
        ]);
    }

    // ---- detection ---------------------------------------------------------

    #[test]
    fn divergent_active_pledge_is_a_rotation_candidate() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");

        let found = select_rotation_candidates(&mut conn, APP, &selves()).expect("detect");

        assert_eq!(
            found.len(),
            1,
            "the drifted pledge is exactly one candidate"
        );
        let c = &found[0];
        assert_eq!(c.old_commitment_id, "custody-blob-stale");
        assert_eq!(c.content_id, "c1");
        assert_eq!(c.provider, SELF);
        assert_eq!(
            c.receiver, STEWARD,
            "rotation re-pledges the SAME relationship"
        );
        assert_eq!(c.current_blob_hash, NEW_BLOB);
        assert_eq!(c.superseded_blob_markers, vec![OLD_BLOB.to_string()]);
        assert_eq!(c.artifact_role, ArtifactRole::Content);
        assert_eq!(
            c.successor_id,
            crate::services::rea_commitment_service::deterministic_custody_id(
                SELF, STEWARD, NEW_BLOB
            ),
        );
    }

    #[test]
    fn pledge_naming_the_current_blob_is_not_a_candidate() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        let metadata = serde_json::json!({ "contentId": "c1" }).to_string();
        insert_commitment(
            &mut conn,
            "custody-blob-current",
            SELF,
            STEWARD,
            NEW_BLOB,
            "active",
            Some(&metadata),
            APP,
        );

        let _w = counter_window();
        let before = skip_count(CustodyRotationSkip::NotDivergent);
        let found = select_rotation_candidates(&mut conn, APP, &selves()).expect("detect");

        assert!(found.is_empty(), "a current pledge must never rotate");
        assert_eq!(
            skip_count(CustodyRotationSkip::NotDivergent),
            before + 1,
            "the non-divergent decision is counted, not silent"
        );
    }

    #[test]
    fn pledge_without_a_resolvable_content_id_is_skipped_and_counted() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        // (a) no metadata at all, (b) metadata that is not JSON, (c) metadata
        // with no contentId key. All three are the SAME honest answer.
        insert_commitment(
            &mut conn, "cb-a", SELF, STEWARD, OLD_BLOB, "active", None, APP,
        );
        insert_commitment(
            &mut conn,
            "cb-b",
            SELF,
            STEWARD,
            OLD_BLOB,
            "active",
            Some("not json at all"),
            APP,
        );
        insert_commitment(
            &mut conn,
            "cb-c",
            SELF,
            STEWARD,
            OLD_BLOB,
            "active",
            Some(r#"{"blobHash":"sha256-7ce8oldblob"}"#),
            APP,
        );

        let _w = counter_window();
        let before = skip_count(CustodyRotationSkip::NoContentId);
        let found = select_rotation_candidates(&mut conn, APP, &selves()).expect("detect");

        assert!(
            found.is_empty(),
            "an unbound pledge is never matched to a content row by guesswork"
        );
        assert_eq!(
            skip_count(CustodyRotationSkip::NoContentId),
            before + 3,
            "each unbound pledge is counted (honest absence, C4)"
        );
    }

    #[test]
    fn pledge_for_missing_content_or_empty_blob_is_skipped_and_counted() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // "c-gone" has no content row; "c-empty" has a row with a NULL blob.
        insert_content(&mut conn, "c-empty", None, None);
        seed_stale_pledge(&mut conn, "cb-gone", "c-gone");
        seed_stale_pledge(&mut conn, "cb-empty", "c-empty");

        let _w = counter_window();
        let before = skip_count(CustodyRotationSkip::ContentBlobAbsent);
        let found = select_rotation_candidates(&mut conn, APP, &selves()).expect("detect");

        assert!(found.is_empty());
        assert_eq!(
            skip_count(CustodyRotationSkip::ContentBlobAbsent),
            before + 2
        );
    }

    /// An ssr-server pledge is compared against `server_blob_hash`, never the
    /// browser bundle — otherwise it would read divergent every tick and rotate
    /// onto the wrong artifact.
    #[test]
    fn ssr_server_pledge_tracks_the_server_blob_column() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), Some("sha256-serverNEW"));
        let metadata = serde_json::json!({
            "contentId": "c1",
            "artifactRole": "ssr-server",
        })
        .to_string();
        insert_commitment(
            &mut conn,
            "cb-ssr",
            SELF,
            STEWARD,
            "sha256-serverOLD",
            "active",
            Some(&metadata),
            APP,
        );

        let found = select_rotation_candidates(&mut conn, APP, &selves()).expect("detect");

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].current_blob_hash, "sha256-serverNEW");
        assert_eq!(found[0].artifact_role, ArtifactRole::SsrServer);
    }

    #[test]
    fn non_self_provider_and_non_active_rows_are_invisible() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        let metadata = serde_json::json!({ "contentId": "c1" }).to_string();
        // Another peer's pledge — not ours to rotate.
        insert_commitment(
            &mut conn,
            "cb-other",
            "uhCAkOtherPeer",
            STEWARD,
            OLD_BLOB,
            "active",
            Some(&metadata),
            APP,
        );
        // Already retired.
        insert_commitment(
            &mut conn,
            "cb-superseded",
            SELF,
            STEWARD,
            OLD_BLOB,
            "superseded",
            Some(&metadata),
            APP,
        );

        let found = select_rotation_candidates(&mut conn, APP, &selves()).expect("detect");
        assert!(found.is_empty());
    }

    #[test]
    fn existing_successor_makes_detection_an_idempotent_no_op() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");
        let successor_id = crate::services::rea_commitment_service::deterministic_custody_id(
            SELF, STEWARD, NEW_BLOB,
        );
        insert_commitment(
            &mut conn,
            &successor_id,
            SELF,
            STEWARD,
            NEW_BLOB,
            "active",
            None,
            APP,
        );

        let _w = counter_window();
        let before = skip_count(CustodyRotationSkip::SuccessorExists);
        let found = select_rotation_candidates(&mut conn, APP, &selves()).expect("detect");

        // The residual (successor notarized, predecessor still active) is a
        // CONVERGENCE-ONLY candidate — never an authoring one (C6b: replay
        // mints nothing; C3: the residual must not park forever).
        assert_eq!(found.len(), 1);
        assert!(!found[0].author_needed, "authoring already happened");
        assert_eq!(found[0].successor_id, successor_id);
        assert_eq!(skip_count(CustodyRotationSkip::SuccessorExists), before + 1);
    }

    /// A successor stuck in `created` (create succeeded, activate failed) is NOT
    /// an authored successor: the custody fold reads only `state = 'active'`
    /// rows, so treating the bare row as done would retire the predecessor while
    /// the promise it replaces never takes effect — a terminal strand.
    #[test]
    fn stuck_created_successor_still_needs_authoring() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");
        let successor_id = crate::services::rea_commitment_service::deterministic_custody_id(
            SELF, STEWARD, NEW_BLOB,
        );
        insert_commitment(
            &mut conn,
            &successor_id,
            SELF,
            STEWARD,
            NEW_BLOB,
            "created",
            None,
            APP,
        );

        let _w = counter_window();
        let found = select_rotation_candidates(&mut conn, APP, &selves()).expect("detect");
        assert_eq!(found.len(), 1);
        assert!(
            found[0].author_needed,
            "a created-but-never-activated successor must route back through the author"
        );
    }

    /// The pass-level guarantee for the same residue: the stuck successor is
    /// activated (not re-created) BEFORE the predecessor retires, so no tick
    /// ordering can leave the node with zero effective custody pledges.
    #[test]
    fn stuck_created_successor_is_activated_not_stranded() {
        let _w = counter_window();
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");
        let successor_id = crate::services::rea_commitment_service::deterministic_custody_id(
            SELF, STEWARD, NEW_BLOB,
        );
        insert_commitment(
            &mut conn,
            &successor_id,
            SELF,
            STEWARD,
            NEW_BLOB,
            "created",
            None,
            APP,
        );

        let author = RecordingAuthor::default();
        let outcome = run_rotation_pass(&mut conn, &ctx(), &selves(), &holding_new_blob(), &author)
            .expect("pass");

        assert_eq!(outcome.rotated, 1);
        assert_eq!(
            state_of(&mut conn, &successor_id),
            "active",
            "the stuck successor must be activated, not left 'created'"
        );
        assert_eq!(state_of(&mut conn, "custody-blob-stale"), "superseded");
    }

    /// The C3 liveness leg: author-succeeded-supersede-failed residue converges
    /// on the next pass — predecessor retired WITHOUT re-authoring, with no
    /// byte precondition (the promise over the new bytes already stands) — and
    /// the state then settles (a further pass examines nothing).
    #[test]
    fn residual_successor_without_supersession_converges_and_then_settles() {
        let _w = counter_window();
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");
        let successor_id = crate::services::rea_commitment_service::deterministic_custody_id(
            SELF, STEWARD, NEW_BLOB,
        );
        insert_commitment(
            &mut conn,
            &successor_id,
            SELF,
            STEWARD,
            NEW_BLOB,
            "active",
            None,
            APP,
        );

        let author = RecordingAuthor::default();
        // Deliberately EMPTY pantry: convergence must not gate on bytes.
        let outcome = run_rotation_pass(&mut conn, &ctx(), &selves(), &FakeStore(vec![]), &author)
            .expect("pass");

        assert!(
            author.authored.lock().unwrap().is_empty(),
            "convergence must not re-author"
        );
        assert_eq!(outcome.converged, 1);
        assert_eq!(outcome.rotated, 0);
        assert_eq!(outcome.bytes_absent, 0);

        let predecessor =
            crate::db::rea_commitments::get_commitment(&mut conn, &ctx(), "custody-blob-stale")
                .expect("query")
                .expect("row");
        assert_eq!(predecessor.state, "superseded");

        // Settled: nothing active diverges any more — replay examines nothing.
        let again = run_rotation_pass(&mut conn, &ctx(), &selves(), &FakeStore(vec![]), &author)
            .expect("pass 2");
        assert_eq!(
            again.examined, 0,
            "settled state mints and converges nothing"
        );
    }

    #[test]
    fn no_self_identity_is_a_safe_no_op() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");

        assert!(select_rotation_candidates(&mut conn, APP, &[])
            .expect("detect")
            .is_empty());
    }

    // ---- the pass ----------------------------------------------------------

    #[test]
    fn rotation_pass_authors_the_successor_and_supersedes_first_write_wins() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");
        let author = RecordingAuthor::default();

        let outcome = run_rotation_pass(&mut conn, &ctx(), &selves(), &holding_new_blob(), &author)
            .expect("pass");

        assert_eq!(outcome.examined, 1);
        assert_eq!(outcome.rotated, 1);
        assert_eq!(author.authored.lock().unwrap().len(), 1);
        assert_eq!(
            state_of(&mut conn, "custody-blob-stale"),
            crate::db::rea_commitments::SUPERSEDED_STATE,
            "the predecessor is retired once the successor is notarized"
        );
        let successor_id = crate::services::rea_commitment_service::deterministic_custody_id(
            SELF, STEWARD, NEW_BLOB,
        );
        assert_eq!(state_of(&mut conn, &successor_id), "active");

        // Second pass: the successor exists and the predecessor is superseded —
        // nothing further is authored (first-write-wins; at most one
        // non-superseded successor per predecessor).
        let again = run_rotation_pass(&mut conn, &ctx(), &selves(), &holding_new_blob(), &author)
            .expect("pass");
        assert_eq!(again.rotated, 0);
        assert_eq!(
            author.authored.lock().unwrap().len(),
            1,
            "a replayed pass mints no second successor"
        );
    }

    #[test]
    fn rotation_pass_authors_nothing_when_the_bytes_are_absent_locally() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");
        let author = RecordingAuthor::default();

        let _w = counter_window();
        let before = skip_count(CustodyRotationSkip::BytesAbsent);
        let outcome = run_rotation_pass(
            &mut conn,
            &ctx(),
            &selves(),
            &FakeStore(vec!["sha256-somethingelse".to_string()]),
            &author,
        )
        .expect("pass");

        assert_eq!(outcome.bytes_absent, 1);
        assert_eq!(outcome.rotated, 0);
        assert!(
            author.authored.lock().unwrap().is_empty(),
            "never pledge custody of bytes we do not hold"
        );
        assert_eq!(skip_count(CustodyRotationSkip::BytesAbsent), before + 1);
        assert_eq!(
            state_of(&mut conn, "custody-blob-stale"),
            "active",
            "a deferred rotation must not retire the standing promise"
        );
    }

    #[test]
    fn a_failed_author_leaves_the_predecessor_standing() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");
        let author = RecordingAuthor {
            fail: true,
            ..Default::default()
        };

        let outcome = run_rotation_pass(&mut conn, &ctx(), &selves(), &holding_new_blob(), &author)
            .expect("pass");

        assert_eq!(outcome.author_failed, 1);
        assert_eq!(outcome.rotated, 0);
        assert_eq!(state_of(&mut conn, "custody-blob-stale"), "active");
    }

    // ---- end-to-end: the fold reports stocked ------------------------------

    /// The whole ch07 chain in one test: a rotated pledge plus locally-witnessed
    /// possession of the current blob's shard makes the custody fold report
    /// `stocked >= 1`. Before rotation the same fixture reports zero stocked —
    /// that assertion is the finish line this module moves.
    #[test]
    fn rotation_makes_the_custody_fold_report_stocked() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_content(&mut conn, "c1", Some(NEW_BLOB), None);
        seed_stale_pledge(&mut conn, "custody-blob-stale", "c1");

        // encoding "none": the single shard IS the whole blob, so
        // shard_hash == blob_hash (services::self_stewardship's rationale).
        let manifest = NewShardManifest {
            content_id: "c1",
            h_app_id: APP,
            blob_hash: NEW_BLOB,
            blob_cid: None,
            encoding: "none",
            data_shard_count: 1,
            parity_shard_count: 0,
            shard_hashes_json: &serde_json::to_string(&[NEW_BLOB]).unwrap(),
            total_size_bytes: 0,
            shard_size_bytes: 0,
            mime_type: "html5-app",
            reach: "commons",
        };
        crate::db::shard_manifests::upsert_manifest(&mut conn, &manifest).expect("manifest");

        // Locally-witnessed possession, recorded only because the bytes are here.
        let store = holding_new_blob();
        assert_eq!(
            crate::services::self_stewardship::record_self_held_shard(
                &mut conn, &store, NEW_BLOB, SELF, APP
            )
            .expect("record"),
            crate::services::self_stewardship::SelfHeldOutcome::Recorded,
        );

        let now_micros = chrono::Utc::now().timestamp_micros();
        let shards = vec![NEW_BLOB.to_string()];

        // BEFORE rotation: the pledge names the old blob, so the fold sees a
        // measured absence — witnessed bytes, no matching promise.
        let before = elohim_facings::folds::operational_weave::observed_custody_class_counts(
            &crate::services::custody_facing::load_custody_observation_relation(
                &mut conn, APP, SELF, &shards, now_micros, 86_400,
            )
            .expect("relation"),
        );
        assert_eq!(
            before.stocked, 0,
            "precondition: the stale pledge cannot make custody stocked"
        );

        run_rotation_pass(
            &mut conn,
            &ctx(),
            &selves(),
            &store,
            &RecordingAuthor::default(),
        )
        .expect("pass");

        let after = elohim_facings::folds::operational_weave::observed_custody_class_counts(
            &crate::services::custody_facing::load_custody_observation_relation(
                &mut conn, APP, SELF, &shards, now_micros, 86_400,
            )
            .expect("relation"),
        );
        assert!(
            after.stocked >= 1,
            "rotation + witnessed possession must fold to stocked, got {after:?}"
        );
    }
}
