//! The device's content-addressed payload store.
//!
//! [`BlobStore`] holds byte-profile-0 payloads (CIDv1 raw + BLAKE3-256): an inline file for a
//! payload of at most one chunk, otherwise fastcdc-2020 chunks behind a layout record and a
//! marker written last; a dag-cbor record lane beside them; verify-on-read with quarantine.
//! It moved here from elohim-storage (artifact-store E4) so that one store per device can be
//! opened by every process on it — elohim-storage's tier and epr-cli's payload store — with the
//! owner/shared rule in [`store`]'s module docs deciding who may move or remove a file.
//!
//! - [`store`]: the store itself.
//! - [`marker`]: the payload directory's format marker (`root/STORE`).
//! - `tier` (feature `tier`): [`tier::BlobStoreTier`], the store as an eprfs tier, and
//!   [`tier::compose_shared_payload_tier`], the marker-gated opener elohim-storage composes.
//!
//! This crate knows nothing of its consumers: elohim-storage maps [`BlobStoreError`] onto its
//! own error type and schedules its own compute-payload cleanup beside the `/blob` store.
//!
//! It is a reusable primitive, licensed MPL-2.0 (see `LICENSE`) with no Exhibit B notice
//! attached, so it stays compatible with secondary licences. Apache-2.0 epr-cli may therefore
//! link it by default. elohim-storage, which consumes it, remains AGPL-3.0.

pub mod error;
pub mod marker;
pub mod store;
#[cfg(feature = "tier")]
pub mod tier;

pub use error::{BlobStoreError, Result};
pub use store::{
    byte_cid_serde, fanout_path, BlobStore, ChunkRef, Exists, LayoutRecord, OpenMode,
    RecordStoreResult, StorageStats, StoreResult, MARKER_MAGIC, OPS_LOCK, OWNER_LOCK,
    STORE_PROFILE,
};
