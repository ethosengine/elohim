//! Iroh-side P2P configuration.
//!
//! Disjoint by design from the libp2p path:
//! - libp2p uses `<storage_dir>/identity.key` + `<storage_dir>/blobs/`
//! - iroh uses `<storage_dir>/iroh.key` + `<storage_dir>/blobs_iroh/`
//!
//! Phase 1 sets up the config surface; later phases add ALPN registration,
//! discovery options, etc.

use std::path::{Path, PathBuf};

use url::Url;

/// One discovery resolver entry. Maps to the federation manifest's
/// `DiscoveryResolver` shape (see
/// `elohim/sdk/schemas/v1/manifests/discovery-resolvers.schema.json`).
#[derive(Debug, Clone)]
pub struct DiscoveryResolverConfig {
    /// Base HTTPS URL of the pkarr relay endpoint (no trailing /<key>).
    /// Example: https://doorway.elohim.host/pkarr
    pub url: Url,
    /// Provenance, for logs + dashboard. Not consulted by the wire protocol.
    pub kind: DiscoveryResolverKind,
}

/// Provenance of a discovery resolver. Audit + UI hint only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryResolverKind {
    N0Default,
    OperatorSelfHosted,
    FederatedPeer,
    ThirdParty,
}

impl DiscoveryResolverConfig {
    /// The current n0 production resolver. Kept as a default so peers
    /// without explicit configuration retain today's behavior.
    pub fn n0_default() -> Self {
        Self {
            url: "https://dns.iroh.link/pkarr"
                .parse()
                .expect("static url is valid"),
            kind: DiscoveryResolverKind::N0Default,
        }
    }
}

/// Configuration for the iroh-based P2P node.
#[derive(Debug, Clone)]
pub struct IrohConfig {
    /// Directory holding the iroh-blobs store. BLAKE3-keyed; disjoint from
    /// the SHA256-keyed legacy `<storage_dir>/blobs/`.
    pub blobs_dir: PathBuf,

    /// Path to the persisted iroh secret key. Generated on first run.
    /// Distinct from the libp2p `<storage_dir>/identity.key`.
    pub secret_key_path: PathBuf,

    /// The self-hosted iroh-relay this node uses for NAT traversal, e.g.
    /// `https://relay.elohim.host`. This is a DIFFERENT config surface from
    /// the Holochain conductor's `network.relay_url` (kitsune2/transport-iroh)
    /// even though the two may point at the same hostname on the same box —
    /// never conflate the layers (see the Wave-2 relay-sovereignty design,
    /// D1 "Scope of D1 — both layers"). `None` means no relay is configured;
    /// `build_endpoint` then falls back to `use_n0_relays` (default `false`)
    /// or, failing that, disables relaying entirely.
    pub relay_url: Option<String>,

    /// Whether to fall back to n0's hosted relay infrastructure for NAT
    /// traversal when `relay_url` is unset. **Defaults to `false` — this
    /// node never phones n0's relay fleet unless an operator opts in.**
    /// n0 remains available as an explicit opt-in for dev experiments
    /// (`ELOHIM_IROH_USE_N0_RELAYS=true`), never as a silent default.
    pub use_n0_relays: bool,

    /// Whether to enable n0's hosted DNS-based peer discovery (Phase 10).
    ///
    /// Deprecated: use `discovery_resolvers` instead. When `true` and
    /// `discovery_resolvers` is empty, a synthetic `DiscoveryResolverConfig::n0_default()`
    /// is used. When `discovery_resolvers` is non-empty, this field is ignored.
    ///
    /// **Defaults to `false`.** Publishing this node's endpoint addresses to
    /// n0's pkarr DNS is a sovereignty concern (Wave-2 relay-sovereignty
    /// design D1), not just a relay one — never a silent default. n0 remains
    /// available as an explicit opt-in for dev experiments
    /// (`ELOHIM_IROH_USE_N0_DISCOVERY=true`).
    pub use_n0_discovery: bool,

