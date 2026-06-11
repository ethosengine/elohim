//! Reach enum — envelope-level scoping primitive.
//!
//! Protocol-owned. No app may redefine what these mean. Gateways enforce
//! reach rules without parsing payload.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Note: Ord/PartialOrd are intentionally NOT derived. The declaration order
// (Private first, Commons last) is from most restrictive to most open per the
// DNA-notarized CORE_REACH_LEVELS vocabulary. A derived `Ord` would make
// `Private < Commons`, which is the opposite of the semantic openness ranking
// that `openness()` exposes (Commons = most open = 8). Use `openness()` for
// comparisons; do not add `#[derive(Ord)]` here without re-reviewing semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../../sdk/epr-ts/src/generated/")]
pub enum Reach {
    /// Fully private; outside the substrate's public surface.
    Private,
    /// Scope of the authoring self / owner only.
    #[serde(rename = "self")]
    SelfScope,
    /// Intimate circle — closest relationships.
    Intimate,
    /// Trusted relationships — beyond intimate.
    Trusted,
    /// Familiar contacts — known, not close.
    Familiar,
    /// Open within the broader community / network.
    Community,
    /// Public — openly visible on the substrate.
    Public,
    /// Commons-level — maximally open, cooperatively held.
    Commons,
}

impl Reach {
    /// Monotonically increasing openness score (1 = most restrictive, 8 = most open).
    /// Matches the CORE_REACH_LEVELS declaration order (private → commons).
    pub const fn openness(self) -> u8 {
        match self {
            Reach::Private => 1,
            Reach::SelfScope => 2,
            Reach::Intimate => 3,
            Reach::Trusted => 4,
            Reach::Familiar => 5,
            Reach::Community => 6,
            Reach::Public => 7,
            Reach::Commons => 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::REACH_OPENNESS;

    /// Pins the hand-written `openness()` match to the schema-generated
    /// `REACH_OPENNESS` slice (codegen emits it from reach.schema.json's
    /// `_ordinal` marker). The enum match stays exhaustive/const; this test
    /// fails the moment it drifts from the schema's declared ordinal order.
    #[test]
    fn openness_matches_generated_ordinal() {
        // One entry per Reach variant. A mismatch means reach.rs and reach.schema.json
        // are out of sync — regenerate: pnpm run schema:codegen:rs
        assert_eq!(
            REACH_OPENNESS.len(),
            8,
            "REACH_OPENNESS length != Reach variant count"
        );
        for (name, score) in REACH_OPENNESS {
            let r: Reach =
                serde_json::from_value(serde_json::Value::String((*name).into())).unwrap();
            assert_eq!(r.openness(), *score, "openness drift for {name}");
        }
    }
}
