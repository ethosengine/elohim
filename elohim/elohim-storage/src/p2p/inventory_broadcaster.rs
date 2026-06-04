//! Inventory broadcaster — snapshot timer + delta emitter + sequence allocator.
//!
//! Responsibilities:
//! - Periodically (per archetype-tunable cadence) compute the local set of
//!   hosted blob hashes and publish a `BlobInventorySnapshot` to
//!   `elohim/inventory/blob`.
//! - On local blob add/remove events, emit a `BlobInventoryDelta` (with a
//!   small batching window to coalesce bursts).
//! - Allocate per-this-peer monotonic sequence numbers for both message types.
//!
//! Source of truth for "what blobs do I host": the local blob store; the
//! enumeration is delegated to `LocalInventory` so it can be mocked in tests.
//!
//! ## Wave 3: gather_hints
//!
//! `gather_hints(conn, hashes, local_peer_id)` enriches a set of local blob
//! hashes with per-blob metadata derived from the `content` projection:
//! - `epr_kind`: canonical EprKind entry-type (`"Content"`), matching the
//!   `scope_filter.epr_kinds` schema vocabulary.
//! - `size_bytes`: `content_size_bytes` (uncompressed).
//! - `recipient_hub_id`: the ADVERTISING peer's dwelling hub via
//!   `hub_resolver::resolve_peer_dwelling_hub` (custody chain, once per call).
//! - `tier`: left `None` (TierController is a separate epic).
//!
//! A hint is emitted for every hash with a matching `content` row. The join is
//! inline Diesel (read-side) over the `content` projection; no
//! ReconcileController writes are performed here.

use crate::p2p::inventory_gossip::{
    BlobAddress, BlobHint, BlobInventoryDelta, BlobInventorySnapshot,
};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Trait for enumerating the local blob inventory. Production: walks the
/// blob store. Tests: returns a fixed set.
pub trait LocalInventory: Send + Sync {
    fn current_hashes(&self) -> Vec<BlobAddress>;
}

/// T22 review fix #1: wraps a pre-fetched list of hashes for `LocalInventory`.
///
/// Production callers fetch via `BlobStore::list_hashes()` first and pass the
/// result here so I/O failure can return early *before* ever entering
/// `build_snapshot`. Previously `BlobStoreInventory` swallowed the error and
/// returned an empty `Vec`, which caused `apply_snapshot` on every remote peer
/// to evict this peer's inventory entries during a transient I/O blip.
pub struct StaticInventory(Vec<BlobAddress>);

impl StaticInventory {
    pub fn new(hashes: Vec<BlobAddress>) -> Self {
        Self(hashes)
    }
}

impl LocalInventory for StaticInventory {
    fn current_hashes(&self) -> Vec<BlobAddress> {
        self.0.clone()
    }
}

/// Per-this-peer monotonic sequence allocator.
#[derive(Debug, Clone, Default)]
pub struct SequenceAllocator {
    inner: Arc<AtomicU64>,
}

impl SequenceAllocator {
    pub fn new(initial: u64) -> Self {
        Self {
            inner: Arc::new(AtomicU64::new(initial)),
        }
    }

