//! Resolves the facts [`crate::private_reach::private_serve_verdict`] decides on.
//!
//! The predicate is pure; everything impure lives here: which agent a transport
//! identity resolves to, whose row this is, and which live custody commitments
//! the requester provides. Station 3b (M9).
//!
//! ## Where standing comes from
//!
//! From `rea_commitments` — the local projection of notarized `Commitment`
//! entries on the elohim DNA — and never from anything the requester says (C5).
//! Two actions carry it:
//!
//! * `custody-spool` (provider = custodian, receiver = ward): the STANDING
//!   consent [`crate::services::spool_custody_author`] expands per witness.
//! * `custody-blob` (provider = custodian, receiver = ward,
//!   `resource_classified_as` = `sha256-<digest>`): the per-blob pledge.
//!
//! "Live" is defined once, by [`crate::services::spool_custody_author::RETIRED_STATES`]
//! — gating on a single state string would silently disable the station, since
//! the DNA mints `created` and the accept path walks
//! `proposed`/`accepted`/`activated`/`active`.
//!
//! ## Bounded work (C6a)
//!
//! Commitments converge from the OWN conductor with peers as discovery only
//! (`p2p::projection_reconcile`), so the ward's peer may not yet hold a
//! custodian's `custody-spool` projection when that custodian asks. The id is
//! deterministic ([`deterministic_spool_custody_id`]), so this resolver can ask
//! its OWN conductor for the exact id — but that call is uncancellable, so it is
//! bounded BEFORE it is made:
//!
//! * at most ONE conductor call per distinct `(requester_agent, ward)` per TTL
//!   (60 s), cached in BOTH signs, so a negative answer costs one call per
//!   minute rather than one per row;
//! * it is skipped entirely when the local projection already answered, when the
//!   requester IS the ward, or when a blob-custody pledge already stands;
//! * every other lookup is a set built by ONE indexed SELECT per agent per TTL,
//!   so a `ListContent` page of N private rows performs O(1) queries, not O(N).
//!
//! A non-`private` row costs NOTHING: [`ProjectionCustodyStanding::facts_for`]
//! returns before touching the pool.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use diesel::prelude::*;
use tracing::{debug, warn};

use crate::blob_store::BlobStore;
use crate::db::DbPool;
use crate::private_reach::{is_private, PrivateServeFacts};
use crate::services::rea_commitment_service::{
    deterministic_spool_custody_id, spool_classification,
};
use crate::services::spool_custody_author::{RETIRED_STATES, SPOOL_CUSTODY_ACTION};

/// The REA action a per-blob custody pledge carries. Sibling of
/// [`SPOOL_CUSTODY_ACTION`]; both ride the existing elohim-DNA `Commitment`
/// entry type (no new entry type — see `p2p-design-gate`).
pub const CUSTODY_BLOB_ACTION: &str = "custody-blob";

/// How long a resolved fact stands before it is re-read. Covers the identity
/// resolution, both custody sets, and BOTH signs of the conductor fallback.
pub const STANDING_TTL: Duration = Duration::from_secs(60);

/// Row cap on any one custody-set SELECT. A peer providing more live custody
/// pledges than this answers from the capped set — the cap is far above any
/// household pledge count and exists so one poisoned projection cannot turn a
/// per-page bounded read into an unbounded one.
const MAX_CUSTODY_ROWS: i64 = 5_000;

type CustodyKey = (String, String);
type ConductorSingleflight =
    Arc<tokio::sync::Mutex<HashMap<CustodyKey, Arc<tokio::sync::Mutex<()>>>>>;
type SpoolWardProjectionRow = (String, String, Option<String>, String, Option<String>);
type CustodyBlobProjectionRow = (String, Option<String>, String, String, Option<String>);

// ─────────────────────────────────────────────────────────────────────────────
// Requester
// ─────────────────────────────────────────────────────────────────────────────

/// The transport identity a shard request arrived under.
///
/// Deliberately holds STRINGS, not `libp2p::PeerId` / `iroh::NodeId`: this
/// module lives under `services/` (not feature-gated on `p2p` / `p2p-iroh`),
/// and the identity namespaces are never string-compared across each other —
/// each variant is resolved through its OWN projection
/// (`peer_identity_bindings` for libp2p, `peer_transport_manifest` for iroh).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransportId {
    /// libp2p peer id, base58 (`12D3Koo…`) — as `PeerId::to_base58()` writes it.
    Libp2p(String),
    /// iroh `NodeId`, in the rendering `peer_transport_manifest.iroh_node_id` stores.
    Iroh(String),
    /// No remote transport: an in-process caller on this node's own storage.
    Local,
}

/// Who is asking. One field today; a struct rather than a bare enum so a later
/// station can carry the connection's observed facts without changing every
/// call site's shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Requester {
    pub transport: TransportId,
}

impl Requester {
    /// A libp2p peer, by its base58 rendering.
    pub fn libp2p(peer_id_b58: impl Into<String>) -> Self {
        Self {
            transport: TransportId::Libp2p(peer_id_b58.into()),
        }
    }

    /// An iroh node, by its `NodeId` rendering.
    pub fn iroh(node_id: impl Into<String>) -> Self {
        Self {
            transport: TransportId::Iroh(node_id.into()),
        }
    }

    /// An in-process caller (this node's own storage). NOT a remote peer.
    pub fn local() -> Self {
        Self {
            transport: TransportId::Local,
        }
    }

