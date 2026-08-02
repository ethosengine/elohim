//! DID document and identity endpoints
//!
//! Serves the doorway's public DID resolution surfaces (DID bridge design §3.5):
//!   - `GET /.well-known/did.json` — the doorway's own `did:web` document.
//!   - `GET /1.0/identifiers/{did}` — universal-resolver-compatible resolution
//!     (did:key locally/offline, did:elohim forwarded to storage, else
//!     methodNotSupported).
//!
//! DID documents are assembled with the `did-types` / `did-bridge` crates
//! (`bridges/did`) so the surfaces are standards-legible and schema-conformant —
//! a *projection of substrate truth, never truth itself* (P1). See
//! `genesis/docs/superpowers/specs/2026-07-17-did-bridge-identity-resolution-design.md`.
//!
//! Also provides a transparent proxy to elohim-storage `/api/v1/identity/*`.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{debug, warn};

use did_bridge::{assemble_did_key_document, DidResolutionError, DidResolutionResult};
use did_types::{Context, Did, DidDocument, Service, ServiceEndpoint, DID_CONTEXT_V1_1};

use crate::server::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// The doorway's own did:web document (GET /.well-known/did.json, /identity/did)
// ─────────────────────────────────────────────────────────────────────────────

/// Derive the doorway's public host — the `<host>` in `did:web:<host>` — from
/// its configuration, mirroring the precedence the doorway uses elsewhere:
/// explicit `doorway_id`, else the domain of `doorway_url`, else a local-dev
/// fallback.
fn derive_doorway_host(state: &AppState) -> String {
    if let Some(ref doorway_id) = state.args.doorway_id {
        // doorway_id is like "alpha-elohim-host" or "doorway-a.elohim.host".
        // Dots ⇒ already a domain; otherwise convert dashes to dots.
        if doorway_id.contains('.') {
            doorway_id.clone()
        } else {
            doorway_id.replace('-', ".")
        }
    } else if let Some(ref doorway_url) = state.args.doorway_url {
        extract_domain(doorway_url)
            .unwrap_or_else(|| format!("localhost:doorway:{}", state.args.node_id))
    } else {
        format!("localhost:doorway:{}", state.args.node_id)
    }
}

/// Extract domain from a URL (e.g., "https://alpha.elohim.host" -> "alpha.elohim.host")
fn extract_domain(url: &str) -> Option<String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    // Take everything before the first "/" or ":"
    let domain = without_scheme.split('/').next()?.split(':').next()?;

    if domain.is_empty() {
        None
    } else {
        Some(domain.to_string())
    }
}

/// Build the doorway's own `did:web` document.
///
/// Phase 1 (DID bridge §3.5) is a *minimal, honest* did:web document: no
/// verification methods (the doorway holds no DID key — its federation-signing
/// key is published separately as JWKS at `/.well-known/doorway-keys`, never
/// fabricated here), and a single `DIDResolution` service advertising this
/// doorway's universal-resolver endpoint. Assembled via the `did-types` DID 1.1
/// model so the emitted document is standards-legible and validates against the
/// W3C DID 1.1 conformance schema.
fn build_did_web_document(host: &str) -> Result<DidDocument, DidResolutionError> {
    let did = Did::parse(&format!("did:web:{host}"))
        .map_err(|e| DidResolutionError::Internal(e.to_string()))?;

    let mut doc = DidDocument::new(Context::Single(DID_CONTEXT_V1_1.to_string()), did.clone());
    doc.service = Some(vec![Service {
        id: did.with_fragment("resolver"),
        type_: "DIDResolution".to_string(),
        service_endpoint: ServiceEndpoint::Uri(format!("https://{host}/1.0/identifiers/")),
        extra: BTreeMap::new(),
    }]);
    // No verificationMethod set: legal and honest — an empty key set with
    // services is a valid DID 1.1 document (§3.5).
    Ok(doc)
}

/// Handle GET /.well-known/did.json
///
/// Returns the doorway's W3C `did:web` document for federation discovery.
/// Public — no authentication required.
pub fn handle_did_document(state: Arc<AppState>) -> Response<Full<Bytes>> {
    let host = derive_doorway_host(&state);
    let document = match build_did_web_document(&host) {
        Ok(doc) => doc,
        Err(e) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to build DID document: {e}"),
            );
        }
    };

    let body = match serde_json::to_string_pretty(&document) {
        Ok(json) => json,
        Err(e) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to serialize DID document: {e}"),
            );
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/did+ld+json")
        .header("Cache-Control", "public, max-age=300") // 5 minute cache
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Handle GET /identity/did (alternative endpoint)
///
/// Same document as /.well-known/did.json but at an explicit path.
pub fn handle_did_endpoint(state: Arc<AppState>) -> Response<Full<Bytes>> {
    handle_did_document(state)
}

