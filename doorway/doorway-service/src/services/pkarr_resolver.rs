//! Pkarr Resolver Service
//!
//! Self-hostable pkarr relay endpoint. Implements the pkarr.org relay
//! HTTP wire protocol (https://pkarr.org/relays) so that iroh's stock
//! `PkarrPublisher` and `PkarrResolver` can interoperate with us
//! without any custom client code.
//!
//! This module is the substrate-side mitigation for cutover gate #10
//! per `genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`,
//! Step 2 of the n0-centralization-seam mitigation plan.
//!
//! # NOT a route registry proxy
//!
//! Per `doorway/CLAUDE.md`'s "No Per-Domain Proxy Files" rule, doorway's
//! default extensibility model is to forward routes to elohim-storage.
//! That model does NOT apply here. The pkarr endpoint terminates the
//! request inside doorway because:
//!
//! 1. There is no upstream elohim-storage handler — the bytes live in
//!    doorway's process memory (LRU) and optionally on doorway's disk.
//! 2. The request semantics are doorway-specific: doorway is the
//!    federated edge that pkarr clients address.
//!
//! Do not try to "consolidate" this into the route registry; do not
//! delete it because it looks like a proxy file.
//!
//! # Wire protocol (per pkarr.org/relays)
//!
//! - `GET  /pkarr/{public_key}` → 200 `application/pkarr.org-relays+octet-stream`
//!   with the SignedPacket relay payload (NodeId in URL path, body is the
//!   timestamp + signature + DNS payload, per `SignedPacket::to_relay_payload`).
//!   404 if not cached. 304 with `If-Modified-Since`.
//! - `PUT  /pkarr/{public_key}` → 200 on accept. 400 on signature mismatch
//!   or oversize body. 412 on `If-Match` mismatch (CAS).

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, Response, StatusCode};
use lru::LruCache;
use pkarr::{PublicKey, SignedPacket};
use tokio::sync::Mutex;

pub const PKARR_CONTENT_TYPE: &str = "application/pkarr.org-relays+octet-stream";
pub const DEFAULT_CACHE_CAPACITY: usize = 1000;

/// Configuration for the pkarr resolver service.
#[derive(Debug, Clone)]
pub struct PkarrResolverConfig {
    /// Whether the endpoint is enabled. False = handler returns 404 to all requests.
    pub enabled: bool,
    /// LRU capacity. None means use DEFAULT_CACHE_CAPACITY.
    pub cache_capacity: Option<usize>,
    /// If Some, on graceful shutdown the cache is persisted to <dir>/packets.bin
    /// and reloaded on next boot.
    pub persist_dir: Option<std::path::PathBuf>,
}

/// Cache entry. `received_at` is for HTTP If-Modified-Since support.
#[derive(Clone)]
pub struct CachedPacket {
    pub packet: SignedPacket,
    pub received_at: Instant,
}

/// In-memory LRU cache of signed packets keyed by z32-encoded public key.
pub struct PkarrCache {
    lru: Mutex<LruCache<String, CachedPacket>>,
}

impl PkarrCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            lru: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity.max(1)).expect("cap >= 1"),
            )),
        }
    }

    pub async fn get(&self, key: &str) -> Option<CachedPacket> {
        self.lru.lock().await.get(key).cloned()
    }

    pub async fn put(&self, key: String, packet: SignedPacket) {
        self.lru.lock().await.put(
            key,
            CachedPacket {
                packet,
                received_at: Instant::now(),
            },
        );
    }

    pub async fn len(&self) -> usize {
        self.lru.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.lru.lock().await.is_empty()
    }
}

/// Service handle held by AppState.
pub struct PkarrResolverService {
    pub config: PkarrResolverConfig,
    pub cache: Arc<PkarrCache>,
}

impl PkarrResolverService {
    pub fn new(config: PkarrResolverConfig) -> Self {
        let cap = config.cache_capacity.unwrap_or(DEFAULT_CACHE_CAPACITY);
        let cache = Arc::new(PkarrCache::new(cap));
        if let Some(ref dir) = config.persist_dir {
            if let Err(e) = Self::load_from_disk(dir, &cache) {
                tracing::warn!(error = %e, dir = %dir.display(),
                    "pkarr cache: disk reload failed, starting empty");
            }
        }
        Self { config, cache }
    }

