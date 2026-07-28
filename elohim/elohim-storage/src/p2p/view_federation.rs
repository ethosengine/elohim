//! `/elohim/view-federation/1.0.0` — request-response protocol for federated
//! view-slice fetch across household peers.
//!
//! ## Source of Truth
//!
//! This module is **Operational (Category C)** per the p2p-design-gate output
//! in `genesis/docs/superpowers/specs/2026-05-01-light-up-the-topology-design.md`.
//! Wire-protocol messages, not stored. Federation is gated cryptographically
//! by DHT-notarized `AgentPeerBinding`s — verification happens after response
//! receipt by the `Federator` in `services/federator.rs` (lands in F-T21).
//!
//! ## Wire format
//!
//! Length-prefixed (4-byte big-endian `u32`) MessagePack via `rmp_serde` —
//! identical shape to [`super::blob_protocol`]. The cap is
//! [`MAX_PAYLOAD`] (1 MiB) and is enforced both at length-prefix-read time
//! (rejecting an oversized claim before any allocation) and at
//! serialized-frame-write time (rejecting an oversized payload before
//! sending it).
//!
//! ## Size limits
//!
//! 1 MiB comfortably exceeds every legitimate slice the protocol carries today:
//! - `ViewKind::Cluster`: ~2 devices × ~1 KiB metadata each (typical).
//! - `ViewKind::PeerTopology`: a small set of household reciprocation edges.
//! - `ViewKind::ProjectionInventory`: the largest — the content inventory at
//!   [`PROJECTION_INVENTORY_CAP`] (2000) entries serializes to ≈400 KiB.
//!
//! It is also small enough to bound memory from a malicious peer claiming a
//! multi-MB length prefix.

use async_trait::async_trait;
use base64::Engine as _;
use futures::prelude::*;
use libp2p::request_response;
use std::io;

use crate::views::{
    ContentHeadRecordPayload, Freshness, FreshnessState, JsonVal, ProjectionInventoryEntry,
    ProjectionInventoryPayload, ViewFederationRequest, ViewFederationResponse, ViewKind, ViewSlice,
};

/// v1 cap on entries returned in a single `ProjectionInventory` slice. A content
/// entry is `(id, dhtAnchorHash)` — with MessagePack named fields (`id`, up to
/// ~60 chars; the 13-char `dhtAnchorHash` key + a 53-char anchor) it runs
/// ~170–200 bytes/entry, so 2000 entries ≈ 340–400 KiB. That fits under
/// [`MAX_PAYLOAD`] (1 MiB) with room for the slice envelope + base64 signature;
/// [`fit_inventory_to_budget`] is the belt-and-suspenders guard that keeps the
/// serialized frame under the codec bound even if entries grow wider.
/// The `total` field reports the true row count when truncated.
pub const PROJECTION_INVENTORY_CAP: i64 = 2000;

/// The REA-commitment projection table (v1 reconciliation stream). The
/// `ViewKind::ProjectionInventory { table }` discriminator is the seam for
/// `agreements` / `economic_events` later; an unknown table yields an empty
/// inventory (honest: "I hold nothing for a table I don't know").
pub const PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS: &str = "rea_commitments";

/// The CONTENT projection table (notary-authority Leg 4 — cross-peer
/// content-anchor reconcile). Serves the `(id, dhtAnchorHash)` inventory of
/// this peer's anchored, distribution-safe content rows (see
/// `db::content_diesel::list_content_anchor_inventory`) so a peer that missed a
/// `ContentCommitted` signal can discover WHICH ids to re-resolve against its
/// OWN conductor. Discovery-only: the advertised anchor is never written into a
/// consumer's projection.
pub const PROJECTION_INVENTORY_TABLE_CONTENT: &str = "content";

/// The COLLECTIVES projection table (cross-peer collective-identity reconcile).
/// Serves the `(id, collective_cid)` inventory of this peer's DHT-anchored,
/// non-dissolved collectives, so a peer that never saw a `CollectiveCommitted`
/// signal (post_commit fires only on the AUTHORING conductor) can discover WHICH
/// collective cids exist and resolve each against its OWN conductor.
///
/// The `dhtAnchorHash` slot carries the `collective:{action_hash}` CID — that IS
/// the reconciliation identity; the `id` slot carries this peer's local routing
/// alias (see `db::collectives::list_collective_cid_inventory` for why
/// NULL-cid rows are excluded). Discovery-only: the advertised cid is never
/// written into a consumer's projection without an own-conductor read.
pub const PROJECTION_INVENTORY_TABLE_COLLECTIVES: &str = "collectives";

/// Protocol identifier for federated view-slice fetch.
pub const VIEW_FEDERATION_PROTOCOL_ID: &str = "/elohim/view-federation/1.0.0";

/// Hard cap on a single request OR response payload — 1 MiB.
///
/// This bound is a **pre-allocation DoS guard** (a malicious peer must not be
/// able to make us allocate a multi-MB body buffer from a length prefix alone),
/// so it MUST comfortably exceed the largest *legitimate* payload the protocol
/// emits. The largest is the content `ProjectionInventory` at
/// [`PROJECTION_INVENTORY_CAP`] (2000) entries ≈ 400 KiB; 1 MiB gives bounded
/// headroom above that while still shutting down the allocation-amplification
/// attack. It is intentionally not config because the protocol is uniform across
/// peers. The responder additionally never emits a frame it would itself reject
/// (see [`fit_inventory_to_budget`]).
///
/// MIXED-VERSION NOTE: an old-reader (256 KiB) peer that has not yet taken this
/// bump rejects any >256 KiB frame the same way it does today — no worse during
/// the deploy window (the pre-bump responder already couldn't *write* such a
/// frame, so the id-cap kept it under 256 KiB). The fleet converges to the 1 MiB
/// bound as peers update; the byte-budget trim below keeps every emitted frame
/// self-consistent with THIS peer's codec regardless.
pub const MAX_PAYLOAD: usize = 1024 * 1024;