// ─────────────────────────────────────────────────────────────────────────────
// Universal-resolver surface (GET /1.0/identifiers/{did})
// ─────────────────────────────────────────────────────────────────────────────

/// Handle `GET /1.0/identifiers/{did}` — universal-resolver-compatible DID
/// resolution (DID bridge §3.5):
///   - `did:key` resolves LOCALLY, fully offline (no I/O).
///   - `did:elohim` forwards to the primary storage's `/db/identity/did/{did}`
///     projection route.
///   - `did:web` is NOT resolved here in this leg (egress policy undecided; the
///     web-resolver feature stays off) — reported as `methodNotSupported`.
///   - any other method → `methodNotSupported`.
pub async fn handle_universal_resolver(
    state: Arc<AppState>,
    did_param: &str,
    accept: Option<String>,
) -> Response<Full<Bytes>> {
    resolve_did_request(
        state.args.storage_url.as_deref(),
        did_param,
        accept.as_deref(),
    )
    .await
}

/// Whether the client asked for the bare DID document (`application/did+json`)
/// rather than the full resolution-result envelope. The `application/did+ld+json`
/// media type is deliberately NOT matched here.
fn wants_bare_document(accept: Option<&str>) -> bool {
    accept
        .map(|a| {
            a.split(',')
                .any(|part| part.trim().starts_with("application/did+json"))
        })
        .unwrap_or(false)
}

