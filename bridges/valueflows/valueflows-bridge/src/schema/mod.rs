//! GraphQL schema for the valueflows bridge.
//!
//! M1: minimal schema with `EconomicEvent` returning fixture data + writing
//! a TranslationPoint observation to the learning ledger.

use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema, ID};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;

pub mod economic_event;

pub use economic_event::EconomicEventGql;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Context injected into the schema; held in async-graphql's data so resolvers
/// can grab the pool to log TranslationPoints.
#[derive(Clone)]
pub struct BridgeContext {
    pub pool: DbPool,
}

pub type BridgeSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(ctx: BridgeContext) -> BridgeSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(ctx)
        .finish()
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Look up a VF EconomicEvent by id. M1 returns fixture data and logs a
    /// TranslationPoint observation for every call.
    async fn economic_event(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: ID,
    ) -> async_graphql::Result<Option<EconomicEventGql>> {
        let bridge_ctx = ctx.data::<BridgeContext>()?;
        economic_event::resolve(bridge_ctx, id.to_string()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_builds_with_empty_pool() {
        // Build a pool against an in-memory sqlite for the schema test.
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("build pool");
        let ctx = BridgeContext { pool };
        let _ = build_schema(ctx);
    }
}