    /// Allocate the next sequence number.
    pub fn next(&self) -> u64 {
        self.inner.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn current(&self) -> u64 {
        self.inner.load(Ordering::SeqCst)
    }
}

/// Gather per-blob hints for the given set of hashes.
///
/// For each hash, joins the `content` projection on `blob_hash` to derive:
/// - `epr_kind`: the canonical EprKind entry-type string — always `"Content"`
///   for rows sourced from the `content` projection. Must stay in the
///   `scope_filter.epr_kinds` schema vocabulary (EprKind names), NOT the lamad
///   `content_format` vocabulary (storage-tier review 2026-06-04, finding #2).
/// - `size_bytes`: `content_size_bytes` (uncompressed byte count).
/// - `recipient_hub_id`: the ADVERTISING peer's dwelling hub via
///   `hub_resolver::resolve_peer_dwelling_hub(local_peer_id)` — the
///   `peer_id → agent_cid → dwelling_hub_id` custody chain (dwelling-hub spec
///   §8.2), resolved ONCE per call since every advertised blob is in this
///   peer's custody. NOT the content author's hub: a blob authored elsewhere
///   but custodied here belongs to THIS dwelling for replication-matching
///   (storage-tier review 2026-06-04, finding #3).
/// - `tier`: `None` — TierController is a separate epic.
///
/// A hint is emitted for every hash with a matching `content` row.
/// This is a read-side inline Diesel query — no writes, no ReconcileController.
///
/// # Errors
///
/// Database errors are logged at `warn` level and the affected hint is dropped.
/// An empty `Vec` is always valid (hints are advisory; absence does not block gossip).
pub fn gather_hints(
    conn: &mut diesel::SqliteConnection,
    hashes: &[BlobAddress],
    local_peer_id: &str,
) -> Vec<BlobHint> {
    use crate::db::diesel_schema::content::dsl as ct;
    use crate::services::hub_resolver;
    use diesel::prelude::*;

    if hashes.is_empty() {
        return Vec::new();
    }

    // Custody chain, once per call: all advertised blobs are in this peer's
    // custody, so they all carry the same dwelling hub.
    let now_iso = chrono::Utc::now().to_rfc3339();
    let recipient_hub_id: Option<String> = match hub_resolver::resolve_peer_dwelling_hub(
        conn,
        local_peer_id,
        &now_iso,
    ) {
        Ok(opt) => opt,
        Err(e) => {
            tracing::warn!(
                target: "elohim_storage::inventory",
                error = %e,
                peer_id = %local_peer_id,
                "gather_hints: resolve_peer_dwelling_hub failed; hints emitted without recipient_hub_id"
            );
            None
        }
    };

    // Collect hash strings for the IN query.
    let hash_strs: Vec<&str> = hashes.iter().map(|a| a.as_str()).collect();

    // One query for all matching content rows — avoids N+1.
    let rows: Vec<(String, Option<i32>)> = match ct::content
        .filter(ct::blob_hash.eq_any(&hash_strs))
        .select((ct::blob_hash.assume_not_null(), ct::content_size_bytes))
        .load::<(String, Option<i32>)>(conn)
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                target: "elohim_storage::inventory",
                error = %e,
                "gather_hints: content join failed; emitting empty hints"
            );
            return Vec::new();
        }
    };

    let mut hints = Vec::with_capacity(rows.len());
    for (blob_hash, size_bytes) in rows {
        let Ok(address) = BlobAddress::new(blob_hash) else {
            continue;
        };

        // Canonical EprKind entry-type, NOT content_format: the scorer matches
        // this against commitment scope_filter.epr_kinds, whose schema enum is
        // EprKind names ("Content", "Manifest", …). Everything in the content
        // projection is a Content entry, so the kind is always populated and
        // every matched row emits a hint (the old sparse-skip is unreachable).
        let epr_kind = Some(
            crate::services::epr_kind::kind_canonical_str(elohim_epr::EprKind::Content).to_string(),
        );
        let size: Option<u64> = size_bytes.map(|s| s.max(0) as u64);

        hints.push(BlobHint {
            address,
            recipient_hub_id: recipient_hub_id.clone(),
            epr_kind,
            size_bytes: size,
            tier: None, // TierController is a separate epic
        });
    }
    hints
}

/// Build a snapshot for the given peer id with the given inventory and optional hints.
///
/// Wave 3: `hints` are gathered by the caller via `gather_hints(conn, &hashes, local_peer_id)`
/// before construction so the DB call can be skipped on I/O failure.
pub fn build_snapshot<I: LocalInventory>(
    peer_id: &str,
    inventory: &I,
    seq: &SequenceAllocator,
    now_micros: i64,
    hints: Vec<BlobHint>,
) -> BlobInventorySnapshot {
    BlobInventorySnapshot {
        peer_id: peer_id.to_string(),
        hashes: inventory.current_hashes(),
        hints,
        snapshot_at: now_micros,
        sequence: seq.next(),
        signature: vec![0x00], // Stage 1 structural non-empty
    }
}

/// Build a delta for the given add/remove batch and optional hints.
///
/// Wave 3: `hints` are gathered by the caller via `gather_hints(conn, &added_hashes, local_peer_id)`.
pub fn build_delta(
    peer_id: &str,
    added: Vec<BlobAddress>,
    removed: Vec<BlobAddress>,
    seq: &SequenceAllocator,
    now_micros: i64,
    hints: Vec<BlobHint>,
) -> BlobInventoryDelta {
    BlobInventoryDelta {
        peer_id: peer_id.to_string(),
        added,
        removed,
        hints,
        emitted_at: now_micros,
        sequence: seq.next(),
        signature: vec![0x00], // Stage 1 structural non-empty
    }
}

