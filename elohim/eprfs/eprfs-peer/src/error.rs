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

    /// A peer's tier served a leaf whose bytes fail its address. Never accepted as an answer.
    #[error("the peer's tier served bytes that fail their address {cid}")]
    TierLeafIntegrity { cid: eprfs_core::BlobCid },
}
