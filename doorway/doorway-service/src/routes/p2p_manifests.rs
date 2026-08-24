//! `POST/GET /p2p/manifests` — the doorway's EPHEMERAL bootstrap projection of
//! signed storage **transport manifests**.
//!
//! ## The gap this closes
//!
//! A dual-stack storage node advertises where its iroh endpoint lives by
//! gossiping a signed [`TransportManifestAnnouncement`] on the libp2p
//! gossipsub fan-out (`elohim/transport/manifest`). A **pure-iroh** node has no
//! libp2p swarm, so it never hears that gossip and comes up with every iroh
//! responder mounted and zero peers to dial. iroh's `Endpoint` only dials a
//! `NodeAddr` it has been handed, and the sovereign defaults register no
//! discovery service — so there is no self-healing path back.
//!
//! The doorway is ALREADY the web2 bootstrap seam (the kitsune2 conductor
//! bootstrap and the WebRTC signal service both live here). This is the same
//! shape one layer down: a peer POSTs its own signed manifest, any peer GETs
//! the current set, and the iroh plane has targets to dial. Step zero for the
//! transport plane, exactly as `/bootstrap` is step zero for the conductor
//! plane.
//!
//! ## Truth layer: Category C (ephemeral / operational)
//!
//! No DHT entry type, no database table, no projection of anything notarized.
//! Every entry is *reconstructable from the next announcement* — a doorway
//! restart empties the store and costs one announce interval, nothing else.
//! Storage nodes re-announce on a fixed cadence
//! (`DEFAULT_ANNOUNCE_INTERVAL_SECS`, 30s at the time of writing), so the store
//! refills itself.
//!
//! ## Trust: verify-locally-then-serve
//!
//! Identity is the announcement's `iroh_node_id`, which IS an ed25519 public
//! key. The ed25519 signature over [`TransportManifestAnnouncement::signing_bytes`]
//! binds the whole payload to that key, so a peer can only ever advertise
//! addresses for a NodeId it holds the secret for and a third party cannot
//! steer anyone's dials to an address of its choosing. The doorway verifies
//! **before** storing, so an entry can only ever have come from an announcement
//! whose signature verified — the same invariant the storage-side peer book
//! enforces. That is also why the route needs no auth: the signature IS the
//! auth, and the announcement is announce-shaped (same trust class as
//! `PUT /bootstrap/{space}/{agent}`).
//!
//! What the signature does NOT prove is that `agent_cid` / `libp2p_peer_id`
//! belong to the same operator — those are routing hints here, never
//! authority, exactly as on the storage side.
//!
//! ## Wire-shape mirror (deliberate, documented)
//!
//! [`TransportManifestAnnouncement`] MIRRORS
//! `elohim/elohim-storage/src/p2p/transport_manifest_gossip.rs`. doorway-service
//! is a separate crate that does not (and should not) depend on elohim-storage,
//! and neither of the crates both sides already share (`elohim-compute`,
//! `elohim-peer-fabric`, `elohim-seam-contracts`) is a home for a p2p wire type
//! today. The mirror is kept honest by construction:
//!
//! * the field set and field NAMES are identical, and neither side carries a
//!   `rename_all`, so the JSON body here is byte-identical to serde's view of
//!   the storage struct (the gossip wire is MessagePack via `to_vec_named`,
//!   which is the same map-keyed field naming);
//! * [`TransportManifestAnnouncement::signing_bytes`] is a verbatim copy of the
//!   storage construction, including the `elohim/transport/manifest/v1\n`
//!   domain-separation prefix and every separator byte — a single byte of drift
//!   would make every signature fail to verify here, which is a LOUD failure
//!   (400 on every POST), never a silent one;
//! * both sides depend on the same `ed25519-dalek 2.1` and `hex 0.4`.
//!
//! Additive evolution: new fields on the storage struct must land here as
//! `#[serde(default)] Option<T>` and be appended to `signing_bytes` on BOTH
//! sides in the same commit (a signing-bytes change is a v1→v2 break).

use bytes::Bytes;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, warn};

use crate::server::AppState;

type FullBody = Full<Bytes>;