/// Compute the resolved cadence for this peer's archetype, honoring the
/// 4-layer override pattern (archetype default ← policy.toml ← env/CLI ←
/// admin trigger). Returns `None` to mean "broadcasting disabled."
pub fn resolved_cadence(archetype: Option<&str>, config_override: Option<u64>) -> Option<u64> {
    if let Some(seconds) = config_override {
        if seconds == 0 {
            return None; // explicit "0" means disabled
        }
        return Some(seconds);
    }
    crate::config::inventory_broadcast_seconds_default(archetype)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::inventory_gossip::BlobAddress;

    fn sha256_wire(byte: char) -> String {
        format!(
            "sha256-{}",
            std::iter::repeat_n(byte, 64).collect::<String>()
        )
    }

    struct MockInventory(Vec<BlobAddress>);
    impl LocalInventory for MockInventory {
        fn current_hashes(&self) -> Vec<BlobAddress> {
            self.0.clone()
        }
    }

    #[test]
    fn sequence_allocator_increments() {
        let alloc = SequenceAllocator::new(0);
        assert_eq!(alloc.next(), 1);
        assert_eq!(alloc.next(), 2);
        assert_eq!(alloc.next(), 3);
        assert_eq!(alloc.current(), 3);
    }

    #[test]
    fn snapshot_includes_inventory_and_advances_sequence() {
        let inv = MockInventory(vec![
            BlobAddress::new(sha256_wire('a')).unwrap(),
            BlobAddress::new(sha256_wire('b')).unwrap(),
        ]);
        let alloc = SequenceAllocator::new(10);
        let snapshot = build_snapshot("12D3KooWtest", &inv, &alloc, 1_700_000_000_000_000, vec![]);

        assert_eq!(snapshot.peer_id, "12D3KooWtest");
        assert_eq!(snapshot.hashes.len(), 2);
        assert_eq!(snapshot.sequence, 11);
        assert_eq!(snapshot.signature, vec![0x00]);
        assert!(snapshot.hints.is_empty(), "no hints provided → empty");
    }

    #[test]
    fn delta_carries_added_and_removed() {
        let alloc = SequenceAllocator::new(0);
        let delta = build_delta(
            "12D3KooWtest",
            vec![BlobAddress::new(sha256_wire('a')).unwrap()],
            vec![BlobAddress::new(sha256_wire('b')).unwrap()],
            &alloc,
            1_700_000_001_000_000,
            vec![],
        );

        assert_eq!(delta.added.len(), 1);
        assert_eq!(delta.removed.len(), 1);
        assert_eq!(delta.sequence, 1);
        assert!(delta.hints.is_empty(), "no hints provided → empty");
    }

    #[test]
    fn resolved_cadence_uses_override_when_present() {
        assert_eq!(resolved_cadence(Some("node"), Some(120)), Some(120));
    }

    #[test]
    fn resolved_cadence_falls_back_to_archetype_default() {
        assert_eq!(resolved_cadence(Some("node"), None), Some(60));
        assert_eq!(resolved_cadence(Some("desktop"), None), Some(300));
        assert_eq!(resolved_cadence(Some("mobile"), None), None);
        assert_eq!(resolved_cadence(Some("steward"), None), Some(60));
    }

    #[test]
    fn resolved_cadence_zero_override_disables() {
        assert_eq!(resolved_cadence(Some("node"), Some(0)), None);
    }

    /// T22 review fix #1: `StaticInventory` round-trips its provided hashes
    /// through `LocalInventory::current_hashes`. The production caller fetches
    /// hashes via `BlobStore::list_hashes()` *before* constructing this and
    /// returns early on I/O failure; the adapter itself is intentionally
    /// dumb so the failure path can never reach `build_snapshot`. BlobStore's
    /// own `list_hashes` tests cover the I/O behavior.
    #[test]
    fn static_inventory_returns_provided_hashes() {
        let addr_a = BlobAddress::new(sha256_wire('a')).unwrap();
        let addr_b = BlobAddress::new(sha256_wire('b')).unwrap();
        let inv = StaticInventory::new(vec![addr_a.clone(), addr_b.clone()]);
        let hashes = inv.current_hashes();
        assert_eq!(hashes, vec![addr_a, addr_b]);
    }

    // -----------------------------------------------------------------------
    // Wave 3: gather_hints unit tests
    // -----------------------------------------------------------------------

    fn test_db_pool() -> crate::db::DbPool {
        use crate::db::{run_migrations, DbPool};
        use diesel::r2d2::{ConnectionManager, Pool};
        use diesel::SqliteConnection;
        let url = format!(
            "file:broadcaster_hints_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool: DbPool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn seed_content_with_blob(
        conn: &mut diesel::SqliteConnection,
        blob_hash: &str,
        content_format: &str,
        size_bytes: Option<i32>,
        created_by: Option<&str>,
    ) {
        use crate::db::diesel_schema::content::dsl as ct;
        use diesel::prelude::*;
        diesel::insert_into(ct::content)
            .values((
                ct::id.eq(format!("content-{blob_hash}")),
                ct::h_app_id.eq("test-app"),
                ct::title.eq("test"),
                ct::content_type.eq("Concept"),
                ct::content_format.eq(content_format),
                ct::blob_hash.eq(blob_hash),
                ct::content_size_bytes.eq(size_bytes),
                ct::created_by.eq(created_by),
                ct::reach.eq("commons"),
                ct::validation_status.eq("valid"),
                ct::created_at.eq("2026-01-01T00:00:00Z"),
                ct::updated_at.eq("2026-01-01T00:00:00Z"),
            ))
            .execute(conn)
            .expect("seed content");
    }

    /// Wave 3 T3-1: `gather_hints` returns an empty vec when no content rows match.
    #[test]
    fn gather_hints_empty_when_no_content_match() {
        let pool = test_db_pool();
        let mut conn = pool.get().expect("conn");
        // Use 'f' — valid lowercase hex char not used in other gather_hints tests.
        let addr = BlobAddress::new(sha256_wire('f')).unwrap();
        let hints = gather_hints(&mut conn, &[addr], "12D3KooWUnboundPeer");
        assert!(hints.is_empty(), "no matching content → no hints");
    }

    /// Wave 3 T3-2: `gather_hints` populates epr_kind and size_bytes when content exists.
    #[test]
    fn gather_hints_populates_epr_kind_and_size() {
        let pool = test_db_pool();
        let mut conn = pool.get().expect("conn");

        let hash = sha256_wire('a');
        seed_content_with_blob(&mut conn, &hash, "sophia-quiz-json", Some(8192), None);

        let addr = BlobAddress::new(hash.clone()).unwrap();
        let hints = gather_hints(&mut conn, &[addr], "12D3KooWUnboundPeer");

        assert_eq!(hints.len(), 1);
        let h = &hints[0];
        // EprKind entry-type, regardless of the row's content_format
        // (storage-tier review 2026-06-04, finding #2).
        assert_eq!(h.epr_kind.as_deref(), Some("Content"));
        assert_eq!(h.size_bytes, Some(8192));
        assert!(
            h.recipient_hub_id.is_none(),
            "no identity binding for the advertising peer → no hub"
        );
        assert!(
            h.tier.is_none(),
            "tier always None (TierController deferred)"
        );
    }

    /// Storage-tier review 2026-06-04, finding #2: the hint's `epr_kind` must
    /// be the canonical EprKind entry-type string ("Content"), NOT the lamad
    /// `content_format` ("markdown", "sophia-quiz-json", …). Commitment
    /// `scope_filter.epr_kinds` is schema-constrained to EprKind names
    /// (replicates-dwelling scope_filter enum), so a content_format value can
    /// never match a schema-valid scope filter — every scoped blob scored Skip
    /// and commitment-driven replication silently fetched nothing.
    #[test]
    fn gather_hints_emits_canonical_epr_kind_not_content_format() {
        let pool = test_db_pool();
        let mut conn = pool.get().expect("conn");
        // Use 'd' — valid lowercase hex char not used in other gather_hints tests.
        let hash = sha256_wire('d');
        seed_content_with_blob(&mut conn, &hash, "markdown", Some(1024), None);
        let addr = BlobAddress::new(hash).unwrap();
        let hints = gather_hints(&mut conn, &[addr], "12D3KooWUnboundPeer");
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0].epr_kind.as_deref(),
            Some("Content"),
            "hint epr_kind must be the EprKind entry-type, not content_format"
        );
    }

    /// Cross-module canary (storage-tier review 2026-06-04, finding #2): a hint
    /// produced by `gather_hints` must score `High` against a commitment whose
    /// `scope_epr_kinds` uses the schema vocabulary. This pins the producer and
    /// the scorer to the SAME vocabulary — the original unit tests hand-picked
    /// identical schema-invalid strings on both sides, masking the mismatch.
    #[test]
    fn gathered_hint_scores_high_against_schema_valid_scope() {
        use crate::services::replication_prioritizer::{
            score_advertised_blob, ActiveCommitment, AdvertisedBlob, FetchPriority,
        };

        let pool = test_db_pool();
        let mut conn = pool.get().expect("conn");
        // Use 'e' — valid lowercase hex char not used in other gather_hints tests.
        let hash = sha256_wire('e');
        seed_content_with_blob(&mut conn, &hash, "markdown", Some(2048), None);
        let addr = BlobAddress::new(hash).unwrap();
        let hints = gather_hints(&mut conn, &[addr], "12D3KooWUnboundPeer");
        assert_eq!(hints.len(), 1);
        let h = &hints[0];

        // Mirror the receive-arm BlobHint → AdvertisedBlob mapping (p2p/mod.rs).
        let advertised = AdvertisedBlob {
            blob_cid: h.address.as_str().to_string(),
            source_peer_cid: "peer:source".into(),
            blob_size_bytes: h.size_bytes,
            recipient_hub_id_hint: Some("collective:hubH".into()),
            epr_kind_hint: h.epr_kind.clone(),
        };
        // Schema-valid scope straight from the replicates-dwelling enum.
        let commitment = ActiveCommitment {
            commitment_cid: "comm:canary".into(),
            action: "replicates-dwelling".into(),
            recipient_hub_id: "collective:hubH".into(),
            scope_epr_kinds: Some(vec!["Content".into()]),
            bytes_per_blob_max: Some(10_000_000),
        };
        assert_eq!(
            score_advertised_blob(&advertised, &[commitment]),
            FetchPriority::High,
            "a gather_hints-produced kind must match a schema-valid scope filter"
        );
    }

    /// Storage-tier review 2026-06-04, finding #3: `recipient_hub_id` must
    /// derive from the ADVERTISING peer's custody chain (`peer_id → agent_cid
    /// → dwelling_hub_id` — dwelling-hub spec §8.2 "derivable from
    /// peer_identity_bindings → hub lookup"; tiered-quilt §2 names the same
    /// chain "a reusable mapping, not a per-feature one-off"), NOT from the
    /// content author's owning hub. A blob authored by household A's agent but
    /// custodied on household H's peer must advertise H — otherwise stewards
    /// committed to H never match and cross-hub replication is silently dead.
    #[test]
    fn gather_hints_recipient_hub_from_advertising_peer_not_author() {
        use crate::db::collectives::{create_collective, CreateCollectiveInput};
        use crate::db::context::AppContext;
        use crate::db::humans::CreateHumanInput;
        use diesel::prelude::*;

        let pool = test_db_pool();
        let mut conn = pool.get().expect("conn");
        let app_ctx = AppContext::new("test-app");

        // Two households with canonical CIDs: H hosts the peer, A owns the author.
        for (slug, name, cid) in [
            (
                "household-h",
                "Hosting Household",
                "collective:uhCkkHostingH001",
            ),
            (
                "household-a",
                "Author Household",
                "collective:uhCkkAuthorA0001",
            ),
        ] {
            create_collective(
                &mut conn,
                &app_ctx,
                &CreateCollectiveInput {
                    id: slug.to_string(),
                    name: name.to_string(),
                    description: None,
                    governance_layer: "family".to_string(),
                    constitutional_parent_id: None,
                    reach: "private".to_string(),
                    metadata_json: None,
                    created_by: None,
                },
            )
            .expect("seed collective");
            use crate::db::diesel_schema::collectives;
            diesel::update(
                collectives::table
                    .filter(collectives::h_app_id.eq("test-app"))
                    .filter(collectives::id.eq(slug)),
            )
            .set(collectives::collective_cid.eq(Some(cid)))
            .execute(&mut conn)
            .expect("set collective_cid");
        }

        // X = the advertising peer's agent (household H); Y = the author (household A).
        for (agent, name, household) in [
            ("agent:uhCAkHostAgentX01", "Host Agent X", "household-h"),
            ("agent:uhCAkAuthorY0001", "Author Y", "household-a"),
        ] {
            crate::db::humans::create_human(
                &mut conn,
                CreateHumanInput {
                    id: agent.to_string(),
                    agent_pub_key: None,
                    display_name: name.to_string(),
                    bio: None,
                    affinities: "[]".to_string(),
                    profile_reach: "commons".to_string(),
                    location: None,
                    profile_photo_url: None,
                    h_app_id: "test-app".to_string(),
                    household_id: Some(household.to_string()),
                },
            )
            .expect("seed human");
        }

        // Active identity binding: local peer → agent X.
        {
            use crate::db::diesel_schema::peer_identity_bindings;
            let row = crate::db::models::NewPeerIdentityBindingRow {
                peer_id: "12D3KooWHostPeer".to_string(),
                agent_cid: "agent:uhCAkHostAgentX01".to_string(),
                dht_anchor_hash: "anchor:host-binding".to_string(),
                valid_from: "2026-01-01T00:00:00Z".to_string(),
                valid_until: None,
                observed_at: "2026-01-01T00:00:00Z".to_string(),
                source: "handshake".to_string(),
                device_archetype: "node".to_string(),
                superseded_by: None,
            };
            diesel::insert_into(peer_identity_bindings::table)
                .values(&row)
                .execute(&mut conn)
                .expect("seed binding");
        }

        // Blob authored by Y (household A) but custodied by THIS peer (household H).
        // Use 'c' — valid lowercase hex char not used in other gather_hints tests.
        let hash = sha256_wire('c');
        seed_content_with_blob(
            &mut conn,
            &hash,
            "markdown",
            Some(512),
            Some("agent:uhCAkAuthorY0001"),
        );

        let addr = BlobAddress::new(hash).unwrap();
        let hints = gather_hints(&mut conn, &[addr], "12D3KooWHostPeer");
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0].recipient_hub_id.as_deref(),
            Some("collective:uhCkkHostingH001"),
            "recipient_hub_id must be the advertising peer's dwelling hub, not the author's"
        );
    }

    /// Wave 3 T3-3: `gather_hints` populates `recipient_hub_id` via hub_resolver
    /// when the advertising peer's bound agent has a household collective.
    #[test]
    fn gather_hints_populates_recipient_hub_id() {
        use crate::db::collectives::{create_collective, CreateCollectiveInput};
        use crate::db::context::AppContext;
        use crate::db::humans::CreateHumanInput;
        use diesel::prelude::*;

        let pool = test_db_pool();
        let mut conn = pool.get().expect("conn");

        let app_ctx = AppContext::new("test-app");

        // Seed collective
        create_collective(
            &mut conn,
            &app_ctx,
            &CreateCollectiveInput {
                id: "family-hints".to_string(),
                name: "Hints Collective".to_string(),
                description: None,
                governance_layer: "family".to_string(),
                constitutional_parent_id: None,
                reach: "private".to_string(),
                metadata_json: None,
                created_by: None,
            },
        )
        .expect("seed collective");

        // Set collective_cid
        {
            use crate::db::diesel_schema::collectives;
            diesel::update(
                collectives::table
                    .filter(collectives::h_app_id.eq("test-app"))
                    .filter(collectives::id.eq("family-hints")),
            )
            .set(collectives::collective_cid.eq(Some("collective:uhCkkHintsTest001")))
            .execute(&mut conn)
            .expect("set collective_cid");
        }

        // Seed human with household_id
        crate::db::humans::create_human(
            &mut conn,
            CreateHumanInput {
                id: "agent:uhCAkHintsAuthor001".to_string(),
                agent_pub_key: None,
                display_name: "Hints Author".to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: "test-app".to_string(),
                household_id: Some("family-hints".to_string()),
            },
        )
        .expect("seed human");

        // Bind the advertising peer to the agent (custody chain, finding #3).
        {
            use crate::db::diesel_schema::peer_identity_bindings;
            let row = crate::db::models::NewPeerIdentityBindingRow {
                peer_id: "12D3KooWHintsPeer".to_string(),
                agent_cid: "agent:uhCAkHintsAuthor001".to_string(),
                dht_anchor_hash: "anchor:hints-binding".to_string(),
                valid_from: "2026-01-01T00:00:00Z".to_string(),
                valid_until: None,
                observed_at: "2026-01-01T00:00:00Z".to_string(),
                source: "handshake".to_string(),
                device_archetype: "node".to_string(),
                superseded_by: None,
            };
            diesel::insert_into(peer_identity_bindings::table)
                .values(&row)
                .execute(&mut conn)
                .expect("seed binding");
        }

        let hash = sha256_wire('b');
        seed_content_with_blob(
            &mut conn,
            &hash,
            "markdown",
            Some(4096),
            Some("agent:uhCAkHintsAuthor001"),
        );

        let addr = BlobAddress::new(hash).unwrap();
        let hints = gather_hints(&mut conn, &[addr], "12D3KooWHintsPeer");

        assert_eq!(hints.len(), 1);
        let h = &hints[0];
        assert_eq!(
            h.recipient_hub_id.as_deref(),
            Some("collective:uhCkkHintsTest001"),
            "recipient_hub_id resolved via the peer→agent→hub custody chain"
        );
        assert_eq!(h.epr_kind.as_deref(), Some("Content"));
        assert_eq!(h.size_bytes, Some(4096));
    }

    /// Wave 3 T3-4: `gather_hints` with an empty hash slice returns empty immediately.
    #[test]
    fn gather_hints_returns_empty_for_no_hashes() {
        let pool = test_db_pool();
        let mut conn = pool.get().expect("conn");
        let hints = gather_hints(&mut conn, &[], "12D3KooWUnboundPeer");
        assert!(hints.is_empty());
    }

    /// T22 review fix #4: unknown archetype strings (typos, future archetypes
    /// not yet known to this build) fall back to the conservative `node`
    /// cadence and emit a `tracing::warn!` so the misconfiguration surfaces.
    /// We assert the return value here; the warn is exercised at runtime.
    #[test]
    fn unknown_archetype_logs_warn_and_defaults_to_node() {
        // Misspelled "nod" (missing 'e') and a plausible future archetype "tablet".
        assert_eq!(resolved_cadence(Some("tablet"), None), Some(60));
        assert_eq!(resolved_cadence(Some("nod"), None), Some(60));
    }

    /// Sprint 0 Task 0.5: producer-to-verifier round-trip from real BlobStore.
    ///
    /// Proves the full produce→encode→decode→verify pipeline works end-to-end
    /// for VALID data: uses real `BlobStore` to write blobs, lists them via
    /// `list_hashes`, wraps as `BlobAddress` instances, builds a snapshot,
    /// MessagePack-round-trips, and asserts `verify_structural() == Ok(())`.
    ///
    /// After this test, producer/verifier wire-format drift becomes impossible
    /// to land green.
    #[tokio::test]
    async fn snapshot_built_from_real_blobstore_decodes_with_valid_addresses() {
        use crate::blob_store::BlobStore;
        use crate::p2p::inventory_gossip::BlobInventorySnapshot;

        let temp = tempfile::TempDir::new().unwrap();
        let store = BlobStore::new(temp.path()).await.unwrap();
        store.store(b"payload-a").await.unwrap();
        store.store(b"payload-b").await.unwrap();

        let hashes: Vec<BlobAddress> = store
            .list_hashes()
            .unwrap()
            .into_iter()
            .map(|s| {
                BlobAddress::new(s).expect("BlobStore::list_hashes returns canonical wire format")
            })
            .collect();
        assert_eq!(hashes.len(), 2, "two blobs stored, two hashes listed");

        let inv = StaticInventory::new(hashes);
        let alloc = SequenceAllocator::new(0);
        let snapshot = build_snapshot("12D3KooWtest", &inv, &alloc, 1, vec![]);

        // Round-trip through MessagePack — the wire path real gossip takes.
        let bytes = snapshot.to_bytes().unwrap();
        let decoded = BlobInventorySnapshot::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.hashes.len(), 2);
        assert_eq!(
            decoded.verify_structural(),
            Ok(()),
            "real-BlobStore snapshot must pass structural verify"
        );
    }
}

