//! VF `EconomicEvent` GraphQL object.
//!
//! M1: fixture resolver returns deterministic synthesized data. The fixture
//! shape matches VF's canonical `EconomicEvent` (see
//! `/projects/research/vf-graphql/lib/schemas/observation.gql`). M3+ replaces
//! the fixture with real reads from hREA DNA.

use async_graphql::{Object, ID};

/// VF EconomicEvent — minimal M1 surface. Canonical VF fields (per
/// `/projects/research/vf-graphql/lib/schemas/observation.gql`); elohim
/// extensions (Reach, sidecar links) land in M3+.
pub struct EconomicEventGql {
    pub id: String,
    pub action: String,        // VF action id (e.g., "transfer", "use")
    pub provider_id: String,   // VF Agent id (M1 fixture)
    pub receiver_id: String,   // VF Agent id (M1 fixture)
    pub note: Option<String>,
}

impl EconomicEventGql {
    /// Synthesize a fixture EconomicEvent for the M1 tracer bullet.
    ///
    /// The id passed in is echoed back so callers can verify the route
    /// is exercising the right resolver code path.
    pub fn fixture(id: String) -> Self {
        Self {
            id,
            action: "transfer".to_string(),
            provider_id: "agent-fixture-provider".to_string(),
            receiver_id: "agent-fixture-receiver".to_string(),
            note: Some("M1 tracer-bullet fixture; M3 will return real hREA data".to_string()),
        }
    }
}

#[Object]
impl EconomicEventGql {
    /// VF EconomicEvent identifier.
    async fn id(&self) -> ID {
        ID::from(self.id.clone())
    }

    /// VF action id describing the kind of event.
    async fn action(&self) -> &str {
        &self.action
    }

    /// VF Agent id of the provider.
    async fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// VF Agent id of the receiver.
    async fn receiver_id(&self) -> &str {
        &self.receiver_id
    }

    /// Optional free-form note.
    async fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}
