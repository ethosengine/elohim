//! GraphQL schema construction and hyper handler.
//!
//! This crate uses hyper directly (not axum), so `async-graphql-axum` is not available.
//! Instead we hand-roll the handler:
//!   1. Collect body bytes via `http_body_util::BodyExt::collect`.
//!   2. Deserialize as `async_graphql::Request` (serde).
//!   3. Execute against the schema built from `build_schema`.
//!   4. Serialize the response as JSON.
//!   5. Return `Response<Full<Bytes>>`.
//!
//! ## Schema lifetime
//!
//! `AppSchema` is `Clone + Send + Sync`. In production, Phase 8 will build it
//! once at startup and store it in the `EprFanOutCtx`. For Phase 7 the schema
//! is built per-request from the `Arc<GraphEngine>` — inexpensive because the
//! build just stores the Arc and compiles the static SDL.

#![cfg(feature = "graph-native")]

use std::sync::Arc;

use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

use crate::db::DbPool;
use crate::error::StorageError;
use crate::graph::engine::GraphEngine;
use crate::graphql::resolvers::QueryRoot;
use crate::services::federator::Federator;

/// The compiled schema type.
pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

/// Build the GraphQL schema, injecting shared context data.
///
/// `graph_engine` — CozoDB engine for EPR graph queries (existing resolvers).
/// `pool`         — Diesel connection pool for topology resolvers (viewer.hub etc.).
/// `federator`    — View-federation handle for cluster fan-out.
///
/// The schema is `Clone + Send + Sync` — callers may store it in an `Arc` or
/// clone it cheaply per request. All Arcs are reference-counted; no additional
/// heap allocation beyond what the schema's internal registry needs.
pub fn build_schema(
    graph_engine: Arc<GraphEngine>,
    pool: DbPool,
    federator: Arc<Federator>,
) -> AppSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(graph_engine)
        .data(pool)
        .data(federator)
        .finish()
}

/// Handle `POST /api/v1/graphql` and `GET /api/v1/graphql`.
///
/// GET returns a minimal JSON hint. POST executes the GraphQL query against the
/// schema built from `graph_engine`. When `graph_engine` is `None` (engine not
/// yet initialised — Phase 8 owns main.rs wiring), returns 503.
///
/// `pool` and `federator` are optional — when either is `None` the schema is
/// still built (existing EPR/contributor resolvers still work), but any
/// `viewer.hub` call will return a resolver error rather than panic.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    graph_engine: Option<&Arc<GraphEngine>>,
    pool: Option<&DbPool>,
    federator: Option<&Arc<Federator>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let Some(engine) = graph_engine else {
        return Ok(Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(
                r#"{"errors":[{"message":"GraphQL engine not available"}]}"#,
            )))
            .unwrap());
    };

    // Build schema with whatever context is available. When pool or federator is
    // absent, viewer.hub will return a resolver error instead of a 503.
    let schema = match (pool, federator) {
        (Some(p), Some(f)) => build_schema(Arc::clone(engine), p.clone(), Arc::clone(f)),
        _ => {
            // Fallback: build without pool/federator — existing EPR resolvers still work;
            // viewer.hub will return "requested extension is not available" error.
            Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
                .data(Arc::clone(engine))
                .finish()
        }
    };

    match method {
        Method::POST => execute_graphql(req, &schema).await,
        Method::GET => {
            // Minimal hint; full playground is out of scope for Phase 7.
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"message":"Send POST requests to this endpoint with a JSON body: {\"query\":\"...\"}"}"#,
                )))
                .unwrap())
        }
        _ => Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::new()))
            .unwrap()),
    }
}

async fn execute_graphql(
    req: Request<Incoming>,
    schema: &AppSchema,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Collect the full body.
    let body_bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|e| StorageError::Internal(format!("Failed to read GraphQL body: {}", e)))?
        .to_bytes();

    // Deserialize the GraphQL request.
    let gql_req: async_graphql::Request = serde_json::from_slice(&body_bytes)
        .map_err(|e| StorageError::InvalidInput(format!("Invalid GraphQL request JSON: {}", e)))?;

    // Execute.
    let gql_response = schema.execute(gql_req).await;

    // Serialize.
    let response_bytes = serde_json::to_vec(&gql_response).map_err(|e| {
        StorageError::Internal(format!("Failed to serialize GraphQL response: {}", e))
    })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(response_bytes)))
        .unwrap())
}