    /// Bounded, single-line label for logs and cache keys.
    pub fn label(&self) -> String {
        match &self.transport {
            TransportId::Libp2p(id) => format!("libp2p:{id}"),
            TransportId::Iroh(id) => format!("iroh:{id}"),
            TransportId::Local => "local".to_string(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Row facts
// ─────────────────────────────────────────────────────────────────────────────

/// The parts of a `content` row this gate reads. Built by the caller from the
/// row it already loaded, so the gate never re-queries what the serve path has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowFacts {
    /// Declared audience, verbatim.
    pub reach: String,
    /// Content id — for logs only; standing never keys on it.
    pub id: String,
    /// Canonical lowercase multihash digest of the row's bytes, if it has any.
    /// Built with [`BlobStore::parse_content_address`] so `sha256-<hex>` and
    /// `bafkrei…` collapse to ONE key (identity = one digest, two renderings).
    pub blob_digest: Option<String>,
    /// The ark passport `node` of the berth that produced the witness. Per
    /// `ark_core::passport::Passport::node` this is an **agent CID string**
    /// once identity is available — so on both the ward's own copy and a
    /// replicated copy it names the WARD, not a berth label.
    pub created_by: Option<String>,
    /// Whether the row carries a DHT anchor. Recorded for the ward-resolution
    /// audit trail; a locally-ingested witness has none.
    pub dht_anchor_hash_present: bool,
    /// `metadata.kind`, when the row's metadata is a JSON object. The local
    /// unanchored fallback is restricted to exactly `death-witness`.
    pub metadata_kind: Option<String>,
}

impl RowFacts {
    /// Build from a projected `content` row.
    pub fn from_content(row: &crate::db::models::Content) -> Self {
        let digest = row
            .blob_hash
            .as_deref()
            .or(row.blob_cid.as_deref())
            .and_then(digest_key);
        Self {
            reach: row.reach.clone(),
            id: row.id.clone(),
            blob_digest: digest,
            created_by: row.created_by.clone(),
            dht_anchor_hash_present: row.dht_anchor_hash.is_some(),
            metadata_kind: row
                .metadata_json
                .as_deref()
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                .and_then(|metadata| {
                    metadata
                        .get("kind")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                }),
        }
    }
}

/// Canonical digest key shared by every accepted content-address rendering
/// (`sha256-<hex>`, bare hex, `bafkrei…`). Lowercased so a set membership test
/// cannot miss on case.
pub fn digest_key(address: &str) -> Option<String> {
    BlobStore::parse_content_address(address)
        .ok()
        .map(|d| d.to_ascii_lowercase())
}

// ─────────────────────────────────────────────────────────────────────────────
// The trait
// ─────────────────────────────────────────────────────────────────────────────

/// Resolves [`PrivateServeFacts`] for one (requester, row) pair.
///
/// Async because the bounded own-conductor fallback is a real zome call. A
/// non-`private` row must never await anything.
#[async_trait::async_trait]
pub trait CustodyStanding: Send + Sync + 'static {
    async fn facts_for(&self, requester: &Requester, row: &RowFacts) -> PrivateServeFacts;
}

// ─────────────────────────────────────────────────────────────────────────────
// Production resolver
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Cached<T> {
    value: T,
    at: Instant,
}

impl<T: Clone> Cached<T> {
    fn fresh(&self) -> Option<T> {
        if self.at.elapsed() < STANDING_TTL {
            Some(self.value.clone())
        } else {
            None
        }
    }
}

#[derive(Default)]
struct StandingCache {
    self_agent: Option<Cached<Option<String>>>,
    /// requester label → agent cid
    requester: HashMap<String, Cached<Option<String>>>,
    /// custodian agent → wards it provides a live `custody-spool` for
    spool_wards: HashMap<String, Cached<HashSet<String>>>,
    /// custodian agent → digests it provides a live `custody-blob` for
    blob_digests: HashMap<String, Cached<HashSet<String>>>,
    /// digest → ward, from THIS peer's own live `custody-blob` rows
    own_blob_wards: Option<Cached<HashMap<String, String>>>,
    /// (requester agent, ward) → the own-conductor answer, BOTH signs
    conductor: HashMap<CustodyKey, Cached<bool>>,
}

/// The conductor evidence needed by this read gate. Keeping the fallback
/// behind this narrow seam makes positive, negative, TTL and single-flight
/// behavior testable without a live conductor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorSpoolEvidence {
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_classified_as: Vec<String>,
    pub finished: bool,
    pub state: String,
}

#[async_trait::async_trait]
pub trait ConductorCustodyLookup: Send + Sync + 'static {
    async fn get_spool_commitment(
        &self,
        id: &str,
    ) -> Result<Option<ConductorSpoolEvidence>, crate::error::StorageError>;
}

struct HcConductorCustodyLookup {
    hc: Arc<crate::hc_client::HcClient>,
}

#[async_trait::async_trait]
impl ConductorCustodyLookup for HcConductorCustodyLookup {
    async fn get_spool_commitment(
        &self,
        id: &str,
    ) -> Result<Option<ConductorSpoolEvidence>, crate::error::StorageError> {
        Ok(
            crate::services::conductor_writes::get_rea_commitment(&self.hc, id)
                .await?
                .map(|out| {
                    let commitment = out.commitment;
                    let resource_classified_as = crate::db::rea_commitments::classifications_of(
                        Some(commitment.resource_classified_as_json.as_str()),
                    );
                    ConductorSpoolEvidence {
                        action: commitment.action,
                        provider: commitment.provider,
                        receiver: commitment.receiver,
                        resource_classified_as,
                        finished: commitment.finished,
                        state: commitment.state,
                    }
                }),
        )
    }
}

/// Reads standing from the local `rea_commitments` projection, with a bounded
/// own-conductor fallback by deterministic id.
pub struct ProjectionCustodyStanding {
    pool: DbPool,
    /// Retained separately from the lookup trait so self-agent resolution can
    /// use the conductor cell key when no active local session exists.
    hc: Option<Arc<crate::hc_client::HcClient>>,
    /// The lamad/content_store conductor bridge. `None` ⇒ no fallback; the local
    /// projection alone decides (a missing projection then reads as no standing,
    /// which is fail-closed).
    conductor: Option<Arc<dyn ConductorCustodyLookup>>,
    #[cfg(feature = "p2p")]
    libp2p_identity_map: Option<Arc<dyn crate::p2p::identity_map::PeerIdentityMap>>,
    /// Whether this process owns the configured ark spool. This is evidence
    /// used only by the unanchored local death-witness ward fallback.
    spool_ingest_enabled: bool,
    cache: Arc<Mutex<StandingCache>>,
    /// One mutex per in-flight `(requester, ward)` fallback. A waiter rechecks
    /// the TTL cache after acquiring it, so only the first miss dispatches.
    conductor_singleflight: ConductorSingleflight,
}

impl Clone for ProjectionCustodyStanding {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            hc: self.hc.clone(),
            conductor: self.conductor.clone(),
            #[cfg(feature = "p2p")]
            libp2p_identity_map: self.libp2p_identity_map.clone(),
            spool_ingest_enabled: self.spool_ingest_enabled,
            cache: Arc::clone(&self.cache),
            conductor_singleflight: Arc::clone(&self.conductor_singleflight),
        }
    }
}

impl std::fmt::Debug for ProjectionCustodyStanding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionCustodyStanding")
            .field("has_conductor_fallback", &self.conductor.is_some())
            .field("spool_ingest_enabled", &self.spool_ingest_enabled)
            .finish_non_exhaustive()
    }
}

