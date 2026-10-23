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
//!
//! **Admission at connect time is the possession proof.** iroh authenticates the remote node id
//! cryptographically on every connection: the TLS handshake carries the peer's raw ed25519
//! public key and proves it holds the matching secret, and `Connection::remote_node_id` reads
//! the id from that handshake. A node id [`Admission::admits`] for a connection's authenticated
//! remote is therefore held by the device the roster names. No further challenge is needed —
//! provided the caller passes only `remote_node_id`, never an id a peer merely claims.

mod admission;
mod endpoint;
mod error;
mod key;

pub use admission::{roster_rows_from_file, Admission};
pub use endpoint::{bind, relay_from_env, relay_from_value, RELAY_ENV};
pub use error::PeerError;
pub use key::PeerKey;
