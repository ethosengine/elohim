//! Inventory broadcaster: signed delta pages, sequence allocation, hints and parity.
//!
//! - On local blob add/remove events, emit signed [`InventoryEnvelope`] pages (each at
//!   most [`INVENTORY_PAGE_LIMIT`] encoded bytes, its own sequence) on
//!   [`crate::p2p::inventory_gossip::INVENTORY_TOPIC`].
//! - Full-state convergence is range reconciliation
//!   ([`crate::p2p::inventory_recon`]); the periodic cadence ([`resolved_cadence`]) now
//!   paces recon sessions. There is no snapshot.
//! - Allocate this peer's monotonic sequence numbers.
//!
//! Source of truth for "what blobs do I host": the local blob store
//! ([`crate::blob_store::BlobStore::list_cids`]), enumerated through
//! [`LocalInventory`] so it can be mocked.
//!
//! `gather_hints(conn, addresses, local_peer_id)` enriches local addresses with
//! advisory metadata from the `content` projection (`epr_kind`, `size_bytes`, the
//! advertising peer's dwelling hub as `recipient_hub_id`).

pub use crate::p2p::inventory_gossip::INVENTORY_PAGE_LIMIT;
use crate::p2p::inventory_gossip::{signed_pages, BlobAddress, BlobHint, InventoryEnvelope};
use elohim_epr::proof::AgentKeypair;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Enumerates the local blob inventory. Production: [`StaticInventory`] over
/// `BlobStore::list_cids()`. Tests: a fixed set.
pub trait LocalInventory: Send + Sync {
    fn current_addresses(&self) -> Vec<BlobAddress>;
}

/// A pre-fetched list of addresses.
///
/// Production callers fetch via `BlobStore::list_cids()` first and pass the result
/// here, so an I/O failure returns early instead of announcing an empty inventory.
pub struct StaticInventory(Vec<BlobAddress>);

impl StaticInventory {
    pub fn new(addresses: Vec<BlobAddress>) -> Self {
        Self(addresses)
    }
}