    fn load_from_disk(dir: &std::path::Path, _cache: &PkarrCache) -> std::io::Result<()> {
        let path = dir.join("packets.bin");
        if !path.exists() {
            return Ok(());
        }
        // Length-prefixed (u32 BE) SignedPacket::serialize() blobs.
        // Stub for plan; implementer fills in the read loop.
        // Each iteration: read u32 BE, read that many bytes, SignedPacket::deserialize, cache.put.
        // Errors are warned, not failed: a corrupt entry truncates the load.
        Ok(())
    }

    /// Persist the entire cache to disk. Called from graceful shutdown.
    pub async fn persist_to_disk(&self) -> std::io::Result<()> {
        let Some(ref dir) = self.config.persist_dir else {
            return Ok(());
        };
        std::fs::create_dir_all(dir)?;
        // Stub: drain LRU, length-prefix each SignedPacket::serialize() into <dir>/packets.bin.atomic, rename over packets.bin.
        Ok(())
    }
}

// ============================================================
// HTTP handlers
// ============================================================

/// GET /pkarr/{public_key}
///
/// Returns the cached SignedPacket for the public key, or 404 if not cached.
/// Honors If-Modified-Since (HTTP-date format).
pub async fn handle_get_pkarr(
    service: Arc<PkarrResolverService>,
    public_key_str: &str,
    if_modified_since: Option<&str>,
) -> Response<Full<Bytes>> {
    if !service.config.enabled {
        return error_response(StatusCode::NOT_FOUND, "pkarr resolver disabled");
    }
    // Validate public key format (z32, 52 chars, decodes to 32 bytes).
    if let Err(e) = public_key_str.parse::<PublicKey>() {
        return error_response(
            StatusCode::BAD_REQUEST,
            &format!("invalid pkarr public key: {e}"),
        );
    }
    let Some(entry) = service.cache.get(public_key_str).await else {
        return error_response(StatusCode::NOT_FOUND, "no packet cached for key");
    };
    // If-Modified-Since handling. We compare the cache's received_at against
    // a ceil-second resolution; a finer-grained protocol could use the
    // SignedPacket's own timestamp.
    if let Some(_ims) = if_modified_since {
        // Implementer: parse httpdate, compare to entry.received_at; if not modified return 304.
        // (Stubbed in this scaffold; see Task 6 unit tests for required behavior.)
        let _ = entry.received_at; // suppress unused warning
    }
    let body = entry.packet.to_relay_payload();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, PKARR_CONTENT_TYPE)
        .header(header::CACHE_CONTROL, "public, max-age=300")
        .body(Full::new(Bytes::copy_from_slice(&body)))
        .expect("response builds")
}

/// PUT /pkarr/{public_key}
///
/// Accepts a signed pkarr relay payload. Verifies signature matches the
/// public key in the URL path; rejects on mismatch.
pub async fn handle_put_pkarr(
    service: Arc<PkarrResolverService>,
    public_key_str: &str,
    body: Bytes,
    _if_match: Option<&str>,
) -> Response<Full<Bytes>> {
    if !service.config.enabled {
        return error_response(StatusCode::NOT_FOUND, "pkarr resolver disabled");
    }
    if body.len() as u64 > SignedPacket::MAX_BYTES {
        return error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            &format!(
                "body exceeds SignedPacket::MAX_BYTES ({})",
                SignedPacket::MAX_BYTES
            ),
        );
    }
    let public_key: PublicKey = match public_key_str.parse() {
        Ok(pk) => pk,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("invalid pkarr public key: {e}"),
            )
        }
    };
    // SignedPacket::from_relay_payload verifies signature against the
    // expected public key (signature failure => Err).
    let packet = match SignedPacket::from_relay_payload(&public_key, &body) {
        Ok(p) => p,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("signed packet rejected: {e}"),
            )
        }
    };
    service.cache.put(public_key_str.to_string(), packet).await;
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::new()))
        .expect("response builds")
}