impl ProjectionCustodyStanding {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            hc: None,
            conductor: None,
            #[cfg(feature = "p2p")]
            libp2p_identity_map: None,
            spool_ingest_enabled: false,
            cache: Arc::new(Mutex::new(StandingCache::default())),
            conductor_singleflight: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Attach the own-conductor bridge used for the deterministic-id fallback.
    pub fn with_conductor(mut self, hc: Option<Arc<crate::hc_client::HcClient>>) -> Self {
        self.hc = hc.clone();
        self.conductor = hc
            .map(|hc| Arc::new(HcConductorCustodyLookup { hc }) as Arc<dyn ConductorCustodyLookup>);
        self
    }

    /// Test/composition seam for the exact-id conductor lookup.
    pub fn with_conductor_lookup(mut self, lookup: Arc<dyn ConductorCustodyLookup>) -> Self {
        self.conductor = Some(lookup);
        self
    }

    /// Declare that this process owns the ark spool ingest path.
    pub fn with_spool_ingest(mut self, enabled: bool) -> Self {
        self.spool_ingest_enabled = enabled;
        self
    }

    /// Use the same libp2p identity seam held by `P2PNode`.
    #[cfg(feature = "p2p")]
    pub fn with_libp2p_identity_map(
        mut self,
        identity_map: Arc<dyn crate::p2p::identity_map::PeerIdentityMap>,
    ) -> Self {
        self.libp2p_identity_map = Some(identity_map);
        self
    }

    fn conn(
        &self,
    ) -> Option<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<SqliteConnection>>>
    {
        match self.pool.get() {
            Ok(c) => Some(c),
            Err(e) => {
                warn!(error = %e, "custody-standing: pool exhausted — every private row fails closed");
                None
            }
        }
    }

    /// This node's own holochain `agent_cid`. Never a transport id.
    fn self_agent(&self) -> Option<String> {
        if let Some(hit) = self
            .cache
            .lock()
            .ok()
            .and_then(|c| c.self_agent.as_ref().and_then(Cached::fresh))
        {
            return hit;
        }
        let mut conn = self.conn()?;
        let resolved = match self.hc.as_ref() {
            Some(hc) => {
                crate::services::salvage_commitment_author::resolve_self_agent_cid(&mut conn, hc)
            }
            None => {
                let session = crate::db::local_sessions::get_active_session(&mut conn)
                    .ok()
                    .flatten()
                    .map(|s| s.agent_pub_key);
                crate::identity_namespace::resolve_agent_cid_write(&[session.as_deref()])
            }
        };
        if let Ok(mut c) = self.cache.lock() {
            c.self_agent = Some(Cached {
                value: resolved.clone(),
                at: Instant::now(),
            });
        }
        resolved
    }

    /// Resolve the requesting transport identity to an agent CID.
    ///
    /// Each namespace through its own projection — NEVER a raw-string match
    /// across namespaces (that is the join that silently empties, per the
    /// crate's identity-coherence rule).
    fn requester_agent(&self, requester: &Requester) -> Option<String> {
        let key = requester.label();
        if let Some(hit) = self
            .cache
            .lock()
            .ok()
            .and_then(|c| c.requester.get(&key).and_then(Cached::fresh))
        {
            return hit;
        }
        let resolved = match &requester.transport {
            TransportId::Local => self.self_agent(),
            TransportId::Libp2p(peer_b58) => {
                #[cfg(feature = "p2p")]
                {
                    let peer = match peer_b58.parse::<libp2p::PeerId>() {
                        Ok(peer) => peer,
                        Err(e) => {
                            warn!(peer = %peer_b58, error = %e, "custody-standing: invalid libp2p requester id");
                            return None;
                        }
                    };
                    self.libp2p_identity_map
                        .as_ref()
                        .and_then(|map| match map.lookup(&peer) {
                            crate::p2p::identity_map::CallerIdentity::Agent(agent) => Some(agent),
                            crate::p2p::identity_map::CallerIdentity::Anonymous => None,
                        })
                }
                #[cfg(not(feature = "p2p"))]
                {
                    let _ = peer_b58;
                    None
                }
            }
            TransportId::Iroh(node_id) => {
                let mut conn = self.conn()?;
                match p2p_iroh_peer_lookup(&mut conn, node_id) {
                    Ok(agent) => agent,
                    Err(e) => {
                        warn!(node = %node_id, error = %e, "custody-standing: iroh manifest lookup failed");
                        None
                    }
                }
            }
        };
        if let Ok(mut c) = self.cache.lock() {
            c.requester.insert(
                key,
                Cached {
                    value: resolved.clone(),
                    at: Instant::now(),
                },
            );
        }
        resolved
    }

    /// Wards this agent provides a LIVE `custody-spool` for. ONE indexed SELECT
    /// per agent per TTL — the per-page bound.
    fn spool_wards(&self, agent: &str) -> HashSet<String> {
        if let Some(hit) = self
            .cache
            .lock()
            .ok()
            .and_then(|c| c.spool_wards.get(agent).and_then(Cached::fresh))
        {
            return hit;
        }
        let mut set = HashSet::new();
        if let Some(mut conn) = self.conn() {
            use crate::db::diesel_schema::rea_commitments as rc;
            let rows: Vec<SpoolWardProjectionRow> = rc::table
                .filter(rc::action.eq(SPOOL_CUSTODY_ACTION))
                .filter(rc::provider.eq(agent))
                .filter(rc::finished.eq(0))
                .select((
                    rc::id,
                    rc::receiver,
                    rc::resource_classified_as,
                    rc::state,
                    rc::dht_anchor_hash,
                ))
                .limit(MAX_CUSTODY_ROWS)
                .load(&mut conn)
                .unwrap_or_default();
            for (id, receiver, classified, state, dht_anchor_hash) in rows {
                let expected = spool_classification(&receiver);
                let classification_matches =
                    crate::db::rea_commitments::classifications_of(classified.as_deref())
                        .iter()
                        .any(|value| value == &expected);
                if RETIRED_STATES.contains(&state.as_str()) || !classification_matches {
                    continue;
                }
                if dht_anchor_hash.is_none() {
                    debug!(
                        commitment = %id,
                        provider = %agent,
                        ward = %receiver,
                        "custody-standing: granting spool standing from an unanchored projection row"
                    );
                }
                set.insert(receiver);
            }
        }
        if let Ok(mut c) = self.cache.lock() {
            c.spool_wards.insert(
                agent.to_string(),
                Cached {
                    value: set.clone(),
                    at: Instant::now(),
                },
            );
        }
        set
    }

