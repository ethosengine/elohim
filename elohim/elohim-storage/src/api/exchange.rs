//! Exchange API controller
//!
//! Routes: `/api/v1/exchange[/requests|/offers][/{id}][/match]`
//!
//! Delegates to `ExchangeService` for business logic, which wraps
//! compound 3-write operations (request + intent + event) in Diesel transactions.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::exchange_service::{
    CreateOfferInput, CreateRequestInput, OfferQuery, RequestQuery, UpdateOfferInput,
    UpdateRequestInput,
};
use crate::services::response;
use crate::services::ExchangeService;

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Extract `{id}` from a path segment like `/{id}`, `/{id}/match`, etc.
fn extract_id(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let id = trimmed.split('/').next()?;
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

/// Extract sub-action after `/{id}/` from a resource path
fn extract_action(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    let mut parts = trimmed.splitn(2, '/');
    let _ = parts.next(); // skip id
    parts.next().filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// Archive body (requester_id or offeror_id for ownership check)
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveBody {
    requester_id: Option<String>,
    offeror_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Requests handlers
// ---------------------------------------------------------------------------

async fn list_requests(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query_str = req.uri().query().unwrap_or("");
    let query: RequestQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = get_conn(pool)?;
    let requests = ExchangeService::list_requests(&mut conn, ctx, &query)?;
    Ok(response::ok(&requests))
}

async fn get_request(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(response::from_option(
        ExchangeService::get_request(&mut conn, ctx, id),
        &format!("Request {} not found", id),
    ))
}

async fn create_request(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: CreateRequestInput = parse_body(req).await?;
    let mut conn = get_conn(pool)?;
    Ok(response::from_create_result(
        ExchangeService::create_request(&mut conn, ctx, body),
    ))
}

async fn update_request(
    id: &str,
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: UpdateRequestInput = parse_body(req).await?;
    let mut conn = get_conn(pool)?;
    Ok(response::from_result(ExchangeService::update_request(
        &mut conn, ctx, id, body,
    )))
}

async fn archive_request(
    id: &str,
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: ArchiveBody = parse_body(req).await?;
    let requester_id = body
        .requester_id
        .ok_or_else(|| StorageError::InvalidInput("requesterId is required".into()))?;

    let mut conn = get_conn(pool)?;
    match ExchangeService::archive_request(&mut conn, ctx, id, &requester_id) {
        Ok(()) => Ok(response::no_content()),
        Err(e) => Ok(response::error_response(e)),
    }
}

async fn match_request(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(response::from_result(ExchangeService::match_request(
        &mut conn, ctx, id,
    )))
}

// ---------------------------------------------------------------------------
// Offers handlers
// ---------------------------------------------------------------------------

async fn list_offers(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let query_str = req.uri().query().unwrap_or("");
    let query: OfferQuery = serde_urlencoded::from_str(query_str).unwrap_or_default();

    let mut conn = get_conn(pool)?;
    let offers = ExchangeService::list_offers(&mut conn, ctx, &query)?;
    Ok(response::ok(&offers))
}

async fn get_offer(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(response::from_option(
        ExchangeService::get_offer(&mut conn, ctx, id),
        &format!("Offer {} not found", id),
    ))
}

async fn create_offer(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: CreateOfferInput = parse_body(req).await?;
    let mut conn = get_conn(pool)?;
    Ok(response::from_create_result(ExchangeService::create_offer(
        &mut conn, ctx, body,
    )))
}

async fn update_offer(
    id: &str,
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: UpdateOfferInput = parse_body(req).await?;
    let mut conn = get_conn(pool)?;
    Ok(response::from_result(ExchangeService::update_offer(
        &mut conn, ctx, id, body,
    )))
}

async fn archive_offer(
    id: &str,
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let body: ArchiveBody = parse_body(req).await?;
    let offeror_id = body
        .offeror_id
        .ok_or_else(|| StorageError::InvalidInput("offerorId is required".into()))?;

    let mut conn = get_conn(pool)?;
    match ExchangeService::archive_offer(&mut conn, ctx, id, &offeror_id) {
        Ok(()) => Ok(response::no_content()),
        Err(e) => Ok(response::error_response(e)),
    }
}

async fn match_offer(
    id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    Ok(response::from_result(ExchangeService::match_offer(
        &mut conn, ctx, id,
    )))
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/exchange*` requests
///
/// Sub-routes:
/// - `/exchange/requests[/{id}[/match]]`
/// - `/exchange/offers[/{id}[/match]]`
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let trimmed = resource_path.trim_end_matches('/');

    // Route: /exchange/requests...
    if trimmed.starts_with("/requests") {
        let after_requests = trimmed.strip_prefix("/requests").unwrap_or("");
        let after_trimmed = after_requests.trim_end_matches('/');

        return match (&method, after_trimmed) {
            // GET /exchange/requests  — list
            (&Method::GET, "") | (&Method::GET, "/") => list_requests(req, pool, ctx).await,

            // POST /exchange/requests — create
            (&Method::POST, "") | (&Method::POST, "/") => create_request(req, pool, ctx).await,

            _ => {
                let id = match extract_id(after_trimmed) {
                    Some(id) => id,
                    None => {
                        return Ok(response::not_found(&format!(
                            "Unknown requests route: {} {}",
                            method, resource_path
                        )))
                    }
                };
                let action = extract_action(after_trimmed);

                match (&method, action) {
                    // GET /exchange/requests/{id}
                    (&Method::GET, None) => get_request(id, pool, ctx).await,

                    // PATCH /exchange/requests/{id}
                    (&Method::PATCH, None) => update_request(id, req, pool, ctx).await,

                    // DELETE /exchange/requests/{id}
                    (&Method::DELETE, None) => archive_request(id, req, pool, ctx).await,

                    // POST /exchange/requests/{id}/match
                    (&Method::POST, Some("match")) => match_request(id, pool, ctx).await,

                    _ => Ok(response::not_found(&format!(
                        "Unknown requests route: {} {}",
                        method, resource_path
                    ))),
                }
            }
        };
    }

    // Route: /exchange/offers...
    if trimmed.starts_with("/offers") {
        let after_offers = trimmed.strip_prefix("/offers").unwrap_or("");
        let after_trimmed = after_offers.trim_end_matches('/');

        return match (&method, after_trimmed) {
            // GET /exchange/offers  — list
            (&Method::GET, "") | (&Method::GET, "/") => list_offers(req, pool, ctx).await,

            // POST /exchange/offers — create
            (&Method::POST, "") | (&Method::POST, "/") => create_offer(req, pool, ctx).await,

            _ => {
                let id = match extract_id(after_trimmed) {
                    Some(id) => id,
                    None => {
                        return Ok(response::not_found(&format!(
                            "Unknown offers route: {} {}",
                            method, resource_path
                        )))
                    }
                };
                let action = extract_action(after_trimmed);

                match (&method, action) {
                    // GET /exchange/offers/{id}
                    (&Method::GET, None) => get_offer(id, pool, ctx).await,

                    // PATCH /exchange/offers/{id}
                    (&Method::PATCH, None) => update_offer(id, req, pool, ctx).await,

                    // DELETE /exchange/offers/{id}
                    (&Method::DELETE, None) => archive_offer(id, req, pool, ctx).await,

                    // POST /exchange/offers/{id}/match
                    (&Method::POST, Some("match")) => match_offer(id, pool, ctx).await,

                    _ => Ok(response::not_found(&format!(
                        "Unknown offers route: {} {}",
                        method, resource_path
                    ))),
                }
            }
        };
    }

    Ok(response::not_found(&format!(
        "Unknown exchange route: {} {}",
        method, resource_path
    )))
}