impl LocalInventory for StaticInventory {
    fn current_addresses(&self) -> Vec<BlobAddress> {
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

/// Gather advisory hints for local addresses from the `content` projection.
///
/// - `epr_kind`: the canonical EprKind entry-type (`"Content"`), the
///   `scope_filter.epr_kinds` vocabulary, never `content_format`.
/// - `size_bytes`: `content_size_bytes`.
/// - `recipient_hub_id`: the ADVERTISING peer's dwelling hub, resolved once per call.
/// - `tier`: `None`.
///
/// Database errors are logged and yield fewer (or no) hints; hints never block gossip.
pub fn gather_hints(
    conn: &mut diesel::SqliteConnection,
    addresses: &[BlobAddress],
    local_peer_id: &str,
) -> Vec<BlobHint> {
    use crate::db::diesel_schema::content::dsl as ct;
    use crate::services::hub_resolver;
    use diesel::prelude::*;

    if addresses.is_empty() {
        return Vec::new();
    }

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

    let cid_strs: Vec<String> = addresses.iter().map(|a| a.to_string()).collect();

    // One query for all matching content rows.
    let rows: Vec<(String, Option<i32>)> = match ct::content
        .filter(ct::blob_cid.eq_any(&cid_strs))
        .select((ct::blob_cid.assume_not_null(), ct::content_size_bytes))
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
    for (blob_cid, size_bytes) in rows {
        let Ok(address) = BlobAddress::parse(&blob_cid) else {
            continue;
        };
        let epr_kind = Some(
            crate::services::epr_kind::kind_canonical_str(elohim_epr::EprKind::Content).to_string(),
        );
        hints.push(BlobHint {
            tier: None,
            address,
            epr_kind,
            size_bytes: size_bytes.map(|s| s.max(0) as u64),
            recipient_hub_id: recipient_hub_id.clone(),
        });
    }
    hints
}

/// Build the signed pages announcing `added`/`removed`, each with the next sequence
/// from `seq`, signed by `keypair` as `author_peer` (whose embedded key it must be).
pub fn build_delta_pages(
    keypair: &AgentKeypair,
    author_peer: &str,
    added: &[BlobAddress],
    removed: &[BlobAddress],
    seq: &SequenceAllocator,
    now_micros: i64,
    hints: &[BlobHint],
) -> Vec<InventoryEnvelope> {
    signed_pages(
        keypair,
        author_peer,
        added,
        removed,
        hints,
        now_micros,
        || seq.next(),
    )
}

/// The resolved cadence for this peer's archetype, honoring the 4-layer override
/// pattern (archetype default ← policy.toml ← env/CLI ← admin trigger). `None` means
/// disabled. It paces recon sessions with known holders.
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

    fn cid_wire(byte: char) -> String {
        elohim_epr::address::compute_bytes_cid(&[byte as u8]).to_string()
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

    /// `StaticInventory` returns the addresses it was given; the production caller
    /// fetches them via `BlobStore::list_cids()` first and returns early on I/O
    /// failure.
    #[test]
    fn static_inventory_returns_provided_addresses() {
        let addr_a = BlobAddress::parse(&cid_wire('a')).unwrap();
        let addr_b = BlobAddress::parse(&cid_wire('b')).unwrap();
        let inv = StaticInventory::new(vec![addr_a, addr_b]);
        let hashes = inv.current_addresses();
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
        blob_cid: &str,
        content_format: &str,
        size_bytes: Option<i32>,
        created_by: Option<&str>,
    ) {
        use crate::db::diesel_schema::content::dsl as ct;
        use diesel::prelude::*;
        diesel::insert_into(ct::content)
            .values((
                ct::id.eq(format!("content-{blob_cid}")),
                ct::h_app_id.eq("test-app"),
                ct::title.eq("test"),
                ct::content_type.eq("Concept"),
                ct::content_format.eq(content_format),
                ct::blob_cid.eq(blob_cid),
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
        let addr = BlobAddress::parse(&cid_wire('f')).unwrap();
        let hints = gather_hints(&mut conn, &[addr], "12D3KooWUnboundPeer");
        assert!(hints.is_empty(), "no matching content → no hints");
    }

    /// Wave 3 T3-2: `gather_hints` populates epr_kind and size_bytes when content exists.
    #[test]
    fn gather_hints_populates_epr_kind_and_size() {
        let pool = test_db_pool();
        let mut conn = pool.get().expect("conn");

        let hash = cid_wire('a');
        seed_content_with_blob(&mut conn, &hash, "sophia-quiz-json", Some(8192), None);

        let addr = BlobAddress::parse(&hash.clone()).unwrap();
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
        let hash = cid_wire('d');
        seed_content_with_blob(&mut conn, &hash, "markdown", Some(1024), None);
        let addr = BlobAddress::parse(&hash).unwrap();
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
        let hash = cid_wire('e');
        seed_content_with_blob(&mut conn, &hash, "markdown", Some(2048), None);
        let addr = BlobAddress::parse(&hash).unwrap();
        let hints = gather_hints(&mut conn, &[addr], "12D3KooWUnboundPeer");
        assert_eq!(hints.len(), 1);
        let h = &hints[0];

        // Mirror the receive-arm BlobHint → AdvertisedBlob mapping (p2p/mod.rs).
        let advertised = AdvertisedBlob {
            blob_cid: h.address.to_string(),
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
            head_ref: None,
        };
        assert_eq!(
            score_advertised_blob(&advertised, &[commitment], None),
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
                    region: None,
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
                signature: String::new(),
                proof_status: crate::p2p::binding_proof_wire::BindingProofStatus::unverified(),
            };
            diesel::insert_into(peer_identity_bindings::table)
                .values(&row)
                .execute(&mut conn)
                .expect("seed binding");
        }

        // Blob authored by Y (household A) but custodied by THIS peer (household H).
        // Use 'c' — valid lowercase hex char not used in other gather_hints tests.
        let hash = cid_wire('c');
        seed_content_with_blob(
            &mut conn,
            &hash,
            "markdown",
            Some(512),
            Some("agent:uhCAkAuthorY0001"),
        );

        let addr = BlobAddress::parse(&hash).unwrap();
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
                region: None,
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
                signature: String::new(),
                proof_status: crate::p2p::binding_proof_wire::BindingProofStatus::unverified(),
            };
            diesel::insert_into(peer_identity_bindings::table)
                .values(&row)
                .execute(&mut conn)
                .expect("seed binding");
        }

        let hash = cid_wire('b');
        seed_content_with_blob(
            &mut conn,
            &hash,
            "markdown",
            Some(4096),
            Some("agent:uhCAkHintsAuthor001"),
        );

        let addr = BlobAddress::parse(&hash).unwrap();
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

    /// Producer to verifier, end to end: real BlobStore listing, signed pages,
    /// admission under the receiver's rules, projection into the real
    /// `peer_blob_inventory` cursor.
    #[tokio::test]
    async fn pages_from_a_real_blobstore_are_admitted_and_projected() {
        use crate::blob_store::BlobStore;
        use crate::p2p::inventory_gossip::{
            admit, Admission, AuthorWitness, HolderResolver, INVENTORY_PAGE_LIMIT,
        };
        use elohim_epr::signature::Multikey;

        struct Expect(Multikey);
        impl HolderResolver for Expect {
            fn expected_holder(&self, author_peer: &str) -> Option<Multikey> {
                (author_peer == "12D3KooWtest").then(|| self.0.clone())
            }
        }

        let temp = tempfile::TempDir::new().unwrap();
        let store = BlobStore::new(temp.path()).await.unwrap();
        store.store(b"payload-a").await.unwrap();
        store.store(b"payload-b").await.unwrap();
        let addresses: Vec<BlobAddress> = store
            .list_cids()
            .unwrap()
            .into_iter()
            .map(BlobAddress::from)
            .collect();
        assert_eq!(addresses.len(), 2);

        let keypair = AgentKeypair::from_secret(&[3u8; 32]).unwrap();
        let resolver = Expect(Multikey::ed25519(&keypair.public_key_bytes()));
        let seq = SequenceAllocator::new(10);
        let pages = build_delta_pages(&keypair, "12D3KooWtest", &addresses, &[], &seq, 1, &[]);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].body.sequence, 11);

        let pool = test_db_pool();
        let mut conn = pool.get().unwrap();
        for page in pages {
            let bytes = page.to_bytes();
            assert!(bytes.len() <= INVENTORY_PAGE_LIMIT);
            let Admission::Admitted(env) = admit(
                &bytes,
                AuthorWitness::Gossipsub(Some("12D3KooWtest")),
                &resolver,
            ) else {
                panic!("a page this node signs must be admitted");
            };
            let added: Vec<String> = env.body.added.iter().map(|a| a.to_string()).collect();
            crate::db::peer_blob_inventory::apply_delta(
                &mut conn,
                &env.author_peer,
                &added,
                &[],
                env.body.sequence as i64,
                "2026-11-05T00:00:00Z",
                crate::db::peer_blob_inventory::InventoryWitness::SignedAuthor,
            )
            .unwrap();
        }
        for address in &addresses {
            let hosts = crate::db::peer_blob_inventory::lookup_hosts(
                &mut conn,
                &address.to_string(),
                "2026-11-04T00:00:00Z",
            )
            .unwrap();
            assert_eq!(hosts.len(), 1);
        }
    }