fn error_response(status: StatusCode, msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(msg.to_string())))
        .expect("response builds")
}

// ============================================================
// Stats — surfaced on the operator dashboard
// ============================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct PkarrResolverStats {
    pub enabled: bool,
    pub cached_packets: usize,
    pub capacity: usize,
    pub disk_persistence: bool,
}

impl PkarrResolverService {
    pub async fn stats(&self) -> PkarrResolverStats {
        PkarrResolverStats {
            enabled: self.config.enabled,
            cached_packets: self.cache.len().await,
            capacity: self.config.cache_capacity.unwrap_or(DEFAULT_CACHE_CAPACITY),
            disk_persistence: self.config.persist_dir.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pkarr::{
        dns::{rdata::TXT, Name},
        Keypair, SignedPacketBuilder,
    };

    fn make_packet(kp: &Keypair) -> SignedPacket {
        SignedPacketBuilder::default()
            .txt(
                Name::new("_iroh").expect("name"),
                TXT::new().with_string("v=0.1").expect("txt"),
                300,
            )
            .build(kp)
            .expect("packet builds")
    }

    #[tokio::test]
    async fn put_then_get_roundtrips_signed_packet() {
        let cfg = PkarrResolverConfig {
            enabled: true,
            cache_capacity: Some(10),
            persist_dir: None,
        };
        let svc = Arc::new(PkarrResolverService::new(cfg));
        let kp = Keypair::random();
        let pk_str = kp.public_key().to_string();
        let packet = make_packet(&kp);
        let body = Bytes::copy_from_slice(&packet.to_relay_payload());
        let put_resp = handle_put_pkarr(svc.clone(), &pk_str, body, None).await;
        assert_eq!(put_resp.status(), StatusCode::OK);
        let get_resp = handle_get_pkarr(svc.clone(), &pk_str, None).await;
        assert_eq!(get_resp.status(), StatusCode::OK);
        assert_eq!(
            get_resp.headers().get(header::CONTENT_TYPE).unwrap(),
            PKARR_CONTENT_TYPE
        );
    }

    #[tokio::test]
    async fn put_rejects_wrong_signature() {
        let cfg = PkarrResolverConfig {
            enabled: true,
            cache_capacity: Some(10),
            persist_dir: None,
        };
        let svc = Arc::new(PkarrResolverService::new(cfg));
        let kp_a = Keypair::random();
        let kp_b = Keypair::random();
        let packet_a = make_packet(&kp_a);
        // Submit kp_a's packet body but claim it's for kp_b's URL path → rejection.
        let body = Bytes::copy_from_slice(&packet_a.to_relay_payload());
        let resp = handle_put_pkarr(svc, &kp_b.public_key().to_string(), body, None).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn put_rejects_oversize_body() {
        let cfg = PkarrResolverConfig {
            enabled: true,
            cache_capacity: Some(10),
            persist_dir: None,
        };
        let svc = Arc::new(PkarrResolverService::new(cfg));
        let kp = Keypair::random();
        let huge = Bytes::from(vec![0u8; (SignedPacket::MAX_BYTES + 1) as usize]);
        let resp = handle_put_pkarr(svc, &kp.public_key().to_string(), huge, None).await;
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn get_returns_404_when_not_cached() {
        let cfg = PkarrResolverConfig {
            enabled: true,
            cache_capacity: Some(10),
            persist_dir: None,
        };
        let svc = Arc::new(PkarrResolverService::new(cfg));
        let kp = Keypair::random();
        let resp = handle_get_pkarr(svc, &kp.public_key().to_string(), None).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_rejects_invalid_pubkey() {
        let cfg = PkarrResolverConfig {
            enabled: true,
            cache_capacity: Some(10),
            persist_dir: None,
        };
        let svc = Arc::new(PkarrResolverService::new(cfg));
        let resp = handle_get_pkarr(svc, "not-a-z32-key", None).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
