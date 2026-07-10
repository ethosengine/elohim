//! Content-projection producer — the organ that lights the Automerge content-sync plane.
//!
//! The CRDT sync engine (`/elohim/storage-sync/1.0.0`) is fully wired end-to-end
//! but inert: nothing fills the DocStore, so every sync round round-trips empty.
//! This module adds the go-forward producer: a second `EventBus` subscriber
//! (alongside `spawn_logging_listener`) that turns each content write into an
//! Automerge document in the DocStore under the `"elohim"` sync namespace.

use std::sync::Arc;

use automerge::transaction::Transactable;
use automerge::{Automerge, ReadDoc, ScalarValue, Value};
use tokio::sync::broadcast::error::RecvError;

use crate::db::context::AppContext;
use crate::db::models::Content;
use crate::db::{content_diesel, DbPool};
use crate::error::StorageError;
use crate::services::events::{EventBus, StorageEvent};
use crate::sync::SyncManager;

/// The canonical content-node doc-id for a content row.
pub fn content_doc_id(id: &str) -> String {
    format!("node:{id}")
}

/// Gate for the cold-start corpus back-fill (`ELOHIM_DOCSTORE_BACKFILL`).
///
/// Default **ON** — the back-fill is a reconciliation-controller leg (SQL corpus
/// = desired state, DocStore = converged state; eager reconcile, P1) and it is
/// idempotent, so every cold start converges the DocStore with zero per-env
/// config. A peer that never projects its corpus strands every pre-producer row
/// out of the sync plane (the elohim.host `App not found` class). Only an
/// explicit `0`/`false` opts out (escape hatch for constrained disks).
pub fn backfill_enabled(env_val: Option<&str>) -> bool {
    !matches!(
        env_val.map(str::trim),
        Some(v) if v == "0" || v.eq_ignore_ascii_case("false")
    )
}

/// Broadcast-tier allowlist for the content-sync plane — FAIL-CLOSED.
///
/// The sync plane serves `ListDocuments`/`SyncChanges` to ANY connected peer
/// with no receiver-side reach pre-authorization (unlike the shard plane's
/// `reach_authorization`), and the projected doc carries the full content
/// body. So only the broadcast tiers of the DNA-notarized reach enum
/// (`community`/`public`/`commons`) may enter the plane; the private family
/// (`private`/`self`/`intimate`) and the relationship-scoped tiers
/// (`trusted`/`familiar`) must not — nor may UNKNOWN values (the reach
/// vocabulary has known 3-way drift; an unrecognized value could be a
/// drifted private tier, so unknown = excluded). Live corpus check
/// 2026-07-01: 574 `public` + 5 `commons`, zero excluded rows — the
/// fail-closed posture costs nothing today. Receiver-authorized sync for
/// scoped tiers is a separate arc (rides the shard plane's auth model).
pub fn reach_is_distribution_safe(reach: &str) -> bool {
    matches!(
        reach.trim().to_ascii_lowercase().as_str(),
        "community" | "public" | "commons"
    )
}

/// One projected field value — a string or an integer scalar. Kept as an enum
/// so the projection WRITE and the idempotency COMPARE share a single field
/// list (`projected_fields`) and can never drift.
enum FieldVal {
    S(String),
    I(i64),
}

/// The ordered field set projected from a content row into its Automerge doc —
/// the single source of truth for both the write and the idempotency compare
/// (`doc_matches` checks exactly this list; keys NOT in the list are "don't
/// care", so a converged peer value a local row can't produce is never fought).
///
/// `blobHash`/`serverBlobHash`/`blobCid`/`contentSizeBytes` are LOAD-BEARING for
/// peer convergence of the SERVING path: a peer whose content row lost its
/// `blobHash` (e.g. a deploy PATCH that never landed on a degraded conductor) can
/// only re-derive it by converging this doc from a healthy peer.
///
/// Two invariants keep a FLEET-WIDE backfill safe (every peer projects its own
/// local corpus concurrently):
///
/// 1. **Empty-never-projects.** A serving field this peer doesn't know is
///    ABSENT from its projection, never `""`/`0`. Projecting an empty value
///    enters LWW competition against a healthy peer's real hash — and can WIN
///    the merge (actor-id ordering), poisoning the fleet's converged value.
///    The consumer mirrors this as empty-never-wins (`reverse_project_content_doc`).
///    Deliberate consequence: a set→unset transition does not propagate; serving
///    fields only ever move unset→set or set→set (deploy re-stages).
/// 2. **No peer-local metadata.** `updatedAt` is NOT projected: two peers seeded
///    at different times hold different values for identical content, so
///    projecting it would make every peer's backfill append a ping-pong change
///    per doc per restart (unbounded history inflation). Causality lives in the
///    CRDT history itself.
fn projected_fields(content: &Content) -> Vec<(&'static str, FieldVal)> {
    let mut fields = vec![
        ("id", FieldVal::S(content.id.clone())),
        ("hAppId", FieldVal::S(content.h_app_id.clone())),
        ("title", FieldVal::S(content.title.clone())),
        ("contentType", FieldVal::S(content.content_type.clone())),
        ("contentFormat", FieldVal::S(content.content_format.clone())),
        ("reach", FieldVal::S(content.reach.clone())),
    ];
    // Nullable content fields follow the same absent-not-empty rule as the
    // serving fields: a NULL/default value projects as ABSENCE, never as
    // ""/"{}" — an empty put deterministically erases a peer's real value.
    if let Some(d) = &content.description {
        if !d.is_empty() {
            fields.push(("description", FieldVal::S(d.clone())));
        }
    }
    if let Some(b) = &content.content_body {
        if !b.is_empty() {
            fields.push(("body", FieldVal::S(b.clone())));
        }
    }
    if let Some(m) = &content.metadata_json {
        if !m.is_empty() && m != "{}" {
            fields.push(("metadata", FieldVal::S(m.clone())));
        }
    }
    for (key, val) in [
        ("blobHash", &content.blob_hash),
        ("serverBlobHash", &content.server_blob_hash),
        ("blobCid", &content.blob_cid),
    ] {
        if let Some(v) = val {
            if !v.is_empty() {
                fields.push((key, FieldVal::S(v.clone())));
            }
        }
    }
    if let Some(size) = content.content_size_bytes {
        if size > 0 {
            fields.push(("contentSizeBytes", FieldVal::I(size as i64)));
        }
    }
    // `headActionHash` — an OBSERVABILITY/HINT scalar for peers (Plan C2,
    // scalar variant): converges what the author's node last DECLARED as this
    // content id's version-DAG head. Same absent-not-empty rule as the serving
    // fields (an empty put would erase a peer's real hint via LWW). It is a
    // HINT only, never an authority signal: any peer can put any bytes in a
    // CRDT doc, so consumers must never treat the converged value as
    // notarization — see the REQ-N5 guard on `reverse_project_content_doc`.
    if let Some(h) = &content.declared_head_action_hash {
        if !h.is_empty() {
            fields.push(("headActionHash", FieldVal::S(h.clone())));
        }
    }
    fields
}

/// The version-DAG key for a content row's current serving version (Plan C2).
/// A "version" is addressed by its serving bytes: the blob's content hash IS the
/// `versionCid`. Two peers that authored DISTINCT bytes for the same id therefore
/// hold DISTINCT version keys — so both versions COEXIST in the grow-only
/// `versions` map (never an LWW clobber), and HEAD-election (Plan C3) picks which
/// is canonical. Absent/empty `blobHash` ⇒ no serving version to record yet.
fn head_version_cid(content: &Content) -> Option<String> {
    content
        .blob_hash
        .as_deref()
        .filter(|b| !b.is_empty())
        .map(str::to_string)
}