    /// Pkarr discovery resolvers iroh queries (and publishes to). Empty
    /// list means "no discovery" — peer addresses must be exchanged
    /// out-of-band via `Endpoint::add_node_addr` (this is what tests do).
    ///
    /// When non-empty, each entry becomes a (PkarrPublisher + PkarrResolver)
    /// pair registered on the Endpoint via `add_discovery`, wrapped by
    /// iroh's `ConcurrentDiscovery` for parallel querying.
    ///
    /// **Defaults to empty.** n0's resolver is never wired in by default;
    /// operators opt in explicitly (see `use_n0_discovery`) or configure
    /// their own operator-self-hosted resolver.
    ///
    /// See genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
    /// (cutover gate #10) for the operator-self-hostable rationale.
    pub discovery_resolvers: Vec<DiscoveryResolverConfig>,
}

impl IrohConfig {
    /// Construct a default config rooted at `storage_dir`. Both file paths
    /// are intentionally disjoint from the libp2p path's filenames so the
    /// two stacks can coexist on a single storage volume during cutover.
    ///
    /// **Sovereign by default**: no relay, no discovery, no n0 dependency of
    /// any kind. Wave-2 relay-sovereignty design D1 forbids a silent n0
    /// default at this layer — the fallback posture is LAN/direct-only,
    /// degraded reach, which is an accepted trade-off; a silent n0 phone-home
    /// is not. Use `apply_env` (or the `ELOHIM_IROH_*` env vars it reads) to
    /// opt into `relay_url` or the n0 fallbacks explicitly.
    pub fn from_storage_dir(storage_dir: &Path) -> Self {
        Self {
            blobs_dir: storage_dir.join("blobs_iroh"),
            secret_key_path: storage_dir.join("iroh.key"),
            relay_url: None,
            use_n0_relays: false,
            use_n0_discovery: false,
            discovery_resolvers: vec![],
        }
    }

    /// Construct a default config, then apply the `ELOHIM_IROH_*` env-var
    /// overrides. This is the constructor production call sites should use
    /// (`main.rs`); `from_storage_dir` alone stays available for tests that
    /// want the bare sovereign defaults with no env interference.
    pub fn from_storage_dir_and_env(storage_dir: &Path) -> Self {
        let mut cfg = Self::from_storage_dir(storage_dir);
        cfg.apply_env();
        cfg
    }