    #[test]
    fn delta_pages_carry_added_and_removed_in_sequence() {
        let keypair = AgentKeypair::from_secret(&[3u8; 32]).unwrap();
        let alloc = SequenceAllocator::new(0);
        let pages = build_delta_pages(
            &keypair,
            "12D3KooWtest",
            &[BlobAddress::parse(&cid_wire('a')).unwrap()],
            &[BlobAddress::parse(&cid_wire('b')).unwrap()],
            &alloc,
            1_700_000_001_000_000,
            &[],
        );
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].body.added.len(), 1);
        assert_eq!(pages[0].body.removed.len(), 1);
        assert_eq!(pages[0].body.sequence, 1);
        assert_eq!(pages[0].author_peer, "12D3KooWtest");
    }

    #[test]
    fn shard_only_inventory_makes_that_peer_specific_to_the_matching_shard() {
        let shard_cids: Vec<String> = (0..3u64)
            .map(|i| elohim_epr::address::compute_bytes_cid(&i.to_le_bytes()).to_string())
            .collect();
        let pool = test_db_pool();
        let mut conn = pool.get().unwrap();
        crate::db::peer_blob_inventory::apply_delta(
            &mut conn,
            "peer-B",
            &[shard_cids[2].clone()],
            &[],
            1,
            "2026-08-23T00:00:00Z",
            crate::db::peer_blob_inventory::InventoryWitness::SignedAuthor,
        )
        .unwrap();

        let rows = crate::db::peer_blob_inventory::lookup_hosts(
            &mut conn,
            &shard_cids[2],
            "2026-08-22T00:00:00Z",
        )
        .unwrap();
        // B (minimal compile fix): `plan_shard_holders` takes byte CIDs.
        let units: Vec<elohim_epr::address::ByteCid> = shard_cids
            .iter()
            .map(|c| elohim_epr::address::ByteCid::parse(c).unwrap())
            .collect();
        let mut per_shard = std::collections::HashMap::new();
        per_shard.insert(units[2], rows.into_iter().map(|row| row.peer_id).collect());
        let plan =
            crate::p2p::blob_swarm::plan_shard_holders(&units, &["peer-A".to_string()], &per_shard);

        assert_eq!(plan[0].1, vec!["peer-A"]);
        assert_eq!(plan[1].1, vec!["peer-A"]);
        assert_eq!(plan[2].1, vec!["peer-B"]);
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
    /// Hashes that were included in the last published inventory but are absent
    /// from the local filesystem.  Non-empty means gossip over-reports custody.
    pub gossiped_but_missing: Vec<String>,
    /// Hashes present on the local filesystem that were not in the last published
    /// inventory.  Non-empty means gossip under-reports custody.
    pub local_but_not_gossiped: Vec<String>,
    /// Number of blobs actually on the local filesystem at time of check.
    pub filesystem_count: usize,
    /// Number of addresses in the last published inventory (0 when nothing has
    /// been published yet — Stage 1 baseline).
    pub gossiped_count: usize,
    /// ISO-8601 timestamp of when this report was generated.
    pub checked_at: String,
}

