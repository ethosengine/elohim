//! HTTP response building helpers
//!
//! Provides a consistent API for building HTTP responses across all handlers.
//! Reduces boilerplate and ensures consistent error formatting.

use bytes::Bytes;
use hyper::{header, Response, StatusCode};
use http_body_util::Full;
use serde::Serialize;

use crate::error::StorageError;

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

/// Convert a StorageError to an appropriate HTTP response
pub fn error_response(error: StorageError) -> Response<Full<Bytes>> {
    let (status, message) = match &error {
        StorageError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
        StorageError::BlobNotFound(msg) => (StatusCode::NOT_FOUND, format!("Blob not found: {}", msg)),
        StorageError::HashMismatch { expected, actual } => (
            StatusCode::CONFLICT,
            format!("Hash mismatch: expected {}, got {}", expected, actual),
        ),
        StorageError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
        StorageError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
        StorageError::Parse(msg) => (StatusCode::BAD_REQUEST, format!("Parse error: {}", msg)),
        StorageError::Json(e) => (StatusCode::BAD_REQUEST, format!("JSON error: {}", e)),
        StorageError::Connection(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
        StorageError::Timeout(msg) => (StatusCode::GATEWAY_TIMEOUT, msg.clone()),
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
}
