//! eprfs tier adapters (interfaces spec §1.1 L2, §2.A.6): this crate's side of the storage-tier
//! seam, translating its own stores and records into eprfs-core's traits.
//!
//! - [`BlobStoreTier`] implements `eprfs_core::EprfsStorage` over a payload store: the peer's
//!   own store as a tier (class `Own`). It verifies what it reads and refuses to keep bytes that
//!   fail their address; it decides no reach (a serving tier is fronted by a `TierEndpoint`) and
//!   ignores fetch policy, as every leaf store does (contract 11). It moved with the store to
//!   `crates/elohim-blob-store` (artifact-store E4) and is re-exported here under its old path,
//!   with one signature change: its constructors return `elohim_blob_store::BlobStoreError`, and
//!   [`BlobStoreTier::store`] the crate's `elohim_blob_store::BlobStore`, not this crate's
//!   [`crate::blob_store::BlobStore`] shim (no caller of this crate uses either; the adapter is
//!   unwrapped rather than wrapped so it stays the one implementation the tier tests hold).
//!   `dedicated(root)` still opens `root/eprfs-tier`; `at(root, mode)` opens exactly `root`.
//! - [`RootReachGate`] implements `eprfs_core::ReachGate`: `blob_reach::blob_serve_verdict`
//!   generalized from content rows to root records, composed with
//!   `private_reach::private_serve_verdict` for `private` records. It reads only records this
//!   peer holds, through [`ReachRecords`]; an unknown record refuses (`NoRecord`).
//! - [`compose_payload_tier_from_env`]: the device payload store as this process's tier, when
//!   `EPR_PAYLOAD_STORE` names a root whose marker says `blob-store-v1` (opened shared).
//!
//! Neither adapter is wired into a route or the reconcile loop yet: the `POST /eprfs/v1/blobs`
//! host over `TierEndpoint`, `PeerTransportTier`, this crate's own `ForwardGate` and the custody
//! `Ports` are follow-ons (spec §2.A.6, §2.D). Rail R-E3 (`crate::boundary::eprfs_direction`)
//! keeps this crate's eprfs dependencies to eprfs-core (and eprfs-serve's harness in tests).

pub mod root_reach_gate;

use std::path::{Path, PathBuf};

pub use elohim_blob_store::tier::{BlobStoreTier, PayloadTier, TIER_SUBDIR};
pub use elohim_blob_store::OpenMode;
pub use root_reach_gate::{ReachRecords, RecordsUnavailable, RootReachGate};

use tracing::{info, warn};

/// The environment variable naming the device's payload store (the same name `epr peer serve`
/// reads).
pub const PAYLOAD_STORE_ENV: &str = "EPR_PAYLOAD_STORE";

/// The storage process's payload tier, from [`PAYLOAD_STORE_ENV`]: unset composes nothing and
/// logs nothing (today's behaviour). Set, its root's marker decides: `blob-store-v1` composes
/// a **shared** [`BlobStoreTier`] over exactly that root and logs one line naming the path and
/// format; any other marker, none, or a root that does not open logs one warning and composes
/// nothing — no backend is opened and nothing is created there. The caller holds the returned
/// tier for the process's lifetime.
pub async fn compose_payload_tier_from_env() -> Option<BlobStoreTier> {
    let root = std::env::var_os(PAYLOAD_STORE_ENV).filter(|value| !value.is_empty())?;
    compose_payload_tier(&PathBuf::from(root)).await
}

/// [`compose_payload_tier_from_env`] for an explicit root.
pub async fn compose_payload_tier(root: &Path) -> Option<BlobStoreTier> {
    match elohim_blob_store::tier::compose_shared_payload_tier(root).await {
        Ok(PayloadTier::Composed(tier)) => {
            info!(
                path = %root.display(),
                format = elohim_blob_store::marker::BLOB_STORE_V1,
                "payload tier composed (shared instance of the device payload store)"
            );
            Some(tier)
        }
        Ok(PayloadTier::NotBlobStore { marker }) => {
            warn!(
                path = %root.display(),
                marker = marker.as_deref().unwrap_or("<none>"),
                "{PAYLOAD_STORE_ENV} names a payload store that is not {}: no payload tier \
                 composed",
                elohim_blob_store::marker::BLOB_STORE_V1
            );
            None
        }
        Err(error) => {
            warn!(
                path = %root.display(),
                %error,
                "{PAYLOAD_STORE_ENV} payload store did not open: no payload tier composed"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn private_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        dir
    }

    fn entries(root: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(root)
            .map(|d| {
                d.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    #[tokio::test]
    async fn a_blob_store_marker_composes_a_shared_tier_over_exactly_that_root() {
        let dir = private_dir();
        std::fs::write(
            dir.path().join(elohim_blob_store::marker::FILE),
            format!("{}\n", elohim_blob_store::marker::BLOB_STORE_V1),
        )
        .unwrap();
        let tier = compose_payload_tier(dir.path()).await.expect("composed");
        assert_eq!(tier.store().root_dir(), dir.path());
        assert_eq!(tier.store().open_mode(), Some(OpenMode::Shared));
        assert!(!dir.path().join(TIER_SUBDIR).exists());
    }

    #[tokio::test]
    async fn another_marker_or_none_composes_nothing_and_creates_nothing() {
        for marker in [None, Some("directory-v1"), Some("blob-store-v9")] {
            let dir = private_dir();
            if let Some(marker) = marker {
                std::fs::write(dir.path().join(elohim_blob_store::marker::FILE), marker).unwrap();
            }
            let before = entries(dir.path());
            assert!(
                compose_payload_tier(dir.path()).await.is_none(),
                "{marker:?}"
            );
            assert_eq!(entries(dir.path()), before, "{marker:?}: nothing created");
        }
        let dir = private_dir();
        let absent = dir.path().join("never-made");
        assert!(compose_payload_tier(&absent).await.is_none());
        assert!(!absent.exists(), "an absent root is never created");
    }
}
