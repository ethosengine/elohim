//! HTTP response building helpers
//!
//! Provides a consistent API for building HTTP responses across all handlers.
//! Reduces boilerplate and ensures consistent error formatting.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, Response, StatusCode};
use serde::Serialize;

use crate::error::StorageError;
use crate::views::SUPPORTED_SCHEMA_VERSIONS;

/// Build a JSON response with the given status code
pub fn json_response<T: Serialize>(status: StatusCode, body: &T) -> Response<Full<Bytes>> {
    let json = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}

/// Build a JSON response with 200 OK status
pub fn ok<T: Serialize>(body: &T) -> Response<Full<Bytes>> {
    json_response(StatusCode::OK, body)
}

/// Build a JSON response with 201 Created status
pub fn created<T: Serialize>(body: &T) -> Response<Full<Bytes>> {
    json_response(StatusCode::CREATED, body)
}

/// Build a JSON response with 200 OK status and X-Supported-Schema-Versions header.
/// Used by bulk endpoints to advertise supported schema versions to clients.
pub fn ok_with_schema_info<T: Serialize>(body: &T) -> Response<Full<Bytes>> {
    let versions = SUPPORTED_SCHEMA_VERSIONS
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let json = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Supported-Schema-Versions", versions)
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}

/// Build an empty response with 204 No Content status
pub fn no_content() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Build a 404 Not Found response with message
pub fn not_found(message: &str) -> Response<Full<Bytes>> {
    json_response(
        StatusCode::NOT_FOUND,
        &serde_json::json!({ "error": message }),
    )
}

/// Build a 400 Bad Request response with message
pub fn bad_request(message: &str) -> Response<Full<Bytes>> {
    json_response(
        StatusCode::BAD_REQUEST,
        &serde_json::json!({ "error": message }),
    )
}

/// Build a 405 Method Not Allowed response
pub fn method_not_allowed() -> Response<Full<Bytes>> {
    json_response(
        StatusCode::METHOD_NOT_ALLOWED,
        &serde_json::json!({ "error": "Method not allowed" }),
    )
}

/// Build a 409 Conflict response with message
pub fn conflict(message: &str) -> Response<Full<Bytes>> {
    json_response(
        StatusCode::CONFLICT,
        &serde_json::json!({ "error": message }),
    )
}

/// Build a 403 Forbidden response with structured body
pub fn forbidden<T: Serialize>(body: &T) -> Response<Full<Bytes>> {
    json_response(StatusCode::FORBIDDEN, body)
}

/// Build a 500 Internal Server Error response with message
pub fn internal_error(message: &str) -> Response<Full<Bytes>> {
    json_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        &serde_json::json!({ "error": message }),
    )
}

/// Build a 503 Service Unavailable response with message
pub fn service_unavailable(message: &str) -> Response<Full<Bytes>> {
    json_response(
        StatusCode::SERVICE_UNAVAILABLE,
        &serde_json::json!({ "error": message }),
    )
}

/// Build a propagated-backpressure shed response: 503 + Retry-After +
/// X-Available-Permits + structured {status:"catching-up", retryAfter:N} body.
/// (Named per the inbound-admission plan; uses 503 for saturation consistency.)
pub fn too_many_requests_with_retry(
    retry_after_secs: u64,
    available: usize,
) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "status": "catching-up",
        "retryAfter": retry_after_secs,
    });
    let json = body.to_string();
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::RETRY_AFTER, retry_after_secs.to_string())
        .header("X-Available-Permits", available.to_string())
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}

/// The wire shape of a conductor-admission shed: the same catching-up 503 that
/// storage's own request-admission ceiling emits.
///
/// ONE home for both pools. A shed says the conductor never saw the call, so the
/// honest answer is "come back", not "it broke" — and a caller that must guess
/// which of the two gates refused it cannot write a single retry rule.
pub fn admission_shed_backpressure() -> Response<Full<Bytes>> {
    let gate = crate::conductor_admission::admission();
    let available = gate.capacity().saturating_sub(gate.in_flight()) as usize;
    too_many_requests_with_retry(crate::conductor_admission::SHED_RETRY_AFTER_SECS, available)
}