/// Render a resolution result as an HTTP response with content negotiation: on
/// success, `Accept: application/did+json` yields the bare `didDocument`;
/// otherwise the full `DidResolutionResult` envelope as `application/json`.
fn render_resolution(
    status: StatusCode,
    result: &DidResolutionResult,
    accept: Option<&str>,
) -> Response<Full<Bytes>> {
    if status.is_success() && wants_bare_document(accept) {
        if let Some(doc) = &result.did_document {
            let body = serde_json::to_string_pretty(doc).unwrap_or_else(|_| "{}".to_string());
            return Response::builder()
                .status(status)
                .header("Content-Type", "application/did+json")
                .body(Full::new(Bytes::from(body)))
                .unwrap();
        }
    }

    let body = serde_json::to_string_pretty(result).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// Map a resolution error to its universal-resolver HTTP status. `invalidDid`
/// is 400 and `methodNotSupported` is 501 per the DID bridge contract; storage
/// failures surface as `internalError` (500) with a spec-shaped body — never a
/// bare 502.
///
/// **Exhaustive by design — no `_` arm.** The DID error-code vocabulary is
/// deliberately lossy (`IdentityHeadUnresolvable` and `IdentityHeadMalformed`
/// both project to `internalError` on the wire), so the *status* is the only
/// place the distinction survives for a caller deciding whether to retry. A
/// catch-all would silently serve a future variant as somebody else's status —
/// the compiler catching an unmapped variant here IS the guard. The mapping
/// mirrors elohim-storage's `/db/identity/did/{did}` handler exactly, so the
/// same resolution answers the same way whether the client reaches storage
/// directly or through this doorway.
fn error_status(err: &DidResolutionError) -> StatusCode {
    match err {
        DidResolutionError::InvalidDid(_) => StatusCode::BAD_REQUEST,
        DidResolutionError::NotFound(_) => StatusCode::NOT_FOUND,
        DidResolutionError::MethodNotSupported(_) => StatusCode::NOT_IMPLEMENTED,
        DidResolutionError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        // A FETCHED document described a different subject. Not the requester's
        // fault and not ours — the upstream responder answered badly, which is
        // what 502 means.
        DidResolutionError::SubjectMismatch { .. } => StatusCode::BAD_GATEWAY,
        // The head could not be determined (undelivered DHT record, timed-out
        // read, a lagging projection). Fail-closed and RETRYABLE — 503, never
        // 404: "we could not establish it" must not be served as "it does not
        // exist".
        DidResolutionError::IdentityHeadUnresolvable(_) => StatusCode::SERVICE_UNAVAILABLE,
        // A head resolved but is unusable as declared (e.g. an empty controller
        // set, which would read as implicit self-control). A substrate-side
        // defect, not a client error.
        DidResolutionError::IdentityHeadMalformed(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn resolution_error(err: DidResolutionError, accept: Option<&str>) -> Response<Full<Bytes>> {
    render_resolution(
        error_status(&err),
        &DidResolutionResult::from_error(&err),
        accept,
    )
}

/// Core dispatch — storage-URL-injectable so the did:elohim leg is unit-testable
/// against a mock storage server.
async fn resolve_did_request(
    storage_url: Option<&str>,
    did_param: &str,
    accept: Option<&str>,
) -> Response<Full<Bytes>> {
    // The DID may arrive percent-encoded; decode leniently (plain DIDs, whose
    // colons are legal path chars, pass through unchanged).
    let decoded = urlencoding::decode(did_param)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| did_param.to_string());

    let did = match Did::parse(&decoded) {
        Ok(d) => d,
        Err(e) => {
            return resolution_error(DidResolutionError::InvalidDid(e.to_string()), accept);
        }
    };

    match did.method() {
        // did:key — resolve locally, fully offline.
        "key" => match assemble_did_key_document(&did) {
            Ok(doc) => {
                render_resolution(StatusCode::OK, &DidResolutionResult::success(doc), accept)
            }
            Err(e) => resolution_error(e, accept),
        },
        // did:elohim — forward to the primary storage's projection route.
        "elohim" => resolve_elohim_via_storage(storage_url, &did, accept).await,
        // did:web and everything else are not resolved at the doorway in this leg.
        other => resolution_error(
            DidResolutionError::MethodNotSupported(other.to_string()),
            accept,
        ),
    }
}

/// Forward a `did:elohim` resolution to the primary storage's
/// `GET /db/identity/did/{did}` route (the manifest-declared identity
/// projection) and pass through its `DidResolutionResult`. A storage-unavailable
/// condition maps to a spec-shaped `internalError`, never a bare 502.
async fn resolve_elohim_via_storage(
    storage_url: Option<&str>,
    did: &Did,
    accept: Option<&str>,
) -> Response<Full<Bytes>> {
    let storage_url = match storage_url {
        Some(u) => u,
        None => {
            warn!("did:elohim resolution requested but STORAGE_URL not configured");
            return resolution_error(
                DidResolutionError::Internal("storage service not configured".to_string()),
                accept,
            );
        }
    };

    let url = format!(
        "{}/db/identity/did/{}",
        storage_url.trim_end_matches('/'),
        did.as_string()
    );
    debug!(url = %url, "Forwarding did:elohim resolution to storage");

    let client = reqwest::Client::new();
    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "did:elohim storage forward failed");
            return resolution_error(
                DidResolutionError::Internal(format!("storage unavailable: {e}")),
                accept,
            );
        }
    };

    let status =
        StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let raw = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            warn!(error = %e, "did:elohim storage response body read failed");
            return resolution_error(
                DidResolutionError::Internal(format!("storage response unreadable: {e}")),
                accept,
            );
        }
    };

    // Re-frame the storage DidResolutionResult so the doorway applies the same
    // content negotiation as the local paths. If storage returned something we
    // can't parse, pass the body through unchanged as application/json.
    match serde_json::from_slice::<DidResolutionResult>(&raw) {
        Ok(result) => render_resolution(status, &result, accept),
        Err(_) => Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(raw.to_vec())))
            .unwrap(),
    }
}

/// Small JSON error helper for the did:web document handlers.
fn json_error(status: StatusCode, msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(format!(r#"{{"error": "{msg}"}}"#))))
        .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────────
// Transparent proxy to elohim-storage /api/v1/identity/*
// ─────────────────────────────────────────────────────────────────────────────

/// Transparent proxy to elohim-storage `/api/v1/identity/*`
///
/// Forwards all HTTP methods verbatim, preserving status codes and
/// content-type from the upstream response. Mirrors the pattern used
/// by `handle_presence_request` in presence.rs.
pub async fn handle_identity_api_request(
    req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_url = match &state.args.storage_url {
        Some(url) => url.clone(),
        None => {
            warn!("Identity API proxy called but STORAGE_URL not configured");
            return identity_service_unavailable(
                "Storage service not configured. Set STORAGE_URL env var.",
            );
        }
    };
    forward_identity_to_storage(req, &storage_url, path).await
}

async fn forward_identity_to_storage(
    req: Request<Incoming>,
    storage_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{storage_endpoint}?{q}"),
        None => storage_endpoint,
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding identity request to elohim-storage");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        Method::HEAD => client.head(&full_url),
        Method::PATCH => client.patch(&full_url),
        _ => {
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
                .unwrap();
        }
    };

    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }

    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    if matches!(method, Method::POST | Method::PUT | Method::PATCH) {
        match req.collect().await {
            Ok(collected) => {
                builder = builder.body(collected.to_bytes().to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read identity request body");
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Failed to read request body: {e}"}}"#
                    ))))
                    .unwrap();
            }
        }
    }

    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => Response::builder()
                    .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                    .header("Content-Type", content_type)
                    .header("Cross-Origin-Resource-Policy", "cross-origin")
                    .body(Full::new(Bytes::from(body.to_vec())))
                    .unwrap(),
                Err(e) => {
                    warn!(error = %e, "Failed to read identity storage response body");
                    Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(format!(
                            r#"{{"error": "Failed to read storage response: {e}"}}"#
                        ))))
                        .unwrap()
                }
            }
        }
        Err(e) => {
            warn!(error = %e, path = %path, "Failed to forward identity request to storage");
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error": "Failed to connect to storage: {e}"}}"#
                ))))
                .unwrap()
        }
    }
}

