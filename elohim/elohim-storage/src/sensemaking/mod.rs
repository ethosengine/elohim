//! Sensemaking: opinion clustering for governance deliberation
//!
//! Implements Polis-inspired sensemaking where participants submit statements
//! and vote agree/disagree/pass. The clustering algorithm groups participants
//! by voting similarity and identifies bridging statements that cross cluster
//! boundaries.

pub mod clustering;