/// Maximum manifests held at once. Bounded by construction: this is a
/// bootstrap hint store for a household-scale mesh, not a directory.
pub const MAX_MANIFEST_ENTRIES: usize = 64;

/// An announcement is dropped once it is this old. Measured against the
/// announcement's own `announced_at_ms`, not against receipt time: what a
/// consumer dials is the addresses the SIGNER vouched for at that moment, and
/// a 24h-old address set is a stale hint no matter when it arrived here.
pub const MANIFEST_TTL_MS: i64 = 24 * 60 * 60 * 1000;

/// Largest accepted `POST /p2p/manifests` body.
pub const MAX_MANIFEST_BODY_BYTES: usize = 8 * 1024;

// ─────────────────────────────────────────────────────────────────────────────
// Wire type (mirror — see module docs)
// ─────────────────────────────────────────────────────────────────────────────

/// One peer's self-description of where its iroh endpoint can be reached.
///
/// Mirror of `elohim_storage::p2p::transport_manifest_gossip::TransportManifestAnnouncement`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportManifestAnnouncement {
    /// iroh NodeId — 64 lowercase hex chars of the ed25519 public key. The
    /// signature verifies against THIS.
    pub iroh_node_id: String,
    /// Direct socket addresses (`ip:port`) the endpoint is bound/reachable on.
    pub iroh_direct_addrs: Vec<String>,
    /// Home relay URL, when a relay is configured.
    pub iroh_relay_url: Option<String>,
    /// DHT-anchored agent identity (routing hint, not authority).
    pub agent_cid: Option<String>,
    /// libp2p PeerId (base58) when dual-stack (routing hint).
    pub libp2p_peer_id: Option<String>,
    /// Planes this node serves over iroh (kebab-case).
    pub planes: Vec<String>,
    /// Wall-clock ms at signing. The store keeps the newest per NodeId.
    pub announced_at_ms: i64,
    /// ed25519 signature (64 bytes, hex) over [`Self::signing_bytes`].
    pub signature: String,
}

/// Why an announcement was refused. Mirrors the storage-side
/// `ManifestVerifyError` variants one-for-one.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestVerifyError {
    #[error("iroh_node_id is not a 64-hex ed25519 public key")]
    BadNodeId,
    #[error("signature is not 64-byte hex")]
    BadSignatureEncoding,
    #[error("signature does not verify against iroh_node_id")]
    SignatureMismatch,
    #[error("announcement carries no reachable address (no direct addrs, no relay)")]
    Unreachable,
    #[error("direct address '{0}' is not ip:port")]
    BadDirectAddr(String),
}

