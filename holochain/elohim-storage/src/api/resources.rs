//! Stewarded Resources API controller
//!
//! Routes: `/api/v1/resources[/{id}][/dashboard|/allocations|/usage|/constitutional-limits]`
//!
//! Delegates to `ResourceService` for business logic, which calls
//! `db::stewardship_allocations` and `db::economic_events` for Diesel queries.
//!
//! ## Route Table
//!
//! | Method | Path                                          | Handler               |
//! |--------|-----------------------------------------------|-----------------------|
//! | GET    | /api/v1/resources                             | list resources        |
//! | POST   | /api/v1/resources                             | create resource       |
//! | GET    | /api/v1/resources/{id}                        | get resource          |
//! | GET    | /api/v1/resources/dashboard/{stewardId}       | dashboard             |
//! | POST   | /api/v1/resources/{id}/allocations            | create allocation     |
//! | POST   | /api/v1/resources/{id}/usage                  | record usage          |
//! | GET    | /api/v1/resources/constitutional-limits/{cat} | constitutional limits |

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use std::sync::Arc;

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::resource_service::{
    CreateAllocationRequest, CreateResourceRequest, RecordUsageRequest, ResourceService,
};
use crate::services::{response, Services};

use super::parse_body;

/// Handle `/api/v1/resources*` requests
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
    _services: Option<Arc<Services>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Strip leading slash for easier matching
    let path = resource_path.trim_start_matches('/');

    // Extract query string from the original URI
    let query_string = req.uri().query().unwrap_or("").to_string();

    match (&method, path) {
        // GET /api/v1/resources  — list (optionally filtered by ?stewardId=)
        (&Method::GET, "" | "/") => {
            let steward_id = extract_query_param(&query_string, "stewardId");
            let resources = ResourceService::list_resources(steward_id.as_deref());
            Ok(response::ok(&resources))
        }

        // POST /api/v1/resources  — create resource
        (&Method::POST, "" | "/") => {
            let req_body: CreateResourceRequest = parse_body(req).await?;
            match ResourceService::create_resource(req_body, pool, ctx) {
                Ok(resource) => Ok(response::created(&resource)),
                Err(e) => Ok(response::error_response(e)),
            }
        }

        // GET /api/v1/resources/dashboard/{stewardId}
        (&Method::GET, p) if p.starts_with("dashboard/") => {
            let steward_id = p.trim_start_matches("dashboard/");
            if steward_id.is_empty() {
                return Ok(response::bad_request("Missing stewardId"));
            }
            let dashboard = ResourceService::get_dashboard(steward_id);
            Ok(response::ok(&dashboard))
        }

        // GET /api/v1/resources/constitutional-limits/{category}
        (&Method::GET, p) if p.starts_with("constitutional-limits/") => {
            let category = p.trim_start_matches("constitutional-limits/");
            if category.is_empty() {
                return Ok(response::bad_request("Missing category"));
            }
            match ResourceService::get_constitutional_limit(category) {
                Some(limit) => Ok(response::ok(&limit)),
                None => Ok(response::not_found(&format!(
                    "No constitutional limit defined for category: {}",
                    category
                ))),
            }
        }

        // POST /api/v1/resources/{id}/allocations  — create allocation
        (&Method::POST, p) if p.ends_with("/allocations") => {
            let resource_id = p.trim_end_matches("/allocations");
            if resource_id.is_empty() {
                return Ok(response::bad_request("Missing resource id"));
            }
            let req_body: CreateAllocationRequest = parse_body(req).await?;
            match ResourceService::create_allocation(resource_id, req_body, pool, ctx) {
                Ok(allocation) => Ok(response::created(&allocation)),
                Err(e) => Ok(response::error_response(e)),
            }
        }

        // POST /api/v1/resources/{id}/usage  — record usage
        (&Method::POST, p) if p.ends_with("/usage") => {
            let resource_id = p.trim_end_matches("/usage");
            if resource_id.is_empty() {
                return Ok(response::bad_request("Missing resource id"));
            }
            let req_body: RecordUsageRequest = parse_body(req).await?;
            match ResourceService::record_usage(resource_id, req_body, pool, ctx) {
                Ok(usage) => Ok(response::created(&usage)),
                Err(e) => Ok(response::error_response(e)),
            }
        }

        // GET /api/v1/resources/{id}  — get resource by ID
        (&Method::GET, id) if !id.is_empty() && !id.contains('/') => {
            match ResourceService::get_resource(id) {
                Some(resource) => Ok(response::ok(&resource)),
                None => Ok(response::not_found(&format!("Resource not found: {}", id))),
            }
        }

        _ => Ok(response::not_found(&format!(
            "Unknown resources route: {} {}",
            method, resource_path
        ))),
    }
}

/// Extract a single query parameter value by name from a query string.
fn extract_query_param(query: &str, name: &str) -> Option<String> {
    if query.is_empty() {
        return None;
    }
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        if key == name && !value.is_empty() {
            // Percent-decode simple cases
            return Some(value.replace("%40", "@").replace('+', " "));
        }
    }
    None
}
