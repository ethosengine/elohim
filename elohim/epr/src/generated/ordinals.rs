//! AUTO-GENERATED from protocol JSON schemas with the `_ordinal` marker.
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json (`_ordinal: true`)
//!
//! Lives in epr (not elohim-storage) because epr is upstream of storage and
//! cannot import storage's generated enums without a circular dependency.

/// (value, ordinal score) — 1 = lowest, 8 = highest.
pub const REACH_OPENNESS: &[(&str, u8)] = &[
    ("private", 1),
    ("self", 2),
    ("intimate", 3),
    ("trusted", 4),
    ("familiar", 5),
    ("community", 6),
    ("public", 7),
    ("commons", 8),
];