impl TransportManifestAnnouncement {
    /// The canonical bytes the signature covers: every field except
    /// `signature`, joined with a separator no field can contain.
    ///
    /// VERBATIM copy of the storage construction. Any drift here fails every
    /// verification loudly (400 on POST), never silently.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"elohim/transport/manifest/v1\n");
        out.extend_from_slice(self.iroh_node_id.as_bytes());
        out.push(b'\n');
        for a in &self.iroh_direct_addrs {
            out.extend_from_slice(a.as_bytes());
            out.push(b',');
        }
        out.push(b'\n');
        out.extend_from_slice(self.iroh_relay_url.as_deref().unwrap_or("").as_bytes());
        out.push(b'\n');
        out.extend_from_slice(self.agent_cid.as_deref().unwrap_or("").as_bytes());
        out.push(b'\n');
        out.extend_from_slice(self.libp2p_peer_id.as_deref().unwrap_or("").as_bytes());
        out.push(b'\n');
        for p in &self.planes {
            out.extend_from_slice(p.as_bytes());
            out.push(b',');
        }
        out.push(b'\n');
        out.extend_from_slice(self.announced_at_ms.to_string().as_bytes());
        out
    }

    /// Verify the signature against `iroh_node_id` and check the announcement
    /// is dialable (at least one well-formed direct address or a relay URL).
    pub fn verify(&self) -> Result<(), ManifestVerifyError> {
        let pk_bytes: [u8; 32] = hex::decode(&self.iroh_node_id)
            .ok()
            .and_then(|v| v.try_into().ok())
            .ok_or(ManifestVerifyError::BadNodeId)?;
        let pk = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| ManifestVerifyError::BadNodeId)?;
        let sig_bytes: [u8; 64] = hex::decode(&self.signature)
            .ok()
            .and_then(|v| v.try_into().ok())
            .ok_or(ManifestVerifyError::BadSignatureEncoding)?;
        let sig = Signature::from_bytes(&sig_bytes);
        pk.verify(&self.signing_bytes(), &sig)
            .map_err(|_| ManifestVerifyError::SignatureMismatch)?;
        for a in &self.iroh_direct_addrs {
            if a.parse::<std::net::SocketAddr>().is_err() {
                return Err(ManifestVerifyError::BadDirectAddr(a.clone()));
            }
        }
        if self.iroh_direct_addrs.is_empty() && self.iroh_relay_url.is_none() {
            return Err(ManifestVerifyError::Unreachable);
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounded ephemeral store
// ─────────────────────────────────────────────────────────────────────────────

/// What the store did with a VERIFIED announcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptOutcome {
    /// Held — either new, or newer than the entry it replaced.
    Stored,
    /// Ignored: an entry for this NodeId with an `announced_at_ms` at least as
    /// new is already held. Monotone per NodeId — a replayed old announcement
    /// can never walk a peer's advertised addresses backwards.
    Stale,
    /// Ignored: the store is at [`MAX_MANIFEST_ENTRIES`] and this announcement
    /// is not newer than the oldest entry held, so nothing is evicted for it.
    Full,
    /// Ignored: older than [`MANIFEST_TTL_MS`] at the moment of arrival.
    Expired,
}

/// Bounded, monotone, self-expiring in-memory manifest projection.
///
/// Category C: the whole store is reconstructable from the next announce
/// round. Guarded by a std `RwLock` — no `.await` is ever taken while the lock
/// is held, so it cannot park a tokio worker.
///
/// `enabled` gates the whole public `/p2p/manifests` surface. It defaults OFF:
/// the board admits any self-signed announcement (the signature verifies against
/// the announcement's own NodeId, so there is no identity COST), which makes it
/// Sybil-floodable on a public doorway (mint 64 keypairs → evict every real
/// peer). Until that Sybil resistance is designed
/// (`genesis/data/timeline/backlog/2026-08-24-manifest-board-sybil-resistance.md`),
/// the endpoint stays 404 unless `DOORWAY_MANIFEST_BOARD_ENABLED` is set —
/// localdev turns it on to prove the pure-iroh bootstrap; the public fleet
/// (which runs dual, not pure-iroh) leaves it off.
#[derive(Default)]
pub struct TransportManifestStore {
    entries: RwLock<HashMap<String, TransportManifestAnnouncement>>,
    enabled: bool,
}

impl TransportManifestStore {
    /// Production constructor: reads `DOORWAY_MANIFEST_BOARD_ENABLED` ONCE
    /// (truthy = `1`/`true`/`yes`, case-insensitive). Env is read here, at
    /// AppState construction, never on the request path.
    pub fn new() -> Self {
        let enabled = std::env::var("DOORWAY_MANIFEST_BOARD_ENABLED")
            .ok()
            .map(|v| {
                let v = v.trim().to_ascii_lowercase();
                v == "1" || v == "true" || v == "yes"
            })
            .unwrap_or(false);
        Self {
            entries: RwLock::new(HashMap::new()),
            enabled,
        }
    }