    /// Digests this agent provides a LIVE `custody-blob` for. ONE SELECT per
    /// agent per TTL.
    fn blob_digests(&self, agent: &str) -> HashSet<String> {
        if let Some(hit) = self
            .cache
            .lock()
            .ok()
            .and_then(|c| c.blob_digests.get(agent).and_then(Cached::fresh))
        {
            return hit;
        }
        let mut set = HashSet::new();
        for (classified, _receiver) in self.live_custody_blob_rows(agent) {
            if let Some(digest) = digest_key(&classified) {
                set.insert(digest);
            }
        }
        if let Ok(mut c) = self.cache.lock() {
            c.blob_digests.insert(
                agent.to_string(),
                Cached {
                    value: set.clone(),
                    at: Instant::now(),
                },
            );
        }
        set
    }

    /// digest → ward, from THIS peer's own live `custody-blob` rows. The
    /// fallback ward resolution for a replicated copy whose `created_by` did
    /// not name an agent. ONE SELECT per TTL.
    fn own_blob_wards(&self) -> HashMap<String, String> {
        if let Some(hit) = self
            .cache
            .lock()
            .ok()
            .and_then(|c| c.own_blob_wards.as_ref().and_then(Cached::fresh))
        {
            return hit;
        }
        let mut map = HashMap::new();
        if let Some(self_agent) = self.self_agent() {
            for (classified, receiver) in self.live_custody_blob_rows(&self_agent) {
                if let Some(digest) = digest_key(&classified) {
                    map.insert(digest, receiver);
                }
            }
        }
        if let Ok(mut c) = self.cache.lock() {
            c.own_blob_wards = Some(Cached {
                value: map.clone(),
                at: Instant::now(),
            });
        }
        map
    }

    /// `(classification, receiver)` for every LIVE `custody-blob` this agent
    /// provides. One classification per element — the column is a JSON list by
    /// contract, read through [`crate::db::rea_commitments::classifications_of`].
    fn live_custody_blob_rows(&self, agent: &str) -> Vec<(String, String)> {
        let Some(mut conn) = self.conn() else {
            return Vec::new();
        };
        use crate::db::diesel_schema::rea_commitments as rc;
        let rows: Vec<CustodyBlobProjectionRow> = rc::table
            .filter(rc::action.eq(CUSTODY_BLOB_ACTION))
            .filter(rc::provider.eq(agent))
            .filter(rc::finished.eq(0))
            .select((
                rc::id,
                rc::resource_classified_as,
                rc::receiver,
                rc::state,
                rc::dht_anchor_hash,
            ))
            .limit(MAX_CUSTODY_ROWS)
            .load(&mut conn)
            .unwrap_or_default();
        rows.into_iter()
            .filter(|(_, _, _, state, _)| !RETIRED_STATES.contains(&state.as_str()))
            .flat_map(|(id, classified, receiver, _, dht_anchor_hash)| {
                if dht_anchor_hash.is_none() {
                    debug!(
                        commitment = %id,
                        provider = %agent,
                        ward = %receiver,
                        "custody-standing: granting blob standing from an unanchored projection row"
                    );
                }
                crate::db::rea_commitments::classifications_of(classified.as_deref())
                    .into_iter()
                    .map(move |c| (c, receiver.clone()))
            })
            .collect()
    }

    /// Whose private row is this?
    ///
    /// 1. `created_by` when it is agent-CID-shaped. The ark passport's `node`
    ///    IS an agent CID once identity is available, and it rides the
    ///    `ContentRecord` wire — so this names the ward on the ward's OWN copy
    ///    and on a replicated copy alike.
    /// 2. else, the `receiver` of this peer's own live `custody-blob` for the
    ///    digest — the custodian's answer when the passport carried no identity.
    /// 3. else, an unanchored death-witness owned by this process's configured
    ///    spool ingest belongs to this peer's own agent.
    /// 4. else `None` → `WardUnresolved`.
    fn ward_for(&self, row: &RowFacts) -> Option<String> {
        if let Some(created_by) = row.created_by.as_deref() {
            if crate::identity_namespace::is_agent_cid(created_by) {
                return Some(created_by.to_string());
            }
        }
        if let Some(digest) = row.blob_digest.as_deref() {
            if let Some(ward) = self.own_blob_wards().get(digest).cloned() {
                return Some(ward);
            }
        }
        if !row.dht_anchor_hash_present
            && row.metadata_kind.as_deref() == Some("death-witness")
            && self.spool_ingest_enabled
        {
            return self.self_agent();
        }
        None
    }

    /// The bounded own-conductor fallback: does `requester` hold a live
    /// `custody-spool` for `ward` that this peer's projection has not yet
    /// converged?
    ///
    /// ONE call per `(requester, ward)` per TTL, cached in both signs. NOT
    /// wrapped in a timeout: a conductor zome call is uncancellable, so the
    /// budget is spent by bounding how often it is made, never by dropping it
    /// mid-flight.
    async fn conductor_spool_standing(&self, requester_agent: &str, ward: &str) -> bool {
        let key = (requester_agent.to_string(), ward.to_string());
        if let Some(hit) = self.cached_conductor(&key) {
            return hit;
        }
        let flight = {
            let mut flights = self.conductor_singleflight.lock().await;
            Arc::clone(
                flights
                    .entry(key.clone())
                    .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
            )
        };
        let guard = flight.lock().await;
        if let Some(hit) = self.cached_conductor(&key) {
            self.conductor_singleflight.lock().await.remove(&key);
            drop(guard);
            return hit;
        }
        let Some(conductor) = self.conductor.clone() else {
            self.remember_conductor(key.clone(), false);
            self.conductor_singleflight.lock().await.remove(&key);
            drop(guard);
            return false;
        };
        let id = deterministic_spool_custody_id(requester_agent, ward, ward);
        let holds = match conductor.get_spool_commitment(&id).await {
            Ok(Some(c)) => {
                !c.finished
                    && !RETIRED_STATES.contains(&c.state.as_str())
                    && c.action == SPOOL_CUSTODY_ACTION
                    && c.provider == requester_agent
                    && c.receiver == ward
                    && c.resource_classified_as
                        .iter()
                        .any(|value| value == &spool_classification(ward))
            }
            Ok(None) => false,
            Err(e) => {
                debug!(commitment = %id, error = %e, "custody-standing: conductor fallback unavailable");
                false
            }
        };
        self.remember_conductor(key.clone(), holds);
        self.conductor_singleflight.lock().await.remove(&key);
        drop(guard);
        holds
    }

