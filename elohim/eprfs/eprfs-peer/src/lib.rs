//! A device's iroh peer: the key it answers on, the endpoint it binds, and which authenticated
//! peers it admits (spec `2026-10-23-workspaces-cooperate-as-devices-design.md` §3.1–§3.2).
//!
//! - [`PeerKey`] is the device's iroh secret key, generated once at
//!   `$XDG_CONFIG_HOME/epr/peer/iroh.key` (mode 0600) and never copied, sent, printed or logged.
//!   A rebuilt workspace mints a new key and writes a new `PeerEndpoint` row.
//! - [`bind`] opens the endpoint on that key with the protocols the caller serves.
//! - [`Admission`] maps an authenticated iroh node id to the device `did:key` whose valid
//!   `PeerEndpoint` row names it, read from the roster in this device's own checkout
//!   ([`roster_rows_from_file`]). Everyone else is refused.
//! - [`TierProtocol`] serves this device's eprfs store on [`TIER_ALPN`] behind
//!   [`StewardDevicesGate`], which answers only admitted devices; everyone else gets
//!   eprfs-serve's uniform refusal. [`tier_call`] and [`tier_request`] ask a peer's tier.
//!
//! - The steward channel (§3.3): [`project_channel`] places this device's crossing events (the
//!   [`CrossingAllowList`] from `pool-policy.json`) in a channel tree served from a
//!   [`ChannelStore`]; [`ChannelHead`] is the signed head a device gossips on [`topic_for`] its
//!   lineage, carrying its acknowledgement vector; [`Channel`] keeps the announce rhythm and judges
//!   the heads it hears; [`fold_root`] folds another device's events into this device's flow store
//!   `via` that device; [`HeadTable`] persists what was folded; [`PeerDaemon`] runs it all behind
//!   `epr peer serve`.
//!
//! **Admission at connect time is the possession proof.** iroh authenticates the remote node id
//! cryptographically on every connection: the TLS handshake carries the peer's raw ed25519
//! public key and proves it holds the matching secret, and `Connection::remote_node_id` reads
//! the id from that handshake. A node id [`Admission::admits`] for a connection's authenticated
//! remote is therefore held by the device the roster names. No further challenge is needed —
//! provided the caller passes only `remote_node_id`, never an id a peer merely claims.

mod admission;
mod allow;
mod announce;
mod channel;
mod daemon;
mod endpoint;
mod error;
mod fold;
mod gate;
mod gossip;
mod head;
mod head_table;
mod key;
mod tier;

pub use admission::{roster_rows_from_file, Admission};
pub use allow::{via_device, CrossingAllowList, VIA_SLOT};
pub use announce::{
    topic_for, Channel, HeadVerdict, ANNOUNCE_DEBOUNCE, REANNOUNCE_EVERY, TOPIC_PREFIX,
};
pub use channel::{
    channel_tier_id, project_channel, ChannelStore, ChannelTree, CHANNEL_EVENTS, RETAINED_ROOTS,
};
pub use daemon::{PeerConfig, PeerDaemon, FOLD_DEADLINE, TICK};
pub use endpoint::{bind, bind_on, peer_from_arg, relay_from_env, relay_from_value, RELAY_ENV};
pub use error::PeerError;
pub use fold::{
    fetch_channel, fold_events, fold_root, folded_from, ChannelEvent, Fetched, FoldLimits,
    FoldOutcome, FoldReport, FoldTarget, MAX_EVENT_BYTES, MAX_FOLD_BYTES, MAX_FOLD_EVENTS,
};
pub use gate::{SharedAdmission, StewardDevicesGate};
pub use gossip::GatedGossip;
pub use head::{ChannelHead, HeadSigner, MAX_HEAD_BYTES};
pub use head_table::{ChannelCounters, DeviceHead, DroppedHeads, HeadTable, HEAD_TABLE_VERSION};
/// The iroh address types a dialer names peers with, so a consumer needs no iroh dependency of
/// its own.
pub use iroh::{NodeAddr, NodeId};
pub use key::PeerKey;
pub use tier::{
    tier_call, tier_request, TierAnswer, TierCountersSnapshot, TierLimits, TierProtocol,
    TierRequest, FRAME_DEADLINE, MAX_ANSWER_FRAME_BYTES, MAX_CONCURRENT_STREAMS,
    MAX_IN_FLIGHT_STREAMS, MAX_REQUEST_FRAME_BYTES, MAX_UNADMITTED_IN_FLIGHT_STREAMS, TIER_ALPN,
};
