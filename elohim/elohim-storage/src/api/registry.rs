//! Enum Registry API controller
//!
//! Routes:
//!   GET  `/api/v1/registry/{enumName}` — list registered values
//!   POST `/api/v1/registry/{enumName}` — register a new value
//!
//! Category C (operational, SQLite-only). No DHT provenance.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;

use super::{get_conn, parse_body};

/// Handle `/api/v1/registry*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let enum_name = resource_path.trim_start_matches('/');

    if enum_name.is_empty() {
        return Ok(response::bad_request("Missing enum name in path"));
    }

    match method {
        Method::GET => Ok(handle_list(enum_name, pool, ctx).await),
        Method::POST => Ok(handle_register(req, enum_name, pool, ctx).await),
        _ => Ok(response::not_found(&format!(
            "Unknown registry route: {} /api/v1/registry/{}",
            method, enum_name
        ))),
    }
}

async fn handle_list(
    enum_name: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::enum_registry::list_enum_values(&mut conn, ctx, enum_name) {
        Ok(items) => {
            let values: Vec<&str> = items.iter().map(|e| e.enum_value.as_str()).collect();
            response::ok(&values)
        }
        Err(e) => response::error_response(e),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterValueInput {
    value: String,
    #[serde(default = "default_tier")]
    tier: String,
    added_by: Option<String>,
}

fn default_tier() -> String {
    "extensible".to_string()
}

async fn handle_register(
    req: Request<Incoming>,
    enum_name: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Response<Full<Bytes>> {
    let input: RegisterValueInput = match parse_body(req).await {
        Ok(v) => v,
        Err(_) => return response::bad_request("Invalid JSON body"),
    };

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => return response::error_response(e),
    };

    match crate::db::enum_registry::register_enum_value(
        &mut conn,
        ctx,
        enum_name,
        &input.value,
        &input.tier,
        input.added_by.as_deref(),
    ) {
        Ok(()) => response::ok(&serde_json::json!({
            "status": "registered",
            "enumName": enum_name,
            "value": input.value,
        })),
        Err(e) => response::error_response(e),
    }
}