/// Compute a `ParityReport` by diffing `local_store.current_addresses()` against
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
        .current_addresses()
        .into_iter()
        .map(|a| a.to_string())
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

    fn cid_wire(byte: char) -> String {
        elohim_epr::address::compute_bytes_cid(&[byte as u8]).to_string()
    }

    struct MockInventory(Vec<BlobAddress>);
    impl LocalInventory for MockInventory {
        fn current_addresses(&self) -> Vec<BlobAddress> {
            self.0.clone()
        }
    }

    #[test]
    fn parity_clean_when_sets_match() {
        let addr_a = BlobAddress::parse(&cid_wire('a')).unwrap();
        let addr_b = BlobAddress::parse(&cid_wire('b')).unwrap();
        let hashes_str = vec![addr_a.to_string(), addr_b.to_string()];
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
        // Gossip claims cid_wire('c') is hosted but filesystem only has 'a' and 'b'.
        let addr_a = BlobAddress::parse(&cid_wire('a')).unwrap();
        let addr_b = BlobAddress::parse(&cid_wire('b')).unwrap();
        let addr_c_str = cid_wire('c');
        let local = MockInventory(vec![addr_a, addr_b]);
        let gossiped = vec![addr_a.to_string(), addr_b.to_string(), addr_c_str.clone()];
        let report = compute_parity(&local, &gossiped, "2026-05-02T00:00:00Z");

        assert_eq!(report.gossiped_but_missing, vec![addr_c_str]);
        assert!(report.local_but_not_gossiped.is_empty());
        assert_eq!(report.filesystem_count, 2);
        assert_eq!(report.gossiped_count, 3);
    }

    #[test]
    fn parity_detects_local_but_not_gossiped() {
        // Filesystem has cid_wire('d') that was never included in the published inventory.
        let addr_a = BlobAddress::parse(&cid_wire('a')).unwrap();
        let addr_d = BlobAddress::parse(&cid_wire('d')).unwrap();
        let local = MockInventory(vec![addr_a, addr_d]);
        let gossiped = vec![addr_a.to_string()];
        let report = compute_parity(&local, &gossiped, "2026-05-02T00:00:00Z");

        assert!(report.gossiped_but_missing.is_empty());
        assert_eq!(report.local_but_not_gossiped, vec![addr_d.to_string()]);
        assert_eq!(report.filesystem_count, 2);
        assert_eq!(report.gossiped_count, 1);
    }
}
