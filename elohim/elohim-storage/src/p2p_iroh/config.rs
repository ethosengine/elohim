//! Iroh-side P2P configuration.
//!
//! Disjoint by design from the libp2p path:
//! - libp2p uses `<storage_dir>/identity.key` + `<storage_dir>/blobs/`
//! - iroh uses `<storage_dir>/iroh.key` + `<storage_dir>/blobs_iroh/`
//!
//! Phase 1 sets up the config surface; later phases add ALPN registration,
//! discovery options, etc.

use std::path::{Path, PathBuf};

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
    /// `true` for production peers — replaces libp2p Kademlia for record
    /// publication. `false` in tests where peer addresses are exchanged
    /// out-of-band via [`iroh::Endpoint::add_node_addr`].
    pub use_n0_discovery: bool,
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
