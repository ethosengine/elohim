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
//!   `EPR_PAYLOAD_STORE` names a root whose marker says `blob-store-v1` (opened shared) and that
//!   neither is, contains, nor lies inside the `/blob` store's root ([`BlobRootOverlap`]).
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
/// logs nothing (today's behaviour). Set, a root that overlaps `blobs_dir`, the `/blob` store's
/// root ([`blob_root_overlap`]), logs one warning naming the overlap and composes nothing: one
/// directory configured as both the unlocked `/blob` store and the device's owner/shared store is
/// the one way the two representations could meet. Otherwise its marker decides:
/// `blob-store-v1` composes a **shared** [`BlobStoreTier`] over exactly that root and logs one
/// line naming the path and format; any other marker, none, or a root that does not open logs
/// one warning and composes nothing — no backend is opened and nothing is created there. The
/// caller holds the returned tier for the process's lifetime.
pub async fn compose_payload_tier_from_env(blobs_dir: &Path) -> Option<BlobStoreTier> {
    let root = std::env::var_os(PAYLOAD_STORE_ENV).filter(|value| !value.is_empty())?;
    compose_payload_tier_beside(&PathBuf::from(root), blobs_dir).await
}

/// How a payload-store root overlaps the `/blob` store's root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobRootOverlap {
    /// They are the same directory.
    Same,
    /// The payload root contains the `/blob` root.
    PayloadIsAncestor,
    /// The payload root lies inside the `/blob` root.
    PayloadIsDescendant,
}

/// Whether `payload` and `blobs` overlap, compared as resolved paths: each is made absolute and
/// canonicalized, and a path that does not exist yet is resolved through its nearest existing
/// ancestor (so a payload root not created yet still compares by where it would be made).
pub fn blob_root_overlap(payload: &Path, blobs: &Path) -> Option<BlobRootOverlap> {
    let (payload, blobs) = (resolved(payload), resolved(blobs));
    if payload == blobs {
        Some(BlobRootOverlap::Same)
    } else if blobs.starts_with(&payload) {
        Some(BlobRootOverlap::PayloadIsAncestor)
    } else if payload.starts_with(&blobs) {
        Some(BlobRootOverlap::PayloadIsDescendant)
    } else {
        None
    }
}

/// `path` absolute and canonical through its nearest existing ancestor, the rest appended.
fn resolved(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut existing = absolute.as_path();
    let mut missing = Vec::new();
    loop {
        if let Ok(real) = existing.canonicalize() {
            return missing.iter().rev().fold(real, |acc, part| acc.join(part));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name.to_os_string());
                existing = parent;
            }
            _ => return absolute,
        }
    }
}

/// [`compose_payload_tier_from_env`] for an explicit payload root beside an explicit `/blob`
/// root.
pub async fn compose_payload_tier_beside(root: &Path, blobs_dir: &Path) -> Option<BlobStoreTier> {
    if let Some(overlap) = blob_root_overlap(root, blobs_dir) {
        warn!(
            path = %root.display(),
            blobs_dir = %blobs_dir.display(),
            ?overlap,
            "{PAYLOAD_STORE_ENV} overlaps the /blob store's root: no payload tier composed (one \
             directory must not hold both the unlocked store and the device payload store)"
        );
        return None;
    }
    compose_payload_tier(root).await
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

    #[tokio::test]
    async fn a_payload_root_overlapping_the_blob_root_composes_nothing() {
        let dir = private_dir();
        let blobs = dir.path().join("storage").join("blobs");
        std::fs::create_dir_all(&blobs).unwrap();
        let marked = |root: &Path| {
            std::fs::create_dir_all(root).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
            std::fs::write(
                root.join(elohim_blob_store::marker::FILE),
                format!("{}\n", elohim_blob_store::marker::BLOB_STORE_V1),
            )
            .unwrap();
        };
        let cases = [
            (blobs.clone(), BlobRootOverlap::Same),
            (
                dir.path().join("storage"),
                BlobRootOverlap::PayloadIsAncestor,
            ),
            (blobs.join("payload"), BlobRootOverlap::PayloadIsDescendant),
        ];
        for (root, overlap) in cases {
            assert_eq!(
                blob_root_overlap(&root, &blobs),
                Some(overlap),
                "{}",
                root.display()
            );
            marked(&root);
            let before = entries(&root);
            assert!(
                compose_payload_tier_beside(&root, &blobs).await.is_none(),
                "{overlap:?}: composed"
            );
            assert_eq!(entries(&root), before, "{overlap:?}: nothing created");
        }
        // The same path spelled through `..`, and a payload root not created yet inside blobs.
        let spelled = dir
            .path()
            .join("storage")
            .join("x")
            .join("..")
            .join("blobs");
        std::fs::create_dir_all(dir.path().join("storage").join("x")).unwrap();
        assert_eq!(
            blob_root_overlap(&spelled, &blobs),
            Some(BlobRootOverlap::Same)
        );
        assert_eq!(
            blob_root_overlap(&blobs.join("not").join("yet"), &blobs),
            Some(BlobRootOverlap::PayloadIsDescendant)
        );
        // A sibling does not overlap, even when its name extends the blob root's.
        let sibling = dir.path().join("storage").join("blobs-payload");
        assert_eq!(blob_root_overlap(&sibling, &blobs), None);
        marked(&sibling);
        assert!(compose_payload_tier_beside(&sibling, &blobs)
            .await
            .is_some());
    }
}
