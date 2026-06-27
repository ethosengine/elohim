//! Lens-market view types — the wire shape for GET /api/v1/epr/{eprScope}/lens-market.
//!
//! CONTRACT: these structs CONFORM to `sdk/schemas/v1/views/lens-market-view.schema.json`,
//! which is the authoritative front↔back contract (the schema is SoT for field names; ts-rs
//! is SoT for serde behavior; `elohim-storage/tests/schema_contract.rs` catches drift). The
//! values are assembled by the storage facing adapter from the `elohim-facings::folds::lens_*`
//! folds over the notarized projections — Operational Category C (reconstructable per request).
//!
//! Charter: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
//! Plan I5/I6: genesis/docs/superpowers/plans/2026-06-27-plural-mishpat-lenses-wave1-plan.md

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One lens governing an EPR, side by side with its peers (plural, co-valid).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct LensBindingView {
    /// The lens entry_hash (= CID, the read/scope key; never action_hash).
    pub lens_cid: String,
    /// The collective / school-of-thought label (feeds the contributor-card).
    pub school: String,
    /// Governance role: `lens` | `floor` | `ceiling`.
    pub role: String,
    /// Short human summary of what this lens steers toward.
    pub telos_summary: String,
    /// Earned affinity in this EPR context — distinct attested selectors (gaming-resistant).
    pub affinity_in_context: u32,
    /// This lens's current deterministic reading over the EPR; `None` until it fires here.
    pub current_verdict: Option<String>,
    /// Fail-closed flag — `false` = degraded row, excluded from selection but still surfaced.
    pub valid: bool,
}

/// The plural-Mishpat-lens market over one EPR (Operational Category C — assembled per request).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct LensMarketView {
    /// The EPR slug-id this market governs (the scope key, NOT the dag-cbor CID).
    pub epr_scope: String,
    /// The governing lenses, affinity-ranked (deterministic), invalid rows degraded-not-dropped.
    pub lenses: Vec<LensBindingView>,
    /// Controversy spread in [0,1]: 1.0 = perfectly split, 0.0 = unanimous/none.
    pub contention_index: f64,
    /// Regime-drift status: `stable` | `drifting` | `breached` (breached = the joint fusion).
    pub regime_status: String,
    /// Commitment CID of an open fresh-lens bounty for this EPR, or `None`.
    pub open_bounty_cid: Option<String>,
    /// RFC3339 timestamp this projection was assembled.
    pub computed_at: String,
}
