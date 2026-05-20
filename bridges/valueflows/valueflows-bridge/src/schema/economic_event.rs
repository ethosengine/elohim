//! VF `EconomicEvent` GraphQL object + M1 fixture resolver.
//!
//! Every resolve call writes a TranslationPoint observation to the ledger
//! so the M5 upstream-contribution + R&O compatibility reports have
//! coverage data even for the M1 tracer-bullet path.

use async_graphql::{Object, ID};
use chrono::Utc;
use diesel::prelude::*;
use valueflows_types::{
    ClientCapability, Direction, OntologicalCommitment, SemanticCost, TranslationKind,
    TranslationPoint,
};

use super::BridgeContext;

pub struct EconomicEventGql {
    pub(crate) id: String,
    pub(crate) action: String,
    pub(crate) provider_id: String,
    pub(crate) receiver_id: String,
    pub(crate) note: Option<String>,
}

impl EconomicEventGql {
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

    /// VF Agent id of the provider (M1: scalar ID; M3 swaps to Agent reference).
    #[graphql(name = "provider")]
    async fn provider_id(&self) -> ID {
        ID::from(self.provider_id.clone())
    }

    /// VF Agent id of the receiver (M1: scalar ID; M3 swaps to Agent reference).
    #[graphql(name = "receiver")]
    async fn receiver_id(&self) -> ID {
        ID::from(self.receiver_id.clone())
    }

    /// Optional free-form note.
    async fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

/// M1 resolve: synthesize fixture, log observation, return.
pub async fn resolve(
    ctx: &BridgeContext,
    id: String,
) -> async_graphql::Result<Option<EconomicEventGql>> {
    let point = TranslationPoint {
        at_iso: Utc::now().to_rfc3339(),
        at_block_height: None,
        direction: Direction::Read,
        vf_type: "EconomicEvent".to_string(),
        elohim_source: "fixture".to_string(),
        translation_kind: TranslationKind::IdentityShape,
        semantic_cost: SemanticCost::Mechanical,
        ontological_commitment: None,
        client_capability: ClientCapability::StockVf,
        code_location: concat!(file!(), ":", line!()).to_string(),
        notes: Some("M1 tracer bullet — fixture resolver".to_string()),
    };

    // Fire-and-forget the ledger write in a blocking task so the Tokio
    // executor isn't starved by r2d2 pool.get(). Resolver succeeds even
    // if the ledger insert fails (the response is the canonical artifact).
    let ctx_clone = ctx.clone();
    let observation_result =
        tokio::task::spawn_blocking(move || write_observation(&ctx_clone, point)).await;
    match observation_result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("translation_observations insert failed: {e}"),
        Err(e) => tracing::warn!("translation_observations spawn_blocking panic: {e}"),
    }

    Ok(Some(EconomicEventGql::fixture(id)))
}

/// Insert the observation via a raw Diesel call. We inline the SQL here in
/// M1 to avoid a circular dependency with elohim-storage (which consumes
/// this crate). In M2+ this is refactored to call back into elohim-storage's
/// `db::translation_observations::insert_observation`.
///
/// IMPORTANT: uses `.as_ledger_str()` (not `format!("{:?}", …)`) for all
/// enum-to-string conversions. Debug format is not a stable wire format; a
/// custom Debug impl on any enum would silently break the CHECK constraints
/// in the `translation_observations` migration.
///
// Note: block_height is intentionally omitted from this INSERT — TranslationPoint::at_block_height
// is always None in M1 (no conductor context). M3+ will add the column to the INSERT and switch
// to elohim-storage's `db::translation_observations::insert_observation` once the circular-dep
// concern is resolved (likely by exposing valueflows-types as the canonical helper home).
fn write_observation(
    ctx: &BridgeContext,
    p: TranslationPoint,
) -> Result<(), diesel::result::Error> {
    use diesel::sql_query;
    let mut conn = ctx
        .pool
        .get()
        .map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;

    let ontological: Option<String> = p
        .ontological_commitment
        .map(OntologicalCommitment::as_ledger_str)
        .map(str::to_string);

    sql_query(
        r#"INSERT INTO translation_observations
        (observed_at, direction, vf_type, elohim_source, translation_kind,
         semantic_cost, ontological_commitment, client_capability,
         code_location, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind::<diesel::sql_types::Text, _>(p.at_iso)
    .bind::<diesel::sql_types::Text, _>(p.direction.as_ledger_str())
    .bind::<diesel::sql_types::Text, _>(p.vf_type)
    .bind::<diesel::sql_types::Text, _>(p.elohim_source)
    .bind::<diesel::sql_types::Text, _>(p.translation_kind.as_ledger_str())
    .bind::<diesel::sql_types::Text, _>(p.semantic_cost.as_ledger_str())
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(ontological)
    .bind::<diesel::sql_types::Text, _>(p.client_capability.as_ledger_str())
    .bind::<diesel::sql_types::Text, _>(p.code_location)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(p.notes)
    .execute(&mut conn)?;
    Ok(())
}
