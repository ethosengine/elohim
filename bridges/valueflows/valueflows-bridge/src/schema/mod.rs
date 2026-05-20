//! GraphQL schema for the valueflows bridge.
//!
//! M1: minimal schema with `EconomicEvent` returning fixture data.
//! M2+ adds Agent + identity bridge.
//! M3+ adds Proposal/Intent + authority gate + real hREA projection.

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema, ID};

pub mod economic_event;

pub use economic_event::EconomicEventGql;

/// M1 schema entry point. Empty mutation + subscription; queries return
/// fixture data only.
pub type BridgeSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema() -> BridgeSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish()
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Look up a VF EconomicEvent by id. M1 returns fixture data for any id.
    async fn economic_event(&self, id: ID) -> Option<EconomicEventGql> {
        Some(EconomicEventGql::fixture(id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Request;

    #[tokio::test]
    async fn schema_serves_fixture_economic_event() {
        let schema = build_schema();
        let req = Request::new(
            r#"query { economicEvent(id: "test-id") { id action provider receiver note } }"#
                .to_string(),
        );
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "errors: {:?}", resp.errors);
        let data = resp.data.into_json().expect("data");
        assert_eq!(data["economicEvent"]["id"], "test-id");
        assert!(!data["economicEvent"]["action"].is_null());
        assert_eq!(data["economicEvent"]["provider"], "agent-fixture-provider");
        assert_eq!(data["economicEvent"]["receiver"], "agent-fixture-receiver");
        let note_text = data["economicEvent"]["note"]
            .as_str()
            .expect("note is a string");
        assert!(
            note_text.contains("M1 tracer-bullet"),
            "fixture note should mention M1: got {note_text}"
        );
    }
}
