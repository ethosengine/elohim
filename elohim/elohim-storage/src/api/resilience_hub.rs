//! `/api/v1/resilience/{content_id}/hub` handler — polymorphic hub projection.
//!
//! Route: `GET /api/v1/resilience/<content_id>/hub`
//!
//! Returns a `ResilienceHubView` listing every hub (dwelling / collective /
//! computed) known to hold at least one replica of the named content item,
//! derived from `peer_blob_inventory` + peer-identity bindings.
//!
//! Operational Category C — no DHT entry; projection only.
//!
//! See `services/resilience.rs` for classification logic and hub-kind doctrine.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use elohim_views::ResilienceHubView;

pub async fn handle(
    _req: Request<hyper::body::Incoming>,
    method: Method,
    content_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    if method != Method::GET {
        return Ok(response::not_found(&format!(
            "Unknown resilience hub method: {}",
            method
        )));
    }

    match crate::services::resilience::hub_summary(pool, ctx, content_id) {
        Ok(hubs) => {
            let view = ResilienceHubView {
                content_id: content_id.to_string(),
                hubs,
            };
            Ok(response::ok(&view))
        }
        Err(e) => {
            tracing::warn!(
                content_id = %content_id,
                error = %e,
                "resilience hub projection failed"
            );
            Ok(response::internal_error(&format!(
                "resilience hub: {e}"
            )))
        }
    }
}
