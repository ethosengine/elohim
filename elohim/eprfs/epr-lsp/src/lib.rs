//! One shared, session-attributed language server per `(language, toolchain pin, checkout root)`.
//!
//! Many agent sessions attach to one supervised language server through this multiplexer. Each
//! client is keyed by its `epr session` identity (contract C1), so every session can tell its own
//! diagnostics from those it caused elsewhere and from other sessions' in-flight work (C7).
//!
//! The crate follows a core/IO split. These modules are pure and unit-tested without I/O:
//! - [`frame`]: LSP `Content-Length` framing.
//! - [`mux`]: the client table, request-id remapping, request routing, URI refcounts and the
//!   cached `initialize` result, as a state machine of inputs to outputs.
//! - [`attrib`]: the per-URI diagnostic diff, the edit-event window, ownership and ambiguity.
//! - [`deliver`]: the per-session view, the folded summaries and the attribution snapshot.
//! - [`uri`]: `file://` URIs to and from paths.
//! - [`limits`]: the transport bounds and the per-client outbound accounting.
//!
//! These modules carry the I/O, on tokio:
//! - [`server`]: the policy (C5), the server spec and the child process.
//! - [`socket`]: the runtime directory (C3), the serve lock, the hello line and the listener.
//! - [`daemon`]: the serve loop that joins the socket, the child and the mux, with idle reap.
//! - [`attach`]: the stdio shim's proxy and its null-server fallback (C4).
//! - [`gatefeed`]: the device mux's tail of gate-live run logs, published as `gate-live`
//!   diagnostics (its parser and line splitter are pure).
//!
//! Edit events and attributions are ephemeral and device-local: they are never notarized,
//! projected or read by anything but this multiplexer and `epr lsp show`.

pub mod attach;
pub mod attrib;
pub mod daemon;
pub mod deliver;
pub mod frame;
pub mod gatefeed;
pub mod limits;
pub mod mux;
pub mod server;
pub mod socket;
pub mod uri;

pub use attrib::{Attribution, EditEvent, SessionKey};
pub use mux::{ClientId, Input, Mux, MuxConfig, Output};