/// Marker type for the federation protocol — implements `AsRef<str>` so
/// `request_response::Behaviour::with_codec` can advertise the protocol ID.
#[derive(Debug, Clone)]
pub struct ViewFederationProtocol;

impl AsRef<str> for ViewFederationProtocol {
    fn as_ref(&self) -> &str {
        VIEW_FEDERATION_PROTOCOL_ID
    }
}

/// Codec for the federation protocol. Length-prefixed MessagePack frames.
#[derive(Debug, Clone, Default)]
pub struct ViewFederationCodec;

#[async_trait]
impl request_response::Codec for ViewFederationCodec {
    type Protocol = ViewFederationProtocol;
    type Request = ViewFederationRequest;
    type Response = ViewFederationResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("view-federation request too large: {len} > {MAX_PAYLOAD}"),
            ));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("view-federation response too large: {len} > {MAX_PAYLOAD}"),
            ));
        }
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;
        rmp_serde::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        request: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = rmp_serde::to_vec_named(&request)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if data.len() > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "view-federation request exceeds MAX_PAYLOAD: {} > {MAX_PAYLOAD}",
                    data.len()
                ),
            ));
        }
        let len: u32 = data.len().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "view-federation request frame too large for u32 prefix: {} bytes",
                    data.len()
                ),
            )
        })?;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&data).await?;
        io.flush().await
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        response: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = rmp_serde::to_vec_named(&response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if data.len() > MAX_PAYLOAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "view-federation response exceeds MAX_PAYLOAD: {} > {MAX_PAYLOAD}",
                    data.len()
                ),
            ));
        }
        let len: u32 = data.len().try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "view-federation response frame too large for u32 prefix: {} bytes",
                    data.len()
                ),
            )
        })?;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&data).await?;
        io.flush().await
    }
}

/// Context for [`build_response_slice`] — bundles identity + transport fields
/// to keep the function signature within clippy's argument-count limit.
pub struct SliceContext<'a> {
    /// CID of the requesting agent (from the inbound request).
    pub agent_cid: String,
    /// Request correlation ID (echoed into the response).
    pub request_id: String,
    /// This node's agent CID — used to decide ownership.
    pub local_agent_cid: &'a str,
    /// This node's peer ID string — embedded in the returned slice.
    pub local_peer_id: String,
    /// Snapshot of connected peers at request receipt time.
    pub connected_peers: &'a [libp2p::PeerId],
    /// Signing keypair for the slice signature.
    pub keypair: &'a libp2p::identity::Keypair,
    /// DB pool for local-slice queries, or `None` in test contexts.
    pub pool: Option<&'a crate::db::DbPool>,
    /// Rotating-window offset for a `ProjectionInventory` request (from
    /// `ViewFederationRequest.inventory_offset`; `None` ⇒ 0). Ignored for other
    /// view kinds. Lets successive reconcile sweeps window across the whole
    /// corpus instead of re-serving only the capped hot set.
    pub inventory_offset: Option<u32>,
    /// Conductor registry for the `ContentHeadRecord` view kind, which must ask
    /// THIS peer's conductor for the serialized `Record` behind its declared
    /// head. `None` in test contexts and wherever the bridge is not wired — the
    /// responder then answers an honest empty record rather than erroring.
    ///
    /// A registry rather than an `HcClient`, deliberately: `lamad_client()` is
    /// interior-mutable, so a bridge that connects LATE (the common case on a
    /// slow conductor) is picked up without re-plumbing this context.
    pub hc_registry: Option<&'a crate::hc_client_registry::HcClientRegistry>,
}

