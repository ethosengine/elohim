use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    /// Optional content hash for cache-key derivation.
    pub content_hash: Option<String>,
}

/// Whose authority a [`DataFetcher`]'s outbound requests are made under.
///
/// This is a **security-relevant declaration, not a hint.** `elohim-render`
/// reuses one V8 isolate across sequential renders and swaps only the fetcher
/// between them (see [`crate::runtime::JsRuntime::set_fetcher`]). JS global
/// state written during one render — module-scope caches, Angular DI
/// singletons, anything stashed on `globalThis` — is still live during the
/// next. Whether that reuse crosses a trust boundary depends **entirely** on
/// this value.
///
/// # Why `trust_scope` has no default implementation
///
/// A defaulted `Ambient` would silently mis-declare exactly the fetcher that
/// matters: the one somebody adds later that carries a user credential. Making
/// the method required means a new `DataFetcher` impl **cannot compile** without
/// consciously answering "whose authority is this?" — that compile error is the
/// tripwire. Do not add a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetcherTrust {
    /// Requests carry only the rendering peer's own service-to-service
    /// identity. Every render through such a fetcher reaches the same data, so
    /// isolate reuse across renders leaks nothing that was not already shared.
    Ambient,
    /// Requests carry an end-user principal's credential (session cookie,
    /// bearer token, steward attestation). Data reachable through this fetcher
    /// is reachable *because of who asked* — so any residue it leaves in a
    /// reused isolate is that principal's data sitting in the next principal's
    /// render.
    ///
    /// A decorator that wraps another fetcher MUST delegate to its inner
    /// fetcher's scope rather than returning [`FetcherTrust::Ambient`];
    /// a wrapper that answers for itself launders a `Principal` into an
    /// `Ambient` and defeats the whole mechanism.
    Principal,
}

impl FetcherTrust {
    /// Whether a render performed under this trust scope may share a cache
    /// entry with other requests.
    ///
    /// Only [`FetcherTrust::Ambient`] is shareable. A [`FetcherTrust::Principal`]
    /// render resolved reach-gated content **because of who asked**, so its HTML
    /// is that principal's data: it must never be stored under a key other
    /// principals can hit, and a principal must never be served an entry
    /// produced for someone else.
    ///
    /// # Why this lives here and not in a host
    ///
    /// Every render host — the doorway web2 projection, a peer runtime serving
    /// its own content through `elohim-storage`'s `ssr` feature, or any future
    /// host — must make the SAME decision. Rendering is a capability of the peer
    /// runtime; doorway is one optional projection of it, never a required
    /// component of the render path. So the *decision* belongs beside the trust
    /// vocabulary in the shared render layer, and each host's cache adapter only
    /// calls it. A host that re-derives this rule locally is how the two
    /// diverge. See
    /// `genesis/docs/superpowers/specs/2026-07-30-render-delivery-manifest-adapter-design.md`
    /// §3e/§6 (doorway and storage-ssr converge on symmetric thin hosts).
    ///
    /// The render cache key is deliberately NOT extended with a credential
    /// fingerprint instead: caching reach-gated HTML at all is a liability, and
    /// per-principal keys multiply cache cardinality for a short-TTL win.
    /// Skipping the cache in both directions is the correct shape.
    pub fn is_cache_shareable(self) -> bool {
        matches!(self, FetcherTrust::Ambient)
    }
}

#[async_trait]
pub trait DataFetcher: Send + Sync {
    async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse>;

    /// Declare whose authority this fetcher's requests are made under.
    ///
    /// See [`FetcherTrust`] for the contract and for why this method is
    /// deliberately **not** defaulted.
    fn trust_scope(&self) -> FetcherTrust;
}