/// Read a top-level string scalar from a doc, or None.
fn root_str(doc: &Automerge, key: &str) -> Option<String> {
    match doc.get(automerge::ROOT, key) {
        Ok(Some((Value::Scalar(s), _))) => match s.as_ref() {
            ScalarValue::Str(smol) => Some(smol.to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// Resolve the head version's blobHash from a projected doc:
/// `head → versions[head] → blobHash`, falling back to the flat ROOT `blobHash`
/// for docs written by a pre-C2 peer (the dual-write compat leg). Returns None if
/// neither is present. This is the C2 READER half — consumed by the serving /
/// HEAD-election path (Plan C3) via [`crate::sync::SyncManager::declared_head_blob`],
/// the C3 read side that lets `/apps` serve the notary-declared head's bytes.
pub(crate) fn read_head_blob_hash(doc: &Automerge) -> Option<String> {
    if let Some(head) = root_str(doc, "head") {
        if let Ok(Some((Value::Object(automerge::ObjType::Map), versions_id))) =
            doc.get(automerge::ROOT, "versions")
        {
            if let Ok(Some((Value::Scalar(s), _))) = doc.get(&versions_id, &head) {
                if let ScalarValue::Str(smol) = s.as_ref() {
                    return Some(smol.to_string());
                }
            }
        }
    }
    // Fallback: flat ROOT blobHash (pre-C2 dual-write leg).
    root_str(doc, "blobHash")
}

/// Read the doc's `headActionHash` observability/hint scalar (Plan C2) — the
/// author's DECLARED version-DAG head action, projected by `projected_fields`.
///
/// This is the REQ-F4 anti-laundering gate input for the C3 read side
/// ([`crate::sync::SyncManager::declared_head_blob`]): a serving node honors a
/// doc's converged head blob ONLY when this hint matches the row's own
/// `declared_head_action_hash`, proving the doc is projecting THE declared head
/// rather than a peer's un-elected recency head. Returns `None` when the doc
/// carries no declared head (pre-C2 / never-declared).
pub(crate) fn doc_head_action_hash(doc: &Automerge) -> Option<String> {
    root_str(doc, "headActionHash")
}

/// Whether the doc's version-DAG already records `version_cid` as head with its
/// entry present in the grow-only `versions` map — the C2 idempotency leg, so a
/// re-projection of an unchanged head appends no change.
fn version_dag_current(doc: &Automerge, version_cid: &str) -> bool {
    if root_str(doc, "head").as_deref() != Some(version_cid) {
        return false;
    }
    match doc.get(automerge::ROOT, "versions") {
        Ok(Some((Value::Object(automerge::ObjType::Map), versions_id))) => {
            matches!(doc.get(&versions_id, version_cid), Ok(Some(_)))
        }
        _ => false,
    }
}

/// True when the doc ALREADY carries exactly the projected field set — the
/// idempotency guard. A missing field, a changed value, or a scalar-type
/// mismatch each mean "needs (re)projection". A freshly-created empty doc has no
/// fields, so it never matches → always projects on first sight.
fn doc_matches(doc: &Automerge, fields: &[(&'static str, FieldVal)]) -> bool {
    for (key, val) in fields {
        match doc.get(automerge::ROOT, *key) {
            Ok(Some((Value::Scalar(scalar), _))) => match (val, scalar.as_ref()) {
                (FieldVal::S(want), ScalarValue::Str(got)) if got.as_str() == want => {}
                (FieldVal::I(want), ScalarValue::Int(got)) if got == want => {}
                _ => return false,
            },
            _ => return false,
        }
    }
    true
}

/// Project a single content row into its Automerge doc under the "elohim" sync
/// namespace. Returns `true` if the doc was (re)written, `false` if it already
/// matched — an **idempotent** skip that appends no new change, so re-running the
/// producer or the corpus back-fill never inflates the doc's change history.
///
/// THE NAMESPACE IS LOAD-BEARING: `initiate_sync_round` (p2p/mod.rs:6996) only
/// lists `"elohim"`; a doc written under any other namespace sits inert forever.
/// (`PROJECTION_NAMESPACE` makes this coupling explicit — see the G4 guard.)
///
/// `SyncManager` exposes no `save`; the canonical mutate+persist idiom (proven in
/// tests/sync_integration.rs) is `apply_changes(ns, doc_id, vec![doc.save()])` —
/// the same path a peer's changes take, so a local projection and a remote merge
/// converge identically.
pub async fn project_content_doc(
    sync: &SyncManager,
    content: &Content,
) -> Result<bool, StorageError> {
    // Fail-closed reach gate — see `reach_is_distribution_safe`.
    if !reach_is_distribution_safe(&content.reach) {
        return Ok(false);
    }
    let doc_id = content_doc_id(&content.id);
    let mut doc = sync
        .get_or_create_doc(PROJECTION_NAMESPACE, &doc_id)
        .await?;
    let fields = projected_fields(content);
    let head_vcid = head_version_cid(content);
    // Idempotency now spans BOTH legs: the flat field set AND the version-DAG head
    // (a doc whose flat fields match but whose grow-only `versions`/`head` is not
    // yet current must still (re)project to record the version).
    let dag_current = match &head_vcid {
        Some(vcid) => version_dag_current(&doc, vcid),
        None => true, // no serving version yet ⇒ no version-DAG obligation
    };
    if doc_matches(&doc, &fields) && dag_current {
        return Ok(false);
    }
    doc.transact::<_, _, automerge::AutomergeError>(|tx| {
        for (key, val) in &fields {
            match val {
                FieldVal::S(s) => tx.put(automerge::ROOT, *key, s.as_str())?,
                FieldVal::I(i) => tx.put(automerge::ROOT, *key, *i)?,
            }
        }
        // Version-DAG leg (Plan C2): record this serving version in the grow-only
        // `versions` map keyed by its content-address, and move `head` to it. The
        // flat ROOT `blobHash` above is the dual-write compat leg for pre-C2 peers;
        // grow-only means a divergent peer version is ADDED, never overwriting.
        if let Some(vcid) = &head_vcid {
            let versions_id = match tx.get(automerge::ROOT, "versions")? {
                Some((Value::Object(automerge::ObjType::Map), id)) => id,
                _ => tx.put_object(automerge::ROOT, "versions", automerge::ObjType::Map)?,
            };
            tx.put(&versions_id, vcid.as_str(), vcid.as_str())?;
            tx.put(automerge::ROOT, "head", vcid.as_str())?;
        }
        Ok(())
    })
    .map_err(|e| StorageError::Sync(format!("projector transact failed: {e:?}")))?;
    sync.apply_changes(PROJECTION_NAMESPACE, &doc_id, vec![doc.save()])
        .await?;
    Ok(true)
}

/// Reconciliation-mode projection — the corpus back-fill's write path.
///
/// OFFER what the doc lacks; never fight what it holds. A back-fill replays
/// possibly-stale local rows causally AFTER the doc's current heads, so an
/// assert-style write from here would deterministically overwrite fresher
/// converged values (last-restarter-wins) and re-open the fight on every
/// restart of every divergent peer — unbounded history inflation plus a
/// stale hash re-asserted fleet-wide. Rule: a field is written iff the doc's
/// current value is ABSENT or EMPTY (`""`/`0` — legacy pre-guard projections
/// stay fillable); a present non-empty doc value is never contested from
/// reconciliation, whatever the local row says. Fresh authoritative writes
/// ride the EVENT path (`project_content_doc`), which asserts all fields.
pub async fn project_content_doc_reconcile(
    sync: &SyncManager,
    content: &Content,
) -> Result<bool, StorageError> {
    if !reach_is_distribution_safe(&content.reach) {
        return Ok(false);
    }
    let doc_id = content_doc_id(&content.id);
    let mut doc = sync
        .get_or_create_doc(PROJECTION_NAMESPACE, &doc_id)
        .await?;
    let fields: Vec<(&'static str, FieldVal)> = projected_fields(content)
        .into_iter()
        .filter(|(key, val)| match doc.get(automerge::ROOT, *key) {
            Ok(Some((Value::Scalar(scalar), _))) => match (val, scalar.as_ref()) {
                // Present-but-empty is fillable; equal or different-non-empty
                // is left alone (equal = no-op, different = never fight).
                (FieldVal::S(want), ScalarValue::Str(cur)) => {
                    cur.as_str().is_empty() && !want.is_empty()
                }
                (FieldVal::I(want), ScalarValue::Int(cur)) => *cur == 0 && *want != 0,
                // Scalar type mismatch: don't fight from reconciliation.
                _ => false,
            },
            // Non-scalar present: don't fight. Absent: fill.
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => false,
        })
        .collect();
    if fields.is_empty() {
        return Ok(false);
    }
    doc.transact::<_, _, automerge::AutomergeError>(|tx| {
        for (key, val) in &fields {
            match val {
                FieldVal::S(s) => tx.put(automerge::ROOT, *key, s.as_str())?,
                FieldVal::I(i) => tx.put(automerge::ROOT, *key, *i)?,
            }
        }
        Ok(())
    })
    .map_err(|e| StorageError::Sync(format!("projector reconcile transact failed: {e:?}")))?;
    sync.apply_changes(PROJECTION_NAMESPACE, &doc_id, vec![doc.save()])
        .await?;
    Ok(true)
}

/// Outcome of a corpus back-fill pass.
#[derive(Debug, Default, Clone, Copy)]
pub struct BackfillStats {
    pub scanned: u64,
    pub projected: u64,
    pub skipped: u64,
    pub failed: u64,
}

/// Back-fill the Automerge DocStore from the pre-existing SQL content corpus.
///
/// The go-forward producer (`spawn_content_projection_listener`) only projects
/// content written AFTER it starts. On a fresh deploy or a sled DocStore reset,
/// the large already-seeded corpus would never converge until each row was
/// re-written. This one-shot, **idempotent** pass closes that gap: it pages every
/// content row (all app scopes, provenance-ungated — a projection REBUILD over
/// already-notarized content, not a read surface) and projects each via
/// `project_content_doc`, which skips rows already present with matching values.
/// Safe to run on every cold start (gated at the call site).
///
/// Batched, and yields between pages so it never starves the live write path or
/// the 60s sync timer. `batch` bounds the per-page SELECT (clamped to ≥ 1).
pub async fn backfill_content_docs(
    sync: &SyncManager,
    pool: &DbPool,
    batch: i64,
) -> Result<BackfillStats, StorageError> {
    let batch = batch.max(1);
    let mut stats = BackfillStats::default();
    let mut offset: i64 = 0;
    loop {
        let rows = {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {e}")))?;
            content_diesel::list_all_content_rows(&mut conn, offset, batch)?
        };
        if rows.is_empty() {
            break;
        }
        let page_len = rows.len();
        for content in &rows {
            stats.scanned += 1;
            // Reconcile mode, NOT event mode: a back-fill replays possibly-
            // stale rows and must offer-not-fight (see the reconcile fn).
            match project_content_doc_reconcile(sync, content).await {
                Ok(true) => stats.projected += 1,
                Ok(false) => stats.skipped += 1,
                Err(e) => {
                    stats.failed += 1;
                    tracing::error!(id = %content.id, error = %e, "backfill: projection failed");
                }
            }
        }
        offset += page_len as i64;
        // Yield so a large back-fill never monopolises the runtime.
        tokio::task::yield_now().await;
        if (page_len as i64) < batch {
            break;
        }
    }
    tracing::info!(
        scanned = stats.scanned,
        projected = stats.projected,
        skipped = stats.skipped,
        failed = stats.failed,
        "backfill: corpus projection pass complete"
    );
    Ok(stats)
}

/// Reverse-project a converged content doc back into the SQL content projection —
/// the CONSUMER heal leg (the missing other half of the sync plane; today
/// `apply_changes` writes the sled DocStore ONLY). When a peer's content doc
/// converges into our DocStore, re-derive the serving-critical `blob_hash` into the
/// local SQL row at the **amber** tier, so SERVING (which reads SQL, not the
/// DocStore) heals WITHOUT the notary. Returns `true` if a heal was written.
///
/// Division of labor (spec §5.5): the shard/replication plane heals ABSENCE (a
/// missing row/bytes, via `bulk_create_content`); this heals DRIFT — a stale/null
/// `blob_hash` on a row that already EXISTS locally. An absent row is skipped
/// (`Ok(false)`) — that is the shard plane's job, not the drift-heal's.
///
/// Guards: (1) **empty-never-wins** — only heals when the converged `blob_hash` is
/// non-empty (a peer holding `""` never marks us amber or clobbers us); (2)
/// **green-inviolable** — the write rides `update_content`'s amber path
/// (`crdt_converged_at` set), which never overwrites a notarized (`dht_anchor_hash`)
/// `blob_hash` but DOES replace a non-green one, so amber rows converge set→set
/// instead of freezing on their first heal (A3 precedence: green > amber);
/// (3) **namespace** — writes under `default_lamad` (the serving scope), never
/// the doc's `"elohim"` sync namespace (REQ-F5).
///
/// # REQ-N5 — the converged `headActionHash` doc field is NEVER consumed into SQL
///
/// The doc also carries a `headActionHash` scalar (the author's declared
/// version-DAG head, projected by `projected_fields`). **Do not read it here.
/// Do not add a heal for it. Ever.** Notarization provenance —
/// `dht_anchor_hash` and `declared_head_action_hash` — is written ONLY by
/// conductor-verified paths (the `ContentCommitted` projection /
/// `upsert_with_anchor` / the declared-head stamp in `content_diesel`). The
/// converged doc value is unauthenticated peer input: any peer can put any
/// bytes into a CRDT doc, so consuming it into either column would launder
/// gossip into notarization provenance — an amber-tier hint silently promoted
/// to authority. This function heals `blobHash` ONLY (amber, `crdt_converged_at`
/// marker). If you are about to plumb another doc field into a SQL write,
/// stop: route it through a conductor-verified path instead.
/// Guard test: `converged_head_hint_is_never_stamped`.
pub async fn reverse_project_content_doc(
    sync: &SyncManager,
    pool: &DbPool,
    doc_id: &str,
) -> Result<bool, StorageError> {
    let Some(id) = doc_id.strip_prefix("node:") else {
        return Ok(false); // not a content-node doc
    };
    // Empty-never-wins: read the converged blob_hash; skip if absent/empty.
    let blob_hash = match sync
        .get_doc_field(PROJECTION_NAMESPACE, doc_id, "blobHash")
        .await
    {
        Ok(h) if !h.is_empty() => h,
        _ => return Ok(false),
    };
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("Pool error: {e}")))?;
    let ctx = AppContext::default_lamad();
    let input = content_diesel::UpdateContentInput {
        id: id.to_string(),
        blob_hash: Some(blob_hash),
        // The amber marker: switches update_content into the no-clobber amber path
        // (stamps crdt_converged_at, NEVER dht_anchor_hash).
        crdt_converged_at: Some(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    };
    match content_diesel::update_content(&mut conn, &ctx, input) {
        Ok(_) => Ok(true),
        // Absent row → the shard/replication plane heals it, not the drift-heal.
        Err(StorageError::NotFound(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Re-SELECT the full content row by id.
///
/// `StorageEvent::ContentCreated/Updated` carries only `{id, ..}`, so the
/// producer re-reads the full row. Scoped to `default_lamad()` because
/// `Services::new` (services/mod.rs) builds its `ContentService` under the
/// lamad app context — the scope content is written under. `require_provenance`
/// is `false` so pre-drain rows project immediately (mirrors the internal
/// `ContentService::get` convention).
async fn load_content_row(pool: &DbPool, id: &str) -> Result<Option<Content>, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("Pool error: {e}")))?;
    let ctx = AppContext::default_lamad();
    content_diesel::get_content(&mut conn, &ctx, id, content_diesel::MinTrust::Invisible)
}

/// Chunk size for bulk-event projection paging — mirrors the back-fill's page
/// size so a multi-thousand-id seed yields to the runtime at the same cadence
/// (a full seed can carry ~3.4k ids in a single `ContentBulkCreated`).
const BULK_PROJECTION_CHUNK: usize = 200;

/// Outcome of projecting one `ContentBulkCreated` batch. `skipped` counts the
/// idempotent no-ops (already-projected rows), which is the normal fate of a
/// partial re-seed's already-present ids — not an error.
#[derive(Debug, Default, Clone, Copy)]
struct BulkProjectionStats {
    projected: u64,
    skipped: u64,
    failed: u64,
}

/// Project a `ContentBulkCreated` batch of content ids into the sync DocStore.
///
/// Chunked + yielding so a large seed never starves the live write path or the
/// 60s sync timer (same pacing discipline as `backfill_content_docs`).
///
/// RECONCILE, NOT ASSERT: this uses `project_content_doc_reconcile` (offer-not-
/// fight), NOT the assertive `project_content_doc` — for the same reason the
/// corpus back-fill does. A `ContentBulkCreated` event re-delivers a stale local
/// row: on a re-seed of already-existing content, `bulk_create` skips the row but
/// still lists its id, so the row this handler loads may be causally OLDER than a
/// value the fleet already converged (peer N's doc for X = fresh B2 while N's SQL
/// row still holds B1). An assertive write would overwrite the converged B2 back
/// to the stale B1 at seed scale — a fleet-wide regression. Reconcile fills only
/// ABSENT/empty doc fields and never contests a present non-empty value, so a
/// genuinely-new row (empty doc) is filled exactly as an assert would, while a
/// converged doc is left intact.
///
/// TRAP the caller must not "optimise" away: `ids` covers the WHOLE input batch
/// while the event's `count` counts only the newly-inserted rows, so on a partial
/// re-seed `ids.len() != count` and already-projected ids are re-delivered here.
/// That is safe and expected — reconcile is idempotent (fills nothing when the doc
/// already carries the values), so a re-delivered id is a `skipped` no-op, never a
/// duplicate, a regression, or an error. The fail-closed reach gate lives inside
/// `project_content_doc_reconcile`, so non-broadcast rows in the batch are silently
/// declined there and land in `skipped`.
async fn project_content_bulk(
    sync: &SyncManager,
    pool: &DbPool,
    ids: &[String],
) -> BulkProjectionStats {
    let mut stats = BulkProjectionStats::default();
    for chunk in ids.chunks(BULK_PROJECTION_CHUNK) {
        for id in chunk {
            match load_content_row(pool, id).await {
                Ok(Some(content)) => match project_content_doc_reconcile(sync, &content).await {
                    Ok(true) => stats.projected += 1,
                    Ok(false) => stats.skipped += 1,
                    Err(e) => {
                        stats.failed += 1;
                        tracing::error!(%id, error = %e, "projector: bulk doc projection failed");
                    }
                },
                // A row named in the batch that no longer exists (deleted between
                // emit and projection) is a benign skip, not a failure.
                Ok(None) => {
                    stats.skipped += 1;
                    tracing::warn!(%id, "projector: bulk content row vanished");
                }
                Err(e) => {
                    stats.failed += 1;
                    tracing::error!(%id, error = %e, "projector: bulk load failed");
                }
            }
        }
        // Yield between pages so a 3k-id seed never monopolises the runtime.
        tokio::task::yield_now().await;
    }
    stats
}

/// Spawn the content-projection listener.
///
/// A second `EventBus` subscriber (mirrors `spawn_logging_listener`) that
/// projects each content write into its Automerge doc:
/// `ContentCreated`/`ContentUpdated` project the single named row inline;
/// `ContentBulkCreated` is handed to a SEPARATE spawned task. The separation is
/// load-bearing: a bulk seed can carry ~3.4k ids, and projecting them inline would
/// block this loop from draining the bounded broadcast channel (capacity 1024) —
/// concurrent `ContentCreated`s would overflow it and surface as `RecvError::Lagged`,
/// permanently dropping those events for this subscriber. The spawned bulk task
/// returns the loop to `recv()` immediately. A single-flight `Mutex` serialises
/// overlapping bulk tasks (bulk events are rare; the lock is a courtesy against
/// DocStore thrash, not a correctness requirement — reconcile-mode is idempotent).
///
/// Bulk projection uses reconcile semantics (`project_content_doc_reconcile`), so a
/// re-seed re-delivering a stale local row can never regress a value the fleet
/// already converged, and the `ContentBulkCreated` partial-re-seed trap (its `ids`
/// cover the whole input batch, not just the `count` newly-inserted rows) is a
/// no-op skip. Projection writes only the sled DocStore (the value plane); it never
/// stamps notary/anchor provenance — `dht_anchor_hash` is written exclusively by the
/// conductor-verified path (`upsert_with_anchor`).
pub fn spawn_content_projection_listener(
    events: Arc<EventBus>,
    sync: Arc<SyncManager>,
    pool: DbPool,
) -> tokio::task::JoinHandle<()> {
    let mut rx = events.subscribe();
    // Serialises overlapping bulk-projection tasks — see the doc comment.
    let bulk_guard = Arc::new(tokio::sync::Mutex::new(()));
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(StorageEvent::ContentCreated { id, .. })
                | Ok(StorageEvent::ContentUpdated { id }) => {
                    match load_content_row(&pool, &id).await {
                        Ok(Some(content)) => {
                            if let Err(e) = project_content_doc(&sync, &content).await {
                                tracing::error!(%id, error = %e, "projector: doc projection failed");
                            } else {
                                tracing::debug!(%id, "projector: content projected to sync DocStore");
                            }
                        }
                        Ok(None) => tracing::warn!(%id, "projector: content row vanished"),
                        Err(e) => tracing::error!(%id, error = %e, "projector: load failed"),
                    }
                }
                Ok(StorageEvent::ContentBulkCreated { ids, .. }) => {
                    // Spawn onto its own task so a large batch never blocks the
                    // listener draining the broadcast channel (see doc comment).
                    let sync = Arc::clone(&sync);
                    let pool = pool.clone();
                    let guard = Arc::clone(&bulk_guard);
                    tokio::spawn(async move {
                        let _lock = guard.lock().await;
                        let stats = project_content_bulk(&sync, &pool, &ids).await;
                        tracing::debug!(
                            ids = ids.len(),
                            projected = stats.projected,
                            skipped = stats.skipped,
                            failed = stats.failed,
                            "projector: bulk content projected to sync DocStore"
                        );
                    });
                }
                // Ignore everything else (non-content events).
                Ok(_) => {}
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "projector: event bus lagged");
                }
                Err(RecvError::Closed) => {
                    tracing::debug!("projector: event bus closed, stopping listener");
                    break;
                }
            }
        }
    })
}

/// The sync-partition namespace — the single source of truth for the `h_app_id`
/// that partitions content sync. BOTH sides reference this const: the producer
/// (`project_content_doc`, below) writes docs under it, and the consumer
/// (`initiate_sync_round` in p2p/mod.rs) lists documents under it. Because both
/// reference the const directly, producer/consumer drift is now compile-impossible
/// (each used to carry a bare `"elohim"` literal — a silent content-sync killer if
/// either drifted). The remaining invariant — the wire value stays `"elohim"`
/// unless every peer + the DNA migrate in lockstep — is pinned by
/// `projection_namespace_is_wire_contract`.
pub const PROJECTION_NAMESPACE: &str = "elohim";

#[cfg(test)]
mod tests {
    use crate::db::models::Content;
    use crate::sync::{DocStore, DocStoreConfig, StreamTracker, SyncManager};
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Build a SyncManager over a temp sled DocStore (mirrors
    /// tests/sync_integration.rs::create_sync_manager).
    async fn test_sync_manager() -> (SyncManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let doc_store = Arc::new(
            DocStore::new(DocStoreConfig {
                db_path: temp_dir.path().join("projector.sled"),
                ..Default::default()
            })
            .await
            .unwrap(),
        );
        let stream_tracker = Arc::new(StreamTracker::new());
        (SyncManager::new(doc_store, stream_tracker), temp_dir)
    }

    fn sample_content(id: &str, title: &str) -> Content {
        Content {
            id: id.to_string(),
            h_app_id: "lamad".to_string(),
            title: title.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: Some("{}".to_string()),
            reach: "commons".to_string(),
            validation_status: "valid".to_string(),
            created_by: None,
            created_at: "2026-06-27T00:00:00Z".to_string(),
            updated_at: "2026-06-27T00:00:00Z".to_string(),
            content_body: Some("hello".to_string()),
            dht_anchor_hash: None,
            p2p_published_at: None,
            server_blob_hash: None,
            crdt_converged_at: None,
            declared_head_action_hash: None,
        }
    }

    #[tokio::test]
    async fn producer_projects_content_create_into_docstore_under_elohim() {
        let (sync, _temp) = test_sync_manager().await;
        let content = sample_content("edit-prop-1", "v1");

        // Act: project once.
        super::project_content_doc(&sync, &content).await.unwrap();

        // Assert: doc exists under "elohim" at node:{id}, with the title and heads.
        let heads = sync
            .get_heads("elohim", "node:edit-prop-1")
            .await
            .expect("doc exists");
        assert!(!heads.is_empty(), "projected doc must have heads");

        let title = sync
            .get_doc_field("elohim", "node:edit-prop-1", "title")
            .await
            .unwrap();
        assert_eq!(title, "v1");

        // The doc_id helper is the canonical content-node address.
        assert_eq!(super::content_doc_id("edit-prop-1"), "node:edit-prop-1");
    }

    /// Pin the on-the-wire sync namespace. Producer/consumer drift is now
    /// compile-enforced — `initiate_sync_round` (p2p/mod.rs) and
    /// `project_content_doc` both reference `PROJECTION_NAMESPACE` directly, so
    /// neither can silently diverge. (The previous version of this test compared
    /// a LOCAL COPY of the `"elohim"` literal to the const and so guarded nothing:
    /// renaming the sync-timer's `h_app_id` would have passed while killing content
    /// sync fleet-wide.) What remains to guard is the WIRE VALUE: remote peers and
    /// the DNA expect `"elohim"`. Changing the const moves both sides coherently
    /// but alters the wire protocol — this fails loudly so that change is deliberate
    /// and coordinated, never an incidental rename.
    #[test]
    fn projection_namespace_is_wire_contract() {
        assert_eq!(
            super::PROJECTION_NAMESPACE,
            "elohim",
            "sync-partition namespace is a wire contract with every peer + the DNA"
        );
    }

    /// Re-projecting identical content is a no-op — no new change is appended, so
    /// the corpus back-fill (and a redundant go-forward update) never inflate a
    /// doc's change history. This is the idempotency the back-fill relies on.
    #[tokio::test]
    async fn project_is_idempotent_on_unchanged_content() {
        let (sync, _temp) = test_sync_manager().await;
        let content = sample_content("idem-1", "v1");

        assert!(
            super::project_content_doc(&sync, &content).await.unwrap(),
            "first projection must write"
        );
        let heads1 = sync.get_heads("elohim", "node:idem-1").await.unwrap();
        assert!(!heads1.is_empty());

        assert!(
            !super::project_content_doc(&sync, &content).await.unwrap(),
            "re-projecting unchanged content must be a skip"
        );
        let heads2 = sync.get_heads("elohim", "node:idem-1").await.unwrap();
        assert_eq!(
            heads1, heads2,
            "idempotent re-projection must not append a change"
        );
    }

    /// Plan C2 proof — distinct serving versions of the SAME content id COEXIST in
    /// the grow-only version-DAG (never an LWW clobber); head moves to the latest,
    /// the notary HINT (headActionHash) is carried, and the flat ROOT blobHash
    /// dual-writes head's bytes for pre-C2 readers. This coexistence is what lets
    /// HEAD-election (C3) choose a canonical head ACROSS divergent authorings
    /// instead of last-writer-wins (the elohim-host-landing divergence class).
    #[tokio::test]
    async fn distinct_versions_coexist_head_notary_set() {
        use automerge::ReadDoc;
        let (sync, _temp) = test_sync_manager().await;
        let mut content = sample_content("vdag-1", "v1");

        content.blob_hash = Some("sha256-aaa".to_string());
        content.declared_head_action_hash = Some("uhCkkAAA".to_string());
        assert!(super::project_content_doc(&sync, &content).await.unwrap());

        content.blob_hash = Some("sha256-bbb".to_string());
        content.declared_head_action_hash = Some("uhCkkBBB".to_string());
        assert!(super::project_content_doc(&sync, &content).await.unwrap());

        let doc = sync
            .get_or_create_doc(
                super::PROJECTION_NAMESPACE,
                &super::content_doc_id("vdag-1"),
            )
            .await
            .unwrap();

        // Both versions coexist in the grow-only map (A is NOT clobbered by B).
        let versions_id = match doc.get(automerge::ROOT, "versions").unwrap() {
            Some((automerge::Value::Object(automerge::ObjType::Map), id)) => id,
            _ => panic!("versions map must exist after C2 projection"),
        };
        assert!(
            doc.get(&versions_id, "sha256-aaa").unwrap().is_some(),
            "version A must remain — the version-DAG is grow-only, not LWW"
        );
        assert!(
            doc.get(&versions_id, "sha256-bbb").unwrap().is_some(),
            "version B must be present"
        );
        // Head selects the latest version; the reader resolves its blobHash.
        assert_eq!(super::root_str(&doc, "head").as_deref(), Some("sha256-bbb"));
        assert_eq!(
            super::read_head_blob_hash(&doc).as_deref(),
            Some("sha256-bbb")
        );
        // Dual-write compat: flat ROOT blobHash mirrors head's bytes.
        assert_eq!(
            super::root_str(&doc, "blobHash").as_deref(),
            Some("sha256-bbb")
        );
        // Notary HINT carried alongside (never authority — see the REQ-N5 guard).
        assert_eq!(
            super::root_str(&doc, "headActionHash").as_deref(),
            Some("uhCkkBBB")
        );
    }

    /// The C2 reader falls back to the flat ROOT blobHash for a doc written by a
    /// pre-C2 peer (no `versions` map / `head`) — the dual-write compat leg that
    /// keeps convergence working across the mixed-version window.
    #[tokio::test]
    async fn read_head_blob_hash_falls_back_to_root_for_pre_c2_doc() {
        use automerge::transaction::Transactable;
        let (sync, _temp) = test_sync_manager().await;
        let doc_id = super::content_doc_id("pre-c2");
        let mut doc = sync
            .get_or_create_doc(super::PROJECTION_NAMESPACE, &doc_id)
            .await
            .unwrap();
        doc.transact::<_, _, automerge::AutomergeError>(|tx| {
            tx.put(automerge::ROOT, "blobHash", "sha256-legacy")?;
            Ok(())
        })
        .unwrap();
        sync.apply_changes(super::PROJECTION_NAMESPACE, &doc_id, vec![doc.save()])
            .await
            .unwrap();
        let doc = sync
            .get_or_create_doc(super::PROJECTION_NAMESPACE, &doc_id)
            .await
            .unwrap();
        assert_eq!(
            super::read_head_blob_hash(&doc).as_deref(),
            Some("sha256-legacy")
        );
    }

    /// The `headActionHash` observability hint is readable off a projected doc —
    /// the input to the REQ-F4 anti-laundering gate in the C3 read side.
    #[tokio::test]
    async fn doc_head_action_hash_reads_projected_hint() {
        let (sync, _temp) = test_sync_manager().await;
        let mut content = sample_content("hint-1", "v1");
        content.blob_hash = Some("sha256-head".to_string());
        content.declared_head_action_hash = Some("uhCkkDECLARED".to_string());
        assert!(super::project_content_doc(&sync, &content).await.unwrap());

        let doc = sync
            .get_or_create_doc(
                super::PROJECTION_NAMESPACE,
                &super::content_doc_id("hint-1"),
            )
            .await
            .unwrap();
        assert_eq!(
            super::doc_head_action_hash(&doc).as_deref(),
            Some("uhCkkDECLARED")
        );
    }

    /// C3 read side (SyncManager::declared_head_blob): when the doc's
    /// `headActionHash` hint MATCHES the row's declared head, the head blob
    /// resolves — the serve path can prefer it. (declared-head-present case.)
    #[tokio::test]
    async fn declared_head_blob_returns_head_when_hint_matches() {
        let (sync, _temp) = test_sync_manager().await;
        let mut content = sample_content("dhb-1", "v1");
        content.blob_hash = Some("sha256-head".to_string());
        content.declared_head_action_hash = Some("uhCkkDECLARED".to_string());
        assert!(super::project_content_doc(&sync, &content).await.unwrap());

        let got = sync
            .declared_head_blob(
                super::PROJECTION_NAMESPACE,
                &super::content_doc_id("dhb-1"),
                "uhCkkDECLARED",
            )
            .await;
        assert_eq!(got.as_deref(), Some("sha256-head"));
    }

    /// REQ-F4 / no-laundering gate: a doc head whose `headActionHash` does NOT
    /// match the row's declared head is NEVER served — the serve path degrades to
    /// the own row rather than adopting a peer's un-elected recency head.
    #[tokio::test]
    async fn declared_head_blob_none_when_hint_mismatches() {
        let (sync, _temp) = test_sync_manager().await;
        let mut content = sample_content("dhb-2", "v1");
        content.blob_hash = Some("sha256-head".to_string());
        content.declared_head_action_hash = Some("uhCkkPEER".to_string());
        assert!(super::project_content_doc(&sync, &content).await.unwrap());

        let got = sync
            .declared_head_blob(
                super::PROJECTION_NAMESPACE,
                &super::content_doc_id("dhb-2"),
                "uhCkkMINE",
            )
            .await;
        assert_eq!(got, None, "mismatched declared-head hint must not launder");
    }

    /// A never-synced content id has no doc — the read-only resolver returns None
    /// (and, unlike `get_or_create_doc`, creates nothing).
    #[tokio::test]
    async fn declared_head_blob_none_for_missing_doc() {
        let (sync, _temp) = test_sync_manager().await;
        let got = sync
            .declared_head_blob(
                super::PROJECTION_NAMESPACE,
                &super::content_doc_id("never-synced"),
                "uhCkkX",
            )
            .await;
        assert_eq!(got, None);
    }

    /// A changed serving-critical field (blobHash) forces a re-projection and the
    /// new value converges into the doc.
    #[tokio::test]
    async fn project_rewrites_when_a_field_changes() {
        let (sync, _temp) = test_sync_manager().await;
        let mut content = sample_content("chg-1", "v1");

        assert!(super::project_content_doc(&sync, &content).await.unwrap());
        let heads1 = sync.get_heads("elohim", "node:chg-1").await.unwrap();

        content.blob_hash = Some("sha256-deadbeef".to_string());
        assert!(
            super::project_content_doc(&sync, &content).await.unwrap(),
            "a changed field must re-project"
        );
        let heads2 = sync.get_heads("elohim", "node:chg-1").await.unwrap();
        assert_ne!(heads1, heads2, "a changed field must append a change");

        let blob = sync
            .get_doc_field("elohim", "node:chg-1", "blobHash")
            .await
            .unwrap();
        assert_eq!(blob, "sha256-deadbeef");
    }

    /// The projected doc carries the serving-critical blob fields (this is what
    /// lets a degraded peer re-derive a lost `blobHash` from a healthy peer).
    #[tokio::test]
    async fn projects_serving_critical_blob_fields() {
        let (sync, _temp) = test_sync_manager().await;
        let mut content = sample_content("blob-1", "landing");
        content.blob_hash = Some("sha256-abc".to_string());
        content.server_blob_hash = Some("sha256-ssr".to_string());

        super::project_content_doc(&sync, &content).await.unwrap();

        assert_eq!(
            sync.get_doc_field("elohim", "node:blob-1", "blobHash")
                .await
                .unwrap(),
            "sha256-abc"
        );
        assert_eq!(
            sync.get_doc_field("elohim", "node:blob-1", "serverBlobHash")
                .await
                .unwrap(),
            "sha256-ssr"
        );
        assert_eq!(
            sync.get_doc_field("elohim", "node:blob-1", "hAppId")
                .await
                .unwrap(),
            "lamad"
        );
    }

    /// End-to-end back-fill over a real (in-memory) SQL corpus: the first pass
    /// projects every row; the second is a full no-op. Proves the O(rows) back-fill
    /// is idempotent and safe to run on every cold start.
    #[tokio::test]
    async fn backfill_projects_all_rows_and_is_idempotent() {
        use crate::db::content_diesel::{self, CreateContentInput};
        use crate::db::context::AppContext;

        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();

        {
            let mut conn = pool.get().unwrap();
            for i in 0..3 {
                let input = CreateContentInput {
                    id: format!("bf-{i}"),
                    title: format!("row {i}"),
                    description: None,
                    content_type: "concept".to_string(),
                    content_format: "markdown".to_string(),
                    blob_hash: Some(format!("sha256-{i}")),
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "commons".to_string(),
                    created_by: None,
                    tags: vec![],
                    content_body: Some("body".to_string()),
                    dht_anchor_hash: None,
                };
                content_diesel::create_content(&mut conn, &ctx, input).unwrap();
            }
        }

        // First pass projects all 3 (batch smaller than corpus exercises paging).
        let s1 = super::backfill_content_docs(&sync, &pool, 2).await.unwrap();
        assert_eq!(s1.scanned, 3);
        assert_eq!(s1.projected, 3);
        assert_eq!(s1.skipped, 0);
        assert_eq!(sync.count_documents("elohim").await.unwrap(), 3);

        // Second pass is a full no-op — the idempotency guarantee.
        let s2 = super::backfill_content_docs(&sync, &pool, 2).await.unwrap();
        assert_eq!(s2.scanned, 3);
        assert_eq!(s2.projected, 0);
        assert_eq!(s2.skipped, 3);
        assert_eq!(sync.count_documents("elohim").await.unwrap(), 3);

        // The seeded blobHash converged into the sync doc.
        assert_eq!(
            sync.get_doc_field("elohim", "node:bf-0", "blobHash")
                .await
                .unwrap(),
            "sha256-0"
        );
    }

    /// B1 proof — the consumer heal leg: a converged real blob_hash re-derives into
    /// a NULL SQL row (amber-stamped, never notarized); an EMPTY converged blob_hash
    /// never overwrites a present real hash (empty-never-wins). This is the
    /// elohim.host `blobHash: null` 404 as a red→green, WITHOUT the notary.
    #[tokio::test]
    async fn reverse_project_heals_null_and_empty_never_wins() {
        use crate::db::content_diesel::{self, CreateContentInput};
        use crate::db::context::AppContext;

        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();

        // Local rows: heal-null (blob_hash NULL — the elohim.host case); heal-real
        // (a present real blob_hash that empty must never clobber).
        {
            let mut conn = pool.get().unwrap();
            for (id, bh) in [
                ("heal-null", None),
                ("heal-real", Some("sha256-existing".to_string())),
            ] {
                content_diesel::create_content(
                    &mut conn,
                    &ctx,
                    CreateContentInput {
                        id: id.to_string(),
                        title: "t".to_string(),
                        description: None,
                        content_type: "concept".to_string(),
                        content_format: "markdown".to_string(),
                        blob_hash: bh,
                        blob_cid: None,
                        content_size_bytes: None,
                        metadata_json: None,
                        reach: "commons".to_string(),
                        created_by: None,
                        tags: vec![],
                        content_body: Some("b".to_string()),
                        dht_anchor_hash: None,
                    },
                )
                .unwrap();
            }
        }

        // Peer A's converged doc for heal-null carries a REAL blob_hash.
        let mut peer_a = sample_content("heal-null", "t");
        peer_a.blob_hash = Some("sha256-converged".to_string());
        super::project_content_doc(&sync, &peer_a).await.unwrap();
        // Peer B's converged doc for heal-real carries an EMPTY blob_hash (None → "").
        let peer_b = sample_content("heal-real", "t");
        super::project_content_doc(&sync, &peer_b).await.unwrap();

        assert!(
            super::reverse_project_content_doc(&sync, &pool, "node:heal-null")
                .await
                .unwrap(),
            "null local + real converged → heals"
        );
        assert!(
            !super::reverse_project_content_doc(&sync, &pool, "node:heal-real")
                .await
                .unwrap(),
            "empty converged → skipped (empty never wins)"
        );

        let mut conn = pool.get().unwrap();
        let healed = content_diesel::get_content(
            &mut conn,
            &ctx,
            "heal-null",
            content_diesel::MinTrust::Invisible,
        )
        .unwrap()
        .unwrap();
        assert_eq!(healed.blob_hash.as_deref(), Some("sha256-converged"));
        assert!(
            healed.crdt_converged_at.is_some(),
            "healed row is amber-stamped"
        );
        assert!(healed.dht_anchor_hash.is_none(), "heal never notarizes");

        let untouched = content_diesel::get_content(
            &mut conn,
            &ctx,
            "heal-real",
            content_diesel::MinTrust::Invisible,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            untouched.blob_hash.as_deref(),
            Some("sha256-existing"),
            "empty never clobbers a real hash"
        );
    }

    /// Empty serving-critical fields are NOT projected into the doc at all —
    /// the producer-side mirror of the heal's empty-never-wins. Projecting
    /// `""` would enter LWW competition against a healthy peer's real hash
    /// (and can win the merge), so absence — not empty — is the wire form of
    /// "this peer doesn't know". `updatedAt` is peer-local row metadata: two
    /// peers seeded at different times hold different values for identical
    /// content, so projecting it makes every fleet-wide backfill append a
    /// no-op change per doc per peer (history inflation). Causality lives in
    /// the CRDT history itself.
    #[tokio::test]
    async fn empty_serving_fields_are_not_projected() {
        use automerge::ReadDoc;

        let (sync, _temp) = test_sync_manager().await;
        // sample_content: blob_hash/server_blob_hash/blob_cid/content_size_bytes all unset.
        let content = sample_content("empty-1", "t");
        super::project_content_doc(&sync, &content).await.unwrap();

        for field in ["blobHash", "serverBlobHash", "blobCid"] {
            assert!(
                sync.get_doc_field("elohim", "node:empty-1", field)
                    .await
                    .is_err(),
                "unset {field} must be ABSENT from the doc, not projected as \"\""
            );
        }
        let doc = sync
            .get_or_create_doc("elohim", "node:empty-1")
            .await
            .unwrap();
        assert!(
            doc.get(automerge::ROOT, "contentSizeBytes")
                .unwrap()
                .is_none(),
            "unset contentSizeBytes must be absent, not projected as 0"
        );
        assert!(
            doc.get(automerge::ROOT, "updatedAt").unwrap().is_none(),
            "peer-local updatedAt must not be projected (backfill ping-pong inflation)"
        );
        // Nullable content fields follow the same absent-not-empty rule: a
        // NULL description / default "{}" metadata must not project as ""/"{}"
        // — an empty put deterministically erases a peer's real value in the
        // converged doc (same LWW mechanic as the serving fields).
        assert!(
            doc.get(automerge::ROOT, "description").unwrap().is_none(),
            "NULL description must be absent, not projected as empty string"
        );
        assert!(
            doc.get(automerge::ROOT, "metadata").unwrap().is_none(),
            "default metadata must be absent, not projected as {{}}"
        );
        // Sanity: real fields still project.
        assert_eq!(
            sync.get_doc_field("elohim", "node:empty-1", "title")
                .await
                .unwrap(),
            "t"
        );
    }

    /// Only broadcast-tier reach enters the sync plane (fail-closed): the
    /// plane has no receiver-side reach pre-authorization, so private-family,
    /// relationship-scoped, and UNKNOWN (drifted-vocabulary) reach values must
    /// never be projected — by either the event or the reconcile path.
    #[tokio::test]
    async fn non_broadcast_reach_is_never_projected() {
        let (sync, _temp) = test_sync_manager().await;
        for (i, reach) in [
            "private",
            "self",
            "intimate",
            "trusted",
            "familiar",
            "household",
        ]
        .iter()
        .enumerate()
        {
            let mut content = sample_content(&format!("reach-{i}"), "t");
            content.reach = reach.to_string();
            assert!(
                !super::project_content_doc(&sync, &content).await.unwrap(),
                "reach {reach} must not project (event path)"
            );
            assert!(
                !super::project_content_doc_reconcile(&sync, &content)
                    .await
                    .unwrap(),
                "reach {reach} must not project (reconcile path)"
            );
            assert!(
                sync.get_heads("elohim", &format!("node:reach-{i}"))
                    .await
                    .is_err()
                    || sync
                        .get_heads("elohim", &format!("node:reach-{i}"))
                        .await
                        .unwrap()
                        .is_empty(),
                "no doc may exist for reach {reach}"
            );
        }
        for (i, reach) in ["community", "public", "commons"].iter().enumerate() {
            let mut content = sample_content(&format!("bcast-{i}"), "t");
            content.reach = reach.to_string();
            assert!(
                super::project_content_doc(&sync, &content).await.unwrap(),
                "broadcast reach {reach} must project"
            );
        }
    }

    /// Reconciliation (back-fill) OFFERS, never FIGHTS: a converged non-empty
    /// doc value survives a reconcile from a row holding a DIFFERENT non-empty
    /// value (the stale-restarter case), while a legacy empty-string doc value
    /// stays fillable. The event path keeps assert semantics.
    #[tokio::test]
    async fn reconcile_fills_gaps_but_never_fights_converged_values() {
        let (sync, _temp) = test_sync_manager().await;

        // Fresh event projection establishes the converged truth (H2).
        let mut fresh = sample_content("rec-1", "t");
        fresh.blob_hash = Some("sha256-h2".to_string());
        super::project_content_doc(&sync, &fresh).await.unwrap();
        let heads_converged = sync.get_heads("elohim", "node:rec-1").await.unwrap();

        // A stale peer's back-fill (row still holds H1) must not fight.
        let mut stale = sample_content("rec-1", "t");
        stale.blob_hash = Some("sha256-h1".to_string());
        assert!(
            !super::project_content_doc_reconcile(&sync, &stale)
                .await
                .unwrap(),
            "reconcile must not contest a present non-empty doc value"
        );
        assert_eq!(
            sync.get_heads("elohim", "node:rec-1").await.unwrap(),
            heads_converged,
            "the declined fight must append no change"
        );
        assert_eq!(
            sync.get_doc_field("elohim", "node:rec-1", "blobHash")
                .await
                .unwrap(),
            "sha256-h2"
        );

        // Legacy pre-guard docs hold blobHash:"" — reconcile treats empty as
        // absent and fills it (the legacy self-heal path).
        use automerge::transaction::Transactable;
        let mut legacy = sync
            .get_or_create_doc("elohim", "node:rec-2")
            .await
            .unwrap();
        legacy
            .transact::<_, _, automerge::AutomergeError>(|tx| {
                tx.put(automerge::ROOT, "title", "t")?;
                tx.put(automerge::ROOT, "blobHash", "")?;
                Ok(())
            })
            .unwrap();
        sync.apply_changes("elohim", "node:rec-2", vec![legacy.save()])
            .await
            .unwrap();
        let mut healthy = sample_content("rec-2", "t");
        healthy.blob_hash = Some("sha256-real".to_string());
        assert!(
            super::project_content_doc_reconcile(&sync, &healthy)
                .await
                .unwrap(),
            "reconcile must fill a legacy empty-string value"
        );
        assert_eq!(
            sync.get_doc_field("elohim", "node:rec-2", "blobHash")
                .await
                .unwrap(),
            "sha256-real"
        );
    }

    /// Amber heals converge set→set: an amber (converged-but-unnotarized)
    /// blob_hash is REPLACEABLE by a newer amber heal, so a peer holding a
    /// stale hash converges instead of serving it forever; a green
    /// (dht-anchored) blob_hash is never overwritten by amber.
    #[tokio::test]
    async fn amber_heal_replaces_amber_but_never_green() {
        use crate::db::content_diesel::{self, CreateContentInput};
        use crate::db::context::AppContext;

        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        {
            let mut conn = pool.get().unwrap();
            for (id, bh, anchor) in [
                ("amber-stale", None::<String>, None::<String>),
                (
                    "green-locked",
                    Some("sha256-green-h1".to_string()),
                    Some("uhCkk-fake-anchor".to_string()),
                ),
            ] {
                content_diesel::create_content(
                    &mut conn,
                    &ctx,
                    CreateContentInput {
                        id: id.to_string(),
                        title: "t".to_string(),
                        description: None,
                        content_type: "concept".to_string(),
                        content_format: "markdown".to_string(),
                        blob_hash: bh,
                        blob_cid: None,
                        content_size_bytes: None,
                        metadata_json: None,
                        reach: "commons".to_string(),
                        created_by: None,
                        tags: vec![],
                        content_body: Some("b".to_string()),
                        dht_anchor_hash: anchor,
                    },
                )
                .unwrap();
            }
        }

        // Round 1: heal amber-stale to H1 (row NULL → amber write).
        let mut v1 = sample_content("amber-stale", "t");
        v1.blob_hash = Some("sha256-h1".to_string());
        super::project_content_doc(&sync, &v1).await.unwrap();
        assert!(
            super::reverse_project_content_doc(&sync, &pool, "node:amber-stale")
                .await
                .unwrap()
        );

        // Round 2: the fleet converges to H2 — the amber row must FOLLOW.
        let mut v2 = sample_content("amber-stale", "t");
        v2.blob_hash = Some("sha256-h2".to_string());
        super::project_content_doc(&sync, &v2).await.unwrap();
        assert!(
            super::reverse_project_content_doc(&sync, &pool, "node:amber-stale")
                .await
                .unwrap()
        );
        {
            let mut conn = pool.get().unwrap();
            let row = content_diesel::get_content(
                &mut conn,
                &ctx,
                "amber-stale",
                content_diesel::MinTrust::Invisible,
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                row.blob_hash.as_deref(),
                Some("sha256-h2"),
                "amber must replace amber (set→set convergence)"
            );
            assert!(row.dht_anchor_hash.is_none());
        }

        // Green is inviolable: a converged doc value never overwrites an
        // anchored blob_hash.
        let mut g2 = sample_content("green-locked", "t");
        g2.blob_hash = Some("sha256-green-h2".to_string());
        super::project_content_doc(&sync, &g2).await.unwrap();
        super::reverse_project_content_doc(&sync, &pool, "node:green-locked")
            .await
            .unwrap();
        {
            let mut conn = pool.get().unwrap();
            let row = content_diesel::get_content(
                &mut conn,
                &ctx,
                "green-locked",
                content_diesel::MinTrust::Invisible,
            )
            .unwrap()
            .unwrap();
            assert_eq!(
                row.blob_hash.as_deref(),
                Some("sha256-green-h1"),
                "amber must never clobber a green (anchored) blob_hash"
            );
        }
    }

    /// A doc that already converged a peer's real blobHash is left untouched by a
    /// local re-projection whose row lacks that hash — the local peer's ignorance
    /// is not a "change". Idempotency must hold so backfill on the degraded peer
    /// never appends history against the converged value.
    #[tokio::test]
    async fn projection_skips_when_local_serving_field_is_empty_and_doc_converged() {
        let (sync, _temp) = test_sync_manager().await;

        let mut healthy = sample_content("conv-1", "t");
        healthy.blob_hash = Some("sha256-real".to_string());
        assert!(super::project_content_doc(&sync, &healthy).await.unwrap());
        let heads_converged = sync.get_heads("elohim", "node:conv-1").await.unwrap();

        // Same row as this peer sees it: blob_hash never landed locally.
        let degraded = sample_content("conv-1", "t");
        assert!(
            !super::project_content_doc(&sync, &degraded).await.unwrap(),
            "empty local serving field vs converged doc value must be a skip"
        );
        let heads_after = sync.get_heads("elohim", "node:conv-1").await.unwrap();
        assert_eq!(
            heads_converged, heads_after,
            "the skip must not append a change"
        );
        assert_eq!(
            sync.get_doc_field("elohim", "node:conv-1", "blobHash")
                .await
                .unwrap(),
            "sha256-real",
            "converged value survives the degraded re-projection"
        );
    }

    /// Fleet-wide backfill safety: a degraded peer's projection (no blobHash)
    /// merged with a healthy peer's projection (real blobHash) converges to the
    /// real value in BOTH merge orders. With empty projected as `""` this is an
    /// LWW coin-flip on actor id; with absence it is structurally guaranteed.
    #[tokio::test]
    async fn degraded_peer_backfill_cannot_clobber_converged_blob_hash() {
        let (healthy_mgr, _t1) = test_sync_manager().await;
        let (degraded_mgr, _t2) = test_sync_manager().await;

        let mut healthy = sample_content("clob-1", "t");
        healthy.blob_hash = Some("sha256-real".to_string());
        super::project_content_doc(&healthy_mgr, &healthy)
            .await
            .unwrap();
        let degraded = sample_content("clob-1", "t");
        super::project_content_doc(&degraded_mgr, &degraded)
            .await
            .unwrap();

        // Capture both peers' PRE-merge doc states (what a sync round would carry).
        let healthy_bytes = healthy_mgr
            .get_or_create_doc("elohim", "node:clob-1")
            .await
            .unwrap()
            .save();
        let degraded_bytes = degraded_mgr
            .get_or_create_doc("elohim", "node:clob-1")
            .await
            .unwrap()
            .save();

        // Order 1: healthy converges INTO the degraded peer.
        degraded_mgr
            .apply_changes("elohim", "node:clob-1", vec![healthy_bytes])
            .await
            .unwrap();
        assert_eq!(
            degraded_mgr
                .get_doc_field("elohim", "node:clob-1", "blobHash")
                .await
                .unwrap(),
            "sha256-real",
            "degraded peer must converge to the real hash"
        );

        // Order 2: degraded converges INTO the healthy peer.
        healthy_mgr
            .apply_changes("elohim", "node:clob-1", vec![degraded_bytes])
            .await
            .unwrap();
        assert_eq!(
            healthy_mgr
                .get_doc_field("elohim", "node:clob-1", "blobHash")
                .await
                .unwrap(),
            "sha256-real",
            "healthy peer must retain the real hash after merging a degraded doc"
        );
    }

    /// The corpus backfill is a reconciliation-controller leg: default ON so every
    /// cold start converges the DocStore to the SQL corpus with zero per-env
    /// config. Only an explicit `0`/`false` opts out.
    #[test]
    fn backfill_env_gate_defaults_on() {
        assert!(super::backfill_enabled(None), "unset → backfill runs");
        assert!(!super::backfill_enabled(Some("0")));
        assert!(!super::backfill_enabled(Some("false")));
        assert!(!super::backfill_enabled(Some("FALSE")));
        assert!(super::backfill_enabled(Some("1")));
        assert!(super::backfill_enabled(Some("true")));
        assert!(
            super::backfill_enabled(Some("unrecognized")),
            "default-on posture: only an explicit off-value disables"
        );
    }

    /// The declared-head hint scalar follows the absent-not-empty rule exactly
    /// like blobHash: a set `declared_head_action_hash` projects as
    /// `headActionHash`; None (and empty-string) project as ABSENCE — an empty
    /// put would erase a peer's real hint via LWW.
    #[tokio::test]
    async fn head_action_hash_projected_when_set_absent_when_none() {
        let (sync, _temp) = test_sync_manager().await;

        // None → absent from the doc entirely.
        let none = sample_content("head-none", "t");
        super::project_content_doc(&sync, &none).await.unwrap();
        assert!(
            sync.get_doc_field("elohim", "node:head-none", "headActionHash")
                .await
                .is_err(),
            "unset declared head must be ABSENT from the doc, not projected as \"\""
        );

        // Some(non-empty) → projected.
        let mut set = sample_content("head-set", "t");
        set.declared_head_action_hash = Some("uhCkk-head-1".to_string());
        super::project_content_doc(&sync, &set).await.unwrap();
        assert_eq!(
            sync.get_doc_field("elohim", "node:head-set", "headActionHash")
                .await
                .unwrap(),
            "uhCkk-head-1"
        );

        // Some("") behaves like None (absent-not-empty).
        let mut empty = sample_content("head-empty", "t");
        empty.declared_head_action_hash = Some(String::new());
        super::project_content_doc(&sync, &empty).await.unwrap();
        assert!(
            sync.get_doc_field("elohim", "node:head-empty", "headActionHash")
                .await
                .is_err(),
            "empty declared head must be ABSENT from the doc"
        );
    }

    /// doc_matches idempotency covers the new scalar: an unchanged declared
    /// head is a no-op skip (no history inflation), an ADVANCED declared head
    /// forces a re-projection and the new value converges into the doc.
    #[tokio::test]
    async fn changed_declared_head_forces_reprojection() {
        let (sync, _temp) = test_sync_manager().await;
        let mut content = sample_content("head-chg", "t");
        content.declared_head_action_hash = Some("uhCkk-head-1".to_string());

        assert!(super::project_content_doc(&sync, &content).await.unwrap());
        let heads1 = sync.get_heads("elohim", "node:head-chg").await.unwrap();

        // Unchanged declared head → idempotent skip, no change appended.
        assert!(
            !super::project_content_doc(&sync, &content).await.unwrap(),
            "unchanged declared head must be a skip"
        );
        assert_eq!(
            heads1,
            sync.get_heads("elohim", "node:head-chg").await.unwrap(),
            "the skip must not append a change"
        );

        // The author declares a new head → must re-project.
        content.declared_head_action_hash = Some("uhCkk-head-2".to_string());
        assert!(
            super::project_content_doc(&sync, &content).await.unwrap(),
            "a changed declared head must re-project"
        );
        assert_ne!(
            heads1,
            sync.get_heads("elohim", "node:head-chg").await.unwrap(),
            "a changed declared head must append a change"
        );
        assert_eq!(
            sync.get_doc_field("elohim", "node:head-chg", "headActionHash")
                .await
                .unwrap(),
            "uhCkk-head-2"
        );
    }

    /// REQ-N5 laundering guard: a converged doc carrying `headActionHash` NEVER
    /// stamps `dht_anchor_hash` or `declared_head_action_hash` on the SQL row.
    /// The reverse projection heals `blobHash` ONLY (amber-marked) — anchors
    /// are written exclusively by conductor-verified paths, and consuming the
    /// doc's head hint here would launder unauthenticated peer input into
    /// notarization provenance.
    #[tokio::test]
    async fn converged_head_hint_is_never_stamped() {
        use crate::db::content_diesel::{self, CreateContentInput};
        use crate::db::context::AppContext;

        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();

        // Local row: blob_hash NULL (healable), no anchor, no declared head.
        {
            let mut conn = pool.get().unwrap();
            content_diesel::create_content(
                &mut conn,
                &ctx,
                CreateContentInput {
                    id: "head-launder".to_string(),
                    title: "t".to_string(),
                    description: None,
                    content_type: "concept".to_string(),
                    content_format: "markdown".to_string(),
                    blob_hash: None,
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "commons".to_string(),
                    created_by: None,
                    tags: vec![],
                    content_body: Some("b".to_string()),
                    dht_anchor_hash: None,
                },
            )
            .unwrap();
        }

        // A peer's converged doc carries BOTH a real blobHash and a head hint —
        // exactly what an adversarial (or merely divergent) peer could gossip.
        let mut peer = sample_content("head-launder", "t");
        peer.blob_hash = Some("sha256-converged".to_string());
        peer.declared_head_action_hash = Some("uhCkk-peer-declared-head".to_string());
        super::project_content_doc(&sync, &peer).await.unwrap();
        assert_eq!(
            sync.get_doc_field("elohim", "node:head-launder", "headActionHash")
                .await
                .unwrap(),
            "uhCkk-peer-declared-head",
            "precondition: the hint IS present in the converged doc"
        );

        // The heal runs (blobHash converges into SQL)…
        assert!(
            super::reverse_project_content_doc(&sync, &pool, "node:head-launder")
                .await
                .unwrap()
        );

        // …but the head hint is NEVER laundered into notarization provenance.
        let mut conn = pool.get().unwrap();
        let row = content_diesel::get_content(
            &mut conn,
            &ctx,
            "head-launder",
            content_diesel::MinTrust::Invisible,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            row.blob_hash.as_deref(),
            Some("sha256-converged"),
            "the blobHash heal itself must still land"
        );
        assert!(
            row.crdt_converged_at.is_some(),
            "the heal is amber-stamped as ever"
        );
        assert!(
            row.dht_anchor_hash.is_none(),
            "REQ-N5: a converged doc must never stamp dht_anchor_hash"
        );
        assert!(
            row.declared_head_action_hash.is_none(),
            "REQ-N5: the converged headActionHash hint must never be consumed \
             into declared_head_action_hash"
        );
    }

    /// Seed N content rows and return their ids (helper for the bulk-event tests).
    fn seed_rows(pool: &crate::db::DbPool, prefix: &str, n: usize) -> Vec<String> {
        use crate::db::content_diesel::{self, CreateContentInput};
        use crate::db::context::AppContext;

        let ctx = AppContext::default_lamad();
        let mut conn = pool.get().unwrap();
        let mut ids = Vec::with_capacity(n);
        for i in 0..n {
            let id = format!("{prefix}-{i}");
            content_diesel::create_content(
                &mut conn,
                &ctx,
                CreateContentInput {
                    id: id.clone(),
                    title: format!("row {i}"),
                    description: None,
                    content_type: "concept".to_string(),
                    content_format: "markdown".to_string(),
                    blob_hash: Some(format!("sha256-{prefix}-{i}")),
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "commons".to_string(),
                    created_by: None,
                    tags: vec![],
                    content_body: Some("body".to_string()),
                    dht_anchor_hash: None,
                },
            )
            .unwrap();
            ids.push(id);
        }
        ids
    }

    /// (a) A `ContentBulkCreated` batch projects every distribution-safe id in the
    /// batch into the DocStore — the ignore is closed, bulk-seeded rows now enter
    /// the CRDT plane.
    #[tokio::test]
    async fn bulk_created_projects_all_ids_into_docstore() {
        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ids = seed_rows(&pool, "bulk", 5);

        let stats = super::project_content_bulk(&sync, &pool, &ids).await;
        assert_eq!(stats.projected, 5, "every seeded id projects");
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(sync.count_documents("elohim").await.unwrap(), 5);
        assert_eq!(
            sync.get_doc_field("elohim", "node:bulk-0", "blobHash")
                .await
                .unwrap(),
            "sha256-bulk-0",
            "the seeded blobHash converged into the sync doc"
        );
    }

    /// (b) Re-delivering the SAME `ContentBulkCreated` event yields a single
    /// effect: the second pass projects nothing and appends no Automerge change
    /// (mirrors `backfill_projects_all_rows_and_is_idempotent`).
    #[tokio::test]
    async fn bulk_created_is_idempotent_on_redelivery() {
        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ids = seed_rows(&pool, "redeliver", 3);

        let s1 = super::project_content_bulk(&sync, &pool, &ids).await;
        assert_eq!(s1.projected, 3);
        let heads1 = sync.get_heads("elohim", "node:redeliver-0").await.unwrap();

        // Same event delivered twice — the write path may re-emit on a retry.
        let s2 = super::project_content_bulk(&sync, &pool, &ids).await;
        assert_eq!(s2.projected, 0, "re-delivery must project nothing");
        assert_eq!(s2.skipped, 3, "every id is an idempotent skip");
        assert_eq!(s2.failed, 0);
        let heads2 = sync.get_heads("elohim", "node:redeliver-0").await.unwrap();
        assert_eq!(heads1, heads2, "re-delivery must not append a change");
        assert_eq!(sync.count_documents("elohim").await.unwrap(), 3);
    }

    /// (c) The partial-re-seed TRAP: `ids` covers the whole input batch while
    /// `count` counts only newly-inserted rows, so a batch re-delivers an
    /// already-projected id alongside never-projected ones. Only the missing ones
    /// project; the already-present id is an idempotent skip, and the batch never
    /// errors.
    #[tokio::test]
    async fn bulk_created_partial_reseed_projects_only_missing() {
        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ids = seed_rows(&pool, "partial", 3);

        // partial-0 was projected on an earlier pass (already in the DocStore).
        let pre = super::project_content_bulk(&sync, &pool, &ids[..1]).await;
        assert_eq!(pre.projected, 1);

        // A partial re-seed emits the WHOLE batch though only 2 ids are new.
        let stats = super::project_content_bulk(&sync, &pool, &ids).await;
        assert_eq!(
            stats.projected, 2,
            "only the two never-projected ids project"
        );
        assert_eq!(stats.skipped, 1, "the already-projected id is a skip");
        assert_eq!(stats.failed, 0, "the partial-re-seed trap must not error");
        assert_eq!(sync.count_documents("elohim").await.unwrap(), 3);
    }

    /// Finding-1 regression: bulk projection uses offer-not-fight reconcile
    /// semantics so a re-seed re-delivering a STALE local row can never regress a
    /// value the fleet already converged. Pre-converge the doc to a fresher peer
    /// value (B2); the local SQL row still holds the stale B1; deliver the id via
    /// the bulk path; the converged B2 must survive untouched. (With the assertive
    /// path this test would fail — the doc would be rewritten back to B1.)
    #[tokio::test]
    async fn bulk_created_reconcile_never_regresses_converged_value() {
        use crate::db::content_diesel::{self, CreateContentInput};
        use crate::db::context::AppContext;

        let (sync, _temp) = test_sync_manager().await;
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();

        // Local SQL row holds the STALE hash B1. Its body matches sample_content
        // so blobHash is the ONLY field that could differ from the converged doc.
        {
            let mut conn = pool.get().unwrap();
            content_diesel::create_content(
                &mut conn,
                &ctx,
                CreateContentInput {
                    id: "regress-x".to_string(),
                    title: "t".to_string(),
                    description: None,
                    content_type: "concept".to_string(),
                    content_format: "markdown".to_string(),
                    blob_hash: Some("sha256-B1".to_string()),
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "commons".to_string(),
                    created_by: None,
                    tags: vec![],
                    content_body: Some("hello".to_string()),
                    dht_anchor_hash: None,
                },
            )
            .unwrap();
        }

        // Pre-converge the doc to the FRESHER peer value B2 via the assertive event
        // path (what a healthy peer's sync round would have delivered).
        let mut fresher = sample_content("regress-x", "t");
        fresher.blob_hash = Some("sha256-B2".to_string());
        super::project_content_doc(&sync, &fresher).await.unwrap();
        let heads_converged = sync.get_heads("elohim", "node:regress-x").await.unwrap();

        // A re-seed re-delivers the id though the SQL row is the stale B1.
        let stats = super::project_content_bulk(&sync, &pool, &["regress-x".to_string()]).await;
        assert_eq!(
            stats.projected, 0,
            "reconcile must not write against a fully-converged doc"
        );
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.failed, 0);

        assert_eq!(
            sync.get_doc_field("elohim", "node:regress-x", "blobHash")
                .await
                .unwrap(),
            "sha256-B2",
            "the converged value must NOT be regressed to the stale SQL row's B1"
        );
        assert_eq!(
            sync.get_heads("elohim", "node:regress-x").await.unwrap(),
            heads_converged,
            "the declined reconcile must append no change"
        );
    }

    /// Listener-level convergence + non-starvation: drive a multi-chunk
    /// `ContentBulkCreated` (250 ids → two chunks, exercises the chunk+yield loop)
    /// through the REAL `EventBus` + spawned listener, concurrently with a single
    /// `ContentCreated`. Because bulk projection runs on its own task, the listener
    /// keeps draining the channel, so BOTH the whole bulk batch and the concurrent
    /// single event converge — nothing is lost.
    #[tokio::test]
    async fn listener_converges_multichunk_bulk_and_concurrent_single() {
        use crate::services::events::{EventBus, StorageEvent};
        use std::time::Duration;

        let events = Arc::new(EventBus::new());
        let (sync, _temp) = test_sync_manager().await;
        let sync = Arc::new(sync);
        let pool = crate::test_util::test_pool();

        // 250 rows for the bulk batch (>200 → two chunks) + 1 for the single event.
        let bulk_ids = seed_rows(&pool, "big", 250);
        let _ = seed_rows(&pool, "solo", 1);

        // `subscribe()` runs synchronously inside spawn_content_projection_listener,
        // so the receiver exists before we emit — no subscribe/emit race, no loss.
        let handle = super::spawn_content_projection_listener(
            Arc::clone(&events),
            Arc::clone(&sync),
            pool.clone(),
        );

        events.emit(StorageEvent::ContentBulkCreated {
            count: bulk_ids.len(),
            ids: bulk_ids.clone(),
        });
        events.emit(StorageEvent::ContentCreated {
            id: "solo-0".to_string(),
            title: "t".to_string(),
            content_type: None,
        });

        // Poll for convergence of all 251 docs (bounded so a hang fails, not spins).
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            if sync.count_documents("elohim").await.unwrap() == 251 {
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "listener did not converge all 251 docs (got {})",
                sync.count_documents("elohim").await.unwrap()
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        // The concurrent single event converged too — proof the bulk task never
        // starved the listener's drain of the broadcast channel.
        assert!(
            !sync
                .get_heads("elohim", "node:solo-0")
                .await
                .unwrap()
                .is_empty(),
            "the concurrent ContentCreated must have projected"
        );
        handle.abort();
    }
}