/// Filesystem-vs-gossip parity report.
///
/// Populated by `compute_parity` and served at
/// `GET /api/v1/diagnostics/inventory-parity`.  Defends against the failure
/// mode where gossip runs cleanly but bytes never replicate — inventory lists
/// diverge before blob mobility does.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ParityReport {
    /// Hashes that were included in the last gossiped snapshot but are absent
    /// from the local filesystem.  Non-empty means gossip over-reports custody.
    pub gossiped_but_missing: Vec<String>,
    /// Hashes present on the local filesystem that were not in the last gossiped
    /// snapshot.  Non-empty means gossip under-reports custody.
    pub local_but_not_gossiped: Vec<String>,
    /// Number of blobs actually on the local filesystem at time of check.
    pub filesystem_count: usize,
    /// Number of hashes in the last gossiped snapshot (0 when no snapshot has
    /// been published yet — Stage 1 baseline).
    pub gossiped_count: usize,
    /// ISO-8601 timestamp of when this report was generated.
    pub checked_at: String,
}

/// Compute a `ParityReport` by diffing `local_store.current_hashes()` against
/// `last_gossiped`.
///
/// Uses `HashSet::difference` for O(n) comparison; the returned vecs are
/// sorted for deterministic output.
pub fn compute_parity<I: LocalInventory>(
    local_store: &I,
    last_gossiped: &[String],
    now_iso: &str,
) -> ParityReport {
    let local: HashSet<String> = local_store
        .current_hashes()
        .into_iter()
        .map(|a| a.as_str().to_string())
        .collect();
    let gossiped: HashSet<String> = last_gossiped.iter().cloned().collect();

    let mut gossiped_but_missing: Vec<String> = gossiped.difference(&local).cloned().collect();
    gossiped_but_missing.sort();

    let mut local_but_not_gossiped: Vec<String> = local.difference(&gossiped).cloned().collect();
    local_but_not_gossiped.sort();

    ParityReport {
        filesystem_count: local.len(),
        gossiped_count: gossiped.len(),
        gossiped_but_missing,
        local_but_not_gossiped,
        checked_at: now_iso.to_string(),
    }
}

