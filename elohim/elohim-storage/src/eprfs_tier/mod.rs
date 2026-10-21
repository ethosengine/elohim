//! eprfs tier adapters (interfaces spec §1.1 L2, §2.A.6): this crate's side of the storage-tier
//! seam, translating its own stores and records into eprfs-core's traits.
//!
//! - [`BlobStoreTier`] implements `eprfs_core::EprfsStorage` over a [`crate::blob_store::BlobStore`]:
//!   the peer's own store as a tier (class `Own`). It verifies what it reads and refuses to keep
//!   bytes that fail their address; it decides no reach (a serving tier is fronted by a
//!   `TierEndpoint`) and ignores fetch policy, as every leaf store does (contract 11).
//! - [`RootReachGate`] implements `eprfs_core::ReachGate`: `blob_reach::blob_serve_verdict`
//!   generalized from content rows to root records, composed with
//!   `private_reach::private_serve_verdict` for `private` records. It reads only records this
//!   peer holds, through [`ReachRecords`]; an unknown record refuses (`NoRecord`).
//!
//! Neither is wired into a route or the reconcile loop yet: the `POST /eprfs/v1/blobs` host over
//! `TierEndpoint`, `PeerTransportTier`, this crate's own `ForwardGate` and the custody `Ports`
//! are follow-ons (spec §2.A.6, §2.D). Rail R-E3 (`crate::boundary::eprfs_direction`) keeps this
//! crate's eprfs dependencies to eprfs-core (and eprfs-serve's harness in tests).

pub mod blob_store_tier;
pub mod root_reach_gate;

pub use blob_store_tier::{BlobStoreTier, TIER_SUBDIR};
pub use root_reach_gate::{ReachRecords, RecordsUnavailable, RootReachGate};
