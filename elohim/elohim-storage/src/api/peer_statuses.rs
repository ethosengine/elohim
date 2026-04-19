//! PeerStatus read API controller
//!
//! Routes:
//! - `GET /api/v1/peer-statuses` — List current PeerStatus projection rows
//!   Optional query parameter: `?householdId=<id>` (falls back to list_all
//!   until Task C3 adds `stewarded_nodes.household_id` to the schema).

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};

use crate::db::{peer_statuses as db, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::PeerStatusView;

use super::get_conn;

/// Handle `/api/v1/peer-statuses*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    Ok(match (&method, path) {
        // GET /api/v1/peer-statuses  (path is "" after stripping the prefix)
        (&Method::GET, "") => handle_list(req, pool).await,
        _ => response::not_found(&format!(
            "Unknown peer-statuses route: {} /api/v1/peer-statuses/{}",
            method, path
        )),
    })
}

async fn handle_list(req: Request<Incoming>, pool: &DbPool) -> Response<Full<Bytes>> {
    // Parse optional ?householdId= query param
    let household = req
        .uri()
        .query()
        .and_then(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .find(|(k, _)| k == "householdId")
                .map(|(_, v)| v.into_owned())
        });

    let mut conn = match get_conn(pool) {
        Ok(c) => c,
        Err(_) => return response::internal_error("pool unavailable"),
    };

    let rows = match household {
        Some(h) => db::list_by_household(&mut conn, &h),
        None => db::list_current(&mut conn),
    };

    match rows {
        Ok(rs) => {
            let views: Vec<PeerStatusView> = rs.into_iter().map(Into::into).collect();
            response::ok(&views)
        }
        Err(e) => response::internal_error(&format!("{e}")),
    }
}
