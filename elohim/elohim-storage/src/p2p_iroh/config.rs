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

    /// Whether to use n0's hosted relay infrastructure for NAT traversal.
    /// `false` in tests where both endpoints are loopback.
    pub use_n0_relays: bool,

    /// Whether to enable n0's hosted DNS-based peer discovery (Phase 10).
    ///
    /// Deprecated: use `discovery_resolvers` instead. When `true` and
    /// `discovery_resolvers` is empty, a synthetic `DiscoveryResolverConfig::n0_default()`
    /// is used. When `discovery_resolvers` is non-empty, this field is ignored.
    pub use_n0_discovery: bool,

    /// Pkarr discovery resolvers iroh queries (and publishes to). Empty
    /// list means "no discovery" — peer addresses must be exchanged
    /// out-of-band via `Endpoint::add_node_addr` (this is what tests do).
    ///
    /// When non-empty, each entry becomes a (PkarrPublisher + PkarrResolver)
    /// pair registered on the Endpoint via `add_discovery`, wrapped by
    /// iroh's `ConcurrentDiscovery` for parallel querying.
    ///
    /// See genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md
    /// (cutover gate #10) for the operator-self-hostable rationale.
    pub discovery_resolvers: Vec<DiscoveryResolverConfig>,
}

impl IrohConfig {
    /// Construct a default config rooted at `storage_dir`. Both file paths
    /// are intentionally disjoint from the libp2p path's filenames so the
    /// two stacks can coexist on a single storage volume during cutover.
    pub fn from_storage_dir(storage_dir: &Path) -> Self {
        Self {
            blobs_dir: storage_dir.join("blobs_iroh"),
            secret_key_path: storage_dir.join("iroh.key"),
            use_n0_relays: true,
            use_n0_discovery: true,
            // Back-compat: use_n0_discovery=true → n0 default resolver.
            // Operators override by setting discovery_resolvers explicitly.
            discovery_resolvers: vec![DiscoveryResolverConfig::n0_default()],
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
    fn defaults_enable_n0_relays() {
        let cfg = IrohConfig::from_storage_dir(Path::new("/x"));
        assert!(cfg.use_n0_relays);
    }
}