    fn cached_conductor(&self, key: &(String, String)) -> Option<bool> {
        self.cache
            .lock()
            .ok()
            .and_then(|c| c.conductor.get(key).and_then(Cached::fresh))
    }

    fn remember_conductor(&self, key: (String, String), holds: bool) {
        if let Ok(mut c) = self.cache.lock() {
            c.conductor.insert(
                key,
                Cached {
                    value: holds,
                    at: Instant::now(),
                },
            );
        }
    }
}

struct LocalStandingFacts {
    requester_agent: String,
    ward: Option<String>,
    requester_is_ward: bool,
    custody_for_ward: bool,
    custody_for_digest: bool,
}

#[async_trait::async_trait]
impl CustodyStanding for ProjectionCustodyStanding {
    async fn facts_for(&self, requester: &Requester, row: &RowFacts) -> PrivateServeFacts {
        // A non-private row costs nothing — no pool checkout, no query, no await.
        if !is_private(&row.reach) {
            return PrivateServeFacts::not_gated(&row.reach);
        }

        let resolver = self.clone();
        let requester_for_db = requester.clone();
        let row_for_db = row.clone();
        let local = tokio::task::spawn_blocking(move || {
            let requester_agent = resolver.requester_agent(&requester_for_db)?;
            let ward = resolver.ward_for(&row_for_db);
            let requester_is_ward = ward.as_deref() == Some(requester_agent.as_str());
            let custody_for_digest = row_for_db
                .blob_digest
                .as_deref()
                .is_some_and(|digest| resolver.blob_digests(&requester_agent).contains(digest));
            let custody_for_ward = ward
                .as_deref()
                .is_some_and(|ward| resolver.spool_wards(&requester_agent).contains(ward));
            Some(LocalStandingFacts {
                requester_agent,
                ward,
                requester_is_ward,
                custody_for_ward,
                custody_for_digest,
            })
        })
        .await;
        let mut local = match local {
            Ok(Some(local)) => local,
            Ok(None) => return PrivateServeFacts::unresolved_requester(&row.reach),
            Err(error) => {
                warn!(error = %error, "custody-standing: blocking projection resolver failed closed");
                return PrivateServeFacts::unresolved_requester(&row.reach);
            }
        };

        // Only pay for the uncancellable conductor read when nothing local has
        // already decided to serve.
        if !local.custody_for_ward && !local.requester_is_ward && !local.custody_for_digest {
            if let Some(w) = local.ward.as_deref() {
                local.custody_for_ward = self
                    .conductor_spool_standing(&local.requester_agent, w)
                    .await;
            }
        }

        PrivateServeFacts {
            reach: row.reach.clone(),
            requester_resolved: true,
            requester_is_ward: local.requester_is_ward,
            custody_for_ward: local.custody_for_ward,
            custody_for_digest: local.custody_for_digest,
            ward_resolved: local.ward.is_some(),
        }
    }
}

/// iroh `NodeId` → `agent_cid`, through `peer_transport_manifest`. Behind a
/// helper so the `p2p-iroh`-gated module is referenced in exactly one place;
/// without the feature there is no iroh transport to resolve, so an iroh
/// requester resolves to nothing and fails closed.
#[cfg(feature = "p2p-iroh")]
fn p2p_iroh_peer_lookup(
    conn: &mut SqliteConnection,
    node_id: &str,
) -> Result<Option<String>, crate::error::StorageError> {
    Ok(crate::p2p_iroh::peer_map::lookup_by_iroh_node_id(conn, node_id)?.map(|m| m.agent_cid))
}

#[cfg(not(feature = "p2p-iroh"))]
fn p2p_iroh_peer_lookup(
    _conn: &mut SqliteConnection,
    _node_id: &str,
) -> Result<Option<String>, crate::error::StorageError> {
    Ok(None)
}

// ─────────────────────────────────────────────────────────────────────────────
// Test double
// ─────────────────────────────────────────────────────────────────────────────

/// In-memory [`CustodyStanding`] for tests: register a transport identity's
/// agent, the ward of a digest, and the custody an agent holds. Anything not
/// registered resolves to nothing — fail-closed, like production.
#[derive(Default)]
pub struct FakeCustodyStanding {
    inner: Mutex<FakeState>,
}

#[derive(Default)]
struct FakeState {
    agents: HashMap<String, String>,
    wards_by_digest: HashMap<String, String>,
    ward_by_content: HashMap<String, String>,
    spool: HashSet<(String, String)>,
    blob: HashSet<(String, String)>,
}

impl FakeCustodyStanding {
    pub fn new() -> Self {
        Self::default()
    }

    /// This transport identity resolves to this agent.
    pub fn bind(&self, requester: &Requester, agent: &str) -> &Self {
        self.inner
            .lock()
            .unwrap()
            .agents
            .insert(requester.label(), agent.to_string());
        self
    }

    /// This digest's row belongs to this ward.
    pub fn ward_of_digest(&self, digest: &str, ward: &str) -> &Self {
        self.inner
            .lock()
            .unwrap()
            .wards_by_digest
            .insert(digest.to_ascii_lowercase(), ward.to_string());
        self
    }

    /// This content id's row belongs to this ward (for rows with no blob).
    pub fn ward_of_content(&self, content_id: &str, ward: &str) -> &Self {
        self.inner
            .lock()
            .unwrap()
            .ward_by_content
            .insert(content_id.to_string(), ward.to_string());
        self
    }

    /// `agent` holds a live `custody-spool` for `ward`.
    pub fn spool_custody(&self, agent: &str, ward: &str) -> &Self {
        self.inner
            .lock()
            .unwrap()
            .spool
            .insert((agent.to_string(), ward.to_string()));
        self
    }

    /// `agent` holds a live `custody-blob` for `digest`.
    pub fn blob_custody(&self, agent: &str, digest: &str) -> &Self {
        self.inner
            .lock()
            .unwrap()
            .blob
            .insert((agent.to_string(), digest.to_ascii_lowercase()));
        self
    }
}