#[cfg(test)]
mod parity_tests {
    use super::*;
    use crate::p2p::inventory_gossip::BlobAddress;

    fn sha256_wire(byte: char) -> String {
        format!(
            "sha256-{}",
            std::iter::repeat_n(byte, 64).collect::<String>()
        )
    }

    struct MockInventory(Vec<BlobAddress>);
    impl LocalInventory for MockInventory {
        fn current_hashes(&self) -> Vec<BlobAddress> {
            self.0.clone()
        }
    }

    #[test]
    fn parity_clean_when_sets_match() {
        let addr_a = BlobAddress::new(sha256_wire('a')).unwrap();
        let addr_b = BlobAddress::new(sha256_wire('b')).unwrap();
        let hashes_str = vec![addr_a.as_str().to_string(), addr_b.as_str().to_string()];
        let inv = MockInventory(vec![addr_a, addr_b]);
        let report = compute_parity(&inv, &hashes_str, "2026-05-02T00:00:00Z");

        assert!(report.gossiped_but_missing.is_empty());
        assert!(report.local_but_not_gossiped.is_empty());
        assert_eq!(report.filesystem_count, 2);
        assert_eq!(report.gossiped_count, 2);
        assert_eq!(report.checked_at, "2026-05-02T00:00:00Z");
    }