/// Convert a StorageError to an appropriate HTTP response
pub fn error_response(error: StorageError) -> Response<Full<Bytes>> {
    // Classified FIRST, and by MARKER rather than variant: a shed rides
    // `Timeout`, whose mapping below (504) would tell the caller the conductor
    // took too long — the exact opposite of what happened, which is that the
    // conductor was never asked.
    if crate::conductor_admission::is_admission_shed(&error) {
        return admission_shed_backpressure();
    }
    let (status, message) = match &error {
        StorageError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
        StorageError::AlreadyExists(msg) => (StatusCode::CONFLICT, msg.clone()),
        StorageError::BlobNotFound(msg) => {
            (StatusCode::NOT_FOUND, format!("Blob not found: {}", msg))
        }
        StorageError::HashMismatch { expected, actual } => (
            StatusCode::CONFLICT,
            format!("Hash mismatch: expected {}, got {}", expected, actual),
        ),
        StorageError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
        StorageError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
        // D.4: author-side reach earning refused → 403 Forbidden (author lacks earned reach).
        StorageError::Unauthorized(msg) => (StatusCode::FORBIDDEN, msg.clone()),
        StorageError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
        StorageError::Parse(msg) => (StatusCode::BAD_REQUEST, format!("Parse error: {}", msg)),
        StorageError::Json(e) => (StatusCode::BAD_REQUEST, format!("JSON error: {}", e)),
        StorageError::Connection(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
        StorageError::Timeout(msg) => (StatusCode::GATEWAY_TIMEOUT, msg.clone()),
        // Conductor unavailable (e.g. lamad bridge not yet connected) is a transient
        // readiness failure, not a hard error — 503 so seeders/clients can retry.
        StorageError::Conductor(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };

    json_response(status, &serde_json::json!({ "error": message }))
}

/// Build a binary response with the given content type
pub fn binary_response(
    status: StatusCode,
    content_type: &str,
    body: Vec<u8>,
) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Build a streaming response placeholder (for blob streaming)
pub fn stream_response(content_type: &str, content_length: u64) -> hyper::http::response::Builder {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_LENGTH, content_length)
}

/// Result type alias for handlers
pub type HandlerResult = Result<Response<Full<Bytes>>, StorageError>;

/// Wrap a service result into an HTTP response
pub fn from_result<T: Serialize>(result: Result<T, StorageError>) -> Response<Full<Bytes>> {
    match result {
        Ok(value) => ok(&value),
        Err(e) => error_response(e),
    }
}

/// Wrap an optional service result into an HTTP response
/// Returns 404 if None
pub fn from_option<T: Serialize>(
    result: Result<Option<T>, StorageError>,
    not_found_msg: &str,
) -> Response<Full<Bytes>> {
    match result {
        Ok(Some(value)) => ok(&value),
        Ok(None) => not_found(not_found_msg),
        Err(e) => error_response(e),
    }
}

/// Wrap a create result into an HTTP response with 201 Created
pub fn from_create_result<T: Serialize>(result: Result<T, StorageError>) -> Response<Full<Bytes>> {
    match result {
        Ok(value) => created(&value),
        Err(e) => error_response(e),
    }
}

/// Wrap a delete result into an HTTP response with 204 No Content
pub fn from_delete_result(result: Result<(), StorageError>) -> Response<Full<Bytes>> {
    match result {
        Ok(()) => no_content(),
        Err(e) => error_response(e),
    }
}

/// Wrap a delete result (bool) into an HTTP response
/// Returns 204 No Content if deleted, 404 Not Found if not found
pub fn from_delete_bool_result(
    result: Result<bool, StorageError>,
    not_found_msg: &str,
) -> Response<Full<Bytes>> {
    match result {
        Ok(true) => no_content(),
        Ok(false) => not_found(not_found_msg),
        Err(e) => error_response(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_response() {
        let resp = ok(&serde_json::json!({"test": true}));
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn test_error_response_not_found() {
        let resp = error_response(StorageError::NotFound("test".into()));
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_response_invalid_input() {
        let resp = error_response(StorageError::InvalidInput("bad field".into()));
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn too_many_requests_is_503_with_retry_and_permits_and_body() {
        let resp = too_many_requests_with_retry(2, 0);
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(resp.headers().get(header::RETRY_AFTER).unwrap(), "2");
        assert_eq!(resp.headers().get("X-Available-Permits").unwrap(), "0");
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    /// A conductor-admission shed is BACKPRESSURE, not a failure verdict: the
    /// conductor never saw the call, so the caller must be told to come back —
    /// not told the work was attempted and broke. Measured live 2026-08-18
    /// against a local island conductor: the shed reached the wire as a plain
    /// `500 Error: Conductor error: ... conductor unavailable`, with no
    /// Retry-After and no way for any client to classify it.
    #[test]
    fn an_admission_shed_answers_backpressure_not_a_failure_verdict() {
        let resp = error_response(shed_err());
        assert_eq!(
            resp.status(),
            StatusCode::SERVICE_UNAVAILABLE,
            "a shed must not read as a failure verdict"
        );
        assert_eq!(
            resp.headers().get(header::RETRY_AFTER).unwrap(),
            "2",
            "a shed must tell the caller when to come back"
        );
    }

    /// The shed body must be the SAME catching-up shape storage's own
    /// request-admission ceiling already emits, so one client retry rule covers
    /// both pools — a caller neither can nor should tell which one shed it.
    #[test]
    fn an_admission_shed_uses_the_same_catching_up_body_as_the_request_ceiling() {
        let resp = error_response(shed_err());
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        assert!(
            resp.headers().get("X-Available-Permits").is_some(),
            "a shed must report the gate occupancy it refused against"
        );
    }

    /// Non-shed timeouts keep their existing mapping — the classification must
    /// key on the shed marker, never on the `Timeout` variant as a whole.
    #[test]
    fn a_plain_timeout_is_not_reclassified_as_backpressure() {
        let resp = error_response(StorageError::Timeout("upstream read timed out".into()));
        assert_eq!(resp.status(), StatusCode::GATEWAY_TIMEOUT);
    }

    /// The shed shape `ConductorAdmission::shed_error` actually produces.
    fn shed_err() -> StorageError {
        StorageError::Timeout(format!(
            "{}: no conductor permit for imagodei within 100ms \
             (class=interactive, capacity=4, in_flight=4) — nothing was dispatched",
            crate::conductor_admission::ADMISSION_SHED_MARKER
        ))
    }
}