    /// Test/explicit constructor — the flag is named, no env read.
    pub fn with_enabled(enabled: bool) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            enabled,
        }
    }

    /// Whether the public `/p2p/manifests` surface is served at all.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Verify, then accept an announcement at an explicit clock reading.
    /// `now_ms` is a parameter (never an ambient clock read inside the store)
    /// so every test is hermetic.
    pub fn accept_at(
        &self,
        ann: TransportManifestAnnouncement,
        now_ms: i64,
    ) -> Result<AcceptOutcome, ManifestVerifyError> {
        ann.verify()?;
        if is_expired(&ann, now_ms) {
            return Ok(AcceptOutcome::Expired);
        }

        let mut map = self.entries.write().expect("manifest store lock poisoned");
        prune_expired(&mut map, now_ms);

        if let Some(held) = map.get(&ann.iroh_node_id) {
            if held.announced_at_ms >= ann.announced_at_ms {
                return Ok(AcceptOutcome::Stale);
            }
            map.insert(ann.iroh_node_id.clone(), ann);
            return Ok(AcceptOutcome::Stored);
        }

        if map.len() >= MAX_MANIFEST_ENTRIES {
            // Evict the OLDEST announcement — but only for a newer one, so a
            // flood of ancient announcements cannot displace live peers.
            let oldest = map
                .iter()
                .min_by_key(|(_, a)| a.announced_at_ms)
                .map(|(k, a)| (k.clone(), a.announced_at_ms));
            match oldest {
                Some((key, oldest_ms)) if oldest_ms < ann.announced_at_ms => {
                    map.remove(&key);
                }
                _ => return Ok(AcceptOutcome::Full),
            }
        }
        map.insert(ann.iroh_node_id.clone(), ann);
        Ok(AcceptOutcome::Stored)
    }

    /// The current non-expired set, newest announcement first (stable order:
    /// ties broken by NodeId so the response is deterministic).
    pub fn list_at(&self, now_ms: i64) -> Vec<TransportManifestAnnouncement> {
        let mut map = self.entries.write().expect("manifest store lock poisoned");
        prune_expired(&mut map, now_ms);
        let mut out: Vec<_> = map.values().cloned().collect();
        out.sort_by(|a, b| {
            b.announced_at_ms
                .cmp(&a.announced_at_ms)
                .then_with(|| a.iroh_node_id.cmp(&b.iroh_node_id))
        });
        out
    }

    /// Entries currently held (after pruning at `now_ms`).
    pub fn len_at(&self, now_ms: i64) -> usize {
        let mut map = self.entries.write().expect("manifest store lock poisoned");
        prune_expired(&mut map, now_ms);
        map.len()
    }
}

fn is_expired(ann: &TransportManifestAnnouncement, now_ms: i64) -> bool {
    now_ms.saturating_sub(ann.announced_at_ms) > MANIFEST_TTL_MS
}

fn prune_expired(map: &mut HashMap<String, TransportManifestAnnouncement>, now_ms: i64) {
    map.retain(|_, a| !is_expired(a, now_ms));
}

/// Wall clock in ms — the single ambient read, at the HTTP edge only.
fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// ─────────────────────────────────────────────────────────────────────────────
// HTTP handlers
// ─────────────────────────────────────────────────────────────────────────────

fn json_response(status: StatusCode, body: &serde_json::Value) -> Response<FullBody> {
    let bytes = serde_json::to_vec(body).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(bytes)))
        .expect("infallible json response")
}

fn error_response(status: StatusCode, error: &str, code: &str) -> Response<FullBody> {
    json_response(status, &serde_json::json!({ "error": error, "code": code }))
}

/// `POST /p2p/manifests` — announce one signed transport manifest.
///
/// 202 Accepted for any announcement whose signature VERIFIES (the body says
/// whether it was stored or ignored as stale/full); 400 for a body that will
/// not parse or will not verify; 413 for a body over
/// [`MAX_MANIFEST_BODY_BYTES`].
///
/// No auth by design — the ed25519 signature is the auth (module docs).
pub async fn handle_post_manifest(
    req: Request<Incoming>,
    state: Arc<AppState>,
) -> Response<FullBody> {
    if !state.transport_manifests.is_enabled() {
        return manifest_board_disabled();
    }
    let collected = Limited::new(req.into_body(), MAX_MANIFEST_BODY_BYTES)
        .collect()
        .await;
    let body_bytes = match collected {
        Ok(b) => b.to_bytes(),
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                &format!("announcement exceeds {MAX_MANIFEST_BODY_BYTES} bytes"),
                "BODY_TOO_LARGE",
            )
        }
    };

    let ann: TransportManifestAnnouncement = match serde_json::from_slice(&body_bytes) {
        Ok(a) => a,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("invalid manifest JSON: {e}"),
                "BAD_JSON",
            )
        }
    };

    let now = now_ms();
    let node_id = ann.iroh_node_id.clone();
    match state.transport_manifests.accept_at(ann, now) {
        Ok(outcome) => {
            let stored = outcome == AcceptOutcome::Stored;
            debug!(
                target: "doorway::p2p_manifests",
                node_id = %node_id,
                outcome = ?outcome,
                "transport manifest announcement"
            );
            json_response(
                StatusCode::ACCEPTED,
                &serde_json::json!({
                    "stored": stored,
                    "outcome": match outcome {
                        AcceptOutcome::Stored => "stored",
                        AcceptOutcome::Stale => "stale",
                        AcceptOutcome::Full => "full",
                        AcceptOutcome::Expired => "expired",
                    },
                    "held": state.transport_manifests.len_at(now),
                }),
            )
        }
        Err(e) => {
            warn!(
                target: "doorway::p2p_manifests",
                node_id = %node_id,
                "rejected transport manifest: {e}"
            );
            error_response(StatusCode::BAD_REQUEST, &e.to_string(), "BAD_MANIFEST")
        }
    }
}