fn identity_service_unavailable(msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(format!(r#"{{"error": "{msg}"}}"#))))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn read_body(resp: Response<Full<Bytes>>) -> String {
        let collected = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(collected.to_vec()).unwrap()
    }

    const DID_KEY_FIXTURE: &str = "did:key:z6MkuWzukKSaEVxe76gbFYrnW7jUUftksarjkrjUwKdEp8Lr";

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://alpha.elohim.host"),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            extract_domain("https://alpha.elohim.host/path"),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            extract_domain("https://alpha.elohim.host:8080"),
            Some("alpha.elohim.host".to_string())
        );
        assert_eq!(
            extract_domain("http://localhost:8080"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn identity_service_unavailable_returns_503() {
        let resp = identity_service_unavailable("test error");
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn identity_storage_url_built_with_query() {
        let base = "http://localhost:8090";
        let path = "/api/v1/identity";
        let query = "agentId=abc";
        let url = format!("{}{}?{}", base.trim_end_matches('/'), path, query);
        assert_eq!(url, "http://localhost:8090/api/v1/identity?agentId=abc");
    }

    #[test]
    fn identity_storage_url_built_without_query() {
        let base = "http://localhost:8090/";
        let path = "/api/v1/identity/abc-123";
        let url = format!("{}{}", base.trim_end_matches('/'), path);
        assert_eq!(url, "http://localhost:8090/api/v1/identity/abc-123");
    }

    // ── did:web document (GET /.well-known/did.json) ──────────────────────────

    #[test]
    fn did_web_document_has_resolver_service_and_no_keys() {
        let doc = build_did_web_document("alpha.elohim.host").unwrap();
        assert_eq!(doc.id.as_string(), "did:web:alpha.elohim.host");
        // Honest: the doorway holds no DID key, so no verification methods.
        assert!(doc.verification_method.is_none());
        let services = doc.service.as_ref().expect("service set present");
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].type_, "DIDResolution");
        match &services[0].service_endpoint {
            ServiceEndpoint::Uri(u) => {
                assert_eq!(u, "https://alpha.elohim.host/1.0/identifiers/");
            }
            _ => panic!("expected a single-URI service endpoint"),
        }
    }

    #[test]
    fn did_web_document_validates_against_did11_schema() {
        // The hand-derived W3C DID 1.1 conformance schema (bridges/did).
        // CARGO_MANIFEST_DIR is doorway/doorway-service; ../../bridges/did/... is
        // the repo path. The edge check stage COPYs bridges/did/schemas to
        // /bridges/did/schemas so this include_str! resolves there too
        // (CARGO_MANIFEST_DIR=/app in the container).
        const SCHEMA: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../bridges/did/schemas/did-document-1.1.schema.json"
        ));
        let schema: serde_json::Value = serde_json::from_str(SCHEMA).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let doc = build_did_web_document("alpha.elohim.host").unwrap();
        let instance = serde_json::to_value(&doc).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{} (at {})", e, e.instance_path))
            .collect();
        assert!(
            errors.is_empty(),
            "did:web document failed DID 1.1 schema:\n{}\ninstance: {}",
            errors.join("\n"),
            serde_json::to_string_pretty(&instance).unwrap()
        );
    }

    // ── Universal resolver (GET /1.0/identifiers/{did}) ───────────────────────

    #[tokio::test]
    async fn universal_resolver_did_key_resolves_offline() {
        let resp = resolve_did_request(None, DID_KEY_FIXTURE, None).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let result: DidResolutionResult = serde_json::from_str(&read_body(resp).await).unwrap();
        let doc = result.did_document.expect("did:key resolves to a document");
        assert_eq!(doc.id.as_string(), DID_KEY_FIXTURE);
        assert_eq!(
            result.did_resolution_metadata.content_type.as_deref(),
            Some("application/did+ld+json")
        );
    }

    #[tokio::test]
    async fn universal_resolver_accept_did_json_returns_bare_document() {
        let resp = resolve_did_request(None, DID_KEY_FIXTURE, Some("application/did+json")).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "application/did+json"
        );
        // Body is the bare document, not the resolution-result envelope.
        let doc: DidDocument = serde_json::from_str(&read_body(resp).await).unwrap();
        assert_eq!(doc.id.method(), "key");
    }

    #[tokio::test]
    async fn universal_resolver_invalid_did_is_400_invalid_did() {
        let resp = resolve_did_request(None, "not-a-did", None).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let result: DidResolutionResult = serde_json::from_str(&read_body(resp).await).unwrap();
        assert_eq!(
            result.did_resolution_metadata.error.as_deref(),
            Some("invalidDid")
        );
        assert!(result.did_document.is_none());
    }

    #[tokio::test]
    async fn universal_resolver_did_web_is_501_method_not_supported() {
        // did:web is deliberately not resolved at the doorway in this leg.
        let resp = resolve_did_request(None, "did:web:example.com", None).await;
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
        let result: DidResolutionResult = serde_json::from_str(&read_body(resp).await).unwrap();
        assert_eq!(
            result.did_resolution_metadata.error.as_deref(),
            Some("methodNotSupported")
        );
    }

    #[tokio::test]
    async fn universal_resolver_unknown_method_is_501() {
        let resp = resolve_did_request(None, "did:plc:z72i7hd", None).await;
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn universal_resolver_did_elohim_forwards_to_storage() {
        use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let did = "did:elohim:uhCAkabcdef";
        let storage_body = serde_json::json!({
            "didDocument": {
                "@context": "https://www.w3.org/ns/did/v1.1",
                "id": did
            },
            "didResolutionMetadata": { "contentType": "application/did+ld+json" },
            "didDocumentMetadata": {}
        });
        Mock::given(matchers::method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(storage_body))
            .mount(&server)
            .await;

        let resp = resolve_did_request(Some(&server.uri()), did, None).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let result: DidResolutionResult = serde_json::from_str(&read_body(resp).await).unwrap();
        assert_eq!(result.did_document.unwrap().id.as_string(), did);
    }

    /// The doorway's status mapping must agree with elohim-storage's, variant for
    /// variant — a client must get the same answer whether it reaches storage
    /// directly or through this doorway. The three identity-head-era variants are
    /// pinned explicitly because their DID error CODES collapse
    /// (`IdentityHeadUnresolvable` and `IdentityHeadMalformed` both project to
    /// `internalError`), so the status is the only surviving distinction — and
    /// 503-vs-500 is what tells a caller whether to retry.
    #[test]
    fn error_status_mapping_is_exhaustive_and_mirrors_storage() {
        assert_eq!(
            error_status(&DidResolutionError::InvalidDid("x".into())),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            error_status(&DidResolutionError::NotFound("x".into())),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            error_status(&DidResolutionError::MethodNotSupported("x".into())),
            StatusCode::NOT_IMPLEMENTED
        );
        assert_eq!(
            error_status(&DidResolutionError::Internal("x".into())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            error_status(&DidResolutionError::SubjectMismatch {
                requested: "did:web:a.example".into(),
                returned: "did:web:b.example".into(),
            }),
            StatusCode::BAD_GATEWAY,
            "a document naming another subject is an upstream fault → 502"
        );
        assert_eq!(
            error_status(&DidResolutionError::IdentityHeadUnresolvable(
                "lagging".into()
            )),
            StatusCode::SERVICE_UNAVAILABLE,
            "unresolvable head is retryable → 503, NEVER 404 (absence was not established)"
        );
        assert_eq!(
            error_status(&DidResolutionError::IdentityHeadMalformed(
                "empty set".into()
            )),
            StatusCode::INTERNAL_SERVER_ERROR,
            "a malformed declaration is a substrate defect, not a client error"
        );
    }

    #[tokio::test]
    async fn universal_resolver_did_elohim_storage_unavailable_is_internal_error() {
        // Point at an unroutable storage URL so the forward fails at connect;
        // the doorway must map that to a spec-shaped internalError, not a bare 502.
        let resp =
            resolve_did_request(Some("http://127.0.0.1:1"), "did:elohim:uhCAkabcdef", None).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let result: DidResolutionResult = serde_json::from_str(&read_body(resp).await).unwrap();
        assert_eq!(
            result.did_resolution_metadata.error.as_deref(),
            Some("internalError")
        );
    }
}
