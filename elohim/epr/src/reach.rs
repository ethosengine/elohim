//! Reach enum — envelope-level scoping primitive.
//!
//! Protocol-owned. No app may redefine what these mean. Gateways enforce
//! reach rules without parsing payload.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Note: Ord/PartialOrd are intentionally NOT derived. The declaration order
// (Commons first, Private last) is opposite to the semantic openness ranking
// that `openness()` returns (Commons = most open = 5). A derived `Ord` would
// make `Commons < Private`, which is the opposite of any operator's intuition.
// Use `openness()` for comparisons; do not add `#[derive(Ord)]` here without
// first reversing the variant declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../sdk/epr-ts/src/generated/")]
pub enum Reach {
    /// Open to all — commons-level content.
    Commons,
    /// Open within the broader community / network.
    Community,
    /// Scoped to a specific collective / affinity group.
    Collective,
    /// Visible only to explicit stewards.
    Steward,
    /// Fully private; outside the substrate's public surface.
    Private,
}

impl Reach {
    /// Monotonically decreasing openness score (5 = most open, 1 = most closed).
    pub const fn openness(self) -> u8 {
        match self {
            Reach::Commons => 5,
            Reach::Community => 4,
            Reach::Collective => 3,
            Reach::Steward => 2,
            Reach::Private => 1,
        }
    }
}