/// Build a signed [`ViewFederationResponse`] for an inbound request.
///
/// T26 wires real per-view-kind slice sources: when `agent_cid` matches the
/// local agent and `pool` is `Some`, dispatches to
/// `services::cluster_view::build_local_slice` (Cluster) or
/// `services::peer_topology_view::build_local_slice` (PeerTopology).
/// When `pool` is `None` (test contexts), falls back to `serde_json::json!({})`
/// preserving the F-T20 stub behavior that tests verify.
///
/// When `agent_cid` does not match, payload is `serde_json::Value::Null`
/// (Offline freshness) regardless of pool.
///
/// `ProjectionInventory` is the exception to agent-scoping: the inventory is
/// "what THIS peer's projection holds for `table`", independent of `agent_cid`.
/// It always reports `Live` when a pool is available (local SQL is present
/// truth), and an empty inventory when `pool` is `None` (test contexts).
///
/// The `ViewSlice` is signed with `keypair` using
/// [`ViewSlice::canonical_bytes_for_signing`]. The signature is base64-encoded
/// (standard alphabet, with padding) in `slice.signature`.
///
/// # Errors
///
/// Returns `libp2p::identity::SigningError` if the keypair cannot sign (e.g. it is
/// a public-key-only handle without a private key).
pub async fn build_response_slice(
    view_kind: ViewKind,
    ctx: SliceContext<'_>,
) -> Result<ViewFederationResponse, libp2p::identity::SigningError> {
    // ProjectionInventory: not agent-scoped. Build from local SQL directly.
    if let ViewKind::ProjectionInventory { table } = &view_kind {
        let (payload, state) =
            build_inventory_payload(ctx.pool, table, ctx.inventory_offset.unwrap_or(0));
        let mut slice = ViewSlice {
            peer_id: ctx.local_peer_id,
            view_kind: view_kind.clone(),
            freshness: Freshness {
                state,
                stale_since_ms: None,
            },
            payload: JsonVal(payload),
            signature: String::new(),
        };
        let canonical = slice.canonical_bytes_for_signing();
        let sig_bytes = ctx.keypair.sign(&canonical)?;
        slice.signature = base64::engine::general_purpose::STANDARD.encode(&sig_bytes);
        return Ok(ViewFederationResponse {
            view_kind,
            agent_cid: ctx.agent_cid,
            request_id: ctx.request_id,
            slice,
        });
    }

    // ContentHeadRecord: not agent-scoped either. "What head Record does THIS
    // peer hold for this id" is a property of the responder, not of whoever
    // asked — same exception as ProjectionInventory, and for the same reason.
    if let ViewKind::ContentHeadRecord { content_id } = &view_kind {
        let (payload, state) =
            build_content_head_record_payload(ctx.pool, ctx.hc_registry, content_id).await;
        let mut slice = ViewSlice {
            peer_id: ctx.local_peer_id,
            view_kind: view_kind.clone(),
            freshness: Freshness {
                state,
                stale_since_ms: None,
            },
            payload: JsonVal(payload),
            signature: String::new(),
        };
        let canonical = slice.canonical_bytes_for_signing();
        let sig_bytes = ctx.keypair.sign(&canonical)?;
        slice.signature = base64::engine::general_purpose::STANDARD.encode(&sig_bytes);
        return Ok(ViewFederationResponse {
            view_kind,
            agent_cid: ctx.agent_cid,
            request_id: ctx.request_id,
            slice,
        });
    }

    let owns_agent = ctx.agent_cid == ctx.local_agent_cid;
    let payload = if owns_agent {
        match view_kind {
            ViewKind::Cluster => match ctx.pool {
                Some(p) => crate::services::cluster_view::build_local_slice(p).await,
                None => serde_json::json!({}),
            },
            ViewKind::PeerTopology => match ctx.pool {
                Some(p) => {
                    crate::services::peer_topology_view::build_local_slice(p, ctx.connected_peers)
                        .await
                }
                None => serde_json::json!({}),
            },
            // Both handled above by their early-returns.
            ViewKind::ProjectionInventory { .. } | ViewKind::ContentHeadRecord { .. } => {
                unreachable!()
            }
        }
    } else {
        serde_json::Value::Null
    };
    let freshness_state = if owns_agent {
        FreshnessState::Live
    } else {
        FreshnessState::Offline
    };
    let mut slice = ViewSlice {
        peer_id: ctx.local_peer_id,
        view_kind: view_kind.clone(),
        freshness: Freshness {
            state: freshness_state,
            stale_since_ms: None,
        },
        payload: JsonVal(payload),
        signature: String::new(),
    };
    let canonical = slice.canonical_bytes_for_signing();
    let sig_bytes = ctx.keypair.sign(&canonical)?;
    slice.signature = base64::engine::general_purpose::STANDARD.encode(&sig_bytes);
    Ok(ViewFederationResponse {
        view_kind,
        agent_cid: ctx.agent_cid,
        request_id: ctx.request_id,
        slice,
    })
}

/// Build the `(payload_json, freshness)` for a [`ViewKind::ContentHeadRecord`]
/// request — the P2P twin of `GET /db/content/{id}/head-record`.
///
/// EVERY failure is an honest ABSENCE, never a transport error: no pool, no
/// conductor bridge, no row, no declared head, or a conductor that cannot
/// retrieve its own head action all answer a well-formed payload with
/// `headActionHash: null`. The requester then falls through to the author path.
/// Erroring instead would make a peer's inability to serve a record look like a
/// network fault, and the caller would retry it forever.
///
/// The head hash comes from the LOCAL PROJECTION (the same answer
/// `GET /db/content/{id}/head` gives), so the carried record always matches the
/// hash a caller read from this peer's inventory a moment earlier.
async fn build_content_head_record_payload(
    pool: Option<&crate::db::DbPool>,
    hc_registry: Option<&crate::hc_client_registry::HcClientRegistry>,
    content_id: &str,
) -> (serde_json::Value, FreshnessState) {
    let absent = |state: FreshnessState| {
        (
            serde_json::to_value(ContentHeadRecordPayload {
                content_id: content_id.to_string(),
                head_action_hash: None,
                declared_at: None,
                record: None,
            })
            .unwrap_or(serde_json::Value::Null),
            state,
        )
    };

    let Some(p) = pool else {
        return absent(FreshnessState::Offline);
    };
    let Ok(mut conn) = p.get() else {
        return absent(FreshnessState::Offline);
    };
    let app_ctx = crate::db::AppContext::default_lamad();

    let row = match crate::db::content_diesel::get_content_with_tags(
        &mut conn,
        &app_ctx,
        content_id,
        crate::db::content_diesel::MinTrust::Invisible,
    ) {
        Ok(Some(cwt)) => cwt,
        // No row, or an unreadable one — honest absence, still Live (this IS
        // this peer's truth about the id).
        _ => return absent(FreshnessState::Live),
    };
    let declared_at = row.content.declared_head_at;
    let Some(view) = crate::views::content_head_view_from_content(&row.content) else {
        return absent(FreshnessState::Live);
    };
    let head_action_hash = view.head_action_hash;
    drop(conn);

    // Ask this peer's own conductor for the serialized Record. Without a bridge
    // we still serve the HASH — a requester whose own conductor can retrieve the
    // target does not need the bytes.
    let record_b64 = match hc_registry.and_then(|r| r.lamad_client()) {
        Some(hc) => {
            match crate::services::conductor_writes::call_get_record_for_action(
                &hc,
                &head_action_hash,
            )
            .await
            {
                Ok(Some(carried)) => {
                    Some(base64::engine::general_purpose::STANDARD.encode(&carried.record))
                }
                Ok(None) => None,
                Err(e) => {
                    tracing::debug!(
                        target: "elohim_storage::view_federation",
                        content_id = %content_id,
                        error = %e,
                        "ContentHeadRecord: conductor could not serve the record; \
                         answering with the head hash alone"
                    );
                    None
                }
            }
        }
        None => None,
    };

    (
        serde_json::to_value(ContentHeadRecordPayload {
            content_id: content_id.to_string(),
            head_action_hash: Some(head_action_hash),
            declared_at,
            record: record_b64,
        })
        .unwrap_or(serde_json::Value::Null),
        FreshnessState::Live,
    )
}

