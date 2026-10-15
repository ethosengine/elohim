//! AUTO-GENERATED from protocol JSON schemas whose `_codegen.rust` names "epr".
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json
//!
//! Lives in epr because epr is upstream of epr-rea and elohim-storage, and because these
//! vocabularies must never be generated into an integrity zome (that moves the DNA hash).

/// Core reaaction — Declared in the elohim DNA's REA_ACTIONS (content_store_integrity).
pub const CORE_REA_ACTIONS: &[&str] = &[
    "use",
    "consume",
    "cite",
    "produce",
    "raise",
    "lower",
    "transfer",
    "transfer-custody",
    "transfer-all-rights",
    "move",
    "modify",
    "combine",
    "separate",
    "work",
    "deliver-service",
    "pickup",
    "dropoff",
    "give",
    "take",
    "accept",
    "custody-blob",
    "serve-blob",
    "ack-projection",
    "operate-doorway",
    "project-epr",
];

/// All reaaction — includes storage-only and extensible.
pub const ALL_REA_ACTIONS: &[&str] = &[
    "use",
    "consume",
    "cite",
    "produce",
    "raise",
    "lower",
    "transfer",
    "transfer-custody",
    "transfer-all-rights",
    "move",
    "modify",
    "combine",
    "separate",
    "work",
    "deliver-service",
    "pickup",
    "dropoff",
    "give",
    "take",
    "accept",
    "custody-blob",
    "serve-blob",
    "ack-projection",
    "operate-doorway",
    "project-epr",
    "affirm",
    "dismiss",
];
