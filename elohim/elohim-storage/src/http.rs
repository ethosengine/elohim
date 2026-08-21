//! HTTP API for shard storage
//!
//! Provides REST endpoints for storing and retrieving shards:
//!
//! ## Shard API (Direct Shard Access)
//! - `PUT /shard/{shard_hash}` - Store a shard
//! - `GET /shard/{shard_hash}` - Retrieve a shard
//! - `HEAD /shard/{shard_hash}` - Check if shard exists
//!
//! ## Blob API (Convenience - Auto-Sharding)
//! - `PUT /blob/{blob_hash}` - Store blob, auto-create manifest
//! - `GET /blob/{blob_hash}` - Reassemble blob from shards
//! - `GET /manifest/{blob_hash}` - Get shard manifest
//!
//! ## Example Usage
//!
//! ```bash
//! # Store a shard
//! curl -X PUT -H "Content-Type: application/octet-stream" \
//!      --data-binary @video-chunk.bin \
//!      http://localhost:8090/shard/sha256-abc123
//!
//! # Retrieve a shard
//! curl http://localhost:8090/shard/sha256-abc123 > chunk.bin
//!
//! # Store a blob (auto-shards into manifest)
//! curl -X PUT -H "Content-Type: video/mp4" \
//!      --data-binary @video.mp4 \
//!      http://localhost:8090/blob/sha256-xyz789
//!
//! # Get manifest to see shards
//! curl http://localhost:8090/manifest/sha256-xyz789
//! ```

use crate::blob_store::BlobStore;
use crate::db::content_diesel::ContentQuery;
use crate::db::policy_cache::{
    ContentMetadata, PolicyDecision, PolicyEnforcement, PolicyEvent, PolicyEventType,
};
use crate::db::{self, AppContext, DbPool, PooledConn};
use crate::db::{
    collectives, content_mastery, contributor_presences, economic_events, human_relationships,
    humans, stewardship_allocations,
};
use crate::error::StorageError;
use crate::import_api::ImportApi;
use crate::progress_hub::ProgressHub;
use crate::progress_ws;
use crate::services::{response, Services};
use crate::sharding::{ShardEncoder, ShardManifest};
use crate::sync::SyncManager;
use crate::views::{
    validate_schema_versions,
    AccountIdentityView,
    AccountImportResultView,
    AccountPackageInputView,
    AccountPackageView,
    BeginObservationInputView,
    BeginObservationResponseView,
    ChallengeOutcomeView,
    CollectiveParticipationView,
    CollectiveSeedView,
    CollectiveView,
    ContentAssignmentView,
    ContentMasteryView,
    ContentStewardshipView,
    ContentView,
    ContentWithTagsView,
    ContributorPresenceView,
    CreateAllocationInputView,
    CreateCollectiveInputView,
    // InputView types for API boundary (camelCase with parsed JSON)
    CreateContentInputView,
    CreateContributorPresenceInputView,
    CreateEconomicEventInputView,
    CreateHumanRelationshipInputView,
    CreateMasteryInputView,
    CreateNodeStewardshipInputView,
    CreateRelationshipInputView,
    CreateScheduleInputView,
    CreateStewardedNodeInputView,
    EconomicEventView,
    EprHeadInputView,
    EprHeadView,
    ExchangeSessionTokenInputView,
    GateDecisionAttestationView,
    GateDecisionChallengeView,
    HumanView,
    InitiateClaimInputView,
    LocalSessionView,
    ObservationDurationView,
    ObservationEntryInputView,
    ObservationEntryView,
    ObservationIssueView,
    ObservationReportView,
    ObservationSummaryView,
    ObservationSystemStateView,
    PackageManifestView,
    RelationshipSeedView,
    RelationshipView,
    ScheduleView,
    StewardedNodeView,
    StewardshipAllocationView,
    StewardshipSeedView,
    UpdateAllocationInputView,
    UpdateContentInputView,
    SUPPORTED_SCHEMA_VERSIONS,
};
use bytes::Bytes;
use doorway_client::{DoorwayRoutesBuilder, Route};
use elohim_cache_core::extraction::ExtractionCache;
use http_body_util::{BodyExt, Either, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header, Method, Request, Response, StatusCode};

/// Unified response body: Left for normal (buffered) responses, Right for SSE (streaming).
pub type ApiBody = Either<Full<Bytes>, crate::sse::SseBody>;
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, warn};

/// Maximum number of concurrent MUTATING (write) HTTP requests. Excess writes
/// are SHED (per-request try_acquire → 503 + Retry-After), not queued — an
/// unbounded wait is just a slower wedge and gates /health too. Shedding a write
/// under burst is CORRECT backpressure: the seeder retries the catching-up shed
/// (`DoorwayClient.fetch()`), so the write lands once the projector drains.
const MAX_CONCURRENT_REQUESTS: usize = 64;

/// Maximum number of concurrent READ (GET/HEAD) requests — a SEPARATE, larger
/// pool from writes. Reads are cheap projection serves; a write burst (e.g.
/// genesis seeding) must never starve a cheap already-projected `/epr/*` read
/// (the read-path shed observed in genesis #1272 E2E). Independent pools
/// guarantee reads a floor no write storm can consume. Still bounded (reads at
/// enough concurrency cost memory/fds), just bounded on their own budget.
const MAX_CONCURRENT_READS: usize = 256;

/// How many times a coalesced `/apps` reader re-enters the extraction flight
/// after its post-wait cache re-check misses, before extracting anyway.
///
/// Re-entering is the correct move (the previous extractor may have failed, so
/// SOMEONE must extract) but it must be bounded: if every extraction keeps
/// failing, an unbounded retry turns the herd into a spin. Three rounds is
/// enough for the ordinary "the winner failed, the next one succeeds" case and
/// short enough that a genuinely broken bundle still reaches its honest error.
const MAX_EXTRACTION_COALESCE_ROUNDS: u32 = 3;

/// Retry-After (seconds) when storage sheds at the request-admission ceiling.
const STORAGE_SHED_RETRY_AFTER_SECS: u64 = 2;

/// Which admission pool a request draws from. Safe/idempotent methods (GET,
/// HEAD) are cheap reads and use the larger read pool; every other method
/// mutates and uses the tighter write pool. `http::Method` GET/HEAD are
/// associated consts (not enum variants), so this is equality, not a match.
fn admission_is_read_method(method: &Method) -> bool {
    *method == Method::GET || *method == Method::HEAD
}

/// True for the notary HEAD-declare write route (`POST /db/content/{id}/head`) —
/// the single-author, chain-membership-gated mutation whose 401/403 authority
/// refusal (`handle_content_head`'s "(a) AUTH FIRST" block) must never be masked
/// by the blanket admission shed below. This route is carved out of the
/// top-of-dispatch admission gate (`admission_exempt` in `handle_request`) and
/// instead applies its OWN admission check — but only AFTER authority is
/// confirmed (see `handle_content_head`'s post-author-gate `try_acquire_owned`).
/// A non-author gets refused regardless of write-pool pressure; only a confirmed
/// author can be shed with the catching-up 503 if the pool is genuinely full.
///
/// Deliberately excludes `/head-record` (no author gate; a read surface) and
/// `/canonical-head` (god-mode staging tier, no author gate — see
/// `handle_content_canonical_head`'s doc comment) — those stay behind the
/// blanket gate unchanged.
fn is_head_declare_write(method: &Method, path: &str) -> bool {
    *method == Method::POST && path.starts_with("/db/content/") && path.ends_with("/head")
}

#[cfg(test)]
mod head_declare_write_admission_carveout_tests {
    use super::*;

    #[test]
    fn matches_post_head_declare() {
        assert!(is_head_declare_write(
            &Method::POST,
            "/db/content/some-id/head"
        ));
    }

    #[test]
    fn excludes_get() {
        assert!(!is_head_declare_write(
            &Method::GET,
            "/db/content/some-id/head"
        ));
    }

    #[test]
    fn excludes_head_record() {
        assert!(!is_head_declare_write(
            &Method::POST,
            "/db/content/some-id/head-record"
        ));
    }

    #[test]
    fn excludes_canonical_head() {
        assert!(!is_head_declare_write(
            &Method::POST,
            "/db/content/some-id/canonical-head"
        ));
    }

    #[test]
    fn excludes_unrelated_write_route() {
        assert!(!is_head_declare_write(&Method::POST, "/db/content/bulk"));
    }
}

/// HTTP server state
pub struct HttpServer {
    blob_store: Arc<BlobStore>,
    manifests: Arc<RwLock<std::collections::HashMap<String, ShardManifest>>>,
    bind_addr: SocketAddr,
    /// Optional Import API for handling /import/* routes
    import_api: Option<Arc<RwLock<ImportApi>>>,
    /// Progress hub for WebSocket streaming
    progress_hub: Option<Arc<ProgressHub>>,
    /// Sync manager for CRDT document sync
    sync_manager: Option<Arc<SyncManager>>,
    /// Diesel connection pool for database operations
    db_pool: Option<DbPool>,
    /// Service layer for business logic
    services: Option<Arc<Services>>,
    /// Policy enforcement for stewardship content filtering
    policy_enforcement: Option<Arc<PolicyEnforcement>>,
    /// Node Registry API for tracking shards
    node_registry_api: Option<Arc<crate::node_registry_api::NodeRegistryApi>>,
    /// P2P handle for status endpoint (Send+Sync safe)
    #[cfg(feature = "p2p")]
    p2p_handle: Option<crate::p2p::P2PHandle>,
    /// Extraction cache for HTML5 app files (None = disabled)
    extraction_cache: Option<Arc<ExtractionCache>>,
    /// T7 (head-plane trust-gradient program): process-lifetime
    /// verification-memo store, wired once at startup via
    /// [`Self::with_memo_store`] and cloned into the per-request
    /// `EprService` the HTTP reach-authorization gate constructs (the
    /// `EprService::new(...)` call inside the content-resolve handler) —
    /// the prerequisite fix that makes the memo store's lifetime correct
    /// (process, not per-request). Not yet consulted for any decision;
    /// `TrustGradient::inert()` still passes `memo: None` (T9 wires reads).
    memo_store: Option<Arc<dyn crate::trust::VerificationMemoStore>>,
    /// In-memory index: slug -> blobHash (avoids per-request SQLite scan)
    slug_index: Arc<RwLock<std::collections::HashMap<String, String>>>,
    /// Write-admission limiter (mutating requests): prevents OOM under burst
    /// traffic (e.g., HTML5 app loads, seed storms). Reads use `read_semaphore`.
    request_semaphore: Arc<Semaphore>,
    /// Read-admission limiter (GET/HEAD): a separate, larger pool so a write
    /// burst can't starve cheap projection reads (genesis #1272 `/epr/*` shed).
    read_semaphore: Arc<Semaphore>,
    /// Whether the conductor is managed as an embedded child process
    embedded_conductor: bool,
    /// Operator-configured elohim capability profile (Category C — operational, not DHT-derived).
    /// Loaded once at startup from ELOHIM_CAPABILITY_CONFIG_FILE. None for storage/relay-only nodes.
    elohim_capability: Option<crate::views::ElohimCapabilityProfile>,
    /// SSR render capability profile fetched from DOORWAY_CAPABILITY_URL at startup.
    /// None when env var is unset or the fetch failed — peer-status shows null render_capability.
    render_capability: Option<crate::views::RenderCapabilityProfile>,
    /// Registered capability extensions (Tier-2 advertised capabilities beyond elohim + render).
    /// None until the capability registry is populated at startup (Task 9).
    extensions: Option<crate::views::CapabilityExtensions>,
    /// Manifest registry for projector status endpoint.
    /// Wired at startup via `with_manifest_registry`. None = endpoint returns empty cursors.
    manifest_registry: Option<Arc<crate::projector::ManifestRegistry>>,
    /// Conductor signing client for /api/v1/signal/emit (EPR Phase 2B Task C.2).
    /// Wired at startup via `with_signing_client`. None = endpoint returns 503.
    signing_client: Option<Arc<crate::signing::ConductorSigningClient>>,
    /// Composed write-through state (EPR Phase 2B Task C.4).
    /// Wired at startup via `with_write_through_state`. When absent, every
    /// non-integrity (pillar, kind) resolves to OFF (implicit default).
    write_through_state: Option<Arc<crate::write_through::WriteThroughState>>,
    /// Registry of per-cell HcClient instances for zome forwarding (Phase 11).
    /// Wired at startup via `with_hc_registry`. None = conductor bridge unavailable.
    hc_registry: Option<Arc<crate::hc_client_registry::HcClientRegistry>>,
    /// Embedded conductor manager — wired at startup (embedded mode only) so the
    /// authority-arc actuation endpoint can rewrite the conductor-config and
    /// RESTART the conductor (the only way to apply target_arc_factor — spec §2).
    /// None = no embedded conductor → actuation unavailable.
    conductor_manager: Option<Arc<tokio::sync::Mutex<crate::conductor::ConductorManager>>>,
    /// Operator-runtime-surface slice 1: capacity-1 kick channel into the
    /// projection-reconcile loop, wired at startup via [`Self::with_reconcile_kick`].
    /// `POST /api/v1/operator/reconcile` (commitment-gated) try_sends here; the
    /// loop treats a kick as an immediate extra wake. None = verb answers 503.
    reconcile_kick: Option<tokio::sync::mpsc::Sender<()>>,
    /// Embedded conductor ADMIN websocket — wired at startup (embedded mode
    /// only) so `GET /db/p2p/conductor-diagnostics` can read the conductor's
    /// peer store (`agent_info`) and transport/fetch state (`dump_network_stats`
    /// / `dump_network_metrics`). Cat-C operational read: the handle an
    /// operator, a negotiating peer, or an embedded agent uses to see WHY a
    /// cross-conductor fetch is failing. None = route answers 503.
    admin_websocket: Option<Arc<holochain_client::AdminWebsocket>>,
    /// FeedbackSignal fan-out context (Phase 3.5 T22).
    /// When set, `PUT /api/v1/epr` with FeedbackSignal kind runs
    /// project_signal + back_prop_one_hop + flood_feedback end-to-end.
    /// When absent (no P2P swarm, test fixtures), fan-out steps are skipped gracefully.
    fan_out_ctx: Option<Arc<crate::api::epr::EprFanOutCtx>>,
    /// T10 (head-plane trust-gradient program §3 L5 / minutes-quiesce §3 W2
    /// task Q6): declared-stakes resolver, wired at startup via
    /// `with_network_stakes_resolver`. Shares the same `ManifestRegistry`
    /// Arc as `fan_out_ctx.manifest_registry` (constructed once, cloned).
    /// Used by `PUT /admin/seed/network-stakes` to refresh the registry
    /// cache after an insert; NOT yet consulted by any pricing decision —
    /// `TrustGradient::inert()` still governs every `authorize_reach_for_human`
    /// call (Q7 is the task that wires a resolved stage into a decision).
    network_stakes_resolver: Option<Arc<crate::trust::ManifestStakesResolver>>,
    /// SSR renderer (ssr feature only). Constructed once at startup from SSR_BUNDLE_PATH.
    /// None when the feature is off or the bundle failed to load — handlers return 503.
    #[cfg(feature = "ssr")]
    ssr_state: Option<crate::ssr::SsrState>,
    /// T17: CID of this peer's steward. Populated from `Config::self_cid` at startup.
    /// Used as the `receiver` field in `serve-blob` REA events emitted on a
    /// successful GET-time race-fetch.  Empty string when not configured.
    self_cid: String,
    /// T17: per-peer timeout for race-fetch (seconds). From `Config::fetch_blob_timeout_seconds`.
    fetch_blob_timeout_seconds: u64,
    /// T17: max parallel peer attempts per race-fetch batch. From `Config::fetch_blob_parallelism`.
    fetch_blob_parallelism: usize,
    /// T17: freshness window (seconds) for `peer_blob_inventory` rows consulted
    /// during a GET-time heal. From `Config::inventory_freshness_seconds`.
    /// Replaces a previously hardcoded 600s window so the same operator knob
    /// that governs the reconciler's staleness threshold also governs heal.
    inventory_freshness_seconds: u64,
    /// In-request time budget (ms) for the peer-heal leg of `get_blob_or_heal`.
    /// From `Config::heal_on_read_budget_ms`. On expiry the request degrades to
    /// a `Syncing` outcome (503 + Retry-After) while the race-fetch continues
    /// detached in the background so heal still converges (Fix B).
    heal_on_read_budget_ms: u64,
    /// Iroh-side blob backend (BLAKE3-keyed). Wired at startup via
    /// [`HttpServer::with_iroh_blob_store`] when the iroh node is
    /// active. None means iroh is disabled or not yet wired — every
    /// `/blob/{hash}` request degrades to the legacy SHA256 path.
    #[cfg(feature = "p2p-iroh")]
    iroh_blob_store: Option<Arc<crate::p2p_iroh::IrohBlobStore>>,
    /// This node's transport-profile manifest (Plan 1). Used by the
    /// blob handler's backend selector. Wired at startup via
    /// [`HttpServer::with_self_transport_manifest`].
    #[cfg(feature = "p2p-iroh")]
    self_transport_manifest: Option<Arc<crate::p2p_iroh::peer_map::PeerTransportManifest>>,
    /// This node's libp2p peer ID string (e.g. "12D3KooW...").
    /// Used by `handle_put_blob` to stamp `peer_blob_inventory` with
    /// `source='self-custody'` on dual-write. Wired at startup via
    /// [`HttpServer::with_self_peer_id`]. Falls back to "unknown-peer"
    /// when not configured (legacy deployments without P2P).
    self_peer_id: String,
    /// Transport-identity seam (libp2p OR iroh). When `p2p_handle` is absent
    /// (iroh mode), `/p2p/status` reports this transport's `status_peer_id()`
    /// as `.peerId` instead of returning 503 — so the iroh dataplane lights the
    /// resilience card. In libp2p mode the existing `p2p_handle.status()` path
    /// is unchanged and authoritative; this is only the fallback. `None` when no
    /// transport identity was resolved (P2P disabled / key load failed).
    node_transport: Option<Arc<dyn crate::node_transport::NodeTransport>>,
    /// Counter — number of `GET /blob` requests served from iroh.
    /// Read by parity-soak diagnostics; never reset at runtime.
    blob_iroh_served_count: Arc<std::sync::atomic::AtomicU64>,
    /// Counter — number of `GET /blob` requests served from libp2p.
    blob_libp2p_served_count: Arc<std::sync::atomic::AtomicU64>,
    /// Content graph engine (graph-native feature). When present, the HTTP
    /// prerequisite-mastery access gate enforces via `PREREQUISITE` edges
    /// (`*epr_edge{rel_type: 'PREREQUISITE'}`), mirroring the P2P gate in
    /// `EprService`. Wired at startup via [`HttpServer::with_graph_engine`].
    /// `None` (no graph, tests) makes the prerequisite gate a no-op (allow).
    /// NOTE: this is a DEDICATED field, not reached through `fan_out_ctx` —
    /// `fan_out_ctx` is `None` in direct/Tauri (:8090) single-user mode where
    /// the HTTP gate is the only gate; piggybacking would silently fail-open.
    #[cfg(feature = "graph-native")]
    graph_engine: Option<Arc<crate::graph::engine::GraphEngine>>,
    /// Demand-driven auto-pin controller (self-healing opportunity map row 15).
    /// On a confirmed local content read-miss, authors an `item` DevicePin so
    /// the acquisition+provide loops acquire and provide the demanded content —
    /// the runtime input the provide-loop was starved of. Config-gated
    /// (`demand_autopin_enabled`) + throttled. Default-disabled until
    /// `with_fetch_config` threads the operator config (prod default: enabled).
    demand_autopin: Arc<crate::services::demand_autopin::DemandAutoPin>,
}

/// Extract X-Schema-Version header from request and validate it.
/// Returns Ok(Some(version)) if present and valid, Ok(None) if absent, Err if unsupported.
fn validate_schema_version_header(req: &Request<Incoming>) -> Result<Option<u32>, String> {
    match req.headers().get("X-Schema-Version") {
        Some(val) => {
            let version = val
                .to_str()
                .map_err(|_| "Invalid X-Schema-Version header encoding".to_string())?
                .parse::<u32>()
                .map_err(|_| "X-Schema-Version must be a positive integer".to_string())?;
            if !SUPPORTED_SCHEMA_VERSIONS.contains(&version) {
                return Err(format!(
                    "Unsupported schema version: {}. Supported: {:?}",
                    version, SUPPORTED_SCHEMA_VERSIONS
                ));
            }
            Ok(Some(version))
        }
        None => {
            warn!("Bulk request missing X-Schema-Version header (deprecated: clients should send this header)");
            Ok(None)
        }
    }
}

/// Check whether an identifier is a content address rather than a
/// human-readable slug. Used by `/apps/` routes to decide whether to resolve
/// via `slug_index` or treat the identifier as a direct blob address.
///
/// TWO spellings of one address are accepted, and both are canonical inputs:
///
/// - **CIDv1 base32** (`bafkrei…` raw+sha2-256, `bafyrei…` dag-cbor) — the
///   canonical protocol address (root CLAUDE.md / p2p-design-gate). This is
///   what `X-Content-Address` reports, so a client that round-trips that header
///   back into `/apps/{identifier}/…` MUST resolve; before this recognizer knew
///   CIDv1, it fell through to slug lookup and 404'd `App not found: bafkrei…`.
/// - **`sha256-{hex}`** — the legacy alias, and the on-disk blob-store key.
///
/// A CID candidate is validated with the `cid` crate rather than matched on its
/// `baf` prefix, so a slug that merely starts with `baf` (`bafflegab-app`)
/// stays a slug.
fn is_content_address(identifier: &str) -> bool {
    if identifier.starts_with("sha256-") && identifier.len() > 10 {
        return true;
    }
    identifier.starts_with("baf") && cid::Cid::from_str(identifier).is_ok()
}

/// Canonical `sha256-{hex}` form of a content address (CIDv1 or `sha256-…`),
/// or `None` when the identifier is not an address at all.
///
/// Both spellings of one address normalize to a single key so `/apps/{cid}/…`
/// and `/apps/{sha256-…}/…` share ONE extraction-cache entry (and one blob
/// path) instead of extracting the same bundle twice under two names.
fn canonical_content_address(identifier: &str) -> Option<String> {
    BlobStore::parse_content_address(identifier)
        .ok()
        .map(|hex| format!("sha256-{}", hex))
}

/// Fix B syncing-status JSON body for the `GET /blob/{hash}` route: the
/// in-request heal budget expired while bytes are still being pulled from peers
/// in the background. Paired with a 503 + `Retry-After: 5`. Deliberately
/// distinct from the `FinalizeFailed` 503 bodies (which are plain text) so a
/// client can tell "syncing, retry" from "finalize failed, retry".
fn syncing_blob_body(hash: &str) -> String {
    format!(
        r#"{{"status":"syncing","blobHash":"{}","hint":"bytes are being pulled from peers; retry"}}"#,
        hash
    )
}

/// Fix B syncing-status JSON body for the `/apps/` resolver. Same shape as
/// [`syncing_blob_body`] but keyed by the app slug/identifier. Load-bearing: it
/// MUST NOT say "App not found" — a transient byte-lag on a healed pointer is a
/// syncing degrade (option (b) of the `.epr-meta` peer-fallback invariant), not
/// a missing deploy, and the federation-deploy scenario greps for the
/// App-not-found body.
fn syncing_app_body(slug: &str) -> String {
    format!(
        r#"{{"status":"syncing","slug":"{}","hint":"bytes are being pulled from peers; retry"}}"#,
        slug
    )
}

/// Outcome of [`HttpServer::get_blob_or_heal`]: the shared local-read +
/// T17 peer-heal sequence behind both `GET /blob/{hash}` and the `/apps/`
/// resolver. Callers map each variant onto their own HTTP semantics — the
/// `/blob` route distinguishes 503 (finalize failed) from 404 (no heal), and
/// preserves its existing metrics/headers; the apps resolver continues into ZIP
/// extraction on `Bytes` and keeps its existing 404 body otherwise.
#[derive(Debug)]
enum BlobHealOutcome {
    /// Bytes available — either a local hit (`healed_from = None`) or fetched
    /// from a peer and durably finalized (`healed_from = Some(peer_id)`).
    Bytes {
        bytes: Vec<u8>,
        healed_from: Option<String>,
    },
    /// Local miss and no peer served verified bytes (no candidates, race miss,
    /// or no p2p/db wired) → caller returns 404.
    NotFound,
    /// Bytes were fetched from a peer but local finalize failed → caller
    /// returns 503 to preserve downstream retry semantics. The variant carries
    /// the precise failure leg so the `/blob` route keeps its distinct bodies.
    FinalizeFailed(FinalizeFailure),
    /// The in-request heal budget (`heal_on_read_budget_ms`) expired before the
    /// peer race-fetch completed. The fetch is still running detached in the
    /// background (heal converges on a later request), so both callers degrade
    /// legibly with a syncing-status 503 + `Retry-After` rather than blocking
    /// the request open for the full cross-peer race (observed >30s live). Fix B
    /// of genesis backlog `heal-pointer-bytes-ordering-blocking-serve.md`;
    /// option (b) of the `.epr-meta` peer-fallback invariant. `started` records
    /// whether a background heal task was actually spawned (always `true` today;
    /// the field keeps the distinction explicit for callers/classification).
    Syncing { started: bool },
}

/// Outcome of [`HttpServer::read_local_blob`] — the ONE local-read answer every
/// blob-serving path shares.
///
/// A blob we hold locally can be on disk in either of two shapes, and every
/// caller must accept both:
///
/// - **As manifest-named shards.** `put_blob_bytes` stores each shard under its
///   own hash and never stores the composite under its own name, so a raw
///   `BlobStore` probe on the composite of a `>16MB` blob always answers
///   "absent" for bytes we fully hold.
/// - **Whole, under its own name.** A composite healed from a peer is persisted
///   by `finalize_fetch_success` after reassembly (and a small blob is always
///   this shape), while the shards a manifest row names may never have arrived.
///
/// Checking only one shape is how `/blob` and the `/apps` resolver came to
/// disagree about the same bytes.
#[derive(Debug)]
enum LocalBlobRead {
    /// Bytes we hold. `manifest` is carried when one participated in the read so
    /// the `/blob` route can keep reporting the recorded mime type and ETag.
    /// Boxed: a `ShardManifest` is an order of magnitude larger than the other
    /// variants, and every local read moves this enum.
    Bytes {
        bytes: Vec<u8>,
        manifest: Option<Box<ShardManifest>>,
    },
    /// A manifest resolved, a shard it names is absent, and the composite is not
    /// held whole either. Carries the gap so callers name the missing shard
    /// instead of reporting the composite as simply absent — AND the manifest
    /// itself, because this is the variant that precedes a peer heal: the bytes
    /// a peer supplies are described by exactly this manifest, so its mime type
    /// must reach the response rather than being recomputed or dropped.
    MissingShard {
        manifest: Box<ShardManifest>,
        index: usize,
        shard_hash: String,
    },
    /// No manifest and no bytes under this address.
    Absent,
}

/// The two finalize failure legs the `/blob` route distinguishes, each with its
/// own 503 body (preserved verbatim from the pre-refactor handler).
#[derive(Debug)]
enum FinalizeFailure {
    /// Connection pool exhausted; could not acquire a connection to finalize.
    PoolExhausted,
    /// Filesystem persist or the wrapping SQL transaction failed.
    Persist,
}

/// Extract the `elohim_session` cookie value from a request's `Cookie` header.
///
/// The header is a `; `-separated list of `name=value` pairs. Returns the
/// first `elohim_session` value found, trimmed. `None` when the header is
/// absent, unparseable, or carries no `elohim_session` pair. Used by
/// `GET /auth/me` to project the cookie-named session (GAP-2b multi-session
/// browser path) in preference to the single active session.
fn extract_session_cookie(headers: &hyper::HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix("elohim_session=") {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// SPA deep-link discrimination — the shared ROUTE/ASSET rule (§12.2 of the
/// pillar-EPR decomposition spec): a requested sub-path is a **ROUTE** iff its
/// FINAL segment contains no `.`; otherwise it is an **ASSET**.
///
/// Routes are fallback-eligible — on a bundle miss `handle_app_request` serves
/// the bundle's own `index.html` so the SPA router can render the designed
/// experience. Asset misses stay honest 404s — a missing hashed bundle file is
/// a real deploy bug and MUST surface, never be masked by index.html.
///
/// Mirrors doorway-service's `derive_app_subpath` discriminator; both crates
/// drive this off the one shared test-vector table
/// (`sdk/fixtures/spa-route-discrimination.vectors.json`) — the two-layer drift
/// guard.
fn is_spa_route_subpath(sub: &str) -> bool {
    let final_segment = sub.rsplit('/').next().unwrap_or(sub);
    !final_segment.contains('.')
}

/// Fetch the `blob_hash` for a content row by its stable id (slug), respecting
/// the same provenance gate (`require_provenance = true`) that
/// `derive_epr_head` uses for HTTP callers.
///
/// Returns `None` when the row is missing, fails the provenance gate, or has
/// no `blob_hash` populated yet (pre-distribution content). Used by the EPR
/// head handler to hydrate `DistributionSummary` (Phase 5 T34) — distribution
/// is best-effort, so any miss collapses to `None` rather than surfacing an
/// error onto the head response.
fn view_blob_hash_for_id(
    conn: &mut diesel::SqliteConnection,
    app_ctx: &db::AppContext,
    id: &str,
) -> Option<String> {
    db::content_diesel::get_content_with_tags(
        conn,
        app_ctx,
        id,
        db::content_diesel::MinTrust::Amber,
    )
    .ok()
    .flatten()
    .and_then(|cwt| cwt.content.blob_hash)
}

/// Project one kitsune2 `AgentInfoSigned` JSON string (as returned by the
/// conductor admin `agent_info` call) into a compact diagnostic view.
///
/// Wire shape: `{"agentInfo":"<inner JSON string>","signature":"<b64>"}` where
/// the inner document carries `agent`, `space`, `createdAt`, `expiresAt`,
/// `isTombstone`, `url` (null when the peer has no reachable transport), and
/// `storageArc`. Tolerant by design — any parse miss degrades to `{"raw": …}`
/// so a conductor-side encoding change can never blank the whole diagnostic.
fn project_agent_info(signed_json: &str) -> serde_json::Value {
    let outer: serde_json::Value = match serde_json::from_str(signed_json) {
        Ok(v) => v,
        Err(_) => return serde_json::json!({ "raw": signed_json }),
    };
    let inner: serde_json::Value = match outer.get("agentInfo").and_then(|v| v.as_str()) {
        Some(inner_str) => match serde_json::from_str(inner_str) {
            Ok(v) => v,
            Err(_) => return serde_json::json!({ "raw": signed_json }),
        },
        // Some encodings inline the object rather than string-wrapping it.
        None => outer.clone(),
    };
    serde_json::json!({
        "agent": inner.get("agent").cloned().unwrap_or(serde_json::Value::Null),
        "space": inner.get("space").cloned().unwrap_or(serde_json::Value::Null),
        "url": inner.get("url").cloned().unwrap_or(serde_json::Value::Null),
        "createdAt": inner.get("createdAt").cloned().unwrap_or(serde_json::Value::Null),
        "expiresAt": inner.get("expiresAt").cloned().unwrap_or(serde_json::Value::Null),
        "isTombstone": inner.get("isTombstone").cloned().unwrap_or(serde_json::Value::Null),
        "storageArc": inner.get("storageArc").cloned().unwrap_or(serde_json::Value::Null),
    })
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(blob_store: Arc<BlobStore>, bind_addr: SocketAddr) -> Self {
        Self {
            blob_store,
            manifests: Arc::new(RwLock::new(std::collections::HashMap::new())),
            bind_addr,
            import_api: None,
            progress_hub: None,
            sync_manager: None,
            db_pool: None,
            services: None,
            policy_enforcement: None,
            node_registry_api: None,
            #[cfg(feature = "p2p")]
            p2p_handle: None,
            extraction_cache: None,
            memo_store: None,
            slug_index: Arc::new(RwLock::new(std::collections::HashMap::new())),
            request_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
            read_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_READS)),
            embedded_conductor: false,
            elohim_capability: None,
            render_capability: None,
            extensions: None,
            manifest_registry: None,
            signing_client: None,
            write_through_state: None,
            hc_registry: None,
            conductor_manager: None,
            reconcile_kick: None,
            admin_websocket: None,
            fan_out_ctx: None,
            network_stakes_resolver: None,
            #[cfg(feature = "ssr")]
            ssr_state: None,
            self_cid: String::new(),
            fetch_blob_timeout_seconds: 5,
            fetch_blob_parallelism: 3,
            inventory_freshness_seconds: 600,
            heal_on_read_budget_ms: 5000,
            #[cfg(feature = "p2p-iroh")]
            iroh_blob_store: None,
            #[cfg(feature = "p2p-iroh")]
            self_transport_manifest: None,
            self_peer_id: "unknown-peer".to_string(),
            node_transport: None,
            blob_iroh_served_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            blob_libp2p_served_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            #[cfg(feature = "graph-native")]
            graph_engine: None,
            // Disabled until `with_fetch_config` threads the operator config.
            demand_autopin: Arc::new(crate::services::demand_autopin::DemandAutoPin::disabled()),
        }
    }

    /// Set the operator-configured elohim capability profile (Category C — operational).
    ///
    /// Call with the result of `load_elohim_capability_from_env()` at startup.
    /// None means this node declares no elohim capability (pure storage/relay).
    pub fn with_elohim_capability(
        mut self,
        capability: Option<crate::views::ElohimCapabilityProfile>,
    ) -> Self {
        self.elohim_capability = capability;
        self
    }

    /// Set the SSR render capability profile fetched from DOORWAY_CAPABILITY_URL.
    ///
    /// Call with the result of `load_render_capability_from_url().await` at startup.
    /// None means this node has no render capability advertised (fetch failed or env var unset).
    pub fn with_render_capability(
        mut self,
        capability: Option<crate::views::RenderCapabilityProfile>,
    ) -> Self {
        self.render_capability = capability;
        self
    }

    /// Set the Tier-2 capability extensions map.
    ///
    /// None until the capability registry is populated (Task 9).
    pub fn with_extensions(
        mut self,
        extensions: Option<crate::views::CapabilityExtensions>,
    ) -> Self {
        self.extensions = extensions;
        self
    }

    /// Set the manifest registry for the projector status endpoint.
    ///
    /// When set, `GET /api/v1/status/projector` returns live cursor + lag data
    /// computed from the registry's registered projections. When absent, the
    /// endpoint returns an empty `{ cursors: [], lag: [] }` response.
    pub fn with_manifest_registry(
        mut self,
        registry: Arc<crate::projector::ManifestRegistry>,
    ) -> Self {
        self.manifest_registry = Some(registry);
        self
    }

    /// Set the conductor signing client for `/api/v1/signal/emit`.
    ///
    /// When set, signal-intent payloads are composed into EPR Envelopes,
    /// signed via the imagodei `sign_for_agent` zome, and ingested. When
    /// absent, the endpoint returns 503 — storage instances without a
    /// conductor connection (test fixtures, dev environments) gracefully
    /// degrade rather than panicking.
    ///
    /// EPR Phase 2B Task C.2.
    pub fn with_signing_client(
        mut self,
        client: Arc<crate::signing::ConductorSigningClient>,
    ) -> Self {
        self.signing_client = Some(client);
        self
    }

    /// Set the composed write-through state (EPR Phase 2B Task C.4).
    ///
    /// When set, `GET /api/v1/status/write-through` reports the effective
    /// per-(pillar, kind) flag composed across all 4 override layers, and
    /// (Task C.5) the signal-emit endpoint consults it to decide whether to
    /// project. When absent, the status endpoint reports an empty
    /// `effective[]` array (registry-derived rows are still included if a
    /// manifest registry is wired) and the implicit default OFF applies for
    /// all non-integrity kinds.
    pub fn with_write_through_state(
        mut self,
        state: Arc<crate::write_through::WriteThroughState>,
    ) -> Self {
        self.write_through_state = Some(state);
        self
    }

    /// Wire the HcClientRegistry for per-cell zome forwarding (Phase 11).
    pub fn with_hc_registry(
        mut self,
        registry: std::sync::Arc<crate::hc_client_registry::HcClientRegistry>,
    ) -> Self {
        self.hc_registry = Some(registry);
        self
    }

    /// Wire the embedded conductor manager so the authority-arc actuation
    /// endpoint can rewrite the conductor-config and restart the conductor.
    pub fn with_conductor_manager(
        mut self,
        manager: Arc<tokio::sync::Mutex<crate::conductor::ConductorManager>>,
    ) -> Self {
        self.conductor_manager = Some(manager);
        self
    }

    /// Wire the embedded conductor's ADMIN websocket so
    /// `GET /db/p2p/conductor-diagnostics` can read peer store + transport
    /// state straight from the conductor (Cat-C operational diagnostics).
    pub fn with_admin_websocket(mut self, admin_ws: Arc<holochain_client::AdminWebsocket>) -> Self {
        self.admin_websocket = Some(admin_ws);
        self
    }

    /// Wire the FeedbackSignal fan-out context (Phase 3.5 T22).
    ///
    /// When set, `PUT /api/v1/epr` with FeedbackSignal kind activates the full
    /// fan-out: project_signal + back_prop_one_hop + flood_feedback. When absent
    /// (no P2P swarm, test fixtures, dev without key config), fan-out steps that
    /// require missing dependencies are skipped gracefully.
    pub fn with_fan_out_ctx(mut self, ctx: Arc<crate::api::epr::EprFanOutCtx>) -> Self {
        self.fan_out_ctx = Some(ctx);
        self
    }

    /// Wire the declared-stakes resolver (T10). `PUT /admin/seed/network-stakes`
    /// uses it to refresh the `ManifestRegistry` cache after an insert. Absent
    /// = the seed route still writes durably (just without the immediate
    /// same-process cache refresh) and no boot-time stage is logged.
    pub fn with_network_stakes_resolver(
        mut self,
        resolver: Arc<crate::trust::ManifestStakesResolver>,
    ) -> Self {
        self.network_stakes_resolver = Some(resolver);
        self
    }

    /// Wire the projection-reconcile kick channel (operator-runtime-surface
    /// slice 1). The sender feeds the capacity-1 channel whose receiver lives
    /// in the reconcile loop's `tokio::select!`; absent = the operator verb
    /// answers 503 (honest: this peer runs no reconcile loop).
    pub fn with_reconcile_kick(mut self, kick: tokio::sync::mpsc::Sender<()>) -> Self {
        self.reconcile_kick = Some(kick);
        self
    }

    /// Wire the content graph engine so the HTTP prerequisite-mastery gate can
    /// enforce via `PREREQUISITE` edges (mirrors the P2P gate in `EprService`).
    /// Dedicated wiring (NOT via `fan_out_ctx`) so the gate enforces even in
    /// direct/Tauri single-user mode where no P2P swarm — and thus no
    /// `fan_out_ctx` — exists.
    #[cfg(feature = "graph-native")]
    pub fn with_graph_engine(
        mut self,
        graph_engine: Option<Arc<crate::graph::engine::GraphEngine>>,
    ) -> Self {
        self.graph_engine = graph_engine;
        self
    }

    /// Wire the iroh-side blob store. When set, `GET /blob/{hash}` may
    /// serve from BLAKE3-keyed iroh storage for iroh-capable callers
    /// before falling through to the legacy SHA256 path.
    #[cfg(feature = "p2p-iroh")]
    pub fn with_iroh_blob_store(mut self, store: Arc<crate::p2p_iroh::IrohBlobStore>) -> Self {
        self.iroh_blob_store = Some(store);
        self
    }

    /// Wire this node's transport-profile manifest (Plan 1). Required
    /// for the blob backend selector to ever return Iroh; absent
    /// manifest forces libp2p-only.
    #[cfg(feature = "p2p-iroh")]
    pub fn with_self_transport_manifest(
        mut self,
        manifest: Arc<crate::p2p_iroh::peer_map::PeerTransportManifest>,
    ) -> Self {
        self.self_transport_manifest = Some(manifest);
        self
    }

    /// Set this node's libp2p peer ID string for `peer_blob_inventory`
    /// self-custody stamps. Typically sourced from `self_transport_manifest`
    /// (Plan 1) or the libp2p swarm identity. When absent the inventory row
    /// is stamped with `"unknown-peer"` (correct but unroutable for reverse
    /// lookup by other peers).
    pub fn with_self_peer_id(mut self, peer_id: String) -> Self {
        self.self_peer_id = peer_id;
        self
    }

    /// Wire the transport-identity seam (libp2p OR iroh). Lets `/p2p/status`
    /// report the active transport's `status_peer_id()` as `.peerId` when the
    /// libp2p `p2p_handle` is absent (iroh mode) — the seam that lets the iroh
    /// dataplane light the resilience card. No effect on the libp2p path, where
    /// `p2p_handle.status()` stays authoritative.
    pub fn with_node_transport(
        mut self,
        transport: Arc<dyn crate::node_transport::NodeTransport>,
    ) -> Self {
        self.node_transport = Some(transport);
        self
    }

    /// Wire the SSR renderer (ssr feature only).
    ///
    /// Call with `crate::ssr::SsrState::from_env()` at startup. When absent
    /// (feature off, or bundle failed to load), `GET /spa/*` and `POST /render`
    /// return 503. Peers that cannot sustain V8 simply omit this call.
    #[cfg(feature = "ssr")]
    pub fn with_ssr_state(mut self, state: crate::ssr::SsrState) -> Self {
        self.ssr_state = Some(state);
        self
    }

    /// Mark this server as running in embedded conductor mode
    pub fn with_embedded_conductor(mut self) -> Self {
        self.embedded_conductor = true;
        self
    }

    /// Set the Import API handler
    pub fn with_import_api(mut self, import_api: Arc<RwLock<ImportApi>>) -> Self {
        self.import_api = Some(import_api);
        self
    }

    /// Set the Progress Hub for WebSocket streaming
    pub fn with_progress_hub(mut self, hub: Arc<ProgressHub>) -> Self {
        self.progress_hub = Some(hub);
        self
    }

    /// Set the Node Registry API
    pub fn with_node_registry_api(
        mut self,
        api: Arc<crate::node_registry_api::NodeRegistryApi>,
    ) -> Self {
        self.node_registry_api = Some(api);
        self
    }

    /// Set the Sync Manager for CRDT document sync
    pub fn with_sync_manager(mut self, sync_manager: Arc<SyncManager>) -> Self {
        self.sync_manager = Some(sync_manager);
        self
    }

    /// Set the Service layer for business logic
    pub fn with_services(mut self, services: Arc<Services>) -> Self {
        self.services = Some(services);
        self
    }

    /// Set the Diesel connection pool for new entity endpoints
    pub fn with_db_pool(mut self, pool: DbPool) -> Self {
        self.db_pool = Some(pool);
        self
    }

    /// Set the Policy Enforcement engine for stewardship content filtering
    pub fn with_policy_enforcement(mut self, enforcement: Arc<PolicyEnforcement>) -> Self {
        self.policy_enforcement = Some(enforcement);
        self
    }

    /// Set the P2P handle for status endpoint
    #[cfg(feature = "p2p")]
    pub fn with_p2p_handle(mut self, handle: crate::p2p::P2PHandle) -> Self {
        self.p2p_handle = Some(handle);
        self
    }

    /// Wire T17 race-fetch parameters from the runtime Config.
    /// Call this at startup when a `Config` is available so the GET-time
    /// blob fallback knows its timeout, parallelism, and self-CID.
    pub fn with_fetch_config(mut self, config: &crate::config::Config) -> Self {
        self.self_cid = config.self_cid.clone().unwrap_or_default();
        self.fetch_blob_timeout_seconds = config.fetch_blob_timeout_seconds;
        self.fetch_blob_parallelism = config.fetch_blob_parallelism;
        self.inventory_freshness_seconds = config.inventory_freshness_seconds;
        self.heal_on_read_budget_ms = config.heal_on_read_budget_ms;
        self.demand_autopin = Arc::new(crate::services::demand_autopin::DemandAutoPin::new(
            config.demand_autopin_enabled,
            std::time::Duration::from_secs(config.demand_autopin_throttle_seconds),
        ));
        self
    }

    /// Set the extraction cache
    pub fn with_extraction_cache(mut self, cache: Arc<ExtractionCache>) -> Self {
        self.extraction_cache = Some(cache);
        self
    }

    /// Wire the process-lifetime verification-memo store (T7). Call once at
    /// startup with the SAME `Arc` clone threaded to `P2PNode` and the iroh
    /// `EprService`, so every transport's `EprService` shares one store
    /// instance.
    pub fn with_memo_store(mut self, store: Arc<dyn crate::trust::VerificationMemoStore>) -> Self {
        self.memo_store = Some(store);
        self
    }

    /// Load the slug index from database (call after db_pool is set)
    pub async fn load_slug_index(&self) {
        let mut conn = match self.get_conn() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to get DB connection for slug index: {}", e);
                return;
            }
        };
        let app_ctx = db::AppContext::default_lamad();
        // Startup slug index refresh. This cache backs external HTML5 app
        // routing, so it must only index content that has a provenance
        // marker (dht_anchor_hash or p2p_published_at). If the drain has
        // not run yet, the index will be empty — that is the desired
        // behavior: we must never serve undrained rows to browsers.
        // App-bundle content formats served via /apps/{slug}: legacy
        // `html5-app` and Angular `spa-bundle` (e.g. lamad-spa). Storage is the
        // layer that resolves /apps and emits "App not found", so this must stay
        // in sync with doorway's app_file_cache $in:[html5-app,spa-bundle] fix
        // (b62c5ff2c) — a spa-bundle-only row (lamad-spa) was 404ing here.
        const APP_BUNDLE_FORMATS: [&str; 2] = ["html5-app", "spa-bundle"];

        let mut index = self.slug_index.write().await;
        index.clear();
        for fmt in APP_BUNDLE_FORMATS {
            let query = ContentQuery {
                content_format: Some(fmt.to_string()),
                limit: 100,
                ..Default::default()
            };
            // External slug index — require the Amber serving floor on every row.
            match db::content_diesel::list_content(
                &mut conn,
                &app_ctx,
                &query,
                db::content_diesel::MinTrust::Amber,
            ) {
                Ok(items) => {
                    for item in items {
                        let blob_hash = item.content.blob_hash.clone().unwrap_or_default();
                        if blob_hash.is_empty() {
                            continue;
                        }
                        // Index by row id (the slug the EPR router requests via
                        // /apps/{epr_id}) AND by the inner content_body slug, so a
                        // row whose inner slug differs from its id (lamad-spa's
                        // inner slug is "lamad") still resolves by either key.
                        index.insert(item.content.id.clone(), blob_hash.clone());
                        if let Some(ref content_body) = item.content.content_body {
                            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(content_body)
                            {
                                if let Some(slug) = obj.get("slug").and_then(|v| v.as_str()) {
                                    index.insert(slug.to_string(), blob_hash.clone());
                                }
                            }
                        }
                        info!(id = %item.content.id, blob_hash = %blob_hash, "Indexed app bundle");
                    }
                }
                Err(e) => {
                    warn!(format = fmt, "Failed to load slug index: {}", e);
                }
            }
        }
        info!(count = index.len(), "Slug index loaded");
    }

    /// Get a connection from the Diesel pool
    fn get_conn(&self) -> Result<PooledConn, StorageError> {
        self.db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))
    }

    /// Run the HTTP server
    pub async fn run(self: Arc<Self>) -> Result<(), StorageError> {
        let listener = TcpListener::bind(self.bind_addr).await?;
        info!(
            addr = %self.bind_addr,
            max_concurrent = MAX_CONCURRENT_REQUESTS,
            "HTTP server listening"
        );

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let server = self.clone();

            tokio::spawn(async move {
                // Admission moved from per-CONNECTION (acquire().await, which queued
                // under burst and wedged /health) to per-REQUEST (try_acquire shed)
                // in handle_request — see the Pillar 2 admission gate there.
                let service = service_fn(move |req| {
                    let server = server.clone();
                    async move { server.handle_request(req).await }
                });

                // Enable HTTP upgrades for WebSocket support
                // Without .with_upgrades(), WebSocket handshakes fail immediately
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .with_upgrades()
                    .await
                {
                    warn!(addr = %remote_addr, error = %err, "Connection error");
                }
            });
        }
    }

    /// Route requests to handlers
    async fn handle_request(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<ApiBody>, hyper::Error> {
        let path = req.uri().path().to_string();
        let method = req.method().clone();

        // Extract observation session ID before req is consumed -- used by middleware aspect.
        let obs_session_id: Option<String> = req
            .headers()
            .get("X-Observation-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Capture method string for middleware aspect before method is moved into the match.
        let method_str = method.to_string();

        debug!(method = %method, path = %path, "Incoming request");

        // ── Pillar 2 / layer 2: per-request admission (shed, never queue) ─────
        // Exempt /health, /version, and OPTIONS (preflight) so monitoring + CORS
        // stay live under shed. try_acquire (NOT acquire().await): at the ceiling
        // return 503 + Retry-After + X-Available-Permits immediately. The permit
        // is held for the whole request via `_admit` (function-scope binding).
        // The notary HEAD-declare write is carved out of the blanket shed: its
        // authority refusal (401/403, `handle_content_head`'s "AUTH FIRST" block)
        // must run before any admission decision — a non-author cannot be allowed
        // to win by racing the shed. Admission still applies to this route, just
        // AFTER authority is confirmed (see the `try_acquire_owned` call inside
        // `handle_content_head`, right before the conductor write).
        // `/debug/pprof/profile` is exempt for the opposite reason to /health:
        // not because it is cheap, but because it deliberately BLOCKS for 1..60s.
        // Holding a read-admission permit for the whole profile window is exactly
        // the slot-starvation the split read/write pools exist to prevent. The
        // endpoint carries its own, stricter admission control — the process-
        // global `PPROF_BUSY` flag admits exactly one concurrent profile and
        // answers 429 otherwise — so exempting it here removes a permit leak
        // without removing backpressure. Inert unless ELOHIM_PPROF_ENABLED.
        let admission_exempt = method == Method::OPTIONS
            || matches!(
                path.as_str(),
                "/health" | "/version" | "/debug/pprof/profile"
            )
            || is_head_declare_write(&method, &path);
        let _admit = if admission_exempt {
            None
        } else {
            // Reads (GET/HEAD) draw from a separate, larger pool than writes, so a
            // write burst (e.g. genesis seeding) cannot starve cheap projection
            // reads — the `/epr/*` read-path shed in genesis #1272 E2E. Shedding a
            // WRITE under burst is correct backpressure (the seeder retries the
            // catching-up shed); shedding a READ the node can serve is not.
            let is_read = admission_is_read_method(&method);
            let sem = if is_read {
                &self.read_semaphore
            } else {
                &self.request_semaphore
            };
            match sem.clone().try_acquire_owned() {
                Ok(permit) => Some(permit),
                Err(_) => {
                    let available = sem.available_permits();
                    warn!(
                        target: "upstream_shed",
                        counter = "storage_admission_shed_total",
                        path = %path,
                        pool = if is_read { "read" } else { "write" },
                        available,
                        "storage request admission at ceiling — shedding (503 + Retry-After)"
                    );
                    return Ok(crate::services::response::too_many_requests_with_retry(
                        STORAGE_SHED_RETRY_AFTER_SECS,
                        available,
                    )
                    .map(Either::Left));
                }
            }
        };

        let result = match (method, path.as_str()) {
            // CORS preflight for all routes
            (Method::OPTIONS, _) => Ok(Self::cors_preflight()),

            // Health check (supports ?detail=error|warn|info|debug|trace)
            (Method::GET, "/health") => {
                let query = req.uri().query().unwrap_or("");
                self.handle_health(query).await
            }

            // Build/version info
            (Method::GET, "/version") => {
                let info = elohim_compute::BuildInfo::new("elohim-storage");
                let body = serde_json::to_string(&info)
                    .unwrap_or_else(|_| r#"{"version":"unknown","commit":"unknown"}"#.to_string());
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body)))
                    .unwrap())
            }

            // Prometheus app-metrics surface (design-decision toolkit P0).
            // text/plain exposition of the toolkit Registry — scraped by the
            // Prometheus Operator via PodMonitor. No auth (in-cluster scrape;
            // same posture as /p2p/status).
            (Method::GET, "/metrics") => {
                let body = crate::metrics::gather_text();
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
                    .body(Full::new(Bytes::from(body)))
                    .unwrap())
            }

            // CPU profiling — the Go `net/http/pprof` wire contract, so Grafana
            // Alloy's `pyroscope.scrape` can treat this peer as an ordinary
            // Go-pprof CPU target. Blocks for ?seconds=N (default 15, clamped
            // 1..=60) and answers a gzip-framed profile.proto.
            //
            // Env-gated (ELOHIM_PPROF_ENABLED, default OFF): when disabled this
            // guard fails, the arm never matches, and the route falls through to
            // the 404 catch-all — which is precisely what a scraper sees from a
            // peer that does not carry the endpoint. Honest absence, not error.
            // Body + concurrency guard live in `crate::pprof_endpoint`; this arm
            // stays a router line (http.rs LoC ceiling, finding a711583b7334).
            (Method::GET, "/debug/pprof/profile") if crate::pprof_endpoint::is_enabled() => {
                let seconds = crate::pprof_endpoint::parse_seconds(req.uri().query().unwrap_or(""));
                Ok(crate::pprof_endpoint::profile_response(seconds).await)
            }

            // Shard API
            (Method::PUT, p) if p.starts_with("/shard/") => {
                let hash = p.strip_prefix("/shard/").unwrap_or("");
                self.handle_put_shard(req, hash).await
            }
            (Method::GET, p) if p.starts_with("/shard/") => {
                let hash = p.strip_prefix("/shard/").unwrap_or("");
                self.handle_get_shard(hash).await
            }
            (Method::HEAD, p) if p.starts_with("/shard/") => {
                let hash = p.strip_prefix("/shard/").unwrap_or("");
                self.handle_head_shard(hash).await
            }

            // Blob API (convenience with auto-sharding)
            (Method::PUT, p) if p.starts_with("/blob/") => {
                let hash = p.strip_prefix("/blob/").unwrap_or("");
                self.handle_put_blob(req, hash).await
            }
            (Method::GET, p) if p.starts_with("/blob/") => {
                let hash = p.strip_prefix("/blob/").unwrap_or("");
                let agent_id = Self::extract_agent_id(&req);
                self.handle_get_blob(hash, agent_id.as_deref()).await
            }

            // Manifest API
            (Method::GET, p) if p.starts_with("/manifest/") => {
                let hash = p.strip_prefix("/manifest/").unwrap_or("");
                self.handle_get_manifest(hash).await
            }

            // WebSocket upgrade for progress streaming
            (Method::GET, "/import/progress") if progress_ws::is_websocket_upgrade(&req) => {
                if let Some(ref hub) = self.progress_hub {
                    match progress_ws::handle_progress_upgrade(req, Arc::clone(hub)).await {
                        Ok(response) => Ok(response),
                        Err(e) => {
                            error!(error = %e, "WebSocket upgrade failed");
                            Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .body(Full::new(Bytes::from("WebSocket upgrade failed")))
                                .unwrap())
                        }
                    }
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Progress hub not enabled"}"#,
                        )))
                        .unwrap())
                }
            }

            // Import API (forwarded from doorway)
            (_, p) if p.starts_with("/import/") => {
                if let Some(ref import_api) = self.import_api {
                    // Lazy reconnection: if HcClient is not connected, attempt to reconnect
                    // This handles the case where elohim-storage starts before the hApp is installed
                    {
                        let api = import_api.read().await;
                        if api.needs_reconnect() {
                            drop(api); // Release read lock before acquiring write lock
                            let mut api_write = import_api.write().await;
                            if api_write.needs_reconnect() {
                                // Double-check after acquiring write lock
                                info!("Import API: Attempting lazy reconnection to conductor...");
                                match api_write.connect_conductor().await {
                                    Ok(_) => info!("Import API: Lazy reconnection successful"),
                                    Err(e) => {
                                        warn!(error = %e, "Import API: Lazy reconnection failed")
                                    }
                                }
                            }
                        }
                    }

                    let api = import_api.read().await;
                    api.handle_request(req, &path).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Import API not enabled. Set ENABLE_IMPORT_API=true"}"#,
                        )))
                        .unwrap())
                }
            }

            // P2P Status endpoint
            #[cfg(feature = "p2p")]
            (Method::GET, "/p2p/status") => self.handle_p2p_status().await,
            #[cfg(feature = "p2p")]
            (Method::GET, "/p2p/peers") => self.handle_p2p_peers().await,

            // Delivery peers — discovered peers with delivery capabilities
            // Used by frontend for multi-peer app delivery scoring
            #[cfg(feature = "p2p")]
            (Method::GET, "/api/v1/peers/delivery") => self.handle_delivery_peers().await,

            // Hosted-at binding resolution (doorway-federation-failover T2.2).
            // Serves the `hosted_agent_bindings` projection of the imagodei
            // Category-A2 `hosted-at` link so a SIBLING doorway can answer
            // "where is this agent hosted?" from substrate truth.
            (Method::GET, p) if p.starts_with("/api/v1/federation/hosted-binding/") => {
                let agent = p
                    .trim_start_matches("/api/v1/federation/hosted-binding/")
                    .to_string();
                self.handle_hosted_binding(&agent).await
            }

            // Sync API: /sync/v1/{h_app_id}/docs[/{doc_id}[/heads|/changes]]
            (method, p) if p.starts_with("/sync/v1/") => {
                if let Some(ref sync_manager) = self.sync_manager {
                    self.handle_sync_request(req, method, &path, sync_manager.clone())
                        .await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Sync API not enabled"}"#,
                        )))
                        .unwrap())
                }
            }

            // Projector status — cursor + reconciliation-lag per (pillar, kind).
            // Matched before /api/v1/ catch-all so the registry can be injected
            // directly from HttpServer state rather than through handle_api_request.
            (Method::GET, "/api/v1/status/projector") => {
                if let Some(ref pool) = self.db_pool {
                    if let Some(ref registry) = self.manifest_registry {
                        let app_ctx = crate::db::AppContext {
                            h_app_id: "elohim".to_string(),
                            local_libp2p_peer_id: None,
                        };
                        crate::api::projector_status::handle(
                            req,
                            Method::GET,
                            "",
                            pool,
                            &app_ctx,
                            registry,
                        )
                        .await
                    } else {
                        // No registry wired — return empty status (not an error).
                        let empty = crate::projector::ProjectorStatusView {
                            cursors: vec![],
                            lag: vec![],
                        };
                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(
                                serde_json::to_string(&empty).unwrap_or_default(),
                            )))
                            .unwrap())
                    }
                } else {
                    Ok(response::service_unavailable(
                        "Database pool not configured — projector status unavailable",
                    ))
                }
            }

            // Write-through status — effective per-(pillar, kind) flag composed
            // across all 4 override layers. Matched before the /api/v1/
            // catch-all so the registry + state are injected from HttpServer
            // directly. EPR Phase 2B Task C.4.
            (Method::GET, "/api/v1/status/write-through") => {
                let registry = self
                    .manifest_registry
                    .clone()
                    .unwrap_or_else(|| Arc::new(crate::projector::ManifestRegistry::empty()));
                let state = self
                    .write_through_state
                    .clone()
                    .unwrap_or_else(|| Arc::new(crate::write_through::WriteThroughState::empty()));
                crate::api::write_through_status::handle(
                    req,
                    Method::GET,
                    "",
                    &state,
                    registry.as_ref(),
                )
                .await
            }

            // Conductor authority-arc Auto policy gauge (operational, Cat-C; spec §6 of
            // 2026-06-13-conductor-authority-arc-auto-policy.md). Surfaces derive()'s
            // computed arc aim + live inputs + "why"; on the deployed line the
            // actuatable lever is {0,1} (spike §2), so the fractional value is a SIGNAL.
            (Method::GET, "/api/v1/status/arc-policy") => self.handle_arc_policy_status().await,

            // Operator/agent-initiated authority-arc actuation (explicit POST trigger,
            // spec §5). Reads the authorizing sets-authority-arc commitment, runs the
            // coverage gate, and RESTARTS the conductor on success. Node-local
            // (deliberately NOT in build_manifest); no auto-subscriber (churn risk).
            (Method::POST, "/admin/arc-policy/actuate") => {
                self.handle_arc_policy_actuate(req).await
            }

            // Operator/seed deterministic shard-manifest + agent-keyed locations
            // write — the gated lever that lights the resilience card's
            // distributionState + stewardingCollectives for blob-backed demo/test
            // content WITHOUT waiting on full P2P gossip. Off by default; requires
            // ALLOW_SEED_SHARD_MANIFEST=1. The genuine distribution path is
            // POST /db/content (blob) → distribute_shards; this is the seed analogue
            // for a thin mesh. Node-local (deliberately NOT in build_manifest);
            // status="seeded" keeps the rows' provenance legible. Idempotent.
            (Method::PUT, "/admin/seed/shard-manifest") => {
                self.handle_seed_shard_manifest(req).await
            }

            // Operator-runtime-surface slice 1: commitment-gated reconcile verb.
            // The peer authorizes for ITSELF (delegates-compute grant with
            // scope="operator-reconcile" via the shared authorize_operation gate),
            // kicks the existing projection-reconcile loop, and answers with an
            // attestation naming the commitment CID it acted under. Manifest-
            // declared (doorway-proxied) — see api/operator_verbs.rs for why
            // that is safe, unlike the off-manifest authorize-operation oracle.
            (Method::POST, "/api/v1/operator/reconcile") => {
                if let Some(pool) = self.db_pool.as_ref() {
                    crate::api::operator_verbs::handle_reconcile(
                        req,
                        pool,
                        self.reconcile_kick.as_ref(),
                        &self.self_cid,
                    )
                    .await
                } else {
                    Ok(response::service_unavailable("db pool not available"))
                }
            }

            // Dev-seed endpoint for `delegates-compute` mishpat commitments
            // (Che op-gate Slice 1, §14). Flag-gated (ALLOW_SEED_DELEGATES_COMPUTE=1);
            // writes directly into mishpat_commitments with a synthesized dev anchor.
            // Deliberately NOT in build_manifest() — never doorway-proxied.
            // Revoke variant: body { cid, revoke: true }.
            (Method::POST, "/admin/seed/delegates-compute") => {
                if let Some(pool) = self.db_pool.as_ref() {
                    crate::api::seed_delegates_compute::handle(req, pool).await
                } else {
                    Ok(response::service_unavailable("db pool not available"))
                }
            }

            // Operator/seed write of a declared network-stakes manifest — T10 runtime
            // half (head-plane trust-gradient program §3 L5; minutes-quiesce fixture-
            // trust-swarm plan §3 W2, task Q6). Accepts the seeder's stakes-declaration
            // artifact (`genesis/seeder/src/corpus-trust.ts`, STAKES-DECLARATION-SEAM-Q6
            // contract) and stores it into `manifests` (manifest_kind="network-stakes")
            // — local-only, unsigned, NOT DHT-published, same bootstrap-local posture as
            // `services::bootstrap_manifests`. Flag-gated (ALLOW_SEED_NETWORK_STAKES=1);
            // deliberately NOT in build_manifest() (never doorway-proxied).
            // `ManifestStakesResolver::stage_for` is the read side.
            (Method::PUT, "/admin/seed/network-stakes") => {
                if let Some(pool) = self.db_pool.as_ref() {
                    crate::api::seed_network_stakes::handle(
                        req,
                        pool,
                        self.network_stakes_resolver.clone(),
                    )
                    .await
                } else {
                    Ok(response::service_unavailable("db pool not available"))
                }
            }

            // Operator setter for per-object blob transport affinity
            // (iroh-toggle sprint). POST /admin/blob-transport-affinity/{hash}
            // body {"affinity":"iroh-only"|"libp2p-only"|"prefer-iroh"|
            // "prefer-libp2p"|"auto"}. "auto" (or null) clears to default
            // policy. Node-local; makes the choose_backend toggle usable for
            // A/B without waiting on gossip. Inert in libp2p mode.
            #[cfg(feature = "p2p-iroh")]
            (Method::POST, p) if p.starts_with("/admin/blob-transport-affinity/") => {
                let hash = p
                    .strip_prefix("/admin/blob-transport-affinity/")
                    .unwrap_or("")
                    .to_string();
                self.handle_set_blob_transport_affinity(&hash, req).await
            }

            // SSE event stream — must be matched before the /api/v1/ catch-all
            (Method::GET, "/api/v1/events") => {
                if let Some(ref services) = self.services {
                    let response = crate::sse::create_sse_stream(&services.events);
                    return Ok(response.map(Either::Right));
                } else {
                    Ok(response::service_unavailable("Event bus not available"))
                }
            }

            // Cache stream for projection warm-up (SSE)
            (Method::GET, "/api/v1/cache/stream") => {
                if let Some(ref pool) = self.db_pool {
                    let response = crate::cache_stream::create_cache_stream(
                        pool.clone(),
                        "lamad", // Default app context for cache stream
                    );
                    return Ok(response.map(Either::Right));
                } else {
                    Ok(response::service_unavailable("Database not available"))
                }
            }

            // Filesystem parity sweep — T18 diagnostic endpoint.
            // Reports the diff between local pantry and last gossiped inventory.
            // Matched before the /api/v1/ catch-all.
            (Method::GET, "/api/v1/diagnostics/inventory-parity") => {
                self.handle_inventory_parity().await
            }

            // Bounds-validator diagnostic — Sprint 2 Task 6. Runs the substrate-wide
            // bounds_validator::validate against a caller-supplied EconomicEvent payload
            // + the production CommitmentFetcher / RateHistory. Returns
            // BoundsValidationResultView per the schema. Matched before the /api/v1/ catch-all.
            (Method::POST, "/api/v1/diagnostics/validate-bounds") => {
                if let Some(ref pool) = self.db_pool {
                    let hc_lamad = self.hc_registry.as_ref().and_then(|r| r.lamad_client());
                    crate::api::diagnostics_bounds::handle(
                        req,
                        Method::POST,
                        pool,
                        hc_lamad.as_ref(),
                    )
                    .await
                } else {
                    Ok(response::service_unavailable(
                        "Database pool not configured — /api/v1/diagnostics/validate-bounds unavailable",
                    ))
                }
            }

            // Signal-emit endpoint — composes EPR Envelope, signs via conductor,
            // ingests. Matched before the /api/v1/ catch-all so the manifest
            // registry + signing client are injected directly from HttpServer
            // state. EPR Phase 2B Task C.2.
            (method, "/api/v1/signal/emit") => {
                if let Some(ref pool) = self.db_pool {
                    if let Some(ref registry) = self.manifest_registry {
                        crate::api::signal_emit::handle(
                            req,
                            method,
                            pool,
                            registry,
                            self.signing_client.as_ref(),
                            self.write_through_state.as_ref(),
                        )
                        .await
                    } else {
                        Ok(response::service_unavailable(
                            "ManifestRegistry not configured — /api/v1/signal/emit unavailable",
                        ))
                    }
                } else {
                    Ok(response::service_unavailable(
                        "Database pool not configured — /api/v1/signal/emit unavailable",
                    ))
                }
            }

            // Acquisition DevicePins (spec §1.1, §4.4) — OWN NODE ONLY.
            // Deliberately absent from build_manifest(); a doorway MUST NEVER
            // serve another agent's pins. Matched before the /api/v1/ catch-all
            // so these routes are DB-only (airplane-mode): no p2p feature gate.
            (Method::GET, "/api/v1/pins") => self.handle_list_pins().await,
            (Method::POST, "/api/v1/pins") => self.handle_create_pin(req).await,
            // GET /api/v1/pins/{eprId}/pull — per-EPR pull progress (own node
            // only). Matched before the bare-id catch-all so the `/pull` suffix
            // routes here, not to a pin-id parse.
            (Method::GET, p) if p.starts_with("/api/v1/pins/") && p.ends_with("/pull") => {
                let epr_id = p
                    .trim_start_matches("/api/v1/pins/")
                    .trim_end_matches("/pull")
                    .to_string();
                self.handle_epr_pull(&epr_id).await
            }
            (Method::DELETE, p) if p.starts_with("/api/v1/pins/") => {
                let id_str = p.trim_start_matches("/api/v1/pins/").to_string();
                self.handle_remove_pin(&id_str).await
            }

            // Observation Session API -- must be matched before the /api/v1/ catch-all
            (method, p) if p.starts_with("/api/v1/observations") => {
                if let Some(ref pool) = self.db_pool {
                    let sub_path = p.strip_prefix("/api/v1/observations").unwrap_or("");
                    self.handle_observation_request(req, method, sub_path, pool.clone())
                        .await
                } else {
                    Ok(response::service_unavailable("Database not available"))
                }
            }

            // Enriched API: Business logic endpoints
            (method, p) if p.starts_with("/api/v1/") => {
                if let Some(ref pool) = self.db_pool {
                    // D.2: thread swarm_tx into handle_api_request so epr::handle
                    // can issue KadStartProviding on successful puts.
                    // D.7: also thread local_libp2p_peer_id so FederatedEprStore
                    // can dedup self-reports from the providers list.
                    #[cfg(feature = "p2p")]
                    let swarm_tx = self.p2p_handle.as_ref().map(|h| h.command_sender());
                    #[cfg(not(feature = "p2p"))]
                    let swarm_tx: Option<
                        tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>,
                    > = None;
                    #[cfg(feature = "p2p")]
                    let local_peer_id = self.p2p_handle.as_ref().map(|h| h.local_peer_id());
                    #[cfg(not(feature = "p2p"))]
                    let local_peer_id: Option<String> = None;
                    // Phase 5: thread p2p_handle so cluster/peer-topology handlers
                    // can construct a Federator for view-federation fan-out.
                    let p2p_handle = self.p2p_handle.clone();
                    crate::api::handle_api_request(
                        req,
                        method,
                        &path,
                        pool.clone(),
                        self.services.clone(),
                        self.hc_registry.clone(),
                        swarm_tx,
                        local_peer_id,
                        self.fan_out_ctx.clone(),
                        p2p_handle,
                        self.elohim_capability.clone(),
                        self.render_capability.clone(),
                        self.extensions.clone(),
                    )
                    .await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "API not available - database pool not configured"}"#,
                        )))
                        .unwrap())
                }
            }

            // Database API: Content, Paths, Stats
            (method, p) if p.starts_with("/db/") => {
                if self.db_pool.is_some() {
                    self.handle_db_request(req, method, &path).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Content database not enabled"}"#,
                        )))
                        .unwrap())
                }
            }

            // Delivery capability probe: HEAD /apps/{slug}/_capability
            // Lightweight probe for delivery negotiation — no body, just headers.
            // Reports whether extraction cache is warm for this app.
            (Method::HEAD, p) if p.starts_with("/apps/") && p.ends_with("/_capability") => {
                self.handle_app_capability(p).await
            }

            // HTML5 App serving: /apps/{slug}/{file_path}
            (Method::GET, p) if p.starts_with("/apps/") => {
                if self.db_pool.is_some() {
                    let query = req.uri().query().unwrap_or("");
                    self.handle_app_request(&path, query).await
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Content database not enabled"}"#,
                        )))
                        .unwrap())
                }
            }

            // Session API: Local session management for Tauri native handoff
            (Method::GET, "/session") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_get_session(pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::POST, "/session") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_create_session(req, pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::DELETE, "/session") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_delete_session(pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::GET, "/session/all") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_list_sessions(pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::POST, "/session/intent") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_set_session_intent(req, pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }

            // Steward-side portal session exchange (doorway → portal-handoff,
            // GAP-2b). A browser redirected from a doorway login portal lands at
            // the steward's portal-host origin carrying a single-use redemption
            // token. Storage redeems the token back-channel against the *issuing*
            // (TOFU-allowlisted) doorway and, on success, seeds a LocalSession +
            // sets the elohim_session cookie. This is a steward-local browser
            // surface reached at the portal-host origin — it is intentionally
            // NOT declared in build_manifest() (never doorway-proxied).
            (Method::POST, "/session/exchange") => {
                if let Some(ref pool) = self.db_pool {
                    self.handle_session_exchange(req, pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }

            // Auth identity endpoint: same wire shape as doorway's /auth/me so
            // the standalone peer OAuth portal bundle can consume either source
            // without knowing which transport is in play (spec §3.2 Transport β).
            //
            // When the request carries an `elohim_session` cookie naming an
            // existing, active session, that session is projected (GAP-2b multi-
            // session browser path). Otherwise the current single-active-session
            // behavior is preserved exactly (Tauri-native compat must not change).
            (Method::GET, "/auth/me") => {
                if let Some(ref pool) = self.db_pool {
                    let cookie_session_id = extract_session_cookie(req.headers());
                    self.handle_auth_me(pool.clone(), cookie_session_id).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }

            // EPR Head API: DAG-CBOR encoded three-pillar metadata
            (Method::PUT, p) if p.starts_with("/epr-head/") => {
                let id = p.strip_prefix("/epr-head/").unwrap_or("");
                self.handle_put_epr_head(req, id).await
            }
            (Method::GET, p) if p.starts_with("/epr-head/") => {
                let id = p.strip_prefix("/epr-head/").unwrap_or("");
                self.handle_get_epr_head(req, id).await
            }

            // IPFS block API: raw block retrieval by CID
            (Method::GET, p) if p.starts_with("/ipfs/") => {
                let cid_str = p.strip_prefix("/ipfs/").unwrap_or("");
                self.handle_get_ipfs_block(cid_str).await
            }
            (Method::HEAD, p) if p.starts_with("/ipfs/") => {
                let cid_str = p.strip_prefix("/ipfs/").unwrap_or("");
                self.handle_head_ipfs_block(cid_str).await
            }

            // DAG API: decoded IPLD operations
            (Method::GET, p) if p.starts_with("/dag/") && p.ends_with("/links") => {
                let cid_str = p
                    .strip_prefix("/dag/")
                    .and_then(|s| s.strip_suffix("/links"))
                    .unwrap_or("");
                self.handle_get_dag_links(cid_str).await
            }
            (Method::GET, p) if p.starts_with("/dag/") => {
                let cid_str = p.strip_prefix("/dag/").unwrap_or("");
                self.handle_get_dag(cid_str).await
            }

            // Account API: Import/Export account packages
            (Method::POST, "/account/import") => {
                if let Some(ref pool) = self.db_pool {
                    self.do_account_import(req, pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }
            (Method::GET, p) if p.starts_with("/account/export/") => {
                let human_id = p.strip_prefix("/account/export/").unwrap_or("");
                if let Some(ref pool) = self.db_pool {
                    self.do_account_export(human_id, pool.clone()).await
                } else {
                    Ok(response::service_unavailable("Database not enabled"))
                }
            }

            // Route manifest — declares the API surface for doorway dynamic discovery
            (Method::GET, "/manifest") => self.handle_manifest().await,

            // Admin: extraction cache stats
            (Method::GET, "/admin/extraction-cache/stats") => {
                self.handle_extraction_cache_stats().await
            }

            // Admin: evict app from extraction cache
            (Method::POST, p) if p.starts_with("/admin/extraction-cache/evict/") => {
                let slug = p
                    .strip_prefix("/admin/extraction-cache/evict/")
                    .unwrap_or("");
                self.handle_extraction_cache_evict(slug).await
            }

            // Admin: replace the layer-4 write-through override.
            // EPR Phase 2B Task C.6 layer 4 (live admin trigger).
            (method, "/admin/write-through") => {
                let state = self
                    .write_through_state
                    .clone()
                    .unwrap_or_else(|| Arc::new(crate::write_through::WriteThroughState::empty()));
                crate::api::write_through_admin::handle(req, method, &state).await
            }

            // SSR: direct-peer rendering (ssr feature only)
            //
            // GET /spa/* — peer-to-peer libp2p-routed direct rendering.
            //   The leading "/spa/" prefix is stripped and the remainder is
            //   treated as an Angular route, e.g. /spa/lamad/concept/foo →
            //   renders /lamad/concept/foo.
            //
            // POST /render — internal endpoint for doorway co-location.
            //   Body: JSON { "url": "/lamad/concept/foo" }
            //   Response: rendered HTML (text/html).
            //
            // Both return 503 when the renderer is absent (feature compiled in
            // but SSR_BUNDLE_PATH unset or bundle load failed).
            #[cfg(feature = "ssr")]
            (Method::GET, p) if p.starts_with("/spa/") => {
                let inner_path = format!("/{}", &p[5..]); // strip "/spa/"
                match self.ssr_state.as_ref() {
                    Some(state) => {
                        match crate::ssr::render_url(state, inner_path).await {
                            Ok(html) => Ok(Self::html_response(html)),
                            Err(e) => {
                                tracing::warn!(
                                    target: "elohim_storage::ssr",
                                    error = %e,
                                    path = %p,
                                    "SSR render error"
                                );
                                Ok(Response::builder()
                                    .status(StatusCode::SERVICE_UNAVAILABLE)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(
                                        r#"{"error":"render failed"}"#,
                                    )))
                                    .unwrap())
                            }
                        }
                    }
                    None => Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error":"SSR not available — SSR_BUNDLE_PATH not set or bundle failed to load"}"#,
                        )))
                        .unwrap()),
                }
            }

            #[cfg(feature = "ssr")]
            (Method::POST, "/render") => {
                match self.ssr_state.as_ref() {
                    Some(state) => {
                        // Parse body: { "url": "..." }. On body-read failure, return 400.
                        let url = match req.collect().await {
                            Ok(collected) => {
                                let body_bytes = collected.to_bytes();
                                let parsed: serde_json::Value =
                                    serde_json::from_slice(&body_bytes).unwrap_or_default();
                                parsed
                                    .get("url")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("/")
                                    .to_string()
                            }
                            Err(_) => "/".to_string(),
                        };
                        match crate::ssr::render_url(state, url).await {
                            Ok(html) => Ok(Self::html_response(html)),
                            Err(e) => {
                                tracing::warn!(
                                    target: "elohim_storage::ssr",
                                    error = %e,
                                    "SSR /render error"
                                );
                                Ok(Response::builder()
                                    .status(StatusCode::SERVICE_UNAVAILABLE)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(
                                        r#"{{"error":"render failed: {e}"}}"#
                                    ))))
                                    .unwrap())
                            }
                        }
                    }
                    None => Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error":"SSR not available — SSR_BUNDLE_PATH not set or bundle failed to load"}"#,
                        )))
                        .unwrap()),
                }
            }

            // Runtime-served native chrome static assets (the omnibar element).
            // The Tauri shell talks to this sidecar directly, so it serves the
            // element the same way the doorway does. V8-free (light asset crate),
            // so this is in the DEFAULT build, NOT behind the `ssr` feature.
            // Placed ABOVE the catch-all so the 404 fallthrough can't shadow it
            // (the sidecar's equivalent of the doorway's is_service_path guard —
            // storage has no EPR-router wildcard, so an explicit arm here is the
            // guard).
            (Method::GET, p) if p.starts_with("/chrome/") => Ok(Self::handle_chrome_asset(p)),

            // Not found
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not Found")))
                .unwrap()),
        };

        match result {
            Ok(mut response) => {
                // Observation middleware aspect: record non-2xx failures when session is active.
                if let Some(ref session_id) = obs_session_id {
                    self.maybe_observe_request(
                        session_id,
                        &method_str,
                        &path,
                        response.status().as_u16(),
                    );
                }

                // Add CORS headers to ALL responses (not just preflight)
                let headers = response.headers_mut();
                headers.insert(
                    "Access-Control-Allow-Origin",
                    hyper::header::HeaderValue::from_static("*"),
                );
                headers.insert(
                    "Access-Control-Allow-Methods",
                    hyper::header::HeaderValue::from_static(
                        "GET, PUT, POST, PATCH, DELETE, HEAD, OPTIONS",
                    ),
                );
                headers.insert(
                    "Access-Control-Allow-Headers",
                    hyper::header::HeaderValue::from_static(
                        "Content-Type, Authorization, X-Agent-Id, X-Agent-Cid, X-Schema-Version, X-Observation-Id",
                    ),
                );
                Ok(response.map(Either::Left))
            }
            Err(e) => {
                error!(error = %e, "Request error");
                Ok(Self::escaped_error_response(&e).map(Either::Left))
            }
        }
    }

    /// Map an error that ESCAPED a handler onto the wire.
    ///
    /// Everything is a plain 500 here by design: a handler that did not choose
    /// its own status does not get one invented for it at the last seam. The ONE
    /// exception is a conductor-admission shed — nothing was dispatched, the
    /// conductor never saw the call, so it is backpressure and must carry
    /// Retry-After. Before this classification existed the shed reached the wire
    /// as a bare `500 Error: ...`, indistinguishable from a conductor that tried
    /// and broke, which made the gate's own `is_admission_shed` contract
    /// unusable by anything outside this process.
    /// Both arms carry the cross-origin headers themselves (`with_cors_headers`
    /// parity): dropping `Cross-Origin-Resource-Policy` here would make a COEP
    /// embedder block the body, degrading a legible 503-with-Retry-After into
    /// an opaque network failure client-side.
    fn escaped_error_response(e: &StorageError) -> Response<Full<Bytes>> {
        let mut response = if crate::conductor_admission::is_admission_shed(e) {
            crate::services::response::admission_shed_backpressure()
        } else {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from(format!("Error: {}", e))))
                .unwrap()
        };
        let headers = response.headers_mut();
        headers.insert(
            "Access-Control-Allow-Origin",
            hyper::header::HeaderValue::from_static("*"),
        );
        headers.insert(
            "Cross-Origin-Resource-Policy",
            hyper::header::HeaderValue::from_static("cross-origin"),
        );
        response
    }

    /// Health check endpoint with tiered detail via `?detail=` query parameter.
    ///
    /// Detail levels (cumulative):
    /// - `error` / `warn`: status + build info only
    /// - `info` (default): adds blob stats and import status
    /// - `debug`: adds manifest count, concurrency limit, app index size
    /// - `trace`: adds semaphore permits, db pool, extraction cache status
    async fn handle_health(&self, query: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        let detail = query
            .split('&')
            .find_map(|p| p.strip_prefix("detail="))
            .and_then(|v| v.parse::<elohim_compute::DetailLevel>().ok())
            .unwrap_or_default();

        let build = elohim_compute::BuildInfo::new("elohim-storage");

        // error + warn level: always present
        let mut body = serde_json::json!({
            "status": "ok",
            "build": build,
        });
        // Promoted to default detail (was Trace-only) so any caller — incl.
        // doorway honoring backpressure — can read remaining admission headroom.
        body["semaphorePermits"] = serde_json::json!(self.request_semaphore.available_permits());
        // Read pool headroom — separate from writes so a caller can see whether
        // reads are being starved (they should not be) vs writes shedding.
        body["readSemaphorePermits"] = serde_json::json!(self.read_semaphore.available_permits());

        // info level (default): basic operational data
        if detail >= elohim_compute::DetailLevel::Info {
            let stats = self.blob_store.stats().await?;
            body["blobs"] = serde_json::json!({
                "total": stats.total_blobs,
                "iroh_served": self.blob_iroh_served_count.load(std::sync::atomic::Ordering::Relaxed),
                "libp2p_served": self.blob_libp2p_served_count.load(std::sync::atomic::Ordering::Relaxed),
            });
            body["bytes"] = serde_json::json!(stats.total_bytes);
            body["importEnabled"] = serde_json::json!(self.import_api.is_some());
            body["conductor"] = serde_json::json!({
                "mode": if self.embedded_conductor { "embedded" } else { "external" },
            });

            // Backing-aware DHT-participation identity (Tier C — peer-discovery
            // fractal-federation spec, 2026-07-09 §6). This node self-reports the
            // part of its DHT identity that storage can honestly observe: the
            // per-role DNA hash as its own cell sees it (canonical `uhC0k…`).
            //
            // HONESTY GAP (documented, not fabricated): the DNA hash ALONE does
            // NOT prove co-membership — two nodes with identical DNA hashes can
            // sit on partitioned DHTs when their discovery *backing* (kitsune
            // bootstrap table + signal relay) differs (the adam/matthew
            // diagnosis). That backing lives in the CONDUCTOR's kitsune plane,
            // NOT storage's libp2p mesh, so it is not visible from here. The
            // doorway `/health` `dhtBacking` block is the honest home for the
            // backing; read `dnaHashes` together with it for the full identity.
            let mut dna_hashes = serde_json::Map::new();
            if let Some(reg) = self.hc_registry.as_ref() {
                if let Some(hc) = reg.infrastructure.as_ref() {
                    dna_hashes.insert(
                        "infrastructure".to_string(),
                        serde_json::json!(hc.cell_id().dna_hash().to_string()),
                    );
                }
                if let Some(hc) = reg.imagodei.as_ref() {
                    dna_hashes.insert(
                        "imagodei".to_string(),
                        serde_json::json!(hc.cell_id().dna_hash().to_string()),
                    );
                }
                if let Some(hc) = reg.lamad_client() {
                    dna_hashes.insert(
                        "lamad".to_string(),
                        serde_json::json!(hc.cell_id().dna_hash().to_string()),
                    );
                }
            }
            body["dhtParticipation"] = serde_json::json!({
                "dnaHashes": dna_hashes,
                // The discovery backing is NOT observable from storage's plane.
                "backingVisible": false,
                "backingSource": "doorway /health dhtBacking",
            });
        }

        // debug level: resource details
        if detail >= elohim_compute::DetailLevel::Debug {
            body["manifests"] = serde_json::json!(self.manifests.read().await.len());
            body["concurrencyLimit"] = serde_json::json!(MAX_CONCURRENT_REQUESTS);
            body["slugIndex"] = serde_json::json!(self.slug_index.read().await.len());
        }

        // trace level: full internal state
        if detail >= elohim_compute::DetailLevel::Trace {
            // semaphorePermits promoted to default detail (above).
            body["dbPoolEnabled"] = serde_json::json!(self.db_pool.is_some());
            body["extractionCacheEnabled"] = serde_json::json!(self.extraction_cache.is_some());
        }

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// GET /manifest - Declare the route surface for doorway dynamic discovery
    ///
    /// Returns a `DoorwayRoutes` JSON payload listing all `/api/v1/*` and `/db/*`
    /// routes that doorway should proxy to clients, plus the blob proxy config.
    /// Infrastructure endpoints (/health, /shard/*, /sync/*, /import/*) are
    /// intentionally omitted — doorway proxies only API routes.
    async fn handle_manifest(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        let routes = build_manifest();
        let body = serde_json::to_string(&routes)
            .map_err(|e| StorageError::Internal(format!("Failed to serialize manifest: {}", e)))?;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap())
    }

    /// GET /admin/extraction-cache/stats — Extraction cache statistics
    ///
    /// Returns the number of warm apps, total cached bytes, budget, and TTL.
    /// Used by delivery diagnostics to observe the cache layer without evicting.
    async fn handle_extraction_cache_stats(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = if let Some(ref cache) = self.extraction_cache {
            let stats = cache.stats().await;
            serde_json::json!({
                "warmApps": stats.cached_apps,
                "totalCachedBytes": stats.total_cached_bytes,
                "budgetBytes": stats.budget_bytes,
                "ttlSecs": stats.ttl_secs,
            })
        } else {
            serde_json::json!({ "error": "extraction cache not configured" })
        };
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// POST /admin/extraction-cache/evict/{slug} — Evict an app from the extraction cache
    ///
    /// Forces re-extraction on the next request for this app.
    /// Used by delivery diagnostics tests to peel back the extraction layer
    /// and reveal the blob decompression layer beneath.
    async fn handle_extraction_cache_evict(
        &self,
        slug: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if slug.is_empty() {
            let body = serde_json::json!({ "error": "missing slug" });
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(body.to_string())))
                .unwrap());
        }
        let body = if let Some(ref cache) = self.extraction_cache {
            cache.evict_app(slug).await.map_err(|e| {
                StorageError::Internal(format!("Extraction cache evict failed: {}", e))
            })?;
            info!(slug = %slug, "Extraction cache evicted by admin");
            serde_json::json!({ "slug": slug, "evicted": true })
        } else {
            serde_json::json!({ "error": "extraction cache not configured" })
        };
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// PUT /shard/{hash} - Store a shard
    async fn handle_put_shard(
        &self,
        req: Request<Incoming>,
        expected_hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Read body
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let data = body.to_bytes();

        // Verify hash - normalize both to hex for comparison
        // URL may contain raw hex, sha256-prefixed, or CID format
        let computed_hash = BlobStore::compute_hash(&data);
        let computed_hex = computed_hash
            .strip_prefix("sha256-")
            .unwrap_or(&computed_hash);
        let expected_hex = expected_hash
            .strip_prefix("sha256-")
            .unwrap_or(expected_hash);

        if !expected_hash.is_empty() && computed_hex != expected_hex {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(
                    "Hash mismatch: expected {}, got {}",
                    expected_hash, computed_hash
                ))))
                .unwrap());
        }

        // Store shard
        let result = self.blob_store.store(&data).await?;

        info!(
            hash = %result.hash,
            size = result.size_bytes,
            existed = result.already_existed,
            "Stored shard"
        );

        let body = serde_json::json!({
            "hash": result.hash,
            "sizeBytes": result.size_bytes,
            "alreadyExisted": result.already_existed,
        });

        Ok(Response::builder()
            .status(if result.already_existed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            })
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap())
    }

    /// GET /shard/{hash} - Retrieve a shard
    async fn handle_get_shard(&self, hash: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing shard hash")))
                .unwrap());
        }

        match self.blob_store.get(hash).await {
            Ok(data) => {
                info!(hash = %hash, size = data.len(), "Serving shard");

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/octet-stream")
                    .header(header::CONTENT_LENGTH, data.len())
                    .header(header::ETAG, format!("\"{}\"", hash))
                    .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                    .body(Full::new(Bytes::from(data)))
                    .unwrap())
            }
            Err(StorageError::NotFound(_)) => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Shard not found")))
                .unwrap()),
            Err(e) => Err(e),
        }
    }

    /// HEAD /shard/{hash} - Check if shard exists
    async fn handle_head_shard(&self, hash: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::new()))
                .unwrap());
        }

        match self.blob_store.size(hash).await {
            Ok(size) => Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::CONTENT_LENGTH, size)
                .header(header::ETAG, format!("\"{}\"", hash))
                .body(Full::new(Bytes::new()))
                .unwrap()),
            Err(_) => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::new()))
                .unwrap()),
        }
    }

    /// PUT /blob/{hash} - Store blob with auto-sharding
    async fn handle_put_blob(
        &self,
        req: Request<Incoming>,
        expected_hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Get content type from header
        let mime_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let agent_id =
            Self::extract_agent_id(&req).unwrap_or_else(|| "did:elohim:storage".to_string());

        // Read body
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let data = body.to_bytes().to_vec();

        self.put_blob_bytes(data, expected_hash, &mime_type, &agent_id)
            .await
    }

    /// Inner implementation of blob PUT — decoupled from `Request<Incoming>` so
    /// integration tests can call it directly without a live HTTP connection.
    async fn put_blob_bytes(
        &self,
        data: Vec<u8>,
        expected_hash: &str,
        mime_type: &str,
        agent_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Verify hash if provided - normalize both to hex for comparison
        // URL may contain raw hex, sha256-prefixed, or CID format
        let computed_hash = BlobStore::compute_hash(&data);
        let computed_hex = computed_hash
            .strip_prefix("sha256-")
            .unwrap_or(&computed_hash);
        let expected_hex = expected_hash
            .strip_prefix("sha256-")
            .unwrap_or(expected_hash);

        if !expected_hash.is_empty() && computed_hex != expected_hex {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(format!(
                    "Hash mismatch: expected {}, got {}",
                    expected_hash, computed_hash
                ))))
                .unwrap());
        }

        // Create shard encoder and generate manifest
        let encoder = ShardEncoder::new(crate::sharding::ShardConfig::default());
        let manifest = match encoder.create_manifest(&data, mime_type, "commons") {
            Ok(m) => m,
            Err(e) => {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from(format!(
                        "shard encoding failed: {e}"
                    ))))
                    .unwrap());
            }
        };

        // Store each shard
        for (i, shard_hash) in manifest.shard_hashes.iter().enumerate() {
            // For "none" encoding, the whole blob is one shard
            let shard_data = if manifest.encoding == "none" {
                data.clone()
            } else {
                // For chunked encoding, split the data
                let start = i * manifest.shard_size as usize;
                let end = ((i + 1) * manifest.shard_size as usize).min(data.len());
                data[start..end].to_vec()
            };

            // Verify shard hash matches
            let actual_hash = BlobStore::compute_hash(&shard_data);
            if actual_hash != *shard_hash {
                warn!(
                    expected = %shard_hash,
                    actual = %actual_hash,
                    index = i,
                    "Shard hash mismatch during blob storage"
                );
            }

            self.blob_store.store(&shard_data).await?;

            // Register with Node Registry if available
            if let Some(ref nr_api) = self.node_registry_api {
                let assignment = crate::node_registry_api::ShardAssignment {
                    assignment_hash: None,
                    content_hash: expected_hex.to_string(), // The full content hash
                    // Ideally, use the actual agent ID (e.g. from X-Agent-Id header) if provided;
                    // fallback to a generic placeholder or the configured connection's role for now.
                    custodian_did: agent_id.to_string(),
                    shard_index: i as u32,
                    strategy: crate::node_registry_api::ShardingStrategy::Geographic,
                    status: crate::node_registry_api::ShardStatus::Active,
                    verified_at: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                if let Err(e) = nr_api.create_shard_assignment(assignment).await {
                    warn!(
                        error = %e,
                        shard_index = i,
                        "Failed to register shard assignment with Node Registry"
                    );
                } else {
                    info!(
                        shard_index = i,
                        content_hash = %expected_hex,
                        "Registered shard assignment with Node Registry"
                    );
                }
            }
        }

        // Store manifest
        self.manifests
            .write()
            .await
            .insert(manifest.blob_hash.clone(), manifest.clone());

        // Blob ingestion is also a producer for the operational
        // shard->blob relation. The generated manifest used to remain only in
        // this in-memory cache, leaving persisted shard locations impossible
        // for the custody-facing fold to resolve after a restart. This is a
        // Category-C projection of bytes we just verified and stored, not a
        // new content record or a DHT write.
        if let Some(pool) = self.db_pool.as_ref() {
            let projection_id = format!("blob:{}", manifest.blob_cid);
            let mut conn = pool.get().map_err(|e| {
                StorageError::Internal(format!("Failed to get manifest DB connection: {e}"))
            })?;
            if let Err(e) = crate::db::shard_manifests::record_generated_manifest(
                &mut conn,
                &projection_id,
                "lamad",
                &manifest,
            ) {
                warn!(
                    blob_hash = %manifest.blob_hash,
                    error = %e,
                    "Failed to persist generated shard manifest"
                );
            }
        }

        info!(
            blob_hash = %manifest.blob_hash,
            total_size = manifest.total_size,
            shards = manifest.shard_hashes.len(),
            encoding = %manifest.encoding,
            "Stored blob with manifest"
        );

        // ── Cutover gate #3: dual-write to IrohBlobStore when wired ──────────
        // Best-effort: if the iroh side fails, the SHA256 write already
        // succeeded — return 201 with blake3_hash: null per the pre-flight
        // invariant in the plan ("partial readiness is OK").
        #[cfg(feature = "p2p-iroh")]
        let blake3_hash_str: Option<String> = match self.iroh_blob_store.as_ref() {
            Some(iroh) => match iroh.add_bytes(data.clone()).await {
                Ok(hash) => {
                    tracing::debug!(
                        sha256 = %manifest.blob_hash,
                        blake3 = %hash,
                        "Dual-wrote blob bytes to IrohBlobStore"
                    );
                    // Store with the canonical "blake3-{hex}" prefix so the
                    // GET handler and peer_blob_inventory queries can strip it
                    // consistently (see handle_get_blob line that strips the prefix).
                    Some(format!("blake3-{hash}"))
                }
                Err(e) => {
                    tracing::warn!(
                        sha256 = %manifest.blob_hash,
                        error = %e,
                        "IrohBlobStore dual-write failed; SHA256 store still succeeded"
                    );
                    None
                }
            },
            None => None,
        };
        #[cfg(not(feature = "p2p-iroh"))]
        let blake3_hash_str: Option<String> = None;

        // Stamp self-custody inventory row — best-effort; log on failure.
        if let Some(pool) = self.db_pool.as_ref() {
            let now = chrono::Utc::now().to_rfc3339();
            let sha = manifest.blob_hash.clone();
            let blake3_for_row = blake3_hash_str.clone();
            let peer_id = self.self_peer_id.clone();
            let pool_clone = pool.clone();
            tokio::task::spawn_blocking(move || {
                let mut conn = match pool_clone.get() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "self-custody: db pool checkout failed"
                        );
                        return;
                    }
                };
                if let Err(e) = crate::db::peer_blob_inventory::record_self_custody(
                    &mut conn,
                    &peer_id,
                    &sha,
                    blake3_for_row.as_deref(),
                    &now,
                ) {
                    tracing::warn!(
                        error = %e,
                        sha256 = %sha,
                        "self-custody: inventory stamp failed"
                    );
                }
            });
        }

        let view = crate::views::put_blob_response_view_from_manifest(manifest, blake3_hash_str);
        let body =
            serde_json::to_string(&view).map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok(Response::builder()
            .status(StatusCode::CREATED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap())
    }

    /// Extract agent ID from request headers (set by doorway)
    fn extract_agent_id(req: &Request<Incoming>) -> Option<String> {
        req.headers()
            .get("x-agent-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    /// CORS preflight response for cross-origin requests
    fn cors_preflight() -> Response<Full<Bytes>> {
        Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header(
                "Access-Control-Allow-Methods",
                "GET, PUT, POST, PATCH, DELETE, HEAD, OPTIONS",
            )
            .header(
                "Access-Control-Allow-Headers",
                "Content-Type, Authorization, X-Agent-Id, X-Agent-Cid, X-Schema-Version",
            )
            .header("Access-Control-Max-Age", "86400")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    /// Add CORS headers to a response
    fn with_cors_headers(
        builder: hyper::http::response::Builder,
    ) -> hyper::http::response::Builder {
        builder
            .header("Access-Control-Allow-Origin", "*")
            .header("Cross-Origin-Resource-Policy", "cross-origin")
    }

    /// Build a `text/html` response (used by SSR handlers, ssr feature only).
    #[cfg(feature = "ssr")]
    fn html_response(html: String) -> Response<Full<Bytes>> {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Full::new(Bytes::from(html)))
            .unwrap()
    }

    /// `GET /chrome/*` — runtime-served native chrome static assets.
    ///
    /// The Tauri desktop shell talks to this storage sidecar directly (no
    /// doorway), so the sidecar must serve the omnibar element the same way the
    /// doorway does (`doorway/.../routes/chrome.rs`). The element bytes/hash/path
    /// come from the light, V8-free `elohim-chrome-asset` crate — so this route
    /// is in the DEFAULT storage build (it does NOT need the `ssr` feature).
    ///
    /// Two known paths:
    /// - `GET /chrome/omni-element.{sha256}.js` — the content-addressed element,
    ///   served immutable (a changed script ⇒ a new hash ⇒ a new path).
    /// - `GET /chrome/omni-element.js` — a STABLE alias for the CURRENT element,
    ///   for static `index.html` refs (Tauri) that cannot embed a content hash.
    ///   NOT immutable: its bytes change with the element, so it is served with a
    ///   revalidation-friendly cache policy.
    ///
    /// Any other `/chrome/*` path (including a stale/mismatched hash) → 404.
    fn handle_chrome_asset(path: &str) -> Response<Full<Bytes>> {
        if path == elohim_chrome_asset::element_script_path() {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/javascript; charset=utf-8")
                // Content-addressed ⇒ immutable: a changed script changes the
                // hash (and the path), so the bytes at THIS path never change.
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .body(Full::new(Bytes::from_static(
                    elohim_chrome_asset::element_js_bytes(),
                )))
                .unwrap();
        }

        if path == elohim_chrome_asset::STABLE_ELEMENT_PATH {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/javascript; charset=utf-8")
                // The stable alias serves the CURRENT element, which changes over
                // time — so revalidate rather than cache immutably. An ETag lets a
                // client skip the body when the content address is unchanged.
                .header(header::CACHE_CONTROL, "public, max-age=0, must-revalidate")
                .header(
                    header::ETAG,
                    format!("\"{}\"", elohim_chrome_asset::element_js_hash()),
                )
                .body(Full::new(Bytes::from_static(
                    elohim_chrome_asset::element_js_bytes(),
                )))
                .unwrap();
        }

        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(Full::new(Bytes::from_static(b"chrome asset not found")))
            .unwrap()
    }

    /// Resolve the shard manifest for a canonical `sha256-{hex}` blob hash.
    ///
    /// The in-process `manifests` map only ever holds what THIS process minted:
    /// a restart, or a peer whose shards arrived by replication rather than by
    /// ingest, has no entry even though every shard sits on local disk. Since
    /// `put_blob_bytes` never stores the composite under its own name, a
    /// `>16MB` blob was unservable everywhere except its minting process.
    ///
    /// The durable projection (`shard_manifests`) is the fallback; a hit
    /// hydrates the map so subsequent reads stay warm. Lazy — not a boot
    /// reload — because the manifest is a Category-C projection: paying for it
    /// on the first read of a given blob is cheaper and correct for blobs whose
    /// rows land after startup (replication, backfill).
    async fn resolve_manifest(&self, hash: &str) -> Option<ShardManifest> {
        if let Some(manifest) = self.manifests.read().await.get(hash).cloned() {
            return Some(manifest);
        }

        let pool = self.db_pool.as_ref()?;
        let mut conn = pool.get().ok()?;
        let row = crate::db::shard_manifests::get_manifest_by_blob_hash(&mut conn, hash)
            .map_err(|e| warn!(hash = %hash, error = %e, "shard manifest lookup failed"))
            .ok()
            .flatten()?;
        let manifest = crate::db::shard_manifests::hydrate_manifest(&row)
            .map_err(|e| warn!(hash = %hash, error = %e, "shard manifest hydration failed"))
            .ok()?;

        debug!(
            hash = %hash,
            encoding = %manifest.encoding,
            shards = manifest.shard_hashes.len(),
            "shard manifest rehydrated from durable projection"
        );
        self.manifests
            .write()
            .await
            .insert(hash.to_string(), manifest.clone());
        Some(manifest)
    }

    /// Reassemble a manifest's blob from locally-stored shards.
    ///
    /// On a local miss returns the offending `(index, shard_hash)` so callers
    /// can name which shard is absent instead of reporting the composite as
    /// simply "not found".
    async fn reassemble_from_local_shards(
        &self,
        manifest: &ShardManifest,
    ) -> Result<Vec<u8>, (usize, String)> {
        let mut data = Vec::with_capacity(manifest.total_size as usize);

        for (index, shard_hash) in manifest.shard_hashes.iter().enumerate() {
            match self.blob_store.get(shard_hash).await {
                Ok(shard_data) => data.extend_from_slice(&shard_data),
                Err(_) => return Err((index, shard_hash.clone())),
            }
        }

        // Truncate to actual size (last shard may be padded)
        data.truncate(manifest.total_size as usize);
        Ok(data)
    }

    /// Read a blob from local storage, whatever shape it is held in.
    ///
    /// This is the single local-read seam behind `GET /blob/{hash}` and the
    /// `/apps/{slug}` resolver (via `get_blob_or_heal`). Both shapes are tried:
    /// the manifest's shards first (the shape `put_blob_bytes` leaves for
    /// anything over `SINGLE_SHARD_MAX`, and the one that carries the recorded
    /// mime type), then the composite under its own name (a small blob, or a
    /// `>16MB` composite reassembled from peers and persisted by
    /// `finalize_fetch_success`).
    ///
    /// Trying only one shape is exactly how the two routes came to disagree
    /// about the same bytes: the `/blob` route resolved a manifest and 404'd on
    /// a shard gap without ever reading the composite it held whole, while the
    /// apps resolver read the composite and served it.
    ///
    /// `hash` MUST already be canonical `sha256-{hex}` (both callers normalize
    /// first) — `resolve_manifest` and `peer_blob_inventory` are keyed on it.
    async fn read_local_blob(&self, hash: &str) -> LocalBlobRead {
        let manifest = self.resolve_manifest(hash).await.map(Box::new);

        // `"none"` manifests name the composite itself, so reassembling one is
        // just the direct read below with an extra copy.
        let reassembled = match manifest.as_ref() {
            Some(m) if m.encoding != "none" => Some(self.reassemble_from_local_shards(m).await),
            _ => None,
        };

        let gap = match reassembled {
            Some(Ok(bytes)) => {
                debug!(
                    hash = %hash,
                    size = bytes.len(),
                    "blob read locally by reassembling manifest shards"
                );
                return LocalBlobRead::Bytes { bytes, manifest };
            }
            Some(Err(gap)) => Some(gap),
            None => None,
        };

        if let Ok(bytes) = self.blob_store.get(hash).await {
            return LocalBlobRead::Bytes { bytes, manifest };
        }

        // A gap can only come from a manifest, so the two are always both
        // present here; the `match` keeps that provable rather than asserted.
        match (gap, manifest) {
            (Some((index, shard_hash)), Some(manifest)) => LocalBlobRead::MissingShard {
                manifest,
                index,
                shard_hash,
            },
            _ => LocalBlobRead::Absent,
        }
    }

    /// Do we hold this blob locally, without reading it?
    ///
    /// The presence twin of [`Self::read_local_blob`]: metadata probes only, so
    /// a caller that just needs "yes/no" does not pull an 18MB bundle into
    /// memory. Same two shapes, same answer — a raw `exists_by_address` probe
    /// on a sharded composite reports absence for bytes we fully hold, which
    /// would silently exclude every oversized bundle from the declared-head
    /// serve preference.
    async fn blob_available_locally(&self, address: &str) -> bool {
        let hash = match crate::blob_store::BlobStore::parse_content_address(address) {
            Ok(hex) => format!("sha256-{hex}"),
            Err(_) => return false,
        };

        if self.blob_store.exists(&hash).await {
            return true;
        }

        match self.resolve_manifest(&hash).await {
            Some(manifest) if manifest.encoding != "none" => {
                if manifest.shard_hashes.is_empty() {
                    return false;
                }
                for shard_hash in &manifest.shard_hashes {
                    if !self.blob_store.exists(shard_hash).await {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }

    /// GET /blob/{hash} - Reassemble blob from shards
    /// Checks policy enforcement if agent_id is provided
    async fn handle_get_blob(
        &self,
        hash: &str,
        agent_id: Option<&str>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing blob hash")))
                .unwrap());
        }

        // Cutover gate #2 (Phase 11, Plan 2): try iroh-side blob backend first
        // when the caller is iroh-capable AND the blob has a known BLAKE3 alias.
        // Falls through to the legacy SHA256 path on miss, libp2p-only caller,
        // or no IrohBlobStore wired.
        #[cfg(feature = "p2p-iroh")]
        if let Some(ref iroh) = self.iroh_blob_store {
            // Normalize to the router's expected form: sha256-{hex} or blake3-{hex}.
            let normalized_for_router: String = if hash.starts_with("blake3-") {
                hash.to_string()
            } else {
                match crate::blob_store::BlobStore::parse_content_address(hash) {
                    Ok(h) => format!("sha256-{}", h),
                    Err(_) => String::new(), // fall through to legacy parser below
                }
            };

            if !normalized_for_router.is_empty() {
                let mut conn_opt = self.db_pool.as_ref().and_then(|p| p.get().ok());
                let blake3_alias = if !normalized_for_router.starts_with("blake3-") {
                    conn_opt.as_mut().and_then(|c| {
                        crate::db::peer_blob_inventory::lookup_blake3_for_sha256(
                            c,
                            &normalized_for_router,
                        )
                        .ok()
                        .flatten()
                    })
                } else {
                    None
                };
                let sha256_alias = if normalized_for_router.starts_with("blake3-") {
                    conn_opt.as_mut().and_then(|c| {
                        crate::db::peer_blob_inventory::lookup_sha256_for_blake3(
                            c,
                            &normalized_for_router,
                        )
                        .ok()
                        .flatten()
                    })
                } else {
                    None
                };

                let caller_manifest = match (agent_id, conn_opt.as_mut()) {
                    (Some(cid), Some(c)) => crate::p2p_iroh::peer_map::lookup_by_agent_cid(c, cid)
                        .ok()
                        .flatten(),
                    _ => None,
                };

                // Per-object transport affinity (Category C). NULL → Auto
                // (today's behavior). Read from the same inventory row.
                let affinity = conn_opt
                    .as_mut()
                    .and_then(|c| {
                        crate::db::peer_blob_inventory::lookup_transport_affinity(
                            c,
                            &normalized_for_router,
                        )
                        .ok()
                        .flatten()
                    })
                    .as_deref()
                    .map(crate::http_blob_router::TransportAffinity::parse)
                    .unwrap_or_default();

                let choice = crate::http_blob_router::choose_backend(
                    crate::http_blob_router::ChooseInputs {
                        normalized_hash: &normalized_for_router,
                        blake3_alias_for_sha256: blake3_alias,
                        sha256_alias_for_blake3: sha256_alias,
                        self_manifest: self.self_transport_manifest.as_deref(),
                        caller_manifest: caller_manifest.as_ref(),
                        affinity,
                    },
                );

                if let crate::http_blob_router::BlobBackendChoice::IrohThenLibp2p {
                    blake3_hash,
                    ..
                } = choice
                {
                    let blake3_hex = blake3_hash.strip_prefix("blake3-").unwrap_or(&blake3_hash);
                    if let Ok(iroh_hash) = blake3_hex.parse::<iroh_blobs::Hash>() {
                        match iroh.get_bytes(iroh_hash).await {
                            Ok(data) => {
                                self.blob_iroh_served_count
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                debug!(
                                    hash = %hash,
                                    backend = "iroh",
                                    size = data.len(),
                                    "blob served from iroh backend (cutover gate #2)"
                                );
                                return Ok(Self::with_cors_headers(Response::builder())
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/octet-stream")
                                    .header(header::CONTENT_LENGTH, data.len())
                                    .header(
                                        header::CACHE_CONTROL,
                                        "public, max-age=31536000, immutable",
                                    )
                                    .body(Full::new(Bytes::from(data.to_vec())))
                                    .unwrap());
                            }
                            Err(e) => {
                                debug!(
                                    hash = %hash,
                                    error = %e,
                                    "iroh-side miss; falling through to legacy backend"
                                );
                            }
                        }
                    }
                }
            }
        }

        // Parse content address (accepts CID, sha256-prefixed hash, or raw hex)
        let normalized_hash = match crate::blob_store::BlobStore::parse_content_address(hash) {
            Ok(h) => format!("sha256-{}", h),
            Err(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Full::new(Bytes::from(format!(
                        "Invalid content address: {}",
                        hash
                    ))))
                    .unwrap());
            }
        };
        let hash = normalized_hash.as_str();

        // Check policy enforcement if enabled and agent_id is provided
        if let (Some(ref enforcement), Some(agent)) = (&self.policy_enforcement, agent_id) {
            // Create content metadata for policy check
            // For now, we only have the hash - in future, we could look up metadata
            let content = ContentMetadata {
                hash: hash.to_string(),
                categories: Vec::new(), // TODO: Could be looked up from db_pool
                age_rating: None,
                reach_level: None,
            };

            match enforcement.can_serve(agent, &content) {
                Ok(PolicyDecision::Allow) => {
                    debug!(hash = %hash, agent = %agent, "Policy check passed");
                }
                Ok(PolicyDecision::Block { reason }) => {
                    warn!(hash = %hash, agent = %agent, reason = %reason, "Content blocked by policy");

                    // Log the policy event
                    let _ = enforcement.cache().log_event(
                        agent,
                        None,
                        &PolicyEvent {
                            event_type: PolicyEventType::BlockedContent,
                            details: reason.clone(),
                            content_hash: Some(hash.to_string()),
                            feature_name: None,
                        },
                        30, // Default retention days
                    );

                    return Ok(Response::builder()
                        .status(StatusCode::FORBIDDEN)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "blocked", "reason": "{}"}}"#,
                            reason
                        ))))
                        .unwrap());
                }
                Err(e) => {
                    // Policy check failed, but we shouldn't block content - log and proceed
                    warn!(error = %e, "Policy check failed, allowing access");
                }
            }
        }

        // One local read for both shapes a held blob can take (manifest shards
        // or the composite under its own name) — the same seam `/apps` reads
        // through, so the two routes cannot disagree about the same bytes.
        //
        // The manifest is kept past this point: when the local read comes up
        // short and a peer supplies the bytes instead, those bytes are the ones
        // THIS manifest describes, so its mime type still governs the response.
        let (local_gap, manifest_ctx) = match self.read_local_blob(hash).await {
            LocalBlobRead::Bytes { bytes, manifest } => {
                info!(
                    hash = %hash,
                    size = bytes.len(),
                    shards = manifest.as_ref().map(|m| m.shard_hashes.len()).unwrap_or(1),
                    "Serving reassembled blob"
                );
                debug!(hash = %hash, backend = "libp2p", "blob served from legacy backend (local)");
                return Ok(self.blob_ok_response(hash, bytes, manifest.as_deref()));
            }
            // A resolved manifest whose shards are absent is NOT a terminal
            // answer: the bytes may still be reachable from a peer. Falling
            // straight to 404 here (as the pre-unification manifest arm did)
            // silently retired peer-heal for every blob whose manifest row
            // outlives its bytes — a restart, or a manifest that arrived by
            // projection ahead of the shards. Keep the gap so the eventual 404
            // still names the missing shard.
            LocalBlobRead::MissingShard {
                manifest,
                index,
                shard_hash,
            } => {
                warn!(
                    hash = %hash,
                    shard_index = index,
                    shard_hash = %shard_hash,
                    shards = manifest.shard_hashes.len(),
                    "blob heal: manifest resolved but shard missing locally"
                );
                (Some((index, shard_hash)), Some(manifest))
            }
            LocalBlobRead::Absent => (None, None),
        };

        // Local miss — heal from peers (T17). `heal_from_peers` is the same
        // race-fetch + finalize leg `get_blob_or_heal` runs; calling it directly
        // keeps the local read we already performed from happening twice.
        let outcome = self.heal_from_peers(hash).await;
        Ok(self.blob_response_for_heal(hash, outcome, local_gap, manifest_ctx.as_deref()))
    }

    /// The 200 response for blob bytes — one builder for every arm that serves
    /// them, so a locally-read blob and a peer-healed one cannot describe the
    /// same bytes differently.
    ///
    /// With a manifest the recorded mime type and an ETag are reported; without
    /// one (bytes we hold but have no manifest row for) it degrades to
    /// `application/octet-stream`, the historical shape for that case.
    fn blob_ok_response(
        &self,
        hash: &str,
        bytes: Vec<u8>,
        manifest: Option<&ShardManifest>,
    ) -> Response<Full<Bytes>> {
        self.blob_libp2p_served_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let builder = Self::with_cors_headers(Response::builder())
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, bytes.len())
            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
        let builder = match manifest {
            Some(m) => builder
                .header(header::CONTENT_TYPE, &m.mime_type)
                .header(header::ETAG, format!("\"{}\"", hash)),
            None => builder.header(header::CONTENT_TYPE, "application/octet-stream"),
        };
        builder.body(Full::new(Bytes::from(bytes))).unwrap()
    }

    /// Map a peer-heal outcome onto the `/blob` route's response shape.
    ///
    /// `local_gap` and `manifest` carry forward what the local read already
    /// learned: the gap names the missing shard in a terminal 404, and the
    /// manifest supplies the mime type/ETag for bytes a peer just supplied —
    /// the arm this diff's shard-gap→peer-heal fall-through newly reaches.
    fn blob_response_for_heal(
        &self,
        hash: &str,
        outcome: BlobHealOutcome,
        local_gap: Option<(usize, String)>,
        manifest: Option<&ShardManifest>,
    ) -> Response<Full<Bytes>> {
        match outcome {
            BlobHealOutcome::Bytes { bytes, healed_from } => {
                if healed_from.is_some() {
                    debug!(hash = %hash, backend = "libp2p", "blob served from legacy backend (race-fetch)");
                } else {
                    debug!(hash = %hash, backend = "libp2p", "blob served from legacy backend (direct)");
                }
                // The manifest the local read resolved describes exactly these
                // bytes, so a healed composite reports the same mime type and
                // ETag a locally-read one does. Without it, a `>16MB` bundle
                // served as opaque octets purely because it arrived by heal.
                self.blob_ok_response(hash, bytes, manifest)
            }
            BlobHealOutcome::FinalizeFailed(FinalizeFailure::PoolExhausted) => {
                Self::with_cors_headers(Response::builder())
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Full::new(Bytes::from("Storage unavailable; retry")))
                    .unwrap()
            }
            BlobHealOutcome::FinalizeFailed(FinalizeFailure::Persist) => {
                Self::with_cors_headers(Response::builder())
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Full::new(Bytes::from(
                        "Blob fetched from peer but local persist failed; retry",
                    )))
                    .unwrap()
            }
            BlobHealOutcome::Syncing { started } => {
                // Fix B: the peer-heal exceeded its in-request budget; the
                // fetch continues in the background. Degrade with a
                // syncing 503 + Retry-After (distinct JSON body from the
                // FinalizeFailed plain-text 503s above) instead of
                // blocking the request open for the full cross-peer race.
                debug!(hash = %hash, background_fetch = started, "Fix B: heal budget expired — returning syncing 503");
                Self::with_cors_headers(Response::builder())
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .header(header::RETRY_AFTER, "5")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(syncing_blob_body(hash))))
                    .unwrap()
            }
            // Nothing local, nothing from peers. Naming the absent shard keeps
            // a partially-replicated composite distinguishable from an unknown
            // blob — the peer race has now been tried either way.
            BlobHealOutcome::NotFound => match local_gap {
                Some((index, shard_hash)) => {
                    let total_shards = manifest.map(|m| m.shard_hashes.len()).unwrap_or(0);
                    warn!(
                        hash = %hash,
                        shard_index = index,
                        shard_hash = %shard_hash,
                        shards = total_shards,
                        "Blob manifest resolved but a shard is missing locally and no peer served it"
                    );
                    Self::with_cors_headers(Response::builder())
                        .status(StatusCode::NOT_FOUND)
                        .body(Full::new(Bytes::from(format!(
                            "Blob incomplete: shard {} of {} ({}) missing",
                            index, total_shards, shard_hash
                        ))))
                        .unwrap()
                }
                None => {
                    debug!(hash = %hash, "T17: race-fetch miss — returning 404");
                    Self::with_cors_headers(Response::builder())
                        .status(StatusCode::NOT_FOUND)
                        .body(Full::new(Bytes::from("Blob not found")))
                        .unwrap()
                }
            },
        }
    }

    /// Read a blob locally, healing from peers on a local miss (T17).
    ///
    /// This is the shared core extracted from `GET /blob/{hash}`'s
    /// direct-blob-lookup arm: on a local hit it returns the bytes; on any
    /// local-read error (a miss) it consults `peer_blob_inventory`, races a
    /// fetch across connected hosting peers (`crate::p2p::blob_fetch::race_fetch`),
    /// and on a verified hit persists + records via `finalize_fetch_success`
    /// (which also books the `serve-blob` REA event).
    ///
    /// `hash` MUST already be in canonical `sha256-{hex}` form (the form the
    /// `/blob` route normalizes to, and the form `peer_blob_inventory` and
    /// `verify_blob_hash` expect). The `#[cfg(feature = "p2p")]` gating keeps
    /// non-p2p builds at exactly the prior behaviour: a local miss yields
    /// `NotFound` with no peer attempt.
    ///
    /// The returned `BlobHealOutcome` carries the heal/503/404 distinction so
    /// each caller maps it onto its own response shape (the `/blob` route keeps
    /// its distinct 503 bodies and metrics; the apps resolver continues into
    /// ZIP extraction on `Bytes`). The helper is infallible — every local-read
    /// or heal failure resolves to a `BlobHealOutcome` variant — so callers map
    /// it directly rather than propagating a `StorageError`.
    async fn get_blob_or_heal(&self, hash: &str) -> BlobHealOutcome {
        // Local hit — no heal needed.
        //
        // Any local-read error (NotFound, a corrupt-chunk HashMismatch, or an
        // IO error) is treated as a miss and falls through to the peer-heal
        // attempt below. This matches the pre-extraction `/blob` handler's
        // `Err(_)` catch-all exactly (preserving its observable behavior) and
        // is the more resilient choice for the apps resolver: a locally corrupt
        // ZIP blob re-heals from a peer rather than 404ing.
        // Sharded composites are never stored under their own name (see
        // `put_blob_bytes`), so a bare `blob_store` read ALWAYS misses for
        // `encoding != "none"`. `read_local_blob` is the shared seam that tries
        // both shapes — manifest shards and the composite itself — and is what
        // makes `/apps/{slug}` (and every other `get_blob_or_heal` caller) able
        // to serve a `>16MB` blob at all: previously a 404 by construction.
        match self.read_local_blob(hash).await {
            LocalBlobRead::Bytes { bytes, .. } => {
                return BlobHealOutcome::Bytes {
                    bytes,
                    healed_from: None,
                };
            }
            LocalBlobRead::MissingShard {
                manifest,
                index,
                shard_hash,
            } => {
                // Shards are ordinary content-addressed blobs, so the peer-heal
                // below still has a path — it just races on the composite hash.
                // Naming the gap keeps the eventual 404 diagnosable.
                warn!(
                    hash = %hash,
                    shard_index = index,
                    shard_hash = %shard_hash,
                    shards = manifest.shard_hashes.len(),
                    "blob heal: manifest resolved but shard missing locally"
                );
            }
            LocalBlobRead::Absent => {
                // Local miss — fall through to the peer-heal attempt below.
            }
        }

        self.heal_from_peers(hash).await
    }

    /// The peer-heal leg on its own: inventory → race-fetch → finalize.
    ///
    /// Split out of [`Self::get_blob_or_heal`] so a caller that has ALREADY
    /// performed the local read (the `/blob` route, which needs the manifest
    /// context the read produced) can race peers without repeating it.
    /// `get_blob_or_heal` remains the local-read + heal composition every other
    /// caller uses.
    ///
    /// `hash` MUST already be canonical `sha256-{hex}` — `peer_blob_inventory`
    /// and `verify_blob_hash` are keyed on that form.
    async fn heal_from_peers(&self, hash: &str) -> BlobHealOutcome {
        // Attempt peer fallback. T17: race-fetch helper.
        // Requires P2P feature + db pool; otherwise we degrade to NotFound,
        // exactly as the pre-refactor handler did.
        #[cfg(feature = "p2p")]
        if let (Some(ref handle), Some(ref pool)) = (&self.p2p_handle, &self.db_pool) {
            // Short hash prefix for log correlation across legs without
            // dumping the full 64-char hex on every line.
            let hash_prefix: String = hash.chars().take(20).collect();

            // Inventory-first: consult the peer_blob_inventory projection for
            // peers known to host this blob within the configured freshness
            // window. Driven by `Config::inventory_freshness_seconds` (was a
            // hardcoded 600s) so the same operator knob governs heal and the
            // reconciler's staleness threshold.
            let fresh_after = (chrono::Utc::now()
                - chrono::Duration::seconds(self.inventory_freshness_seconds as i64))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
            let inventory_candidates = pool
                .get()
                .ok()
                .and_then(|mut conn| {
                    crate::db::peer_blob_inventory::lookup_hosts(&mut conn, hash, &fresh_after).ok()
                })
                .map(|rows| rows.into_iter().map(|r| r.peer_id).collect::<Vec<_>>())
                .unwrap_or_default();

            // Wave-3 serve-routing: re-order `inventory_candidates` by capability/headroom/RTT/bond/delivery/diversity.
            //
            // `select_serve_peers` returns score-ordered agent_cids from the `shard_locations`
            // agent_cid-native source.  Each agent_cid is then resolved to a libp2p peer id via
            // `peer_transport_manifest` (the agent_cid → libp2p_peer_id direction IS available).
            // Resolvable entries form a score-ordered prefix; the original `inventory_candidates`
            // (libp2p-keyed, unordered) are appended deduped as the fallback tail.
            //
            // Partial-coverage caveat: in prod `peer_transport_manifest.libp2p` is NULL
            // for most entries (the column has only `#[cfg(test)]` writers — the population gap
            // is tracked in genesis/data/timeline/backlog/).  An empty manifest means:
            //   score_ordered_prefix = [] → inventory_candidates unchanged = today's behavior.
            // This is the correct safe degradation — no availability regression.
            let inventory_candidates = {
                // Resolution requires peer_transport_manifest (iroh feature).
                // Without it the prefix is empty → inventory unchanged (safe degradation).
                #[cfg(feature = "p2p-iroh")]
                let score_ordered_libp2p: Vec<String> = pool
                    .get()
                    .ok()
                    .map(|mut conn| {
                        // select_serve_peers: load shard_locations → fold → select_diverse.
                        // Returns agent_cids ordered by score (best first).
                        let agent_cids = crate::services::serve_routing::select_serve_peers(
                            &mut conn,
                            hash,
                            inventory_candidates.len().max(8),
                        )
                        .unwrap_or_default();

                        // Resolve each agent_cid → libp2p_peer_id via peer_transport_manifest.
                        // Only peers with a populated libp2p_peer_id make it to the prefix.
                        agent_cids
                            .into_iter()
                            .filter_map(|cid| {
                                crate::p2p_iroh::peer_map::lookup_by_agent_cid(&mut conn, &cid)
                                    .ok()
                                    .flatten()
                                    .and_then(|m| m.libp2p.map(|l| l.peer_id))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                #[cfg(not(feature = "p2p-iroh"))]
                let score_ordered_libp2p: Vec<String> = Vec::new();

                if score_ordered_libp2p.is_empty() {
                    // Manifest unpopulated (expected in prod this wave) — preserve today's list.
                    inventory_candidates
                } else {
                    // Score-ordered prefix + original list (deduped). Peers already in the
                    // prefix are not re-added from the tail, preserving score ordering.
                    let prefix_set: std::collections::HashSet<&str> =
                        score_ordered_libp2p.iter().map(String::as_str).collect();
                    let tail: Vec<String> = inventory_candidates
                        .into_iter()
                        .filter(|p| !prefix_set.contains(p.as_str()))
                        .collect();
                    let mut merged = score_ordered_libp2p;
                    merged.extend(tail);
                    merged
                }
            };

            // Snapshot the connected-peer set via ListPeers command once;
            // used both as the race_fetch membership filter and (on empty
            // inventory) as the connected-fallback candidate set.
            let connected_peers: Vec<String> = handle
                .list_peers()
                .await
                .into_iter()
                .map(|p| p.peer_id)
                .collect();

            // Connected-fallback: when gossipsub inventory publish has failed
            // (alpha after pod restarts: `InsufficientPeers`, no fresh row for
            // a peer that is nonetheless connected and advertising the blob),
            // the inventory candidate set comes up empty even though a direct
            // fetch would succeed. Rather than 404 on an empty set, race the
            // currently-connected peers directly. Inventory hits stay strictly
            // preferred; this fallback fires ONLY when inventory is empty.
            // Bounded to 8 peers so a large mesh can't fan out unboundedly;
            // the per-peer timeout still bounds total time for a genuine miss.
            const CONNECTED_FALLBACK_CAP: usize = 8;
            let (candidates, candidate_source) = if !inventory_candidates.is_empty() {
                (inventory_candidates, "inventory")
            } else if !connected_peers.is_empty() {
                let mut capped = connected_peers.clone();
                capped.truncate(CONNECTED_FALLBACK_CAP);
                (capped, "connected-fallback")
            } else {
                // Distinct label: a structured `source=connected-fallback` next
                // to "no connected peers" reads as the fallback having fired.
                (Vec::new(), "no-peers")
            };

            if !candidates.is_empty() {
                let cmd_tx = handle.command_sender();
                let parallelism = self.fetch_blob_parallelism;
                let per_peer_timeout =
                    std::time::Duration::from_secs(self.fetch_blob_timeout_seconds);
                let candidate_count = candidates.len();
                info!(
                    hash_prefix = %hash_prefix,
                    candidate_count,
                    source = candidate_source,
                    "blob heal: racing peers for local miss"
                );
                // race_fetch filters candidates to those connected at call
                // time; the closure checks membership in the snapshot above.
                let connected_set: std::collections::HashSet<String> =
                    connected_peers.into_iter().collect();

                // Fix B: run the race-fetch + finalize as a detached task and
                // bound only the time THIS request waits on it. The heal can run
                // for the full cross-peer race (observed >30s live) when a healed
                // content pointer references bytes not yet replicated locally;
                // holding the HTTP request open that long stacks every landing
                // request under byte-lag. On budget expiry we return `Syncing`
                // and let the task keep running (dropping its JoinHandle does NOT
                // abort it — see blob_fetch::race_fetch), so the heal still
                // converges for a later request. Captures are cloned so the task
                // outlives the borrow of `self`.
                let hash_owned = hash.to_string();
                let hash_prefix_task = hash_prefix.clone();
                let pool_bg = pool.clone();
                let blob_store_bg = self.blob_store.clone();
                let self_cid_bg = self.self_cid.clone();
                let fresh_after_task = fresh_after.clone();
                let heal_task = tokio::spawn(async move {
                    // Q3/Q4 (minutes-quiesce W1.2/W1.3): this is the exact
                    // cross-process 404 class Q2 closed LOCALLY (a sharded
                    // composite's bytes exist only under its shard hashes,
                    // never under the composite hash itself) — here it is
                    // cross-PEER: a connected peer may hold every shard
                    // without holding the composite's manifest cache. Plain
                    // `race_fetch` on the composite hash would 404 forever in
                    // that case; `race_fetch_with_swarm` accepts a
                    // manifest-only reply, persists it, and spreads the shard
                    // fetch across the known holder set instead of dead-ending.
                    let mut conn = match pool_bg.get() {
                        Ok(c) => c,
                        Err(e) => {
                            error!(
                                hash = %hash_owned,
                                error = %e,
                                "T20/Q3: pool exhausted, cannot race-fetch"
                            );
                            return BlobHealOutcome::FinalizeFailed(FinalizeFailure::PoolExhausted);
                        }
                    };
                    let swarm_params = crate::p2p::blob_swarm::SwarmFetchParams {
                        cmd_tx: &cmd_tx,
                        connected: &connected_set,
                        per_shard_parallelism: parallelism,
                        per_peer_timeout,
                        // Modest total-inflight bound on CONCURRENT shard
                        // races (distinct from `parallelism`, which bounds
                        // peers raced WITHIN one shard) — mirrors the
                        // `fetch_blob_parallelism * ~4` sizing
                        // `RaceFetchKicker::kick_semaphore` already uses
                        // elsewhere for a similar fan-out bound.
                        total_inflight: parallelism.max(1) * 4,
                        self_cid: &self_cid_bg,
                        blob_store: &blob_store_bg,
                    };
                    let outcome = crate::p2p::blob_swarm::race_fetch_with_swarm(
                        &hash_owned,
                        candidates,
                        &mut conn,
                        &fresh_after_task,
                        &swarm_params,
                    )
                    .await;
                    drop(conn);

                    match outcome {
                        crate::p2p::blob_swarm::SwarmRaceOutcome::Hit { bytes, source_peer } => {
                            // T20: persist + SQL collapse into a single async
                            // `finalize_fetch_success` call.
                            //
                            // Ordering inside finalize: filesystem persist FIRST,
                            // then `record_fetch_success`, then `serve-blob` REA
                            // event (the latter two atomic via a single
                            // `conn.transaction`). The FinalizeFailed legs below
                            // cover all three failure cases: pool exhaustion
                            // (cannot acquire conn), filesystem write failure (no
                            // SQL written), or SQL failure inside the wrapping
                            // transaction (rolled back by Diesel; orphan blob on
                            // disk reconciled by the T18 parity sweep). Either way
                            // the blob is not yet durably accounted for through
                            // this gateway, and callers return 503 (not 404) to
                            // preserve retry semantics on the downstream cache.
                            //
                            // `source_peer == "swarm"` names a Q4 reassembly:
                            // no single peer served the whole composite (each
                            // shard was already persisted + REA-booked
                            // individually inside `fetch_shards_via_swarm`),
                            // so finalizing again here would falsely claim a
                            // peer literally named "swarm" served every byte.
                            // The composite is already durable via its
                            // manifest + shards (Q2's philosophy: never store
                            // the composite a second time under its own
                            // name) — just serve the bytes.
                            if source_peer == "swarm" {
                                info!(
                                    hash_prefix = %hash_prefix_task,
                                    size = bytes.len(),
                                    candidate_count,
                                    source = candidate_source,
                                    "blob heal: Q4 swarm shard-fetch HIT — composite reassembled from shards"
                                );
                                return BlobHealOutcome::Bytes {
                                    bytes,
                                    healed_from: Some(source_peer),
                                };
                            }

                            let mut conn = match pool_bg.get() {
                                Ok(c) => c,
                                Err(e) => {
                                    error!(
                                        hash = %hash_owned,
                                        error = %e,
                                        "T20: pool exhausted, cannot finalize fetch"
                                    );
                                    return BlobHealOutcome::FinalizeFailed(
                                        FinalizeFailure::PoolExhausted,
                                    );
                                }
                            };
                            if let Err(e) = crate::p2p::blob_fetch::finalize_fetch_success(
                                &mut conn,
                                &hash_owned,
                                &source_peer,
                                &bytes,
                                &self_cid_bg,
                                &blob_store_bg,
                            )
                            .await
                            {
                                error!(
                                    hash = %hash_owned,
                                    source_peer = %source_peer,
                                    error = %e,
                                    "T20: finalize_fetch_success failed; returning 503"
                                );
                                return BlobHealOutcome::FinalizeFailed(FinalizeFailure::Persist);
                            }

                            info!(
                                hash_prefix = %hash_prefix_task,
                                source_peer = %source_peer,
                                size = bytes.len(),
                                candidate_count,
                                source = candidate_source,
                                "blob heal: race-fetch HIT — blob persisted and recorded"
                            );
                            BlobHealOutcome::Bytes {
                                bytes,
                                healed_from: Some(source_peer),
                            }
                        }
                        // Q3: a manifest was received and persisted, but at
                        // least one shard could not be swarm-fetched this
                        // round. The manifest is durable regardless — a
                        // later request (this same path again, replication
                        // landing the missing shard, or Q2's local
                        // reassembly) can complete it without repeating the
                        // manifest round-trip. 404 for THIS request; do not
                        // conflate with a plain Miss in the log line.
                        crate::p2p::blob_swarm::SwarmRaceOutcome::ManifestPersistedIncomplete {
                            manifest,
                            missing_shards,
                        } => {
                            info!(
                                hash_prefix = %hash_prefix_task,
                                total_shards = manifest.shard_hashes.len(),
                                missing_shards,
                                candidate_count,
                                source = candidate_source,
                                "blob heal: Q3/Q4 manifest persisted but swarm shard-fetch incomplete; returning 404"
                            );
                            BlobHealOutcome::NotFound
                        }
                        crate::p2p::blob_swarm::SwarmRaceOutcome::Miss => {
                            info!(
                                hash_prefix = %hash_prefix_task,
                                candidate_count,
                                source = candidate_source,
                                "blob heal: race-fetch ALL-MISSES — no peer served verified bytes; returning 404"
                            );
                            BlobHealOutcome::NotFound
                        }
                        crate::p2p::blob_swarm::SwarmRaceOutcome::NoCandidates => {
                            // candidate_count > 0 here, but none were connected at
                            // race time (the inventory rows pointed at peers that
                            // have since disconnected). Distinct from the empty-set
                            // leg below so a 404 names the failing reason precisely.
                            info!(
                                hash_prefix = %hash_prefix_task,
                                candidate_count,
                                source = candidate_source,
                                "blob heal: race-fetch NO-CANDIDATES — candidates known but none connected; returning 404"
                            );
                            BlobHealOutcome::NotFound
                        }
                    }
                });

                // Bound only the in-request wait; the task's own per-peer timeout
                // still bounds a genuine miss. On budget expiry the JoinHandle is
                // dropped (which does NOT abort the task) and we degrade to
                // Syncing while heal converges in the background.
                let budget = std::time::Duration::from_millis(self.heal_on_read_budget_ms);
                match tokio::time::timeout(budget, heal_task).await {
                    Ok(Ok(outcome)) => return outcome,
                    Ok(Err(join_err)) => {
                        // Task panicked — surface as a heal miss (404) rather than
                        // masking a bug behind a syncing status.
                        error!(
                            hash_prefix = %hash_prefix,
                            error = %join_err,
                            "blob heal: background heal task panicked; returning 404"
                        );
                    }
                    Err(_elapsed) => {
                        info!(
                            hash_prefix = %hash_prefix,
                            budget_ms = self.heal_on_read_budget_ms,
                            source = candidate_source,
                            "blob heal: in-request budget expired — fetch continues in background; returning syncing"
                        );
                        return BlobHealOutcome::Syncing { started: true };
                    }
                }
            } else {
                info!(
                    hash_prefix = %hash_prefix,
                    source = candidate_source,
                    "blob heal: NO-CANDIDATES — empty inventory and no connected peers; returning 404"
                );
            }
        }

        BlobHealOutcome::NotFound
    }

    /// GET /manifest/{hash} - Get shard manifest
    async fn handle_get_manifest(&self, hash: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        if hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("Missing blob hash")))
                .unwrap());
        }

        let manifest = self.resolve_manifest(hash).await;

        match manifest {
            Some(m) => {
                let body =
                    serde_json::to_string(&m).map_err(|e| StorageError::Internal(e.to_string()))?;

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body)))
                    .unwrap())
            }
            None => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Manifest not found")))
                .unwrap()),
        }
    }

    /// Handle P2P status request
    #[cfg(feature = "p2p")]
    async fn handle_p2p_status(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        // libp2p path: the swarm-backed handle is authoritative and unchanged —
        // byte-identical to before (full P2PStatusInfo incl. provideLoop). The
        // transport seam is NOT consulted here so libp2p behavior is preserved.
        if let Some(ref handle) = self.p2p_handle {
            let status = handle.status();
            let json = serde_json::to_string(&status).map_err(|e| {
                StorageError::Internal(format!("Failed to serialize P2P status: {}", e))
            })?;
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap());
        }

        // iroh path (no libp2p handle): report the transport's status peerId as
        // `.peerId` so the seeder can key commitments to this node and the
        // resilience card lights — instead of the legacy 503. The seam is the
        // ONLY new behavior; libp2p never reaches here.
        if let Some(peer_id) = self
            .node_transport
            .as_ref()
            .and_then(|t| t.status_peer_id())
        {
            let body = serde_json::json!({ "peerId": peer_id });
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(body.to_string())))
                .unwrap());
        }

        Ok(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(
                r#"{"error": "P2P networking not enabled"}"#,
            )))
            .unwrap())
    }

    /// Handle /p2p/peers — connected peers with identify info
    #[cfg(feature = "p2p")]
    async fn handle_p2p_peers(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        use crate::p2p::PeerListView;

        if let Some(ref handle) = self.p2p_handle {
            let peers = handle.list_peers().await;
            let total = peers.len();
            let response = PeerListView { peers, total };
            let json = serde_json::to_string(&response).map_err(|e| {
                StorageError::Internal(format!("Failed to serialize peer list: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap())
        } else {
            let empty = PeerListView {
                peers: vec![],
                total: 0,
            };
            let json = serde_json::to_string(&empty).unwrap();
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap())
        }
    }

    /// POST /admin/blob-transport-affinity/{hash} — set the per-object blob
    /// transport affinity (iroh-toggle sprint). Writes
    /// `peer_blob_inventory.transport_affinity` for the addressed blob.
    /// `"auto"` (the default) clears the column to NULL. Other values:
    /// `prefer-iroh`, `prefer-libp2p`, `iroh-only`, `libp2p-only`.
    ///
    /// Feature-gated: references `http_blob_router::TransportAffinity`, which is
    /// only compiled under `p2p-iroh`. The matching route arm is gated
    /// identically, so the default/libp2p build never compiles or reaches this.
    #[cfg(feature = "p2p-iroh")]
    async fn handle_set_blob_transport_affinity(
        &self,
        hash: &str,
        req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Normalize to the router's form: sha256-{hex} or blake3-{hex}.
        let normalized: String = if hash.starts_with("blake3-") {
            hash.to_string()
        } else {
            match crate::blob_store::BlobStore::parse_content_address(hash) {
                Ok(h) => format!("sha256-{}", h),
                Err(_) => return Ok(response::bad_request("Invalid content address")),
            }
        };

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("read body: {e}")))?
            .to_bytes();
        #[derive(serde::Deserialize)]
        struct AffinityReq {
            affinity: String,
        }
        let parsed: AffinityReq = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => return Ok(response::bad_request(&format!("Invalid JSON: {e}"))),
        };

        // Validate against the known vocabulary; unknown strings are rejected
        // here (the read-path `parse` is lenient, but a setter should be
        // strict). "auto" clears the column.
        let affinity = crate::http_blob_router::TransportAffinity::parse(&parsed.affinity);
        if affinity == crate::http_blob_router::TransportAffinity::Auto && parsed.affinity != "auto"
        {
            return Ok(response::bad_request(&format!(
                "unknown affinity '{}'",
                parsed.affinity
            )));
        }
        let stored: Option<&str> = match affinity {
            crate::http_blob_router::TransportAffinity::Auto => None, // clear → default
            other => Some(other.as_str()),
        };

        let mut conn = self.get_diesel_conn()?;
        let updated =
            crate::db::peer_blob_inventory::set_transport_affinity(&mut conn, &normalized, stored)?;
        Ok(response::json_response(
            StatusCode::OK,
            &serde_json::json!({
                "hash": normalized,
                "affinity": affinity.as_str(),
                "rowsUpdated": updated,
            }),
        ))
    }

    /// Operator/agent-initiated authority-arc actuation — the explicit POST
    /// trigger (spec §5). Body: `{"proposedFactor": 0|1, "commitmentCid": "..."}`.
    /// Reads the authorizing `sets-authority-arc` commitment by CID, reconstructs
    /// its grant payload, reads observed_N from the conductor, and runs
    /// `apply_arc_actuation` — which authorizes + runs the coverage gate and, only
    /// on pass, rewrites the config and RESTARTS the conductor (the only way to
    /// apply `target_arc_factor` — spec §2). A refusal (out-of-grant / expired /
    /// would-break-coverage) returns 409 with the finding to elevate; no restart.
    /// Explicit POST only — deliberately NO auto-subscriber that restarts the
    /// conductor on a commitment landing (churn risk); node-local (not in
    /// build_manifest, so not doorway-proxied).
    async fn handle_arc_policy_actuate(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        use crate::services::arc_actuator::{self, ActuationRefusal, ApplyError};

        // Actuation restarts the embedded conductor — unavailable without one.
        let conductor = match &self.conductor_manager {
            Some(cm) => cm.clone(),
            None => {
                return Ok(response::json_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &serde_json::json!({ "error": "arc actuation unavailable: no embedded conductor" }),
                ))
            }
        };

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("read body: {e}")))?
            .to_bytes();
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ActuateReq {
            proposed_factor: u32,
            commitment_cid: String,
        }
        let body: ActuateReq = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => return Ok(response::bad_request(&format!("Invalid JSON: {e}"))),
        };

        // Read the authorizing grant from the projection.
        let mut conn = self.get_diesel_conn()?;
        let row = match crate::db::mishpat_commitments::get_by_cid(&mut conn, &body.commitment_cid)
            .map_err(|e| StorageError::Database(e.to_string()))?
        {
            Some(r) => r,
            None => {
                return Ok(response::json_response(
                    StatusCode::NOT_FOUND,
                    &serde_json::json!({ "error": format!("no commitment for cid {}", body.commitment_cid) }),
                ))
            }
        };
        if row.action != "sets-authority-arc" {
            return Ok(response::bad_request(
                "commitment is not a sets-authority-arc grant",
            ));
        }
        if row.state != "active" || row.revoked_at.is_some() {
            return Ok(response::json_response(
                StatusCode::CONFLICT,
                &serde_json::json!({ "error": "grant is not active (revoked or not yet active)" }),
            ));
        }
        // Reconstruct the grant payload `compute_actuation` expects from the row.
        let bounds: serde_json::Value = serde_json::from_str(&row.bounds_json)
            .map_err(|e| StorageError::Internal(format!("bounds_json parse: {e}")))?;
        let grant_payload = serde_json::json!({
            "action": row.action,
            "bounds": bounds,
            "valid_until": row.valid_until,
        });

        // observed_N (conductor DHT peer count) — the coverage gate needs it.
        let observed_n = match self.hc_registry.as_ref().and_then(|r| r.lamad_client()) {
            Some(hc) => hc
                .get_health()
                .await
                .network
                .as_ref()
                .and_then(|nw| nw.peer_count)
                .map(|c| c as u32),
            None => None,
        };
        let observed_n = match observed_n {
            Some(n) => n,
            None => {
                return Ok(response::json_response(
                    StatusCode::SERVICE_UNAVAILABLE,
                    &serde_json::json!({ "error": "cannot read conductor peer count (observed_N) — refusing actuation" }),
                ))
            }
        };

        let now = chrono::Utc::now().timestamp().max(0) as u64;
        let outcome = {
            let mut cm = conductor.lock().await;
            arc_actuator::apply_arc_actuation(
                body.proposed_factor,
                &body.commitment_cid,
                &grant_payload,
                observed_n,
                &mut cm, // tokio MutexGuard<ConductorManager> → &mut ConductorManager (DerefMut)
                now,
            )
            .await
        };

        match outcome {
            Ok(applied) => Ok(response::json_response(
                StatusCode::OK,
                &serde_json::json!({
                    "status": "applied",
                    "targetArcFactor": applied,
                    "commitmentCid": body.commitment_cid,
                    "observedN": observed_n,
                }),
            )),
            Err(ApplyError::Refused(ActuationRefusal { code, elevate })) => {
                Ok(response::json_response(
                    StatusCode::CONFLICT,
                    &serde_json::json!({
                        "status": "refused",
                        "code": format!("{code:?}"),
                        "elevate": elevate,
                    }),
                ))
            }
            Err(other) => Ok(response::json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &serde_json::json!({ "status": "error", "error": format!("{other:?}") }),
            )),
        }
    }

    /// PUT /admin/seed/shard-manifest — operator/seed deterministic write of a
    /// shard manifest + agent-keyed shard_locations for a content.
    ///
    /// This is the **deterministic lever** that lights the resilience card's
    /// `distributionState: measured` + `stewardingCollectives` for blob-backed
    /// demo/test content WITHOUT waiting on full P2P gossip. The genuine
    /// distribution path is `POST /db/content` (with a blob_hash) →
    /// `distribute_shards` over a live mesh; this endpoint produces the same
    /// table shape for a thin mesh or an acceptance test.
    ///
    /// ## Honesty gate (off by default)
    ///
    /// Refuses with 403 unless `ALLOW_SEED_SHARD_MANIFEST=1`. Lighting these
    /// columns is a CLAIM that the named households hold the content; a real
    /// deployment (flag unset) can never fabricate distribution through this path.
    /// Seeded rows carry `status="seeded"` so their provenance stays legible
    /// alongside runtime `announced`/`verified` rows.
    ///
    /// Category C (operational): no new DHT entity, no new table, no signal.
    /// Identity is agent-keyed (`peerId` = the steward's `agent_pub_key`).
    async fn handle_seed_shard_manifest(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        use crate::services::seed_shard_manifest::{apply, SeedShardManifestInput, SeedSteward};

        // --- Honesty gate: off unless the operator explicitly opts in. --------
        let allowed = std::env::var("ALLOW_SEED_SHARD_MANIFEST")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !allowed {
            return Ok(response::forbidden(&serde_json::json!({
                "error": "seed shard-manifest is disabled",
                "hint": "set ALLOW_SEED_SHARD_MANIFEST=1 to enable this operator/seed lever",
                "note": "this path fabricates distribution rows for demo/test; production lights the card via the real distribute_shards path",
            })));
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("read body: {e}")))?
            .to_bytes();

        // Local request shape — camelCase wire, operator/seed-only (no TS client
        // consumes it, so no ts-rs View; mirrors arc-policy/actuate's local req).
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SeedReq {
            content_id: String,
            #[serde(default)]
            h_app_id: Option<String>,
            #[serde(default)]
            reach: Option<String>,
            #[serde(default)]
            shard_hashes: Vec<String>,
            #[serde(default)]
            blob_hash: Option<String>,
            /// Stewards as agent_pub_key strings.
            stewards: Vec<String>,
            #[serde(default)]
            total_size_bytes: Option<i64>,
        }

        let body: SeedReq = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => return Ok(response::bad_request(&format!("Invalid JSON: {e}"))),
        };

        let input = SeedShardManifestInput {
            h_app_id: body.h_app_id.unwrap_or_else(|| "lamad".to_string()),
            content_id: body.content_id,
            reach: body.reach.unwrap_or_else(|| "commons".to_string()),
            shard_hashes: body.shard_hashes,
            blob_hash: body.blob_hash,
            stewards: body
                .stewards
                .into_iter()
                .map(|peer_id| SeedSteward { peer_id })
                .collect(),
            total_size_bytes: body.total_size_bytes.unwrap_or(0),
        };

        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("pool: {e}")))?;

        match apply(&mut conn, &input) {
            Ok(report) => Ok(response::created(&report)),
            Err(e @ StorageError::InvalidInput(_)) => Ok(response::bad_request(&format!("{e}"))),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// Operational (Cat-C) gauge for the conductor authority-arc Auto policy
    /// (spec §6 of `2026-06-13-conductor-authority-arc-auto-policy.md`). A
    /// node-local read-model — NOT a source of truth: it surfaces the live
    /// inputs `derive()` uses (cgroup mem ceiling via M1, conductor peer count +
    /// corpus working-set via `get_health`), the derived arc aim, the coverage
    /// floor, and the derivation "why". Reconstructable from live state, so no
    /// `dht_anchor_hash`. On the deployed substrate the actuatable lever is
    /// `{0,1}` (spike §2), so the derived fractional value is a SIGNAL surfaced
    /// here, not yet directly actuatable.
    async fn handle_arc_policy_status(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        use crate::services::arc_policy::{self, ArcInputs, CoverageParams};
        use crate::services::system_metrics;

        // M1: the CONTAINER cgroup ceiling (not host RAM). Headroom for the
        // storage parent (it shares the cgroup with the conductor) ≈ this
        // process's own RSS.
        let mem_ceiling = system_metrics::container_memory_limit_bytes();
        let storage_headroom = system_metrics::process_memory_bytes().unwrap_or(0);

        // observed_N (conductor DHT peer count) + corpus working-set proxy
        // (storage.bytes_used) from one health call. Best-effort: a missing
        // conductor client / health failure leaves these None → safe full-arc.
        let (observed_n, corpus_bytes, entry_count) =
            if let Some(hc) = self.hc_registry.as_ref().and_then(|r| r.lamad_client()) {
                let h = hc.get_health().await;
                let n = h
                    .network
                    .as_ref()
                    .and_then(|nw| nw.peer_count)
                    .map(|c| c as u32);
                let c = h.storage.as_ref().map(|s| s.bytes_used);
                let e = h.storage.as_ref().map(|s| s.entry_count);
                (n, c, e)
            } else {
                (None, None, None)
            };

        let coverage = CoverageParams::default();
        let decision = arc_policy::derive(ArcInputs {
            mem_ceiling_bytes: mem_ceiling,
            storage_headroom_bytes: storage_headroom,
            observed_n,
            corpus_bytes,
            // L (own authored share) is not yet split out of the corpus stat;
            // best-effort 0 (treats all corpus as foreign-arc-eligible — slightly
            // optimistic on a_mem). Refining L is a follow-up (spec §3, §8).
            local_authored_bytes: 0,
            archetype_base_arc: 1.0,
            coverage,
        });

        let body = serde_json::json!({
            "resources": {
                "memCeilingBytes": mem_ceiling,
                "storageHeadroomBytes": storage_headroom,
                "observedN": observed_n,
                "corpusBytes": corpus_bytes,
                "entryCount": entry_count,
            },
            "derived": {
                "targetArcFactor": decision.arc_factor,
                // Spike §2: fractional arc is NOT actuatable on the deployed line;
                // the only lever is {0,1}. This fractional value is a SIGNAL.
                "leverTier": "T1-binary",
                "actuatableValues": [0, 1],
                "safeDefault": decision.safe_default,
            },
            "overrides": {},
            "coverage": {
                "rFloor": coverage.r_floor,
                "rTarget": coverage.r_target,
                "aCovFloor": decision.coverage_floor,
                "coverageOverMemory": decision.coverage_over_memory,
                "meshCoverage": "unknown@P0",
            },
            "reasons": decision.reasons,
            "elevate": decision.elevate,
        });

        let json = serde_json::to_string(&body)
            .map_err(|e| StorageError::Internal(format!("arc-policy status serialize: {e}")))?;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(json)))
            .unwrap())
    }

    /// Handle delivery peers request — returns discovered peers with capabilities
    #[cfg(feature = "p2p")]
    /// `GET /api/v1/federation/hosted-binding/{agent_pub_key}` — resolve an
    /// agent's home doorway from the `hosted_agent_bindings` projection.
    ///
    /// 200 with the binding, or 404 `{"error":"no-hosted-binding"}` when the
    /// agent has none. A 404 here is HONEST ABSENCE, not a failure: the caller
    /// (a sibling doorway's Chaperone) degrades it to the existing 409
    /// `hosted-elsewhere` response rather than guessing a redirect target.
    async fn handle_hosted_binding(
        &self,
        agent_pub_key: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // `HoloHashB64` renders base64url (no `+`, `/`, or `=`), so the agent key
        // is already path-safe — no percent-decoding step to get wrong.
        let agent = agent_pub_key.to_string();
        if agent.trim().is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error":"agent_pub_key required"}"#,
                )))
                .unwrap());
        }

        let pool = self.db_pool.as_ref().ok_or_else(|| {
            StorageError::Internal("hosted-binding lookup requires a database".to_string())
        })?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Database(e.to_string()))?;

        match crate::db::hosted_agent_bindings::current_binding_for_agent(&mut conn, &agent)? {
            Some(binding) => {
                let json = serde_json::to_string(&binding).map_err(|e| {
                    StorageError::Internal(format!("Failed to serialize hosted binding: {e}"))
                })?;
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(json)))
                    .unwrap())
            }
            None => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"no-hosted-binding"}"#)))
                .unwrap()),
        }
    }

    async fn handle_delivery_peers(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        if let Some(ref handle) = self.p2p_handle {
            let mut peers = handle.delivery_peers();

            // Request-time enrichment (soft-fail): the discovery path can't reach
            // the DB, so we resolve each peer's owning household hub and its
            // active provide-commitment reach tags here. The a2o resilience
            // precondition counts distinct `householdId`s among peers whose
            // `commitments` include the requested reach (e.g. "commons") — see
            // genesis/a2o/steps/resilience.steps.ts. Any DB/lookup error degrades
            // that peer to None/empty; it never fails the endpoint.
            if let Some(ref pool) = self.db_pool {
                let app_ctx = crate::db::AppContext::default_lamad();
                let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                if let Ok(mut conn) = pool.get() {
                    for peer in peers.iter_mut() {
                        peer.household_id =
                            crate::services::hub_resolver::resolve_peer_dwelling_hub(
                                &mut conn,
                                &peer.peer_id,
                                &now_iso,
                            )
                            .unwrap_or(None);
                        peer.commitments = crate::db::rea_commitments::active_provide_reaches(
                            &mut conn,
                            &app_ctx,
                            &peer.peer_id,
                        )
                        .unwrap_or_default();
                    }
                }
            }

            let json = serde_json::to_string(&peers).map_err(|e| {
                StorageError::Internal(format!("Failed to serialize delivery peers: {}", e))
            })?;
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(json)))
                .unwrap())
        } else {
            // No P2P — return empty array (not an error)
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from("[]")))
                .unwrap())
        }
    }

    /// GET /api/v1/diagnostics/inventory-parity
    ///
    /// Returns a `ParityReport` comparing the local filesystem blob inventory
    /// against the last gossiped inventory snapshot.  This defends against the
    /// failure mode where gossip runs cleanly but bytes never replicate.
    ///
    /// Stage 1: `gossiped_count` will always be 0 (no broadcaster timer yet).
    /// Stage 2: broadcaster timer populates `P2PHandle::set_last_gossiped_inventory`.
    async fn handle_inventory_parity(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        use crate::p2p::inventory_broadcaster::{compute_parity, ParityReport};

        // StoreAdapter satisfies LocalInventory by delegating to BlobStore::list_hashes,
        // converting raw filenames to validated BlobAddress instances and dropping
        // any non-canonical entries (logged once per session).
        struct StoreAdapter<'a>(&'a crate::blob_store::BlobStore);
        impl crate::p2p::inventory_broadcaster::LocalInventory for StoreAdapter<'_> {
            fn current_hashes(&self) -> Vec<crate::p2p::inventory_gossip::BlobAddress> {
                use crate::p2p::inventory_gossip::BlobAddress;
                use std::sync::Once;
                static WARN_ONCE: Once = Once::new();

                let raw = self.0.list_hashes().unwrap_or_default();
                let mut dropped = 0_usize;
                let addresses: Vec<BlobAddress> = raw
                    .into_iter()
                    .filter_map(|s| match BlobAddress::new(s) {
                        Ok(a) => Some(a),
                        Err(_) => {
                            dropped += 1;
                            None
                        }
                    })
                    .collect();

                if dropped > 0 {
                    WARN_ONCE.call_once(|| {
                        tracing::warn!(
                            target: "elohim_storage::inventory",
                            dropped,
                            "BlobStore::list_hashes returned non-canonical filenames; \
                             dropped from inventory advertisement (logged once per session)"
                        );
                    });
                }
                addresses
            }
        }

        let adapter = StoreAdapter(&self.blob_store);

        #[cfg(feature = "p2p")]
        let gossiped: Vec<String> = self
            .p2p_handle
            .as_ref()
            .and_then(|h| h.last_gossiped_inventory())
            .unwrap_or_default();

        #[cfg(not(feature = "p2p"))]
        let gossiped: Vec<String> = vec![];

        let now_iso = chrono::Utc::now().to_rfc3339();
        let report: ParityReport = compute_parity(&adapter, &gossiped, &now_iso);

        let json = serde_json::to_string(&report).map_err(|e| {
            StorageError::Internal(format!("Failed to serialize parity report: {}", e))
        })?;

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(json)))
            .unwrap())
    }

    /// Handle sync API requests
    ///
    /// Routes:
    /// - GET /sync/v1/{h_app_id}/docs - List documents
    /// - GET /sync/v1/{h_app_id}/docs/{doc_id}/heads - Get document heads
    /// - GET /sync/v1/{h_app_id}/docs/{doc_id}/changes?have={heads} - Get changes since heads
    /// - POST /sync/v1/{h_app_id}/docs/{doc_id}/changes - Apply changes
    async fn handle_sync_request(
        &self,
        req: Request<Incoming>,
        method: Method,
        path: &str,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Parse path: /sync/v1/{h_app_id}/docs[/{doc_id}[/heads|/changes]]
        let parts: Vec<&str> = path.trim_start_matches("/sync/v1/").split('/').collect();

        if parts.is_empty() || parts[0].is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Missing h_app_id"}"#)))
                .unwrap());
        }

        let h_app_id = parts[0];

        // /sync/v1/{h_app_id}/docs
        if parts.len() == 2 && parts[1] == "docs" {
            return self
                .handle_sync_list_docs(method, h_app_id, &req, sync_manager)
                .await;
        }

        // /sync/v1/{h_app_id}/docs/{doc_id}
        if parts.len() == 3 && parts[1] == "docs" {
            let doc_id = parts[2];
            return self
                .handle_sync_doc(method, h_app_id, doc_id, req, sync_manager)
                .await;
        }

        // /sync/v1/{h_app_id}/docs/{doc_id}/{action}
        if parts.len() == 4 && parts[1] == "docs" {
            let doc_id = parts[2];
            let action = parts[3];

            return match action {
                "heads" => {
                    self.handle_sync_heads(method, h_app_id, doc_id, sync_manager)
                        .await
                }
                "changes" => {
                    self.handle_sync_changes(method, h_app_id, doc_id, req, sync_manager)
                        .await
                }
                _ => Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Unknown action: {}"}}"#,
                        action
                    ))))
                    .unwrap()),
            };
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(r#"{"error": "Invalid sync path"}"#)))
            .unwrap())
    }

    /// GET /sync/v1/{h_app_id}/docs - List documents
    async fn handle_sync_list_docs(
        &self,
        method: Method,
        h_app_id: &str,
        req: &Request<Incoming>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        // Parse query params: ?prefix=&offset=&limit=
        let query = req.uri().query().unwrap_or("");
        let params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

        let prefix = params.get("prefix").map(|s| s.as_str());
        let offset: u32 = params
            .get("offset")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let limit: u32 = params
            .get("limit")
            .and_then(|s| s.parse().ok())
            .unwrap_or(100);

        match sync_manager
            .list_documents(h_app_id, prefix, offset, limit)
            .await
        {
            Ok((docs, total)) => {
                let documents: Vec<serde_json::Value> = docs
                    .into_iter()
                    .map(|d| {
                        serde_json::json!({
                            "docId": d.doc_id,
                            "docType": d.doc_type,
                            "changeCount": d.change_count,
                            "lastModified": d.last_modified,
                            "heads": d.heads,
                        })
                    })
                    .collect();

                let body = serde_json::json!({
                    "hAppId": h_app_id,
                    "documents": documents,
                    "total": total,
                    "offset": offset,
                    "limit": limit,
                });

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body.to_string())))
                    .unwrap())
            }
            Err(e) => {
                error!(h_app_id = %h_app_id, error = %e, "Failed to list documents");
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                    .unwrap())
            }
        }
    }

    /// Handle document-level requests
    async fn handle_sync_doc(
        &self,
        method: Method,
        h_app_id: &str,
        doc_id: &str,
        _req: Request<Incoming>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        match method {
            Method::GET => {
                // Return document info
                match sync_manager.get_heads(h_app_id, doc_id).await {
                    Ok(heads) => {
                        if heads.is_empty() {
                            return Ok(Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(
                                    r#"{{"error": "Document not found: {}"}}"#,
                                    doc_id
                                ))))
                                .unwrap());
                        }

                        let body = serde_json::json!({
                            "hAppId": h_app_id,
                            "docId": doc_id,
                            "heads": heads,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body.to_string())))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to get document");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                            .unwrap())
                    }
                }
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    /// GET /sync/v1/{h_app_id}/docs/{doc_id}/heads - Get document heads
    async fn handle_sync_heads(
        &self,
        method: Method,
        h_app_id: &str,
        doc_id: &str,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap());
        }

        match sync_manager.get_heads(h_app_id, doc_id).await {
            Ok(heads) => {
                let body = serde_json::json!({
                    "hAppId": h_app_id,
                    "docId": doc_id,
                    "heads": heads,
                });

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(body.to_string())))
                    .unwrap())
            }
            Err(e) => {
                error!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to get heads");
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                    .unwrap())
            }
        }
    }

    /// GET/POST /sync/v1/{h_app_id}/docs/{doc_id}/changes
    async fn handle_sync_changes(
        &self,
        method: Method,
        h_app_id: &str,
        doc_id: &str,
        req: Request<Incoming>,
        sync_manager: Arc<SyncManager>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        match method {
            Method::GET => {
                // GET changes since given heads
                let query = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query.as_bytes())
                        .into_owned()
                        .collect();

                // Parse have_heads from comma-separated list
                let have_heads: Vec<String> = params
                    .get("have")
                    .map(|s| s.split(',').map(|h| h.to_string()).collect())
                    .unwrap_or_default();

                match sync_manager
                    .get_changes_since(h_app_id, doc_id, &have_heads)
                    .await
                {
                    Ok((changes, new_heads)) => {
                        // Encode changes as base64 for JSON transport
                        let changes_b64: Vec<String> = changes
                            .iter()
                            .map(|c| {
                                base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    c,
                                )
                            })
                            .collect();

                        let body = serde_json::json!({
                            "hAppId": h_app_id,
                            "docId": doc_id,
                            "changes": changes_b64,
                            "newHeads": new_heads,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body.to_string())))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to get changes");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                            .unwrap())
                    }
                }
            }
            Method::POST => {
                // Apply changes from client
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();

                // Parse JSON body: { "changes": ["base64..."] }
                let payload: serde_json::Value = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

                let changes_b64 = payload["changes"]
                    .as_array()
                    .ok_or_else(|| StorageError::Internal("Missing 'changes' array".to_string()))?;

                // Decode base64 changes
                let changes: Vec<Vec<u8>> = changes_b64
                    .iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| {
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, s).ok()
                    })
                    .collect();

                if changes.is_empty() {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(r#"{"error": "No valid changes"}"#)))
                        .unwrap());
                }

                match sync_manager.apply_changes(h_app_id, doc_id, changes).await {
                    Ok(new_heads) => {
                        info!(h_app_id = %h_app_id, doc_id = %doc_id, heads = ?new_heads, "Applied changes via HTTP");

                        let body = serde_json::json!({
                            "hAppId": h_app_id,
                            "docId": doc_id,
                            "newHeads": new_heads,
                        });

                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(body.to_string())))
                            .unwrap())
                    }
                    Err(e) => {
                        error!(h_app_id = %h_app_id, doc_id = %doc_id, error = %e, "Failed to apply changes");
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                            .unwrap())
                    }
                }
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap()),
        }
    }

    // =========================================================================
    // Database API handlers
    // =========================================================================

    /// Handle database API requests
    ///
    /// Routes:
    /// - GET /db/stats - Database statistics
    /// - GET /db/content - List content (with query params)
    /// - GET /db/content/{id} - Get content by ID
    /// - POST /db/content - Create single content
    /// - POST /db/content/bulk - Bulk create content
    /// - GET /db/relationships - List relationships (with query params)
    /// - GET /db/relationships/{id} - Get relationship by ID
    /// - POST /db/relationships - Create relationship
    /// - POST /db/relationships/bulk - Bulk create relationships
    /// - GET /db/relationships/graph/{content_id} - Get content graph
    /// - GET /db/knowledge-maps - List knowledge maps
    /// - GET /db/knowledge-maps/{id} - Get knowledge map by ID
    /// - POST /db/knowledge-maps - Create knowledge map
    /// - PUT /db/knowledge-maps/{id} - Update knowledge map
    /// - DELETE /db/knowledge-maps/{id} - Delete knowledge map
    ///
    /// Extract app context from path, supporting both:
    /// - New: /db/{h_app_id}/content/... -> AppContext(h_app_id)
    /// - Legacy: /db/content/... -> AppContext("lamad") for backwards compatibility
    fn extract_app_context(sub_path: &str) -> (db::AppContext, &str) {
        // Check if path starts with a known resource type (legacy route)
        // This must include ALL top-level resource prefixes from handle_db_request
        // to prevent them from being misinterpreted as h_app_id values.
        let legacy_prefixes = [
            // Core content routes
            "content",
            "stats",
            "schema",
            // Diesel entity routes
            "collectives",
            "humans",
            // did:elohim resolution namespace (/db/identity/did/{did}). Without
            // this entry "identity" is consumed as an h_app_id and the
            // resolution dispatch arm becomes dead code (the same shadow class
            // the "p2p" comment below warns about).
            "identity",
            "human-relationships",
            "presences",
            "events",
            "mastery",
            "allocations",
            "nodes",
            "relationships",
            "knowledge-maps",
            // Projection-read routes (single-segment). These MUST be listed here:
            // otherwise extract_app_context's new-route branch (below) treats the
            // single segment as an h_app_id and defaults resource_path to "stats",
            // SHADOWING the real handler (it becomes dead code) and returning an
            // empty DbStats {contentCount:0,uniqueTags:0}. That is exactly why the
            // doorway EprRouter boot-fetch/refresh got "error decoding response body"
            // (DbStats is not Vec<EprProjectionView>) and "/" + "/lamad" 404'd on
            // every doorway even though the project-epr commitments existed in SQL.
            "rea_commitments",
            "gate-decisions",
            "gate-decision-challenges",
            "challenge-outcomes",
            "elohim-reputation",
            // Conductor/network diagnostics namespace (dht-unity T3). Without
            // this entry, "p2p" is consumed as an h_app_id and the
            // conductor-diagnostics dispatch arm is dead code (the shadow
            // class this comment block warns about — proven live on edge
            // #1172: route 404'd "Unknown database endpoint" on new pods).
            "p2p",
        ];
        for prefix in &legacy_prefixes {
            if sub_path == *prefix || sub_path.starts_with(&format!("{}/", prefix)) {
                // Legacy route: default to 'lamad' for learning content
                return (db::AppContext::default_lamad(), sub_path);
            }
        }

        // New route: /db/{h_app_id}/...
        if let Some(slash_pos) = sub_path.find('/') {
            let h_app_id = &sub_path[..slash_pos];
            let resource_path = &sub_path[slash_pos + 1..];
            return (db::AppContext::new(h_app_id), resource_path);
        }

        // Just h_app_id with no resource (e.g., /db/lamad -> stats for that app)
        if !sub_path.is_empty() && !legacy_prefixes.contains(&sub_path) {
            return (db::AppContext::new(sub_path), "stats");
        }

        // Fallback to default
        (db::AppContext::default_lamad(), sub_path)
    }

    async fn handle_db_request(
        &self,
        req: Request<Incoming>,
        method: Method,
        path: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // Strip /db/ prefix
        let sub_path = path.strip_prefix("/db/").unwrap_or("");

        // Extract app context (supports both legacy and new routes)
        let (app_ctx, resource_path) = Self::extract_app_context(sub_path);
        debug!(h_app_id = %app_ctx.h_app_id, resource_path = %resource_path, "DB request routing");

        // Route to specific handlers (all use Diesel pool via self.get_conn())
        if resource_path == "stats" {
            return self.handle_db_stats(method, &app_ctx).await;
        }

        if resource_path == "schema" {
            if method != Method::GET {
                return Ok(response::method_not_allowed());
            }
            let deprecated: Vec<u32> = vec![];
            return Ok(response::ok(&serde_json::json!({
                "supportedVersions": SUPPORTED_SCHEMA_VERSIONS,
                "currentVersion": SUPPORTED_SCHEMA_VERSIONS.last().copied().unwrap_or(1),
                "deprecatedVersions": deprecated,
            })));
        }

        // Conductor DHT diagnostics (Cat-C operational read; dht-unity T3).
        // The permanent seam-visibility handle: what does THIS node's embedded
        // conductor know about its peers (peer store) and its transport/fetch
        // state — the read that distinguishes "never learned the address" from
        // "knows it, cannot connect" during cross-conductor fetch failures.
        if resource_path == "p2p/conductor-diagnostics" {
            return self.handle_conductor_diagnostics(req, method).await;
        }

        // did:elohim resolution (Cat-C operational projection; DID bridge spec
        // §3.4). GET /db/identity/did/{did} → the assembled DID 1.1 document.
        // The `{did}` tail (e.g. `did:elohim:uhCAk…`) carries colons but no
        // slashes, so a single strip_prefix recovers it whole.
        if let Some(did_str) = resource_path.strip_prefix("identity/did/") {
            return self.handle_did_resolution(method, did_str).await;
        }

        if resource_path == "content" {
            return self.handle_db_content_list(req, method).await;
        }

        if resource_path == "content/bulk" {
            return self.handle_db_content_bulk(req, method).await;
        }

        // Entity-nested schedule routes: /db/content/{cid}/schedule
        if let Some(rest) = resource_path.strip_prefix("content/") {
            if let Some(cid) = rest.strip_suffix("/schedule") {
                return self
                    .handle_content_schedule(req, method, cid, &app_ctx)
                    .await;
            }
            // Entity-nested notary-HEAD authority: /db/content/{cid}/head
            // (HEAD-election, Plan C3 / Leg 3). Must precede the generic
            // content/{id} catch-all below.
            // Entity-nested head-RECORD service: /db/content/{cid}/head-record
            // (declare-carries-Record SOURCE half). Kept first for readability —
            // there is no real collision with the "/head" suffix arm below, since
            // strip_suffix("/head") never matches "…/head-record"; keeping the
            // more specific suffix first makes the ordering intent explicit and
            // survives a future "/head*" widening.
            if let Some(cid) = rest.strip_suffix("/head-record") {
                return self.handle_content_head_record(method, cid, &app_ctx).await;
            }
            if let Some(cid) = rest.strip_suffix("/head") {
                return self.handle_content_head(req, method, cid, &app_ctx).await;
            }
            // Entity-nested CROSS-ROOT canonical-head declaration (notary-
            // authority convergence, Model B / Tier-1 STAGING tier). Distinct
            // from the single-author `/head` route above — no author gate;
            // edge auth (auth_required) is the route's protection. Must
            // precede the generic content/{id} catch-all below.
            if let Some(cid) = rest.strip_suffix("/canonical-head") {
                return self
                    .handle_content_canonical_head(req, method, cid, &app_ctx)
                    .await;
            }
        }

        if let Some(content_id) = resource_path.strip_prefix("content/") {
            return self.handle_db_content_by_id(req, method, content_id).await;
        }

        // Relationships routes
        if resource_path == "relationships" {
            return self.handle_db_relationships_list(req, method).await;
        }

        if resource_path == "relationships/bulk" {
            return self.handle_db_relationships_bulk(req, method).await;
        }

        if let Some(rel_id) = resource_path.strip_prefix("relationships/graph/") {
            return self.handle_db_content_graph(req, method, rel_id).await;
        }

        if let Some(rel_id) = resource_path.strip_prefix("relationships/") {
            return self.handle_db_relationship_by_id(req, method, rel_id).await;
        }

        // Knowledge maps routes
        if resource_path == "knowledge-maps" {
            return self.handle_db_knowledge_maps_list(req, method).await;
        }

        if let Some(map_id) = resource_path.strip_prefix("knowledge-maps/") {
            return self
                .handle_db_knowledge_map_by_id(req, method, map_id)
                .await;
        }

        // ============================================================================
        // Diesel-based entity routes (using db_pool)
        // ============================================================================

        // Human relationships routes (Diesel)
        if resource_path == "human-relationships" {
            return self
                .handle_human_relationships_list(req, method, &app_ctx)
                .await;
        }

        if let Some(rel_path) = resource_path.strip_prefix("human-relationships/") {
            // Check for action sub-paths first
            if let Some(rest) = rel_path.strip_suffix("/consent") {
                return self
                    .handle_human_relationship_consent(req, method, rest, &app_ctx)
                    .await;
            }
            if let Some(rest) = rel_path.strip_suffix("/custody") {
                return self
                    .handle_human_relationship_custody(req, method, rest, &app_ctx)
                    .await;
            }
            // Fall back to generic ID handler
            return self
                .handle_human_relationship_by_id(req, method, rel_path, &app_ctx)
                .await;
        }

        // Human directory route (Diesel)
        if resource_path == "humans" && method == Method::GET {
            return self.handle_list_humans(req, &app_ctx).await;
        }

        // Collective routes (Diesel)
        if resource_path == "collectives" {
            return self.handle_collectives_list(req, method, &app_ctx).await;
        }

        if let Some(coll_path) = resource_path.strip_prefix("collectives/") {
            // Check for participants sub-path
            if let Some(rest) = coll_path.strip_suffix("/participants") {
                return self
                    .handle_collective_participants(req, method, rest, &app_ctx)
                    .await;
            }
            // Check for participants/{human_id} delete pattern
            if coll_path.contains("/participants/") {
                let parts: Vec<&str> = coll_path.splitn(3, '/').collect();
                if parts.len() == 3 && parts[1] == "participants" {
                    return self
                        .handle_collective_participant_depart(
                            req, method, parts[0], parts[2], &app_ctx,
                        )
                        .await;
                }
            }
            // Fall back to generic ID handler
            return self
                .handle_collective_by_id(req, method, coll_path, &app_ctx)
                .await;
        }

        // Participations by human route
        if let Some(human_id) = resource_path.strip_prefix("participations/") {
            return self
                .handle_participations_by_human(req, method, human_id, &app_ctx)
                .await;
        }

        // Contributor presences routes (Diesel)
        if resource_path == "presences" {
            return self.handle_presences_list(req, method, &app_ctx).await;
        }

        if resource_path == "presences/bulk" {
            return self.handle_presences_bulk(req, method, &app_ctx).await;
        }

        if let Some(presence_path) = resource_path.strip_prefix("presences/") {
            // Check for action sub-paths first
            if let Some(rest) = presence_path.strip_suffix("/stewardship") {
                return self
                    .handle_presence_stewardship(req, method, rest, &app_ctx)
                    .await;
            }
            if let Some(rest) = presence_path.strip_suffix("/claim") {
                return self
                    .handle_presence_claim(req, method, rest, &app_ctx)
                    .await;
            }
            if let Some(rest) = presence_path.strip_suffix("/verify-claim") {
                return self
                    .handle_presence_verify_claim(req, method, rest, &app_ctx)
                    .await;
            }
            // Fall back to generic ID handler
            return self
                .handle_presence_by_id(req, method, presence_path, &app_ctx)
                .await;
        }

        // Economic events routes (Diesel)
        if resource_path == "events" {
            return self.handle_events_list(req, method, &app_ctx).await;
        }

        if resource_path == "events/bulk" {
            return self.handle_events_bulk(req, method, &app_ctx).await;
        }

        if let Some(event_id) = resource_path.strip_prefix("events/") {
            return self
                .handle_event_by_id(req, method, event_id, &app_ctx)
                .await;
        }

        // Content mastery routes (Diesel)
        if resource_path == "mastery" {
            return self.handle_mastery_list(req, method, &app_ctx).await;
        }

        if resource_path == "mastery/bulk" {
            return self.handle_mastery_bulk(req, method, &app_ctx).await;
        }

        if let Some(mastery_path) = resource_path.strip_prefix("mastery/") {
            // Support /mastery/human/{human_id} and /mastery/{id}
            if let Some(human_id) = mastery_path.strip_prefix("human/") {
                return self
                    .handle_mastery_for_human(req, method, human_id, &app_ctx)
                    .await;
            }
            return self
                .handle_mastery_by_id(req, method, mastery_path, &app_ctx)
                .await;
        }

        // Stewardship allocations routes (Diesel)
        if resource_path == "allocations" {
            return self.handle_allocations_list(req, method, &app_ctx).await;
        }

        if resource_path == "allocations/bulk" {
            return self.handle_allocations_bulk(req, method, &app_ctx).await;
        }

        if let Some(alloc_path) = resource_path.strip_prefix("allocations/") {
            // Support /allocations/content/{content_id} and /allocations/steward/{steward_id}
            if let Some(content_id) = alloc_path.strip_prefix("content/") {
                return self
                    .handle_allocations_for_content(req, method, content_id, &app_ctx)
                    .await;
            }
            if let Some(steward_id) = alloc_path.strip_prefix("steward/") {
                return self
                    .handle_allocations_for_steward(req, method, steward_id, &app_ctx)
                    .await;
            }
            // Check for action sub-paths
            if let Some(rest) = alloc_path.strip_suffix("/dispute") {
                return self
                    .handle_allocation_dispute(req, method, rest, &app_ctx)
                    .await;
            }
            if let Some(rest) = alloc_path.strip_suffix("/resolve") {
                return self
                    .handle_allocation_resolve(req, method, rest, &app_ctx)
                    .await;
            }
            return self
                .handle_allocation_by_id(req, method, alloc_path, &app_ctx)
                .await;
        }

        // Stewarded node routes (Diesel)
        if resource_path == "nodes" {
            return self.handle_nodes_list(req, method, &app_ctx).await;
        }

        if let Some(node_path) = resource_path.strip_prefix("nodes/") {
            // /db/nodes/{id}/stewardship
            if let Some(node_id) = node_path.strip_suffix("/stewardship") {
                return self
                    .handle_node_stewardship(req, method, node_id, &app_ctx)
                    .await;
            }
            // /db/nodes/{id}
            return self
                .handle_node_by_id(req, method, node_path, &app_ctx)
                .await;
        }

        // Gate decision attestation routes (Diesel projection of mishpat DHT)
        // Source of truth: mishpat DNA DHT. These are read-only projections.
        if resource_path == "gate-decisions" {
            return self.handle_gate_decisions_list(req, method, &app_ctx).await;
        }

        if let Some(decision_id) = resource_path.strip_prefix("gate-decisions/") {
            return self
                .handle_gate_decision_by_id(req, method, decision_id, &app_ctx)
                .await;
        }

        // Gate decision challenge routes (Diesel projection of mishpat DHT)
        // Source of truth: mishpat DNA DHT. These are read-only projections.
        if resource_path == "gate-decision-challenges" {
            return self
                .handle_gate_decision_challenges_list(req, method, &app_ctx)
                .await;
        }

        if let Some(challenge_id) = resource_path.strip_prefix("gate-decision-challenges/") {
            return self
                .handle_gate_decision_challenge_by_id(req, method, challenge_id, &app_ctx)
                .await;
        }

        // Challenge outcome routes (Diesel projection of mishpat DHT)
        // Source of truth: mishpat DNA DHT. These are read-only projections.
        if resource_path == "challenge-outcomes" {
            return self
                .handle_challenge_outcomes_list(req, method, &app_ctx)
                .await;
        }

        if let Some(outcome_id) = resource_path.strip_prefix("challenge-outcomes/") {
            return self
                .handle_challenge_outcome_by_id(req, method, outcome_id, &app_ctx)
                .await;
        }

        // Elohim reputation aggregation query (Phase 11 Task 11.3)
        // GET /db/elohim-reputation?elohimId=<id>&windowStart=<iso>&windowEnd=<iso>
        if resource_path == "elohim-reputation" {
            return self.handle_elohim_reputation(req, method, &app_ctx).await;
        }

        // ── Conductor-bridge routes (Phase 10 HTTP pipes, Phase 11 full zome queries) ──
        //
        // These endpoints serve as the gate-client pull-resolver targets for
        // `SourceChainResolver` and `DhtResolver`.  The HTTP pipes are real and
        // routed end-to-end.  The conductor query layer is deferred to Phase 11
        // (requires threading HcClient into HttpServer).  For Phase 10 the handlers
        // return honest empty responses so gates degrade gracefully.
        //
        // Phase 11 migration: add `hc_client: Option<Arc<HcClient>>` to HttpServer,
        // wire it via `with_hc_client()`, and replace the stub bodies below with
        // actual `hc_client.call_zome(...)` invocations.

        // GET /db/source-chain/{agent_id}/entries[?filter={filter}]
        if let Some(sc_path) = resource_path.strip_prefix("source-chain/") {
            return self.handle_source_chain_entries(req, method, sc_path).await;
        }

        // GET /db/dht/{entry_hash}
        if let Some(entry_hash) = resource_path.strip_prefix("dht/") {
            return self.handle_dht_entry(req, method, entry_hash).await;
        }

        // GET /db/rea_commitments?action=project-epr&doorwayId={id}
        // Serves the EprRouter source for doorway: fetches all active project-epr
        // commitments for a given doorway from local SQL. The doorway calls this on
        // startup to populate its route table via replace_all(). Without this arm the
        // request falls through to "Unknown database endpoint" (404) and /lamad + /
        // return 404 on every doorway even when the commitments exist in SQL.
        if resource_path == "rea_commitments" {
            return self.handle_db_rea_commitments(req, method, &app_ctx).await;
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(
                r#"{"error": "Unknown database endpoint"}"#,
            )))
            .unwrap())
    }

    /// GET /db/stats - Database statistics
    async fn handle_db_stats(
        &self,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
        let scoped = db::AppScopedDb::new(pool.clone(), &app_ctx.h_app_id);
        Ok(response::from_result(scoped.stats()))
    }

    /// GET /db/rea_commitments?action=project-epr&doorwayId={id}
    ///
    /// Returns all active project-epr `EprProjectionView`s for the given doorway
    /// from local SQL.  The doorway's EprRouter calls this on startup to populate
    /// its `replace_all()` route table.  Accepts only `action=project-epr`; any
    /// other action value is a 400 because no other action produces projectable
    /// URL paths.
    async fn handle_db_rea_commitments(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        #[derive(serde::Deserialize, Default)]
        #[serde(rename_all = "camelCase")]
        struct ReaCommitmentsQuery {
            action: Option<String>,
            doorway_id: Option<String>,
        }

        let params: ReaCommitmentsQuery = req
            .uri()
            .query()
            .and_then(|q| serde_urlencoded::from_str(q).ok())
            .unwrap_or_default();

        let action = params.action.as_deref().unwrap_or("").to_string();
        let doorway_id = params.doorway_id.as_deref().unwrap_or("").to_string();

        self.handle_db_rea_commitments_inner(&action, &doorway_id, ctx)
            .await
    }

    /// GET /db/content - List content, POST /db/content - Create content
    async fn handle_db_content_list(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        // Parse query params via serde — ContentQuery has #[serde(rename_all = "camelCase")]
        // so the compiler enforces camelCase param names (contentType, contentFormat, etc.)
        // NOTE: ContentQuery deliberately does NOT contain require_provenance — that is a
        // server-side parameter so clients cannot disable the gate via URL params.
        let query_str = req.uri().query().unwrap_or("");
        let query: ContentQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

        match method {
            Method::GET => {
                // External read: bypass ContentService::list and call the gated
                // diesel helper directly with require_provenance=true so
                // unpublished rows never leak to external clients.
                let pool = self
                    .db_pool
                    .as_ref()
                    .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
                let mut conn = pool.get().map_err(|e| {
                    StorageError::Internal(format!("Failed to get connection: {}", e))
                })?;
                let app_ctx = db::AppContext::default_lamad();
                match db::content_diesel::list_content(
                    &mut conn,
                    &app_ctx,
                    &query,
                    db::content_diesel::MinTrust::Amber,
                ) {
                    Ok(items) => {
                        // REACH ENFORCEMENT (2026-08-20). Authorization here is
                        // UNCONDITIONAL at every posture. What a declared dev stage may
                        // cheapen is the DEPTH at which a requester's identity and
                        // relationships are verified — never WHETHER the decision is made,
                        // and never in the open direction.
                        //
                        // What this replaced was not a lenient policy, it was the absence
                        // of one: `has_auth = headers.get(AUTHORIZATION).is_some()` treated
                        // header PRESENCE as authorization, so any caller sending
                        // `Authorization: Bearer <anything>` received every restricted row
                        // WITH its content body. Measured live on doorway-alpha
                        // 2026-08-20: anonymous → 90 rows (all commons); the identical
                        // request with the literal token `bogus` → 1000 rows
                        // (familiar 906, private 3, intimate 1). `X-Agent-Id` is
                        // self-asserted, so no token was needed at all.
                        //
                        // The single-item route already did this correctly; this is the
                        // same resolve-then-authorize path applied per row.
                        // EXPLICIT header only — never the ambient local-session
                        // fallback. A hosted pod mints an active session for its
                        // own human and the doorway omits `X-Agent-Cid` for an
                        // unverified caller, so the cascade form would resolve an
                        // ANONYMOUS request as the node's human and serve that
                        // human's reach. See `extract_agent_cid_explicit`.
                        let requester = crate::api::account::extract_agent_cid_explicit(&req)
                            .and_then(|idv| {
                                crate::db::humans::get_human_by_id(&mut conn, &idv)
                                    .ok()
                                    .flatten()
                                    .or_else(|| {
                                        crate::db::humans::get_human_by_agent_key(&mut conn, &idv)
                                            .ok()
                                            .flatten()
                                    })
                            });
                        let community_idx = crate::epr_service::reach_level_index("community");
                        let reach_gate = crate::epr_service::EprService::new(
                            None,
                            None,
                            None,
                            crate::p2p::trust_cache::PeerTrustCache::new(),
                        )
                        .with_memo_store(self.memo_store.clone());
                        let mut refused: usize = 0;
                        let views: Vec<ContentView> = items
                            .into_iter()
                            .map(Into::into)
                            .filter(|v: &ContentView| {
                                let idx = crate::epr_service::reach_level_index(&v.reach);
                                // commons/public: no identity needed.
                                if idx == 0 {
                                    return true;
                                }
                                // Every restricted tier needs a RESOLVED requester.
                                // Deny-by-default when the caller cannot be resolved —
                                // this is the fail-closed half header-presence skipped.
                                let Some(ref human) = requester else {
                                    refused += 1;
                                    return false;
                                };
                                // community: a resolved identity IS the gate (mirrors the
                                // single-item route, which authorizes only above community).
                                if idx <= community_idx {
                                    return true;
                                }
                                match reach_gate.authorize_reach_for_human_with_own_trust(
                                    &mut conn, &app_ctx, &v.reach, human, &v.id,
                                ) {
                                    Ok(()) => true,
                                    Err(_) => {
                                        refused += 1;
                                        false
                                    }
                                }
                            })
                            .collect();
                        if refused > 0 {
                            tracing::debug!(
                                refused,
                                resolved_requester = requester.is_some(),
                                "reach enforcement filtered rows from /db/content listing"
                            );
                        }
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                            "limit": query.limit,
                            "offset": query.offset,
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
                // Currently null for direct storage writes. Backfill needed for pre-coherence data.
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();

                let input_view: CreateContentInputView = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                // Capture manifest-relevant data before consuming input_view
                let manifest_data = (
                    input_view.id.clone(),
                    input_view.blob_hash.clone(),
                    input_view.blob_cid.clone(),
                    input_view.content_format.clone().unwrap_or_default(),
                    input_view
                        .reach
                        .clone()
                        .unwrap_or_else(|| "commons".to_string()),
                );

                let input: db::content_diesel::CreateContentInput = input_view.into();
                let result = services.content.create(input);

                // EPR Head publishing is handled by the drain loop (p2p/mod.rs::drain_publish_queue),
                // which is the sole writer of p2p_published_at. New content becomes visible to
                // external reads within ~p2p::DRAIN_INTERVAL_SECS seconds of the POST returning.

                // Capture distribution data before manifest recording consumes manifest_data
                #[cfg(feature = "p2p")]
                let distribution_data = (manifest_data.0.clone(), manifest_data.1.clone());

                // Record shard manifest if content has a blob
                if result.is_ok() && manifest_data.1.is_some() {
                    if let Some(ref pool) = self.db_pool {
                        let blob_store = self.blob_store.clone();
                        let pool = pool.clone();
                        let (content_id, blob_hash, blob_cid, content_format, reach) =
                            manifest_data;
                        let blob_hash = blob_hash.unwrap(); // Safe: checked above
                        tokio::spawn(async move {
                            if let Ok(data) = blob_store.get(&blob_hash).await {
                                if let Ok(mut conn) = pool.get() {
                                    match crate::db::shard_manifests::record_manifest_from_bytes(
                                        &mut conn,
                                        &content_id,
                                        "lamad",
                                        &blob_hash,
                                        blob_cid.as_deref(),
                                        &data,
                                        &content_format,
                                        &reach,
                                    ) {
                                        Ok(row) => tracing::debug!(
                                            content_id = %content_id,
                                            encoding = %row.encoding,
                                            "Recorded shard manifest"
                                        ),
                                        Err(e) => tracing::warn!(
                                            content_id = %content_id,
                                            error = %e,
                                            "Failed to record shard manifest"
                                        ),
                                    }
                                }
                            }
                        });
                    }
                }

                // Auto-distribute shards to peers
                #[cfg(feature = "p2p")]
                if result.is_ok() {
                    if let (Some(ref handle), Some(ref pool)) = (&self.p2p_handle, &self.db_pool) {
                        if let (ref content_id, Some(ref blob_hash)) = distribution_data {
                            let handle = handle.clone();
                            let pool = pool.clone();
                            let content_id = content_id.clone();
                            let blob_store = self.blob_store.clone();
                            let blob_hash = blob_hash.clone();
                            tokio::spawn(async move {
                                if let Ok(data) = blob_store.get(&blob_hash).await {
                                    match handle
                                        .distribute_shards(&content_id, &data, &pool, "lamad")
                                        .await
                                    {
                                        Ok(n) => tracing::info!(
                                            content_id = %content_id,
                                            shards = n,
                                            "Shard distribution complete"
                                        ),
                                        Err(e) => tracing::warn!(
                                            content_id = %content_id,
                                            error = %e,
                                            "Shard distribution failed"
                                        ),
                                    }
                                }
                            });
                        }
                    }
                }

                Ok(response::from_create_result(result))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/content/bulk - Bulk create content
    async fn handle_db_content_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let body_bytes = body.to_bytes();

        let input_views: Vec<CreateContentInputView> = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        // Capture manifest-relevant data before input_views are consumed
        let manifest_inputs: Vec<_> = input_views
            .iter()
            .map(|v| {
                (
                    v.id.clone(),
                    v.blob_hash.clone(),
                    v.blob_cid.clone(),
                    v.content_format.clone().unwrap_or_default(),
                    v.reach.clone().unwrap_or_else(|| "commons".to_string()),
                )
            })
            .collect();

        let items: Vec<db::content_diesel::CreateContentInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        let count = items.len();

        // Pause P2P sync during bulk content creation to prevent memory pressure.
        // The guard resumes sync automatically when dropped.
        #[cfg(feature = "p2p")]
        let _sync_guard = if count >= 50 {
            self.p2p_handle
                .as_ref()
                .map(|h| h.pause_sync(&format!("bulk content create ({} items)", count)))
        } else {
            None
        };

        info!(count = count, "Bulk creating content");

        match services.content.bulk_create(items) {
            Ok(result) => {
                // EPR Head publishing is handled by the drain loop
                // (p2p/mod.rs::drain_publish_queue), which is the sole writer of
                // p2p_published_at. New content becomes visible to external reads
                // within ~p2p::DRAIN_INTERVAL_SECS seconds of the POST returning.

                // Capture distribution data before manifest_inputs is consumed
                #[cfg(feature = "p2p")]
                let distribution_items: Vec<(String, String)> = manifest_inputs
                    .iter()
                    .filter_map(|(id, bh, _, _, _)| bh.as_ref().map(|h| (id.clone(), h.clone())))
                    .collect();

                // Record shard manifests for items with blobs
                if let Some(ref pool) = self.db_pool {
                    let blob_store = self.blob_store.clone();
                    let pool = pool.clone();
                    let items_with_blobs: Vec<_> = manifest_inputs
                        .into_iter()
                        .filter(|(_, blob_hash, _, _, _)| blob_hash.is_some())
                        .collect();
                    if !items_with_blobs.is_empty() {
                        tokio::spawn(async move {
                            for (content_id, blob_hash, blob_cid, content_format, reach) in
                                items_with_blobs
                            {
                                let blob_hash = blob_hash.unwrap(); // Safe: filtered above
                                if let Ok(data) = blob_store.get(&blob_hash).await {
                                    let encoder = crate::sharding::ShardEncoder::new(
                                        crate::sharding::ShardConfig::default(),
                                    );
                                    let manifest = match encoder.create_manifest(
                                        &data,
                                        &content_format,
                                        &reach,
                                    ) {
                                        Ok(m) => m,
                                        Err(e) => {
                                            tracing::warn!(
                                                content_id = %content_id,
                                                error = %e,
                                                "shard manifest encode failed; skipping persistence"
                                            );
                                            continue;
                                        }
                                    };
                                    let shard_hashes_json =
                                        serde_json::to_string(&manifest.shard_hashes)
                                            .unwrap_or_else(|_| "[]".to_string());
                                    if let Ok(mut conn) = pool.get() {
                                        let new_manifest = crate::db::models::NewShardManifest {
                                            content_id: &content_id,
                                            h_app_id: "lamad",
                                            blob_hash: &blob_hash,
                                            blob_cid: blob_cid.as_deref(),
                                            encoding: &manifest.encoding,
                                            data_shard_count: manifest.data_shards as i32,
                                            parity_shard_count: (manifest.total_shards
                                                - manifest.data_shards)
                                                as i32,
                                            shard_hashes_json: &shard_hashes_json,
                                            total_size_bytes: manifest.total_size as i64,
                                            shard_size_bytes: manifest.shard_size as i64,
                                            mime_type: &manifest.mime_type,
                                            reach: &reach,
                                        };
                                        if let Err(e) = crate::db::shard_manifests::upsert_manifest(
                                            &mut conn,
                                            &new_manifest,
                                        ) {
                                            tracing::warn!(
                                                content_id = %content_id,
                                                error = %e,
                                                "Failed to record shard manifest"
                                            );
                                        }
                                    }
                                }
                            }
                            tracing::debug!("Bulk shard manifest recording complete");
                        });
                    }
                }

                // Auto-distribute shards to peers
                #[cfg(feature = "p2p")]
                if let (Some(ref handle), Some(ref pool)) = (&self.p2p_handle, &self.db_pool) {
                    if !distribution_items.is_empty() {
                        let handle = handle.clone();
                        let pool = pool.clone();
                        let blob_store = self.blob_store.clone();
                        tokio::spawn(async move {
                            for (content_id, blob_hash) in distribution_items {
                                if let Ok(data) = blob_store.get(&blob_hash).await {
                                    let _ = handle
                                        .distribute_shards(&content_id, &data, &pool, "lamad")
                                        .await;
                                }
                            }
                            tracing::info!("Bulk shard distribution complete");
                        });
                    }
                }

                Ok(response::ok_with_schema_info(&result))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/content/{cid}/schedule - Entity-nested schedule convenience
    async fn handle_content_schedule(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_id: &str,
        app_ctx: &db::AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                match db::schedules::get_schedule(&mut conn, app_ctx, "content", content_id) {
                    Ok(schedule) => Ok(response::ok(&ScheduleView::from(schedule))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                use http_body_util::BodyExt;
                let body_bytes = req
                    .into_body()
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                let input_view: CreateScheduleInputView = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                // Override entity_type/entity_id from URL path
                let input = db::schedules::CreateScheduleInput {
                    entity_type: "content".to_string(),
                    entity_id: content_id.to_string(),
                    scheduled_at: input_view.scheduled_at,
                    expires_at: input_view.expires_at,
                    rrule: input_view.rrule,
                };

                match db::schedules::create_schedule(&mut conn, app_ctx, input) {
                    Ok(schedule) => Ok(response::created(&ScheduleView::from(schedule))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/POST /db/content/{id}/head — the notary-HEAD authority surface
    /// (HEAD-election, Plan C3 / notary-authority Leg 3).
    ///
    /// GET returns the notary's current HEAD for a content id (200), or 404 when
    /// no row exists / the row carries no notary answer. POST is the authority
    /// gate: only the authoring agent may declare (move) the HEAD. The POST order
    /// is LOAD-BEARING — authentication is checked BEFORE any existence lookup so
    /// a non-author probe against a nonexistent id gets 401/403, never a 404 that
    /// would leak existence or mis-signal "not the author" as "not found".
    /// GET /db/p2p/conductor-diagnostics — Cat-C operational read of the
    /// embedded conductor's network view (dht-unity plan T3).
    ///
    /// Returns the conductor's peer store (`agent_info` — every AgentInfoSigned
    /// it currently holds, projected to url/space/timestamps) plus live
    /// transport stats (`dump_network_stats`: open connections, blocked
    /// message counts). Pass `?include=metrics` to add per-DNA
    /// `dump_network_metrics` (fetch queue + peers_on_backoff + gossip round
    /// summaries) — heavier, so opt-in.
    ///
    /// Trust model: everything here is either already public via the shared
    /// bootstrap table (agent infos) or node-local operational state the node
    /// itself owns (its own connection list). No content, no identity secrets.
    async fn handle_conductor_diagnostics(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }
        let Some(admin) = &self.admin_websocket else {
            return Ok(response::service_unavailable(
                "conductor diagnostics unavailable: no embedded conductor admin connection",
            ));
        };

        let include_metrics = req
            .uri()
            .query()
            .map(|q| q.split('&').any(|kv| kv == "include=metrics"))
            .unwrap_or(false);

        let agent_infos = match admin.agent_info(None).await {
            Ok(infos) => infos,
            Err(e) => {
                return Ok(response::service_unavailable(&format!(
                    "conductor agent_info failed: {e}"
                )));
            }
        };
        let agents: Vec<serde_json::Value> =
            agent_infos.iter().map(|s| project_agent_info(s)).collect();

        // Transport stats: best-effort — a failure here should not hide the
        // peer store (the more load-bearing half of the diagnostic).
        let transport_stats = match admin.dump_network_stats().await {
            Ok(stats) => serde_json::to_value(&stats)
                .unwrap_or_else(|e| serde_json::json!({ "serializeError": e.to_string() })),
            Err(e) => serde_json::json!({ "error": e.to_string() }),
        };

        let network_metrics = if include_metrics {
            match admin.dump_network_metrics(None, false).await {
                Ok(metrics) => {
                    let by_dna: serde_json::Map<String, serde_json::Value> = metrics
                        .into_iter()
                        .map(|(dna, m)| {
                            (
                                dna.to_string(),
                                serde_json::to_value(&m).unwrap_or_else(
                                    |e| serde_json::json!({ "serializeError": e.to_string() }),
                                ),
                            )
                        })
                        .collect();
                    Some(serde_json::Value::Object(by_dna))
                }
                Err(e) => Some(serde_json::json!({ "error": e.to_string() })),
            }
        } else {
            None
        };

        let mut body = serde_json::json!({
            "agentCount": agents.len(),
            "agents": agents,
            "transportStats": transport_stats,
        });
        if let Some(metrics) = network_metrics {
            body["networkMetrics"] = metrics;
        }
        Ok(response::ok(&body))
    }

    /// GET /db/identity/did/{did} — resolve a `did:elohim` identifier to its
    /// assembled DID 1.1 document (DID bridge spec §3.4). Cat-C operational
    /// projection (assembled per request, never stored — P1); doorway
    /// auto-forwards it (declared in `build_manifest`). Assembly conforms to the
    /// did-bridge `ElohimIdentityStore` contract — storage supplies the
    /// agent-key / humans / transport joins; the crate owns the codec + framing.
    ///
    /// Error shape is the standard DID resolution result (`didResolutionMetadata.
    /// error` ∈ {`invalidDid`, `methodNotSupported`, `notFound`, `internalError`}):
    /// syntactically invalid or non-`elohim` DIDs return 400; unknown agents 404.
    async fn handle_did_resolution(
        &self,
        method: Method,
        did_str: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        use did_bridge::{DidResolutionError, DidResolutionResult, DidResolver, ElohimResolver};
        use did_types::Did;

        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let Some(pool) = self.db_pool.clone() else {
            return Ok(response::service_unavailable(
                "identity resolution unavailable: no database pool",
            ));
        };

        // Syntax gate → invalidDid (400).
        let did = match Did::parse(did_str) {
            Ok(d) => d,
            Err(e) => {
                let err = DidResolutionError::InvalidDid(e.to_string());
                return Ok(response::json_response(
                    StatusCode::BAD_REQUEST,
                    &DidResolutionResult::from_error(&err),
                ));
            }
        };

        // Only did:elohim is served here → methodNotSupported (400).
        if did.method() != "elohim" {
            let err = DidResolutionError::MethodNotSupported(did.method().to_string());
            return Ok(response::json_response(
                StatusCode::BAD_REQUEST,
                &DidResolutionResult::from_error(&err),
            ));
        }

        // Self-identity seams. The own conductor cell key (`uhCAk…`) marks which
        // agent_cid is *self*; `self_cid` is a TRANSPORT id (libp2p PeerId / iroh
        // NodeId), emitted only as alsoKnownAs — never as the self agent_cid
        // (identity-namespace hazard; see the store module + crate CLAUDE.md).
        let self_agent_cid = self
            .hc_registry
            .as_ref()
            .and_then(|r| r.lamad_client())
            .map(|hc| hc.agent_key_uhcak())
            .filter(|k| !k.is_empty());
        let self_transport_ids = if self.self_cid.is_empty() {
            Vec::new()
        } else {
            vec![self.self_cid.clone()]
        };
        // No canonical public base URL is configured in storage today, so the
        // SELF profile service stays dormant (an absolute endpoint or nothing).
        let public_base_url = None;

        let store = crate::services::did_identity_store::DidIdentityStore::new(
            pool,
            self_agent_cid,
            self_transport_ids,
            public_base_url,
        );
        let resolver = ElohimResolver::new(store);

        match resolver.resolve(&did).await {
            Ok(result) => Ok(response::ok(&result)),
            Err(err) => {
                // Exhaustive by design — no `_` arm. The compiler catching a new
                // resolution error here is the point: an unmapped variant would
                // otherwise be silently served as somebody else's status code.
                let status = match err {
                    DidResolutionError::NotFound(_) => StatusCode::NOT_FOUND,
                    DidResolutionError::InvalidDid(_)
                    | DidResolutionError::MethodNotSupported(_) => StatusCode::BAD_REQUEST,
                    DidResolutionError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
                    // A FETCHED document described a different subject. Not the
                    // requester's fault and not ours — the upstream responder
                    // answered badly, which is what 502 means.
                    DidResolutionError::SubjectMismatch { .. } => StatusCode::BAD_GATEWAY,
                    // The head could not be determined (undelivered DHT record,
                    // timed-out read). Fail-closed and RETRYABLE — 503, never 404:
                    // "we could not establish it" must not be served as "it does
                    // not exist".
                    DidResolutionError::IdentityHeadUnresolvable(_) => {
                        StatusCode::SERVICE_UNAVAILABLE
                    }
                    // A head resolved but is unusable as declared (e.g. an empty
                    // controller set, which would read as implicit self-control).
                    // That is a substrate-side defect, not a client error.
                    DidResolutionError::IdentityHeadMalformed(_) => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };
                Ok(response::json_response(
                    status,
                    &DidResolutionResult::from_error(&err),
                ))
            }
        }
    }

    async fn handle_content_head(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_id: &str,
        app_ctx: &db::AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        use http_body_util::BodyExt;

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                // Internal read (Invisible): the notary answer is authority data —
                // surface it regardless of serving trust tier, then let `trust`
                // label it honestly.
                match db::content_diesel::get_content_with_tags(
                    &mut conn,
                    app_ctx,
                    content_id,
                    db::content_diesel::MinTrust::Invisible,
                )? {
                    None => Ok(response::not_found(&format!(
                        "Content not found: {}",
                        content_id
                    ))),
                    Some(cwt) => match crate::views::content_head_view_from_content(&cwt.content) {
                        Some(view) => Ok(response::ok(&view)),
                        // Honest: the row exists but the notary holds no HEAD for it.
                        None => Ok(response::not_found(
                            "no notarized head declared for this content",
                        )),
                    },
                }
            }
            Method::POST => {
                // Split out so tests can drive the auth/admission ordering without
                // constructing a `Request<Incoming>` (which cannot be built outside
                // a real hyper connection — see `test_handle_content_head_post_raw`).
                // Headers are captured before the body is consumed, preserving the
                // original ordering guarantee ("the header must run while `req` is
                // still intact, before the body consumes it").
                let headers = req.headers().clone();
                let body_bytes = req
                    .into_body()
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                self.handle_content_head_post_bytes(&headers, &body_bytes, content_id, app_ctx)
                    .await
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST body for [`Self::handle_content_head`] — the notary HEAD-declare
    /// write. Split out (headers + raw body bytes rather than `Request<Incoming>`)
    /// so it is directly unit-testable: `Incoming` cannot be constructed outside a
    /// real hyper connection, the same reason `handle_create_pin_bytes` exists.
    ///
    /// Ordering is the load-bearing contract this function exists to pin:
    /// (a)/(a2) authentication + authorship resolution run FIRST, unconditionally
    /// — a missing credential (401) or a resolved-but-wrong author (403) is
    /// refused before any admission/backpressure decision. Only once authorship
    /// is CONFIRMED does (d2) apply write-admission backpressure — a confirmed
    /// author can still see the catching-up 503 if the write pool is genuinely
    /// exhausted, but a non-author can never be masked behind that shed. This is
    /// the fix for the bug where a non-author's move raced the admission shed and
    /// won with a 503 instead of a 401/403 authority refusal (see
    /// `head_declare_authority_precedes_catching_up_shed_tests` below and
    /// `genesis/a2o/features/dataplane/notary-authority.feature`).
    async fn handle_content_head_post_bytes(
        &self,
        headers: &hyper::HeaderMap,
        body_bytes: &[u8],
        content_id: &str,
        app_ctx: &db::AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        /// POST body: the explicit action to pin as HEAD. Absent (or absent body)
        /// asks the coordinator to resolve the author's latest committed action.
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct DeclareHeadBody {
            #[serde(default)]
            head_action_hash: Option<String>,
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        // (a) AUTH FIRST — before any existence check, and (per the exemption in
        // `is_head_declare_write` / `handle_request`) before any admission-shed
        // decision. Caller identity is derived EXCLUSIVELY from the request: the
        // `X-Agent-Cid` header (doorway injects it only after validating a
        // bearer; Tauri-direct callers set it themselves). We deliberately do NOT
        // use `extract_agent_cid`'s `local_sessions` fallback here — genesis pods
        // mint an ambient active session at boot (genesis_self_heal_identity,
        // GENESIS_SELF_HEAL_IDENTITY=1 on the alpha manifests), so an anonymous
        // probe would be silently authenticated on those pods and fall through to
        // a 404 instead of 401. A notarization write requires a request-borne
        // credential: no header → 401, before any DB/conductor access.
        let caller_cid = match headers
            .get("X-Agent-Cid")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
        {
            Some(cid) => cid,
            None => {
                return Ok(response::json_response(
                    StatusCode::UNAUTHORIZED,
                    &serde_json::json!({
                        "error": "authentication required: only the author may move a declared HEAD"
                    }),
                ));
            }
        };

        // (a2) Resolve the caller value to an agent key BEFORE any authorship
        // comparison. `X-Agent-Cid` may carry either a Holochain agent key
        // (uhCA…, Tauri-direct) OR a human slug (doorway forwards
        // `claims.human_id`, e.g. "human-matthew-manager"). These are DISTINCT
        // identity namespaces — a raw slug-vs-agent-key string compare always
        // fails (see "Identity & Transport-Identity Coherence" in this crate's
        // CLAUDE.md: never cross-namespace string-compare; resolve slug →
        // agent_cid first). Resolve a slug to its `humans.agent_pub_key`; fail
        // closed (403) on a write when the caller cannot be proven to own an
        // agent key.
        let caller_agent_key = if caller_cid.starts_with("uhCA") {
            caller_cid.clone()
        } else {
            match crate::db::humans::get_human_by_id(&mut conn, &caller_cid)?
                .and_then(|h| h.agent_pub_key)
            {
                Some(key) => key,
                None => {
                    return Ok(response::forbidden(&serde_json::json!({
                        "error": "caller identity cannot be resolved to an agent key; cannot prove authorship"
                    })));
                }
            }
        };

        // (b) Parse body { headActionHash?: String } — absent body → None.
        let head_action_hash: Option<String> = if body_bytes.is_empty() {
            None
        } else {
            match serde_json::from_slice::<DeclareHeadBody>(body_bytes) {
                Ok(b) => b.head_action_hash,
                Err(e) => return Ok(response::bad_request(&format!("Invalid JSON: {}", e))),
            }
        };

        // (c) Resolve the lamad conductor bridge.
        let hc = match self.hc_registry.as_ref().and_then(|r| r.lamad_client()) {
            Some(hc) => hc,
            None => {
                return Ok(response::service_unavailable(
                    "Conductor bridge unavailable; cannot declare content head",
                ));
            }
        };

        // (d) Resolve the notary HEAD, then author-gate. The comparison is
        // between `caller_agent_key` (resolved slug→agent_cid in (a2)) and
        // `head.author` — BOTH now Holochain agent keys (uhCAk…, same
        // namespace), so the string compare is legal (the earlier "caller_cid
        // and head.author are both uhCAk" claim was wrong: caller_cid can be a
        // doorway human slug). A None author fails closed.
        let head =
            match crate::services::conductor_writes::call_resolve_content_head(&hc, content_id)
                .await?
            {
                Some(h) => h,
                None => {
                    return Ok(response::not_found(
                        "content has no version chain on this notary",
                    ));
                }
            };
        let is_author = head.author.as_ref().map(|a| a.as_str()) == Some(caller_agent_key.as_str());
        if !is_author {
            return Ok(response::forbidden(&serde_json::json!({
                "error": "caller is not the author of this content"
            })));
        }

        // (d2) NOW apply write-admission backpressure — reached only by a caller
        // whose authorship has already been confirmed. This route is carved out
        // of the top-of-dispatch admission gate (`is_head_declare_write` in
        // `handle_request`) specifically so an unauthorized caller is never shed
        // before being refused; a confirmed AUTHOR can still be shed here with the
        // same catching-up 503 the blanket gate would have produced, if the write
        // pool is genuinely exhausted.
        let _write_permit = match self.request_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                let available = self.request_semaphore.available_permits();
                warn!(
                    target: "upstream_shed",
                    counter = "storage_admission_shed_total",
                    path = "/db/content/{id}/head",
                    pool = "write",
                    available,
                    "storage request admission at ceiling — shedding authorized HEAD-declare write (503 + Retry-After)"
                );
                return Ok(crate::services::response::too_many_requests_with_retry(
                    STORAGE_SHED_RETRY_AFTER_SECS,
                    available,
                ));
            }
        };

        // (e) Declare / advance the HEAD via the conductor. Coordinator error
        // substrings map to the right HTTP class.
        let declared = match crate::services::conductor_writes::call_declare_content_head(
            &hc,
            content_id,
            head_action_hash,
        )
        .await
        {
            Ok(h) => h,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not the author") {
                    return Ok(response::forbidden(&serde_json::json!({ "error": msg })));
                } else if msg.contains("not in the version chain") {
                    return Ok(response::bad_request(&msg));
                } else {
                    return Ok(response::json_response(
                        StatusCode::BAD_GATEWAY,
                        &serde_json::json!({ "error": msg }),
                    ));
                }
            }
        };

        // Eager-stamp the projection so the local read is coherent immediately
        // (not only once the async projection signal lands) — mirrors
        // update_via_conductor. Field mapping mirrors the ContentCommitted
        // projection arm (rea_projection.rs).
        let content = declared.content;
        let size_i32 = content
            .content_size_bytes
            .map(|n| i32::try_from(n).unwrap_or(i32::MAX));
        let patch = db::content_diesel::ContentProjectionPatch {
            blob_cid: content.blob_cid,
            content_size_bytes: size_i32,
            title: Some(content.title),
            description: Some(content.description),
            content_type: Some(content.content_type),
            content_format: Some(content.content_format),
            reach: Some(content.reach),
            metadata_json: Some(content.metadata_json),
        };
        db::content_diesel::stamp_declared_head(
            &mut conn,
            app_ctx,
            content_id,
            declared.head_action_hash.as_str(),
            Some(declared.declared_at),
            Some(patch),
        )?;

        // 200 with the fresh HEAD answer, re-read from the stamped row.
        match db::content_diesel::get_content_with_tags(
            &mut conn,
            app_ctx,
            content_id,
            db::content_diesel::MinTrust::Invisible,
        )? {
            Some(cwt) => match crate::views::content_head_view_from_content(&cwt.content) {
                Some(view) => Ok(response::ok(&view)),
                None => Ok(response::not_found(
                    "no notarized head declared for this content",
                )),
            },
            None => Ok(response::not_found(&format!(
                "Content not found: {}",
                content_id
            ))),
        }
    }

    /// POST /db/content/{id}/canonical-head — the CROSS-ROOT canonical-head
    /// declaration surface (notary-authority convergence, Model B / Tier-1
    /// STAGING tier). Converges genuinely-independent roots (different agents,
    /// no supersedes edge between their versions) onto ONE head every
    /// federation peer resolves, via `content_store::declare_canonical_content_head`.
    ///
    /// Unlike [`Self::handle_content_head`]'s POST (single-author,
    /// chain-membership-gated), there is deliberately NO author gate here — the
    /// zome-side staging tier is god-mode by design (any declarer may set/replace
    /// a STAGING canonical; it can never clobber or impersonate an EARNED one,
    /// enforced zome-side). Edge auth is this route's `.auth_required()`
    /// registration alone.
    async fn handle_content_canonical_head(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_id: &str,
        app_ctx: &db::AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        use http_body_util::BodyExt;

        /// POST body: the explicit action to declare as the cross-root
        /// canonical head. REQUIRED — a cross-root declaration always names
        /// its explicit target (mirrors the zome's `DeclareCanonicalHeadInput`).
        ///
        /// `record` is the OPTIONAL declare-carries-Record payload: the
        /// standard-base64 encoding of the target's serialized `Record`, as
        /// served by `GET /db/content/{id}/head-record` on the AUTHORING peer.
        /// Supplying it lets this conductor declare a head it cannot retrieve
        /// itself (the full-arc gossip-gap case); omitting it is the classic
        /// behaviour, unchanged.
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct DeclareCanonicalHeadBody {
            #[serde(default)]
            head_action_hash: Option<String>,
            #[serde(default)]
            record: Option<String>,
        }

        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        // (a) Parse body { headActionHash: String, record?: base64 } —
        // headActionHash missing or empty → 400. `record` is optional; a
        // malformed base64 is a caller error (400) rather than a silent drop,
        // because silently declaring without the carried record would surface
        // as the confusing "not retrievable" refusal the carry exists to cure.
        let body_bytes = req
            .into_body()
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        let (head_action_hash, carried_record): (String, Option<Vec<u8>>) =
            if body_bytes.is_empty() {
                return Ok(response::bad_request(
                    "headActionHash is required to declare a canonical content head",
                ));
            } else {
                match serde_json::from_slice::<DeclareCanonicalHeadBody>(&body_bytes) {
                    Ok(b) => {
                        let head = match b.head_action_hash {
                            Some(h) if !h.trim().is_empty() => h,
                            _ => return Ok(response::bad_request(
                                "headActionHash is required to declare a canonical content head",
                            )),
                        };
                        let carried =
                            match b.record.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                                Some(b64) => {
                                    use base64::{engine::general_purpose::STANDARD, Engine as _};
                                    match STANDARD.decode(b64) {
                                        Ok(bytes) => Some(bytes),
                                        Err(e) => {
                                            return Ok(response::bad_request(&format!(
                                                "record must be standard base64 of a serialized \
                                         Record: {e}"
                                            )))
                                        }
                                    }
                                }
                                None => None,
                            };
                        (head, carried)
                    }
                    Err(e) => return Ok(response::bad_request(&format!("Invalid JSON: {}", e))),
                }
            };

        // (b) Resolve the lamad conductor bridge.
        let hc = match self.hc_registry.as_ref().and_then(|r| r.lamad_client()) {
            Some(hc) => hc,
            None => {
                return Ok(response::service_unavailable(
                    "Conductor bridge unavailable; cannot declare canonical content head",
                ));
            }
        };

        // (c) Declare the cross-root canonical HEAD via the conductor. Errors
        // (including a fn-not-found-class error from a pre-cure coordinator —
        // the hot-swap probe signal callers key on) are surfaced verbatim.
        let declared = match crate::services::conductor_writes::call_declare_canonical_content_head(
            &hc,
            content_id,
            head_action_hash,
            carried_record,
            // ADOPT-BEFORE-AUTHOR is NOT offered on the HTTP declare route, and
            // that is deliberate rather than an omission. This route is an
            // explicit operator/deploy act naming a target; the residual the flag
            // exists for is the AUTOMATED sweep's both-sides-missing class, where
            // no human is choosing the target. Exposing the bypass here would let
            // an HTTP caller declare a head for an id the node has never held —
            // widening the seam well past the residual, with none of the sweep's
            // own gates (backoff, candidacy ledger, declare-storm) in front of it.
            false,
        )
        .await
        {
            Ok(h) => h,
            Err(e) => {
                let msg = e.to_string();
                return Ok(response::json_response(
                    StatusCode::BAD_GATEWAY,
                    &serde_json::json!({ "error": msg }),
                ));
            }
        };

        // Eager-stamp the projection so the local read is coherent immediately
        // (not only once the async projection signal lands) — mirrors
        // handle_content_head's declare arm / update_via_conductor. Field
        // mapping mirrors the ContentCommitted projection arm (rea_projection.rs).
        let content = declared.content;
        let size_i32 = content
            .content_size_bytes
            .map(|n| i32::try_from(n).unwrap_or(i32::MAX));
        let patch = db::content_diesel::ContentProjectionPatch {
            blob_cid: content.blob_cid,
            content_size_bytes: size_i32,
            title: Some(content.title),
            description: Some(content.description),
            content_type: Some(content.content_type),
            content_format: Some(content.content_format),
            reach: Some(content.reach),
            metadata_json: Some(content.metadata_json),
        };
        db::content_diesel::stamp_declared_head(
            &mut conn,
            app_ctx,
            content_id,
            declared.head_action_hash.as_str(),
            Some(declared.declared_at),
            Some(patch),
        )?;

        // Pattern Z.B.1 bridge — the SAME refresh the direct-PATCH arm applies
        // after ContentService::update. `stamp_declared_head` mirrors blob_cid
        // into blob_hash on the row, but the `/apps/{slug}/{file}` fast-path
        // answers from the in-memory `slug_index`, which is only ever built by
        // `load_slug_index()` (at boot, and after a direct PATCH). Without this
        // refresh a peer that ADOPTS a declared head reports the new head on
        // GET /db/content/{id}/head while still serving the OLD bundle bytes
        // until its next restart — converged on the wire, divergent in what the
        // human actually sees. Adopting the declaration is exactly the moment
        // that split must not open, so refresh here too.
        drop(conn);
        self.load_slug_index().await;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        // 200 with the fresh HEAD answer, re-read from the stamped row — same
        // response shape as the single-author declare handler (ContentHeadView).
        match db::content_diesel::get_content_with_tags(
            &mut conn,
            app_ctx,
            content_id,
            db::content_diesel::MinTrust::Invisible,
        )? {
            Some(cwt) => match crate::views::content_head_view_from_content(&cwt.content) {
                Some(view) => Ok(response::ok(&view)),
                None => Ok(response::not_found(
                    "no notarized head declared for this content",
                )),
            },
            None => Ok(response::not_found(&format!(
                "Content not found: {}",
                content_id
            ))),
        }
    }

    /// GET /db/content/{id}/head-record — serve THIS peer's local DHT `Record`
    /// for the id's head action (the SOURCE half of declare-carries-Record).
    ///
    /// The authoring peer holds the head record; a peer with a full-arc gossip
    /// gap cannot fetch it from the network (the `get` cascade short-circuits
    /// at authority, so absence is terminal, not slow). This route lets a
    /// coordinator-level caller ask the peer that HAS the record for the bytes,
    /// then carry them to the peer that does not, which re-verifies them in
    /// wasm before honoring the declaration.
    ///
    /// Response `200 { headActionHash, record }` where `record` is standard
    /// base64 of the serialized `Record` — an OPAQUE token to every layer
    /// between the two conductors. `404` when this peer has no head for the id
    /// or cannot retrieve the head action itself (honest absence: the caller
    /// then declares without a carried record, exactly as before).
    async fn handle_content_head_record(
        &self,
        method: Method,
        content_id: &str,
        app_ctx: &db::AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        // (a) Resolve the head action hash from the projection — the SAME
        // answer GET /db/content/{id}/head gives, so the carried record always
        // matches the hash the caller read from this peer a moment earlier.
        let cwt = match db::content_diesel::get_content_with_tags(
            &mut conn,
            app_ctx,
            content_id,
            db::content_diesel::MinTrust::Invisible,
        )? {
            None => {
                return Ok(response::not_found(&format!(
                    "Content not found: {}",
                    content_id
                )))
            }
            Some(cwt) => cwt,
        };

        // REACH GATE — mirrors the P2P responder's gate in
        // `p2p::view_federation::build_content_head_record_payload`: a
        // non-distribution-safe reach is answered as ABSENCE — byte-identical
        // to an unknown id — so this cross-peer surface can never be used to
        // probe whether a scoped id exists on this HTTP-reachable peer either.
        if !db::content_diesel::is_distribution_safe_reach(&cwt.content.reach) {
            tracing::debug!(
                target: "elohim_storage::http",
                content_id = %content_id,
                "GET .../head-record: scoped-tier reach — answering as absent (no cross-peer surface)"
            );
            return Ok(response::not_found(&format!(
                "Content not found: {}",
                content_id
            )));
        }

        let head_action_hash = match crate::views::content_head_view_from_content(&cwt.content) {
            Some(view) => view.head_action_hash,
            None => {
                return Ok(response::not_found(
                    "no notarized head declared for this content",
                ))
            }
        };
        drop(conn);

        // (b) Resolve the lamad conductor bridge.
        let hc = match self.hc_registry.as_ref().and_then(|r| r.lamad_client()) {
            Some(hc) => hc,
            None => {
                return Ok(response::service_unavailable(
                    "Conductor bridge unavailable; cannot serve a head record",
                ));
            }
        };

        // (c) Ask the conductor for the record. A conductor-level error
        // (including fn-not-found on a pre-cure coordinator) surfaces verbatim
        // as 502 so the caller can distinguish "this peer is too old to serve
        // records" from "this peer does not hold the record".
        let served = match crate::services::conductor_writes::call_get_record_for_action(
            &hc,
            &head_action_hash,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return Ok(response::json_response(
                    StatusCode::BAD_GATEWAY,
                    &serde_json::json!({ "error": e.to_string() }),
                ));
            }
        };

        match served {
            Some(carried) => {
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                Ok(response::ok(&serde_json::json!({
                    "headActionHash": carried.action_hash,
                    "record": STANDARD.encode(&carried.record),
                })))
            }
            None => Ok(response::not_found(
                "this peer cannot retrieve the head action; no record to serve",
            )),
        }
    }

    /// GET/DELETE /db/content/{id} - Get or delete content by ID
    async fn handle_db_content_by_id(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        match method {
            Method::GET => {
                // External read: bypass ContentService::get (which hardcodes
                // require_provenance=false for internal callers) and call the
                // gated diesel helper directly. Unpublished rows must be
                // invisible to external clients.
                let pool = self
                    .db_pool
                    .as_ref()
                    .ok_or_else(|| StorageError::Internal("Database pool not available".into()))?;
                let mut conn = pool.get().map_err(|e| {
                    StorageError::Internal(format!("Failed to get connection: {}", e))
                })?;
                let app_ctx = db::AppContext::default_lamad();
                let result = db::content_diesel::get_content_with_tags(
                    &mut conn,
                    &app_ctx,
                    content_id,
                    db::content_diesel::MinTrust::Amber, // external HTTP boundary → serving floor
                )
                .map(|opt| opt.map(ContentView::from));

                // Layer 1: Reach-based access control
                // commons/public serve without auth, restricted content requires authentication
                if let Ok(Some(ref view)) = result {
                    let is_public = view.reach == "commons" || view.reach == "public";
                    if !is_public {
                        // Header PRESENCE is not authentication — see the /db/content
                        // listing note. Require the caller to RESOLVE to an identity
                        // via the doorway-injected `X-Agent-Cid`; a bare
                        // `Authorization:` header no longer opens restricted tiers,
                        // and neither does the ambient local session (which would
                        // resolve an anonymous request as the node's own human).
                        let resolved =
                            crate::api::account::extract_agent_cid_explicit(&req).is_some();
                        if !resolved {
                            return Ok(Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(
                                    r#"{{"error":"Authentication required","requiredReach":"{}"}}"#,
                                    view.reach
                                ))))
                                .unwrap());
                        }
                    }

                    // Layer 1.5: Steward-specific reach authorization for tiers
                    // ABOVE community (familiar / trusted / intimate / self). The
                    // is_public check above only refuses anonymous; this enforces
                    // the SAME reach gate the P2P transport path uses
                    // (epr_service::authorize_reach_for_human — single source of
                    // truth), resolving the requester by the reliable humans.id
                    // column (then agent key). Deny-by-default when the requester
                    // can't be resolved. NOTE: live-alpha end-to-end enforcement
                    // is coupled to humans.agent_pub_key population (the
                    // resilience-card-self-cid-provide-loop-gate fork); the
                    // household stack (id populated) proves it now.
                    if crate::epr_service::reach_level_index(&view.reach)
                        > crate::epr_service::reach_level_index("community")
                    {
                        // Explicit header only — see `extract_agent_cid_explicit`.
                        let identity = crate::api::account::extract_agent_cid_explicit(&req);
                        let requester = match identity {
                            Some(idv) => {
                                match crate::db::humans::get_human_by_id(&mut conn, &idv)
                                    .ok()
                                    .flatten()
                                {
                                    Some(h) => Some(h),
                                    None => {
                                        crate::db::humans::get_human_by_agent_key(&mut conn, &idv)
                                            .ok()
                                            .flatten()
                                    }
                                }
                            }
                            None => None,
                        };
                        match requester {
                            Some(human) => {
                                let reach_gate = crate::epr_service::EprService::new(
                                    None,
                                    None,
                                    None,
                                    crate::p2p::trust_cache::PeerTrustCache::new(),
                                )
                                // T7: this `EprService` is still constructed
                                // fresh per request (unchanged — cheap,
                                // Option<Arc<_>> handles), but the memo store
                                // it holds is now the SAME process-lifetime
                                // Arc on every request, wired at startup via
                                // `HttpServer::with_memo_store`.
                                .with_memo_store(self.memo_store.clone());
                                // T9: the gradient selection (memo-carrying vs
                                // inert) lives in ONE place — the service's
                                // own dormancy gate. Flag off ⇒ inert() ⇒
                                // byte-identical to pre-T9 behavior.
                                if let Err(reason) = reach_gate
                                    .authorize_reach_for_human_with_own_trust(
                                        &mut conn,
                                        &app_ctx,
                                        &view.reach,
                                        &human,
                                        content_id,
                                    )
                                {
                                    return Ok(Response::builder()
                                        .status(StatusCode::FORBIDDEN)
                                        .header(header::CONTENT_TYPE, "application/json")
                                        .body(Full::new(Bytes::from(format!(
                                            r#"{{"error":"Reach denied","requiredReach":"{}","reason":"{}"}}"#,
                                            view.reach, reason
                                        ))))
                                        .unwrap());
                                }
                            }
                            None => {
                                return Ok(Response::builder()
                                    .status(StatusCode::FORBIDDEN)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(
                                        r#"{{"error":"Reach authorization required","requiredReach":"{}"}}"#,
                                        view.reach
                                    ))))
                                    .unwrap());
                            }
                        }
                    }

                    // Policy enforcement: check device policy ceiling
                    let agent_id = Self::extract_agent_id(&req);
                    if let (Some(ref enforcement), Some(ref agent)) =
                        (&self.policy_enforcement, &agent_id)
                    {
                        // Shared mapping, not a hand-rolled copy: this duplicate
                        // still carried the pre-2026-08-20 `_ => 0` fail-open, so
                        // an unrecognized tier read as commons for device-policy
                        // purposes. `can_serve` blocks on
                        // `content_reach > max_reach`, so the shared fail-closed
                        // u8::MAX is the safe direction here.
                        let reach_level_num = crate::epr_service::reach_level_index(&view.reach);
                        let content_meta = ContentMetadata {
                            hash: content_id.to_string(),
                            categories: Vec::new(),
                            age_rating: None,
                            reach_level: Some(reach_level_num),
                        };
                        match enforcement.can_serve(agent, &content_meta) {
                            Ok(PolicyDecision::Block { reason }) => {
                                return Ok(Response::builder()
                                    .status(StatusCode::FORBIDDEN)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(Full::new(Bytes::from(format!(
                                        r#"{{"error":"Policy blocked","reason":"{}"}}"#,
                                        reason
                                    ))))
                                    .unwrap());
                            }
                            Ok(PolicyDecision::Allow) => {}
                            Err(_) => {} // Policy lookup failure is non-blocking
                        }
                    }

                    // Layer 2: Prerequisite-mastery gate — EPR Heads (discovery)
                    // flow freely; body access requires mastery of every direct
                    // PREREQUISITE of this content. The requirement is a content
                    // graph edge (*epr_edge{rel_type:'PREREQUISITE'}); this path
                    // calls the SAME shared gate as the P2P path
                    // (`epr_service::check_prerequisite_mastery`) so the two can
                    // never drift. No graph engine (tests) → no-op allow.
                    #[cfg(feature = "graph-native")]
                    if let (Some(ref agent), Some(ref graph)) =
                        (Self::extract_agent_id(&req), &self.graph_engine)
                    {
                        if let Ok(mut att_conn) = self.get_conn() {
                            let app_ctx = db::AppContext::default_lamad();
                            if let Ok(Some(human)) =
                                crate::db::humans::get_human_by_agent_key(&mut att_conn, agent)
                            {
                                if crate::epr_service::check_prerequisite_mastery(
                                    graph,
                                    &mut att_conn,
                                    &app_ctx,
                                    &human.id,
                                    content_id,
                                )
                                .is_err()
                                {
                                    return Ok(Response::builder()
                                        .status(StatusCode::FORBIDDEN)
                                        .header(header::CONTENT_TYPE, "application/json")
                                        .body(Full::new(Bytes::from(format!(
                                            r#"{{"error":"Prerequisite mastery required","contentId":"{}"}}"#,
                                            content_id
                                        ))))
                                        .unwrap());
                                }
                            }
                        }
                    }
                }

                // If content found locally or DB error, return immediately
                if !matches!(&result, Ok(None)) {
                    return Ok(response::from_option(
                        result,
                        &format!("Content not found: {}", content_id),
                    ));
                }

                // Demand-driven auto-pin (self-healing opportunity map row 15):
                // a CONFIRMED local read-miss for content this node was asked
                // for IS a demand signal. Author an `item` DevicePin (head_ref ==
                // content id) via the shared idempotent upsert so the acquisition
                // loop fetches it and the provide loop authors a
                // replicates-commons commitment — the runtime input the
                // provide-loop was starved of. Config-gated + throttled (off the
                // hot path); the DOWNSTREAM provide-eligibility gate decides
                // whether anything is ever provided, so we do NOT reach-gate
                // here. Non-fatal: the read path continues regardless.
                if self.demand_autopin.is_enabled() {
                    match self.demand_autopin.record_demand(&mut conn, content_id) {
                        Ok(Some(pin)) => {
                            debug!(id = %content_id, pin_id = pin.id, "demand auto-pin authored on read-miss");
                        }
                        Ok(None) => {}
                        Err(e) => {
                            debug!(id = %content_id, error = %e, "demand auto-pin upsert failed (non-fatal)");
                        }
                    }
                }

                // Content not found locally — try P2P EPR resolution + shard fetch
                #[cfg(feature = "p2p")]
                if let Some(ref handle) = self.p2p_handle {
                    debug!(id = %content_id, "Content not found locally, trying P2P resolve + fetch");
                    if let Some((head, content_bytes)) = handle.resolve_and_fetch(content_id).await
                    {
                        info!(id = %content_id, size = content_bytes.len(), "Content resolved via P2P");

                        // Store blob
                        let blob_result = self.blob_store.store(&content_bytes).await;

                        // Persist to local SQLite so future GETs are local
                        if let Some(ref svc) = self.services {
                            let body_str = String::from_utf8_lossy(&content_bytes).to_string();
                            let input = db::content_diesel::CreateContentInput {
                                id: content_id.to_string(),
                                title: head.lamad.title.clone(),
                                description: head.lamad.description.clone(),
                                content_type: head.lamad.content_type.clone(),
                                content_format: head
                                    .lamad
                                    .content_format
                                    .clone()
                                    .unwrap_or_else(|| "markdown".to_string()),
                                blob_hash: blob_result.as_ref().ok().map(|r| r.hash.clone()),
                                blob_cid: if head.content.is_empty() {
                                    None
                                } else {
                                    Some(head.content.clone())
                                },
                                content_size_bytes: Some(content_bytes.len() as i32),
                                metadata_json: Some(r#"{"resolved_via":"p2p"}"#.to_string()),
                                reach: head
                                    .qahal
                                    .reach
                                    .clone()
                                    .unwrap_or_else(|| "commons".to_string()),
                                created_by: head.author.clone(),
                                tags: head.lamad.tags.clone(),
                                content_body: Some(body_str),
                                dht_anchor_hash: None,
                            };
                            match svc.content.create(input) {
                                Ok(content_with_tags) => {
                                    info!(id = %content_id, "P2P content persisted to local SQLite");

                                    // Distribute recognition through the pipeline (fire-and-forget)
                                    // Pipeline handles: normalize → resolve stewards → weight by affinity → limit → settle
                                    if let Ok(mut recog_conn) = self.get_conn() {
                                        let recog_ctx = db::AppContext::default_lamad();
                                        let trigger = crate::services::recognition_pipeline_service::RecognitionTrigger {
                                            content_id: content_id.to_string(),
                                            event_type: crate::db::models::lamad_event_types::CONTENT_DELIVERY.to_string(),
                                            raw_amount: 1.0,
                                            triggered_by: Some("p2p-epr-resolution".to_string()),
                                        };
                                        match crate::services::recognition_pipeline_service::distribute(
                                            &mut recog_conn,
                                            &recog_ctx,
                                            trigger,
                                        ) {
                                            Ok(result) => {
                                                debug!(
                                                    id = %content_id,
                                                    stewards = result.distributions.len(),
                                                    events = result.economic_event_ids.len(),
                                                    "Recognition distributed via pipeline"
                                                );
                                            }
                                            Err(e) => {
                                                debug!(id = %content_id, error = %e, "Recognition pipeline failed (non-fatal)");
                                            }
                                        }
                                    }

                                    let view = ContentView::from(content_with_tags);
                                    return Ok(response::ok(&view));
                                }
                                Err(e) => {
                                    warn!(id = %content_id, error = %e, "Failed to persist P2P content");
                                    let view = crate::views::content_view_from_epr_head(&head);
                                    return Ok(response::ok(&view));
                                }
                            }
                        }
                    }
                }

                Ok(response::from_option(
                    result,
                    &format!("Content not found: {}", content_id),
                ))
            }
            Method::PATCH => {
                // Single notarized head. A notarized-field PATCH (blob_hash or
                // reach) is authored ONCE through the conductor so the Holochain
                // DHT witnesses the claim from this head — in context with the
                // rest of the network — and gossips it to every peer. There is
                // NO per-host "amber" write escape hatch: a diesel-direct
                // per-peer write would mint an un-witnessed head (green can never
                // be derived for it, since green == DHT-anchored) that diverges
                // across backends. That was the retired `deployTier=amber` /
                // `X-Deploy-Tier: amber` class — the deploy now fails over to a
                // conductor-bridged doorway instead of writing amber locally.
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();

                let view: UpdateContentInputView = serde_json::from_slice(&body_bytes)
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                // Substrate-correct PATCH path: when the patch touches a
                // DNA-notarized field (blob_hash or reach) AND the lamad
                // conductor bridge is wired, route through
                // ContentService::update_via_conductor so the re-authored entry
                // flows into the DHT and propagates to all peers via gossip.
                // Closes Gap C from the 2026-05-26 sprint-result (stageSpaBlobs
                // PATCHes were diesel-direct, never reaching the DHT) and gap G1
                // from the reach-floor plan (a reach-only PATCH fell to diesel
                // and was reverted by the reconciliation controller, so a reach
                // correction never stuck). For metadata-only patches
                // (title/description/tags) we keep the legacy diesel update —
                // those don't break cross-peer divergence.
                //
                // See 2026-05-26-substrate-rea-replication-fix.md Task 8d.
                let needs_conductor = patch_needs_conductor(&view);
                let lamad_hc = self.hc_registry.as_ref().and_then(|r| r.lamad_client());

                let update_result = match (needs_conductor, lamad_hc) {
                    (true, Some(hc)) => {
                        services
                            .content
                            // DECLARE: this is a request-borne re-notarization —
                            // a deploy PATCHing {blobHash, reach} IS the
                            // head-declaration act, and it must keep moving the
                            // head deliberately. The heal-class re-author sweeps
                            // preserve instead; see `HeadElection`.
                            .update_via_conductor(
                                &hc,
                                content_id,
                                view,
                                db::content_diesel::HeadElection::Declare,
                            )
                            .await
                    }
                    (true, None) => {
                        // A notarized change (blob_hash/reach) needs the
                        // conductor to author ONE witnessed head that gossips to
                        // every peer. With no conductor bridge on this backend,
                        // fail loud so the deploy fails over to a bridged
                        // doorway. A diesel-direct write here would mint an
                        // un-witnessed per-peer head that never greens and
                        // diverges across backends (the retired amber write); the
                        // DHT would also revert a diesel-only notarized field.
                        // This peer converges the head later via run_content_sweep
                        // once its own conductor is reachable.
                        return Ok(response::service_unavailable(
                            "Conductor bridge unavailable; cannot re-notarize content change",
                        ));
                    }
                    (false, _) => services.content.update(content_id, view),
                };

                // Pattern Z.B.1 bridge: refresh the in-memory slug_index so the
                // /apps/{slug}/{file} fast-path sees the new blob_hash. Without
                // this, ContentService::update writes to SQLite but the fast-path
                // cache returns the stale (pre-PATCH) blob_hash on the next
                // request — silently serving the OLD bundle while DB says NEW.
                // Sibling bridge: Z.B.2 in doorway-service (storage_events
                // subscriber) handles the cross-service projection cache.
                // See genesis/docs/superpowers/specs/
                // 2026-05-23-doorway-access-tier-patterns.md (Pattern Z).
                if update_result.is_ok() {
                    self.load_slug_index().await;
                }

                Ok(response::from_result(
                    update_result.map(ContentWithTagsView::from),
                ))
            }
            Method::DELETE => {
                let result = services.content.delete_cascade(content_id);
                Ok(response::from_delete_bool_result(
                    result,
                    &format!("Content not found: {}", content_id),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // Relationship handlers
    // =========================================================================

    /// GET /db/relationships - List relationships, POST - Create relationship
    async fn handle_db_relationships_list(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        let query_str = req.uri().query().unwrap_or("");
        let query: db::relationships_diesel::RelationshipQuery =
            serde_urlencoded::from_str(query_str).unwrap_or_default();

        match method {
            Method::GET => match services.relationship.list(&query) {
                Ok(items) => {
                    let views: Vec<RelationshipView> =
                        items.into_iter().map(|r| r.into()).collect();
                    let body = serde_json::json!({
                        "items": views,
                        "count": views.len(),
                        "limit": query.limit,
                        "offset": query.offset,
                    });
                    Ok(response::ok(&body))
                }
                Err(e) => Ok(response::error_response(e)),
            },
            Method::POST => {
                // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
                // Currently null for direct storage writes. Backfill needed for pre-coherence data.
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();
                let input_view: CreateRelationshipInputView =
                    serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let input: db::relationships_diesel::CreateRelationshipInput = input_view.into();
                Ok(response::from_create_result(
                    services.relationship.create(input),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/relationships/bulk - Bulk create relationships
    async fn handle_db_relationships_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let body_bytes = body.to_bytes();

        let input_views: Vec<CreateRelationshipInputView> = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<db::relationships_diesel::CreateRelationshipInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        match services.relationship.bulk_create(inputs) {
            Ok(result) => Ok(response::ok_with_schema_info(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET /db/relationships/graph/{content_id} - Get content graph
    ///
    /// Query parameters:
    /// - `depth`: BFS depth limit, default 1, capped at 3.
    /// - `computed`: include tag-discovery neighbours; accepts `true|1|yes|on`
    ///   (case-insensitive). Anything else (or absent) = false.
    /// - `minSharedTags`: minimum shared tags for tag-discovery, default 1.
    /// - `maxComputed`: max tag-discovered neighbours, default 25, capped at 100.
    /// - `types`: comma-separated edge-type whitelist; absent = resolver default.
    async fn handle_db_content_graph(
        &self,
        req: Request<Incoming>,
        method: Method,
        content_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        let query_str = req.uri().query().unwrap_or("");
        let params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query_str.as_bytes())
                .into_owned()
                .collect();

        // Edge-type filter: comma list -> Vec<String>; empty after trimming -> None
        // (None means "resolver default whitelist", NOT "all edge types").
        let types_vec: Vec<String> = params
            .get("types")
            .map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        let types_opt: Option<&[String]> = if types_vec.is_empty() {
            None
        } else {
            Some(&types_vec)
        };

        // SAFE DEFAULTS + CLAMPS. The caps below are the trust boundary: they
        // close the `usize as i64` -> `LIMIT -1` truncation an attacker could
        // otherwise trigger with a huge/negative cap. depth default 1 (current
        // behaviour) capped at 3; computed OFF by default (the Angular sidebar
        // opts in with `computed=true`); max_computed hard-ceilinged at 100.
        let depth = params
            .get("depth")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1)
            .min(3);
        // computed: accepts true|1|yes|on (case-insensitive); anything else (or absent) = false
        let include_computed = params
            .get("computed")
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "true" | "1" | "yes" | "on"
                )
            })
            .unwrap_or(false);
        let min_shared_tags = params
            .get("minSharedTags")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1)
            .max(1) as usize;
        let max_computed = params
            .get("maxComputed")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(25)
            .min(100) as usize;

        Ok(response::from_result(
            services.relationship.get_graph_query(
                content_id,
                depth,
                include_computed,
                min_shared_tags,
                max_computed,
                types_opt,
            ),
        ))
    }

    /// GET/DELETE /db/relationships/{id} - Get or delete relationship by ID
    async fn handle_db_relationship_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        rel_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        match method {
            Method::GET => {
                let result = services.relationship.get(rel_id);
                Ok(response::from_option(
                    result,
                    &format!("Relationship not found: {}", rel_id),
                ))
            }
            Method::DELETE => {
                let result = services.relationship.delete(rel_id);
                Ok(response::from_delete_bool_result(
                    result,
                    &format!("Relationship not found: {}", rel_id),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // Knowledge Map handlers
    // =========================================================================

    /// GET /db/knowledge-maps - List knowledge maps, POST - Create knowledge map
    async fn handle_db_knowledge_maps_list(
        &self,
        req: Request<Incoming>,
        method: Method,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        let query_str = req.uri().query().unwrap_or("");
        let query: db::knowledge_maps_diesel::KnowledgeMapQuery =
            serde_urlencoded::from_str(query_str).unwrap_or_default();

        match method {
            Method::GET => match services.knowledge.list_knowledge_maps(&query) {
                Ok(items) => {
                    let body = serde_json::json!({
                        "items": items,
                        "count": items.len(),
                        "limit": query.limit,
                        "offset": query.offset,
                    });
                    Ok(response::ok(&body))
                }
                Err(e) => Ok(response::error_response(e)),
            },
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();
                let input: db::knowledge_maps_diesel::CreateKnowledgeMapInput =
                    serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                Ok(response::from_create_result(
                    services.knowledge.create_knowledge_map(input),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/PUT/DELETE /db/knowledge-maps/{id} - Knowledge map by ID
    async fn handle_db_knowledge_map_by_id(
        &self,
        req: Request<Incoming>,
        method: Method,
        map_id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let services = self
            .services
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Services not available".into()))?;

        match method {
            Method::GET => {
                let result = services.knowledge.get_knowledge_map(map_id);
                Ok(response::from_option(
                    result,
                    &format!("Knowledge map not found: {}", map_id),
                ))
            }
            Method::PUT => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let body_bytes = body.to_bytes();
                let input: db::knowledge_maps_diesel::CreateKnowledgeMapInput =
                    serde_json::from_slice(&body_bytes)
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                Ok(response::from_result(
                    services.knowledge.update_knowledge_map(map_id, input),
                ))
            }
            Method::DELETE => {
                let result = services.knowledge.delete_knowledge_map(map_id);
                Ok(response::from_delete_bool_result(
                    result,
                    &format!("Knowledge map not found: {}", map_id),
                ))
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // HTML5 App serving handlers
    // =========================================================================

    /// Handle delivery capability probe for an HTML5 app.
    ///
    /// Route: HEAD /apps/{identifier}/_capability
    ///
    /// The identifier can be either a slug (human-readable name) or a content
    /// address (sha256-...). Returns an empty body with headers describing the
    /// delivery readiness of this storage node for the given app. Used by
    /// service workers and doorway to negotiate the optimal delivery path
    /// (extracted vs compressed).
    async fn handle_app_capability(
        &self,
        path: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let identifier = path
            .strip_prefix("/apps/")
            .and_then(|s| s.strip_suffix("/_capability"))
            .unwrap_or("");

        if identifier.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Length", "0")
                .body(Full::new(Bytes::new()))
                .unwrap());
        }

        // Resolve identifier: content address bypasses slug_index lookup. A
        // CIDv1 and its `sha256-{hex}` alias normalize to one canonical key.
        let is_cid = is_content_address(identifier);
        let canonical = canonical_content_address(identifier)
            .filter(|_| is_cid)
            .unwrap_or_else(|| identifier.to_string());
        let (resolved_slug, blob_hash) = if is_cid {
            (None, Some(canonical.clone()))
        } else {
            let hash = self.slug_index.read().await.get(identifier).cloned();
            (Some(identifier.to_string()), hash)
        };

        // Cache key: use slug when available, otherwise the canonical address
        let cache_key = resolved_slug.as_deref().unwrap_or(canonical.as_str());

        // Check extraction cache warmth
        let (ready, delivery_mode) = match (&self.extraction_cache, &blob_hash) {
            (Some(cache), Some(hash)) => {
                let is_warm = cache.is_current(cache_key, hash).await;
                (is_warm, if is_warm { "extracted" } else { "compressed" })
            }
            _ => (false, "compressed"),
        };

        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header("Content-Length", "0")
            .header("X-Delivery-Mode", delivery_mode)
            .header("X-Cache-Tier", "extraction")
            .header("X-Ready", if ready { "true" } else { "false" });

        if let Some(hash) = &blob_hash {
            builder = builder
                .header("X-Blob-Hash", hash.as_str())
                .header("X-Content-Address", hash.as_str());
        }
        if let Some(ref slug) = resolved_slug {
            builder = builder.header("X-Content-Slug", slug.as_str());
        }

        Ok(builder.body(Full::new(Bytes::new())).unwrap())
    }

    /// Handle HTML5 app file requests
    ///
    /// Route: GET /apps/{identifier}/{file_path}
    ///
    /// The identifier can be either a slug (human-readable name) or a content
    /// address (sha256-...). Slug-based paths resolve via `slug_index`; content
    /// address paths use the hash directly as the blob key.
    ///
    /// Fast path (cache hit): O(1) index lookup + disk read. No DB, no ZIP, no pool.
    /// Slow path (cache miss): DB query + ZIP extract + cache all files + serve.
    async fn handle_app_request(
        &self,
        path: &str,
        query: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        use std::io::Read;
        use zip::ZipArchive;

        // Parse path: /apps/{identifier}/{file_path}
        let remainder = path.strip_prefix("/apps/").unwrap_or("");
        let (identifier, file_path) = match remainder.find('/') {
            Some(pos) => (&remainder[..pos], &remainder[pos + 1..]),
            None => (remainder, "index.html"),
        };

        // SPA deep-link fallback opt-out (§12.2). The storage safety net serves
        // a ROUTE miss's index.html by convention (Class C) — this is what makes
        // tauri-direct deep links work with no doorway in the loop, so the
        // DEFAULT is on. The doorway is contract-authoritative for `spa_fallback`
        // (a field on the project-epr commitment); when a projection sets it
        // false, doorway appends `?spaFallback=0` to the storage proxy URL,
        // propagating its authoritative opt-out down to this convention layer so
        // the two layers stay consistent. Any value other than 0/false/no keeps
        // the default-on behaviour.
        let spa_fallback = {
            let params: std::collections::HashMap<String, String> =
                url::form_urlencoded::parse(query.as_bytes())
                    .into_owned()
                    .collect();
            !matches!(
                params.get("spaFallback").map(String::as_str),
                Some("0") | Some("false") | Some("no")
            )
        };

        if identifier.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "Missing app identifier"}"#,
                )))
                .unwrap());
        }

        // Percent-encoded traversal sequences (%2E%2E etc.) need no extra handling here:
        // the path is matched against ZIP entry names verbatim (never decoded, never used
        // to construct a filesystem path), so an encoded `..` simply fails to match.
        if file_path.contains("..") || file_path.contains('\0') || file_path.starts_with('/') {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Invalid file path"}"#)))
                .unwrap());
        }

        // Resolve identifier: content address bypasses slug_index lookup. A
        // CIDv1 (`bafkrei…`) and its `sha256-{hex}` alias normalize to one
        // canonical key, so the two spellings of one address share a single
        // extraction-cache entry and one blob path.
        let is_cid = is_content_address(identifier);
        let canonical = canonical_content_address(identifier)
            .filter(|_| is_cid)
            .unwrap_or_else(|| identifier.to_string());
        let (resolved_slug, cached_blob_hash) = if is_cid {
            (None, Some(canonical.clone()))
        } else {
            let hash = {
                let index = self.slug_index.read().await;
                index.get(identifier).cloned()
            };
            (Some(identifier.to_string()), hash)
        };

        // Cache key: use slug when available, otherwise the canonical address
        let cache_key = resolved_slug.as_deref().unwrap_or(canonical.as_str());

        debug!(identifier = %identifier, file_path = %file_path, is_cid = %is_cid, "App file request");

        // --- Fast path: check extraction cache ---
        if let (Some(ref cache), Some(ref hash)) = (&self.extraction_cache, &cached_blob_hash) {
            if cache.is_current(cache_key, hash).await {
                if let Some(data) = cache.get_file(cache_key, file_path).await {
                    let content_type = Self::get_mime_type(file_path);
                    debug!(identifier = %identifier, file_path = %file_path, "Cache HIT");
                    let mut builder = Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CONTENT_LENGTH, data.len())
                        .header(header::CACHE_CONTROL, "public, max-age=3600")
                        .header("X-Cache", "HIT")
                        .header("X-Content-Address", hash.as_str());
                    if let Some(ref slug) = resolved_slug {
                        builder = builder
                            .header("X-Slug", slug.as_str())
                            .header("X-Content-Slug", slug.as_str());
                    }
                    return Ok(builder.body(Full::new(Bytes::from(data))).unwrap());
                }
                // SPA deep-link fallback (§12.2): the bundle is cached and
                // current but this sub-path isn't a file in it — a ROUTE miss
                // serves the cached index.html so the SPA router renders. An
                // ASSET miss (or absent cached index.html) falls through to
                // re-extraction, which produces the honest 404. `spa_fallback`
                // off (doorway opt-out via ?spaFallback=0) skips the fallback.
                if spa_fallback && is_spa_route_subpath(file_path) {
                    if let Some(index_html) = cache.get_file(cache_key, "index.html").await {
                        debug!(identifier = %identifier, file_path = %file_path, "Cache HIT — SPA fallback index.html");
                        let mut builder = Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "text/html")
                            .header(header::CONTENT_LENGTH, index_html.len())
                            .header(header::CACHE_CONTROL, "public, max-age=3600")
                            .header("X-Cache", "HIT")
                            .header("X-SPA-Fallback", "1")
                            .header("X-Content-Address", hash.as_str());
                        if let Some(ref slug) = resolved_slug {
                            builder = builder
                                .header("X-Slug", slug.as_str())
                                .header("X-Content-Slug", slug.as_str());
                        }
                        return Ok(builder.body(Full::new(Bytes::from(index_html))).unwrap());
                    }
                }
            }
        }

        // --- Slow path: DB lookup + ZIP extraction ---
        // Thundering herd protection: at most ONE extractor per app_id at a
        // time. Everyone else waits on the broadcast and re-reads the cache.
        let mut won_extraction_rights = true;
        let mut coalesce_rounds = 0u32;
        if let Some(ref cache) = self.extraction_cache {
            // Re-enter the flight on every miss. A waiter whose post-wait
            // re-check misses must ACQUIRE extraction rights, never extract
            // unregistered — see `won_extraction_rights` below.
            loop {
                let Some(mut rx) = cache.begin_extraction(cache_key) else {
                    // Nobody is extracting and `begin_extraction` just
                    // registered US. We own the flight.
                    break;
                };
                // Another request is extracting — wait for it
                debug!(identifier = %identifier, "Waiting for in-flight extraction");
                let _ = rx.recv().await; // ignore errors — extractor may have finished
                                         // Retry from cache
                if let Some(data) = cache.get_file(cache_key, file_path).await {
                    let content_type = Self::get_mime_type(file_path);
                    debug!(identifier = %identifier, file_path = %file_path, "Cache HIT (after wait)");
                    let mut builder = Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CONTENT_LENGTH, data.len())
                        .header(header::CACHE_CONTROL, "public, max-age=3600")
                        .header("X-Cache", "HIT-COALESCED");
                    if let Some(ref hash) = cached_blob_hash {
                        builder = builder.header("X-Content-Address", hash.as_str());
                    }
                    if let Some(ref slug) = resolved_slug {
                        builder = builder
                            .header("X-Slug", slug.as_str())
                            .header("X-Content-Slug", slug.as_str());
                    }
                    return Ok(builder.body(Full::new(Bytes::from(data))).unwrap());
                }
                // SPA deep-link fallback (§12.2) on the coalesced path: a ROUTE
                // miss serves the cached index.html; an ASSET miss (or absent
                // cached index.html) falls through to extract ourselves so the
                // honest 404 surfaces. `spa_fallback` off skips the fallback.
                if spa_fallback && is_spa_route_subpath(file_path) {
                    if let Some(index_html) = cache.get_file(cache_key, "index.html").await {
                        debug!(identifier = %identifier, file_path = %file_path, "Cache HIT (after wait) — SPA fallback index.html");
                        let mut builder = Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "text/html")
                            .header(header::CONTENT_LENGTH, index_html.len())
                            .header(header::CACHE_CONTROL, "public, max-age=3600")
                            .header("X-Cache", "HIT-COALESCED")
                            .header("X-SPA-Fallback", "1");
                        if let Some(ref hash) = cached_blob_hash {
                            builder = builder.header("X-Content-Address", hash.as_str());
                        }
                        if let Some(ref slug) = resolved_slug {
                            builder = builder
                                .header("X-Slug", slug.as_str())
                                .header("X-Content-Slug", slug.as_str());
                        }
                        return Ok(builder.body(Full::new(Bytes::from(index_html))).unwrap());
                    }
                }
                // The post-wait re-check MISSED. Historically this fell
                // straight through and extracted — without ever registering in
                // `in_flight`. That is the thundering-herd hole: when the first
                // extractor's `put_app` fails it leaves NO index entry, so every
                // waiter misses here, and all of them became simultaneous
                // extractors. Their concurrent `put_app` calls then raced
                // `evict_app`'s `remove_dir_all` against each other's directory
                // writes -> `Directory not empty (os error 39)` -> `put_app`
                // returns Err after removing the index entry and before
                // re-inserting it -> the app is permanently uncached -> every
                // later request re-extracts the whole bundle -> more
                // concurrency. Self-sustaining, and measured live on
                // 2026-08-20 (adam + matthew, identifier=elohim-host-landing,
                // two failures 8ms apart), where the resulting multi-second
                // `/apps/<id>/index.html` reads blew the doorway's 10s SSR
                // shell budget three times in a row and opened the per-ENDPOINT
                // upstream breaker, shedding every `/db` route on the peer.
                //
                // So: go round again and contend for the flight properly.
                coalesce_rounds += 1;
                if coalesce_rounds >= MAX_EXTRACTION_COALESCE_ROUNDS {
                    warn!(
                        identifier = %identifier,
                        rounds = coalesce_rounds,
                        "extraction coalescing gave up after repeated post-wait misses — \
                         extracting WITHOUT the flight (bounded livelock escape)"
                    );
                    won_extraction_rights = false;
                    break;
                }
            }
        }
        // Guard lives until end of function — any return triggers
        // finish_extraction. Created ONLY when we actually hold the flight: a
        // caller that took the bounded livelock escape above never registered,
        // and dropping a guard it never earned would broadcast `finish` for the
        // real extractor and release the next herd early.
        let _extraction_guard = if won_extraction_rights {
            self.extraction_cache
                .as_ref()
                .map(|c| c.extraction_guard(cache_key))
        } else {
            None
        };

        debug!(identifier = %identifier, "Cache MISS — extracting from ZIP");

        let blob_hash = match cached_blob_hash {
            Some(h) => h,
            None => {
                if is_cid {
                    // CID was provided but blob not found — nothing to resolve
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "App not found for content address: {}"}}"#,
                            identifier
                        ))))
                        .unwrap());
                }
                match self.lookup_slug_blob_hash(identifier).await? {
                    Some(h) => h,
                    None => {
                        return Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "App not found: {}"}}"#,
                                identifier
                            ))))
                            .unwrap());
                    }
                }
            }
        };

        if blob_hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "App ZIP not available (no blob_hash)"}"#,
                )))
                .unwrap());
        }

        // C3 read side (Plan C3 / notary-authority): honor the notary-DECLARED
        // canonical head over "whatever this peer last authored". For a slug/id
        // (never a bare CID), prefer the declared head's blob when it is a
        // distinct, LOCALLY-PRESENT blob whose declared-head hint matches the
        // row — otherwise degrade to the resolved own-row `blob_hash`
        // (`declared_head_served_blob` never fans out; the own hash still rides
        // the existing `get_blob_or_heal`/T17 peer-fallback below). Keep the
        // slug_index coherent so the O(1) fast path serves the same head next
        // time.
        let blob_hash = if is_cid {
            blob_hash
        } else if let Some(content) = self.lookup_app_content(identifier).await {
            match self.declared_head_served_blob(&content).await {
                Some(head_blob) => {
                    info!(
                        identifier = %identifier,
                        own_blob = %blob_hash,
                        declared_head_blob = %head_blob,
                        "C3: serving notary-declared head blob over own-row blob"
                    );
                    self.slug_index
                        .write()
                        .await
                        .insert(identifier.to_string(), head_blob.clone());
                    head_blob
                }
                None => blob_hash,
            }
        } else {
            blob_hash
        };

        debug!(identifier = %identifier, blob_hash = %blob_hash, "Found blob hash");

        // Fetch the ZIP from the blob store, healing from peers on a local miss
        // (T17). This shares the same race-fetch + finalize machinery as
        // `GET /blob/{hash}`: an alpha-class regression where the db/content row
        // points at a hash whose bytes never landed locally now self-heals on
        // the first page visit instead of 404ing for days (resilient-dual-doorway
        // journal iter-0 Fix B; the manual `GET /blob/<hash>` heal that fixed the
        // live site exercised this exact path). On heal failure the existing 404
        // body is preserved unchanged.
        //
        // `get_blob_or_heal` consults `peer_blob_inventory` keyed by the
        // canonical `sha256-{hex}` form, so normalize first (falling back to the
        // stored value if it doesn't parse — `BlobStore::get`'s `blob_path`
        // tolerates either form, matching the prior direct `blob_store.get`).
        let heal_hash = match crate::blob_store::BlobStore::parse_content_address(&blob_hash) {
            Ok(h) => format!("sha256-{}", h),
            Err(_) => blob_hash.clone(),
        };
        let zip_data = match self.get_blob_or_heal(&heal_hash).await {
            BlobHealOutcome::Bytes { bytes, healed_from } => {
                if let Some(ref peer) = healed_from {
                    warn!(
                        identifier = %identifier,
                        blob_hash = %heal_hash,
                        source_peer = %peer,
                        "app blob healed on-read from peer (T17 race-fetch)"
                    );
                }
                bytes
            }
            // Fix B: the peer-heal exceeded its in-request budget; the fetch
            // continues in the background. This is a transient byte-lag on a
            // (possibly freshly-healed) pointer, NOT a missing deploy — degrade
            // with a syncing 503 + Retry-After keyed by the slug. Must NOT
            // report "App not found" (option (b) of the .epr-meta peer-fallback
            // invariant; the federation-deploy scenario greps the 404 body).
            BlobHealOutcome::Syncing { started } => {
                debug!(identifier = %identifier, blob_hash = %heal_hash, background_fetch = started, "Fix B: app heal budget expired — returning syncing 503");
                return Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .header(header::RETRY_AFTER, "5")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(syncing_app_body(identifier))))
                    .unwrap());
            }
            // NotFound (no peer served) and FinalizeFailed (bytes fetched but
            // could not be durably persisted/recorded) both leave us without a
            // durably-available app blob — keep the existing 404 body unchanged.
            BlobHealOutcome::NotFound | BlobHealOutcome::FinalizeFailed(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "App ZIP blob not found: {}"}}"#,
                        blob_hash
                    ))))
                    .unwrap());
            }
        };

        debug!(identifier = %identifier, zip_size = zip_data.len(), "Fetched ZIP blob");

        // Extract ALL files from ZIP
        let cursor = std::io::Cursor::new(&zip_data);
        let mut archive = match ZipArchive::new(cursor) {
            Ok(a) => a,
            Err(e) => {
                warn!(error = %e, "Invalid ZIP archive");
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Invalid ZIP archive: {}"}}"#,
                        e
                    ))))
                    .unwrap());
            }
        };

        let mut all_files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut requested_file_data: Option<Vec<u8>> = None;
        // Capture the bundle's own index.html for the SPA deep-link fallback
        // (§12.2): on a ROUTE miss we serve it instead of 404. Same matching
        // idiom (exact name or `/`-suffix) as the requested-file lookup below.
        let mut index_html_data: Option<Vec<u8>> = None;
        let normalized_path = file_path.trim_start_matches('/');

        for i in 0..archive.len() {
            if let Ok(mut f) = archive.by_index(i) {
                if f.is_dir() {
                    continue;
                }
                let name = f.name().to_string();
                let mut contents = Vec::new();
                if f.read_to_end(&mut contents).is_ok() {
                    // Check if this is the requested file (exact or suffix match)
                    if requested_file_data.is_none()
                        && (name == normalized_path
                            || name.ends_with(normalized_path)
                            || name.ends_with(&format!("/{}", normalized_path)))
                    {
                        requested_file_data = Some(contents.clone());
                    }
                    if index_html_data.is_none()
                        && (name == "index.html" || name.ends_with("/index.html"))
                    {
                        index_html_data = Some(contents.clone());
                    }
                    all_files.push((name, contents));
                }
            }
        }

        // Cache the extracted files (non-fatal if caching fails)
        if let Some(ref cache) = self.extraction_cache {
            if let Err(e) = cache.put_app(cache_key, &blob_hash, all_files).await {
                warn!(error = %e, identifier = %identifier, "Failed to cache extraction (non-fatal)");
            }
            // Guard drop handles finish_extraction — no explicit call needed
        }

        // Serve the requested file
        let contents = match requested_file_data {
            Some(data) => data,
            None => {
                // SPA deep-link fallback (§12.2): a ROUTE miss serves the
                // bundle's own index.html so the SPA router renders; an ASSET
                // miss (or absent index.html) stays an honest 404. `spa_fallback`
                // off (doorway opt-out via ?spaFallback=0) skips the fallback.
                if spa_fallback && is_spa_route_subpath(normalized_path) {
                    if let Some(index_html) = index_html_data {
                        info!(
                            identifier = %identifier,
                            file_path = %file_path,
                            "Serving SPA fallback index.html for route miss (extracted)"
                        );
                        let mut builder = Response::builder()
                            .status(StatusCode::OK)
                            .header(header::CONTENT_TYPE, "text/html")
                            .header(header::CONTENT_LENGTH, index_html.len())
                            .header(header::CACHE_CONTROL, "public, max-age=3600")
                            .header("X-Cache", "MISS")
                            .header("X-SPA-Fallback", "1")
                            .header("X-Content-Address", &blob_hash);
                        if let Some(ref slug) = resolved_slug {
                            builder = builder
                                .header("X-Slug", slug.as_str())
                                .header("X-Content-Slug", slug.as_str());
                        }
                        return Ok(builder.body(Full::new(Bytes::from(index_html))).unwrap());
                    }
                }
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "File not found in app: {}"}}"#,
                        normalized_path
                    ))))
                    .unwrap());
            }
        };

        let content_type = Self::get_mime_type(file_path);

        info!(
            identifier = %identifier,
            file_path = %file_path,
            content_type = %content_type,
            size = contents.len(),
            "Serving app file (extracted + cached)"
        );

        let mut builder = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, contents.len())
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .header("X-Cache", "MISS")
            .header("X-Content-Address", &blob_hash);
        if let Some(ref slug) = resolved_slug {
            builder = builder
                .header("X-Slug", slug.as_str())
                .header("X-Content-Slug", slug.as_str());
        }
        Ok(builder.body(Full::new(Bytes::from(contents))).unwrap())
    }

    /// Look up blob hash for an app by querying DB and updating slug_index.
    async fn lookup_slug_blob_hash(&self, slug: &str) -> Result<Option<String>, StorageError> {
        let mut conn = self.get_conn()?;
        let app_ctx = db::AppContext::default_lamad();
        // External HTTP slug resolution — only consider rows that carry a
        // provenance marker (Holochain dht_anchor_hash or libp2p Kad publish).
        // App-bundle formats served via /apps/{slug}: html5-app + spa-bundle
        // (kept in sync with load_slug_index + doorway b62c5ff2c).
        const APP_BUNDLE_FORMATS: [&str; 2] = ["html5-app", "spa-bundle"];
        let mut found_hash = None;

        for fmt in APP_BUNDLE_FORMATS {
            let query = ContentQuery {
                content_format: Some(fmt.to_string()),
                limit: 100,
                ..Default::default()
            };
            // REQ-F7: the slug→blob_hash lookup serves at the Amber floor so a
            // converged-only (amber) row resolves instead of 404-ing.
            let items = db::content_diesel::list_content(
                &mut conn,
                &app_ctx,
                &query,
                db::content_diesel::MinTrust::Amber,
            )?;
            for item in items {
                let hash = item.content.blob_hash.clone().unwrap_or_default();
                if hash.is_empty() {
                    continue;
                }
                // Resolve by row id (the EPR router requests /apps/{epr_id}) AND
                // by the inner content_body slug, warming the index by both keys.
                {
                    let mut index = self.slug_index.write().await;
                    index.insert(item.content.id.clone(), hash.clone());
                }
                if item.content.id == slug {
                    found_hash = Some(hash.clone());
                }
                if let Some(ref content_body) = item.content.content_body {
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(content_body) {
                        if let Some(content_slug) = obj.get("slug").and_then(|v| v.as_str()) {
                            {
                                let mut index = self.slug_index.write().await;
                                index.insert(content_slug.to_string(), hash.clone());
                            }
                            if content_slug == slug {
                                found_hash = Some(hash.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(found_hash)
    }

    /// Resolve the app-bundle content ROW backing an `/apps/{identifier}` request
    /// — matched by row id OR inner `content_body.slug`, across the app-bundle
    /// formats, at the Amber serving floor (same query surface as
    /// `lookup_slug_blob_hash` / `load_slug_index`). Returns the full row so the
    /// C3 read side can consult its `declared_head_action_hash`.
    ///
    /// `None` on: no DB pool, query error, or no matching row (a bare CID
    /// identifier has no row here and never reaches this helper).
    async fn lookup_app_content(&self, identifier: &str) -> Option<crate::db::models::Content> {
        let mut conn = self.get_conn().ok()?;
        let app_ctx = db::AppContext::default_lamad();
        const APP_BUNDLE_FORMATS: [&str; 2] = ["html5-app", "spa-bundle"];
        for fmt in APP_BUNDLE_FORMATS {
            let query = ContentQuery {
                content_format: Some(fmt.to_string()),
                limit: 100,
                ..Default::default()
            };
            let items = db::content_diesel::list_content(
                &mut conn,
                &app_ctx,
                &query,
                db::content_diesel::MinTrust::Amber,
            )
            .ok()?;
            for item in items {
                if item.content.id == identifier {
                    return Some(item.content);
                }
                if let Some(ref content_body) = item.content.content_body {
                    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(content_body) {
                        if obj.get("slug").and_then(|v| v.as_str()) == Some(identifier) {
                            return Some(item.content);
                        }
                    }
                }
            }
        }
        None
    }

    /// C3 read side (Plan C3 / notary-authority): the notary-DECLARED head's blob
    /// for `content`, IFF it is a DISTINCT, LOCALLY-PRESENT blob whose declared-
    /// head hint matches the row's `declared_head_action_hash`. `None` ⇒ degrade
    /// to the row's own `blob_hash` (the current serve behavior).
    ///
    /// The serve path prefers the elected canonical head over "whatever this peer
    /// last authored" (the head-election gap the notary-authority spine names),
    /// but NEVER fans out: if the declared head's bytes are not already local,
    /// this returns `None` and the caller serves the own row — the replication
    /// plane heals absence (doorway single-target doctrine; peer-fallback for the
    /// served hash still rides the existing `get_blob_or_heal`/T17 sequence).
    async fn declared_head_served_blob(
        &self,
        content: &crate::db::models::Content,
    ) -> Option<String> {
        let declared = content
            .declared_head_action_hash
            .as_deref()
            .filter(|h| !h.is_empty())?;
        let sync = self.sync_manager.as_ref()?;
        let doc_id = crate::sync::projector::content_doc_id(&content.id);
        let head_blob = sync
            .declared_head_blob(
                crate::sync::projector::PROJECTION_NAMESPACE,
                &doc_id,
                declared,
            )
            .await?;
        // Identical to the row's own blob → nothing to prefer.
        if Some(head_blob.as_str()) == content.blob_hash.as_deref() {
            return None;
        }
        // Local-presence gate — NEVER fan out to fetch a declared head we don't
        // hold; degrade to the own row and let the replication plane heal.
        //
        // Manifest-aware by way of `blob_available_locally`: a `>16MB` bundle is
        // held as its manifest's shards, never as a file under the composite's
        // own name, so a raw `exists_by_address` probe answered "absent" for
        // every oversized declared head — which meant no oversized bundle could
        // ever be served as its notary-declared head.
        if self.blob_available_locally(&head_blob).await {
            Some(head_blob)
        } else {
            None
        }
    }

    // ========================================================================
    // Diesel-based Entity Handlers
    // ========================================================================

    /// Helper to get a Diesel connection from the pool
    fn get_diesel_conn(&self) -> Result<crate::db::PooledConn, StorageError> {
        self.db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Diesel pool not configured".into()))?
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))
    }

    /// GET/POST /db/human-relationships - List or create human relationships
    async fn handle_human_relationships_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: human_relationships::HumanRelationshipQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match human_relationships::list_human_relationships(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let body = serde_json::json!({
                            "items": items,
                            "count": items.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
                // Currently null for direct storage writes. Backfill needed for pre-coherence data.
                //
                // C1 write-side gate (spec §3 fix-2): a caller may consent ONLY for the
                // party they authenticate as. Resolve the caller BEFORE consuming `req`.
                let caller = crate::api::account::extract_agent_cid(&req, &mut conn)?;
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: CreateHumanRelationshipInputView =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let mut input: human_relationships::CreateHumanRelationshipInput =
                    input_view.into();

                // The caller must be authenticated and BE one of the two parties. Set consent
                // only for the caller's own side; force the counterparty's consent to false
                // (the counterparty accepts separately via POST /consent). This makes a
                // unilaterally-forged both-consented edge impossible (the C1 write fix): an
                // attacker naming a victim can only ever produce a half-consented row, which
                // is invisible to `get_consented_relationship_between`.
                let caller = match caller {
                    Some(c) => c,
                    None => {
                        return Ok(response::error_response(StorageError::InvalidInput(
                            "authentication required to create a relationship".into(),
                        )))
                    }
                };
                if caller == input.party_a_id {
                    input.consent_given_by_b = false;
                    input.initiated_by = caller.clone();
                } else if caller == input.party_b_id {
                    input.consent_given_by_a = false;
                    input.initiated_by = caller.clone();
                } else {
                    return Ok(response::error_response(StorageError::InvalidInput(
                        "caller must be a party to the relationship they create".into(),
                    )));
                }

                match human_relationships::create_human_relationship(&mut conn, ctx, input) {
                    Ok(rel) => Ok(response::created(&rel)),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/DELETE /db/human-relationships/{id}
    async fn handle_human_relationship_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => match human_relationships::get_human_relationship(&mut conn, ctx, id) {
                Ok(Some(rel)) => Ok(response::ok(&rel)),
                Ok(None) => Ok(response::not_found(&format!(
                    "Human relationship {} not found",
                    id
                ))),
                Err(e) => Ok(response::error_response(e)),
            },
            Method::DELETE => {
                match human_relationships::delete_human_relationship(&mut conn, ctx, id) {
                    Ok(true) => Ok(response::ok(&serde_json::json!({"deleted": id}))),
                    Ok(false) => Ok(response::not_found(&format!(
                        "Human relationship {} not found",
                        id
                    ))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/human-relationships/{id}/consent - Update consent on a relationship
    async fn handle_human_relationship_consent(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        // C1 write-side gate: you may set consent ONLY for YOUR OWN party — a caller cannot
        // flip the counterparty's consent flag (which is how a half-consented edge would be
        // completed into a leaking both-consented one). Resolve the caller before `req` is consumed.
        let caller = crate::api::account::extract_agent_cid(&req, &mut conn)?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        #[derive(Deserialize)]
        struct ConsentInput {
            party_id: String,
            consent: bool,
        }

        let input: ConsentInput = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        match caller {
            Some(c) if c == input.party_id => {}
            Some(_) => {
                return Ok(response::error_response(StorageError::InvalidInput(
                    "you may only set consent for your own party".into(),
                )))
            }
            None => {
                return Ok(response::error_response(StorageError::InvalidInput(
                    "authentication required to set consent".into(),
                )))
            }
        }

        let consent_update = human_relationships::ConsentUpdate {
            consent_given: input.consent,
        };

        match human_relationships::update_consent(
            &mut conn,
            ctx,
            id,
            &input.party_id,
            &consent_update,
        ) {
            Ok(rel) => Ok(response::ok(&rel)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/human-relationships/{id}/custody - Update custody settings on a relationship
    async fn handle_human_relationship_custody(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        #[derive(Deserialize)]
        struct CustodyInput {
            party_id: String,
            enabled: bool,
            #[serde(default)]
            auto_custody: Option<bool>,
            #[serde(default)]
            emergency_access: Option<bool>,
        }

        let input: CustodyInput = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        let custody_update = human_relationships::CustodyUpdate {
            custody_enabled: input.enabled,
            auto_custody_enabled: input.auto_custody,
            emergency_access_enabled: input.emergency_access,
        };

        match human_relationships::update_custody(
            &mut conn,
            ctx,
            id,
            &input.party_id,
            &custody_update,
        ) {
            Ok(rel) => Ok(response::ok(&rel)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/presences - List or create contributor presences
    async fn handle_presences_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: contributor_presences::ContributorPresenceQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match contributor_presences::list_contributor_presences(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let views: Vec<ContributorPresenceView> =
                            items.into_iter().map(|p| p.into()).collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: CreateContributorPresenceInputView =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let input: contributor_presences::CreateContributorPresenceInput =
                    input_view.into();

                match contributor_presences::create_contributor_presence(&mut conn, ctx, input) {
                    Ok(presence) => {
                        let view: ContributorPresenceView = presence.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/DELETE /db/presences/{id}
    async fn handle_presence_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                match contributor_presences::get_contributor_presence(&mut conn, ctx, id) {
                    Ok(Some(presence)) => {
                        let view: ContributorPresenceView = presence.into();
                        Ok(response::ok(&view))
                    }
                    Ok(None) => Ok(response::not_found(&format!("Presence {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::DELETE => {
                match contributor_presences::delete_contributor_presence(&mut conn, ctx, id) {
                    Ok(true) => Ok(response::ok(&serde_json::json!({"deleted": id}))),
                    Ok(false) => Ok(response::not_found(&format!("Presence {} not found", id))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/presences/{id}/stewardship - Initiate stewardship of a presence
    async fn handle_presence_stewardship(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        let input: contributor_presences::InitiateStewardshipInput =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        match contributor_presences::initiate_stewardship(&mut conn, ctx, id, &input) {
            Ok(presence) => {
                let view: ContributorPresenceView = presence.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/presences/{id}/claim - Initiate claim of a presence
    async fn handle_presence_claim(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        // Deserialize camelCase InputView, convert to internal DB type
        let input_view: InitiateClaimInputView = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let input: contributor_presences::InitiateClaimInput = input_view.into();

        match contributor_presences::initiate_claim(&mut conn, ctx, id, &input) {
            Ok(presence) => {
                let view: ContributorPresenceView = presence.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/presences/{id}/verify-claim - Verify a claim (sets state to claimed)
    async fn handle_presence_verify_claim(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;

        match contributor_presences::verify_claim(&mut conn, ctx, id) {
            Ok(presence) => {
                let view: ContributorPresenceView = presence.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/events - List or record economic events
    async fn handle_events_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: economic_events::EconomicEventQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match economic_events::list_economic_events(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let views: Vec<EconomicEventView> =
                            items.into_iter().map(|e| e.into()).collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: CreateEconomicEventInputView =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let input: economic_events::CreateEconomicEventInput = input_view.into();

                match economic_events::record_event(&mut conn, ctx, input) {
                    Ok(event) => {
                        let view: EconomicEventView = event.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/events/{id}
    async fn handle_event_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => match economic_events::get_economic_event(&mut conn, ctx, id) {
                Ok(Some(event)) => {
                    let view: EconomicEventView = event.into();
                    Ok(response::ok(&view))
                }
                Ok(None) => Ok(response::not_found(&format!("Event {} not found", id))),
                Err(e) => Ok(response::error_response(e)),
            },
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET/POST /db/mastery - List or create content mastery records
    async fn handle_mastery_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: content_mastery::MasteryQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match content_mastery::list_mastery(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let views: Vec<ContentMasteryView> =
                            items.into_iter().map(|m| m.into()).collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let input: content_mastery::CreateMasteryInput =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                match content_mastery::upsert_mastery(&mut conn, ctx, input) {
                    Ok(mastery) => {
                        let view: ContentMasteryView = mastery.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/mastery/{id}
    async fn handle_mastery_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => match content_mastery::get_mastery(&mut conn, ctx, id) {
                Ok(Some(mastery)) => {
                    let view: ContentMasteryView = mastery.into();
                    Ok(response::ok(&view))
                }
                Ok(None) => Ok(response::not_found(&format!(
                    "Mastery record {} not found",
                    id
                ))),
                Err(e) => Ok(response::error_response(e)),
            },
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/mastery/human/{human_id} - Get all mastery records for a human
    async fn handle_mastery_for_human(
        &self,
        _req: Request<Incoming>,
        method: Method,
        human_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => match content_mastery::get_mastery_for_human(&mut conn, ctx, human_id) {
                Ok(items) => {
                    let views: Vec<ContentMasteryView> =
                        items.into_iter().map(|m| m.into()).collect();
                    let count = views.len();
                    let body = serde_json::json!({
                        "items": views,
                        "count": count,
                        "humanId": human_id,
                    });
                    Ok(response::ok(&body))
                }
                Err(e) => Ok(response::error_response(e)),
            },
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // Bulk Endpoints (for seeding/import operations)
    // =========================================================================

    /// POST /db/presences/bulk - Bulk create contributor presences
    async fn handle_presences_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        // Deserialize camelCase InputView array, convert to internal DB types
        let input_views: Vec<CreateContributorPresenceInputView> =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<contributor_presences::CreateContributorPresenceInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        match contributor_presences::bulk_create_presences(&mut conn, ctx, inputs) {
            Ok(result) => Ok(response::ok_with_schema_info(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/events/bulk - Bulk record economic events
    async fn handle_events_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        // Deserialize camelCase InputView array, convert to internal DB types
        let input_views: Vec<CreateEconomicEventInputView> =
            serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<economic_events::CreateEconomicEventInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        match economic_events::bulk_record_events(&mut conn, ctx, inputs) {
            Ok(result) => Ok(response::ok_with_schema_info(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/mastery/bulk - Bulk create/update mastery records
    async fn handle_mastery_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let mut conn = self.get_diesel_conn()?;
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

        // Deserialize camelCase InputView array, convert to internal DB types
        let input_views: Vec<CreateMasteryInputView> = serde_json::from_slice(&body.to_bytes())
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<content_mastery::CreateMasteryInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        match content_mastery::bulk_upsert_mastery(&mut conn, ctx, inputs) {
            Ok(result) => Ok(response::ok_with_schema_info(&result)),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    // =========================================================================
    // Stewardship Allocation handlers
    // =========================================================================

    /// GET/POST /db/allocations - List or create stewardship allocations
    async fn handle_allocations_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: stewardship_allocations::AllocationQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match stewardship_allocations::list_allocations(&mut conn, app_ctx, &query) {
                    Ok(allocations) => {
                        let views: Vec<StewardshipAllocationView> =
                            allocations.into_iter().map(|a| a.into()).collect();
                        Ok(response::ok(&views))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: CreateAllocationInputView = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
                let input: stewardship_allocations::CreateAllocationInput = input_view.into();

                match stewardship_allocations::create_allocation(&mut conn, app_ctx, &input) {
                    Ok(allocation) => {
                        let view: StewardshipAllocationView = allocation.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/allocations/{id}, DELETE /db/allocations/{id}
    async fn handle_allocation_by_id(
        &self,
        req: Request<Incoming>,
        method: Method,
        id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                match stewardship_allocations::get_allocation_by_id(&mut conn, app_ctx, id) {
                    Ok(allocation) => {
                        let view: StewardshipAllocationView = allocation.into();
                        Ok(response::ok(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::PUT => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                // Deserialize camelCase InputView, convert to internal DB type
                let input_view: UpdateAllocationInputView = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
                let input: stewardship_allocations::UpdateAllocationInput = input_view.into();

                match stewardship_allocations::update_allocation(&mut conn, app_ctx, id, &input) {
                    Ok(allocation) => {
                        let view: StewardshipAllocationView = allocation.into();
                        Ok(response::ok(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::DELETE => {
                match stewardship_allocations::delete_allocation(&mut conn, app_ctx, id) {
                    Ok(()) => Ok(response::no_content()),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/allocations/content/{content_id} - Get content stewardship aggregate
    async fn handle_allocations_for_content(
        &self,
        _req: Request<Incoming>,
        method: Method,
        content_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match stewardship_allocations::get_content_stewardship(&mut conn, app_ctx, content_id) {
            Ok(stewardship) => {
                let view: ContentStewardshipView = stewardship.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET /db/allocations/steward/{steward_id} - Get allocations for a steward
    async fn handle_allocations_for_steward(
        &self,
        _req: Request<Incoming>,
        method: Method,
        steward_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match stewardship_allocations::get_allocations_for_steward(&mut conn, app_ctx, steward_id) {
            Ok(allocations) => {
                let views: Vec<StewardshipAllocationView> =
                    allocations.into_iter().map(|a| a.into()).collect();
                Ok(response::ok(&views))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/allocations/{id}/dispute - File a dispute on an allocation
    async fn handle_allocation_dispute(
        &self,
        req: Request<Incoming>,
        method: Method,
        allocation_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct DisputeInput {
            dispute_id: String,
            disputed_by: String,
            reason: String,
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        let input: DisputeInput = serde_json::from_slice(&body)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        match stewardship_allocations::file_dispute(
            &mut conn,
            app_ctx,
            allocation_id,
            &input.dispute_id,
            &input.disputed_by,
            &input.reason,
        ) {
            Ok(allocation) => {
                let view: StewardshipAllocationView = allocation.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/allocations/{id}/resolve - Resolve a dispute (Elohim ratification)
    async fn handle_allocation_resolve(
        &self,
        req: Request<Incoming>,
        method: Method,
        allocation_id: &str,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        #[derive(serde::Deserialize)]
        struct ResolveInput {
            ratifier_id: String,
            new_state: String,
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        let input: ResolveInput = serde_json::from_slice(&body)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        match stewardship_allocations::resolve_dispute(
            &mut conn,
            app_ctx,
            allocation_id,
            &input.ratifier_id,
            &input.new_state,
        ) {
            Ok(allocation) => {
                let view: StewardshipAllocationView = allocation.into();
                Ok(response::ok(&view))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// POST /db/allocations/bulk - Bulk create stewardship allocations
    async fn handle_allocations_bulk(
        &self,
        req: Request<Incoming>,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::POST {
            return Ok(response::method_not_allowed());
        }

        if let Err(msg) = validate_schema_version_header(&req) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }

        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();
        // Deserialize camelCase InputView array, convert to internal DB types
        let input_views: Vec<CreateAllocationInputView> = serde_json::from_slice(&body)
            .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
        let versions: Vec<u32> = input_views.iter().map(|v| v.schema_version).collect();
        if let Err(msg) = validate_schema_versions(&versions) {
            return Ok(response::error_response(StorageError::InvalidInput(msg)));
        }
        let inputs: Vec<stewardship_allocations::CreateAllocationInput> =
            input_views.into_iter().map(|v| v.into()).collect();

        let mut created = 0;
        let mut failed = 0;
        let mut errors: Vec<String> = Vec::new();

        for input in inputs {
            match stewardship_allocations::create_allocation(&mut conn, app_ctx, &input) {
                Ok(_) => created += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: {}", input.content_id, e));
                }
            }
        }

        #[derive(serde::Serialize)]
        struct BulkResult {
            created: usize,
            failed: usize,
            errors: Vec<String>,
        }

        Ok(response::ok_with_schema_info(&BulkResult {
            created,
            failed,
            errors,
        }))
    }

    // =========================================================================
    // Stewarded Node Handlers
    // =========================================================================

    /// GET /db/nodes — list nodes (optional ?claimStatus= filter)
    /// POST /db/nodes — register a new node
    async fn handle_nodes_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let params: std::collections::HashMap<String, String> =
                    url::form_urlencoded::parse(query_str.as_bytes())
                        .into_owned()
                        .collect();
                let claim_status = params.get("claimStatus").map(|s| s.as_str());

                match crate::db::stewarded_nodes::list_stewarded_nodes(&mut conn, claim_status) {
                    Ok(nodes) => {
                        let views: Vec<StewardedNodeView> =
                            nodes.into_iter().map(|n| n.into()).collect();
                        Ok(response::ok(&views))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                let input_view: CreateStewardedNodeInputView = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
                // Inject h_app_id from context
                let mut input: crate::db::stewarded_nodes::CreateStewardedNodeInput =
                    input_view.into();
                input.h_app_id = app_ctx.h_app_id.clone();

                match crate::db::stewarded_nodes::create_stewarded_node(&mut conn, input) {
                    Ok(node) => {
                        let view: StewardedNodeView = node.into();
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/nodes/{id} — get node with stewards joined
    async fn handle_node_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        _app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::GET => {
                match crate::db::stewarded_nodes::get_stewarded_node_by_id(&mut conn, id) {
                    Ok(Some(node)) => {
                        let mut view: StewardedNodeView = node.into();
                        // Join stewards with human display names
                        match crate::db::stewarded_nodes::list_stewards_for_node(&mut conn, id) {
                            Ok(stewards) => {
                                view.stewards = stewards
                                    .into_iter()
                                    .map(|s| {
                                        let name = crate::db::humans::get_human_by_id(
                                            &mut conn,
                                            &s.human_id,
                                        )
                                        .ok()
                                        .flatten()
                                        .map(|h| h.display_name)
                                        .unwrap_or_else(|| s.human_id.clone());
                                        crate::views::node_stewardship_view_from_with_name(s, name)
                                    })
                                    .collect();
                            }
                            Err(e) => {
                                warn!(error = %e, node_id = id, "Failed to load stewards for node");
                            }
                        }
                        Ok(response::ok(&view))
                    }
                    Ok(None) => Ok(response::not_found("Node not found")),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// POST /db/nodes/{id}/stewardship — add a stewardship relationship to a node
    async fn handle_node_stewardship(
        &self,
        req: Request<Incoming>,
        method: Method,
        node_id: &str,
        _app_ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| StorageError::Internal("Database pool not initialized".into()))?;
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Failed to get connection: {}", e)))?;

        match method {
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
                    .to_bytes();
                let mut input_view: CreateNodeStewardshipInputView = serde_json::from_slice(&body)
                    .map_err(|e| StorageError::InvalidInput(format!("Invalid JSON: {}", e)))?;
                // Override node_id from URL path segment (authoritative)
                input_view.node_id = node_id.to_string();
                let input: crate::db::stewarded_nodes::CreateNodeStewardshipInput =
                    input_view.into();

                match crate::db::stewarded_nodes::create_node_stewardship(&mut conn, input) {
                    Ok(stewardship) => {
                        let name =
                            crate::db::humans::get_human_by_id(&mut conn, &stewardship.human_id)
                                .ok()
                                .flatten()
                                .map(|h| h.display_name)
                                .unwrap_or_else(|| stewardship.human_id.clone());
                        let view =
                            crate::views::node_stewardship_view_from_with_name(stewardship, name);
                        Ok(response::created(&view))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    // =========================================================================
    // Gate Decision Attestation Handlers (mishpat DHT projection, read-only)
    // =========================================================================

    /// GET /db/gate-decisions — List gate decisions with optional filters
    ///
    /// Query params (all optional):
    ///   ?elohim={id}   — filter by elohim agent public key
    ///   ?gate={name}   — filter by gate name
    ///   ?phase={phase} — filter by deployment phase
    ///   ?limit={n}     — max results (default 50, max 500)
    ///
    /// Returns `{ items: GateDecisionAttestationView[], count: number }`.
    /// Source of truth is the mishpat DNA DHT; this serves the read-optimised
    /// SQLite projection populated by `MishpatSignal::GateDecisionCreated`.
    async fn handle_gate_decisions_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        _ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        #[derive(serde::Deserialize, Default)]
        #[serde(rename_all = "camelCase")]
        struct GateDecisionQuery {
            elohim: Option<String>,
            gate: Option<String>,
            phase: Option<String>,
            limit: Option<i64>,
        }

        let query_str = req.uri().query().unwrap_or("");
        let query: GateDecisionQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

        let raw_limit = query.limit.unwrap_or(50).clamp(1, 500);

        let mut conn = self.get_diesel_conn()?;

        // Apply the most-selective filter available, then enforce limit in Rust
        // (the individual find_by_* functions don't take a limit parameter — we
        // keep the DB queries simple and trim in application code because the
        // gate-decisions table will remain small — one row per gate evaluation).
        // Phase-2a consolidation: gate decisions live in the unified `attestations`
        // table as `attestation:gate-decision` (the per-app gate-decision projection
        // table was dropped). The unified table is CID-global (no app_id scope), so the
        // app_id filter is no longer applied. The gate-name / phase fields are read from
        // `evidence_json` via json_extract; elohim is the issuer_cid.
        let rows = match (&query.elohim, &query.gate, &query.phase) {
            (Some(elohim_id), _, _) => {
                crate::db::attestations::gate_decision_find_by_elohim(&mut conn, elohim_id)?
            }
            (_, Some(gate_name), _) => {
                crate::db::attestations::gate_decision_find_by_gate(&mut conn, gate_name)?
            }
            (_, _, Some(phase)) => {
                crate::db::attestations::gate_decision_find_by_phase(&mut conn, phase)?
            }
            _ => crate::db::attestations::gate_decision_list_all(&mut conn, raw_limit)?,
        };

        let views: Vec<GateDecisionAttestationView> = rows
            .into_iter()
            .take(raw_limit as usize)
            .map(crate::views_convert::qahal::gate_decision_view_from_attestation)
            .collect();

        let count = views.len();
        Ok(response::ok(&serde_json::json!({
            "items": views,
            "count": count,
        })))
    }

    /// GET /db/gate-decisions/{cid} — Fetch a single decision by its CID
    ///
    /// Returns `GateDecisionAttestationView` on hit, 404 on miss.
    /// The `{cid}` segment is the `decision_id` field (a self-addressing CID).
    async fn handle_gate_decision_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        decision_id: &str,
        _ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;

        match crate::db::attestations::gate_decision_find_by_id(&mut conn, decision_id)? {
            Some(row) => Ok(response::ok(
                &crate::views_convert::qahal::gate_decision_view_from_attestation(row),
            )),
            None => Ok(response::not_found(&format!(
                "Gate decision not found: {}",
                decision_id
            ))),
        }
    }

    // =========================================================================
    // Gate Decision Challenge Handlers (mishpat DHT projection, read-only)
    // =========================================================================

    /// GET /db/gate-decision-challenges — List challenges with optional filters
    ///
    /// Query params (all optional):
    ///   ?challengerId=...             — filter by challenger AgentPubKey
    ///   ?challengedDecisionCid=...    — filter by challenged decision CID
    ///   ?limit={n}                    — max results (default 50, max 500)
    ///
    /// Returns `{ items: GateDecisionChallengeView[], count: number }`.
    /// Source of truth is the mishpat DNA DHT; this serves the read-optimised
    /// SQLite projection populated by `MishpatSignal::GateDecisionChallengeCreated`.
    async fn handle_gate_decision_challenges_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        #[derive(serde::Deserialize, Default)]
        #[serde(rename_all = "camelCase")]
        struct ChallengeQuery {
            challenger_id: Option<String>,
            challenged_decision_cid: Option<String>,
            limit: Option<i64>,
        }

        let uri = req.uri();
        let query: ChallengeQuery = uri
            .query()
            .and_then(|q| serde_urlencoded::from_str(q).ok())
            .unwrap_or_default();

        let raw_limit = query.limit.unwrap_or(50).clamp(1, 500);
        let mut conn = self.get_diesel_conn()?;

        let rows = match (&query.challenger_id, &query.challenged_decision_cid) {
            (Some(challenger_id), _) => crate::db::gate_decision_challenges::find_by_challenger(
                &mut conn,
                &ctx.h_app_id,
                challenger_id,
            )?,
            (_, Some(challenged_decision_cid)) => {
                crate::db::gate_decision_challenges::find_by_challenged_decision(
                    &mut conn,
                    &ctx.h_app_id,
                    challenged_decision_cid,
                )?
            }
            _ => {
                use crate::db::diesel_schema::gate_decision_challenges::dsl;
                use diesel::prelude::*;
                dsl::gate_decision_challenges
                    .filter(dsl::app_id.eq(&ctx.h_app_id))
                    .order(dsl::filed_at.desc())
                    .limit(raw_limit)
                    .load::<crate::db::gate_decision_challenges::GateDecisionChallengeRow>(
                        &mut conn,
                    )?
            }
        };

        let views: Vec<GateDecisionChallengeView> = rows
            .into_iter()
            .take(raw_limit as usize)
            .map(GateDecisionChallengeView::from)
            .collect();
        let count = views.len();
        Ok(response::ok(&serde_json::json!({
            "items": views,
            "count": count,
        })))
    }

    /// GET /db/gate-decision-challenges/{cid} — Fetch a single challenge by its CID
    ///
    /// Returns `GateDecisionChallengeView` on hit, 404 on miss.
    /// The `{cid}` segment is the `challenge_id` field (a self-addressing CID).
    async fn handle_gate_decision_challenge_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        challenge_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;

        match crate::db::gate_decision_challenges::find_by_id(
            &mut conn,
            &ctx.h_app_id,
            challenge_id,
        )? {
            Some(row) => Ok(response::ok(&GateDecisionChallengeView::from(row))),
            None => Ok(response::not_found(&format!(
                "Gate decision challenge not found: {}",
                challenge_id
            ))),
        }
    }

    // =========================================================================
    // Challenge Outcome Handlers (mishpat DHT projection, read-only)
    // =========================================================================

    /// GET /db/challenge-outcomes — List challenge outcomes with optional filters
    ///
    /// Query params (all optional):
    ///   ?challengeCid=...   — filter by challenge CID (returns at most one outcome)
    ///   ?verdict=...        — filter by verdict: upheld | dismissed | superseded
    ///   ?limit={n}          — max results (default 50, max 500)
    ///
    /// Returns `{ items: ChallengeOutcomeView[], count: number }`.
    /// Source of truth is the mishpat DNA DHT; this serves the read-optimised
    /// SQLite projection populated by `MishpatSignal::ChallengeOutcomeCreated`.
    async fn handle_challenge_outcomes_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        #[derive(serde::Deserialize, Default)]
        #[serde(rename_all = "camelCase")]
        struct OutcomeQuery {
            challenge_cid: Option<String>,
            verdict: Option<String>,
            limit: Option<i64>,
        }

        let uri = req.uri();
        let query: OutcomeQuery = uri
            .query()
            .and_then(|q| serde_urlencoded::from_str(q).ok())
            .unwrap_or_default();

        let raw_limit = query.limit.unwrap_or(50).clamp(1, 500);
        let mut conn = self.get_diesel_conn()?;

        let rows = match (&query.challenge_cid, &query.verdict) {
            (Some(challenge_cid), _) => {
                // challenge_cid lookup returns at most one row
                crate::db::challenge_outcomes::find_by_challenge(
                    &mut conn,
                    &ctx.h_app_id,
                    challenge_cid,
                )?
                .into_iter()
                .collect()
            }
            (_, Some(verdict)) => {
                crate::db::challenge_outcomes::find_by_verdict(&mut conn, &ctx.h_app_id, verdict)?
            }
            _ => {
                use crate::db::diesel_schema::challenge_outcomes::dsl;
                use diesel::prelude::*;
                dsl::challenge_outcomes
                    .filter(dsl::app_id.eq(&ctx.h_app_id))
                    .order(dsl::decided_at.desc())
                    .limit(raw_limit)
                    .load::<crate::db::challenge_outcomes::ChallengeOutcomeRow>(&mut conn)?
            }
        };

        let views: Vec<ChallengeOutcomeView> = rows
            .into_iter()
            .take(raw_limit as usize)
            .map(ChallengeOutcomeView::from)
            .collect();
        let count = views.len();
        Ok(response::ok(&serde_json::json!({
            "items": views,
            "count": count,
        })))
    }

    /// GET /db/challenge-outcomes/{cid} — Fetch a single outcome by its CID
    ///
    /// Returns `ChallengeOutcomeView` on hit, 404 on miss.
    /// The `{cid}` segment is the `outcome_id` field (a self-addressing CID).
    async fn handle_challenge_outcome_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        outcome_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;

        match crate::db::challenge_outcomes::find_by_id(&mut conn, &ctx.h_app_id, outcome_id)? {
            Some(row) => Ok(response::ok(&ChallengeOutcomeView::from(row))),
            None => Ok(response::not_found(&format!(
                "Challenge outcome not found: {}",
                outcome_id
            ))),
        }
    }

    // =========================================================================
    // Elohim Reputation Handler (Phase 11 Task 11.3 — computed aggregation)
    // =========================================================================

    /// GET /db/elohim-reputation — Compute a multi-dimensional reputation profile
    ///
    /// Query params:
    ///   ?elohimId=<AgentPubKey>    — required
    ///   ?windowStart=<ISO 8601>    — optional (default: now - 90 days)
    ///   ?windowEnd=<ISO 8601>      — optional (default: now)
    ///
    /// Returns `ElohimReputationProfileView`. Never 404 — an elohim with no
    /// decisions in the window returns a valid profile with all-zero counts.
    /// Returns 400 on invalid ISO-8601 or missing elohimId.
    ///
    /// Reputation is observed, not declared. The profile is computed from the
    /// mishpat DNA outcome graph: GateDecisionAttestations (phase=elohim-active)
    /// -> GateDecisionChallenges -> ChallengeOutcomes.
    async fn handle_elohim_reputation(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        #[derive(serde::Deserialize, Default)]
        #[serde(rename_all = "camelCase")]
        struct ReputationQueryParams {
            elohim_id: Option<String>,
            window_start: Option<String>,
            window_end: Option<String>,
        }

        let params: ReputationQueryParams = req
            .uri()
            .query()
            .and_then(|q| serde_urlencoded::from_str(q).ok())
            .unwrap_or_default();

        let elohim_id = match params.elohim_id {
            Some(id) if !id.is_empty() => id,
            _ => {
                return Ok(response::bad_request(
                    "Missing required query param: elohimId",
                ));
            }
        };

        // Default window: 90 days ending now.
        let now = chrono::Utc::now();
        let default_start = now - chrono::Duration::days(90);

        let window_start = match params.window_start {
            Some(ref s) => {
                // Validate ISO-8601 by attempting a parse; we store as the
                // original string so the view echoes what was requested.
                if chrono::DateTime::parse_from_rfc3339(s).is_err() {
                    return Ok(response::bad_request(&format!(
                        "Invalid ISO-8601 for windowStart: {}",
                        s
                    )));
                }
                s.clone()
            }
            None => default_start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        };

        let window_end = match params.window_end {
            Some(ref s) => {
                if chrono::DateTime::parse_from_rfc3339(s).is_err() {
                    return Ok(response::bad_request(&format!(
                        "Invalid ISO-8601 for windowEnd: {}",
                        s
                    )));
                }
                s.clone()
            }
            None => now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        };

        let q = crate::db::elohim_reputation::ReputationQuery {
            app_id: ctx.h_app_id.clone(),
            elohim_id: elohim_id.clone(),
            window_start: window_start.clone(),
            window_end: window_end.clone(),
        };

        let mut conn = self.get_diesel_conn()?;
        let result = crate::db::elohim_reputation::compute(&mut conn, &q)?;

        let view = crate::views::elohim_reputation_profile_view_from_result(
            elohim_id,
            window_start,
            window_end,
            result,
        );

        Ok(response::ok(&view))
    }

    // =========================================================================
    // Conductor-Bridge Handlers (Phase 10 HTTP pipes, Phase 11 full zome queries)
    // =========================================================================

    /// GET /db/source-chain/{agent_id}/entries?filter={filter}
    ///
    /// Conductor-bridge endpoint for the gate-client `SourceChainResolver`.
    ///
    /// # Phase 10 status
    ///
    /// The HTTP pipe is real and routed end-to-end.  The conductor query layer
    /// is deferred to Phase 11 (requires `HcClient` in `HttpServer`).  This
    /// handler currently returns an empty array so gates degrade honestly rather
    /// than erroring.
    ///
    /// # Phase 11 migration path
    ///
    /// 1. Add `hc_client: Option<Arc<HcClient>>` to `HttpServer`.
    /// 2. Wire it via a new `with_hc_client()` builder method in `main.rs`.
    /// 3. Replace the stub body below with:
    ///    `hc_client.call_zome(role, "content_store", "query_by_content_type", filter_payload)`
    ///    or the appropriate coordinator-zome function once it exists.
    ///
    /// # Gate-client convention
    ///
    /// `SourceChainResolver` constructs the query as:
    /// `/{agent_id}/entries?filter={filter}`
    /// This handler strips the leading `{agent_id}/entries` path and reads the
    /// optional `filter` query parameter.
    async fn handle_source_chain_entries(
        &self,
        req: Request<Incoming>,
        method: Method,
        sc_path: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        // Parse agent_id from path: expect "{agent_id}/entries"
        let agent_id = sc_path
            .strip_suffix("/entries")
            .unwrap_or(sc_path)
            .trim_end_matches('/');

        if agent_id.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "agent_id is required: GET /db/source-chain/{agent_id}/entries"}"#,
                )))
                .unwrap());
        }

        let filter = req
            .uri()
            .query()
            .and_then(|q| {
                serde_urlencoded::from_str::<std::collections::HashMap<String, String>>(q).ok()
            })
            .and_then(|m| m.get("filter").cloned())
            .unwrap_or_default();

        // Phase 10: conductor query not yet wired (Phase 11 — add HcClient to HttpServer).
        // Return empty array so SourceChainResolver degrades to Value::Null → gate continues.
        warn!(
            agent_id = %agent_id,
            filter = %filter,
            "source-chain bridge: conductor query deferred to Phase 11 \
             (HcClient not wired into HttpServer); returning empty entries"
        );

        Ok(response::ok(&serde_json::json!({
            "agentId": agent_id,
            "filter": filter,
            "entries": [],
            "phase": "10-stub",
            "note": "Phase 11: wire HcClient to HttpServer and query content_store zome"
        })))
    }

    /// GET /db/dht/{entry_hash}
    ///
    /// Conductor-bridge endpoint for the gate-client `DhtResolver`.
    ///
    /// # Phase 10 status
    ///
    /// The HTTP pipe is real and routed end-to-end.  The conductor query layer
    /// is deferred to Phase 11 (requires `HcClient` in `HttpServer`).  This
    /// handler returns 404 so `DhtResolver` degrades to `Value::Null`.
    ///
    /// # Phase 11 migration path
    ///
    /// 1. Add `hc_client: Option<Arc<HcClient>>` to `HttpServer`.
    /// 2. Wire it via a new `with_hc_client()` builder method in `main.rs`.
    /// 3. Replace the stub body below with:
    ///    `hc_client.call_zome(role, "content_store", "get_content_by_id", entry_hash_payload)`
    ///    and serialize the returned entry as JSON.
    ///
    /// # Gate-client convention
    ///
    /// `DhtResolver` constructs the query as `/{entry_hash}`.
    /// This handler receives the stripped `{entry_hash}` segment.
    async fn handle_dht_entry(
        &self,
        _req: Request<Incoming>,
        method: Method,
        entry_hash: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let entry_hash = entry_hash.trim();
        if entry_hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "entry_hash is required: GET /db/dht/{entry_hash}"}"#,
                )))
                .unwrap());
        }

        // Phase 10: conductor query not yet wired (Phase 11 — add HcClient to HttpServer).
        // Return 404 so DhtResolver degrades to Value::Null → gate continues.
        warn!(
            entry_hash = %entry_hash,
            "dht bridge: conductor query deferred to Phase 11 \
             (HcClient not wired into HttpServer); returning 404"
        );

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(
                serde_json::to_vec(&serde_json::json!({
                    "error": "DHT entry not found (conductor bridge deferred to Phase 11)",
                    "entryHash": entry_hash,
                    "phase": "10-stub"
                }))
                .unwrap_or_default(),
            )))
            .unwrap())
    }

    // =========================================================================
    // Session Handlers (Tauri Native Handoff)
    // =========================================================================

    /// GET /session - Get active local session
    ///
    /// Returns the currently active session for native app use.
    /// Returns 404 if no active session exists (first run).
    async fn handle_get_session(
        &self,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        match db::local_sessions::get_active_session(&mut conn)? {
            Some(session) => Ok(response::ok(&LocalSessionView::from(session))),
            None => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "No active session"}"#)))
                .unwrap()),
        }
    }

    /// POST /session - Create a new local session
    ///
    /// Called after OAuth handoff from doorway to store session locally.
    /// Automatically deactivates any existing sessions.
    async fn handle_create_session(
        &self,
        req: Request<Incoming>,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let bytes = body.to_bytes();

        let input: db::local_sessions::CreateLocalSessionInput = serde_json::from_slice(&bytes)
            .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        let session = db::local_sessions::create_session(&mut conn, input)?;

        info!(
            session_id = %session.id,
            human_id = %session.human_id,
            "Created local session"
        );

        Ok(response::created(&LocalSessionView::from(session)))
    }

    /// POST /session/exchange — steward-side portal session exchange (GAP-2b).
    ///
    /// A browser redirected from a doorway login portal lands at the steward's
    /// portal-host origin carrying a single-use redemption token plus the
    /// doorway URL that issued it. Storage:
    ///   1. ALLOWLIST: the `doorwayUrl` must already appear (origin-normalized)
    ///      in this steward's `local_sessions` history — the TOFU anti-spoofing
    ///      boundary. Never redeem against an issuer named only by the request.
    ///      Unknown issuer → 403.
    ///   2. REDEEM: back-channel `POST {doorwayUrl}/auth/handoff/redeem`. A 401
    ///      or any non-200 → 401 (invalid token); a network error → 502.
    ///   3. SEED: on success, build a `CreateLocalSessionInput` from the redeemed
    ///      snapshot + the *validated* doorway URL, create the session
    ///      (deactivating priors, as `POST /session` does), and respond 201 with
    ///      the `LocalSessionView` + `Set-Cookie: elohim_session=<id>`.
    ///
    /// Source of truth: the redemption token is Operational (single-use, ~5-min
    /// TTL, issuer-side truth — never persisted here). The conductor's
    /// `/auth/me` remains authority over doorway claims; this exchange only
    /// SEEDS a LocalSession.
    async fn handle_session_exchange(
        &self,
        req: Request<Incoming>,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let bytes = body.to_bytes();

        let input: ExchangeSessionTokenInputView = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(e) => {
                return Ok(response::bad_request(&format!("Invalid JSON: {}", e)));
            }
        };

        // The real redeem path uses storage's standard outbound reqwest client.
        // The orchestration is factored into `exchange_session_with_redeem` so
        // tests can inject a mock redeem without a live doorway.
        self.exchange_session_with_redeem(pool, input, |doorway_url, token| {
            Box::pin(async move {
                crate::services::session_exchange::redeem_token_http(&doorway_url, &token).await
            })
        })
        .await
    }

    /// Orchestrates the `/session/exchange` stages with an injectable redeem
    /// step. `redeem` is `Fn(validated_doorway_url, session_token) -> Future<RedeemOutcome>`
    /// so unit tests can drive the allowlist / create / cookie logic without a
    /// live doorway. See [`handle_session_exchange`] for the wire contract.
    async fn exchange_session_with_redeem<F, Fut>(
        &self,
        pool: DbPool,
        input: ExchangeSessionTokenInputView,
        redeem: F,
    ) -> Result<Response<Full<Bytes>>, StorageError>
    where
        F: FnOnce(String, String) -> Fut,
        Fut: std::future::Future<Output = crate::services::session_exchange::RedeemOutcome>,
    {
        use crate::services::session_exchange as ex;

        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        // --- Stage 1: TOFU allowlist -----------------------------------------
        // The doorwayUrl is attacker-controllable; it MUST already appear in the
        // steward's local_sessions history (the trust-on-first-use record).
        let history = db::local_sessions::list_all_sessions(&mut conn)?;
        let known = ex::known_doorway_urls(&history);
        if !ex::is_doorway_allowlisted(&input.doorway_url, known.iter().map(String::as_str)) {
            warn!(
                doorway_url = %input.doorway_url,
                "session/exchange rejected — issuer not in steward's doorway history (allowlist)"
            );
            return Ok(response::forbidden(&serde_json::json!({
                "error": "doorway not recognized",
                "detail": "The issuing doorway is not in this steward's session history. \
                           Redemption is only attempted against previously-trusted doorways."
            })));
        }

        // --- Stage 2: back-channel redeem ------------------------------------
        let outcome = redeem(input.doorway_url.clone(), input.session_token.clone()).await;
        let snapshot = match outcome {
            ex::RedeemOutcome::Ok(snap) => *snap,
            ex::RedeemOutcome::Invalid => {
                info!(
                    doorway_url = %input.doorway_url,
                    "session/exchange — doorway rejected token (invalid_token)"
                );
                return Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(r#"{"error":"invalid_token"}"#)))
                    .unwrap());
            }
            ex::RedeemOutcome::Upstream => {
                warn!(
                    doorway_url = %input.doorway_url,
                    "session/exchange — issuing doorway unreachable (upstream failure)"
                );
                return Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(r#"{"error":"doorway_unreachable"}"#)))
                    .unwrap());
            }
        };

        // --- Stage 3: seed the LocalSession + set cookie ---------------------
        // Always stamp the session with the *validated* doorway URL, never with
        // whatever the redeem response asserts about itself.
        let create_input = ex::build_create_input(&snapshot, &input.doorway_url);
        let session = db::local_sessions::create_session(&mut conn, create_input)?;
        let session_id = session.id.clone();

        info!(
            session_id = %session_id,
            human_id = %session.human_id,
            doorway_url = %input.doorway_url,
            "session/exchange — seeded LocalSession from doorway handoff"
        );

        let view = LocalSessionView::from(session);
        let body_json = serde_json::to_string(&view).unwrap_or_else(|_| "{}".to_string());
        Ok(Response::builder()
            .status(StatusCode::CREATED)
            .header(header::CONTENT_TYPE, "application/json")
            .header(
                header::SET_COOKIE,
                format!(
                    "elohim_session={}; HttpOnly; SameSite=Lax; Path=/",
                    session_id
                ),
            )
            .body(Full::new(Bytes::from(body_json)))
            .unwrap())
    }

    /// DELETE /session - Delete active session (logout)
    ///
    /// Deactivates and removes the currently active session.
    async fn handle_delete_session(
        &self,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        // Get active session first to know what we're deleting
        let active = db::local_sessions::get_active_session(&mut conn)?;

        if let Some(session) = active {
            db::local_sessions::delete_session(&mut conn, &session.id)?;
            info!(session_id = %session.id, "Deleted local session");
            Ok(response::ok(&serde_json::json!({
                "deleted": true,
                "sessionId": session.id
            })))
        } else {
            Ok(response::ok(&serde_json::json!({
                "deleted": false,
                "message": "No active session to delete"
            })))
        }
    }

    /// GET /session/all - List all sessions (for debugging)
    ///
    /// Returns all sessions including inactive ones.
    async fn handle_list_sessions(
        &self,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        let sessions = db::local_sessions::list_all_sessions(&mut conn)?;
        let views: Vec<LocalSessionView> =
            sessions.into_iter().map(LocalSessionView::from).collect();
        Ok(response::ok(&views))
    }

    /// GET /auth/me — Project LocalSession into the portal MeResponse shape.
    ///
    /// Mirrors the wire shape of `doorway/auth_routes.rs::MeResponse` (Task A2)
    /// so the standalone peer OAuth portal bundle can consume either source
    /// identically (spec §3.2 Transport β).  Storage projections always return
    /// `trustMode: "peer-conductor"` because the storage instance IS the
    /// conductor authority; there is no doorway-host concept on this path.
    ///
    /// Returns 200 + MeResponse on an active session, 401 + `{"error":…}` when
    /// no session exists (never returns a MeResponse body on the 401 path, to
    /// match doorway's contract).
    async fn handle_auth_me(
        &self,
        pool: DbPool,
        cookie_session_id: Option<String>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        #[derive(Debug, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct AuthorityRef {
            label: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            id: Option<String>,
        }

        /// Wire shape kept in sync with `doorway/auth_routes.rs::MeResponse`.
        /// When storage projects /auth/me, `trust_mode` is always
        /// `"peer-conductor"` and `doorway_id` / `doorway_url` are `None`.
        #[derive(Debug, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct StorageMeResponse {
            human_id: String,
            agent_pub_key: String,
            identifier: String,
            permission_level: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            doorway_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            doorway_url: Option<String>,
            authenticated: bool,
            /// Always `"peer-conductor"` for storage projections.
            trust_mode: String,
            authority: AuthorityRef,
            #[serde(skip_serializing_if = "Option::is_none")]
            conductor_endpoint: Option<String>,
        }

        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        // Prefer the session named by an `elohim_session` cookie when the
        // request carries one AND that session exists and is active (multi-
        // session browser path, GAP-2b). Otherwise fall through to the single
        // active session — Tauri-native compat must not change.
        let cookie_session = match cookie_session_id {
            Some(id) => match db::local_sessions::get_session_by_id(&mut conn, &id)? {
                Some(s) if s.is_active == 1 => Some(s),
                // Cookie names a missing or deactivated session — ignore it and
                // fall back to the active-session behavior.
                _ => None,
            },
            None => None,
        };

        let session = match cookie_session {
            Some(s) => s,
            None => {
                let Some(active) = db::local_sessions::get_active_session(&mut conn)? else {
                    return Ok(Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Full::new(Bytes::from(r#"{"error":"no active session"}"#)))
                        .unwrap());
                };
                active
            }
        };

        // Derive a human-readable conductor label from what we know about this
        // node.  Prefer the libp2p peer-id (set via with_self_peer_id); fall
        // back to the bind address; finally a generic phrase understood by the
        // trust-indicator chrome.
        let conductor_endpoint = if self.self_peer_id != "unknown-peer" {
            self.self_peer_id.clone()
        } else {
            let addr = self.bind_addr;
            if addr.ip().is_loopback() {
                "your conductor on this device".to_string()
            } else {
                addr.to_string()
            }
        };

        let resp = StorageMeResponse {
            human_id: session.human_id,
            agent_pub_key: session.agent_pub_key,
            identifier: session.identifier,
            // LocalSession has no permission_level field; default to "standard"
            // — the same default doorway uses when the JWT claim is absent.
            permission_level: "standard".to_string(),
            // doorway fields are None: we are the peer conductor, not a hosted
            // doorway (doorway_url still carries the origin the session was
            // created from, but on this endpoint we expose None to match the
            // trust-mode contract).
            doorway_id: None,
            doorway_url: None,
            authenticated: true,
            trust_mode: "peer-conductor".to_string(),
            authority: AuthorityRef {
                label: conductor_endpoint.clone(),
                id: Some(self.self_peer_id.clone()),
            },
            conductor_endpoint: Some(conductor_endpoint),
        };

        Ok(response::ok(&resp))
    }

    /// POST /session/intent - Set session intent for drift detection
    ///
    /// Declares what the user plans to do this session. Creates a set-point
    /// for the ElohimGate to detect behavioral drift.
    async fn handle_set_session_intent(
        &self,
        req: Request<Incoming>,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let bytes = body.to_bytes();

        let input: crate::views::SetSessionIntentInputView = serde_json::from_slice(&bytes)
            .map_err(|e| StorageError::Internal(format!("Invalid JSON: {}", e)))?;

        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        match db::local_sessions::get_active_session(&mut conn)? {
            Some(session) => {
                db::local_sessions::set_session_intent(&mut conn, &session.id, &input.intent)?;
                info!(session_id = %session.id, "Session intent set");
                Ok(response::ok(&serde_json::json!({ "status": "intent_set" })))
            }
            None => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "No active session"}"#)))
                .unwrap()),
        }
    }

    // =========================================================================
    // EPR Head Handlers
    // =========================================================================

    /// PUT /epr-head/{id} — Accept JSON, encode as DAG-CBOR, store blob, return CID.
    async fn handle_put_epr_head(
        &self,
        req: Request<Incoming>,
        id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if id.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing EPR Head ID"}"#)))
                .unwrap());
        }

        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let data = body.to_bytes();

        // Parse JSON input
        let input: EprHeadInputView = match serde_json::from_slice(&data) {
            Ok(v) => v,
            Err(e) => {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Invalid JSON: {e}"}}"#
                    ))))
                    .unwrap());
            }
        };

        // Convert to EprHead and encode as DAG-CBOR
        let mut head: crate::epr_codec::EprHead = input.into();
        // Override id from URL path
        head.id = id.to_string();

        let (cbor_bytes, cid) = match crate::epr_codec::encode_epr_head(&head) {
            Ok(result) => result,
            Err(e) => {
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Encoding failed: {e}"}}"#
                    ))))
                    .unwrap());
            }
        };

        // Store DAG-CBOR blob
        let store_result = self.blob_store.store_dag_cbor(&cbor_bytes).await?;

        // Build response
        let mut view: EprHeadView = head.into();
        view.cid = Some(cid.to_string());

        let response_body = serde_json::json!({
            "head": view,
            "cid": store_result.cid,
            "hash": store_result.hash,
            "sizeBytes": store_result.size_bytes,
            "alreadyExisted": store_result.already_existed,
        });

        Ok(Response::builder()
            .status(if store_result.already_existed {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            })
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(response_body.to_string())))
            .unwrap())
    }

    /// GET /epr-head/{id} — Retrieve EPR Head.
    /// Content-negotiation via Accept header:
    /// - `application/vnd.ipld.dag-cbor` → raw CBOR bytes
    /// - `application/json` (default) → JSON
    async fn handle_get_epr_head(
        &self,
        req: Request<Incoming>,
        id: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if id.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing EPR Head ID"}"#)))
                .unwrap());
        }

        // Look up EPR Head from content DB by ID
        if let Ok(mut conn) = self.get_conn() {
            let app_ctx = db::AppContext::default_lamad();
            // External HTTP handler — gate on provenance so we never surface a
            // row that has neither been notarized on Holochain nor published to
            // libp2p Kad.  No pillar enrichment: shefa/qahal remain at their
            // empty defaults for the public metadata surface.
            let head_opt = crate::epr_head::derive_epr_head(&mut conn, &app_ctx, id, true, false)?;

            if let Some(head) = head_opt {
                // Check Accept header for content negotiation
                let wants_cbor = req
                    .headers()
                    .get(header::ACCEPT)
                    .and_then(|v| v.to_str().ok())
                    .map(|a| a.contains("application/vnd.ipld.dag-cbor"))
                    .unwrap_or(false);

                if wants_cbor {
                    match crate::epr_codec::encode_epr_head(&head) {
                        Ok((cbor_bytes, _cid)) => {
                            return Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/vnd.ipld.dag-cbor")
                                .body(Full::new(Bytes::from(cbor_bytes)))
                                .unwrap());
                        }
                        Err(e) => {
                            return Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Full::new(Bytes::from(format!(
                                    r#"{{"error":"Encoding failed: {e}"}}"#
                                ))))
                                .unwrap());
                        }
                    }
                }

                // Default: JSON response
                let mut view: EprHeadView = head.clone().into();
                if let Ok((_bytes, cid)) = crate::epr_codec::encode_epr_head(&head) {
                    view.cid = Some(cid.to_string());
                }

                // Phase 5 T34: best-effort distribution-summary hydration.
                // Distribution is purely operational (Category C) — it must
                // never contaminate the canonical CBOR-encoded EprHead, so it
                // lives on the JSON view only. Visitor vs Steward branching
                // mirrors api/blob.rs::handle_distribution_details: if the
                // caller resolves to an agent_cid AND has active peer
                // bindings, hydrate as Steward; otherwise as Visitor.
                if let (Some(pool), Some(blob_hash)) = (
                    self.db_pool.as_ref(),
                    view_blob_hash_for_id(&mut conn, &app_ctx, id),
                ) {
                    let agent_cid_opt = crate::api::account::extract_agent_cid(&req, &mut conn)
                        .ok()
                        .flatten();
                    let bindings = if let Some(ref cid) = agent_cid_opt {
                        let now_iso = chrono::Utc::now().to_rfc3339();
                        crate::db::peer_identity_bindings::list_active_for_agent(
                            &mut conn, cid, &now_iso,
                        )
                        .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    let dist_ctx = match (&agent_cid_opt, bindings.is_empty()) {
                        (Some(cid), false) => {
                            crate::services::distribution_view::DistributionContext::Steward {
                                agent_cid: cid.as_str(),
                                bindings: &bindings,
                            }
                        }
                        _ => crate::services::distribution_view::DistributionContext::Visitor,
                    };
                    if let Ok(summary) =
                        crate::services::distribution_view::compose_distribution_summary(
                            pool, &blob_hash, dist_ctx,
                        )
                        .await
                    {
                        view.distribution = Some(summary);
                    }
                }

                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(
                        serde_json::to_string(&view).unwrap(),
                    )))
                    .unwrap());
            }
        }

        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(format!(
                r#"{{"error":"EPR Head not found: {id}"}}"#
            ))))
            .unwrap())
    }

    // =========================================================================
    // IPFS Block API
    // =========================================================================

    /// GET /ipfs/{cid} — Raw block retrieval by CID.
    ///
    /// Returns the raw bytes of a block stored by its CID.
    async fn handle_get_ipfs_block(
        &self,
        cid_str: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if cid_str.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing CID"}"#)))
                .unwrap());
        }

        match self.blob_store.get_by_address(cid_str).await {
            Ok(data) => Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::CONTENT_LENGTH, data.len().to_string())
                .body(Full::new(Bytes::from(data)))
                .unwrap()),
            Err(StorageError::NotFound(_)) | Err(StorageError::InvalidContentAddress(_)) => {
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Block not found: {cid_str}"}}"#
                    ))))
                    .unwrap())
            }
            Err(e) => Err(e),
        }
    }

    /// HEAD /ipfs/{cid} — Block existence check.
    async fn handle_head_ipfs_block(
        &self,
        cid_str: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if cid_str.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::new()))
                .unwrap());
        }

        match self.blob_store.exists_by_address(cid_str).await {
            Ok(true) => {
                let size = self.blob_store.size_by_address(cid_str).await.unwrap_or(0);
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/octet-stream")
                    .header(header::CONTENT_LENGTH, size.to_string())
                    .body(Full::new(Bytes::new()))
                    .unwrap())
            }
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::new()))
                .unwrap()),
        }
    }

    // =========================================================================
    // DAG API
    // =========================================================================

    /// GET /dag/{cid} — Decode DAG-CBOR block to JSON.
    ///
    /// Retrieves a block by CID and decodes it from DAG-CBOR to JSON for display.
    async fn handle_get_dag(&self, cid_str: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        if cid_str.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing CID"}"#)))
                .unwrap());
        }

        let data = match self.blob_store.get_by_address(cid_str).await {
            Ok(data) => data,
            Err(StorageError::NotFound(_)) | Err(StorageError::InvalidContentAddress(_)) => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Block not found: {cid_str}"}}"#
                    ))))
                    .unwrap());
            }
            Err(e) => return Err(e),
        };

        // Try decoding as DAG-CBOR → JSON
        match serde_ipld_dagcbor::from_slice::<serde_json::Value>(&data) {
            Ok(value) => {
                let json = serde_json::to_string_pretty(&value).unwrap_or_default();
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(json)))
                    .unwrap())
            }
            Err(_) => {
                // Not DAG-CBOR, return error with hint
                Ok(Response::builder()
                    .status(StatusCode::UNPROCESSABLE_ENTITY)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Block is not DAG-CBOR","cid":"{cid_str}","hint":"Use /ipfs/{cid_str} for raw bytes"}}"#
                    ))))
                    .unwrap())
            }
        }
    }

    /// GET /dag/{cid}/links — List CIDs linked from a DAG-CBOR block.
    async fn handle_get_dag_links(
        &self,
        cid_str: &str,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if cid_str.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error":"Missing CID"}"#)))
                .unwrap());
        }

        // Parse CID to determine codec
        let cid = match cid::Cid::from_str(cid_str) {
            Ok(c) => c,
            Err(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Invalid CID: {cid_str}"}}"#
                    ))))
                    .unwrap());
            }
        };

        let data = match self.blob_store.get_by_address(cid_str).await {
            Ok(data) => data,
            Err(StorageError::NotFound(_)) | Err(StorageError::InvalidContentAddress(_)) => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error":"Block not found: {cid_str}"}}"#
                    ))))
                    .unwrap());
            }
            Err(e) => return Err(e),
        };

        let links = crate::dag_store::extract_links(&data, cid.codec());
        let link_strs: Vec<String> = links.iter().map(|c| c.to_string()).collect();

        let response = serde_json::json!({
            "cid": cid_str,
            "links": link_strs,
            "count": link_strs.len(),
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(response.to_string())))
            .unwrap())
    }

    // =========================================================================
    // Human Directory Handler (Qahal - Community Directory)
    // =========================================================================

    /// GET /db/humans - List all humans for the app
    async fn handle_list_humans(
        &self,
        _req: Request<Incoming>,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        self.handle_list_humans_inner(ctx).await
    }

    /// Inner body of `GET /db/humans`. Receives the operating `ctx` but IGNORES it:
    /// humans are the imagodei identity projection, scoped to the canonical
    /// `HUMANS_HAPP_ID`, NOT the operating content scope — else a populated table
    /// reads empty (the 2026-06-19 dark-card probe artifact). Extracted so
    /// `test_list_db_humans` can prove the handler ignores the operating ctx (the
    /// test goes RED if this is ever reverted to `&ctx.h_app_id`).
    async fn handle_list_humans_inner(
        &self,
        _ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;
        match humans::list_humans(&mut conn, crate::db::context::HUMANS_HAPP_ID) {
            Ok(items) => {
                let views: Vec<HumanView> = items.into_iter().map(HumanView::from).collect();
                let body = serde_json::json!({
                    "items": views,
                    "count": views.len(),
                });
                Ok(response::ok(&body))
            }
            Err(e) => Ok(response::error_response(StorageError::Internal(format!(
                "Failed to list humans: {}",
                e
            )))),
        }
    }

    // =========================================================================
    // Collective Handlers (Qahal - Governance Contexts)
    // =========================================================================

    /// GET/POST /db/collectives - List or create collectives
    async fn handle_collectives_list(
        &self,
        req: Request<Incoming>,
        method: Method,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                let query_str = req.uri().query().unwrap_or("");
                let query: collectives::CollectiveQuery =
                    serde_urlencoded::from_str(query_str).unwrap_or_default();

                match collectives::list_collectives(&mut conn, ctx, &query) {
                    Ok(items) => {
                        let views: Vec<CollectiveView> =
                            items.into_iter().map(CollectiveView::from).collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
                let input_view: CreateCollectiveInputView =
                    serde_json::from_slice(&body.to_bytes())
                        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
                let input: collectives::CreateCollectiveInput = input_view.into();

                match collectives::create_collective(&mut conn, ctx, &input) {
                    Ok(c) => Ok(response::created(&CollectiveView::from(c))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// GET /db/collectives/{id}
    async fn handle_collective_by_id(
        &self,
        _req: Request<Incoming>,
        method: Method,
        id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        match collectives::get_collective(&mut conn, ctx, id) {
            Ok(Some(c)) => Ok(response::ok(&CollectiveView::from(c))),
            Ok(None) => Ok(response::not_found(&format!("Collective {} not found", id))),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET/POST /db/collectives/{id}/participants
    async fn handle_collective_participants(
        &self,
        req: Request<Incoming>,
        method: Method,
        collective_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = self.get_diesel_conn()?;

        match method {
            Method::GET => {
                match collectives::get_participants_of_collective(&mut conn, ctx, collective_id) {
                    Ok(items) => {
                        let views: Vec<CollectiveParticipationView> = items
                            .into_iter()
                            .map(CollectiveParticipationView::from)
                            .collect();
                        let body = serde_json::json!({
                            "items": views,
                            "count": views.len(),
                        });
                        Ok(response::ok(&body))
                    }
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            Method::POST => {
                let body = req
                    .collect()
                    .await
                    .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;

                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct AddParticipantInput {
                    human_id: String,
                    #[serde(default)]
                    intimacy_level: Option<String>,
                    #[serde(default)]
                    role_context: Option<String>,
                }

                let input: AddParticipantInput = serde_json::from_slice(&body.to_bytes())
                    .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

                let participation_input = collectives::CreateParticipationInput {
                    id: None,
                    collective_id: collective_id.to_string(),
                    human_id: input.human_id,
                    intimacy_level: input
                        .intimacy_level
                        .unwrap_or_else(|| "recognition".to_string()),
                    role_context: input.role_context,
                    governance_weight: 1.0,
                    consent_state: "consented".to_string(),
                    metadata_json: None,
                };

                match collectives::create_participation(&mut conn, ctx, &participation_input) {
                    Ok(p) => Ok(response::created(&CollectiveParticipationView::from(p))),
                    Err(e) => Ok(response::error_response(e)),
                }
            }
            _ => Ok(response::method_not_allowed()),
        }
    }

    /// DELETE /db/collectives/{id}/participants/{human_id} - Depart from collective
    async fn handle_collective_participant_depart(
        &self,
        _req: Request<Incoming>,
        method: Method,
        collective_id: &str,
        human_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::DELETE {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        match collectives::depart_collective(&mut conn, ctx, collective_id, human_id) {
            Ok(true) => Ok(response::ok(&serde_json::json!({"departed": true}))),
            Ok(false) => Ok(response::not_found("Participation not found")),
            Err(e) => Ok(response::error_response(e)),
        }
    }

    /// GET /db/participations/{human_id} - All collectives for a human
    async fn handle_participations_by_human(
        &self,
        _req: Request<Incoming>,
        method: Method,
        human_id: &str,
        ctx: &AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if method != Method::GET {
            return Ok(response::method_not_allowed());
        }

        let mut conn = self.get_diesel_conn()?;
        match collectives::get_participations_for_human(&mut conn, ctx, human_id) {
            Ok(items) => {
                let views: Vec<CollectiveParticipationView> = items
                    .into_iter()
                    .map(CollectiveParticipationView::from)
                    .collect();
                let body = serde_json::json!({
                    "items": views,
                    "count": views.len(),
                });
                Ok(response::ok(&body))
            }
            Err(e) => Ok(response::error_response(e)),
        }
    }

    // =========================================================================
    // Account Import/Export Handlers
    // =========================================================================

    /// POST /account/import - Import an account package
    ///
    /// Accepts an AccountPackageInputView and orchestrates:
    /// 1. Human relationship creation
    /// 2. Stewardship allocation creation
    /// 3. Collective participation creation
    ///
    /// Content assignments in an account package are viewer-relative recovery
    /// metadata. They must not overwrite the peer-global `content.reach` field.
    ///
    /// This endpoint serves both genesis seeding (initial conditions) and
    /// account recovery (restoring a human's world from a backup).
    async fn do_account_import<B>(
        &self,
        req: Request<B>,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError>
    where
        B: hyper::body::Body<Data = Bytes> + Unpin,
        B::Error: std::fmt::Display,
    {
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        let body_bytes = body.to_bytes();

        let package: AccountPackageInputView = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Parse(format!("Invalid account package JSON: {}", e)))?;

        let human_id = package.identity.human_id.clone();
        let ctx = AppContext::default_lamad();

        let item_count = package.relationships.len() + package.stewardship.len();

        // Pause P2P sync during bulk import to prevent memory pressure.
        // The guard resumes sync automatically when dropped (even on error/panic).
        #[cfg(feature = "p2p")]
        let _sync_guard = self.p2p_handle.as_ref().map(|h| {
            h.pause_sync(&format!(
                "account import for {} ({} items)",
                human_id, item_count
            ))
        });

        info!(
            human_id = %human_id,
            content_count = package.content.len(),
            relationship_count = package.relationships.len(),
            stewardship_count = package.stewardship.len(),
            "Importing account package"
        );

        let mut errors: Vec<String> = Vec::new();
        let content_updated: usize = 0;
        let mut relationships_created: usize = 0;
        let mut stewardship_created: usize = 0;

        if !package.content.is_empty() {
            info!(
                human_id = %human_id,
                content_assignments_ignored = package.content.len(),
                "Ignoring viewer-relative content assignments during account import"
            );
        }

        // Phase 1: Create human relationships (idempotent)
        //
        // Both Adam and Eve's packages may declare the same symmetric edge
        // (e.g. spouse). The directional unique index on human_relationships
        // would reject the second import; here we pre-check with bidirectional
        // awareness, and catch UNIQUE violations as a race-condition fallback
        // (same pattern as stewardship allocations below).
        let mut relationships_skipped: usize = 0;
        if !package.relationships.is_empty() {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

            for rel_seed in &package.relationships {
                let existing = human_relationships::find_existing_relationship(
                    &mut conn,
                    &ctx,
                    &human_id,
                    &rel_seed.target_id,
                    &rel_seed.relationship_type,
                    rel_seed.is_bidirectional,
                )?;

                if existing.is_some() {
                    relationships_skipped += 1;
                    continue;
                }

                let input = human_relationships::CreateHumanRelationshipInput {
                    id: None,
                    party_a_id: human_id.clone(),
                    party_b_id: rel_seed.target_id.clone(),
                    relationship_type: rel_seed.relationship_type.clone(),
                    intimacy_level: rel_seed.intimacy_level.clone(),
                    is_bidirectional: rel_seed.is_bidirectional,
                    consent_given_by_a: true,
                    consent_given_by_b: false,
                    initiated_by: human_id.clone(),
                    governance_layer: None,
                    reach: rel_seed
                        .reach
                        .clone()
                        .unwrap_or_else(|| "private".to_string()),
                    context_json: None,
                    expires_at: None,
                };

                match human_relationships::create_human_relationship(&mut conn, &ctx, input) {
                    Ok(_) => relationships_created += 1,
                    Err(StorageError::Internal(msg)) if msg.contains("UNIQUE constraint") => {
                        // Race: another concurrent import inserted between
                        // our pre-check and our insert. Same-shape row —
                        // treat as skipped, not an error.
                        relationships_skipped += 1;
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Failed to create relationship {} -> {}: {}",
                            human_id, rel_seed.target_id, e
                        ));
                    }
                }
            }

            info!(
                human_id = %human_id,
                relationships_created = relationships_created,
                relationships_skipped = relationships_skipped,
                "Human relationship creation complete"
            );
        }

        // Phase 2: Create stewardship allocations
        // Match stewardship seeds to content by category, create allocations
        if !package.stewardship.is_empty() {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

            // For each stewardship seed, we need a contributor presence for the human.
            // Look up or note that presences should already exist from seeding.
            for steward_seed in &package.stewardship {
                // Find content matching this category via content_type field
                use crate::db::diesel_schema::content;
                use diesel::prelude::*;

                let matching_content: Vec<String> = content::table
                    .filter(content::h_app_id.eq(&ctx.h_app_id))
                    .filter(content::content_type.eq(&steward_seed.content_category))
                    .select(content::id)
                    .load(&mut *conn)
                    .unwrap_or_default();

                // Create allocation for each matching content item
                // Note: This requires a presence_id for the human, which is resolved
                // by the seeder beforehand. For now, use human_id as presence reference.
                for content_id in &matching_content {
                    let input = stewardship_allocations::CreateAllocationInput {
                        content_id: content_id.clone(),
                        steward_presence_id: human_id.clone(),
                        allocation_ratio: steward_seed.allocation_ratio,
                        allocation_method: "computed".to_string(),
                        contribution_type: steward_seed
                            .contribution_type
                            .clone()
                            .unwrap_or_else(|| "inherited".to_string()),
                        contribution_evidence_json: None,
                        note: Some(format!(
                            "Account package import: {} stewardship",
                            steward_seed.content_category
                        )),
                        metadata_json: None,
                    };

                    match stewardship_allocations::create_allocation(&mut conn, &ctx, &input) {
                        Ok(_) => stewardship_created += 1,
                        Err(e) => {
                            // Duplicate allocations are expected on re-import
                            let err_msg = e.to_string();
                            if !err_msg.contains("UNIQUE constraint") {
                                errors.push(format!(
                                    "Failed to create allocation for {}: {}",
                                    content_id, e
                                ));
                            }
                        }
                    }
                }
            }

            info!(
                human_id = %human_id,
                stewardship_created = stewardship_created,
                "Stewardship allocation creation complete"
            );
        }

        // Phase 3: Create collective participations
        let mut collectives_joined: usize = 0;
        if !package.collectives.is_empty() {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

            let qahal_ctx = AppContext::new("qahal");

            for coll_seed in &package.collectives {
                // Ensure the FK parent exists before joining. Collective
                // definitions are seeded to a SINGLE peer (seed-collectives.ts)
                // while account packages import on each human's own
                // hash-selected peer — on every other peer the participation
                // insert hit `FOREIGN KEY constraint failed` (genesis #1105,
                // jessica-alpha storm; the old code probed get_collective and
                // discarded the result). Materialize a stub projection row
                // instead. Probe is app-scope-agnostic (the FK is on bare
                // collectives(id)); the stub is created under the legacy
                // "lamad" scope so the projection controller's
                // CollectiveProjected upsert and seed POSTs (both
                // lamad-scoped) converge on the same row rather than
                // PK-colliding on a second scope.
                match collectives::collective_id_exists(&mut conn, &coll_seed.collective_id) {
                    Ok(true) => {}
                    Ok(false) => {
                        let stub =
                            collectives::CreateCollectiveInput::stub(&coll_seed.collective_id);
                        if let Err(e) = collectives::create_collective(
                            &mut conn,
                            &AppContext::new("lamad"),
                            &stub,
                        ) {
                            errors.push(format!(
                                "Failed to materialize collective stub {}: {}",
                                coll_seed.collective_id, e
                            ));
                        }
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Collective existence probe failed for {}: {}",
                            coll_seed.collective_id, e
                        ));
                    }
                }

                let participation_input = collectives::CreateParticipationInput {
                    id: None,
                    collective_id: coll_seed.collective_id.clone(),
                    human_id: human_id.clone(),
                    intimacy_level: coll_seed
                        .intimacy_level
                        .clone()
                        .unwrap_or_else(|| "connection".to_string()),
                    role_context: coll_seed.role_context.clone(),
                    governance_weight: 1.0,
                    consent_state: "consented".to_string(),
                    metadata_json: None,
                };

                match collectives::create_participation(&mut conn, &qahal_ctx, &participation_input)
                {
                    Ok(_) => collectives_joined += 1,
                    Err(e) => {
                        errors.push(format!(
                            "Failed to join collective {}: {}",
                            coll_seed.collective_id, e
                        ));
                    }
                }
            }

            info!(
                human_id = %human_id,
                collectives_joined = collectives_joined,
                "Collective participation creation complete"
            );
        }

        let result = AccountImportResultView {
            human_id,
            content_updated,
            relationships_created,
            relationships_skipped,
            stewardship_created,
            collectives_joined,
            errors,
        };

        Ok(response::ok(&result))
    }

    /// GET /account/export/{human_id} - Export an account package
    ///
    /// Assembles an account package from the current state of a human's world:
    /// - Content assignments (what they can see and at what reach)
    /// - Human relationships (who they know)
    /// - Stewardship allocations (what they steward)
    /// - Collective participations (what collectives they belong to)
    ///
    /// This serves both backup/recovery and migration between conductors.
    async fn do_account_export(
        &self,
        human_id: &str,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let ctx = AppContext::default_lamad();
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

        info!(human_id = %human_id, "Exporting account package");

        // Phase 1: Gather content assignments
        // Content where this human has specific reach (via stewardship or relationships)
        let content_assignments: Vec<ContentAssignmentView> = {
            use crate::db::diesel_schema::content;
            use diesel::prelude::*;

            // Get all content in this app context — the reach field tells us the assignment
            let items: Vec<(String, String)> = content::table
                .filter(content::h_app_id.eq(&ctx.h_app_id))
                .select((content::id, content::reach))
                .load(&mut *conn)
                .map_err(|e| StorageError::Internal(format!("Content query failed: {}", e)))?;

            items
                .into_iter()
                .map(|(id, reach)| ContentAssignmentView {
                    content_id: id,
                    reach,
                    reason: None,
                    steward_ratio: None,
                })
                .collect()
        };

        // Phase 2: Gather human relationships
        let relationship_seeds: Vec<RelationshipSeedView> = {
            let query = human_relationships::HumanRelationshipQuery {
                party_id: Some(human_id.to_string()),
                limit: 10000,
                ..Default::default()
            };

            let rels = human_relationships::list_human_relationships(&mut conn, &ctx, &query)
                .unwrap_or_default();

            rels.into_iter()
                .map(|r| {
                    // Determine which party is the "other" one
                    let target = if r.party_a_id == human_id {
                        r.party_b_id
                    } else {
                        r.party_a_id
                    };

                    RelationshipSeedView {
                        target_id: target,
                        relationship_type: r.relationship_type,
                        intimacy_level: r.intimacy_level,
                        is_bidirectional: r.is_bidirectional == 1,
                        reach: Some(r.reach),
                    }
                })
                .collect()
        };

        // Phase 3: Gather stewardship allocations
        let stewardship_seeds: Vec<StewardshipSeedView> = {
            let allocs =
                stewardship_allocations::get_allocations_for_steward(&mut conn, &ctx, human_id)
                    .unwrap_or_default();

            // Group by content_type (category) and compute aggregate ratios
            let mut category_map: std::collections::HashMap<String, (f32, String)> =
                std::collections::HashMap::new();

            for alloc in &allocs {
                // Look up content_type for this content
                use crate::db::diesel_schema::content;
                use diesel::prelude::*;

                let content_type: Option<String> = content::table
                    .filter(content::id.eq(&alloc.content_id))
                    .select(content::content_type)
                    .first(&mut *conn)
                    .optional()
                    .unwrap_or(None);

                if let Some(ct) = content_type {
                    let entry = category_map
                        .entry(ct)
                        .or_insert((0.0, alloc.contribution_type.clone()));
                    entry.0 += alloc.allocation_ratio;
                }
            }

            category_map
                .into_iter()
                .map(
                    |(category, (ratio, contribution_type))| StewardshipSeedView {
                        content_category: category,
                        allocation_ratio: ratio,
                        contribution_type: Some(contribution_type),
                    },
                )
                .collect()
        };

        // Phase 4: Gather collective participations
        let collective_seeds: Vec<CollectiveSeedView> = {
            let qahal_ctx = AppContext::new("qahal");
            let participations =
                collectives::get_participations_for_human(&mut conn, &qahal_ctx, human_id)
                    .unwrap_or_default();

            participations
                .into_iter()
                .map(|p| CollectiveSeedView {
                    collective_id: p.collective_id,
                    role_context: p.role_context,
                    intimacy_level: Some(p.intimacy_level),
                })
                .collect()
        };

        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let package = AccountPackageView {
            identity: AccountIdentityView {
                human_id: human_id.to_string(),
                display_name: human_id.to_string(), // Will be enriched when identity service is integrated
                category: None,
                profile_reach: None,
                bio: None,
                location: None,
                affinities: vec![],
                organizations: vec![],
            },
            content: content_assignments,
            relationships: relationship_seeds,
            stewardship: stewardship_seeds,
            collectives: collective_seeds,
            manifest: PackageManifestView {
                version: "1.0.0".to_string(),
                generated_at: now,
                source_story: Some("export".to_string()),
                content_hash: None,
            },
        };

        info!(
            human_id = %human_id,
            content_count = package.content.len(),
            relationship_count = package.relationships.len(),
            stewardship_count = package.stewardship.len(),
            collectives_count = package.collectives.len(),
            "Account package exported"
        );

        Ok(response::ok(&package))
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    /// Get MIME type for a file path based on extension
    fn get_mime_type(path: &str) -> &'static str {
        match path.rsplit('.').next() {
            Some("html") | Some("htm") => "text/html; charset=utf-8",
            Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("json") => "application/json; charset=utf-8",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("ico") => "image/x-icon",
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            Some("ttf") => "font/ttf",
            Some("otf") => "font/otf",
            Some("eot") => "application/vnd.ms-fontobject",
            Some("wasm") => "application/wasm",
            Some("mp3") => "audio/mpeg",
            Some("mp4") => "video/mp4",
            Some("webm") => "video/webm",
            Some("ogg") => "audio/ogg",
            Some("wav") => "audio/wav",
            Some("txt") => "text/plain; charset=utf-8",
            Some("xml") => "application/xml",
            Some("pdf") => "application/pdf",
            Some("zip") => "application/zip",
            Some("map") => "application/json", // source maps
            _ => "application/octet-stream",
        }
    }

    // =========================================================================
    // Observation Session API
    // =========================================================================

    /// Dispatcher for `/api/v1/observations[/*]` requests.
    ///
    /// Routes:
    /// - `POST /api/v1/observations/begin`           -> begin a new session
    /// - `POST /api/v1/observations/{id}/entries`    -> append entries
    /// - `GET  /api/v1/observations/{id}/report`     -> generate/return report
    async fn handle_observation_request(
        &self,
        req: Request<Incoming>,
        method: Method,
        sub_path: &str,
        pool: DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        // POST /api/v1/observations/begin
        if sub_path == "/begin" && method == Method::POST {
            return self.handle_observation_begin(req, &pool).await;
        }

        // POST /api/v1/observations/{id}/entries
        // GET  /api/v1/observations/{id}/report
        if let Some(rest) = sub_path.strip_prefix('/') {
            let parts: Vec<&str> = rest.splitn(2, '/').collect();
            if parts.len() == 2 {
                let session_id = parts[0];
                match (method.clone(), parts[1]) {
                    (Method::POST, "entries") => {
                        return self
                            .handle_observation_add_entries(req, session_id, &pool)
                            .await;
                    }
                    (Method::GET, "report") => {
                        return self.handle_observation_report(session_id, &pool).await;
                    }
                    _ => {}
                }
            }
        }

        Ok(response::not_found(&format!(
            "Unknown observation route: /api/v1/observations{}",
            sub_path
        )))
    }

    /// POST /api/v1/observations/begin
    ///
    /// Begin a new observation session. Returns the session ID and computed expiry.
    async fn handle_observation_begin(
        &self,
        req: Request<Incoming>,
        pool: &DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body_bytes = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();

        let input: BeginObservationInputView = serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

        let metadata_str = input
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(&m.0).unwrap_or_default());

        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("DB connection failed: {}", e)))?;

        let session = db::observation_sessions::begin_session(
            &mut conn,
            &input.source,
            input.ttl_seconds,
            metadata_str.as_deref(),
        )
        .map_err(|e| StorageError::Internal(format!("Failed to begin session: {}", e)))?;

        // Compute expiry from started_at + ttl_seconds
        let expires_at = chrono::DateTime::parse_from_rfc3339(&session.started_at)
            .map(|dt| {
                (dt + chrono::Duration::seconds(session.ttl_seconds as i64))
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            })
            .unwrap_or_else(|_| session.started_at.clone());

        let view = BeginObservationResponseView {
            session_id: session.id,
            expires_at,
        };

        Ok(response::created(&view))
    }

    /// POST /api/v1/observations/{id}/entries
    ///
    /// Append one or an array of observation entries to an active session.
    /// Returns 201 with empty body on success.
    async fn handle_observation_add_entries(
        &self,
        req: Request<Incoming>,
        session_id: &str,
        pool: &DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("DB connection failed: {}", e)))?;

        // Guard: session must be active
        let is_active = db::observation_sessions::is_session_active(&mut conn, session_id)
            .map_err(|e| StorageError::Internal(format!("Session check failed: {}", e)))?;

        if !is_active {
            return Ok(response::bad_request(&format!(
                "Observation session '{}' is not active or does not exist",
                session_id
            )));
        }

        let body_bytes = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?
            .to_bytes();

        // Accept single entry or array
        let entries: Vec<ObservationEntryInputView> = if body_bytes.first() == Some(&b'[') {
            serde_json::from_slice(&body_bytes)
                .map_err(|e| StorageError::Parse(format!("Invalid JSON array: {}", e)))?
        } else {
            let single: ObservationEntryInputView = serde_json::from_slice(&body_bytes)
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;
            vec![single]
        };

        for entry in &entries {
            let context_str = entry
                .context
                .as_ref()
                .map(|c| serde_json::to_string(&c.0).unwrap_or_default());

            db::observation_sessions::append_entry(
                &mut conn,
                session_id,
                &entry.origin,
                &entry.category,
                &entry.severity,
                entry.method.as_deref(),
                entry.path.as_deref(),
                entry.status_code,
                &entry.message,
                context_str.as_deref(),
            )
            .map_err(|e| StorageError::Internal(format!("Append entry failed: {}", e)))?;
        }

        // 201 with empty body -- entries accepted
        Ok(Response::builder()
            .status(StatusCode::CREATED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from("{}")))
            .unwrap())
    }

    /// GET /api/v1/observations/{id}/report
    ///
    /// Generates a diagnostic report for the session, then closes the session
    /// and purges its entries. Idempotent: if a report_content_id is already
    /// recorded on the session, returns the existing report content ID.
    ///
    /// Content ID is stable: `obs-report-for-{scenarioId}` when scenarioId is
    /// present in session metadata, otherwise `obs-report-{sessionId}`.
    async fn handle_observation_report(
        &self,
        session_id: &str,
        pool: &DbPool,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let mut conn = pool
            .get()
            .map_err(|e| StorageError::Internal(format!("DB connection failed: {}", e)))?;

        let session = db::observation_sessions::get_session(&mut conn, session_id)
            .map_err(|e| StorageError::Internal(format!("Session lookup failed: {}", e)))?
            .ok_or_else(|| {
                StorageError::NotFound(format!("Observation session '{}' not found", session_id))
            })?;

        // Idempotent: return existing content ID if already reported
        if let Some(ref existing_id) = session.report_content_id {
            let body = serde_json::json!({ "contentId": existing_id, "cached": true });
            return Ok(response::ok(&body));
        }

        // Read all entries for this session
        let entries = db::observation_sessions::get_entries(&mut conn, session_id)
            .map_err(|e| StorageError::Internal(format!("Get entries failed: {}", e)))?;

        // Build summary counts
        let total_entries = entries.len();
        let mut by_origin: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut by_severity: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut by_category: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for entry in &entries {
            *by_origin.entry(entry.origin.clone()).or_insert(0) += 1;
            *by_severity.entry(entry.severity.clone()).or_insert(0) += 1;
            *by_category.entry(entry.category.clone()).or_insert(0) += 1;
        }

        // Correlate issues from entries
        let issues = correlate_issues(&entries);

        // Determine content ID -- stable per scenarioId if present in metadata
        let scenario_id = session
            .metadata_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| {
                v.get("scenarioId")
                    .and_then(|s| s.as_str())
                    .map(String::from)
            });

        let content_id = match scenario_id {
            Some(ref sid) => format!("obs-report-for-{}", sid),
            None => format!("obs-report-{}", session_id),
        };

        // Compute duration
        let ended_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let duration_ms = chrono::DateTime::parse_from_rfc3339(&session.started_at)
            .map(|start| {
                chrono::Utc::now()
                    .signed_duration_since(start)
                    .num_milliseconds()
            })
            .unwrap_or(0);

        // System state snapshot
        let system_state = ObservationSystemStateView {
            storage_healthy: self.db_pool.is_some(),
            conductor_connected: self.import_api.is_some(),
            p2p_peer_count: 0,
        };

        let metadata = session
            .metadata_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .map(crate::views::JsonVal);

        let entry_views: Vec<ObservationEntryView> = entries
            .iter()
            .cloned()
            .map(ObservationEntryView::from)
            .collect();

        let report = ObservationReportView {
            content_id: content_id.clone(),
            session_id: session_id.to_string(),
            source: session.source.clone(),
            metadata,
            duration: ObservationDurationView {
                started_at: session.started_at.clone(),
                ended_at: ended_at.clone(),
                duration_ms,
            },
            summary: ObservationSummaryView {
                total_entries,
                by_origin,
                by_severity,
                by_category,
            },
            issues,
            system_state,
        };

        // Persist report as a content node so it is addressable as an EPR
        let report_body = serde_json::to_string(&serde_json::json!({
            "entries": entry_views,
            "report": report,
        }))
        .unwrap_or_default();

        if let Some(ref services) = self.services {
            let create_input = db::content_diesel::CreateContentInput {
                id: content_id.clone(),
                title: format!("Observation Report: {}", session.source),
                description: Some(format!(
                    "Diagnostic report for session {} ({})",
                    session_id, session.source
                )),
                content_type: "observation-report".to_string(),
                content_format: "json".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: Some(report_body.len() as i32),
                metadata_json: session.metadata_json.clone(),
                reach: "familiar".to_string(),
                created_by: Some("elohim-storage".to_string()),
                tags: vec!["observation".to_string(), "diagnostic".to_string()],
                content_body: Some(report_body),
                dht_anchor_hash: None,
            };

            // Non-fatal: if content already exists (idempotent scenario), just log
            match services.content.create(create_input) {
                Ok(_) => {
                    debug!(content_id = %content_id, "Observation report persisted as content node");
                }
                Err(e) => {
                    debug!(
                        content_id = %content_id,
                        error = %e,
                        "Observation report content node already exists or failed (non-fatal)"
                    );
                }
            }
        }

        // Close session and purge entries
        db::observation_sessions::close_session(&mut conn, session_id, Some(&content_id))
            .map_err(|e| StorageError::Internal(format!("Close session failed: {}", e)))?;

        db::observation_sessions::purge_entries(&mut conn, session_id)
            .map_err(|e| StorageError::Internal(format!("Purge entries failed: {}", e)))?;

        Ok(response::ok(&report))
    }

    /// Observation middleware aspect.
    ///
    /// Called after the main request handler when `X-Observation-Id` is present.
    /// Appends an entry to the named session for any non-2xx response status.
    /// Successes (2xx) are not recorded -- only failures carry diagnostic signal.
    fn maybe_observe_request(&self, session_id: &str, method: &str, path: &str, status_code: u16) {
        // Only observe failures
        if status_code < 300 {
            return;
        }

        let pool = match self.db_pool.as_ref() {
            Some(p) => p,
            None => return,
        };

        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Infer category from path
        let category = if path.starts_with("/db/content") || path.starts_with("/api/v1/content") {
            "content"
        } else if path.starts_with("/db/allocations") || path.starts_with("/api/v1/stewardship") {
            "stewardship"
        } else if path.starts_with("/api/v1/mastery") {
            "mastery"
        } else if path.starts_with("/api/v1/governance") {
            "governance"
        } else if path.starts_with("/api/v1/identity") || path.starts_with("/db/humans") {
            "identity"
        } else if path.starts_with("/blob/") || path.starts_with("/shard/") {
            "blob"
        } else {
            "api"
        };

        // Infer severity from status code
        let severity = if status_code >= 500 {
            "error"
        } else if status_code >= 400 {
            "warning"
        } else {
            "info"
        };

        let message = format!("HTTP {} {} -> {}", method, path, status_code);

        let _ = db::observation_sessions::append_entry(
            &mut conn,
            session_id,
            "elohim-storage",
            category,
            severity,
            Some(method),
            Some(path),
            Some(status_code as i32),
            &message,
            None,
        );
    }

    /// Snapshot of the iroh-served counter.
    /// Exposed for integration-test and parity-soak diagnostics.
    pub fn blob_iroh_served_count_snapshot(&self) -> u64 {
        self.blob_iroh_served_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Snapshot of the libp2p-served counter.
    /// Exposed for integration-test and parity-soak diagnostics.
    pub fn blob_libp2p_served_count_snapshot(&self) -> u64 {
        self.blob_libp2p_served_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Test entry point: invoke the GET /blob/{hash} dispatcher with an
    /// explicit hash and optional caller agent CID. Wraps `handle_get_blob`
    /// with the same parameters the real router extracts from the request.
    /// Named `_for_test` to signal test-only intent; not guarded by
    /// `#[cfg(test)]` so integration test binaries can call it.
    pub async fn handle_blob_get_for_test(
        &self,
        hash: &str,
        agent_cid: Option<&str>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        self.handle_get_blob(hash, agent_cid).await
    }

    /// Test entry point: invoke the /health endpoint with default detail level.
    /// Returns the full health JSON body. Used to verify blob backend counters
    /// surface correctly in the health response.
    #[cfg(test)]
    pub async fn handle_status_for_test(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        self.handle_health("").await
    }

    /// Set the iroh-served counter to an arbitrary value for unit tests.
    #[cfg(test)]
    pub fn set_iroh_served_count_for_test(&self, n: u64) {
        self.blob_iroh_served_count
            .store(n, std::sync::atomic::Ordering::Relaxed);
    }

    /// Set the libp2p-served counter to an arbitrary value for unit tests.
    #[cfg(test)]
    pub fn set_libp2p_served_count_for_test(&self, n: u64) {
        self.blob_libp2p_served_count
            .store(n, std::sync::atomic::Ordering::Relaxed);
    }

    // ── Test helpers for integration tests (put_blob_dual_write, seed_e2e_dual_address) ──
    // Named `test_*` to signal test-only intent; NOT guarded by `#[cfg(test)]`
    // because integration test binaries in `tests/` compile the library without
    // `cfg(test)` set — only the binary itself carries that flag.

    /// Drive `put_blob_bytes` directly without an HTTP server bind.
    /// Used by integration tests in `tests/put_blob_dual_write.rs` and
    /// `tests/seed_e2e_dual_address.rs`.
    pub async fn test_put_blob(
        &self,
        sha256: &str,
        payload: &[u8],
        mime_type: &str,
    ) -> HttpTestResponse {
        let data = payload.to_vec();
        match self
            .put_blob_bytes(data, sha256, mime_type, "did:elohim:test")
            .await
        {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .unwrap()
                    .to_bytes();
                HttpTestResponse { status, body }
            }
            Err(e) => HttpTestResponse {
                status: 500,
                body: bytes::Bytes::from(format!("error: {e}")),
            },
        }
    }

    /// Drive `handle_get_blob` directly without an HTTP server bind.
    pub async fn test_get_blob(&self, hash: &str) -> HttpTestResponse {
        match self.handle_get_blob(hash, None).await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .unwrap()
                    .to_bytes();
                HttpTestResponse { status, body }
            }
            Err(e) => HttpTestResponse {
                status: 500,
                body: bytes::Bytes::from(format!("error: {e}")),
            },
        }
    }

    /// Drop the in-process shard-manifest cache, simulating a restart (or a
    /// peer that never minted the blob) without tearing down the server.
    pub async fn clear_manifest_cache_for_test(&self) {
        self.manifests.write().await.clear();
    }

    /// Drive the `/blob` route's peer-heal response mapping with bytes a peer
    /// supplied.
    ///
    /// No in-process peer exists in tests, so the peer race itself cannot be
    /// driven end-to-end; everything else is the production path — the local
    /// read runs for real and supplies the gap + manifest context, and the
    /// response is built by the same `blob_response_for_heal` the route calls.
    /// Returns `(status, content_type, etag)`.
    pub async fn test_blob_response_for_healed_bytes(
        &self,
        hash: &str,
        bytes: Vec<u8>,
    ) -> (u16, Option<String>, Option<String>) {
        let (local_gap, manifest_ctx) = match self.read_local_blob(hash).await {
            LocalBlobRead::Bytes { manifest, .. } => (None, manifest),
            LocalBlobRead::MissingShard {
                manifest,
                index,
                shard_hash,
            } => (Some((index, shard_hash)), Some(manifest)),
            LocalBlobRead::Absent => (None, None),
        };
        let outcome = BlobHealOutcome::Bytes {
            bytes,
            healed_from: Some("test-peer".to_string()),
        };
        let resp = self.blob_response_for_heal(hash, outcome, local_gap, manifest_ctx.as_deref());
        let header = |name: header::HeaderName| {
            resp.headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        (
            resp.status().as_u16(),
            header(header::CONTENT_TYPE),
            header(header::ETAG),
        )
    }

    /// Drive `get_blob_or_heal` directly — the path `/apps/{slug}` resolution
    /// takes. Returns the bytes only on the `Bytes` outcome so a test can tell
    /// "served" from every degraded outcome.
    pub async fn test_get_blob_or_heal(&self, hash: &str) -> Option<Vec<u8>> {
        match self.get_blob_or_heal(hash).await {
            BlobHealOutcome::Bytes { bytes, .. } => Some(bytes),
            _ => None,
        }
    }

    /// Drive `GET /db/rea_commitments?action={action}&doorwayId={doorway_id}` directly.
    ///
    /// Not guarded by `#[cfg(test)]` — integration test binaries in `tests/` compile
    /// the library without `cfg(test)` set (only the binary itself carries that flag).
    pub async fn test_get_db_rea_commitments(
        &self,
        action: &str,
        doorway_id: &str,
    ) -> HttpTestResponse {
        let app_ctx = db::AppContext::default_lamad();

        match self
            .handle_db_rea_commitments_inner(action, doorway_id, &app_ctx)
            .await
        {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .unwrap()
                    .to_bytes();
                HttpTestResponse { status, body }
            }
            Err(e) => HttpTestResponse {
                status: 500,
                body: bytes::Bytes::from(format!("error: {e}")),
            },
        }
    }

    /// Drive `GET /db/humans` for integration tests. `operating_scope` is the
    /// content scope the router would pass (e.g. "lamad"); the handler must IGNORE
    /// it and list the imagodei humans projection. RED if the handler is reverted to
    /// read the operating ctx. Not guarded by `#[cfg(test)]` — see note above.
    pub async fn test_list_db_humans(&self, operating_scope: &str) -> HttpTestResponse {
        let ctx = db::AppContext::new(operating_scope);
        match self.handle_list_humans_inner(&ctx).await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .unwrap()
                    .to_bytes();
                HttpTestResponse { status, body }
            }
            Err(e) => HttpTestResponse {
                status: 500,
                body: bytes::Bytes::from(format!("error: {e}")),
            },
        }
    }

    /// Inner handler: validate params and call find_active_projections.
    ///
    /// Extracted so both `handle_db_rea_commitments` (which parses query params from
    /// Request<Incoming>) and `test_get_db_rea_commitments` (which takes pre-parsed
    /// params) can share the same logic without constructing a full HTTP request.
    async fn handle_db_rea_commitments_inner(
        &self,
        action: &str,
        doorway_id: &str,
        ctx: &db::AppContext,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        if action != "project-epr" {
            return Ok(response::error_response(StorageError::InvalidInput(
                "GET /db/rea_commitments requires action=project-epr".into(),
            )));
        }
        if doorway_id.is_empty() {
            return Ok(response::error_response(StorageError::InvalidInput(
                "GET /db/rea_commitments requires doorwayId".into(),
            )));
        }
        let mut conn = self.get_diesel_conn()?;
        let views = db::rea_commitments::find_active_projections(&mut conn, ctx, doorway_id)?;
        // Return a BARE JSON array — the sole consumer is the doorway EprRouter
        // (doorway-service epr_router.rs::fetch_projections_from_storage), which
        // deserializes the body directly as `Vec<EprProjectionView>`. The {items,count}
        // wrapper used by other /db list routes would fail that deserialize and leave
        // the router empty. Do not wrap.
        Ok(response::ok(&views))
    }

    /// Expose the db pool for integration test assertions on
    /// `peer_blob_inventory` rows written by dual-write.
    /// Not guarded by `#[cfg(test)]` — see note above.
    pub fn db_pool(&self) -> Option<crate::db::DbPool> {
        self.db_pool.clone()
    }

    // ── Acquisition DevicePin handlers (spec §1.1, §4.4) ─────────────────────
    // Own-node only. Airplane-mode property: all three handlers touch only the
    // local `acquisition_pins` SQLite table — no network, no conductor.

    /// POST /api/v1/pins — create or idempotently re-pin a DevicePin.
    async fn handle_create_pin(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let body = req
            .collect()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
        self.handle_create_pin_bytes(&body.to_bytes()).await
    }

    /// GET /api/v1/pins — list all pins with optional per-pin pull enrichment.
    async fn handle_list_pins(&self) -> Result<Response<Full<Bytes>>, StorageError> {
        // Wire shape {pins, pull}: clients correlate pull[].pinId → pins[].id
        // (two parallel lists, joined by pin id; pull is null without p2p).
        let mut conn = self.get_diesel_conn()?;
        let pins = db::acquisition_pins::list_all_pins(&mut conn)
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let pin_views: Vec<_> = pins.into_iter().map(pin_to_view).collect();

        // Per-pin pull enrichment: read from the live AcquisitionState when p2p is
        // available; null otherwise (airplane-mode contract).
        #[cfg(feature = "p2p")]
        let pull: Option<Vec<crate::p2p::acquisition::PinPullStatus>> = {
            if let Some(ref handle) = self.p2p_handle {
                Some(handle.acquisition_per_pin().await)
            } else {
                None
            }
        };
        #[cfg(not(feature = "p2p"))]
        let pull: Option<Vec<crate::p2p::acquisition::PinPullStatus>> = None;

        let body = serde_json::json!({
            "pins": pin_views,
            "pull": pull,
        });
        Ok(response::ok(&body))
    }

    /// GET /api/v1/pins/{eprId}/pull — per-EPR pull progress (own node only).
    /// Deliberately absent from build_manifest(): a doorway MUST NEVER serve
    /// another agent's pull state. Groups all pins of `epr_id` by head_ref and
    /// counts shared content once (spec §4.3 / Slice 2b T13).
    ///
    /// The `epr:` prefix is stripped at the boundary so the queried id matches
    /// the bare `head_ref` stored by handle_create_pin (which strips it on POST).
    ///
    /// ## 404 means "not pinned here", not "not measured yet"
    ///
    /// A tracker is born in the acquisition reconcile cycle, not in `POST
    /// /api/v1/pins`. So between accepting a pin (201) and the next reconcile
    /// tick there is a window in which the node holds an ACTIVE pin for the EPR
    /// and has no `GapTracker` for it. Answering 404 there says "this node knows
    /// nothing about that EPR", which is false — and it is the window every
    /// caller hits, because the natural next request after pinning is "how is my
    /// pin doing?" (acquisition-pins.feature:40; the PinProgressComponent asks
    /// the same question).
    ///
    /// So the two absences are told apart:
    ///   * no active pin for this EPR on this node → 404 (genuinely not found)
    ///   * active pin, no tracker yet → 200 with the tri-state nulls
    ///     (`total: null`, `caughtUp: null`) already specified for
    ///     "cannot compute — keep waiting" (spec §4.3). Never a false-complete:
    ///     null caughtUp is not caught up.
    ///
    /// Without a wired `AcquisitionState` at all (no p2p) there is no pull
    /// subsystem to be unmeasured — that stays 404 (airplane-mode contract,
    /// `epr_pull_pinned_but_no_p2p_is_404`).
    async fn handle_epr_pull(&self, epr_id: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        // Canonicalize to the bare content id (epr: stripped) — pins store
        // head_ref bare, so the lookup key must be bare too.
        let epr_id = epr_id.strip_prefix("epr:").unwrap_or(epr_id);

        // pin_id → head_ref from the local table (own-node, airplane-mode).
        // head_ref is normalized to the bare id on the way in for the same
        // reason the query is: legacy rows written before handle_create_pin
        // stripped the prefix still carry `epr:`, and a stored-vs-queried
        // prefix mismatch would silently drop them out of the group.
        let mut conn = self.get_diesel_conn()?;
        let pins = db::acquisition_pins::list_all_pins(&mut conn)
            .map_err(|e| StorageError::Database(e.to_string()))?;
        drop(conn);
        let bare = |head_ref: &str| -> String {
            head_ref
                .strip_prefix("epr:")
                .unwrap_or(head_ref)
                .to_string()
        };
        // Is this EPR pinned here at all? Only an `active` pin counts — a
        // removed/retired pin is no longer a promise this node is keeping.
        let pinned_here = pins
            .iter()
            .any(|p| p.status == "active" && bare(&p.head_ref) == epr_id);
        let pin_heads: std::collections::HashMap<i32, String> = pins
            .into_iter()
            .map(|p| (p.id, bare(&p.head_ref)))
            .collect();

        #[cfg(feature = "p2p")]
        let (acquisition_wired, view) = {
            if let Some(ref handle) = self.p2p_handle {
                (true, handle.acquisition_per_epr(epr_id, &pin_heads).await)
            } else {
                (false, None)
            }
        };
        #[cfg(not(feature = "p2p"))]
        let (acquisition_wired, view): (
            bool,
            Option<elohim_views::acquisition::EprPullStatusView>,
        ) = {
            let _ = &pin_heads; // unused without p2p
            (false, None)
        };

        match view {
            Some(v) => Ok(response::ok(&v)),
            // Pinned, acquisition running, no tracker yet: report the honest
            // unmeasured rollup rather than denying the pin exists.
            None if acquisition_wired && pinned_here => Ok(response::ok(
                &elohim_views::acquisition::EprPullStatusView {
                    epr_id: epr_id.to_string(),
                    total: None,
                    fetched: 0,
                    pending: 0,
                    failed: 0,
                    caught_up: None,
                },
            )),
            None => Ok(response::not_found(&format!(
                "no active pull state for EPR '{}'",
                epr_id
            ))),
        }
    }

    /// DELETE /api/v1/pins/{id} — soft-delete a pin by marking it "removed".
    ///
    /// Un-pin IS real revocation (Slice-2b T10): after flipping the pin to
    /// `removed`, if the pin carries a `commitment_cid` back-reference (the
    /// `replicates-commons` offer the provide reconciler authored for it), this
    /// authors a `revokes-commitment` targeting that CID through the conductor.
    /// The revocation's projection (`mishpat_projection::parse_revokes_commitment`)
    /// sets `revoked_at` on the target row, so the prioritizer stops serving it
    /// and the bounds validator refuses it.
    ///
    /// This is the AUTHORITATIVE un-pin revocation path (targets the pin's
    /// `commitment_cid` directly). It is distinct from the reconciler's
    /// stranded-row safety-net arm, which revokes a live commitment whose logical
    /// key has left the desired set. Best-effort: a conductor failure must NOT
    /// fail the un-pin — the next provide-reconcile tick's stranded-row arm is
    /// the backstop. Prior accepted ProvideAnnounce events stand (no retroactive
    /// change — the audit trail keeps which commitment was revoked).
    async fn handle_remove_pin(&self, id_str: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        let id: i32 = id_str
            .parse()
            .map_err(|_| StorageError::InvalidInput(format!("invalid pin id: '{}'", id_str)))?;

        let mut conn = self.get_diesel_conn()?;

        // Capture the notarized offer back-reference BEFORE flipping status so we
        // can withdraw the commons offer (author a revokes-commitment). Indexed
        // lookup by id — not a full-table scan.
        let offer_cid: Option<String> = db::acquisition_pins::get_pin_by_id(&mut conn, id)
            .map_err(|e| StorageError::Database(e.to_string()))?
            .and_then(|p| p.commitment_cid);

        let rows = db::acquisition_pins::set_pin_status(&mut conn, id, "removed")
            .map_err(|e| StorageError::Database(e.to_string()))?;

        if rows == 0 {
            return Ok(response::not_found(&format!("pin {} not found", id)));
        }
        drop(conn);

        // Withdraw the commons offer: author a revokes-commitment targeting the
        // pin's commitment_cid. Best-effort — the provide reconciler's stranded-
        // row revoke arm is the backstop if the conductor is unavailable here.
        if let (Some(target_cid), Some(hc)) = (
            offer_cid,
            self.hc_registry.as_ref().and_then(|r| r.lamad_client()),
        ) {
            let signed_at = crate::db::models::current_timestamp();
            let payload_json = crate::services::conductor_commitment_author::build_revoke_payload(
                &target_cid,
                &signed_at,
            );
            let input = crate::services::conductor_writes::CreateMishpatCommitmentInput {
                action: "revokes-commitment".to_string(),
                payload_json,
                signed_at,
            };
            if let Err(e) =
                crate::services::conductor_writes::call_create_commitment(&hc, input).await
            {
                tracing::warn!(
                    target: "elohim_storage::provide",
                    pin = id,
                    target_cid = %target_cid,
                    error = %e,
                    "un-pin: revokes-commitment author failed; reconciler stranded-row arm will retry"
                );
            }
        }

        Ok(response::ok(&serde_json::json!({ "removed": id })))
    }

    /// Expose test entry points for the pins handlers.
    /// Not guarded by `#[cfg(test)]` — integration test binaries compile the
    /// library without `cfg(test)` set (only the binary itself carries that flag).
    pub async fn test_handle_list_pins(&self) -> HttpTestResponse {
        match self.handle_list_pins().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .unwrap()
                    .to_bytes();
                HttpTestResponse { status, body }
            }
            Err(e) => HttpTestResponse {
                status: 500,
                body: bytes::Bytes::from(format!("error: {e}")),
            },
        }
    }

    pub async fn test_handle_create_pin_raw(&self, json: &str) -> HttpTestResponse {
        // `Incoming` cannot be constructed directly in tests; delegate to the
        // inner bytes helper which bypasses the body-read step.
        match self.handle_create_pin_bytes(json.as_bytes()).await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .unwrap()
                    .to_bytes();
                HttpTestResponse { status, body }
            }
            Err(e) => HttpTestResponse {
                status: 500,
                body: bytes::Bytes::from(format!("error: {e}")),
            },
        }
    }

    /// Inner helper — parse body bytes and run create_pin logic.
    /// Shared by the real handler (which gets bytes from Incoming) and the test helper.
    async fn handle_create_pin_bytes(
        &self,
        body_bytes: &[u8],
    ) -> Result<Response<Full<Bytes>>, StorageError> {
        let input: elohim_views::acquisition::CreatePinInputView =
            match serde_json::from_slice(body_bytes) {
                Ok(v) => v,
                Err(e) => return Ok(response::bad_request(&format!("Invalid JSON: {}", e))),
            };

        // Canonicalize to the bare content id: the EPR link surface carries an
        // `epr:` prefix, but the whole reconcile→dispatch→completion loop keys
        // on the bare `content.id` (seed ids are unprefixed). Strip at the
        // boundary so a pin's head_ref always matches the content projection.
        let head_ref = input
            .head_ref
            .strip_prefix("epr:")
            .unwrap_or(&input.head_ref)
            .to_string();

        let kind = input.kind.unwrap_or_else(|| "item".to_string());

        if kind == "cluster" {
            return Ok(response::json_response(
                StatusCode::NOT_IMPLEMENTED,
                &serde_json::json!({
                    "error": "cluster pins require the slice-3 closure resolver (spec §5) — not yet implemented"
                }),
            ));
        }

        if kind != "item" {
            return Ok(response::json_response(
                StatusCode::BAD_REQUEST,
                &serde_json::json!({
                    "error": format!("unknown pin kind '{}'; valid values: item, cluster", kind)
                }),
            ));
        }

        // A provide pin makes the node a serveable peer provider; that is only
        // meaningful on a peer-capable node. A browser-only context cannot serve
        // bytes to peers, so refuse the provide flag there (spec slice 2b).
        if input.provide.unwrap_or(false) && input.context.as_deref() == Some("browser") {
            return Ok(response::json_response(
                StatusCode::BAD_REQUEST,
                &serde_json::json!({
                    "error": "provide pins require a peer-capable node; a browser-only context cannot serve bytes to peers"
                }),
            ));
        }

        let mut conn = self.get_diesel_conn()?;
        let pin = db::acquisition_pins::upsert_pin(
            &mut conn,
            db::models::NewAcquisitionPin {
                // Slice 1: single-device identity placeholder. TODO(slice-2): use
                // the wired agent identity (self.p2p_handle.agent_pubkey) so pins
                // are keyed to the real agent, not a shared "local-device".
                agent_pub_key: "local-device".to_string(),
                head_ref,
                kind,
                closure_rule_json: input.closure_rule.map(|v| {
                    serde_json::to_string(&v.0).expect("serializing a JSON Value is infallible")
                }),
                priority: input.priority.unwrap_or(1),
            },
        )
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(response::created(&pin_to_view(pin)))
    }

    pub async fn test_handle_remove_pin(&self, id_str: &str) -> HttpTestResponse {
        match self.handle_remove_pin(id_str).await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .unwrap()
                    .to_bytes();
                HttpTestResponse { status, body }
            }
            Err(e) => HttpTestResponse {
                status: 500,
                body: bytes::Bytes::from(format!("error: {e}")),
            },
        }
    }

    /// Test entry point for the EPR pull handler (mirrors test_handle_list_pins;
    /// not #[cfg(test)]-gated — integration binaries compile the lib without it).
    pub async fn test_handle_epr_pull(&self, epr_id: &str) -> HttpTestResponse {
        match self.handle_epr_pull(epr_id).await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body = http_body_util::BodyExt::collect(resp.into_body())
                    .await
                    .unwrap()
                    .to_bytes();
                HttpTestResponse { status, body }
            }
            Err(e) => HttpTestResponse {
                status: 500,
                body: bytes::Bytes::from(format!("error: {e}")),
            },
        }
    }
}

/// A PATCH must re-notarize through the conductor when it changes a
/// DNA-notarized field. `reach` is class-A notarized — a reach change cannot be
/// a diesel-only write or the reconciliation controller (DHT wins) reverts it,
/// so a reach correction never sticks. `blob_hash` is likewise carried through
/// the re-authored Content entry. Metadata-only patches (title/description/
/// tags) stay on the legacy diesel path.
/// Note: `p2p_published_at` is deliberately excluded — it is a provenance stamp,
/// not a DHT entry field, so its diesel write is not subject to reconciliation-controller reversion.
/// `server_blob_hash` is likewise excluded — it is the SSR *server* bundle hash, a deploy-time
/// projection artifact (not part of the notarized content entry, whose content address is
/// `blob_cid`). Routing a serverBlobHash-only PATCH through the conductor would 503 on
/// household/local stacks with no conductor bridge; the diesel-direct write is correct and is
/// not reverted by the reconciliation controller.
fn patch_needs_conductor(view: &UpdateContentInputView) -> bool {
    view.blob_hash.is_some() || view.reach.is_some()
}

/// Map a DB `AcquisitionPin` model to its wire view.
///
/// `closure_rule_json` is parsed from a JSON string to `serde_json::Value`
/// so TypeScript receives a real object, not a JSON-string-inside-JSON.
fn pin_to_view(p: db::models::AcquisitionPin) -> elohim_views::acquisition::PinView {
    // The DB stores the bare content id (handle_create_pin strips `epr:` so the
    // reconcile loop joins against the unprefixed content projection). The wire
    // shape is the EPR link surface, which carries the `epr:` prefix — re-add it
    // here so the read path matches what callers POST and filter on.
    let head_ref = if p.head_ref.starts_with("epr:") {
        p.head_ref
    } else {
        format!("epr:{}", p.head_ref)
    };
    elohim_views::acquisition::PinView {
        id: p.id,
        agent_pub_key: p.agent_pub_key,
        head_ref,
        kind: p.kind,
        closure_rule: p
            .closure_rule_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .map(elohim_views::shared::JsonVal),
        priority: p.priority,
        status: p.status,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

/// The window between accepting a pin and the first reconcile tick.
///
/// `tests/acquisition_pins_http.rs` covers the no-p2p (airplane-mode) side of
/// GET /api/v1/pins/{eprId}/pull, where 404 is the whole contract. These cover
/// the side that needs a wired `AcquisitionState`: with acquisition running, an
/// active pin whose tracker has not been born yet must report an honest
/// unmeasured rollup, and only a genuinely unpinned EPR may 404.
/// @concern: acquisition-pins.feature:40
#[cfg(all(test, feature = "p2p"))]
mod epr_pull_unmeasured_window_tests {
    use super::HttpServer;
    use std::sync::Arc;

    async fn server_with_acquisition() -> HttpServer {
        let blob_store = Arc::new(
            crate::blob_store::BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
            .with_db_pool(crate::test_util::test_pool())
            .with_p2p_handle(crate::p2p::P2PHandle::for_testing())
    }

    #[tokio::test]
    async fn freshly_pinned_epr_reports_an_unmeasured_rollup_not_404() {
        let server = server_with_acquisition().await;
        let create = server
            .test_handle_create_pin_raw(r#"{"headRef":"epr:strawberry-guide","provide":true}"#)
            .await;
        assert_eq!(create.status, 201, "seed pin must be accepted");

        // No reconcile tick has run, so AcquisitionState holds no tracker.
        let resp = server.test_handle_epr_pull("strawberry-guide").await;
        assert_eq!(
            resp.status,
            200,
            "a pin this node accepted must not read back as 'not found'; body: {:?}",
            String::from_utf8_lossy(&resp.body)
        );

        let val: serde_json::Value = serde_json::from_slice(&resp.body).expect("valid JSON body");
        assert_eq!(val["eprId"], "strawberry-guide");
        assert!(
            val["total"].is_null(),
            "unmeasured total is the tri-state null, not 0; got {}",
            val["total"]
        );
        assert!(
            val["caughtUp"].is_null(),
            "unmeasured must never read as caught up; got {}",
            val["caughtUp"]
        );
        for field in ["fetched", "pending", "failed"] {
            assert_eq!(val[field], 0, "{field} must be 0 before any measurement");
        }
    }

    #[tokio::test]
    async fn epr_prefixed_query_finds_the_same_pin() {
        let server = server_with_acquisition().await;
        server
            .test_handle_create_pin_raw(r#"{"headRef":"epr:strawberry-guide"}"#)
            .await;
        let resp = server.test_handle_epr_pull("epr:strawberry-guide").await;
        assert_eq!(resp.status, 200, "the epr: prefix must normalize away");
    }

    #[tokio::test]
    async fn an_unpinned_epr_is_still_404() {
        let server = server_with_acquisition().await;
        let resp = server.test_handle_epr_pull("never-pinned-here").await;
        assert_eq!(
            resp.status,
            404,
            "404 stays the answer for an EPR this node holds no pin for; body: {:?}",
            String::from_utf8_lossy(&resp.body)
        );
    }

    #[tokio::test]
    async fn a_removed_pin_does_not_keep_reporting_a_rollup() {
        let server = server_with_acquisition().await;
        let create = server
            .test_handle_create_pin_raw(r#"{"headRef":"epr:transient"}"#)
            .await;
        let val: serde_json::Value = serde_json::from_slice(&create.body).expect("valid JSON");
        let id = val["id"].as_i64().expect("pin id");

        server.test_handle_remove_pin(&id.to_string()).await;

        let resp = server.test_handle_epr_pull("transient").await;
        assert_eq!(
            resp.status,
            404,
            "an un-pinned EPR is no longer a promise this node keeps; body: {:?}",
            String::from_utf8_lossy(&resp.body)
        );
    }
}

/// Correlate observation entries into actionable issues.
///
/// Groups error/warning entries by failure pattern:
/// - 401 clusters → single auth issue
/// - 404 per content ID → not-found issue per content
/// - 405 per path → method-not-allowed issue per path
/// - 503 mentioning "imagodei" → identity/conductor issue
///
/// Content IDs are extracted from `/db/content/{id}` and
/// `/db/allocations/content/{id}` paths.
fn correlate_issues(entries: &[crate::db::models::ObservationEntry]) -> Vec<ObservationIssueView> {
    use std::collections::HashMap;

    let mut issues: HashMap<String, ObservationIssueView> = HashMap::new();

    for entry in entries {
        if entry.severity != "error" && entry.severity != "warning" {
            continue;
        }

        let path = entry.path.as_deref().unwrap_or("");
        let status = entry.status_code.unwrap_or(0);

        // Extract content ID from well-known path patterns
        let content_id_from_path = if let Some(rest) = path.strip_prefix("/db/content/") {
            let id = rest.split('/').next().unwrap_or("").to_string();
            if id.is_empty() {
                None
            } else {
                Some(id)
            }
        } else if let Some(rest) = path.strip_prefix("/db/allocations/content/") {
            let id = rest.split('/').next().unwrap_or("").to_string();
            if id.is_empty() {
                None
            } else {
                Some(id)
            }
        } else {
            None
        };

        match status {
            401 => {
                let key = "auth-401".to_string();
                let issue = issues
                    .entry(key.clone())
                    .or_insert_with(|| ObservationIssueView {
                        id: key,
                        category: "auth".to_string(),
                        severity: "error".to_string(),
                        title: "Authentication failures detected".to_string(),
                        entry_count: 0,
                        related_content_ids: vec![],
                        suggested_cause:
                            "Agent session may be missing or expired. Check X-Agent-Id header."
                                .to_string(),
                    });
                issue.entry_count += 1;
                if let Some(ref cid) = content_id_from_path {
                    if !issue.related_content_ids.contains(cid) {
                        issue.related_content_ids.push(cid.clone());
                    }
                }
            }
            404 => {
                let key = if let Some(ref cid) = content_id_from_path {
                    format!("not-found-{}", cid)
                } else {
                    format!("not-found-{}", path.replace('/', "-").trim_matches('-'))
                };
                let related = content_id_from_path
                    .clone()
                    .map(|s| vec![s])
                    .unwrap_or_default();
                let path_clone = path.to_string();
                let issue = issues
                    .entry(key.clone())
                    .or_insert_with(|| ObservationIssueView {
                        id: key,
                        category: "content".to_string(),
                        severity: "warning".to_string(),
                        title: format!("Content not found: {}", path_clone),
                        entry_count: 0,
                        related_content_ids: related,
                        suggested_cause:
                            "Content ID may not be seeded or may belong to a different h_app_id."
                                .to_string(),
                    });
                issue.entry_count += 1;
            }
            405 => {
                let key = format!(
                    "method-not-allowed-{}",
                    path.replace('/', "-").trim_matches('-')
                );
                let path_clone = path.to_string();
                let issue = issues
                    .entry(key.clone())
                    .or_insert_with(|| ObservationIssueView {
                        id: key,
                        category: "routing".to_string(),
                        severity: "warning".to_string(),
                        title: format!("Method not allowed on: {}", path_clone),
                        entry_count: 0,
                        related_content_ids: vec![],
                        suggested_cause:
                            "HTTP method is not registered for this route. Check API specification."
                                .to_string(),
                    });
                issue.entry_count += 1;
            }
            503 if entry.message.contains("imagodei") || path.contains("imagodei") => {
                let key = "conductor-imagodei".to_string();
                let issue = issues.entry(key.clone()).or_insert_with(|| ObservationIssueView {
                    id: key,
                    category: "infrastructure".to_string(),
                    severity: "error".to_string(),
                    title: "Imagodei (identity) conductor unreachable".to_string(),
                    entry_count: 0,
                    related_content_ids: vec![],
                    suggested_cause:
                        "The imagodei DNA conductor is not responding. Verify that elohim-storage                          is connected and the hApp is installed."
                            .to_string(),
                });
                issue.entry_count += 1;
            }
            503 => {
                let key = "service-unavailable".to_string();
                let issue =
                    issues.entry(key.clone()).or_insert_with(|| {
                        ObservationIssueView {
                    id: key,
                    category: "infrastructure".to_string(),
                    severity: "error".to_string(),
                    title: "Service unavailable responses detected".to_string(),
                    entry_count: 0,
                    related_content_ids: vec![],
                    suggested_cause:
                        "Backend service or database pool is not initialised. Check startup logs."
                            .to_string(),
                }
                    });
                issue.entry_count += 1;
            }
            _ => {
                // Other error/warning entries: group by severity + category
                let key = format!("{}-{}", entry.severity, entry.category);
                let sev = entry.severity.clone();
                let cat = entry.category.clone();
                let issue = issues
                    .entry(key.clone())
                    .or_insert_with(|| ObservationIssueView {
                        id: key,
                        category: cat,
                        severity: sev,
                        title: format!(
                            "Repeated {} in category: {}",
                            entry.severity, entry.category
                        ),
                        entry_count: 0,
                        related_content_ids: vec![],
                        suggested_cause: "Review log entries for details.".to_string(),
                    });
                issue.entry_count += 1;
                if let Some(ref cid) = content_id_from_path {
                    if !issue.related_content_ids.contains(cid) {
                        issue.related_content_ids.push(cid.clone());
                    }
                }
            }
        }
    }

    let mut result: Vec<ObservationIssueView> = issues.into_values().collect();
    // Sort: errors first, then warnings; within each severity sort by id
    result.sort_by(|a, b| {
        let sev_order = |s: &str| match s {
            "error" => 0u8,
            "warning" => 1,
            _ => 2,
        };
        sev_order(&a.severity)
            .cmp(&sev_order(&b.severity))
            .then(a.id.cmp(&b.id))
    });
    result
}

/// Build the static route manifest for doorway discovery.
///
/// Declares every `/api/v1/*` and `/db/*` route that elohim-storage serves.
/// Used by `GET /manifest` and consumed by doorway's dynamic route registry.
///
/// Design rule: list only routes that doorway should proxy to clients.
/// Infrastructure routes (/health, /shard/*, /blob/*, /import/*,
/// /p2p/*, /epr-head/*, /ipfs/*, /dag/*, /session, /account/*) are
/// intentionally omitted — doorway handles them independently or not at all.
///
/// EXCEPTION (2026-06-27): the `/sync/v1/*` Automerge doc-sync routes ARE
/// declared here so doorway's route registry proxies browser doc-sync to
/// storage (StorageProxy entries → forward_to_storage). This DELIBERATELY
/// reverses the prior "/sync intentionally omitted" rule — paired with the
/// flipped guard in the `build_manifest_*` test and `is_service_path("/sync")`
/// in doorway. Security-adjacent: this exposes the CRDT doc-sync plane through
/// doorway. The sync API does not itself enforce reach (it shares the known
/// http-reach-enforcement-gap); GET reads mirror /db content-read posture.
pub fn build_manifest() -> doorway_client::DoorwayRoutes {
    DoorwayRoutesBuilder::new()
        // =====================================================================
        // /api/v1/signal — EPR signal emission (browser → conductor → ingest)
        // =====================================================================
        .route(
            Route::post("/api/v1/signal/emit")
                .handler("signal_emit")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/status — Operator status views (write-through, projector)
        // =====================================================================
        .route(
            Route::get("/api/v1/status/write-through")
                .handler("write_through_status")
                .cache_ttl(5)
                .build(),
        )
        // Conductor authority-arc Auto policy gauge (operational, Cat-C; spec §6
        // of 2026-06-13-conductor-authority-arc-auto-policy.md). Node-local —
        // doorway proxies a single node's arc status (single-target, as always).
        .route(
            Route::get("/api/v1/status/arc-policy")
                .handler("arc_policy_status")
                .cache_ttl(5)
                .build(),
        )
        // Projector cursor + reconciliation-lag (operational, Cat-C). The path
        // has an explicit handler arm above (the /api/v1/status/projector match)
        // but must ALSO be declared here so a doorway RouteRegistry learns it is
        // proxiable — single-target, as always. Consumed by /admin/self-healing.
        .route(
            Route::get("/api/v1/status/projector")
                .handler("projector_status")
                .cache_ttl(5)
                .build(),
        )
        // Node-local P2P sync status (operational, Cat-C). The learner-facing
        // sync-progress strip (lamad SyncStatusService) GETs this THROUGH the
        // doorway; declaring it here lets the RouteRegistry proxy it to the
        // serving node's storage (single-target, as always) so the strip shows
        // the real sync state. Without this declaration the doorway had no public
        // /p2p/status route, so the request fell through to the SPA shell (HTML),
        // the client's JSON parse failed, and the strip false-flagged
        // "unreachable" on every load in a healthy mesh. Explicit dispatch arm
        // exists above (GET /p2p/status -> handle_p2p_status). Legitimately
        // doorway-resident node-local operational state (like arc-policy /
        // projector) — NOT another agent's private state.
        .route(
            Route::get("/p2p/status")
                .handler("p2p_status")
                .cache_ttl(2)
                .build(),
        )
        // =====================================================================
        // /sync/v1 — Automerge CRDT doc-sync (browser doc-sync carriage)
        // =====================================================================
        // FLIPPED 2026-06-27: previously omitted as infrastructure. Declared so
        // doorway's route registry proxies these to storage's handle_sync_request
        // (http.rs ~:3380). Handlers already exist; these are proxy declarations.
        // GET reads are NOT cached (live CRDT state must not go stale). POST
        // /changes is a write → auth_required, mirroring /db POST posture.
        // Doc-id path segments are URL-encoded single segments (e.g. node%3Aid).
        .route(
            Route::get("/sync/v1/{hAppId}/docs")
                .handler("sync_list_documents")
                .build(),
        )
        .route(
            Route::get("/sync/v1/{hAppId}/docs/{docId}")
                .handler("sync_get_document")
                .build(),
        )
        .route(
            Route::get("/sync/v1/{hAppId}/docs/{docId}/heads")
                .handler("sync_get_heads")
                .build(),
        )
        .route(
            Route::get("/sync/v1/{hAppId}/docs/{docId}/changes")
                .handler("sync_get_changes")
                .build(),
        )
        .route(
            Route::post("/sync/v1/{hAppId}/docs/{docId}/changes")
                .handler("sync_apply_changes")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/mastery — Content mastery lifecycle
        // =====================================================================
        .route(
            Route::get("/api/v1/mastery")
                .handler("list_mastery")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery")
                .handler("initialize_mastery")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery/engagement")
                .handler("record_engagement")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery/assessment")
                .handler("record_assessment")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery/batch")
                .handler("batch_query_mastery")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/mastery/check-privilege")
                .handler("check_privilege")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/mastery/stats")
                .handler("mastery_stats")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/mastery/pool/{id}")
                .handler("get_practice_pool")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/mastery/path/{path_id}")
                .handler("mastery_for_path")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/mastery/{id}")
                .handler("get_mastery")
                .cache_ttl(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/governance — Governance state and deliberation
        // =====================================================================
        .route(
            Route::get("/api/v1/governance/state")
                .handler("governance_state")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/states")
                .handler("list_governance_states")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/challenges")
                .handler("list_challenges")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/challenges/{id}")
                .handler("get_challenge")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/challenges")
                .handler("file_challenge")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/challenges/{id}/respond")
                .handler("respond_to_challenge")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/challenges/{id}/appeal")
                .handler("file_appeal")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/appeals/{challengeId}")
                .handler("list_appeals")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals")
                .handler("list_proposals")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}")
                .handler("get_proposal")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/precedents")
                .handler("list_precedents")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/precedents/{id}")
                .handler("get_precedent")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/discussions")
                .handler("list_discussions")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/discussions/{id}")
                .handler("get_discussion")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/proposals")
                .handler("create_proposal")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/proposals/{id}/votes")
                .handler("cast_vote")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}/votes")
                .handler("list_votes")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/discussions")
                .handler("create_discussion")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/discussions/{id}/messages")
                .handler("post_message")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/proposals/{id}/options")
                .handler("create_proposal_options")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}/options")
                .handler("list_proposal_options")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/proposals/{id}/ranked-votes")
                .handler("cast_ranked_votes")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}/ranked-votes")
                .handler("list_ranked_votes")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}/tally")
                .handler("compute_tally")
                .cache_ttl(10)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/signals")
                .handler("record_signal")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/signals/aggregate")
                .handler("aggregate_signals")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/signals")
                .handler("list_signals")
                .cache_ttl(30)
                .build(),
        )
        // =====================================================================
        // /api/v1/governance/sensemaking — Sensemaking statements & clustering
        // =====================================================================
        .route(
            Route::post("/api/v1/governance/sensemaking/statements")
                .handler("create_statement")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/sensemaking/statements")
                .handler("list_statements")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/sensemaking/statements/{id}/vote")
                .handler("vote_on_statement")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/sensemaking/votes")
                .handler("list_statement_votes")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/sensemaking/clusters")
                .handler("compute_clusters")
                .cache_ttl(10)
                .build(),
        )
        // =====================================================================
        // /api/v1/economic-events — REA economic events
        // =====================================================================
        .route(
            Route::get("/api/v1/economic-events")
                .handler("list_economic_events")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/economic-events")
                .handler("create_economic_event")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/economic-events/bulk")
                .handler("bulk_create_economic_events")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/economic-events/from-staged")
                .handler("economic_events_from_staged")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/economic-events/agent/{agent_id}")
                .handler("economic_events_by_agent")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/economic-events/content/{content_id}")
                .handler("economic_events_by_content")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/economic-events/appreciations")
                .handler("list_appreciations")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/economic-events/appreciations")
                .handler("create_appreciation")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/economic-events/{id}")
                .handler("get_economic_event")
                .cache_ttl(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/lamad — Intent-driven EconomicEvent composition (M-REA-1)
        // Client sends high-level LamadEventIntent; substrate composes full REA
        // shape via PROTOCOL_EVENT_MAPPINGS. Eliminates F-REA-1 client-side REA
        // composition. Source of truth: EconomicEvent DHT entry (Category A).
        // =====================================================================
        .route(
            Route::post("/api/v1/lamad/events")
                .handler("emit_lamad_event")
                .auth_required()
                .build(),
        )
        // GET /api/v1/lamad/content/{contentId}/engagement — M-AGGR-3
        // Returns ContentEngagementStatsView (Category C projection of EconomicEvent
        // stream filtered by content_id + lamadEventType IN content-view|content-complete).
        // Retires F-AGGR-3: countContentViews/countContentCompletions client-side counting.
        .route(
            Route::get("/api/v1/lamad/content/{contentId}/engagement")
                .handler("get_content_engagement_stats")
                .cache_ttl(30)
                .build(),
        )
        // =====================================================================
        // /api/v1/stewardship — Stewardship allocations and policy
        // =====================================================================
        .route(
            Route::get("/api/v1/stewardship/policy")
                .handler("get_stewardship_policy")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/grants")
                .handler("list_stewardship_grants")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/grants")
                .handler("create_stewardship_grant")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/grants/{id}")
                .handler("get_stewardship_grant")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/grants/{id}/delegate")
                .handler("delegate_stewardship_grant")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/appeals")
                .handler("file_stewardship_appeal")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/activity")
                .handler("log_stewardship_activity")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/allocations")
                .handler("list_allocations")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/allocations")
                .handler("create_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/allocations/content/{content_id}")
                .handler("allocations_for_content")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/allocations/steward/{steward_id}")
                .handler("allocations_for_steward")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/allocations/{id}")
                .handler("get_allocation")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::put("/api/v1/stewardship/allocations/{id}")
                .handler("update_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/api/v1/stewardship/allocations/{id}")
                .handler("delete_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/allocations/{id}/dispute")
                .handler("dispute_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/allocations/{id}/resolve")
                .handler("resolve_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/stewardship/policies")
                .handler("upsert_stewardship_policy")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies")
                .handler("list_stewardship_policies")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies/me/chain")
                .handler("my_policy_chain")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies/{id}")
                .handler("get_stewardship_policy_by_id")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies/{id}/parent")
                .handler("policy_parent")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/policies/{id}/chain")
                .handler("policy_chain")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/stewardship/access/time")
                .handler("check_time_access")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/resilience — Content resilience projection
        // =====================================================================
        .route(
            Route::get("/api/v1/resilience/{content_id}/household")
                .handler("get_household_resilience")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::get("/api/v1/resilience/{content_id}")
                .handler("get_resilience")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/resilience/{content_id}/verify")
                .handler("verify_resilience")
                .build(),
        )
        // =====================================================================
        // /api/v1/recognition — Recognition distribution pipeline
        // =====================================================================
        .route(
            Route::post("/api/v1/recognition/distribute")
                .handler("distribute_recognition")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/schedules — Kairos temporal schedules
        // =====================================================================
        .route(
            Route::get("/api/v1/schedules")
                .handler("list_schedules")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/schedules")
                .handler("create_schedule")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/schedules/{id}")
                .handler("get_schedule")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::patch("/api/v1/schedules/{id}")
                .handler("update_schedule")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/schedules/{id}/advance")
                .handler("advance_schedule")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/places — Governed spatial entities (DHT projection)
        // =====================================================================
        .route(
            Route::get("/api/v1/places")
                .handler("list_places")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/places")
                .handler("create_place")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/places/{id}")
                .handler("get_place")
                .cache_ttl(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/spatial-contexts — Geospatial context (H3 indexed)
        // =====================================================================
        .route(
            Route::get("/api/v1/spatial-contexts")
                .handler("list_spatial_contexts")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/spatial-contexts")
                .handler("create_spatial_context")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/spatial-contexts/{id}")
                .handler("get_spatial_context")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::patch("/api/v1/spatial-contexts/{id}")
                .handler("update_spatial_context")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/api/v1/spatial-contexts/{id}")
                .handler("delete_spatial_context")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/steward-affinity — Steward affinity lifecycle
        // =====================================================================
        .route(
            Route::get("/api/v1/steward-affinity")
                .handler("list_steward_affinity")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/steward-affinity")
                .handler("create_steward_affinity")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/steward-affinity/bulk")
                .handler("bulk_create_steward_affinity")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/steward-affinity/curation-event")
                .handler("steward_affinity_curation_event")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward-affinity/{id}")
                .handler("get_steward_affinity")
                .cache_ttl(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/resources — REA resource management
        // =====================================================================
        .route(
            Route::get("/api/v1/resources")
                .handler("list_resources")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/resources")
                .handler("create_resource")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/resources/dashboard/{id}")
                .handler("resource_dashboard")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/resources/constitutional-limits/{id}")
                .handler("constitutional_limits")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/resources/{id}/allocations")
                .handler("allocate_resource")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/resources/{id}/usage")
                .handler("record_resource_usage")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/resources/{id}")
                .handler("get_resource")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/exchange — Requests and offers
        // =====================================================================
        .route(
            Route::get("/api/v1/exchange/requests")
                .handler("list_requests")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/exchange/requests")
                .handler("create_request")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/exchange/requests/{id}")
                .handler("get_request")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::patch("/api/v1/exchange/requests/{id}")
                .handler("update_request")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/api/v1/exchange/requests/{id}")
                .handler("archive_request")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/exchange/requests/{id}/match")
                .handler("match_request")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/exchange/offers")
                .handler("list_offers")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/exchange/offers")
                .handler("create_offer")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/exchange/offers/{id}")
                .handler("get_offer")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::patch("/api/v1/exchange/offers/{id}")
                .handler("update_offer")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/api/v1/exchange/offers/{id}")
                .handler("archive_offer")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/exchange/offers/{id}/match")
                .handler("match_offer")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/custodians — Infrastructure custodian metrics
        // =====================================================================
        .route(
            Route::get("/api/v1/custodians/metrics")
                .handler("list_custodian_metrics")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/custodians/metrics")
                .handler("report_custodian_metrics")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/custodians/metrics/alerts")
                .handler("custodian_alerts")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/custodians/metrics/{id}")
                .handler("get_custodian_metrics")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/custodians/metrics/{id}/recommendations")
                .handler("custodian_recommendations")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/custodians/protection/{id}")
                .handler("custodian_protection")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/compute — Compute dashboard
        // =====================================================================
        .route(
            Route::get("/api/v1/compute/dashboard")
                .handler("compute_dashboard")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/compute/dashboard/refresh")
                .handler("refresh_compute_dashboard")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/operator — commitment-gated operator runtime verbs
        // (habit operator-runtime-surface). Fail-closed: the peer itself
        // authorizes each verb against an active delegates-compute grant.
        // =====================================================================
        .route(
            Route::post("/api/v1/operator/reconcile")
                .handler("operator_reconcile")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/flow-planning — Resource flow planning
        // =====================================================================
        .route(
            Route::get("/api/v1/flow-planning")
                .handler("list_flow_plans")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/flow-planning")
                .handler("create_flow_plan")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/attestations — legacy content-attestation routes REMOVED.
        // The content-attestations table was dropped (Phase-2a consolidation);
        // prerequisite requirements are now PREREQUISITE content-graph edges and
        // the Category-A DHT projection is served by the /unified routes below.
        // =====================================================================
        // /api/v1/attestations/unified — Unified attestation projection (Category A, DHT)
        // =====================================================================
        .route(
            Route::get("/api/v1/attestations/unified")
                .handler("list_unified_attestations")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/attestations/unified/{id}")
                .handler("get_unified_attestation")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/attestations/unified/{id}/revoke")
                .handler("revoke_unified_attestation")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/governance-actions — Governance action projection + tally (Category A/C)
        // =====================================================================
        .route(
            Route::get("/api/v1/governance-actions")
                .handler("list_governance_actions")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance-actions/{id}")
                .handler("get_governance_action")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::get("/api/v1/governance-actions/{id}/tally")
                .handler("get_governance_action_tally")
                .cache_ttl(10)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance-actions/{id}/vote")
                .handler("cast_governance_vote")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/steward — Steward credentials, gates, grants, access
        // =====================================================================
        .route(
            Route::post("/api/v1/steward/credentials")
                .handler("create_steward_credential")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/credentials")
                .handler("list_steward_credentials")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/credentials/{id}")
                .handler("get_steward_credential")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/steward/gates")
                .handler("create_steward_gate")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/gates")
                .handler("list_steward_gates")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/gates/{id}")
                .handler("get_steward_gate")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/steward/grants")
                .handler("create_steward_grant")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/grants")
                .handler("list_steward_grants")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/grants/{id}")
                .handler("get_steward_grant")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/access")
                .handler("steward_access")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/steward/revenue/{id}")
                .handler("steward_revenue")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/contributors — Contributor dashboards
        // =====================================================================
        .route(
            Route::get("/api/v1/contributors/me/dashboard")
                .handler("my_contributor_dashboard")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/contributors/{id}/dashboard")
                .handler("contributor_dashboard")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/contributors/{id}/impact")
                .handler("contributor_impact")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/api/v1/contributors/{id}/recognition")
                .handler("contributor_recognition")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/presence — Contributor presences
        // =====================================================================
        .route(
            Route::get("/api/v1/presence")
                .handler("list_presences")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/api/v1/presence")
                .handler("create_presence")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/presence/{id}")
                .handler("get_presence")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::delete("/api/v1/presence/{id}")
                .handler("delete_presence")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/presence/{id}/stewardship")
                .handler("begin_stewardship")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/presence/{id}/claim")
                .handler("initiate_claim")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/presence/{id}/verify-claim")
                .handler("verify_claim")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/identity — Human identity registration
        // =====================================================================
        .route(
            Route::post("/api/v1/identity/register")
                .handler("register_human")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/identity/me")
                .handler("get_me")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /auth/me — Peer OAuth portal trust-indicator endpoint (Task A4)
        //
        // Returns the same MeResponse shape as doorway's /auth/me so the
        // standalone portal bundle can consume either source without knowing
        // which transport is in play (spec §3.2 Transport β).
        // cache_ttl(0): session state — never cached.
        // auth not flagged here: handler returns 401 itself when no session.
        // =====================================================================
        .route(
            Route::get("/auth/me")
                .handler("auth_me")
                .cache_ttl(0)
                .build(),
        )
        .route(
            Route::put("/api/v1/identity/me")
                .handler("update_me")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/identity/{id}/profile — viewer-relative disclosure lens.
        //
        // The per-viewer profile lens (spec 2026-06-22 imagodei-profile-page-
        // viewer-lens-design §3/§5). Viewer = authenticated session (resolved
        // server-side from X-Agent-Id), NEVER a ?viewer= param. Deliberately NOT
        // auth_required: an anonymous caller must reach it and resolve to the
        // commons tier (the public/commons face). Dispatch lives in
        // api/identity.rs (`/{subjectId}/profile` strip-suffix arm). This entry
        // is the route-shadow guard (storage has no is_service_path, so the
        // manifest path-coverage test IS the shadow guard).
        // cache_ttl(0): viewer-relative — never shared-cached across viewers.
        // =====================================================================
        .route(
            Route::get("/api/v1/identity/{id}/profile")
                .handler("get_profile_lens")
                .cache_ttl(0)
                .build(),
        )
        // =====================================================================
        // /api/v1/agreements — REA agreements
        // =====================================================================
        .route(
            Route::get("/api/v1/agreements")
                .handler("list_agreements")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/api/v1/agreements")
                .handler("create_agreement")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/agreements/{id}")
                .handler("get_agreement")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/commitments — REA commitments
        // =====================================================================
        .route(
            // TTL 30s (not 300): this route is the observation surface for the
            // runtime commitment producers (the provide loop authors on a 60s
            // tick, and the mishpat→REA mirror lands on a post-commit signal).
            // A 5-minute projection cache made a freshly-notarized commitment
            // invisible for up to 5 ticks, so any acceptance check that polls
            // for a new commitment inside a ~60s window read a stale empty list
            // and reported "producer dead" when the producer had in fact
            // authored. Matched to the /api/v1/resilience TTL so the commitment
            // ledger and the card computed FROM it cannot disagree by cache age.
            Route::get("/api/v1/commitments")
                .handler("list_commitments")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/commitments")
                .handler("create_commitment")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/commitments/capacity")
                .handler("create_capacity_pledge")
                .auth_required()
                .build(),
        )
        // GET /api/v1/commitments/capacity/ratios — consent-legibility read:
        // exposes constitutional_ratio_registry::effective_ratios() so a
        // caller can construct a ratioAttestation that will exact-match the
        // donut check in replicates_commons_validator (the capacity POST
        // above rejects any attestation that doesn't). Category C
        // operational config, no new DHT entity — no auth required (it's
        // what you must SEE before you can consent).
        .route(
            Route::get("/api/v1/commitments/capacity/ratios")
                .handler("get_effective_ratios")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/api/v1/commitments/agent/{agent_id}")
                .handler("commitments_by_agent")
                .cache_ttl(300)
                .build(),
        )
        // =====================================================================
        // /api/v1/commitments/facing/rea — REA economic facing per-commitment
        // read surface over the mishpat compute/recovery-class ledger (Wave 4.2).
        // Viewer-less + node-scoped (no auth). Dispatch lives in
        // api/rea_commitments.rs (`(GET, "facing/rea")` arm); this registry entry
        // carries its cache metadata and is the route-shadow guard (asserted by
        // `test_manifest_builds` — storage has no is_service_path, so the manifest
        // path-coverage test IS the shadow guard).
        // =====================================================================
        .route(
            Route::get("/api/v1/commitments/facing/rea")
                .handler("commitments_facing_rea")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::get("/api/v1/commitments/{id}")
                .handler("get_commitment")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::patch("/api/v1/commitments/{id}")
                .handler("update_commitment_state")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/pins — the acquisition-pin consent surface. The handlers have
        // been live on the node-local listener since the demand-autopin work
        // (dispatch arms at the top of handle_request), but were never declared
        // here, so no doorway could discover them: the resilience card's own
        // call-to-action ("invite a household to help hold these") had no
        // reachable lever from outside the cluster. POST is the explicit
        // stewardship-acceptance act (a pin with provide intent feeds the
        // provide loop's desired set); it carries auth_required like the sibling
        // POST /api/v1/commitments.
        // =====================================================================
        .route(
            Route::get("/api/v1/pins")
                .handler("list_pins")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/pins")
                .handler("create_pin")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/pins/{epr_id}/pull")
                .handler("epr_pull_status")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::delete("/api/v1/pins/{id}")
                .handler("remove_pin")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /db/ — Structured local database (content, paths, relationships, etc.)
        // =====================================================================
        .route(
            Route::get("/db/stats")
                .handler("db_stats")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/db/schema")
                .handler("db_schema")
                .cache_ttl(3600)
                .build(),
        )
        // Content
        .route(
            Route::get("/db/content")
                .handler("list_content")
                .cache_ttl(300)
                .public_if_reach("commons")
                .build(),
        )
        .route(
            Route::post("/db/content")
                .handler("create_content")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/content/bulk")
                .handler("bulk_create_content")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/content/{id}")
                .handler("get_content")
                .cache_ttl(300)
                .public_if_reach("commons")
                .build(),
        )
        .route(
            // Partial-update route. Handler dispatch lives in
            // handle_db_content_by_id (http.rs ~line 3810, Method::PATCH branch);
            // this entry is the auth/metadata registration. Without it, PATCH
            // requests fall through to method_not_allowed (405) — the silent
            // failure mode that broke deploy-time stageSpaBlob. See
            // genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md.
            Route::patch("/db/content/{id}")
                .handler("update_content")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/db/content/{id}")
                .handler("delete_content")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/content/{id}/schedule")
                .handler("get_content_schedule")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/content/{id}/schedule")
                .handler("create_content_schedule")
                .auth_required()
                .build(),
        )
        // Notary-HEAD authority surface (HEAD-election, Plan C3 / Leg 3).
        // cache_ttl(0): a HEAD answer is authority data that can move on any POST;
        // caching would risk serving a superseded HEAD (same precedent as the
        // /auth/me and identity-lens authority reads). Dispatch lives in
        // handle_content_head; auth on POST is enforced there (author-only), the
        // .auth_required() below is the metadata/route-shadow registration.
        .route(
            Route::get("/db/content/{id}/head")
                .handler("get_content_head")
                .cache_ttl(0)
                .build(),
        )
        .route(
            Route::post("/db/content/{id}/head")
                .handler("declare_content_head")
                .auth_required()
                .build(),
        )
        // CROSS-ROOT canonical-head declaration (notary-authority convergence,
        // Model B / Tier-1 STAGING tier). No author gate zome-side (god-mode
        // scaffold; earned-head guard protects an EARNED canonical) — edge
        // auth (.auth_required() below) is this route's sole protection.
        // Dispatch lives in handle_content_canonical_head.
        .route(
            Route::post("/db/content/{id}/canonical-head")
                .handler("declare_canonical_content_head")
                .auth_required()
                .build(),
        )
        // Declare-carries-Record SOURCE half: serve this peer's local DHT
        // Record for the id's head action so a peer that cannot retrieve it
        // (full-arc gossip gap) can be handed verifiable bytes instead of
        // waiting on gossip that will never arrive. cache_ttl(0) for the same
        // reason as /head — the head can move on any declare, and a cached
        // record would hand out a superseded target. Dispatch lives in
        // handle_content_head_record.
        .route(
            Route::get("/db/content/{id}/head-record")
                .handler("get_content_head_record")
                .cache_ttl(0)
                .auth_required()
                .build(),
        )
        // Conductor DHT diagnostics (Cat-C operational; dht-unity T3). Live
        // network state — never cache. Read-only, no auth: agent infos are
        // already public via the shared bootstrap table, and connection stats
        // are the node's own operational state (same class as /health).
        .route(
            Route::get("/db/p2p/conductor-diagnostics")
                .handler("conductor_diagnostics")
                .cache_ttl(0)
                .build(),
        )
        // did:elohim resolution (Cat-C operational projection; DID bridge spec
        // §3.4). Assembled per request from substrate joins — never cache.
        // Public read (the document is a public projection of substrate truth,
        // same class as conductor-diagnostics); doorway auto-forwards it.
        .route(
            Route::get("/db/identity/did/{did}")
                .handler("resolve_did")
                .cache_ttl(0)
                .build(),
        )
        // Knowledge graph relationships
        .route(
            Route::get("/db/relationships")
                .handler("list_relationships")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/db/relationships")
                .handler("create_relationship")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/relationships/bulk")
                .handler("bulk_create_relationships")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/relationships/graph/{content_id}")
                .handler("content_graph")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/db/relationships/{id}")
                .handler("get_relationship")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::delete("/db/relationships/{id}")
                .handler("delete_relationship")
                .auth_required()
                .build(),
        )
        // Knowledge maps
        .route(
            Route::get("/db/knowledge-maps")
                .handler("list_knowledge_maps")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/db/knowledge-maps")
                .handler("create_knowledge_map")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/knowledge-maps/{id}")
                .handler("get_knowledge_map")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::put("/db/knowledge-maps/{id}")
                .handler("update_knowledge_map")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/db/knowledge-maps/{id}")
                .handler("delete_knowledge_map")
                .auth_required()
                .build(),
        )
        // Human relationships
        .route(
            Route::get("/db/human-relationships")
                .handler("list_human_relationships")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/human-relationships")
                .handler("create_human_relationship")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/human-relationships/{id}")
                .handler("get_human_relationship")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::delete("/db/human-relationships/{id}")
                .handler("delete_human_relationship")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/human-relationships/{id}/consent")
                .handler("update_relationship_consent")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/human-relationships/{id}/custody")
                .handler("update_relationship_custody")
                .auth_required()
                .build(),
        )
        // Human directory (qahal)
        .route(
            Route::get("/db/humans")
                .handler("list_humans")
                .cache_ttl(300)
                .build(),
        )
        // Collectives (qahal)
        .route(
            Route::get("/db/collectives")
                .handler("list_collectives")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::post("/db/collectives")
                .handler("create_collective")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/collectives/{id}")
                .handler("get_collective")
                .cache_ttl(300)
                .build(),
        )
        .route(
            Route::get("/db/collectives/{id}/participants")
                .handler("list_collective_participants")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/collectives/{id}/participants")
                .handler("join_collective")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/db/collectives/{id}/participants/{human_id}")
                .handler("depart_collective")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/participations/{human_id}")
                .handler("participations_by_human")
                .cache_ttl(60)
                .build(),
        )
        // Contributor presences
        .route(
            Route::get("/db/presences")
                .handler("list_db_presences")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/presences")
                .handler("create_db_presence")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/presences/bulk")
                .handler("bulk_create_presences")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/presences/{id}")
                .handler("get_db_presence")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::delete("/db/presences/{id}")
                .handler("delete_db_presence")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/presences/{id}/stewardship")
                .handler("db_begin_stewardship")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/presences/{id}/claim")
                .handler("db_initiate_claim")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/presences/{id}/verify-claim")
                .handler("db_verify_claim")
                .auth_required()
                .build(),
        )
        // Economic events
        .route(
            Route::get("/db/events")
                .handler("list_db_events")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/events")
                .handler("create_db_event")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/events/bulk")
                .handler("bulk_create_events")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/events/{id}")
                .handler("get_db_event")
                .cache_ttl(60)
                .build(),
        )
        // Content mastery
        .route(
            Route::get("/db/mastery")
                .handler("list_db_mastery")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/mastery")
                .handler("create_db_mastery")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/mastery/bulk")
                .handler("bulk_create_mastery")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/mastery/human/{human_id}")
                .handler("mastery_for_human")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/db/mastery/{id}")
                .handler("get_db_mastery")
                .cache_ttl(60)
                .build(),
        )
        // Stewardship allocations
        .route(
            Route::get("/db/allocations")
                .handler("list_db_allocations")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::post("/db/allocations")
                .handler("create_db_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/allocations/bulk")
                .handler("bulk_create_allocations")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/allocations/content/{content_id}")
                .handler("db_allocations_for_content")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/db/allocations/steward/{steward_id}")
                .handler("db_allocations_for_steward")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::get("/db/allocations/{id}")
                .handler("get_db_allocation")
                .cache_ttl(60)
                .build(),
        )
        .route(
            Route::delete("/db/allocations/{id}")
                .handler("delete_db_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/allocations/{id}/dispute")
                .handler("dispute_db_allocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/db/allocations/{id}/resolve")
                .handler("resolve_db_allocation")
                .auth_required()
                .build(),
        )
        // Delivery peers — discovered peers with delivery capabilities
        // Used by frontend Service Worker for multi-peer app delivery scoring
        .route(
            Route::get("/api/v1/peers/delivery")
                .handler("delivery_peers")
                .cache_ttl(10)
                .build(),
        )
        // Cache stream for projection warm-up (SSE)
        .route(
            Route::get("/api/v1/cache/stream")
                .handler("cache_stream")
                .build(),
        )
        // =====================================================================
        // /api/v1/observations -- Observation Session API
        // =====================================================================
        .route(
            Route::post("/api/v1/observations/begin")
                .handler("observation_begin")
                .build(),
        )
        .route(
            Route::post("/api/v1/observations/{id}/entries")
                .handler("observation_add_entries")
                .build(),
        )
        .route(
            Route::get("/api/v1/observations/{id}/report")
                .handler("observation_report")
                .build(),
        )
        // =====================================================================
        // /account -- Account Import/Export (seeding & recovery)
        // =====================================================================
        // Serves genesis seeding AND account recovery (restoring a human's world
        // from backup). Dispatch is hand-coded at the top of handle() — these
        // manifest entries exist so doorway discovers and proxies them.
        .route(
            Route::post("/account/import")
                .handler("account_import")
                .build(),
        )
        .route(
            Route::get("/account/export/{human_id}")
                .handler("account_export")
                .build(),
        )
        // =====================================================================
        // /api/v1/peer-statuses — PeerStatus DHT projection (infrastructure DNA)
        // =====================================================================
        // Read-only. Source of truth: infrastructure DNA DHT (Category A, notarized).
        // Rows are written exclusively by `InfrastructureSignal::PeerStatusRecorded`.
        // Optional ?householdId= filter falls back to list-all until Task C3 migration.
        .route(
            Route::get("/api/v1/peer-statuses")
                .handler("list_peer_statuses")
                .cache_ttl(30)
                .build(),
        )
        // =====================================================================
        // /api/v1/nodes/shape — elohim-node durable shape publish
        // =====================================================================
        // Write. Source of truth: node-registry DNA NodeRegistration entry
        // (existing Category A entry type, authored by node's own agent key).
        // This route upserts the stewarded_nodes SQLite projection; the DHT
        // commit via register_node_shape coordinator fn happens from the node's
        // own conductor connection (Task C7) and fills in dht_anchor_hash via
        // post-commit signal projection. No new DHT entry types introduced.
        .route(
            Route::post("/api/v1/nodes/shape")
                .handler("post_node_shape")
                .build(),
        )
        // =====================================================================
        // /api/v1/households/{id}/devices — household devices with peer vitals
        // =====================================================================
        // Read-only. Computed Category C projection (no persistence). Joins
        // stewarded_nodes × peer_statuses for the specified household id.
        // No DHT writes, no new entry types — operational visibility only.
        .route(
            Route::get("/api/v1/households/{id}/devices")
                .handler("get_household_devices")
                .cache_ttl(30)
                .build(),
        )
        // =====================================================================
        // /api/v1/households/{id}/stewardship-allocations — stewarded content
        // =====================================================================
        // Read-only. Joins stewardship_allocations × humans on household_id
        // to list content a household actively stewards. Source of truth:
        // Agreement DHT entries with stewardship-allocation link metadata
        // (Category A2). No new DHT entry types.
        .route(
            Route::get("/api/v1/households/{id}/stewardship-allocations")
                .handler("get_household_stewardship_allocations")
                .cache_ttl(15)
                .build(),
        )
        // =====================================================================
        // /api/v1/network/posture — aggregate health of the live peer fabric
        // =====================================================================
        // Read-only. Computed Category C projection (no persistence, no DHT
        // writes). Aggregates peer_statuses (notarized PeerStatus entries,
        // infrastructure DNA) plus stewarded_nodes reciprocation. Sister to
        // the existing /p2p/status surface but household-aware.
        .route(
            Route::get("/api/v1/network/posture")
                .handler("get_network_posture")
                .cache_ttl(15)
                .build(),
        )
        // =====================================================================
        // /db/gate-decisions — Gate decision attestation projections (mishpat)
        // =====================================================================
        // Read-only. Source of truth: mishpat DNA DHT. These rows are written
        // exclusively by `MishpatSignal::GateDecisionCreated` signal projection.
        // Auth required because gate decisions carry constitutional reasoning
        // that should not be visible to anonymous visitors.
        .route(
            Route::get("/db/gate-decisions")
                .handler("list_gate_decisions")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/gate-decisions/{cid}")
                .handler("get_gate_decision")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /db/gate-decision-challenges — Challenge projections (mishpat)
        // =====================================================================
        // Read-only. Source of truth: mishpat DNA DHT. These rows are written
        // exclusively by `MishpatSignal::GateDecisionChallengeCreated` signal projection.
        // Auth required because challenges carry constitutional grievance details.
        .route(
            Route::get("/db/gate-decision-challenges")
                .handler("list_gate_decision_challenges")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/gate-decision-challenges/{cid}")
                .handler("get_gate_decision_challenge")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /db/challenge-outcomes — Outcome projections (mishpat)
        // =====================================================================
        // Read-only. Source of truth: mishpat DNA DHT. These rows are written
        // exclusively by `MishpatSignal::ChallengeOutcomeCreated` signal projection.
        // Auth required because outcomes carry constitutional reasoning and
        // indemnification actions.
        .route(
            Route::get("/db/challenge-outcomes")
                .handler("list_challenge_outcomes")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/challenge-outcomes/{cid}")
                .handler("get_challenge_outcome")
                .auth_required()
                .build(),
        )
        // ── REA commitments projection query (EprRouter source) ─────────────────
        .route(
            Route::get("/db/rea_commitments")
                .handler("list_epr_projection_commitments")
                .description(
                    "List project-epr projection commitments for a doorway (EprRouter source)",
                )
                .cache_ttl(30)
                .build(),
        )
        // ── Conductor-bridge routes (Phase 10 HTTP pipes, Phase 11 full queries) ──
        // Gate-client pull resolvers (SourceChainResolver, DhtResolver) target these.
        // Phase 10: returns empty/404 honest stubs.
        // Phase 11: wire HcClient into HttpServer to serve real conductor data.
        .route(
            Route::get("/db/source-chain/{agent_id}/entries")
                .handler("source_chain_entries")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/db/dht/{entry_hash}")
                .handler("dht_entry")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/account — M5 Auth Portal Convergence (Recovery Phase 2)
        // =====================================================================
        .route(
            Route::get("/api/v1/account")
                .handler("get_account")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/account/keys")
                .handler("get_account_keys")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/account/revocations")
                .handler("get_account_revocations")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/account/self-revocation")
                .handler("post_self_revocation")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/account/pending-recovery")
                .handler("get_pending_recovery")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/account/recovery/{id}/vote")
                .handler("post_recovery_vote")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/account/portal-hosts")
                .handler("get_portal_hosts")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/account/portal-hosts")
                .handler("post_portal_host")
                .auth_required()
                .build(),
        )
        .route(
            Route::delete("/api/v1/account/portal-hosts/{url_b64}")
                .handler("delete_portal_host")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/blob — Blob-scoped views (Phase 5 T29)
        // =====================================================================
        .route(
            // Visitor-permitted distribution surface; the storage handler branches
            // Visitor vs Steward via the agent_cid cascade in account::extract_agent_cid.
            // No auth_required() so the public surface (replicas, health, reach,
            // freshness) remains addressable without a session.
            Route::get("/api/v1/blob/{hash}/distribution/details")
                .handler("get_blob_distribution_details")
                .cache_ttl(5)
                .rate_limit(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/cluster — Federated agent-scoped cluster view (Phase 5 T30)
        // =====================================================================
        .route(
            Route::get("/api/v1/cluster")
                .handler("get_cluster")
                .auth_required()
                .cache_ttl(2)
                .rate_limit(30)
                .build(),
        )
        // =====================================================================
        // /api/v1/peer-topology — Peer-household topology view (Phase 5 T31)
        // =====================================================================
        .route(
            Route::get("/api/v1/peer-topology")
                .handler("get_peer_topology")
                .auth_required()
                .cache_ttl(2)
                .rate_limit(30)
                .build(),
        )
        // =====================================================================
        // /api/v1/reciprocity — Local REA reciprocity ledger (Phase 5 T32)
        // =====================================================================
        .route(
            Route::get("/api/v1/reciprocity")
                .handler("get_reciprocity")
                .auth_required()
                .cache_ttl(10)
                .rate_limit(20)
                .build(),
        )
        // =====================================================================
        // /api/v1/weave — Cluster-scoped operational-weave lens (Slice 4)
        // Viewer-less + node-scoped (no auth, no app-id): the gauge and the
        // WeaveView reflect the whole node's placement/coverage/capacity/region
        // state. Dispatch lives in api/mod.rs (`sub_path == "weave"`); this
        // registry entry carries its cache metadata and is the route-shadow
        // guard (asserted by `test_manifest_builds`).
        // =====================================================================
        .route(
            Route::get("/api/v1/weave")
                .handler("get_weave")
                .cache_ttl(10)
                .rate_limit(30)
                .build(),
        )
        // =====================================================================
        // /api/v1/epr core surface (api/epr.rs). The controller has served
        // GET list/:cid and PUT :cid since Phase 3.5, but only the three
        // projection reads below were ever declared — so the doorway's
        // manifest matcher 404'd every core-EPR call (seed-epr-atoms failed
        // 5/5 on the 2026-08-16 local mesh; the storage handler was never
        // reached). Declare what the controller serves.
        // =====================================================================
        .route(
            Route::get("/api/v1/epr")
                .handler("list_epr")
                .cache_ttl(10)
                .public_if_reach("commons")
                .build(),
        )
        .route(
            Route::get("/api/v1/epr/{cid}")
                .handler("get_epr")
                .cache_ttl(30)
                .public_if_reach("commons")
                .build(),
        )
        .route(
            Route::put("/api/v1/epr/{cid}")
                .handler("put_epr")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/epr/:cid/nav-context — EPR nav-context projection (Category C read-only)
        // Zero new DHT entry types; zero new tables; zero migrations.
        // =====================================================================
        .route(
            Route::get("/api/v1/epr/{cid}/nav-context")
                .handler("get_epr_nav_context")
                .cache_ttl(30)
                .public_if_reach("commons")
                .build(),
        )
        // =====================================================================
        // /api/v1/epr/:cid/raw — EPR-content facing inspector (Category C read-only)
        // Folds epr_coupling into legs + neighborhood degree + reverse part-of.
        // Zero new DHT entry types; zero new tables; zero migrations.
        // =====================================================================
        .route(
            Route::get("/api/v1/epr/{cid}/raw")
                .handler("get_epr_raw")
                .cache_ttl(30)
                .public_if_reach("commons")
                .build(),
        )
        // =====================================================================
        // /api/v1/epr/:scope/lens-market — plural-Mishpat lens market (lens-market S7).
        // `:scope` is the EPR SLUG-ID (e.g. epr:lamad-spa), NOT a dag-cbor CID.
        // Read-projection of the A-class `lenses` table keyed on `governs_epr`.
        // Dispatch lives in api/epr.rs (`is_lens_market_path`); THIS registry entry
        // is the route-shadow guard — unregistered, doorway's manifest matcher would
        // miss it and the hosted path would fall through to the SPA bootstrap (HTML).
        // =====================================================================
        .route(
            Route::get("/api/v1/epr/{scope}/lens-market")
                .handler("get_epr_lens_market")
                .cache_ttl(30)
                .public_if_reach("commons")
                .build(),
        )
        // =====================================================================
        // /api/v1/collective + /api/v1/collab — Multi-collective collaboration EPR M1
        // See genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md
        // Writes require auth (imagodei conductor bridge); reads are public.
        // =====================================================================
        .route(
            Route::post("/api/v1/collective")
                .handler("create_collective")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/collective/{cid}")
                .handler("get_collective")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/collab/agreement")
                .handler("create_collab_agreement")
                .auth_required()
                .build(),
        )
        .route(
            Route::post("/api/v1/collab/agreement/{cid}/attest")
                .handler("attest_collab_agreement")
                .auth_required()
                .build(),
        )
        .route(
            Route::get("/api/v1/collab/{cid}")
                .handler("get_collab_qahal")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/collab/{cid}/withdraw")
                .handler("withdraw_membership")
                .auth_required()
                .build(),
        )
        // =====================================================================
        // /api/v1/vf-graphql — Wave 3 M1 valueflows bridge endpoint.
        // Stub-stage: serves fixture EconomicEvent via the bridge's GraphQL schema.
        // M2+ adds identity bridge, M3+ adds real hREA projection.
        // See genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md
        // =====================================================================
        .route(
            Route::post("/api/v1/vf-graphql")
                .handler("vf_graphql_handler")
                .auth_required()
                // M1 fixture path has no DHT cost; higher per-window allowance than
                // reciprocity (20) is acceptable. Revisit at M3 when real hREA
                // projection adds per-request DHT read cost.
                .rate_limit(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/graphql — graph-native GraphQL viewer (CozoDB-backed).
        // Consumed by elohim-app topology surfaces (ClusterService viewer-hub
        // query under `useGraphqlTopology`). Identity-scoped resolvers read the
        // agentCid variable, so the edge requires a bearer like /api/v1/cluster.
        // Without this manifest entry the doorway proxy 404s the route and the
        // cluster page falls into its error branch.
        // =====================================================================
        .route(
            Route::post("/api/v1/graphql")
                .handler("graphql_viewer")
                .auth_required()
                .rate_limit(60)
                .build(),
        )
        // =====================================================================
        // /api/v1/placement-gaps — resilience placement-gap projection reads.
        // =====================================================================
        .route(
            Route::get("/api/v1/placement-gaps")
                .handler("get_placement_gaps")
                .auth_required()
                .cache_ttl(15)
                .build(),
        )
        // =====================================================================
        // /lamad/* — SSR-eligible routes (Task 12)
        //
        // These paths are declared with `render: "angular-ssr"`. Doorway
        // dispatches them to its in-process renderer (Task 13) instead of
        // proxying directly to the JSON handler. The handler name is still
        // required so the SSR DataFetcher can call back through the doorway
        // resolver to hydrate the Angular component tree.
        //
        // Handler mapping rationale (none of the view-specific handler names
        // exist yet; we map to the nearest data-equivalent handler):
        //   /lamad/concept/{id}        → get_content    (content node by id)
        //   /lamad/path/{slug}         → list_content   (path = filtered content set)
        //   /lamad/path/{slug}/step/{n}→ get_content    (a step is a content node)
        //   /                          → list_content   (landing aggregates content)
        // =====================================================================
        .route(
            Route::get("/lamad/concept/{id}")
                .handler("get_content")
                .cache_ttl(60)
                .public_if_reach("commons")
                .render("angular-ssr")
                .build(),
        )
        .route(
            Route::get("/lamad/path/{slug}")
                .handler("list_content")
                .cache_ttl(60)
                .public_if_reach("commons")
                .render("angular-ssr")
                .build(),
        )
        .route(
            Route::get("/lamad/path/{slug}/step/{n}")
                .handler("get_content")
                .cache_ttl(60)
                .public_if_reach("commons")
                .render("angular-ssr")
                .build(),
        )
        .route(
            Route::get("/")
                .handler("list_content")
                .cache_ttl(60)
                .public_if_reach("commons")
                .render("angular-ssr")
                .build(),
        )
        // Blob proxy: doorway caches blobs from /blob/{hash}
        .with_blobs_at("/blob")
        .build()
}

/// Simple response container for integration test assertions.
///
/// Defined at module level WITHOUT `#[cfg(test)]` so it is visible to
/// `tests/` integration test binaries — those compile the library crate
/// without `cfg(test)`, so anything guarded by that flag is invisible to them.
pub struct HttpTestResponse {
    pub status: u16,
    pub body: bytes::Bytes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_format() {
        let hash = BlobStore::compute_hash(b"test data");
        assert!(hash.starts_with("sha256-"));
        assert_eq!(hash.len(), 7 + 64); // "sha256-" + 64 hex chars
    }

    // ── /chrome native-chrome asset route (default build, V8-free) ──────────

    async fn collect_body(resp: Response<Full<Bytes>>) -> (StatusCode, hyper::HeaderMap, Vec<u8>) {
        let status = resp.status();
        let headers = resp.headers().clone();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, headers, bytes.to_vec())
    }

    #[tokio::test]
    async fn chrome_serves_content_addressed_element_immutable() {
        let path = elohim_chrome_asset::element_script_path();
        let resp = HttpServer::handle_chrome_asset(&path);
        let (status, headers, body) = collect_body(resp).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(
            headers.get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(body, elohim_chrome_asset::element_js_bytes());
    }

    #[tokio::test]
    async fn chrome_serves_stable_alias_revalidating() {
        let resp = HttpServer::handle_chrome_asset(elohim_chrome_asset::STABLE_ELEMENT_PATH);
        let (status, headers, body) = collect_body(resp).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "text/javascript; charset=utf-8"
        );
        // Stable alias serves the CURRENT element ⇒ revalidate, not immutable.
        assert_eq!(
            headers.get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=0, must-revalidate"
        );
        // ETag carries the content address so a client can skip the body.
        assert_eq!(
            headers.get(header::ETAG).unwrap(),
            &format!("\"{}\"", elohim_chrome_asset::element_js_hash())
        );
        // Same bytes as the content-addressed path.
        assert_eq!(body, elohim_chrome_asset::element_js_bytes());
    }

    #[tokio::test]
    async fn chrome_stale_hash_is_404() {
        let resp = HttpServer::handle_chrome_asset("/chrome/omni-element.deadbeef.js");
        let (status, _h, _b) = collect_body(resp).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn chrome_unrelated_path_is_404() {
        let resp = HttpServer::handle_chrome_asset("/chrome/not-a-thing.js");
        let (status, _h, _b) = collect_body(resp).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn chrome_route_dispatches_through_handle_request() {
        // The is_service_path-equivalent guard: a GET /chrome/* request must
        // reach handle_chrome_asset, not fall through to the 404 catch-all. We
        // assert the dispatch contract at the prefix-match level (the arm uses
        // `p.starts_with("/chrome/")`), proving the explicit arm shadows the
        // catch-all for every /chrome path.
        let hashed = elohim_chrome_asset::element_script_path();
        assert!(
            hashed.starts_with("/chrome/"),
            "hashed path under /chrome: {hashed}"
        );
        assert!(
            elohim_chrome_asset::STABLE_ELEMENT_PATH.starts_with("/chrome/"),
            "stable alias under /chrome"
        );
        // And a known path returns 200 (would be 404 if the catch-all shadowed).
        let (status, _h, _b) = collect_body(HttpServer::handle_chrome_asset(&hashed)).await;
        assert_eq!(status, StatusCode::OK);
    }

    /// Construct an UpdateContentInputView with every field defaulted to None,
    /// overriding only `reach` and `blob_hash`. UpdateContentInputView does not
    /// derive Default (shared ts-rs-anchored view; adding a derive would touch
    /// every consumer), so build it explicitly.
    fn patch_view(reach: Option<&str>, blob_hash: Option<&str>) -> UpdateContentInputView {
        UpdateContentInputView {
            title: None,
            description: None,
            content_body: None,
            content_format: None,
            metadata: None,
            tags: None,
            reach: reach.map(str::to_string),
            blob_hash: blob_hash.map(str::to_string),
            server_blob_hash: None,
            p2p_published_at: None,
        }
    }

    #[test]
    fn reach_only_patch_requires_conductor_route() {
        // A PATCH that changes reach (no blob) must route to the conductor, not
        // diesel — reach is class-A DNA-notarized; a diesel-only write is
        // reverted by the reconciliation controller (DHT wins).
        let view = patch_view(Some("commons"), None);
        assert!(
            patch_needs_conductor(&view),
            "reach-only PATCH must re-notarize via conductor"
        );
    }

    #[test]
    fn metadata_only_patch_stays_diesel() {
        // A PATCH with neither blob nor reach does NOT need the conductor.
        let view = patch_view(None, None);
        assert!(
            !patch_needs_conductor(&view),
            "non-notarized PATCH can use diesel"
        );

        // A title-only patch (no blob, no reach) does not need the conductor.
        let titled = UpdateContentInputView {
            title: Some("new title".into()),
            ..patch_view(None, None)
        };
        assert!(
            !patch_needs_conductor(&titled),
            "title-only PATCH can use diesel"
        );

        // A serverBlobHash-only PATCH (the SSR deploy path) must NOT route to the
        // conductor — server_blob_hash is a deploy-projection artifact, not a
        // DNA-notarized content-entry field. Routing it through the conductor
        // would 503 on household/local stacks with no conductor bridge.
        let ssr = UpdateContentInputView {
            server_blob_hash: Some("sha256-ssrserver".into()),
            ..patch_view(None, None)
        };
        assert!(
            !patch_needs_conductor(&ssr),
            "serverBlobHash-only PATCH must stay on the diesel-direct path"
        );
    }

    #[tokio::test]
    async fn account_import_keeps_peer_global_content_reach_unchanged() {
        use crate::db::content_diesel::{self, CreateContentInput, MinTrust};
        use crate::test_util::test_pool;

        let pool = test_pool();
        let ctx = AppContext::default();
        {
            let mut conn = pool.get().unwrap();
            content_diesel::create_content(
                &mut conn,
                &ctx,
                CreateContentInput {
                    id: "shared-learning-node".into(),
                    title: "Shared learning node".into(),
                    description: None,
                    content_type: "concept".into(),
                    content_format: "markdown".into(),
                    blob_hash: None,
                    blob_cid: None,
                    content_size_bytes: None,
                    metadata_json: None,
                    reach: "community".into(),
                    created_by: None,
                    content_body: None,
                    tags: Vec::new(),
                    dht_anchor_hash: None,
                },
            )
            .unwrap();
        }

        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let server =
            HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(pool.clone());
        let body = serde_json::json!({
            "schemaVersion": 1,
            "identity": {
                "humanId": "human-susan-household",
                "displayName": "Susan"
            },
            "content": [{
                "contentId": "shared-learning-node",
                "reach": "familiar",
                "reason": "affinity",
                "stewardRatio": null
            }],
            "relationships": [],
            "stewardship": [],
            "collectives": []
        });
        let req = Request::builder()
            .method(Method::POST)
            .uri("/account/import")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap();

        let response = server.do_account_import(req, pool.clone()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let mut conn = pool.get().unwrap();
        let row = content_diesel::get_content(
            &mut conn,
            &ctx,
            "shared-learning-node",
            MinTrust::Invisible,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            row.reach, "community",
            "viewer-relative account-package reach must not mutate peer-global content reach"
        );
    }

    #[test]
    fn test_manifest_builds() {
        let manifest = build_manifest();
        assert!(!manifest.routes.is_empty());
        assert!(manifest.blob_proxy.is_some());
        // Spot-check a few known routes exist
        let paths: Vec<&str> = manifest.routes.iter().map(|r| r.path.as_str()).collect();
        assert!(paths.contains(&"/db/content"), "missing /db/content");
        assert!(
            paths.contains(&"/api/v1/mastery"),
            "missing /api/v1/mastery"
        );
        assert!(
            paths.contains(&"/api/v1/economic-events"),
            "missing /api/v1/economic-events"
        );
        assert!(
            paths.contains(&"/api/v1/presence"),
            "missing /api/v1/presence"
        );
        assert!(
            paths.contains(&"/account/import"),
            "missing /account/import (seeder regression)"
        );
        assert!(
            paths.contains(&"/account/export/{human_id}"),
            "missing /account/export/{{human_id}} (recovery regression)"
        );
        // Gate decision attestation routes (mishpat DHT projection)
        assert!(
            paths.contains(&"/db/gate-decisions"),
            "missing /db/gate-decisions"
        );
        assert!(
            paths.contains(&"/db/gate-decisions/{cid}"),
            "missing /db/gate-decisions/{{cid}}"
        );
        // Gate decision challenge routes (mishpat DHT projection — Phase 11 Task 11.2)
        assert!(
            paths.contains(&"/db/gate-decision-challenges"),
            "missing /db/gate-decision-challenges"
        );
        assert!(
            paths.contains(&"/db/gate-decision-challenges/{cid}"),
            "missing /db/gate-decision-challenges/{{cid}}"
        );
        // Challenge outcome routes (mishpat DHT projection — Phase 11 Task 11.2)
        assert!(
            paths.contains(&"/db/challenge-outcomes"),
            "missing /db/challenge-outcomes"
        );
        assert!(
            paths.contains(&"/db/challenge-outcomes/{cid}"),
            "missing /db/challenge-outcomes/{{cid}}"
        );
        // Phase 5 topology routes — Light-Up-The-Topology sprint
        assert!(
            paths.contains(&"/api/v1/blob/{hash}/distribution/details"),
            "missing /api/v1/blob/{{hash}}/distribution/details (T29)"
        );
        assert!(
            paths.contains(&"/api/v1/cluster"),
            "missing /api/v1/cluster (T30)"
        );
        assert!(
            paths.contains(&"/api/v1/peer-topology"),
            "missing /api/v1/peer-topology (T31)"
        );
        assert!(
            paths.contains(&"/api/v1/reciprocity"),
            "missing /api/v1/reciprocity (T32)"
        );
        // Operational-weave Slice 4 — cluster-scoped weave lens.
        // The registry entry is the route-shadow guard in this crate's
        // architecture: a viewer-less `/api/v1/weave` not registered here would
        // dispatch through api/mod.rs but carry no cache/auth metadata and be
        // invisible to this coverage assertion (the storage analogue of the
        // doorway is_service_path guard — project_doorway_main_route_needs_is_service_path).
        assert!(
            paths.contains(&"/api/v1/weave"),
            "missing /api/v1/weave (operational-weave Slice 4)"
        );
        // REA economic facing Wave 4.2 — per-commitment mishpat-ledger read surface.
        // Same route-shadow guard discipline as /api/v1/weave: a `facing/rea` arm
        // dispatched through api/rea_commitments.rs but NOT registered here would
        // carry no cache metadata and be invisible to this coverage assertion.
        assert!(
            paths.contains(&"/api/v1/commitments/facing/rea"),
            "missing /api/v1/commitments/facing/rea (REA economic facing Wave 4.2)"
        );
        assert!(
            paths.contains(&"/api/v1/commitments/capacity"),
            "missing /api/v1/commitments/capacity (capacity pledge consent lever)"
        );
        assert!(
            paths.contains(&"/api/v1/commitments/capacity/ratios"),
            "missing /api/v1/commitments/capacity/ratios (consent-legibility read for the capacity pledge attestation)"
        );
        // Plural-Mishpat lens market (lens-market S7). Same route-shadow guard
        // discipline as /api/v1/weave: the GET arm is dispatched through
        // api/epr.rs (`is_lens_market_path`) but, unregistered here, would be
        // invisible to doorway's manifest matcher and fall through to the SPA
        // bootstrap (HTML, not JSON) on every hosted path — unreachable except
        // storage-direct (project_doorway_main_route_needs_is_service_path).
        assert!(
            paths.contains(&"/api/v1/epr/{scope}/lens-market"),
            "missing /api/v1/epr/{{scope}}/lens-market (lens-market S7 — doorway reachability)"
        );
        // Wave 3 M1 — vf-graphql bridge endpoint
        assert!(
            paths.contains(&"/api/v1/vf-graphql"),
            "missing /api/v1/vf-graphql (Wave 3 M1)"
        );
        // Graph-native viewer + placement-gaps — topology surfaces proxy path
        assert!(
            paths.contains(&"/api/v1/graphql"),
            "missing /api/v1/graphql (graph-native viewer; cluster page GraphQL path)"
        );
        assert!(
            paths.contains(&"/api/v1/placement-gaps"),
            "missing /api/v1/placement-gaps (resilience projection)"
        );
        // Multi-collective collaboration EPR M1 — qahal routes (Task 11)
        assert!(
            paths.contains(&"/api/v1/collective"),
            "missing /api/v1/collective (M1 Task 11)"
        );
        assert!(
            paths.contains(&"/api/v1/collective/{cid}"),
            "missing /api/v1/collective/{{cid}} (M1 Task 11)"
        );
        assert!(
            paths.contains(&"/api/v1/collab/agreement"),
            "missing /api/v1/collab/agreement (M1 Task 11)"
        );
        assert!(
            paths.contains(&"/api/v1/collab/agreement/{cid}/attest"),
            "missing /api/v1/collab/agreement/{{cid}}/attest (M1 Task 11)"
        );
        assert!(
            paths.contains(&"/api/v1/collab/{cid}"),
            "missing /api/v1/collab/{{cid}} (M1 Task 11)"
        );
        assert!(
            paths.contains(&"/api/v1/collab/{cid}/withdraw"),
            "missing /api/v1/collab/{{cid}}/withdraw (M1 Task 11)"
        );
        // SSR-eligible lamad routes (Task 12) — verify paths and render field
        let lamad_routes: std::collections::HashMap<&str, _> = manifest
            .routes
            .iter()
            .filter(|r| {
                matches!(
                    r.path.as_str(),
                    "/lamad/concept/{id}"
                        | "/lamad/path/{slug}"
                        | "/lamad/path/{slug}/step/{n}"
                        | "/"
                )
            })
            .map(|r| (r.path.as_str(), r))
            .collect();
        let concept = lamad_routes
            .get("/lamad/concept/{id}")
            .expect("missing /lamad/concept/{id} (Task 12)");
        assert_eq!(
            concept.render.as_deref(),
            Some("angular-ssr"),
            "/lamad/concept/{{id}} must declare render=angular-ssr"
        );
        let path_view = lamad_routes
            .get("/lamad/path/{slug}")
            .expect("missing /lamad/path/{slug} (Task 12)");
        assert_eq!(
            path_view.render.as_deref(),
            Some("angular-ssr"),
            "/lamad/path/{{slug}} must declare render=angular-ssr"
        );
        let step_view = lamad_routes
            .get("/lamad/path/{slug}/step/{n}")
            .expect("missing /lamad/path/{slug}/step/{n} (Task 12)");
        assert_eq!(
            step_view.render.as_deref(),
            Some("angular-ssr"),
            "/lamad/path/{{slug}}/step/{{n}} must declare render=angular-ssr"
        );
        let landing = lamad_routes
            .get("/")
            .expect("missing / (landing route — Task 12)");
        assert_eq!(
            landing.render.as_deref(),
            Some("angular-ssr"),
            "/ must declare render=angular-ssr"
        );
        // M-REA-1: intent-driven EconomicEvent composition
        assert!(
            paths.contains(&"/api/v1/lamad/events"),
            "missing /api/v1/lamad/events (M-REA-1)"
        );
        // Ensure infrastructure routes are NOT in the manifest
        assert!(
            !paths.contains(&"/health"),
            "health should not be in manifest"
        );
        assert!(
            !paths.iter().any(|p| p.starts_with("/shard")),
            "shard routes should not be in manifest"
        );
        // FLIPPED 2026-06-27: /sync/v1 doc-sync routes ARE now declared so
        // doorway's route registry proxies browser doc-sync to storage. This
        // deliberately reverses the prior "sync routes should not be in manifest"
        // guard (security-adjacent — exposes the CRDT doc-sync plane through
        // doorway). See the build_manifest design-rule note + is_service_path("/sync").
        assert!(
            paths.iter().any(|p| p.starts_with("/sync/v1/")),
            "sync routes must be declared in manifest (doorway doc-sync carriage)"
        );
        // Verify blob proxy points to /blob
        assert_eq!(manifest.blob_proxy.unwrap().base_path, "/blob");
    }

    // -- SPA deep-link fallback: ROUTE/ASSET discrimination (§12.2) --
    //
    // `is_spa_route_subpath` is the storage half of the two-layer drift guard.
    // It MUST agree with doorway-service's `derive_app_subpath` discriminator,
    // so both drive off the ONE shared test-vector table. Editing the rule here
    // without updating the fixture (or vice versa) breaks this test.

    /// Shared fixture mirror — kept minimal: just the fields this crate reads.
    #[derive(serde::Deserialize)]
    struct SpaVector {
        #[serde(rename = "subPath")]
        sub_path: String,
        kind: String,
        note: String,
    }

    #[derive(serde::Deserialize)]
    struct SpaVectorTable {
        vectors: Vec<SpaVector>,
    }

    /// Table-driven over the shared cross-crate fixture (the two-layer drift
    /// guard). Each vector asserts ROUTE => fallback-eligible, ASSET => honest
    /// 404.
    #[test]
    fn is_spa_route_subpath_matches_shared_vectors() {
        const FIXTURE: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../sdk/fixtures/spa-route-discrimination.vectors.json"
        ));
        let table: SpaVectorTable =
            serde_json::from_str(FIXTURE).expect("shared spa-route fixture must parse");
        assert!(
            !table.vectors.is_empty(),
            "shared spa-route fixture must carry vectors"
        );

        for v in &table.vectors {
            let expected_route = match v.kind.as_str() {
                "route" => true,
                "asset" => false,
                other => panic!("unexpected vector kind {other:?} for {:?}", v.sub_path),
            };
            assert_eq!(
                is_spa_route_subpath(&v.sub_path),
                expected_route,
                "vector {:?} (kind={}) misclassified — {}",
                v.sub_path,
                v.kind,
                v.note,
            );
        }
    }

    /// Direct edge-case assertions independent of the fixture, documenting the
    /// rule's boundaries (final-segment-only; non-final dots are irrelevant).
    #[test]
    fn is_spa_route_subpath_edge_cases() {
        // Routes: no dot in the final segment.
        assert!(is_spa_route_subpath("explore"));
        assert!(is_spa_route_subpath(
            "path/foundations-christian-technology/step/2"
        ));
        assert!(is_spa_route_subpath("v1.2/release-notes")); // dot in NON-final segment
        assert!(is_spa_route_subpath("")); // bare/empty → route-eligible
                                           // Assets: dot in the final segment.
        assert!(!is_spa_route_subpath("main-7J5AOAQZ.js"));
        assert!(!is_spa_route_subpath("assets/img/logo.svg"));
        assert!(!is_spa_route_subpath("media/file.name.with.dots.png"));
        assert!(!is_spa_route_subpath(".well-known/assetlinks.json"));
    }

    /// Build a minimal SPA bundle ZIP (index.html + one hashed asset) and store
    /// it; returns `(server, content_address)`. No extraction_cache configured,
    /// so `handle_app_request` goes straight through the fresh-extraction path
    /// and the content-address identifier bypasses slug/DB resolution.
    async fn spa_bundle_server() -> (HttpServer, String) {
        use std::io::Write as _;
        use zip::write::SimpleFileOptions;

        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );

        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            // Stored (no compression) — independent of the `deflate` feature on
            // the write side; ZipArchive reads either method on extraction.
            let opts =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            zw.start_file("index.html", opts).unwrap();
            zw.write_all(b"<!doctype html><title>shell</title>")
                .unwrap();
            zw.start_file("main-7J5AOAQZ.js", opts).unwrap();
            zw.write_all(b"console.log('app')").unwrap();
            zw.finish().unwrap();
        }
        let hash = blob_store.store(&buf).await.unwrap().hash;
        let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap());
        (server, hash)
    }

    async fn body_bytes(resp: Response<Full<Bytes>>) -> Bytes {
        resp.into_body().collect().await.unwrap().to_bytes()
    }

    /// ROUTE miss → 200 + the bundle's index.html + `X-SPA-Fallback: 1`.
    #[tokio::test]
    async fn app_request_route_miss_serves_index_html_fallback() {
        let (server, cid) = spa_bundle_server().await;
        let resp = server
            .handle_app_request(
                &format!("/apps/{cid}/path/foundations-christian-technology"),
                "",
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("X-SPA-Fallback").unwrap(), "1");
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html"
        );
        let body = body_bytes(resp).await;
        assert!(body.starts_with(b"<!doctype html>"));
    }

    /// ROUTE miss with the doorway opt-out (`?spaFallback=0`, the wire form of a
    /// `spa_fallback=false` projection) → the safety net is suppressed and the
    /// ROUTE miss surfaces the honest 404 instead of index.html. This is the
    /// two-layer consistency guard: doorway is contract-authoritative for
    /// `spa_fallback`; storage's convention-level fallback honours the opt-out
    /// when doorway propagates it. End-to-end of the `spa_fallback=false` flow.
    #[tokio::test]
    async fn app_request_route_miss_respects_spa_fallback_opt_out() {
        let (server, cid) = spa_bundle_server().await;
        let resp = server
            .handle_app_request(
                &format!("/apps/{cid}/path/foundations-christian-technology"),
                "spaFallback=0",
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert!(resp.headers().get("X-SPA-Fallback").is_none());
        let body = body_bytes(resp).await;
        assert!(body.starts_with(b"{\"error\""));
    }

    /// ASSET miss → today's 404 JSON, never masked by index.html.
    #[tokio::test]
    async fn app_request_asset_miss_stays_404() {
        let (server, cid) = spa_bundle_server().await;
        let resp = server
            .handle_app_request(&format!("/apps/{cid}/main-DEADBEEF.js"), "")
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert!(resp.headers().get("X-SPA-Fallback").is_none());
        let body = body_bytes(resp).await;
        assert!(body.starts_with(b"{\"error\""));
    }

    /// An exact-named bundle file (here the hashed JS asset that DOES exist) is
    /// still served verbatim — the fallback fires only on a miss.
    #[tokio::test]
    async fn app_request_existing_file_served_verbatim() {
        let (server, cid) = spa_bundle_server().await;
        let resp = server
            .handle_app_request(&format!("/apps/{cid}/main-7J5AOAQZ.js"), "")
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(resp.headers().get("X-SPA-Fallback").is_none());
        let body = body_bytes(resp).await;
        assert_eq!(&body[..], b"console.log('app')");
    }
}

/// Tests for the iroh blob backend wiring (Plan 2 Task 3).
///
/// These tests verify that `HttpServer` correctly exposes and initializes
/// the four new fields added for cutover gate #2.
#[cfg(all(test, feature = "p2p-iroh"))]
mod blob_backend_wiring_tests {
    use super::*;

    #[tokio::test]
    async fn http_server_defaults_iroh_blob_store_to_none() {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap());
        assert!(server.iroh_blob_store.is_none());
        assert!(server.self_transport_manifest.is_none());
        assert_eq!(server.blob_iroh_served_count_snapshot(), 0);
        assert_eq!(server.blob_libp2p_served_count_snapshot(), 0);
    }

    #[tokio::test]
    async fn with_iroh_blob_store_sets_field() {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let iroh_dir = tempfile::tempdir().unwrap();
        let iroh = Arc::new(
            crate::p2p_iroh::IrohBlobStore::load(&iroh_dir.path().join("blobs_iroh"))
                .await
                .unwrap(),
        );
        let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
            .with_iroh_blob_store(iroh.clone());
        assert!(server.iroh_blob_store.is_some());
    }

    #[tokio::test]
    async fn with_self_transport_manifest_sets_field() {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let manifest =
            Arc::new(crate::p2p_iroh::peer_map::PeerTransportManifest::iroh_capable_for_test());
        let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
            .with_self_transport_manifest(manifest);
        assert!(server.self_transport_manifest.is_some());
    }

    #[tokio::test]
    async fn status_response_includes_blob_backend_counters() {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap());
        // Simulate 7 iroh-served and 13 libp2p-served blobs.
        // We call handle_blob_get_for_test in a way that won't actually serve
        // (empty hash → BAD_REQUEST), but we want to test the counter wiring.
        // Instead, directly increment by atomically storing after construction.
        // Access the counters via the Arc handles stored in the server struct
        // through the snapshot helper (which reads them without mutation).
        // We need to drive the counters up — use the Arc clones we can access
        // via the server's public counter snapshot methods for verification.
        //
        // To mutate, we need to reach the Arc — expose a test helper:
        server.set_iroh_served_count_for_test(7);
        server.set_libp2p_served_count_for_test(13);

        let resp = server.handle_status_for_test().await.unwrap();
        let body = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .unwrap()
            .to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["blobs"]["iroh_served"].as_u64(), Some(7));
        assert_eq!(v["blobs"]["libp2p_served"].as_u64(), Some(13));
    }
}

// =============================================================================
// Apps-resolver blob-heal tests — resilient-dual-doorway Fix B
//
// The apps resolver (`handle_app_request`) shares the `GET /blob/{hash}` T17
// heal-on-miss path via `get_blob_or_heal`. These tests pin the behaviour that
// is verifiable WITHOUT a live p2p mesh: the non-p2p path (a local miss still
// 404s with the identical body), the local-hit path (a present blob still
// serves), and the pure helper decision logic. The heal-success leg requires a
// running swarm and a hosting peer, so it is exercised by the soak/a2o layer,
// not here.
//
// Plain `#[cfg(test)]` so the module compiles under the default feature set
// (which includes `p2p`); the `get_blob_or_heal` helper degrades to `NotFound`
// when no `p2p_handle`/`db_pool` is wired, which is exactly what these tests
// drive.
// =============================================================================
#[cfg(test)]
mod apps_resolver_heal_tests {
    use super::*;
    use http_body_util::BodyExt;
    use std::io::Write;

    async fn test_server() -> HttpServer {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        // No p2p_handle and no db_pool wired → `get_blob_or_heal` cannot heal
        // and must degrade to `NotFound`, the pre-Fix-B 404 behaviour.
        HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
    }

    /// Build a minimal valid single-file ZIP (`index.html`) in memory.
    fn tiny_zip() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("index.html", opts).unwrap();
            zip.write_all(b"<!doctype html><title>ok</title>").unwrap();
            zip.finish().unwrap();
        }
        buf
    }

    async fn body_string(resp: Response<Full<Bytes>>) -> (StatusCode, String) {
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).to_string())
    }

    /// Fix B core: a missing app ZIP, with NO peer-heal possible (no p2p handle
    /// / no db pool), still returns 404 with the exact pre-refactor body. The
    /// identifier is a content address, so it bypasses slug resolution and goes
    /// straight to the `get_blob_or_heal` arm.
    #[tokio::test]
    async fn apps_resolver_missing_blob_404s_with_identical_body_no_p2p() {
        let server = test_server().await;
        // 64-hex sha256 content address → is_content_address() true → is_cid
        // path, cached_blob_hash = Some(identifier).
        let cid = format!("sha256-{}", "a".repeat(64));
        let resp = server
            .handle_app_request(&format!("/apps/{cid}/index.html"), "")
            .await
            .unwrap();
        let (status, body) = body_string(resp).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // Body byte-identical to the pre-Fix-B handler.
        assert_eq!(
            body,
            format!(r#"{{"error": "App ZIP blob not found: {cid}"}}"#)
        );
    }

    /// The heal path's local-hit arm preserves normal serving: a blob present in
    /// the local store is extracted and served (200) without any heal attempt.
    #[tokio::test]
    async fn apps_resolver_serves_local_hit() {
        let server = test_server().await;
        let zip = tiny_zip();
        let stored = server.blob_store.store(&zip).await.unwrap();
        // `store` returns the canonical `sha256-{hex}` hash; the resolver accepts
        // a content-address identifier in that form.
        let cid = stored.hash.clone();
        assert!(cid.starts_with("sha256-"), "store yields sha256- form");

        let resp = server
            .handle_app_request(&format!("/apps/{cid}/index.html"), "")
            .await
            .unwrap();
        let (status, body) = body_string(resp).await;
        assert_eq!(status, StatusCode::OK, "local-hit must serve, body={body}");
        assert!(body.contains("<title>ok</title>"));
    }

    /// The `GET /blob/{hash}` refactor is observably unchanged on a local miss
    /// with no peer-heal possible: still 404 with the "Blob not found" body.
    #[tokio::test]
    async fn blob_route_missing_blob_404s_with_identical_body_no_p2p() {
        let server = test_server().await;
        let hash = format!("sha256-{}", "b".repeat(64));
        let resp = server.handle_get_blob(&hash, None).await.unwrap();
        let (status, body) = body_string(resp).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, "Blob not found");
    }

    /// Pure decision logic: a local hit resolves to `Bytes { healed_from: None }`
    /// (no heal attempted).
    #[tokio::test]
    async fn get_blob_or_heal_local_hit_returns_bytes_no_heal() {
        let server = test_server().await;
        let payload = b"hello quilt".to_vec();
        let stored = server.blob_store.store(&payload).await.unwrap();
        match server.get_blob_or_heal(&stored.hash).await {
            BlobHealOutcome::Bytes { bytes, healed_from } => {
                assert_eq!(bytes, payload);
                assert!(
                    healed_from.is_none(),
                    "local hit must not record a heal source"
                );
            }
            other => panic!("expected Bytes on local hit, got {other:?}"),
        }
    }

    /// Pure decision logic: a local miss with no p2p/db wired resolves to
    /// `NotFound` (degraded path), never a panic or an error.
    #[tokio::test]
    async fn get_blob_or_heal_miss_no_p2p_returns_not_found() {
        let server = test_server().await;
        let hash = format!("sha256-{}", "c".repeat(64));
        match server.get_blob_or_heal(&hash).await {
            BlobHealOutcome::NotFound => {}
            other => panic!("expected NotFound on miss with no p2p, got {other:?}"),
        }
    }

    /// Fix B: the `/blob` syncing body is valid JSON with `status:"syncing"`,
    /// echoes the blob hash, and is NOT a "not found" body — a client can
    /// distinguish "retry, syncing" from a 404.
    #[test]
    fn syncing_blob_body_is_syncing_json() {
        let hash = format!("sha256-{}", "d".repeat(64));
        let body = syncing_blob_body(&hash);
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert_eq!(v["status"], "syncing");
        assert_eq!(v["blobHash"], hash);
        assert!(v["hint"].as_str().unwrap().contains("peers"));
        assert!(!body.to_lowercase().contains("not found"));
    }

    /// Fix B: the `/apps/` syncing body is valid JSON keyed by slug and, load-
    /// bearingly, does NOT contain the "App not found" / "App ZIP" text the
    /// federation-deploy scenario greps for — a byte-lag degrade must never read
    /// as a missing deploy.
    #[test]
    fn syncing_app_body_is_syncing_json_not_app_not_found() {
        let slug = "elohim-host-landing";
        let body = syncing_app_body(slug);
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert_eq!(v["status"], "syncing");
        assert_eq!(v["slug"], slug);
        assert!(v["hint"].as_str().unwrap().contains("peers"));
        assert!(!body.contains("App not found"));
        assert!(!body.contains("App ZIP"));
    }

    /// Pins the Fix B outcome variant shape: `Syncing` carries whether a
    /// background heal task was spawned. Both call sites read `started`.
    #[test]
    fn blob_heal_outcome_syncing_carries_started() {
        match (BlobHealOutcome::Syncing { started: true }) {
            BlobHealOutcome::Syncing { started } => assert!(started),
            other => panic!("expected Syncing, got {other:?}"),
        }
    }

    /// CIDv1 base32 (`bafkrei…` raw+sha2-256) is the CANONICAL content address
    /// per the p2p-design-gate; bare `sha256-…` is the legacy alias. Both must
    /// read as content addresses so `/apps/{address}/…` bypasses slug lookup.
    /// A slug that merely starts with `baf` must stay a slug — the recognizer
    /// validates with the `cid` crate rather than pattern-matching a prefix.
    #[test]
    fn is_content_address_recognizes_cidv1_and_rejects_slugs() {
        let sha = format!("sha256-{}", "a".repeat(64));
        assert!(is_content_address(&sha), "legacy sha256- form");

        let cid = BlobStore::hash_to_cid(&sha).unwrap().to_string();
        assert!(cid.starts_with("bafkrei"), "raw+sha2-256 CIDv1, got {cid}");
        assert!(
            is_content_address(&cid),
            "CIDv1 base32 is a content address"
        );

        assert!(!is_content_address("evolution-of-trust"));
        assert!(
            !is_content_address("bafflegab-app"),
            "hyphenated slug is not a CID"
        );
        assert!(!is_content_address("baf"));
        assert!(!is_content_address(""));
    }

    /// Both spellings of ONE content address normalize to the same canonical
    /// `sha256-{hex}` blob key, so `/apps/{cid}/…` and `/apps/{sha256-…}/…`
    /// share one extraction-cache entry instead of extracting the bundle twice.
    #[test]
    fn canonical_content_address_collapses_both_spellings() {
        let sha = format!("sha256-{}", "b".repeat(64));
        let cid = BlobStore::hash_to_cid(&sha).unwrap().to_string();
        assert_eq!(
            canonical_content_address(&cid).as_deref(),
            Some(sha.as_str())
        );
        assert_eq!(
            canonical_content_address(&sha).as_deref(),
            Some(sha.as_str())
        );
        assert_eq!(canonical_content_address("evolution-of-trust"), None);
    }

    /// The regression: `GET /apps/bafkrei…/index.html` 404'd with
    /// "App not found: bafkrei…" because `is_content_address()` knew only the
    /// legacy prefix, so the canonical address fell through to slug lookup.
    /// A locally-present blob must serve under its CIDv1 identifier.
    #[tokio::test]
    async fn apps_resolver_serves_local_hit_via_cidv1_identifier() {
        let server = test_server().await;
        let zip = tiny_zip();
        let stored = server.blob_store.store(&zip).await.unwrap();
        let cid = BlobStore::hash_to_cid(&stored.hash).unwrap().to_string();
        assert!(cid.starts_with("baf"), "CIDv1 identifier, got {cid}");

        let resp = server
            .handle_app_request(&format!("/apps/{cid}/index.html"), "")
            .await
            .unwrap();
        let (status, body) = body_string(resp).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "CID identifier must serve, body={body}"
        );
        assert!(body.contains("<title>ok</title>"));
    }

    /// The capability probe shares the recognizer: a CIDv1 identifier must be
    /// reported as a blob hash, never resolved through `slug_index`.
    #[tokio::test]
    async fn app_capability_reports_blob_hash_for_cidv1_identifier() {
        let server = test_server().await;
        let sha = format!("sha256-{}", "c".repeat(64));
        let cid = BlobStore::hash_to_cid(&sha).unwrap().to_string();
        let resp = server
            .handle_app_capability(&format!("/apps/{cid}/_capability"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("X-Blob-Hash")
                .and_then(|v| v.to_str().ok()),
            Some(sha.as_str()),
            "CID identifier normalizes to the canonical sha256- blob key"
        );
        assert!(
            resp.headers().get("X-Content-Slug").is_none(),
            "a content address is never a slug"
        );
    }
}

// =============================================================================
// /auth/me tests — Task A4 (peer OAuth portal substrate audit §3)
//
// Pure unit tests — no DB or server required. Placed in their own cfg(test)
// module so they compile under the default feature set (no p2p-iroh required).
// =============================================================================
#[cfg(test)]
mod auth_me_tests {
    use super::*;

    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct AuthorityRef {
        label: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    }

    /// Wire shape kept in sync with `doorway/auth_routes.rs::MeResponse`.
    #[derive(Debug, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct StorageMeResponse {
        human_id: String,
        agent_pub_key: String,
        identifier: String,
        permission_level: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        doorway_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        doorway_url: Option<String>,
        authenticated: bool,
        trust_mode: String,
        authority: AuthorityRef,
        #[serde(skip_serializing_if = "Option::is_none")]
        conductor_endpoint: Option<String>,
    }

    /// Verify the wire shape serialises with camelCase keys, trustMode is
    /// "peer-conductor", conductorEndpoint is present, and doorwayId is absent.
    #[test]
    fn auth_me_storage_me_response_serializes_correctly() {
        let resp = StorageMeResponse {
            human_id: "matthew".into(),
            agent_pub_key: "uhCAk_test".into(),
            identifier: "matthew@local".into(),
            permission_level: "standard".into(),
            doorway_id: None,
            doorway_url: None,
            authenticated: true,
            trust_mode: "peer-conductor".into(),
            authority: AuthorityRef {
                label: "your conductor on this device".into(),
                id: Some("12D3KooWTest".into()),
            },
            conductor_endpoint: Some("your conductor on this device".into()),
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["trustMode"], "peer-conductor");
        assert_eq!(json["authenticated"], true);
        assert_eq!(json["authority"]["label"], "your conductor on this device");
        assert_eq!(json["authority"]["id"], "12D3KooWTest");
        assert_eq!(
            json["conductorEndpoint"], "your conductor on this device",
            "conductorEndpoint must be present"
        );
        // doorwayId and doorwayUrl must be absent (skip_serializing_if = None)
        assert!(
            json.get("doorwayId").is_none(),
            "doorwayId must be omitted when None"
        );
        assert!(
            json.get("doorwayUrl").is_none(),
            "doorwayUrl must be omitted when None"
        );
        // Verify camelCase keys
        assert!(json.get("humanId").is_some(), "humanId must be camelCase");
        assert!(
            json.get("agentPubKey").is_some(),
            "agentPubKey must be camelCase"
        );
        assert!(
            json.get("permissionLevel").is_some(),
            "permissionLevel must be camelCase"
        );
    }

    /// Verify that build_manifest() includes the /auth/me route.
    /// This guards against accidental removal from the manifest.
    #[test]
    fn build_manifest_includes_auth_me_route() {
        let manifest = build_manifest();
        let auth_me = manifest
            .routes
            .iter()
            .find(|r| r.path == "/auth/me" && r.method == doorway_client::HttpMethod::Get);
        assert!(
            auth_me.is_some(),
            "GET /auth/me is missing from build_manifest — portal bundle \
             Transport β path would be broken (Task A4)"
        );
        // Must NOT be cached — session state
        let route = auth_me.unwrap();
        assert_eq!(
            route.cache_ttl_secs, 0,
            "/auth/me must have cache_ttl(0) — session data must never be cached"
        );
        // Must NOT require auth at the route level — handler returns 401 itself
        assert!(
            !route.auth_required,
            "/auth/me must not set auth_required at route level; \
             the handler returns 401 itself to match doorway's contract"
        );
    }

    /// Verify build_manifest() declares /api/v1/status/projector so doorway's
    /// RouteRegistry learns it is proxiable (the path has an explicit storage
    /// handler arm but was absent from the manifest → doorway 404 — wf1 fix).
    #[test]
    fn build_manifest_includes_projector_status_route() {
        let manifest = build_manifest();
        let found = manifest.routes.iter().any(|r| {
            r.method == doorway_client::HttpMethod::Get && r.path == "/api/v1/status/projector"
        });
        assert!(
            found,
            "GET /api/v1/status/projector missing from build_manifest — doorway \
             cannot proxy projector lag/caughtUp to the stability surface"
        );
    }
}

// =============================================================================
// /session/exchange tests — GAP-2b (steward-side portal session exchange)
//
// Drive the allowlist / redeem / create / cookie stages via the injectable
// `exchange_session_with_redeem` seam — no live doorway required. Plus the
// /auth/me cookie-vs-active-session selection. These need a DB pool, so they
// live in their own cfg(test) module under the default (non-iroh) feature set.
// =============================================================================
#[cfg(test)]
mod session_exchange_tests {
    use super::*;
    use crate::db::local_sessions::CreateLocalSessionInput;
    use crate::db::models::LocalSession;
    use crate::services::session_exchange::{RedeemOutcome, RedeemSnapshot};
    use crate::test_util::test_pool;
    use http_body_util::BodyExt;

    async fn test_server() -> HttpServer {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(test_pool())
    }

    fn seed_session(pool: &DbPool, human_id: &str, doorway_url: &str) -> LocalSession {
        let mut conn = pool.get().unwrap();
        db::local_sessions::create_session(
            &mut conn,
            CreateLocalSessionInput {
                id: None,
                human_id: human_id.to_string(),
                agent_pub_key: format!("key-{human_id}"),
                doorway_url: doorway_url.to_string(),
                doorway_id: None,
                identifier: format!("{human_id}@alpha.elohim.host"),
                display_name: None,
                profile_image_hash: None,
                bootstrap_url: None,
            },
        )
        .unwrap()
    }

    fn snapshot() -> RedeemSnapshot {
        RedeemSnapshot {
            human_id: "matthew".into(),
            agent_pub_key: "uhCAk_matthew".into(),
            identifier: "matthew@alpha.elohim.host".into(),
            display_name: Some("Matthew".into()),
            doorway_id: Some("doorway-alpha".into()),
            doorway_url: Some("https://alpha.elohim.host".into()),
            permission_level: Some("standard".into()),
        }
    }

    fn status_of(resp: &Response<Full<Bytes>>) -> StatusCode {
        resp.status()
    }

    async fn body_json(resp: Response<Full<Bytes>>) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    // (a) Unknown doorwayUrl → 403, redeem never invoked, no session created.
    #[tokio::test]
    async fn exchange_unknown_doorway_returns_403_and_never_redeems() {
        let server = test_server().await;
        let pool = server.db_pool.clone().unwrap();
        // History only knows alpha; the request names a different origin.
        seed_session(&pool, "matthew", "https://alpha.elohim.host");

        let input = ExchangeSessionTokenInputView {
            session_token: "tok".into(),
            doorway_url: "https://evil.example.com".into(),
        };

        let redeem_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = redeem_called.clone();
        let resp = server
            .exchange_session_with_redeem(pool.clone(), input, move |_url, _tok| {
                let flag = flag.clone();
                Box::pin(async move {
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    RedeemOutcome::Ok(Box::new(snapshot()))
                })
            })
            .await
            .unwrap();

        assert_eq!(status_of(&resp), StatusCode::FORBIDDEN);
        assert!(
            !redeem_called.load(std::sync::atomic::Ordering::SeqCst),
            "redeem must NOT run for a non-allowlisted issuer (anti-spoofing)"
        );
        // No new session created beyond the seeded one.
        let mut conn = pool.get().unwrap();
        assert_eq!(db::local_sessions::session_count(&mut conn).unwrap(), 1);
    }

    // (b) Allowlisted issuer + redeem 401 → 401, no session created.
    #[tokio::test]
    async fn exchange_allowlisted_but_invalid_token_returns_401_no_session() {
        let server = test_server().await;
        let pool = server.db_pool.clone().unwrap();
        seed_session(&pool, "matthew", "https://alpha.elohim.host");

        let input = ExchangeSessionTokenInputView {
            session_token: "expired".into(),
            // Different path on the SAME origin must still allowlist-match.
            doorway_url: "https://alpha.elohim.host/auth/handoff".into(),
        };

        let resp = server
            .exchange_session_with_redeem(pool.clone(), input, |_url, _tok| {
                Box::pin(async move { RedeemOutcome::Invalid })
            })
            .await
            .unwrap();

        assert_eq!(status_of(&resp), StatusCode::UNAUTHORIZED);
        let json = body_json(resp).await;
        assert_eq!(json["error"], "invalid_token");

        // Still only the seeded session — the exchange created nothing.
        let mut conn = pool.get().unwrap();
        assert_eq!(db::local_sessions::session_count(&mut conn).unwrap(), 1);
    }

    // Upstream/network error on an allowlisted issuer → 502.
    #[tokio::test]
    async fn exchange_allowlisted_but_upstream_error_returns_502() {
        let server = test_server().await;
        let pool = server.db_pool.clone().unwrap();
        seed_session(&pool, "matthew", "https://alpha.elohim.host");

        let input = ExchangeSessionTokenInputView {
            session_token: "tok".into(),
            doorway_url: "https://alpha.elohim.host".into(),
        };

        let resp = server
            .exchange_session_with_redeem(pool.clone(), input, |_url, _tok| {
                Box::pin(async move { RedeemOutcome::Upstream })
            })
            .await
            .unwrap();

        assert_eq!(status_of(&resp), StatusCode::BAD_GATEWAY);
        let mut conn = pool.get().unwrap();
        assert_eq!(db::local_sessions::session_count(&mut conn).unwrap(), 1);
    }

    // (c) Allowlisted + redeem 200 → 201, LocalSession row created with snapshot
    //     fields + Set-Cookie header present (HttpOnly, SameSite=Lax, Path=/).
    #[tokio::test]
    async fn exchange_success_creates_session_with_cookie() {
        let server = test_server().await;
        let pool = server.db_pool.clone().unwrap();
        let seeded = seed_session(&pool, "matthew", "https://alpha.elohim.host");

        let input = ExchangeSessionTokenInputView {
            session_token: "good-token".into(),
            doorway_url: "https://alpha.elohim.host".into(),
        };

        let resp = server
            .exchange_session_with_redeem(pool.clone(), input, |_url, _tok| {
                Box::pin(async move { RedeemOutcome::Ok(Box::new(snapshot())) })
            })
            .await
            .unwrap();

        assert_eq!(status_of(&resp), StatusCode::CREATED);

        // Set-Cookie present with the security attributes the contract requires.
        let set_cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .expect("Set-Cookie must be present")
            .to_str()
            .unwrap()
            .to_string();
        assert!(set_cookie.starts_with("elohim_session="));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Lax"));
        assert!(set_cookie.contains("Path=/"));

        let json = body_json(resp).await;
        // Snapshot fields projected through LocalSessionView (camelCase).
        assert_eq!(json["humanId"], "matthew");
        assert_eq!(json["agentPubKey"], "uhCAk_matthew");
        assert_eq!(json["identifier"], "matthew@alpha.elohim.host");
        // Session stamped with the validated doorway URL.
        assert_eq!(json["doorwayUrl"], "https://alpha.elohim.host");
        assert_eq!(json["isActive"], true);
        let new_id = json["id"].as_str().unwrap().to_string();

        // A fresh row exists and the prior seeded session was deactivated.
        let mut conn = pool.get().unwrap();
        assert_eq!(db::local_sessions::session_count(&mut conn).unwrap(), 2);
        let prior = db::local_sessions::get_session_by_id(&mut conn, &seeded.id)
            .unwrap()
            .unwrap();
        assert_eq!(prior.is_active, 0, "prior session must be deactivated");
        let created = db::local_sessions::get_session_by_id(&mut conn, &new_id)
            .unwrap()
            .unwrap();
        assert_eq!(created.agent_pub_key, "uhCAk_matthew");
        assert_eq!(created.is_active, 1);

        // The Set-Cookie session id must name the created row.
        assert!(set_cookie.contains(&new_id));
    }

    // (d) /auth/me WITH an elohim_session cookie naming a valid, active session
    //     → that session is projected (not necessarily the single active one).
    #[tokio::test]
    async fn auth_me_with_cookie_projects_named_session() {
        let server = test_server().await;
        let pool = server.db_pool.clone().unwrap();
        // Two sessions: creating the second deactivates the first. We then name
        // the FIRST (deactivated) — confirming cookie selection is honored only
        // for active sessions, and naming the active one projects it.
        let first = seed_session(&pool, "susan", "https://alpha.elohim.host");
        let second = seed_session(&pool, "matthew", "https://alpha.elohim.host");

        // Cookie names the active (second) session.
        let resp = server
            .handle_auth_me(pool.clone(), Some(second.id.clone()))
            .await
            .unwrap();
        assert_eq!(status_of(&resp), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["humanId"], "matthew");

        // Cookie naming the DEACTIVATED first session must be ignored — falls
        // back to the active session (matthew), never projecting a dead session.
        let resp2 = server
            .handle_auth_me(pool.clone(), Some(first.id.clone()))
            .await
            .unwrap();
        assert_eq!(status_of(&resp2), StatusCode::OK);
        let json2 = body_json(resp2).await;
        assert_eq!(
            json2["humanId"], "matthew",
            "cookie naming a deactivated session must fall back to the active one"
        );
    }

    // /auth/me with a cookie naming a NON-EXISTENT session → fall back to active.
    #[tokio::test]
    async fn auth_me_with_unknown_cookie_falls_back_to_active() {
        let server = test_server().await;
        let pool = server.db_pool.clone().unwrap();
        seed_session(&pool, "matthew", "https://alpha.elohim.host");

        let resp = server
            .handle_auth_me(pool.clone(), Some("no-such-session".into()))
            .await
            .unwrap();
        assert_eq!(status_of(&resp), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["humanId"], "matthew");
    }

    // (e) /auth/me WITHOUT a cookie → existing single-active-session behavior.
    #[tokio::test]
    async fn auth_me_without_cookie_uses_active_session() {
        let server = test_server().await;
        let pool = server.db_pool.clone().unwrap();
        seed_session(&pool, "matthew", "https://alpha.elohim.host");

        let resp = server.handle_auth_me(pool.clone(), None).await.unwrap();
        assert_eq!(status_of(&resp), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["humanId"], "matthew");
        assert_eq!(json["trustMode"], "peer-conductor");
    }

    // /auth/me with no sessions at all and no cookie → 401 (unchanged contract).
    #[tokio::test]
    async fn auth_me_no_session_returns_401() {
        let server = test_server().await;
        let pool = server.db_pool.clone().unwrap();
        let resp = server.handle_auth_me(pool.clone(), None).await.unwrap();
        assert_eq!(status_of(&resp), StatusCode::UNAUTHORIZED);
    }

    // Cookie-extraction helper unit coverage.
    #[test]
    fn extract_session_cookie_parses_pairs() {
        let mut headers = hyper::HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "foo=bar; elohim_session=abc123; baz=qux".parse().unwrap(),
        );
        assert_eq!(extract_session_cookie(&headers), Some("abc123".to_string()));

        let mut none_headers = hyper::HeaderMap::new();
        none_headers.insert(header::COOKIE, "foo=bar; baz=qux".parse().unwrap());
        assert_eq!(extract_session_cookie(&none_headers), None);

        let empty_headers = hyper::HeaderMap::new();
        assert_eq!(extract_session_cookie(&empty_headers), None);
    }

    // /session/exchange must NOT be in build_manifest — it is a steward-local
    // browser surface at the portal-host origin, never doorway-proxied.
    #[test]
    fn build_manifest_excludes_session_exchange() {
        let manifest = build_manifest();
        assert!(
            !manifest
                .routes
                .iter()
                .any(|r| r.path == "/session/exchange"),
            "/session/exchange must NOT be declared in build_manifest() — it is \
             reached at the portal-host origin, never doorway-proxied"
        );
    }

    // Slice-2b T10: un-pin flips the pin to removed and preserves the
    // commitment_cid back-reference (the revoke author targets it but does not
    // erase the audit link). No hc_registry is wired on test_server(), so the
    // conductor author hook is skipped — the local projection effects are what
    // this test asserts (the conductor round-trip is proven by the T1 sweettest).
    #[tokio::test]
    async fn remove_pin_marks_removed_and_preserves_offer_backref() {
        let server = test_server().await;

        let created = server
            .test_handle_create_pin_raw(r#"{"headRef":"epr:album-1","kind":"item"}"#)
            .await;
        assert_eq!(created.status, 201, "pin create must succeed (201 Created)");

        // Back-fill a commitment_cid (simulating the provide reconciler having
        // authored a replicates-commons offer for this pin).
        let pin_id: i32 = {
            let mut conn = server.get_diesel_conn().expect("conn");
            let all = db::acquisition_pins::list_all_pins(&mut conn).expect("list");
            db::acquisition_pins::set_commitment_cid(&mut conn, all[0].id, "anchor:offer-1")
                .expect("backfill");
            all[0].id
        };

        let removed = server.test_handle_remove_pin(&pin_id.to_string()).await;
        assert_eq!(removed.status, 200, "remove must succeed");

        let mut conn = server.get_diesel_conn().expect("conn");
        let all = db::acquisition_pins::list_all_pins(&mut conn).expect("list");
        assert_eq!(all[0].status, "removed", "pin must be flipped to removed");
        // The back-reference is preserved — the revocation targets it, but the
        // audit link to which commitment was revoked is kept.
        assert_eq!(all[0].commitment_cid.as_deref(), Some("anchor:offer-1"));
    }

    // Removing a non-existent pin returns 404 (no author call possible).
    #[tokio::test]
    async fn remove_unknown_pin_returns_404() {
        let server = test_server().await;
        let removed = server.test_handle_remove_pin("99999").await;
        assert_eq!(removed.status, 404, "removing an unknown pin must 404");
    }

    // The write path strips `epr:` (the reconcile loop joins on the bare content
    // id); the read path must re-surface it so a pin POSTed for
    // "epr:strawberry-guide" is listed under the same headRef.
    #[tokio::test]
    async fn pin_epr_prefix_round_trips_through_list() {
        let server = test_server().await;
        let created = server
            .test_handle_create_pin_raw(r#"{"headRef":"epr:strawberry-guide","kind":"item"}"#)
            .await;
        assert_eq!(created.status, 201, "pin create must succeed");

        let listed = server.test_handle_list_pins().await;
        assert_eq!(listed.status, 200);
        let body: serde_json::Value = serde_json::from_slice(&listed.body).expect("json");
        let pins = body["pins"].as_array().expect("pins array");
        let active: Vec<_> = pins
            .iter()
            .filter(|p| p["status"] == "active" && p["headRef"] == "epr:strawberry-guide")
            .collect();
        assert_eq!(
            active.len(),
            1,
            "exactly one active pin must list under the prefixed headRef; got {body}"
        );
    }

    // A provide pin from a browser-only context cannot serve bytes to peers and
    // must be refused with 400 mentioning "peer".
    #[tokio::test]
    async fn provide_pin_on_browser_context_is_refused() {
        let server = test_server().await;
        let resp = server
            .test_handle_create_pin_raw(
                r#"{"headRef":"strawberry-guide","kind":"item","provide":true,"context":"browser"}"#,
            )
            .await;
        assert_eq!(
            resp.status, 400,
            "browser-context provide pin must be refused"
        );
        let body = String::from_utf8_lossy(&resp.body);
        assert!(
            body.contains("peer"),
            "rejection body must mention 'peer'; got: {body}"
        );
    }

    // A provide pin without a browser context is accepted (peer-capable node).
    #[tokio::test]
    async fn provide_pin_without_browser_context_is_accepted() {
        let server = test_server().await;
        let resp = server
            .test_handle_create_pin_raw(
                r#"{"headRef":"strawberry-guide","kind":"item","provide":true}"#,
            )
            .await;
        assert_eq!(
            resp.status, 201,
            "provide pin on a peer-capable node must succeed"
        );
    }
}

// =============================================================================
// POST /db/content/{id}/head — authority-refusal-precedes-catching-up-shed
// ordering (2026-08-09 bug fix).
//
// Measured live on edge #1327's Dataplane Validation: a non-author's move of a
// declared HEAD, probed while the write-admission pool was exhausted
// ("catching-up"), returned 503 {"status":"catching-up"} instead of a 401/403
// authority refusal — the shed raced ahead of `handle_content_head_post_bytes`'s
// (a)/(a2) auth-first checks and won. These tests pin the fixed ordering: the
// authority refusal (401 missing credential, 403 unresolvable/wrong identity)
// is now reachable and correct EVEN with the write-admission pool fully
// drained, because `is_head_declare_write` (above `admission_is_read_method`)
// carves this route out of the blanket top-of-dispatch shed in `handle_request`,
// and the write-admission check moves INSIDE `handle_content_head_post_bytes`,
// after authorship is confirmed (see its "(d2)" comment). The corresponding
// a2o assertion (never edited by this fix, per the shift's oracle rule) is
// `genesis/a2o/steps/dataplane.steps.ts` — "the HEAD authority surface refuses
// a non-author move ..." — which backs `@concern:notary-authority`
// (`genesis/a2o/features/dataplane/notary-authority.feature`).
// =============================================================================
#[cfg(test)]
mod head_declare_authority_precedes_catching_up_shed_tests {
    use super::*;
    use crate::test_util::test_pool;

    async fn test_server() -> HttpServer {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(test_pool())
    }

    /// Drain the write-admission pool to zero permits and hold them for the
    /// caller's scope — models "peer is in the catching-up shed state" (write
    /// pool genuinely exhausted, e.g. post-restart replication churn).
    fn drain_write_pool(server: &HttpServer) -> Vec<tokio::sync::OwnedSemaphorePermit> {
        let mut held = Vec::new();
        while let Ok(permit) = server.request_semaphore.clone().try_acquire_owned() {
            held.push(permit);
        }
        held
    }

    /// THE regression this fix closes: an unauthenticated (no `X-Agent-Cid`
    /// header) HEAD-declare move, probed while the write-admission pool is
    /// fully exhausted, must still be refused 401 — never masked by the
    /// catching-up 503. Exercises the exact request shape the a2o step sends
    /// (`postRaw` with no auth headers).
    #[tokio::test]
    async fn non_author_missing_credential_refused_401_even_when_write_pool_exhausted() {
        let server = test_server().await;
        let _held = drain_write_pool(&server);
        assert_eq!(
            server.request_semaphore.available_permits(),
            0,
            "precondition: write-admission pool must be fully exhausted"
        );

        let ctx = AppContext::default_lamad();
        let headers = hyper::HeaderMap::new(); // no X-Agent-Cid
        let resp = server
            .handle_content_head_post_bytes(
                &headers,
                b"",
                "e2e-notary-authority-nonauthor-probe",
                &ctx,
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "a missing credential must be refused 401 regardless of write-pool pressure — \
             a catching-up 503 here would let a non-author win the race by exhausting the pool"
        );
    }

    /// A credential that cannot be resolved to an agent key (unknown human
    /// slug) is a 403 — also refused before the write-admission shed, for the
    /// same reason: authority refusal must never be masked by backpressure.
    #[tokio::test]
    async fn non_author_unresolvable_identity_refused_403_even_when_write_pool_exhausted() {
        let server = test_server().await;
        let _held = drain_write_pool(&server);
        assert_eq!(server.request_semaphore.available_permits(), 0);

        let ctx = AppContext::default_lamad();
        let mut headers = hyper::HeaderMap::new();
        headers.insert(
            "X-Agent-Cid",
            hyper::header::HeaderValue::from_static("human-does-not-exist"),
        );
        let resp = server
            .handle_content_head_post_bytes(
                &headers,
                b"",
                "e2e-notary-authority-nonauthor-probe",
                &ctx,
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "an unresolvable caller identity must be refused 403 regardless of write-pool pressure"
        );
    }

    /// Baseline (pool NOT exhausted): unchanged behavior — same 401 for a
    /// missing credential, proving the fix didn't alter the happy-path status.
    #[tokio::test]
    async fn non_author_missing_credential_refused_401_when_pool_healthy() {
        let server = test_server().await;
        assert!(server.request_semaphore.available_permits() > 0);

        let ctx = AppContext::default_lamad();
        let headers = hyper::HeaderMap::new();
        let resp = server
            .handle_content_head_post_bytes(
                &headers,
                b"",
                "e2e-notary-authority-nonauthor-probe",
                &ctx,
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    /// Dispatch-level companion to the two tests above: the predicate that
    /// exempts this route from the blanket admission gate in `handle_request`
    /// must match the exact path shape the a2o probe hits.
    #[test]
    fn head_declare_route_is_admission_exempt_at_dispatch() {
        assert!(is_head_declare_write(
            &Method::POST,
            "/db/content/e2e-notary-authority-nonauthor-probe/head"
        ));
    }
}

// =============================================================================
// GET /db/content/{id}/head-record — HTTP-path reach gate tests.
//
// The HTTP route is a pre-existing twin of the P2P responder gated in
// `p2p::view_federation::build_content_head_record_payload` (2175f2b60): both
// read the SAME projection row, so a scoped-tier id must be just as
// unprobeable over this HTTP surface as it is over the P2P one. These tests
// pin (a) a scoped-tier id answers byte-identically to an unknown id, and (b)
// the reach gate does NOT regress the distribution-safe path that
// scripts/ci/stage-spa-blob.sh depends on for elohim-host-landing (community/
// public reach) — that id must still clear the gate and reach the
// conductor-bridge step (which 503s here for lack of a wired hc_registry,
// never a 404 "not found").
// =============================================================================
#[cfg(test)]
mod content_head_record_reach_gate_tests {
    use super::*;
    use crate::db::content_diesel::{create_content, stamp_declared_head, CreateContentInput};
    use crate::test_util::test_pool;
    use http_body_util::BodyExt;

    async fn test_server() -> HttpServer {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_db_pool(test_pool())
    }

    fn seed(pool: &DbPool, id: &str, reach: &str) {
        let ctx = AppContext::default_lamad();
        let mut conn = pool.get().unwrap();
        create_content(
            &mut conn,
            &ctx,
            CreateContentInput {
                id: id.to_string(),
                title: id.to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: reach.to_string(),
                created_by: None,
                tags: vec![],
                content_body: None,
                dht_anchor_hash: Some(format!("anc-{id}")),
            },
        )
        .unwrap();
        stamp_declared_head(&mut conn, &ctx, id, &format!("uhCkk-{id}"), None, None).unwrap();
    }

    async fn status_and_body(resp: Response<Full<Bytes>>) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    /// A scoped-tier (private) id must respond byte-identically to a truly
    /// unknown id — no distinguishable refusal that would let a caller probe
    /// for the existence of a scoped id via this route. Uses the SAME id
    /// string across two independent DB states (one where it exists as a
    /// private-reach row, one where it was never created) so the comparison
    /// is a genuine byte-for-byte match, not merely "same template".
    #[tokio::test]
    async fn scoped_tier_reach_answers_identically_to_unknown_id() {
        const PROBE_ID: &str = "hr-http:probe";
        let ctx = AppContext::default_lamad();

        // State A: the id exists, but is scoped (private) reach.
        let scoped_server = test_server().await;
        seed(&scoped_server.db_pool.clone().unwrap(), PROBE_ID, "private");
        let scoped = scoped_server
            .handle_content_head_record(Method::GET, PROBE_ID, &ctx)
            .await
            .unwrap();
        let (scoped_status, scoped_body) = status_and_body(scoped).await;

        // State B: the id was never created at all.
        let unknown_server = test_server().await;
        let unknown = unknown_server
            .handle_content_head_record(Method::GET, PROBE_ID, &ctx)
            .await
            .unwrap();
        let (unknown_status, unknown_body) = status_and_body(unknown).await;

        assert_eq!(scoped_status, StatusCode::NOT_FOUND);
        assert_eq!(scoped_status, unknown_status);
        assert_eq!(
            scoped_body, unknown_body,
            "a scoped-tier id's refusal must be byte-identical to an unknown id \
             for the SAME id string — existence must not be probeable"
        );
    }

    /// A distribution-safe (community/public) reach id must still clear the
    /// gate — this is the path `scripts/ci/stage-spa-blob.sh` depends on for
    /// elohim-host-landing. No hc_registry is wired in this test server, so a
    /// gate-cleared request reaches the conductor-bridge step and 503s there
    /// (service_unavailable), never the 404 "Content not found" shape a
    /// gated/unknown id gets.
    #[tokio::test]
    async fn distribution_safe_reach_still_clears_the_gate() {
        let server = test_server().await;
        let pool = server.db_pool.clone().unwrap();
        let ctx = AppContext::default_lamad();

        for (id, reach) in [
            ("hr-http:community", "community"),
            ("hr-http:public", "public"),
        ] {
            seed(&pool, id, reach);
            let resp = server
                .handle_content_head_record(Method::GET, id, &ctx)
                .await
                .unwrap();
            let (status, body) = status_and_body(resp).await;
            assert_eq!(
                status,
                StatusCode::SERVICE_UNAVAILABLE,
                "reach={reach} must clear the gate and reach the conductor-bridge \
                 step, not be turned away as not-found: {body:?}"
            );
        }
    }
}

#[cfg(test)]
mod admission_tests {
    use tokio::sync::Semaphore;

    #[test]
    fn try_acquire_sheds_when_exhausted() {
        // Models the per-request gate (Task 7): a 0-permit semaphore fails
        // try_acquire immediately (shed), never blocking like acquire().await.
        let sem = Semaphore::new(0);
        assert!(sem.try_acquire().is_err(), "exhausted => shed, not queue");
        let sem2 = Semaphore::new(1);
        assert!(sem2.try_acquire().is_ok(), "available => admit");
    }

    #[test]
    fn storage_health_version_never_shed_at_zero_permits() {
        use hyper::Method;
        let sem = Semaphore::new(0);
        let exempt = |method: &Method, path: &str| {
            matches!(method, &Method::OPTIONS) || matches!(path, "/health" | "/version")
        };
        for p in ["/health", "/version"] {
            assert!(exempt(&Method::GET, p), "{p} exempt");
        }
        // gated path sheds
        assert!(!exempt(&Method::GET, "/db/content/x"));
        assert!(sem.try_acquire().is_err(), "0 permits => gated path sheds");
    }

    #[test]
    fn reads_draw_from_read_pool_writes_from_write_pool() {
        use hyper::Method;
        // GET/HEAD are reads (larger pool); mutations are writes (tighter pool).
        assert!(super::admission_is_read_method(&Method::GET));
        assert!(super::admission_is_read_method(&Method::HEAD));
        for m in [Method::POST, Method::PATCH, Method::PUT, Method::DELETE] {
            assert!(!super::admission_is_read_method(&m), "{m} is a write");
        }
    }

    #[test]
    fn write_burst_cannot_starve_reads_independent_pools() {
        // The core invariant of the split: a write pool exhausted by a seed
        // burst leaves the read pool fully available — a read never sheds for a
        // write's sins (genesis #1272 `/epr/*` read-path shed).
        let write_pool = Semaphore::new(0); // exhausted by a write storm
        let read_pool = Semaphore::new(super::MAX_CONCURRENT_READS);
        assert!(write_pool.try_acquire().is_err(), "writes shed under burst");
        assert!(read_pool.try_acquire().is_ok(), "reads still admitted");
    }
}

// =============================================================================
// C3 read side — serve-path prefers the notary-DECLARED head blob (Plan C3 /
// notary-authority). Exercises `declared_head_served_blob` across the three
// cases the spine names: declared head present+distinct+LOCAL → serve head;
// no declared head → own row; declared head present but blob NOT local → own
// row (degrade; never fan out — the replication plane heals absence).
// =============================================================================
#[cfg(test)]
mod conductor_diagnostics_tests {
    use super::project_agent_info;

    #[test]
    fn p2p_namespace_survives_app_context_extraction() {
        // Regression (edge #1172): "p2p" absent from legacy_prefixes made
        // extract_app_context eat it as an h_app_id (resource_path became
        // "conductor-diagnostics"), turning the dispatch arm into dead code
        // and 404ing the deployed route.
        let (_ctx, resource_path) =
            super::HttpServer::extract_app_context("p2p/conductor-diagnostics");
        assert_eq!(resource_path, "p2p/conductor-diagnostics");
    }

    #[test]
    fn identity_did_namespace_survives_app_context_extraction() {
        // Same shadow class as the p2p regression above: "identity" must be a
        // legacy prefix so extract_app_context keeps the whole
        // "identity/did/{did}" as resource_path (the DID's colons are NOT a
        // resource separator), else the resolution dispatch arm is dead code.
        let (_ctx, resource_path) =
            super::HttpServer::extract_app_context("identity/did/did:elohim:uhCAkAGENTKEYEXAMPLE");
        assert_eq!(
            resource_path,
            "identity/did/did:elohim:uhCAkAGENTKEYEXAMPLE"
        );
    }

    #[test]
    fn build_manifest_includes_resolve_did_route() {
        let manifest = super::build_manifest();
        let has_route = manifest.routes.iter().any(|r| {
            r.path == "/db/identity/did/{did}" && r.method == doorway_client::HttpMethod::Get
        });
        assert!(
            has_route,
            "GET /db/identity/did/{{did}} missing from build_manifest — doorway \
             will not auto-forward did:elohim resolution"
        );
    }

    #[test]
    fn projects_string_wrapped_agent_info() {
        let inner = r#"{"agent":"AgEnT","space":"SpAcE","createdAt":"1752200000000000","expiresAt":"1752200300000000","isTombstone":false,"url":"wss://signal.example/xyz","storageArc":[0,4294967295]}"#;
        let signed = serde_json::json!({ "agentInfo": inner, "signature": "c2ln" }).to_string();
        let v = project_agent_info(&signed);
        assert_eq!(v["agent"], "AgEnT");
        assert_eq!(v["url"], "wss://signal.example/xyz");
        assert_eq!(v["isTombstone"], false);
        assert_eq!(v["storageArc"][1], 4294967295u32 as u64);
        assert!(v.get("raw").is_none());
    }

    #[test]
    fn projects_inline_object_encoding() {
        // Fallback: some encodings inline the object instead of string-wrapping.
        let signed = r#"{"agent":"A","space":"S","url":null,"createdAt":"1","expiresAt":"2","isTombstone":true}"#;
        let v = project_agent_info(signed);
        assert_eq!(v["agent"], "A");
        assert_eq!(v["url"], serde_json::Value::Null);
        assert_eq!(v["isTombstone"], true);
    }

    #[test]
    fn malformed_input_degrades_to_raw_never_panics() {
        let v = project_agent_info("not json at all");
        assert_eq!(v["raw"], "not json at all");
        let v2 = project_agent_info(r#"{"agentInfo":"{broken","signature":"x"}"#);
        assert_eq!(v2["raw"], r#"{"agentInfo":"{broken","signature":"x"}"#);
    }
}

#[cfg(test)]
mod c3_serve_head_preference_tests {
    use super::*;
    use crate::db::models::Content;
    use crate::sync::{DocStore, DocStoreConfig, StreamTracker, SyncManager};

    fn content_row(id: &str, own_blob: Option<&str>, declared: Option<&str>) -> Content {
        Content {
            id: id.to_string(),
            h_app_id: "lamad".to_string(),
            title: id.to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "spa-bundle".to_string(),
            blob_hash: own_blob.map(str::to_string),
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: Some("{}".to_string()),
            reach: "commons".to_string(),
            validation_status: "valid".to_string(),
            created_by: None,
            created_at: "2026-07-10T00:00:00Z".to_string(),
            updated_at: "2026-07-10T00:00:00Z".to_string(),
            content_body: None,
            // A verified declared-head stamp mirrors the action into both fields.
            dht_anchor_hash: declared.map(str::to_string),
            p2p_published_at: None,
            server_blob_hash: None,
            crdt_converged_at: None,
            declared_head_action_hash: declared.map(str::to_string),
            declared_head_at: None,
            canonical_declared_at: None,
            canonical_earned: None,
        }
    }

    /// Build a SyncManager whose doc for `id` projects `head_blob` as the head
    /// version with `declared` as the `headActionHash` hint (the shape the REQ-F4
    /// gate matches against).
    async fn sync_with_head_doc(
        id: &str,
        head_blob: &str,
        declared: &str,
    ) -> (Arc<SyncManager>, tempfile::TempDir) {
        let temp = tempfile::tempdir().unwrap();
        let doc_store = Arc::new(
            DocStore::new(DocStoreConfig {
                db_path: temp.path().join("c3.sled"),
                ..Default::default()
            })
            .await
            .unwrap(),
        );
        let sync = Arc::new(SyncManager::new(doc_store, Arc::new(StreamTracker::new())));
        let head_content = content_row(id, Some(head_blob), Some(declared));
        crate::sync::projector::project_content_doc(sync.as_ref(), &head_content)
            .await
            .unwrap();
        (sync, temp)
    }

    fn well_formed_absent_addr(byte: &str) -> String {
        format!("sha256-{}", byte.repeat(32))
    }

    /// (a) Declared head present, distinct from the own row, and its bytes are
    /// LOCALLY present → the serve path prefers the declared head's blob.
    #[tokio::test]
    async fn serves_declared_head_blob_when_present_and_local() {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let head_hash = blob_store
            .store(b"DECLARED-HEAD-BUNDLE")
            .await
            .unwrap()
            .hash;
        let (sync, _t) = sync_with_head_doc("epr-c3-a", &head_hash, "uhCkkDECLARED").await;
        let server =
            HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_sync_manager(sync);

        // Own row points at a DIFFERENT (distinct) blob than the declared head.
        let row = content_row(
            "epr-c3-a",
            Some(&well_formed_absent_addr("cd")),
            Some("uhCkkDECLARED"),
        );
        let got = server.declared_head_served_blob(&row).await;
        assert_eq!(
            got.as_deref(),
            Some(head_hash.as_str()),
            "declared head blob is distinct + local → serve it over the own row"
        );
    }

    /// (a2) Declared head is a `>16MB` composite: its bytes live only as the
    /// manifest-named shards, never under the composite's own name. A raw
    /// filesystem probe therefore answers "absent" for a blob `/blob/{head}`
    /// serves in full, so the local-presence gate must read through the same
    /// manifest-aware seam the serving path uses — otherwise no oversized
    /// bundle can EVER be served as its notary-declared head.
    #[tokio::test]
    async fn serves_declared_head_blob_when_only_its_shards_are_local() {
        let dir = tempfile::tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path()).await.unwrap());

        // The shards are ordinary content-addressed blobs; the composite is not
        // stored under its own name — exactly what `put_blob_bytes` leaves.
        let shard_a = blob_store.store(b"HEAD-SHARD-A").await.unwrap().hash;
        let shard_b = blob_store.store(b"HEAD-SHARD-B").await.unwrap().hash;
        let head_hash = well_formed_absent_addr("1a");

        let pool = crate::test_util::test_pool();
        {
            let mut conn = pool.get().unwrap();
            let manifest = ShardManifest {
                blob_cid: String::new(),
                blob_hash: head_hash.clone(),
                total_size: 24,
                mime_type: "application/zip".to_string(),
                encoding: "chunked".to_string(),
                data_shards: 2,
                total_shards: 2,
                shard_size: 12,
                shard_hashes: vec![shard_a, shard_b],
                reach: "commons".to_string(),
                author_id: None,
                created_at: "2026-08-16T00:00:00Z".to_string(),
                verified_at: None,
            };
            crate::db::shard_manifests::record_generated_manifest(
                &mut conn,
                &format!("blob:{head_hash}"),
                "lamad",
                &manifest,
            )
            .expect("persist declared-head manifest");
        }

        let (sync, _t) = sync_with_head_doc("epr-c3-a2", &head_hash, "uhCkkDECLARED").await;
        let server = HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap())
            .with_sync_manager(sync)
            .with_db_pool(pool);

        let row = content_row(
            "epr-c3-a2",
            Some(&well_formed_absent_addr("cd")),
            Some("uhCkkDECLARED"),
        );
        assert_eq!(
            server.declared_head_served_blob(&row).await.as_deref(),
            Some(head_hash.as_str()),
            "a sharded declared head whose shards are ALL local counts as locally \
             present — the composite is never a file, so a raw exists() probe lies"
        );
    }

    /// (b) No declared head on the row → nothing to prefer; degrade to own row.
    #[tokio::test]
    async fn degrades_to_own_when_no_declared_head() {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let (sync, _t) =
            sync_with_head_doc("epr-c3-b", &well_formed_absent_addr("ef"), "uhCkkDECLARED").await;
        let server =
            HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_sync_manager(sync);

        let row = content_row("epr-c3-b", Some(&well_formed_absent_addr("cd")), None);
        assert_eq!(
            server.declared_head_served_blob(&row).await,
            None,
            "no declared head → serve the own row"
        );
    }

    /// (c) Declared head present but its bytes are NOT locally held → degrade to
    /// own row (never fan out; the replication plane heals absence).
    #[tokio::test]
    async fn degrades_to_own_when_declared_head_blob_absent_locally() {
        let blob_store = Arc::new(
            BlobStore::new(tempfile::tempdir().unwrap().path().to_path_buf())
                .await
                .unwrap(),
        );
        let absent_head = well_formed_absent_addr("ab");
        let (sync, _t) = sync_with_head_doc("epr-c3-c", &absent_head, "uhCkkDECLARED").await;
        let server =
            HttpServer::new(blob_store, "127.0.0.1:0".parse().unwrap()).with_sync_manager(sync);

        let row = content_row(
            "epr-c3-c",
            Some(&well_formed_absent_addr("cd")),
            Some("uhCkkDECLARED"),
        );
        assert_eq!(
            server.declared_head_served_blob(&row).await,
            None,
            "declared head blob absent locally → degrade to own row, do NOT fan out"
        );
    }
}

#[cfg(test)]
mod admission_egress_tests {
    use super::*;
    use crate::conductor_admission::ADMISSION_SHED_MARKER;

    /// The shape `ConductorAdmission::shed_error` produces.
    fn shed() -> StorageError {
        StorageError::Timeout(format!(
            "{ADMISSION_SHED_MARKER}: no conductor permit for imagodei within 100ms \
             (class=interactive, capacity=4, in_flight=4) — nothing was dispatched"
        ))
    }

    /// The last seam before the wire. Every error that escapes a handler lands
    /// here, and until 2026-08-18 it collapsed ALL of them to a plain-text 500 —
    /// so a conductor-admission shed (the conductor never saw the call) was
    /// indistinguishable from a conductor that tried and broke. Measured live
    /// against a local island conductor at capacity=1: 4,538 sheds counted at
    /// the gate, 4,538 bare 500s on the wire, zero Retry-After.
    #[test]
    fn an_escaped_admission_shed_reaches_the_wire_as_backpressure() {
        let resp = HttpServer::escaped_error_response(&shed());
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            resp.headers().get(hyper::header::RETRY_AFTER).unwrap(),
            crate::conductor_admission::SHED_RETRY_AFTER_SECS
                .to_string()
                .as_str(),
            "a shed must tell the caller when to come back"
        );
    }

    /// Everything else keeps the existing sink behaviour — this seam gains one
    /// classification, not a new status-code policy for the whole surface.
    #[test]
    fn an_escaped_ordinary_error_still_reaches_the_wire_as_500() {
        let resp = HttpServer::escaped_error_response(&StorageError::Internal("boom".into()));
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// Both escaped arms must keep `with_cors_headers` parity: without
    /// `Cross-Origin-Resource-Policy: cross-origin` a COEP embedder blocks the
    /// body, so the legible 503-with-Retry-After (or the plain 500) degrades
    /// into an opaque network failure client-side.
    #[test]
    fn escaped_errors_carry_cross_origin_headers_on_both_arms() {
        for resp in [
            HttpServer::escaped_error_response(&shed()),
            HttpServer::escaped_error_response(&StorageError::Internal("boom".into())),
        ] {
            assert_eq!(
                resp.headers().get("Access-Control-Allow-Origin").unwrap(),
                "*"
            );
            assert_eq!(
                resp.headers().get("Cross-Origin-Resource-Policy").unwrap(),
                "cross-origin"
            );
        }
    }
}