#[async_trait::async_trait]
impl CustodyStanding for FakeCustodyStanding {
    async fn facts_for(&self, requester: &Requester, row: &RowFacts) -> PrivateServeFacts {
        if !is_private(&row.reach) {
            return PrivateServeFacts::not_gated(&row.reach);
        }
        let state = self.inner.lock().unwrap();
        let Some(agent) = state.agents.get(&requester.label()).cloned() else {
            return PrivateServeFacts::unresolved_requester(&row.reach);
        };
        let digest = row.blob_digest.as_ref().map(|d| d.to_ascii_lowercase());
        let ward = digest
            .as_deref()
            .and_then(|d| state.wards_by_digest.get(d))
            .or_else(|| state.ward_by_content.get(&row.id))
            .cloned();
        PrivateServeFacts {
            reach: row.reach.clone(),
            requester_resolved: true,
            requester_is_ward: ward.as_deref() == Some(agent.as_str()),
            custody_for_ward: ward
                .as_ref()
                .is_some_and(|w| state.spool.contains(&(agent.clone(), w.clone()))),
            custody_for_digest: digest
                .as_ref()
                .is_some_and(|d| state.blob.contains(&(agent.clone(), d.clone()))),
            ward_resolved: ward.is_some(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;
    use crate::private_reach::{
        private_serve_verdict, PrivateServeVerdict, ServeReason, WithholdReason,
    };
    use diesel::r2d2::{ConnectionManager, Pool};
    use std::sync::atomic::{AtomicUsize, Ordering};

    const WARD: &str = "uhCAkWardAgentKeyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const CUSTODIAN: &str = "uhCAkCustodianKeyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const STRANGER: &str = "uhCAkStrangerKeyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const DIGEST: &str = "26d7ced97ee329025135f0ad4791b3e24d526b200b9147943450cb9141480406";

    fn test_pool() -> DbPool {
        let url = format!(
            "file:custody_standing_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn insert_commitment(
        pool: &DbPool,
        id: &str,
        action: &str,
        provider: &str,
        receiver: &str,
        classified: Option<&str>,
        state: &str,
    ) {
        use crate::db::diesel_schema::rea_commitments::dsl as rc;
        let mut conn = pool.get().expect("conn");
        diesel::insert_into(rc::rea_commitments)
            .values((
                rc::id.eq(id),
                rc::h_app_id.eq("lamad"),
                rc::action.eq(action),
                rc::provider.eq(provider),
                rc::receiver.eq(receiver),
                rc::resource_classified_as
                    .eq(classified.map(|c| serde_json::json!([c]).to_string())),
                rc::state.eq(state),
                rc::finished.eq(0),
                rc::created_at.eq("2026-09-03T00:00:00Z"),
            ))
            .execute(&mut conn)
            .expect("insert commitment");
    }

    fn bound_resolver(pool: DbPool, agent: &str) -> (ProjectionCustodyStanding, Requester) {
        let peer = libp2p::PeerId::random();
        let identity_map = Arc::new(crate::p2p::identity_map::StubIdentityMap::new());
        identity_map.register(peer, agent);
        (
            ProjectionCustodyStanding::new(pool).with_libp2p_identity_map(identity_map),
            Requester::libp2p(peer.to_base58()),
        )
    }

    fn unbound_resolver(pool: DbPool) -> (ProjectionCustodyStanding, Requester) {
        let peer = libp2p::PeerId::random();
        let identity_map = Arc::new(crate::p2p::identity_map::StubIdentityMap::new());
        (
            ProjectionCustodyStanding::new(pool).with_libp2p_identity_map(identity_map),
            Requester::libp2p(peer.to_base58()),
        )
    }

    fn private_row() -> RowFacts {
        RowFacts {
            reach: "private".to_string(),
            id: "bafkreiwitness".to_string(),
            blob_digest: Some(DIGEST.to_string()),
            created_by: Some(WARD.to_string()),
            dht_anchor_hash_present: false,
            metadata_kind: Some("death-witness".to_string()),
        }
    }

    #[tokio::test]
    async fn a_non_private_row_never_touches_the_projection() {
        // No pool rows at all, no conductor: a public row must still resolve to
        // Serve, proving the gate short-circuits before any resolution.
        let resolver = ProjectionCustodyStanding::new(test_pool());
        let row = RowFacts {
            reach: "public".to_string(),
            ..private_row()
        };
        let facts = resolver
            .facts_for(&Requester::libp2p("12D3KooWNobody"), &row)
            .await;
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::NonPrivate)
        );
        assert!(
            !facts.requester_resolved,
            "nothing was resolved for a non-private row"
        );
    }

    #[tokio::test]
    async fn an_unbound_libp2p_peer_is_withheld() {
        let (resolver, requester) = unbound_resolver(test_pool());
        let facts = resolver.facts_for(&requester, &private_row()).await;
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Withhold(WithholdReason::UnresolvedRequester)
        );
    }

    #[tokio::test]
    async fn the_ward_is_read_from_created_by_and_serves() {
        let pool = test_pool();
        let (resolver, requester) = bound_resolver(pool, WARD);
        let facts = resolver.facts_for(&requester, &private_row()).await;
        assert!(facts.ward_resolved, "created_by is agent-CID-shaped");
        assert!(facts.requester_is_ward);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::Ward)
        );
    }

    #[tokio::test]
    async fn a_spool_custodian_of_the_ward_serves() {
        let pool = test_pool();
        insert_commitment(
            &pool,
            &deterministic_spool_custody_id(CUSTODIAN, WARD, WARD),
            SPOOL_CUSTODY_ACTION,
            CUSTODIAN,
            WARD,
            Some(&crate::services::rea_commitment_service::spool_classification(WARD)),
            "created",
        );
        let (resolver, requester) = bound_resolver(pool, CUSTODIAN);
        let facts = resolver.facts_for(&requester, &private_row()).await;
        assert!(facts.custody_for_ward);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::SpoolCustody)
        );
    }

    #[tokio::test]
    async fn a_retired_spool_pledge_is_not_standing() {
        let pool = test_pool();
        insert_commitment(
            &pool,
            "spool-revoked",
            SPOOL_CUSTODY_ACTION,
            CUSTODIAN,
            WARD,
            Some(&crate::services::rea_commitment_service::spool_classification(WARD)),
            "revoked",
        );
        let (resolver, requester) = bound_resolver(pool, CUSTODIAN);
        let facts = resolver.facts_for(&requester, &private_row()).await;
        assert!(!facts.custody_for_ward, "a revoked pledge is not live");
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Withhold(WithholdReason::NoStanding)
        );
    }

    #[tokio::test]
    async fn a_blob_custodian_of_the_digest_serves() {
        let pool = test_pool();
        insert_commitment(
            &pool,
            "custody-blob-1",
            CUSTODY_BLOB_ACTION,
            CUSTODIAN,
            WARD,
            Some(&format!("sha256-{DIGEST}")),
            "created",
        );
        let (resolver, requester) = bound_resolver(pool, CUSTODIAN);
        let facts = resolver.facts_for(&requester, &private_row()).await;
        assert!(facts.custody_for_digest);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::BlobCustody)
        );
    }

    #[tokio::test]
    async fn a_resolved_stranger_is_withheld_with_no_standing() {
        let pool = test_pool();
        let (resolver, requester) = bound_resolver(pool, STRANGER);
        let facts = resolver.facts_for(&requester, &private_row()).await;
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Withhold(WithholdReason::NoStanding)
        );
    }

    /// A replicated copy whose ark passport carried no identity: `created_by`
    /// is not agent-CID-shaped, so the holder's OWN `custody-blob` names the
    /// ward. Without that row the holder cannot say whose it is.
    #[tokio::test]
    async fn a_ward_that_cannot_be_named_says_so() {
        let pool = test_pool();
        let (resolver, requester) = bound_resolver(pool, STRANGER);
        let row = RowFacts {
            created_by: Some("matthew-berth".to_string()),
            ..private_row()
        };
        let facts = resolver.facts_for(&requester, &row).await;
        assert!(!facts.ward_resolved);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Withhold(WithholdReason::WardUnresolved)
        );
    }

    /// The digest → ward fallback: the holder's own live `custody-blob` names
    /// the ward even when the row's `created_by` is a bare berth label.
    #[tokio::test]
    async fn own_custody_blob_names_the_ward_for_a_replicated_copy() {
        let pool = test_pool();
        // Self identity, so own_blob_wards can key on this peer's own rows.
        {
            use crate::db::diesel_schema::local_sessions::dsl as ls;
            let mut conn = pool.get().unwrap();
            diesel::insert_into(ls::local_sessions)
                .values((
                    ls::id.eq("session-1"),
                    ls::human_id.eq("human-1"),
                    ls::agent_pub_key.eq(CUSTODIAN),
                    ls::doorway_url.eq("http://localhost"),
                    ls::identifier.eq("custodian"),
                    ls::is_active.eq(1),
                    ls::created_at.eq("2026-09-03T00:00:00Z"),
                    ls::updated_at.eq("2026-09-03T00:00:00Z"),
                ))
                .execute(&mut conn)
                .expect("insert session");
        }
        insert_commitment(
            &pool,
            "custody-blob-own",
            CUSTODY_BLOB_ACTION,
            CUSTODIAN,
            WARD,
            Some(&format!("sha256-{DIGEST}")),
            "created",
        );
        let (resolver, requester) = bound_resolver(pool, WARD);
        let row = RowFacts {
            created_by: Some("matthew-berth".to_string()),
            ..private_row()
        };
        let facts = resolver.facts_for(&requester, &row).await;
        assert!(facts.ward_resolved, "own custody-blob names the ward");
        assert!(facts.requester_is_ward);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::Ward)
        );
    }

    #[tokio::test]
    async fn a_local_unanchored_death_witness_uses_the_spool_peers_self_agent_as_ward() {
        let pool = test_pool();
        {
            use crate::db::diesel_schema::local_sessions::dsl as ls;
            let mut conn = pool.get().unwrap();
            diesel::insert_into(ls::local_sessions)
                .values((
                    ls::id.eq("session-local-ward"),
                    ls::human_id.eq("human-ward"),
                    ls::agent_pub_key.eq(WARD),
                    ls::doorway_url.eq("http://localhost"),
                    ls::identifier.eq("ward"),
                    ls::is_active.eq(1),
                    ls::created_at.eq("2026-09-03T00:00:00Z"),
                    ls::updated_at.eq("2026-09-03T00:00:00Z"),
                ))
                .execute(&mut conn)
                .unwrap();
        }
        let resolver = ProjectionCustodyStanding::new(pool).with_spool_ingest(true);
        let row = RowFacts {
            created_by: None,
            ..private_row()
        };
        let facts = resolver.facts_for(&Requester::local(), &row).await;
        assert!(facts.ward_resolved);
        assert!(facts.requester_is_ward);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::Ward)
        );
    }

    #[tokio::test]
    async fn an_anchored_death_witness_never_uses_the_local_spool_fallback() {
        let pool = test_pool();
        let (resolver, requester) = bound_resolver(pool, STRANGER);
        let resolver = resolver.with_spool_ingest(true);
        let row = RowFacts {
            created_by: None,
            dht_anchor_hash_present: true,
            ..private_row()
        };
        let facts = resolver.facts_for(&requester, &row).await;
        assert!(!facts.ward_resolved);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Withhold(WithholdReason::WardUnresolved)
        );
    }

    #[tokio::test]
    async fn a_digest_custody_commitment_serves_even_when_the_ward_is_unresolved() {
        let pool = test_pool();
        insert_commitment(
            &pool,
            "custody-blob-unresolved-ward",
            CUSTODY_BLOB_ACTION,
            CUSTODIAN,
            WARD,
            Some(&format!("sha256-{DIGEST}")),
            "created",
        );
        let (resolver, requester) = bound_resolver(pool, CUSTODIAN);
        let row = RowFacts {
            created_by: None,
            metadata_kind: None,
            ..private_row()
        };
        let facts = resolver.facts_for(&requester, &row).await;
        assert!(!facts.ward_resolved);
        assert!(facts.custody_for_digest);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Serve(ServeReason::BlobCustody)
        );
    }

    #[tokio::test]
    async fn wrong_action_or_wrong_spool_classification_never_grants_local_standing() {
        let pool = test_pool();
        insert_commitment(
            &pool,
            "wrong-action",
            "reserve",
            CUSTODIAN,
            WARD,
            Some(&spool_classification(WARD)),
            "created",
        );
        insert_commitment(
            &pool,
            "wrong-classification",
            SPOOL_CUSTODY_ACTION,
            CUSTODIAN,
            WARD,
            Some("spool:witness:some-other-ward"),
            "created",
        );
        let (resolver, requester) = bound_resolver(pool, CUSTODIAN);
        let facts = resolver.facts_for(&requester, &private_row()).await;
        assert!(!facts.custody_for_ward);
        assert_eq!(
            private_serve_verdict(&facts),
            PrivateServeVerdict::Withhold(WithholdReason::NoStanding)
        );
    }

    struct CountingConductor {
        calls: AtomicUsize,
        evidence: Option<ConductorSpoolEvidence>,
    }

    #[async_trait::async_trait]
    impl ConductorCustodyLookup for CountingConductor {
        async fn get_spool_commitment(
            &self,
            _id: &str,
        ) -> Result<Option<ConductorSpoolEvidence>, crate::error::StorageError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(self.evidence.clone())
        }
    }

    fn exact_conductor_evidence() -> ConductorSpoolEvidence {
        ConductorSpoolEvidence {
            action: SPOOL_CUSTODY_ACTION.to_string(),
            provider: CUSTODIAN.to_string(),
            receiver: WARD.to_string(),
            resource_classified_as: vec![spool_classification(WARD)],
            finished: false,
            state: "created".to_string(),
        }
    }

    #[tokio::test]
    async fn conductor_fallback_is_positive_negative_single_flight_and_ttl_cached() {
        let pool = test_pool();
        let (resolver, requester) = bound_resolver(pool, CUSTODIAN);
        let positive = Arc::new(CountingConductor {
            calls: AtomicUsize::new(0),
            evidence: Some(exact_conductor_evidence()),
        });
        let resolver = Arc::new(resolver.with_conductor_lookup(positive.clone()));
        let positive_row = private_row();
        let (a, b, c, d) = tokio::join!(
            resolver.facts_for(&requester, &positive_row),
            resolver.facts_for(&requester, &positive_row),
            resolver.facts_for(&requester, &positive_row),
            resolver.facts_for(&requester, &positive_row),
        );
        for facts in [a, b, c, d] {
            assert!(facts.custody_for_ward);
        }
        assert_eq!(positive.calls.load(Ordering::SeqCst), 1);
        assert!(
            resolver
                .facts_for(&requester, &private_row())
                .await
                .custody_for_ward
        );
        assert_eq!(positive.calls.load(Ordering::SeqCst), 1, "positive TTL hit");

        let pool = test_pool();
        let (negative_resolver, negative_requester) = bound_resolver(pool, CUSTODIAN);
        let negative = Arc::new(CountingConductor {
            calls: AtomicUsize::new(0),
            evidence: None,
        });
        let negative_resolver = Arc::new(negative_resolver.with_conductor_lookup(negative.clone()));
        let negative_row = private_row();
        let (a, b, c) = tokio::join!(
            negative_resolver.facts_for(&negative_requester, &negative_row),
            negative_resolver.facts_for(&negative_requester, &negative_row),
            negative_resolver.facts_for(&negative_requester, &negative_row),
        );
        assert!([a, b, c].iter().all(|facts| !facts.custody_for_ward));
        assert_eq!(negative.calls.load(Ordering::SeqCst), 1);
        assert!(
            !negative_resolver
                .facts_for(&negative_requester, &private_row())
                .await
                .custody_for_ward
        );
        assert_eq!(negative.calls.load(Ordering::SeqCst), 1, "negative TTL hit");
    }

    #[tokio::test]
    async fn wrong_action_or_wrong_classification_never_grants_conductor_standing() {
        for evidence in [
            ConductorSpoolEvidence {
                action: "reserve".to_string(),
                ..exact_conductor_evidence()
            },
            ConductorSpoolEvidence {
                resource_classified_as: vec!["spool:witness:other".to_string()],
                ..exact_conductor_evidence()
            },
        ] {
            let pool = test_pool();
            let (resolver, requester) = bound_resolver(pool, CUSTODIAN);
            let lookup = Arc::new(CountingConductor {
                calls: AtomicUsize::new(0),
                evidence: Some(evidence),
            });
            let facts = resolver
                .with_conductor_lookup(lookup)
                .facts_for(&requester, &private_row())
                .await;
            assert!(!facts.custody_for_ward);
        }
    }

    #[cfg(feature = "p2p-iroh")]
    #[tokio::test]
    async fn iroh_requester_uses_the_same_positive_conductor_fallback() {
        let pool = test_pool();
        {
            let mut conn = pool.get().unwrap();
            crate::p2p_iroh::peer_map::record_iroh_observation(
                &mut conn,
                CUSTODIAN,
                "iroh-custodian-node",
                &[],
                &[crate::p2p_iroh::peer_map::Plane::Shard],
                1_787_000_000,
            )
            .unwrap();
        }
        let lookup = Arc::new(CountingConductor {
            calls: AtomicUsize::new(0),
            evidence: Some(exact_conductor_evidence()),
        });
        let resolver = ProjectionCustodyStanding::new(pool).with_conductor_lookup(lookup.clone());
        let facts = resolver
            .facts_for(&Requester::iroh("iroh-custodian-node"), &private_row())
            .await;
        assert!(facts.custody_for_ward);
        assert_eq!(lookup.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn the_fake_double_mirrors_the_production_shape() {
        let fake = FakeCustodyStanding::new();
        let ward_peer = Requester::libp2p("ward");
        let stranger_peer = Requester::iroh("stranger-node");
        fake.bind(&ward_peer, WARD)
            .bind(&stranger_peer, STRANGER)
            .ward_of_digest(DIGEST, WARD);

        assert_eq!(
            private_serve_verdict(&fake.facts_for(&ward_peer, &private_row()).await),
            PrivateServeVerdict::Serve(ServeReason::Ward)
        );
        assert_eq!(
            private_serve_verdict(&fake.facts_for(&stranger_peer, &private_row()).await),
            PrivateServeVerdict::Withhold(WithholdReason::NoStanding)
        );
        assert_eq!(
            private_serve_verdict(&fake.facts_for(&Requester::local(), &private_row()).await),
            PrivateServeVerdict::Withhold(WithholdReason::UnresolvedRequester)
        );
    }

    #[test]
    fn digest_key_collapses_all_three_accepted_address_renderings() {
        let sha = format!("sha256-{DIGEST}");
        let cid = BlobStore::hash_to_cid(DIGEST)
            .expect("valid sha256 hex")
            .to_string();
        assert_eq!(digest_key(&sha), Some(DIGEST.to_string()));
        assert_eq!(digest_key(DIGEST), Some(DIGEST.to_string()));
        assert_eq!(digest_key(&cid), Some(DIGEST.to_string()));
    }
}