/// Fixed headroom reserved for everything in the serialized frame that is NOT the
/// inventory entries: the `ProjectionInventoryPayload` map wrapper (`table` +
/// `total`), the `ViewSlice` envelope (peer_id, view_kind, freshness, ~88-byte
/// base64 signature) and the `ViewFederationResponse` envelope (view_kind,
/// agent_cid, request_id), plus MessagePack map keys. 8 KiB dwarfs what any of
/// these need even with long peer/agent ids — deliberately generous so the
/// payload budget is a safe UNDER-estimate of the room the entries may occupy.
const INVENTORY_ENVELOPE_RESERVE: usize = 8 * 1024;

/// Byte budget for the serialized inventory *payload* alone — [`MAX_PAYLOAD`]
/// minus the envelope reserve. A payload trimmed to this budget guarantees the
/// full `ViewFederationResponse` frame stays under the codec's write bound.
const INVENTORY_PAYLOAD_BUDGET: usize = MAX_PAYLOAD - INVENTORY_ENVELOPE_RESERVE;

/// MessagePack-serialized byte length of an inventory payload — the same
/// named-field encoding the codec puts on the wire (the payload rides the frame
/// as a `JsonVal` map with these exact keys), so this is a faithful,
/// envelope-free proxy for the payload's contribution to the frame.
fn inventory_payload_len(payload: &ProjectionInventoryPayload) -> usize {
    rmp_serde::to_vec_named(payload)
        .map(|v| v.len())
        .unwrap_or(usize::MAX)
}

/// Responder self-protection: trim trailing entries from `payload` until its
/// serialized size fits within `budget`, returning the number of entries dropped
/// (0 if it already fit).
///
/// The invariant this upholds: **the responder must NEVER emit a payload its own
/// codec's `write_response` will reject.** The id-cap ([`PROJECTION_INVENTORY_CAP`])
/// bounds the row COUNT; this bounds the serialized BYTE size, which id width can
/// still push over [`MAX_PAYLOAD`] independent of the count (the scenario-2
/// blocker: 2000 content entries serialized to ~340–400 KiB > the old 256 KiB
/// bound and the responder's own write rejected mid-frame, so the requester saw
/// `Io(UnexpectedEof)`). Entries arrive hot-set-first (the content query orders
/// `updated_at DESC` — the most-recently-changed anchors, the reconcile hot set —
/// see `db::content_diesel::list_content_anchor_inventory`), so a dropped tail is
/// the coldest, least-urgent rows; the tail converges over later sweeps.
///
/// Strategy: compute-from-average with re-measurement. Each pass estimates the
/// fitting count from the current per-entry average and re-measures. While over
/// budget, `floor(budget/avg) < entries.len()`, so every pass strictly shrinks
/// the set — it converges in a handful of iterations without an O(n) per-entry
/// serialize. `total` is left at the pre-trim value so `total > entries.len()`
/// surfaces the drop to the requester (same signal as the id-cap truncation).
fn fit_inventory_to_budget(payload: &mut ProjectionInventoryPayload, budget: usize) -> usize {
    let original = payload.entries.len();
    let mut len = inventory_payload_len(payload);
    while len > budget && !payload.entries.is_empty() {
        let n = payload.entries.len();
        let avg = (len / n).max(1);
        // floor(budget/avg) < n whenever len > budget → strict progress each pass.
        let keep = (budget / avg).min(n - 1);
        payload.entries.truncate(keep);
        len = inventory_payload_len(payload);
    }
    original - payload.entries.len()
}

