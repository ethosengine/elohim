//! ElohimGate — mutation interceptor for protocol-level agent reasoning.
//!
//! Every mutation passes through the gate. The gate computes a TrustContext,
//! classifies an InferenceTier, and returns a GateResult that determines
//! how the mutation settles.
