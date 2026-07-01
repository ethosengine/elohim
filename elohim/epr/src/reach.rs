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

    /// N8 guardrail: pins the canonical Reach vocabulary pair — this enum's
    /// serde serialization ↔ `reach.schema.json`'s `enum` array — before any
    /// reconciliation of the drifted reach vocabularies. The schema declares
    /// `_ordinal: true` (see `codegen-ordinal.test.mjs`): codegen derives
    /// positional ordinals (index + 1 = openness), so ORDER is significant —
    /// this asserts ordered equality, not just set equality.
    #[test]
    fn serde_matches_schema_enum_values_and_order() {
        // Declaration-order list of every variant. The exhaustive match below
        // is the compile-time pin: adding a `Reach` variant breaks it, forcing
        // this list (and the schema) to be updated together.
        const ALL: [Reach; 8] = [
            Reach::Private,
            Reach::SelfScope,
            Reach::Intimate,
            Reach::Trusted,
            Reach::Familiar,
            Reach::Community,
            Reach::Public,
            Reach::Commons,
        ];
        for r in ALL {
            match r {
                Reach::Private
                | Reach::SelfScope
                | Reach::Intimate
                | Reach::Trusted
                | Reach::Familiar
                | Reach::Community
                | Reach::Public
                | Reach::Commons => {}
            }
        }

        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../sdk/schemas/v1/enums/reach.schema.json"))
                .expect("reach.schema.json parses as JSON");

        // `_ordinal: true` is what makes order load-bearing (positional
        // ordinal codegen). If this marker is ever dropped, revisit the
        // ordered-equality assertion below.
        assert_eq!(
            schema["_ordinal"],
            serde_json::Value::Bool(true),
            "reach.schema.json lost its _ordinal: true marker — ordering semantics changed"
        );

        let schema_values: Vec<&str> = schema["enum"]
            .as_array()
            .expect("reach.schema.json has an enum array")
            .iter()
            .map(|v| v.as_str().expect("enum values are strings"))
            .collect();

        let serialized: Vec<String> = ALL
            .iter()
            .map(|r| {
                serde_json::to_value(r)
                    .unwrap()
                    .as_str()
                    .expect("Reach serializes as a JSON string")
                    .to_owned()
            })
            .collect();

        // Sets AND order in one assertion: ordered equality over the full
        // exhaustive variant list. With _ordinal: true the position IS the
        // openness score, so a reorder is as breaking as a rename.
        assert_eq!(
            serialized, schema_values,
            "Reach serde serialization drifted from reach.schema.json enum (values and/or order)"
        );

        // Positional ordinal ↔ openness: index + 1 == openness(), the exact
        // invariant codegen relies on when emitting REACH_OPENNESS.
        for (i, r) in ALL.iter().enumerate() {
            assert_eq!(
                r.openness() as usize,
                i + 1,
                "openness() of {r:?} disagrees with its schema ordinal position"
            );
        }
    }

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