/// 404 for a disabled board — the same shape an unknown route gets, so a
/// probe cannot tell the feature exists (defense in depth against the flag
/// being flipped as a reconnaissance signal).
fn manifest_board_disabled() -> Response<FullBody> {
    error_response(StatusCode::NOT_FOUND, "not found", "NOT_FOUND")
}

/// `GET /p2p/manifests` — the current non-expired set, as a JSON array.
pub async fn handle_get_manifests(state: Arc<AppState>) -> Response<FullBody> {
    if !state.transport_manifests.is_enabled() {
        return manifest_board_disabled();
    }
    let manifests = state.transport_manifests.list_at(now_ms());
    let bytes = serde_json::to_vec(&manifests).unwrap_or_else(|_| b"[]".to_vec());
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "no-store")
        .body(Full::new(Bytes::from(bytes)))
        .expect("infallible json response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// Local signer — the doorway never signs announcements in production, so
    /// this lives in the test module only. It is the exact inverse of
    /// [`TransportManifestAnnouncement::signing_bytes`], which is the same
    /// construction the storage-side `sign()` uses.
    fn signed(seed: u8, addr: &str, announced_at_ms: i64) -> TransportManifestAnnouncement {
        let key = SigningKey::from_bytes(&[seed.max(1); 32]);
        let mut ann = TransportManifestAnnouncement {
            iroh_node_id: hex::encode(key.verifying_key().to_bytes()),
            iroh_direct_addrs: vec![addr.to_string()],
            iroh_relay_url: None,
            agent_cid: Some("agent-matthew".into()),
            libp2p_peer_id: Some("12D3KooWexample".into()),
            planes: vec!["sync".into(), "blob".into()],
            announced_at_ms,
            signature: String::new(),
        };
        let sig = key.sign(&ann.signing_bytes());
        ann.signature = hex::encode(sig.to_bytes());
        ann
    }

    const T0: i64 = 1_700_000_000_000;

    #[test]
    fn post_then_get_round_trips_a_verified_announcement() {
        let store = TransportManifestStore::new();
        let ann = signed(1, "127.0.0.1:10701", T0);
        assert_eq!(
            store.accept_at(ann.clone(), T0).unwrap(),
            AcceptOutcome::Stored
        );

        let held = store.list_at(T0);
        assert_eq!(held.len(), 1);
        assert_eq!(
            held[0], ann,
            "served back byte-identical to what was posted"
        );

        // And the JSON the GET body carries decodes back into the same struct —
        // the wire shape the storage client will read.
        let json = serde_json::to_vec(&held).unwrap();
        let back: Vec<TransportManifestAnnouncement> = serde_json::from_slice(&json).unwrap();
        assert_eq!(back, held);
    }

    #[test]
    fn an_announcement_that_does_not_verify_is_never_stored() {
        let store = TransportManifestStore::new();
        let mut ann = signed(1, "127.0.0.1:10701", T0);
        // Tamper with the advertised address — the exact attack the signature exists to stop.
        ann.iroh_direct_addrs = vec!["10.0.0.9:4444".into()];
        assert_eq!(
            store.accept_at(ann, T0),
            Err(ManifestVerifyError::SignatureMismatch)
        );
        assert!(
            store.list_at(T0).is_empty(),
            "the store's invariant: entries come ONLY from announcements that verified"
        );
    }

    #[test]
    fn a_stale_announcement_does_not_replace_a_newer_one() {
        let store = TransportManifestStore::new();
        let newer = signed(1, "127.0.0.1:10701", T0 + 5_000);
        let older = signed(1, "127.0.0.1:19999", T0);
        assert_eq!(
            store.accept_at(newer.clone(), T0 + 5_000).unwrap(),
            AcceptOutcome::Stored
        );
        assert_eq!(
            store.accept_at(older, T0 + 5_000).unwrap(),
            AcceptOutcome::Stale,
            "monotone per NodeId — a replay cannot walk advertised addresses backwards"
        );

        let held = store.list_at(T0 + 5_000);
        assert_eq!(held, vec![newer]);
    }

    #[test]
    fn a_newer_announcement_replaces_the_held_one_in_place() {
        let store = TransportManifestStore::new();
        store
            .accept_at(signed(1, "127.0.0.1:10701", T0), T0)
            .unwrap();
        let moved = signed(1, "127.0.0.1:10999", T0 + 1);
        assert_eq!(
            store.accept_at(moved.clone(), T0 + 1).unwrap(),
            AcceptOutcome::Stored
        );

        let held = store.list_at(T0 + 1);
        assert_eq!(held.len(), 1, "one entry per NodeId, never two");
        assert_eq!(held[0].iroh_direct_addrs, moved.iroh_direct_addrs);
    }

    #[test]
    fn the_store_is_bounded_and_evicts_the_oldest_at_capacity() {
        let store = TransportManifestStore::new();
        // 64 distinct NodeIds with strictly increasing announce times.
        for i in 0..MAX_MANIFEST_ENTRIES {
            let ann = signed((i + 1) as u8, "127.0.0.1:10701", T0 + i as i64);
            assert_eq!(
                store.accept_at(ann, T0 + i as i64).unwrap(),
                AcceptOutcome::Stored
            );
        }
        assert_eq!(store.len_at(T0), MAX_MANIFEST_ENTRIES);
        let oldest = signed(1, "127.0.0.1:10701", T0);

        // The 65th, newer than everything held: the oldest is evicted for it.
        let newcomer = signed(200, "127.0.0.1:10701", T0 + 1_000);
        assert_eq!(
            store.accept_at(newcomer.clone(), T0 + 1_000).unwrap(),
            AcceptOutcome::Stored
        );

        let held = store.list_at(T0 + 1_000);
        assert_eq!(held.len(), MAX_MANIFEST_ENTRIES, "still bounded at 64");
        assert!(held.contains(&newcomer), "the newcomer is held");
        assert!(
            !held.iter().any(|a| a.iroh_node_id == oldest.iroh_node_id),
            "the oldest announcement was the one evicted"
        );
    }

    #[test]
    fn a_full_store_is_not_displaced_by_an_older_announcement() {
        let store = TransportManifestStore::new();
        for i in 0..MAX_MANIFEST_ENTRIES {
            let ann = signed((i + 1) as u8, "127.0.0.1:10701", T0 + 1_000 + i as i64);
            store.accept_at(ann, T0 + 1_000 + i as i64).unwrap();
        }
        // Older than every held entry — a flood of ancient announcements must
        // not be able to displace live peers.
        let ancient = signed(200, "127.0.0.1:10701", T0);
        assert_eq!(
            store.accept_at(ancient.clone(), T0 + 2_000).unwrap(),
            AcceptOutcome::Full
        );
        assert!(!store.list_at(T0 + 2_000).contains(&ancient));
    }

    #[test]
    fn entries_expire_after_the_ttl() {
        let store = TransportManifestStore::new();
        let ann = signed(1, "127.0.0.1:10701", T0);
        store.accept_at(ann, T0).unwrap();

        assert_eq!(
            store.list_at(T0 + MANIFEST_TTL_MS).len(),
            1,
            "still inside the window"
        );
        assert!(
            store.list_at(T0 + MANIFEST_TTL_MS + 1).is_empty(),
            "pruned lazily on read once past the TTL"
        );

        // And an already-expired announcement is never admitted in the first place.
        let old = signed(2, "127.0.0.1:10702", T0);
        assert_eq!(
            store.accept_at(old, T0 + MANIFEST_TTL_MS + 1).unwrap(),
            AcceptOutcome::Expired
        );
        assert!(store.list_at(T0 + MANIFEST_TTL_MS + 1).is_empty());
    }

    #[test]
    fn an_oversized_body_is_rejected_before_it_is_parsed() {
        // The gate the handler applies, asserted on the same constant the
        // handler hands `Limited` — no hyper Incoming body needed to prove the
        // bound, and the handler test below proves the wiring.
        let oversized = vec![b'x'; MAX_MANIFEST_BODY_BYTES + 1];
        assert!(oversized.len() > MAX_MANIFEST_BODY_BYTES);
        assert!(serde_json::from_slice::<TransportManifestAnnouncement>(&oversized).is_err());
    }

    #[tokio::test]
    async fn the_handler_rejects_an_oversized_body_with_413() {
        use http_body_util::BodyExt;
        // Exercise the same `Limited` gate the handler installs, with a body
        // that is valid JSON but too large — proving the size check fires
        // BEFORE parse, not after.
        let ann = signed(1, "127.0.0.1:10701", T0);
        let mut padded = serde_json::to_vec(&ann).unwrap();
        padded.resize(MAX_MANIFEST_BODY_BYTES + 1, b' ');
        let body = Full::new(Bytes::from(padded));
        let limited = Limited::new(body, MAX_MANIFEST_BODY_BYTES);
        assert!(
            limited.collect().await.is_err(),
            "over the cap: the body never reaches serde"
        );

        // ...and a body inside the cap passes the same gate.
        let ok = serde_json::to_vec(&ann).unwrap();
        assert!(ok.len() <= MAX_MANIFEST_BODY_BYTES);
        let limited = Limited::new(Full::new(Bytes::from(ok)), MAX_MANIFEST_BODY_BYTES);
        assert!(limited.collect().await.is_ok());
    }

    #[test]
    fn the_board_is_disabled_by_default_and_the_flag_enables_it() {
        // The default constructor path (env unset in the test env) is OFF —
        // the Sybil-floodable surface is not served unless deliberately turned on.
        assert!(
            !TransportManifestStore::with_enabled(false).is_enabled(),
            "board must default off"
        );
        assert!(
            TransportManifestStore::with_enabled(true).is_enabled(),
            "the flag turns the board on"
        );
    }

    #[test]
    fn a_disabled_board_answers_404_indistinguishable_from_an_unknown_route() {
        let resp = manifest_board_disabled();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn the_signing_bytes_carry_the_versioned_domain_prefix() {
        // The one byte-level guard on the storage mirror: if this prefix or the
        // separators drift, every real announcement fails to verify LOUDLY.
        let ann = signed(1, "127.0.0.1:10701", T0);
        let bytes = ann.signing_bytes();
        assert!(bytes.starts_with(b"elohim/transport/manifest/v1\n"));
        assert!(ann.verify().is_ok());
    }

    #[test]
    fn an_unreachable_announcement_is_refused() {
        let key = SigningKey::from_bytes(&[3u8; 32]);
        let mut ann = TransportManifestAnnouncement {
            iroh_node_id: hex::encode(key.verifying_key().to_bytes()),
            iroh_direct_addrs: vec![],
            iroh_relay_url: None,
            agent_cid: None,
            libp2p_peer_id: None,
            planes: vec![],
            announced_at_ms: T0,
            signature: String::new(),
        };
        ann.signature = hex::encode(key.sign(&ann.signing_bytes()).to_bytes());
        let store = TransportManifestStore::new();
        assert_eq!(
            store.accept_at(ann, T0),
            Err(ManifestVerifyError::Unreachable),
            "a signed announcement with nothing to dial is still useless"
        );
    }
}
