//! The one error type of the device peer. No variant carries key material.

use std::io;
use std::path::PathBuf;

/// Why the device peer could not load its key, read its roster, bind its endpoint or ask a
/// peer's tier.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PeerError {
    /// The key file grants group or others a permission. It is refused, not repaired: a key
    /// another account could have read is no longer this device's alone. `mode` is the file's
    /// permission bits (`& 0o777`).
    #[error(
        "the peer key `{}` is open to group or others (mode {mode:o}); it may have been read by \
         another account — remove it to mint a new key and write a new PeerEndpoint row",
        path.display()
    )]
    KeyFileExposed { path: PathBuf, mode: u32 },

    /// The key file does not hold a 32-byte ed25519 secret. It is never regenerated in place:
    /// a new key would silently change this device's node id.
    #[error("the peer key `{}` holds {len} bytes, not a 32-byte ed25519 secret", path.display())]
    KeyFileCorrupt { path: PathBuf, len: u64 },

    /// Reading, creating or publishing the key file failed.
    #[error("peer key I/O at `{}`: {source}", path.display())]
    KeyIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The roster file could not be read.
    #[error("cannot read the roster `{}`: {source}", path.display())]
    RosterIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The roster file's name is not `<handle>.jsonl`, or its contents do not verify (a line
    /// that does not parse, a row whose CID no longer matches, a row about another handle).
    #[error("the roster `{}` does not verify: {reason}", path.display())]
    RosterInvalid { path: PathBuf, reason: String },

    /// The configured relay is not a URL. Never a silent fallback to another relay.
    #[error("invalid {} {value:?}: {source}", crate::endpoint::RELAY_ENV)]
    InvalidRelayUrl {
        value: String,
        #[source]
        source: url::ParseError,
    },

    /// A `--peer` address that is not `<node_id>@<ip:port>`.
    #[error("invalid peer address {value:?}: {reason}")]
    InvalidPeer { value: String, reason: String },

    /// iroh could not bind the endpoint.
    #[error("cannot bind the iroh endpoint: {0}")]
    Bind(#[source] Box<iroh::endpoint::BindError>),

    /// A peer's tier could not be dialled.
    #[error("cannot reach the peer's tier: {0}")]
    TierConnect(#[source] Box<iroh::endpoint::ConnectError>),

    /// A tier stream failed or was reset mid-exchange (a peer resets a stream whose request
    /// frame it refused).
    #[error("the tier stream failed: {0}")]
    TierStream(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// A tier frame past its cap, or one that does not decode.
    #[error("bad tier frame: {reason}")]
    TierFrame { reason: String },

    /// A received tier frame whose stated length is past the reader's cap; its body is never read.
    #[error("a tier frame of {len} bytes exceeds the {cap}-byte cap")]
    TierFrameTooLarge { len: u32, cap: u32 },

    /// A peer's tier served a leaf or listing that fails its address (bytes or entries that do
    /// not encode to its CID) or that is at another path than the one asked. Never accepted as an
    /// answer.
    #[error("the peer's tier served an object that fails its address or path: {cid}")]
    TierLeafIntegrity { cid: eprfs_core::BlobCid },

    /// The crossing allow-list (`berth.channel.crosses` in pool-policy.json) could not be read or
    /// is not `{measure: [kind, …]}`. Refused, never widened: a list that does not parse lets
    /// nothing cross.
    #[error("the crossing allow-list at `{}` is invalid: {reason}", path.display())]
    Policy { path: PathBuf, reason: String },

    /// The device's flow store could not be read or appended to.
    #[error("the flow store failed: {0}")]
    FlowStore(#[source] elohim_epr_rea::FabricError),

    /// A channel tree could not be encoded (an event or a directory past the tree limits).
    #[error("cannot build the channel tree: {reason}")]
    ChannelTree { reason: String },

    /// A channel head does not verify against the did:key it names, or names another device than
    /// the one it is checked as.
    #[error("the channel head does not verify as {device}")]
    HeadSignature { device: String },

    /// A channel head that does not encode or decode within its cap.
    #[error("bad channel head: {reason}")]
    HeadFrame { reason: String },

    /// The persisted head table could not be read or written.
    #[error("head table I/O at `{}`: {source}", path.display())]
    HeadTableIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The persisted head table does not parse. It is never silently reset: its own seq keeps
    /// this device's heads strictly increasing.
    #[error("the head table `{}` does not parse: {reason}", path.display())]
    HeadTableInvalid { path: PathBuf, reason: String },

    /// The lease view was asked for on a device its own roster does not admit (no PeerEndpoint
    /// row, or its binding contested): it has no standing on the steward channel to judge from.
    #[error("this device ({device}) is not admitted on its own roster's steward channel")]
    NotAdmitted { device: String },

    /// A peer's channel tree served something that is not a crossing event at its own CID: a leaf
    /// whose name is not the CID of its bytes, bytes that are not a canonical FlowEvent, or an
    /// entry that is not a file under `channel/events`.
    #[error("the peer's channel tree does not hold events at their own CIDs: {reason}")]
    FoldIntegrity { reason: String },

    /// The peer's tier did not answer for a root or path the fold needs (it does not hold the
    /// root, or the device is no longer admitted there). Retried on the next head.
    #[error("the peer's tier did not answer for {what}")]
    FoldUnanswered { what: String },

    /// The device a fold reads from is no longer admitted (a Contest was folded while the fold
    /// ran): nothing of it is appended.
    #[error("{device} is no longer admitted; nothing of its fold is appended")]
    FoldRevoked { device: String },

    /// The daemon's loop ended other than by being stopped: its gossip stream ended, a head could
    /// not be persisted before it was announced, or the loop task failed.
    #[error("the device peer stopped: {reason}")]
    DaemonEnded { reason: String },

    /// The daemon could not start its gossip on the steward topic.
    #[error("cannot join the steward channel topic: {reason}")]
    Gossip { reason: String },
}
