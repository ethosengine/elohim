//! Device-local primitives: process identity and content-addressed executable custody.
#![warn(missing_docs)]
pub mod bin_store;
pub mod checkout_trust;
pub mod commit_attribution;
pub mod device_key_path;
pub mod envelope;
pub mod layout;
pub mod manifest_store;
pub mod session;
pub mod session_stasis;
