//! Recovery — cryptographic share assembly for key-material recovery.
//!
//! This module is the OPTIONAL cryptographic proof layer that sits on top of
//! the attestation-DHT-driven recovery flow.
//!
//! ## Architecture
//!
//! ```text
//! Social-threshold flow (REQUIRED)                    Crypto layer (OPTIONAL)
//! ─────────────────────────────────                   ────────────────────────
//! governance-action:recovery-request  ──creates──►  ShareAssembler::assemble()
//!   attestation:recovery-approval ×N                   │
//!   governance_action_tally.quorum_reached = 1          │
//!                                                        ▼
//!                                              ShamirShareRequest via libp2p
//!                                              (authenticated by DHT approval)
//!                                                        │
//!                                                        ▼
//!                                              ShamirShareResponse (verified)
//!                                                        │
//!                                                        ▼
//!                                              shamir_combine() — DEFERRED
//!                                              see NEEDS_CONTEXT note below
//! ```
//!
//! ## Shamir reconstruction primitive
//!
//! NEEDS_CONTEXT: no `shamir_combine` function exists in this codebase. The
//! `iroh_recovery_cross_stack` test explicitly stubs reconstruction with an XOR
//! scheme and documents that real Shamir is "deferred to the share-custody epic"
//! (`tests/iroh_recovery_cross_stack.rs:52`). The `shamirsecretsharing` crate is
//! NOT a declared dependency and MUST NOT be added here without a Cargo.toml change
//! that is outside this task's scope.
//!
//! `ShareAssembler::assemble` performs steps 1–4 (DHT attestation lookup, libp2p
//! transport, signature verification, threshold accumulation) and returns the
//! raw share bytes for the caller to reconstruct. Step 5 (reconstruction) is
//! exposed as a trait hook `combine_shares` with a stub default impl that
//! returns an error so the caller can plug in the real primitive when available.

pub mod share_assembler;
pub use share_assembler::ShareAssembler;