/// Build the `(payload_json, freshness)` for a `ProjectionInventory` request.
///
/// Reads the local projection for `table` capped at [`PROJECTION_INVENTORY_CAP`].
/// v1 tables: `rea_commitments` (newest-first) and `content` (updated_at-desc —
/// hot set first — notary-authority Leg 4). Every built payload is passed through
/// [`fit_inventory_to_budget`] before it is returned, so the responder NEVER
/// emits a frame its own codec's `write_response` would reject. `Live` when the
/// pool is present (local SQL is present truth); `Offline` with an empty inventory
/// when `pool` is `None` (test/conductor-less contexts) or the table is unknown.
fn build_inventory_payload(
    pool: Option<&crate::db::DbPool>,
    table: &str,
    offset: u32,
) -> (serde_json::Value, FreshnessState) {
    let empty = || {
        (
            serde_json::to_value(ProjectionInventoryPayload {
                table: table.to_string(),
                total: 0,
                entries: Vec::new(),
            })
            .unwrap_or(serde_json::Value::Null),
            FreshnessState::Offline,
        )
    };

    // Known tables only. The discriminator is the seam for other tables; an
    // unknown table yields an honest empty inventory.
    if table != PROJECTION_INVENTORY_TABLE_REA_COMMITMENTS
        && table != PROJECTION_INVENTORY_TABLE_CONTENT
        && table != PROJECTION_INVENTORY_TABLE_COLLECTIVES
    {
        return empty();
    }
    let Some(p) = pool else {
        return empty();
    };
    let Ok(mut conn) = p.get() else {
        return empty();
    };
    let app_ctx = crate::db::AppContext::default_lamad();

    // Content table (notary-authority Leg 4) — anchored, distribution-safe rows.
    if table == PROJECTION_INVENTORY_TABLE_CONTENT {
        return match crate::db::content_diesel::list_content_anchor_inventory(
            &mut conn,
            &app_ctx,
            i64::from(offset),
            PROJECTION_INVENTORY_CAP,
        ) {
            Ok((rows, db_total)) => {
                // HONEST total: the true whole-corpus count (offset-invariant), NOT
                // the served page length. This is what lets the requester detect
                // truncation on the wire and window the cold tail across sweeps.
                let total = usize::try_from(db_total).unwrap_or(usize::MAX);
                let served = rows.len();
                // Truncated only when rows remain BEYOND this page: offset + served
                // < total. Accounting for the offset means the final page of a
                // rotating window (which reaches the corpus end) is NOT mislabelled
                // "windowed" just because served < total. Surfaced as a WARN so a
                // genuine mid-window truncation is never silent.
                let cap_truncated = (offset as usize).saturating_add(served) < total;
                let entries = rows
                    .into_iter()
                    .map(|r| ProjectionInventoryEntry {
                        id: r.id,
                        dht_anchor_hash: r.dht_anchor_hash,
                        // ADOPT-BEFORE-AUTHOR: advertise the DECLARATION too, so a
                        // restarting peer can adopt this one instead of minting a
                        // competing root. Additive on the wire — a pre-cure
                        // requester simply ignores the keys.
                        declared_head_action_hash: r.declared_head_action_hash,
                        declared_head_at: r.declared_head_at,
                    })
                    .collect();
                let mut payload = ProjectionInventoryPayload {
                    table: table.to_string(),
                    total,
                    entries,
                };
                // Responder self-protection: NEVER emit a frame our own
                // `write_response` would reject. The id-cap above bounds the row
                // COUNT; this bounds the serialized BYTE size (id width can push
                // 2000 rows over MAX_PAYLOAD — the scenario-2 EOF blocker). Trims
                // the cold tail first (rows arrive updated_at-desc, hot set first).
                let byte_dropped = fit_inventory_to_budget(&mut payload, INVENTORY_PAYLOAD_BUDGET);
                if cap_truncated {
                    tracing::warn!(
                        target: "elohim_storage::view_federation",
                        table = %table,
                        cap = PROJECTION_INVENTORY_CAP,
                        offset = offset,
                        served = payload.entries.len(),
                        total,
                        "ProjectionInventory: content inventory windowed \
                         (this page does not carry the whole corpus; honest total on the wire \
                         lets the requester window the remainder across sweeps)"
                    );
                }
                if byte_dropped > 0 {
                    tracing::warn!(
                        target: "elohim_storage::view_federation",
                        table = %table,
                        budget = INVENTORY_PAYLOAD_BUDGET,
                        dropped = byte_dropped,
                        served = payload.entries.len(),
                        "ProjectionInventory: content inventory trimmed to byte budget \
                         (dropped the cold tail so the serialized frame stays under MAX_PAYLOAD)"
                    );
                }
                tracing::info!(
                    target: "elohim_storage::view_federation",
                    table = %table,
                    entries = payload.entries.len(),
                    total,
                    "ProjectionInventory: serving local inventory"
                );
                (
                    serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
                    FreshnessState::Live,
                )
            }
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::view_federation",
                    table = %table,
                    error = %e,
                    "ProjectionInventory: local inventory query failed; returning empty"
                );
                empty()
            }
        };
    }

    // Collectives (cross-peer collective-identity reconcile) — DHT-anchored,
    // non-dissolved rows only. The corpus is small (dozens), so the cap is
    // effectively never reached; the truncation/byte guards below are the same
    // belt-and-suspenders the other tables carry, not an expected path.
    if table == PROJECTION_INVENTORY_TABLE_COLLECTIVES {
        return match crate::db::collectives::list_collective_cid_inventory(
            &mut conn,
            &app_ctx,
            i64::from(offset),
            PROJECTION_INVENTORY_CAP,
        ) {
            Ok((rows, db_total)) => {
                let total = usize::try_from(db_total).unwrap_or(usize::MAX);
                let served = rows.len();
                let cap_truncated = (offset as usize).saturating_add(served) < total;
                let entries = rows
                    .into_iter()
                    // The `dhtAnchorHash` slot carries the collective CID — the
                    // reconciliation identity for this table.
                    .map(|(id, collective_cid)| ProjectionInventoryEntry {
                        id,
                        dht_anchor_hash: collective_cid,
                        // Declared-head hints are a CONTENT-table concept — the
                        // collectives arm reconciles on CID identity and has no
                        // notary-head election of its own.
                        declared_head_action_hash: None,
                        declared_head_at: None,
                    })
                    .collect();
                let mut payload = ProjectionInventoryPayload {
                    table: table.to_string(),
                    total,
                    entries,
                };
                let byte_dropped = fit_inventory_to_budget(&mut payload, INVENTORY_PAYLOAD_BUDGET);
                if cap_truncated || byte_dropped > 0 {
                    tracing::warn!(
                        target: "elohim_storage::view_federation",
                        table = %table,
                        cap = PROJECTION_INVENTORY_CAP,
                        offset = offset,
                        dropped = byte_dropped,
                        served = payload.entries.len(),
                        total,
                        "ProjectionInventory: collectives inventory windowed/trimmed \
                         (honest total on the wire lets the requester window the remainder)"
                    );
                }
                tracing::info!(
                    target: "elohim_storage::view_federation",
                    table = %table,
                    entries = payload.entries.len(),
                    total,
                    "ProjectionInventory: serving local inventory"
                );
                (
                    serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
                    FreshnessState::Live,
                )
            }
            Err(e) => {
                tracing::warn!(
                    target: "elohim_storage::view_federation",
                    table = %table,
                    error = %e,
                    "ProjectionInventory: local inventory query failed; returning empty"
                );
                empty()
            }
        };
    }

    // rea_commitments (the pre-Leg-4 path, plus the shared byte-budget guard).
    match crate::db::rea_commitments::inventory_for_reconcile(
        &mut conn,
        &app_ctx,
        i64::from(offset),
        PROJECTION_INVENTORY_CAP,
    ) {
        Ok((rows, total)) => {
            let entries = rows
                .into_iter()
                .map(|(id, dht_anchor_hash)| ProjectionInventoryEntry {
                    id,
                    dht_anchor_hash,
                    // REA commitments have no notary-head election either.
                    declared_head_action_hash: None,
                    declared_head_at: None,
                })
                .collect();
            let mut payload = ProjectionInventoryPayload {
                table: table.to_string(),
                total,
                entries,
            };
            // Responder self-protection (same invariant as the content path):
            // never emit a frame our own codec would reject. A no-op unless a
            // wide rea inventory approaches MAX_PAYLOAD.
            let byte_dropped = fit_inventory_to_budget(&mut payload, INVENTORY_PAYLOAD_BUDGET);
            if byte_dropped > 0 {
                tracing::warn!(
                    target: "elohim_storage::view_federation",
                    table = %table,
                    budget = INVENTORY_PAYLOAD_BUDGET,
                    dropped = byte_dropped,
                    served = payload.entries.len(),
                    "ProjectionInventory: rea inventory trimmed to byte budget \
                     (dropped the tail so the serialized frame stays under MAX_PAYLOAD)"
                );
            }
            // INFO: the serve side of the discovery exchange. Paired with the
            // asker's "peer inventory received" line this makes an empty
            // exchange attributable (served 0 vs response lost vs undecodable).
            tracing::info!(
                target: "elohim_storage::view_federation",
                table = %table,
                entries = payload.entries.len(),
                total,
                "ProjectionInventory: serving local inventory"
            );
            (
                serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
                FreshnessState::Live,
            )
        }
        Err(e) => {
            tracing::warn!(
                target: "elohim_storage::view_federation",
                table = %table,
                error = %e,
                "ProjectionInventory: local inventory query failed; returning empty"
            );
            empty()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::{Freshness, FreshnessState, JsonVal, ViewKind, ViewSlice};
    use libp2p::request_response::Codec;

    #[test]
    fn protocol_id_is_canonical() {
        assert_eq!(
            ViewFederationProtocol.as_ref(),
            "/elohim/view-federation/1.0.0"
        );
    }

    #[test]
    fn request_msgpack_round_trips() {
        let req = ViewFederationRequest {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent_abc".to_string(),
            request_id: "req_001".to_string(),
            inventory_offset: None,
        };
        let bytes = rmp_serde::to_vec_named(&req).unwrap();
        let back: ViewFederationRequest = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back.agent_cid, "agent_abc");
        assert_eq!(back.view_kind, ViewKind::Cluster);
        assert_eq!(back.request_id, "req_001");
    }

    #[tokio::test]
    async fn codec_round_trips_request_via_async_buffer() {
        use futures::io::Cursor;
        let mut codec = ViewFederationCodec;
        let proto = ViewFederationProtocol;
        let req = ViewFederationRequest {
            view_kind: ViewKind::PeerTopology,
            agent_cid: "agent_xyz".to_string(),
            request_id: "req_002".to_string(),
            inventory_offset: None,
        };
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = Cursor::new(&mut buf);
            codec
                .write_request(&proto, &mut writer, req.clone())
                .await
                .expect("write_request");
        }
        let mut reader = Cursor::new(&buf);
        let back = codec
            .read_request(&proto, &mut reader)
            .await
            .expect("read_request");
        assert_eq!(back.agent_cid, req.agent_cid);
        assert_eq!(back.view_kind, req.view_kind);
        assert_eq!(back.request_id, req.request_id);
    }

    #[tokio::test]
    async fn codec_round_trips_response_via_async_buffer() {
        use futures::io::Cursor;
        let mut codec = ViewFederationCodec;
        let proto = ViewFederationProtocol;
        let response = ViewFederationResponse {
            view_kind: ViewKind::Cluster,
            agent_cid: "agent_abc".to_string(),
            request_id: "req_003".to_string(),
            slice: ViewSlice {
                peer_id: "12D3KooWAAA".to_string(),
                view_kind: ViewKind::Cluster,
                freshness: Freshness {
                    state: FreshnessState::Live,
                    stale_since_ms: None,
                },
                payload: JsonVal(serde_json::json!({"devices": []})),
                signature: "sig-base64".to_string(),
            },
        };
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut writer = Cursor::new(&mut buf);
            codec
                .write_response(&proto, &mut writer, response.clone())
                .await
                .expect("write_response");
        }
        let mut reader = Cursor::new(&buf);
        let back = codec
            .read_response(&proto, &mut reader)
            .await
            .expect("read_response");
        assert_eq!(back.agent_cid, response.agent_cid);
        assert_eq!(back.request_id, response.request_id);
        assert_eq!(back.view_kind, response.view_kind);
        assert_eq!(back.slice.peer_id, response.slice.peer_id);
        assert_eq!(back.slice.signature, response.slice.signature);
    }

    #[tokio::test]
    async fn codec_rejects_oversized_request_length_prefix() {
        use futures::io::Cursor;
        let mut codec = ViewFederationCodec;
        let proto = ViewFederationProtocol;
        // The codec must reject before it allocates the body buffer.
        let oversized_len: u32 = (MAX_PAYLOAD as u32) + 1;
        let mut input = Vec::new();
        input.extend_from_slice(&oversized_len.to_be_bytes());
        let mut reader = Cursor::new(&input);
        let result = codec.read_request(&proto, &mut reader).await;
        assert!(
            result.is_err(),
            "codec should reject request length prefix > MAX_PAYLOAD"
        );
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[tokio::test]
    async fn codec_rejects_oversized_response_length_prefix() {
        use futures::io::Cursor;
        let mut codec = ViewFederationCodec;
        let proto = ViewFederationProtocol;
        let oversized_len: u32 = (MAX_PAYLOAD as u32) + 1;
        let mut input = Vec::new();
        input.extend_from_slice(&oversized_len.to_be_bytes());
        let mut reader = Cursor::new(&input);
        let result = codec.read_response(&proto, &mut reader).await;
        assert!(
            result.is_err(),
            "codec should reject response length prefix > MAX_PAYLOAD"
        );
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn content_inventory_empty_and_offline_without_pool() {
        // Poolless/conductor-less context: the content table is KNOWN, but with
        // no pool the responder must return an honest empty inventory (Offline),
        // never a fabricated one.
        let (val, state) = build_inventory_payload(None, PROJECTION_INVENTORY_TABLE_CONTENT, 0);
        assert_eq!(state, FreshnessState::Offline);
        let payload: ProjectionInventoryPayload = serde_json::from_value(val).unwrap();
        assert_eq!(payload.table, "content");
        assert_eq!(payload.total, 0);
        assert!(payload.entries.is_empty());
    }

    #[test]
    fn unknown_table_still_empty_after_content_added() {
        // Adding the `content` table must not make an UNKNOWN table serve rows.
        let (val, state) = build_inventory_payload(None, "agreements", 0);
        assert_eq!(state, FreshnessState::Offline);
        let payload: ProjectionInventoryPayload = serde_json::from_value(val).unwrap();
        assert_eq!(payload.table, "agreements");
        assert!(payload.entries.is_empty());
    }

    /// Responder contract for the `collectives` table (reconcile ruling 1): a
    /// real pool serves ONLY DHT-anchored, non-dissolved rows, `Live`, with the
    /// `collective_cid` in the `dhtAnchorHash` slot. A pre-coherence (NULL-cid)
    /// row has no DHT identity to reconcile on and must never reach the wire.
    #[test]
    fn collectives_inventory_served_from_pool_excludes_unanchored() {
        use crate::db::collectives::{create_collective, CreateCollectiveInput};
        let pool = crate::test_util::test_pool();
        let ctx = crate::db::AppContext::default_lamad();
        const CID: &str = "collective:uhCkkAlphaCollectiveActionHash00000000000";
        {
            let mut conn = pool.get().expect("pool conn");
            let mk = |id: &str| CreateCollectiveInput {
                id: id.to_string(),
                name: id.to_string(),
                description: None,
                governance_layer: "family".to_string(),
                constitutional_parent_id: None,
                reach: "trusted".to_string(),
                region: None,
                metadata_json: None,
                created_by: None,
            };
            create_collective(&mut conn, &ctx, &mk("anchored")).unwrap();
            create_collective(&mut conn, &ctx, &mk("preco")).unwrap();
            use crate::db::diesel_schema::collectives;
            use diesel::prelude::*;
            diesel::update(collectives::table.filter(collectives::id.eq("anchored")))
                .set(collectives::collective_cid.eq(Some(CID)))
                .execute(&mut conn)
                .unwrap();
        }

        let (val, state) =
            build_inventory_payload(Some(&pool), PROJECTION_INVENTORY_TABLE_COLLECTIVES, 0);
        assert_eq!(state, FreshnessState::Live);
        let payload: ProjectionInventoryPayload = serde_json::from_value(val).unwrap();
        assert_eq!(payload.table, "collectives");
        let ids: Vec<&str> = payload.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["anchored"],
            "only the DHT-anchored row is advertised (ruling 1: NULL cid = nothing to reconcile on)"
        );
        assert_eq!(
            payload.entries[0].dht_anchor_hash, CID,
            "the collective CID rides the dhtAnchorHash slot — it IS the reconciliation identity"
        );
        assert_eq!(payload.total, 1);
    }

    #[test]
    fn content_inventory_served_from_pool_filters_scoped_and_unanchored() {
        // Responder path for the content table (mirrors the rea_commitments
        // serve contract): a real pool serves ONLY anchored, distribution-safe
        // rows, Live. Scoped-tier and un-anchored rows must not appear on the
        // cross-peer wire.
        use crate::db::content_diesel::{create_content, CreateContentInput};
        let pool = crate::test_util::test_pool();
        let ctx = crate::db::AppContext::default_lamad();
        {
            let mut conn = pool.get().expect("pool conn");
            let mk = |id: &str, reach: &str, anchor: Option<&str>| CreateContentInput {
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
                dht_anchor_hash: anchor.map(str::to_string),
            };
            create_content(&mut conn, &ctx, mk("wf:public", "public", Some("anc-1"))).unwrap();
            create_content(&mut conn, &ctx, mk("wf:private", "private", Some("anc-2"))).unwrap();
            create_content(&mut conn, &ctx, mk("wf:noanchor", "commons", None)).unwrap();
        }

        let (val, state) =
            build_inventory_payload(Some(&pool), PROJECTION_INVENTORY_TABLE_CONTENT, 0);
        assert_eq!(state, FreshnessState::Live);
        let payload: ProjectionInventoryPayload = serde_json::from_value(val).unwrap();
        assert_eq!(payload.table, "content");
        let ids: Vec<&str> = payload.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["wf:public"],
            "only the anchored distribution-safe row is served"
        );
        assert_eq!(payload.entries[0].dht_anchor_hash, "anc-1");
        assert_eq!(payload.total, 1);
    }

    #[test]
    fn content_inventory_offset_windows_across_corpus_with_honest_total() {
        // Structural non-convergence fix (Fix 2a + 2b): a corpus larger than one
        // page must (a) advertise the HONEST whole-corpus total (not the served
        // count) so truncation is visible on the wire, and (b) advance a window
        // via the request offset so the cold tail is eventually served.
        use crate::db::content_diesel::{create_content, CreateContentInput};
        let pool = crate::test_util::test_pool();
        let ctx = crate::db::AppContext::default_lamad();
        // Three anchored commons rows. Ordering across pages is deterministic via
        // the (updated_at desc, id asc) tiebreak in the query; the assertions here
        // only need the counts, so exact identities are not pinned.
        for id in ["cw:a", "cw:b", "cw:c"] {
            let mut conn = pool.get().expect("pool conn");
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
                    reach: "commons".to_string(),
                    created_by: None,
                    tags: vec![],
                    content_body: None,
                    dht_anchor_hash: Some(format!("anc-{id}")),
                },
            )
            .unwrap();
        }

        // Page 0 (offset 0). The honest total is 3 regardless of how many rows a
        // single page carries — the requester can always detect the remainder.
        let (val0, _) = build_inventory_payload(Some(&pool), PROJECTION_INVENTORY_TABLE_CONTENT, 0);
        let p0: ProjectionInventoryPayload = serde_json::from_value(val0).unwrap();
        assert_eq!(p0.total, 3, "honest total = whole corpus, not served count");

        // A window past the first row (offset 2) reaches the cold tail — proving
        // successive sweeps cover the corpus rather than re-serving the hot set.
        let (val2, _) = build_inventory_payload(Some(&pool), PROJECTION_INVENTORY_TABLE_CONTENT, 2);
        let p2: ProjectionInventoryPayload = serde_json::from_value(val2).unwrap();
        assert_eq!(p2.total, 3, "total is offset-invariant");
        let tail_ids: Vec<&str> = p2.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            tail_ids.len(),
            1,
            "offset 2 over a 3-row corpus serves exactly the cold tail row"
        );
    }

    /// A realistic content entry: ~68-char id + 53-char anchor. Mirrors the live
    /// commons corpus that triggered the scenario-2 EOF (2088 anchored rows).
    fn realistic_entry(i: usize) -> ProjectionInventoryEntry {
        ProjectionInventoryEntry {
            id: format!("epr:commons:lamad/atomic-concept/really-long-content-slug-{i:010}"),
            // 53-char anchor (`uhCEk` + 48 base64-ish chars), like a real ActionHash.
            dht_anchor_hash: format!("uhCEk{}", "A".repeat(48)),
            // Worst case for the byte-budget guard: a peer that declares EVERY
            // row widens each entry by a second ActionHash + a timestamp, which
            // is exactly the width growth `fit_inventory_to_budget` must absorb.
            declared_head_action_hash: Some(format!("uhCkk{}", "B".repeat(48))),
            declared_head_at: Some(1_753_000_000_000_000),
        }
    }

    #[test]
    fn full_cap_inventory_exceeds_old_bound_but_fits_new() {
        // Byte-math proof of the fix: 2000 realistic content entries serialize to
        // MORE than the OLD 256 KiB bound (the responder's own write rejected →
        // requester Io(UnexpectedEof)) yet comfortably UNDER the new 1 MiB
        // MAX_PAYLOAD. This is the exact regression the raise addresses.
        let entries: Vec<ProjectionInventoryEntry> = (0..PROJECTION_INVENTORY_CAP as usize)
            .map(realistic_entry)
            .collect();
        let payload = ProjectionInventoryPayload {
            table: PROJECTION_INVENTORY_TABLE_CONTENT.to_string(),
            total: entries.len(),
            entries,
        };
        let serialized = inventory_payload_len(&payload);
        assert!(
            serialized > 256 * 1024,
            "2000-entry content inventory ({serialized} B) must exceed the OLD 256 KiB bound"
        );
        assert!(
            serialized < MAX_PAYLOAD,
            "2000-entry content inventory ({serialized} B) must fit the new 1 MiB MAX_PAYLOAD"
        );
        // And with the envelope reserve it still fits the payload budget → no trim
        // fires in the healthy path; the raise alone resolves the blocker.
        assert!(serialized <= INVENTORY_PAYLOAD_BUDGET);
    }

    #[test]
    fn over_budget_inventory_is_trimmed_to_fit_and_reports_drop() {
        // Responder self-protection: an over-budget entry set must produce a
        // FITTING frame and report a non-zero drop (the count the WARN emits).
        let entries: Vec<ProjectionInventoryEntry> = (0..PROJECTION_INVENTORY_CAP as usize)
            .map(realistic_entry)
            .collect();
        let original = entries.len();
        let mut payload = ProjectionInventoryPayload {
            table: PROJECTION_INVENTORY_TABLE_CONTENT.to_string(),
            total: original,
            entries,
        };

        // A deliberately small budget forces the trim regardless of MAX_PAYLOAD.
        let budget = 64 * 1024;
        assert!(
            inventory_payload_len(&payload) > budget,
            "fixture must start over the test budget"
        );

        let dropped = fit_inventory_to_budget(&mut payload, budget);

        // The WARN path guard: a positive drop count is exactly what the responder
        // logs (`dropped = byte_dropped`), so proving `dropped > 0` proves the WARN
        // branch is taken for an over-budget set.
        assert!(dropped > 0, "an over-budget set must drop trailing entries");
        assert_eq!(
            dropped,
            original - payload.entries.len(),
            "reported drop count matches the entries removed"
        );
        assert!(
            !payload.entries.is_empty(),
            "a fittable set must not be emptied"
        );
        // The self-protection invariant: the trimmed payload now fits the budget.
        assert!(
            inventory_payload_len(&payload) <= budget,
            "trimmed payload must fit the byte budget"
        );
        // `total` still reports the pre-trim count so `total > entries.len()`
        // surfaces the byte-drop to the requester.
        assert_eq!(payload.total, original);
        assert!(payload.total > payload.entries.len());
    }

    #[test]
    fn within_budget_inventory_is_untouched() {
        // Under budget → no trim, no drop, entries preserved verbatim.
        let mut payload = ProjectionInventoryPayload {
            table: PROJECTION_INVENTORY_TABLE_CONTENT.to_string(),
            total: 3,
            entries: vec![
                ProjectionInventoryEntry {
                    id: "epr:a".to_string(),
                    dht_anchor_hash: "anc-a".to_string(),
                    declared_head_action_hash: None,
                    declared_head_at: None,
                },
                ProjectionInventoryEntry {
                    id: "epr:b".to_string(),
                    dht_anchor_hash: "anc-b".to_string(),
                    declared_head_action_hash: None,
                    declared_head_at: None,
                },
                ProjectionInventoryEntry {
                    id: "epr:c".to_string(),
                    dht_anchor_hash: "anc-c".to_string(),
                    declared_head_action_hash: None,
                    declared_head_at: None,
                },
            ],
        };
        let dropped = fit_inventory_to_budget(&mut payload, INVENTORY_PAYLOAD_BUDGET);
        assert_eq!(dropped, 0, "a small payload under budget is never trimmed");
        assert_eq!(payload.entries.len(), 3);
    }
}