    #[test]
    fn parity_detects_gossiped_but_missing() {
        // Gossip claims sha256_wire('c') is hosted but filesystem only has 'a' and 'b'.
        let addr_a = BlobAddress::new(sha256_wire('a')).unwrap();
        let addr_b = BlobAddress::new(sha256_wire('b')).unwrap();
        let addr_c_str = sha256_wire('c');
        let local = MockInventory(vec![addr_a.clone(), addr_b.clone()]);
        let gossiped = vec![
            addr_a.as_str().to_string(),
            addr_b.as_str().to_string(),
            addr_c_str.clone(),
        ];
        let report = compute_parity(&local, &gossiped, "2026-05-02T00:00:00Z");

        assert_eq!(report.gossiped_but_missing, vec![addr_c_str]);
        assert!(report.local_but_not_gossiped.is_empty());
        assert_eq!(report.filesystem_count, 2);
        assert_eq!(report.gossiped_count, 3);
    }

    #[test]
    fn parity_detects_local_but_not_gossiped() {
        // Filesystem has sha256_wire('d') that was never included in the gossip snapshot.
        let addr_a = BlobAddress::new(sha256_wire('a')).unwrap();
        let addr_d = BlobAddress::new(sha256_wire('d')).unwrap();
        let local = MockInventory(vec![addr_a.clone(), addr_d.clone()]);
        let gossiped = vec![addr_a.as_str().to_string()];
        let report = compute_parity(&local, &gossiped, "2026-05-02T00:00:00Z");

        assert!(report.gossiped_but_missing.is_empty());
        assert_eq!(
            report.local_but_not_gossiped,
            vec![addr_d.as_str().to_string()]
        );
        assert_eq!(report.filesystem_count, 2);
        assert_eq!(report.gossiped_count, 1);
    }
}