    /// Apply `ELOHIM_IROH_*` environment overrides in place. Invalid values
    /// are logged and ignored (same posture as `ELOHIM_TRANSPORT_BACKEND` in
    /// `main.rs`) — a typo never silently reverts to an n0 default, it just
    /// leaves the sovereign default untouched.
    ///
    /// - `ELOHIM_IROH_RELAY_URL` — sets `relay_url` (any non-empty string;
    ///   parse/validity is enforced at `build_endpoint` time, not here, so a
    ///   bad URL fails startup loudly instead of silently here).
    /// - `ELOHIM_IROH_USE_N0_RELAYS=true` — opt in to the n0 relay fallback.
    /// - `ELOHIM_IROH_USE_N0_DISCOVERY=true` — opt in to n0 pkarr discovery
    ///   (re-adds `DiscoveryResolverConfig::n0_default()` when
    ///   `discovery_resolvers` is otherwise empty, matching the historical
    ///   back-compat meaning of this flag).
    pub fn apply_env(&mut self) {
        if let Ok(v) = std::env::var("ELOHIM_IROH_RELAY_URL") {
            if v.trim().is_empty() {
                tracing::warn!("ignoring empty ELOHIM_IROH_RELAY_URL");
            } else {
                self.relay_url = Some(v);
            }
        }

        if let Ok(v) = std::env::var("ELOHIM_IROH_USE_N0_RELAYS") {
            match v.trim().to_ascii_lowercase().parse::<bool>() {
                Ok(b) => self.use_n0_relays = b,
                Err(_) => tracing::warn!(value = %v, "ignoring invalid ELOHIM_IROH_USE_N0_RELAYS"),
            }
        }

        if let Ok(v) = std::env::var("ELOHIM_IROH_USE_N0_DISCOVERY") {
            match v.trim().to_ascii_lowercase().parse::<bool>() {
                Ok(b) => {
                    self.use_n0_discovery = b;
                    if b && self.discovery_resolvers.is_empty() {
                        self.discovery_resolvers = vec![DiscoveryResolverConfig::n0_default()];
                    }
                }
                Err(_) => {
                    tracing::warn!(value = %v, "ignoring invalid ELOHIM_IROH_USE_N0_DISCOVERY")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_storage_dir_uses_disjoint_blob_dir() {
        let cfg = IrohConfig::from_storage_dir(Path::new("/var/elohim"));
        assert_eq!(cfg.blobs_dir, Path::new("/var/elohim/blobs_iroh"));
        assert_ne!(cfg.blobs_dir.file_name().unwrap(), "blobs");
    }

    #[test]
    fn secret_key_path_distinct_from_libp2p_identity() {
        // libp2p path uses `<storage_dir>/identity.key` (see src/main.rs);
        // iroh path must NEVER write to that file.
        let cfg = IrohConfig::from_storage_dir(Path::new("/var/elohim"));
        assert_eq!(cfg.secret_key_path, Path::new("/var/elohim/iroh.key"));
        assert_ne!(cfg.secret_key_path.file_name().unwrap(), "identity.key");
    }

    #[test]
    fn defaults_never_use_n0() {
        // Wave-2 relay-sovereignty design D1: n0 relays/discovery must never
        // be a silent default. This node phones no third party unless an
        // operator explicitly opts in via ELOHIM_IROH_USE_N0_* env vars.
        let cfg = IrohConfig::from_storage_dir(Path::new("/x"));
        assert!(!cfg.use_n0_relays);
        assert!(!cfg.use_n0_discovery);
        assert!(cfg.discovery_resolvers.is_empty());
        assert!(cfg.relay_url.is_none());
    }

    #[test]
    fn apply_env_sets_relay_url() {
        let dir = Path::new("/x");
        let mut cfg = IrohConfig::from_storage_dir(dir);
        // SAFETY: test-only env mutation; scoped to this test's env vars.
        unsafe {
            std::env::set_var("ELOHIM_IROH_RELAY_URL", "https://relay.elohim.host");
        }
        cfg.apply_env();
        unsafe {
            std::env::remove_var("ELOHIM_IROH_RELAY_URL");
        }
        assert_eq!(cfg.relay_url.as_deref(), Some("https://relay.elohim.host"));
    }

    #[test]
    fn apply_env_opt_in_n0_relays() {
        let dir = Path::new("/x");
        let mut cfg = IrohConfig::from_storage_dir(dir);
        // SAFETY: test-only env mutation; scoped to this test's env vars.
        unsafe {
            std::env::set_var("ELOHIM_IROH_USE_N0_RELAYS", "true");
        }
        cfg.apply_env();
        unsafe {
            std::env::remove_var("ELOHIM_IROH_USE_N0_RELAYS");
        }
        assert!(cfg.use_n0_relays);
    }

    #[test]
    fn apply_env_opt_in_n0_discovery_repopulates_default_resolver() {
        let dir = Path::new("/x");
        let mut cfg = IrohConfig::from_storage_dir(dir);
        // SAFETY: test-only env mutation; scoped to this test's env vars.
        unsafe {
            std::env::set_var("ELOHIM_IROH_USE_N0_DISCOVERY", "true");
        }
        cfg.apply_env();
        unsafe {
            std::env::remove_var("ELOHIM_IROH_USE_N0_DISCOVERY");
        }
        assert!(cfg.use_n0_discovery);
        assert_eq!(cfg.discovery_resolvers.len(), 1);
    }

    #[test]
    fn apply_env_ignores_invalid_bool_and_warns() {
        let dir = Path::new("/x");
        let mut cfg = IrohConfig::from_storage_dir(dir);
        // SAFETY: test-only env mutation; scoped to this test's env vars.
        unsafe {
            std::env::set_var("ELOHIM_IROH_USE_N0_RELAYS", "not-a-bool");
        }
        cfg.apply_env();
        unsafe {
            std::env::remove_var("ELOHIM_IROH_USE_N0_RELAYS");
        }
        // Invalid value ignored — sovereign default untouched.
        assert!(!cfg.use_n0_relays);
    }
}
