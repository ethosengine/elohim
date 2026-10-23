//! CSP violation relay for the content-chrome trust edge.
//!
//! Raw content HTML on a public name's apps origin names
//! `POST /_elohim/csp-report/<contentAddress>` as its reporting endpoint: the
//! doorway embeds the address the bytes were served as in the path it emits,
//! so the relay never infers which content a report is about.
//!
//! The doorway STAMPS PROVENANCE AND JUDGES NOTHING. It rejects what is not a
//! report at all (wrong media type, not a JSON report body, oversize) before
//! anything is counted; limits each client per address at the edge; strips any
//! client-supplied `X-Elohim-*` header; and forwards the body VERBATIM, content
//! type passed through, to ONE storage peer's
//! `POST /api/v1/epr/{contentAddress}/signals` (the registry's declared-primary
//! match — single target, no fan-out) with the facts it observed:
//! `X-Elohim-Relayed-By` (this doorway), `X-Elohim-Observed-Origin` (the
//! request's `Origin`, else its `Referer`'s origin) and `X-Elohim-Document-Match`
//! (whether the report's document names the address). A mismatch is forwarded
//! with that fact, never refused: storage records `reported_via`, `relayed_by`,
//! `observed_origin` and `document_match` and weighs them. Signed reporting
//! tokens binding an endpoint to its document are recorded hardening debt.

use bytes::Bytes;
use http_body_util::Full;
use hyper::header::HeaderMap;
use hyper::{Response, StatusCode};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::warn;

use super::apps::{encode_segment, percent_decoded, storage_target};
use crate::server::AppState;

/// The relay's path prefix; the content address is the one segment after it.
pub const CSP_REPORT_PATH: &str = "/_elohim/csp-report/";

/// Storage's content-address signals route the relay forwards to.
pub const CONTENT_SIGNALS_ROUTE: &str = "/api/v1/epr/{id}/signals";

/// Provenance headers the relay stamps (storage `content_signals` reads them).
const RELAYED_BY_HEADER: &str = "x-elohim-relayed-by";
const OBSERVED_ORIGIN_HEADER: &str = "x-elohim-observed-origin";
const DOCUMENT_MATCH_HEADER: &str = "x-elohim-document-match";

/// Largest report body the relay reads.
const CSP_REPORT_MAX_BYTES: usize = 64 * 1024;

/// Reports one client may relay for one address per window (storage limits
/// again per address and per source).
const CSP_REPORTS_PER_CLIENT: u32 = 20;
const CSP_REPORT_WINDOW: std::time::Duration = std::time::Duration::from_secs(60);

/// Most (client, address) windows held at once; expired ones are pruned first,
/// and a full table refuses new keys rather than growing.
const CSP_REPORT_MAX_KEYS: usize = 4096;

/// The relay URL for content served as `address` on `origin` (absolute when
/// the origin is known, else same-origin relative).
pub(crate) fn csp_report_url(origin: Option<String>, address: &str) -> String {
    let path = format!("{CSP_REPORT_PATH}{}", encode_segment(address));
    match origin {
        Some(origin) => format!("{origin}{path}"),
        None => path,
    }
}

/// The `Reporting-Endpoints` header value naming the group the shared content
/// policy's `report-to` names (`elohim_chrome_asset::REPORT_ENDPOINT_NAME`).
pub(crate) fn reporting_endpoints(report_url: &str) -> String {
    format!(
        "{}=\"{report_url}\"",
        elohim_chrome_asset::REPORT_ENDPOINT_NAME
    )
}

/// The content address a relay path names: exactly one non-empty segment that
/// is not a traversal once percent-decoded.
fn address_of(path: &str) -> Option<String> {
    let raw = path.strip_prefix(CSP_REPORT_PATH)?;
    let decoded = percent_decoded(raw)?;
    (!decoded.is_empty()
        && !decoded.contains("..")
        && !decoded.contains(['/', '\\', '?', '#'])
        && !decoded.chars().any(char::is_control))
    .then_some(decoded)
}

/// Fixed-window counters keyed by (client, address), bounded in memory.
#[derive(Debug, Default)]
struct ClientLimiter {
    windows: std::collections::HashMap<(String, String), (std::time::Instant, u32)>,
}

impl ClientLimiter {
    fn admit(
        &mut self,
        key: (String, String),
        now: std::time::Instant,
        limit: u32,
        window: std::time::Duration,
        max_keys: usize,
    ) -> bool {
        if !self.windows.contains_key(&key) && self.windows.len() >= max_keys {
            self.windows
                .retain(|_, (start, _)| now.duration_since(*start) < window);
            if self.windows.len() >= max_keys {
                return false;
            }
        }
        let entry = self.windows.entry(key).or_insert((now, 0));
        if now.duration_since(entry.0) >= window {
            *entry = (now, 0);
        }
        if entry.1 >= limit {
            return false;
        }
        entry.1 += 1;
        true
    }
}

static CSP_REPORT_LIMITER: std::sync::LazyLock<std::sync::Mutex<ClientLimiter>> =
    std::sync::LazyLock::new(Default::default);

/// Whether the content type is one storage's signals route accepts.
fn accepted_report_type(headers: &HeaderMap) -> bool {
    headers
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|ct| ct.split(';').next())
        .map(|ct| ct.trim().to_ascii_lowercase())
        .is_some_and(|ct| {
            matches!(
                ct.as_str(),
                "application/reports+json" | "application/csp-report" | "application/json"
            )
        })
}

/// The document URLs a report body names, or `None` when the body is not a
/// report at all: a Reporting API array (`[{body:{documentURL}}]`), or a
/// legacy `{"csp-report":{"document-uri"}}`. A well-formed report that names
/// no document yields an empty list.
fn report_documents(body: &[u8]) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let text = |v: Option<&serde_json::Value>| v.and_then(|v| v.as_str()).map(str::to_string);
    match &value {
        serde_json::Value::Array(reports) => Some(
            reports
                .iter()
                .filter_map(|r| {
                    text(r.get("body").and_then(|b| b.get("documentURL"))).or(text(r.get("url")))
                })
                .collect(),
        ),
        serde_json::Value::Object(o) => {
            let report = o.get("csp-report")?;
            Some(text(report.get("document-uri")).into_iter().collect())
        }
        _ => None,
    }
}

/// The origin (`scheme://host[:port]`) of an absolute URL.
fn origin_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    (!scheme.is_empty() && !authority.is_empty())
        .then(|| format!("{}://{}", scheme.to_ascii_lowercase(), authority))
}

/// What the request itself says about where it came from: its `Origin`
/// (unless the opaque `null`), else its `Referer`'s origin. Observed, not
/// believed.
fn observed_origin(headers: &HeaderMap) -> Option<String> {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::trim)
            .filter(|v| !v.is_empty())
    };
    header("origin")
        .filter(|o| *o != "null")
        .and_then(origin_of)
        .or_else(|| header("referer").and_then(origin_of))
}

/// The identifier a document URL's path gives for its content: the segment
/// after `apps`, else `None` (the document is not content).
fn document_identifier(document_url: &str) -> Option<String> {
    let after_scheme = document_url
        .split_once("://")
        .map_or(document_url, |(_, rest)| rest);
    let path = after_scheme
        .find('/')
        .map_or("", |i| &after_scheme[i..])
        .split(['?', '#'])
        .next()
        .unwrap_or_default();
    let mut segments = path.split('/').filter(|s| !s.is_empty());
    segments.find(|s| *s == "apps")?;
    let id = segments.next()?;
    percent_decoded(id)
}

/// Whether two identifiers name the same bytes: equal strings, or both content
/// addresses whose sha2-256 digest is the same (`bafkrei…` CIDv1 and the legacy
/// `sha256-<hex>` are two spellings of one address).
fn same_address(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (
        super::blob::parse_content_address(a),
        super::blob::parse_content_address(b),
    ) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// The content addresses `name` is known by HERE, from caches only (the
/// doorway's declared head and descriptor for a slug, and a fresh cached
/// facing) — never a fetch or a Mongo scan; a miss leaves just `name`.
async fn known_addresses(state: &AppState, name: &str) -> Vec<String> {
    let mut out = vec![name.to_string()];
    if let Some(cache) = state.app_file_cache.as_ref() {
        out.extend(cache.cached_addresses(name).await);
    }
    out.extend(super::apps::cached_facing_address(name));
    out
}

/// Whether the report's documents name the relay's address. Each document's
/// `/apps/<id>/` segment matches when `<id>` is the address, or when what is
/// cached for `<id>` (or for the address) resolves to the same bytes. `None`
/// when the report named no document; `false` when nothing cached connects
/// them (the gates weigh it).
async fn document_match(state: &AppState, documents: &[String], address: &str) -> Option<bool> {
    if documents.is_empty() {
        return None;
    }
    let path_forms = known_addresses(state, address).await;
    for id in documents.iter().filter_map(|d| document_identifier(d)) {
        let doc_forms = known_addresses(state, &id).await;
        if doc_forms
            .iter()
            .any(|d| path_forms.iter().any(|p| same_address(d, p)))
        {
            return Some(true);
        }
    }
    Some(false)
}

/// `POST /_elohim/csp-report/<contentAddress>`: refuse what is not a report
/// (before anything is counted), limit the client per address, then forward
/// the body VERBATIM with the doorway's provenance stamps to ONE storage peer.
pub async fn handle_csp_report<B>(
    state: Arc<AppState>,
    req: hyper::Request<B>,
    peer: SocketAddr,
) -> Response<Full<Bytes>>
where
    B: hyper::body::Body,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    use http_body_util::BodyExt;
    let status = |code: StatusCode| {
        Response::builder()
            .status(code)
            .header("Cache-Control", "no-store")
            .body(Full::new(Bytes::new()))
            .unwrap()
    };
    let Some(address) = address_of(req.uri().path()) else {
        return status(StatusCode::NOT_FOUND);
    };
    if !accepted_report_type(req.headers()) {
        return status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    let headers = req.headers().clone();
    let body = match http_body_util::Limited::new(req.into_body(), CSP_REPORT_MAX_BYTES)
        .collect()
        .await
    {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return status(StatusCode::PAYLOAD_TOO_LARGE),
    };
    // Not a report: refused before the limiter, so it consumes nothing.
    let Some(documents) = report_documents(&body) else {
        return status(StatusCode::BAD_REQUEST);
    };
    let client = crate::server::membrane::client_ip_from_xff(
        headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()),
        &peer,
        state.trusted_proxy_hops,
    );
    let admitted = CSP_REPORT_LIMITER
        .lock()
        .map(|mut l| {
            l.admit(
                (client, address.clone()),
                std::time::Instant::now(),
                CSP_REPORTS_PER_CLIENT,
                CSP_REPORT_WINDOW,
                CSP_REPORT_MAX_KEYS,
            )
        })
        .unwrap_or(false);
    if !admitted {
        return status(StatusCode::TOO_MANY_REQUESTS);
    }
    let route = CONTENT_SIGNALS_ROUTE.replace("{id}", &encode_segment(&address));
    let Some((endpoint, _)) =
        storage_target(&state, doorway_client::HttpMethod::Post, &route).await
    else {
        // A lost algedonic signal is never routine: say so where operators look.
        warn!(%address, "csp report: no storage peer to relay to; dropped");
        return status(StatusCode::NO_CONTENT);
    };
    // Only the relay's own stamps travel: nothing the client sent under
    // X-Elohim-* is forwarded (the forward is built fresh; this is the list).
    let mut forward = reqwest::Client::new()
        .post(format!("{endpoint}{route}"))
        .header(
            RELAYED_BY_HEADER,
            crate::server::http::self_doorway_id(&state.args),
        )
        .timeout(std::time::Duration::from_secs(2))
        .body(body.to_vec());
    if let Some(ct) = headers.get(hyper::header::CONTENT_TYPE) {
        forward = forward.header(hyper::header::CONTENT_TYPE.as_str(), ct.as_bytes());
    }
    if let Some(origin) = observed_origin(&headers) {
        forward = forward.header(OBSERVED_ORIGIN_HEADER, origin);
    }
    if let Some(matched) = document_match(&state, &documents, &address).await {
        forward = forward.header(
            DOCUMENT_MATCH_HEADER,
            if matched { "true" } else { "false" },
        );
    }
    match forward.send().await {
        Ok(resp) if !resp.status().is_success() => {
            warn!(%address, %endpoint, status = %resp.status(), "csp report: storage declined the signal");
        }
        Ok(_) => {}
        Err(e) => warn!(%address, %endpoint, error = %e, "csp report relay to storage failed"),
    }
    status(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::HeaderValue;

    async fn state_for(storage: &str) -> Arc<AppState> {
        use clap::Parser;
        let mut args = crate::config::Args::parse_from([
            "doorway",
            "--listen",
            "127.0.0.1:0",
            "--doorway-id",
            "alpha",
        ]);
        args.storage_url = Some(storage.to_string());
        Arc::new(AppState::new(args))
    }

    fn peer(last: u8) -> SocketAddr {
        SocketAddr::from(([203, 0, 113, last], 40000))
    }

    #[test]
    fn the_report_endpoint_carries_the_content_address() {
        assert_eq!(
            csp_report_url(Some("https://apps.elohim.host".into()), "sha256-abc"),
            "https://apps.elohim.host/_elohim/csp-report/sha256-abc"
        );
        assert_eq!(csp_report_url(None, "a b"), "/_elohim/csp-report/a%20b");
        assert_eq!(
            reporting_endpoints("/r"),
            format!("{}=\"/r\"", elohim_chrome_asset::REPORT_ENDPOINT_NAME)
        );
        assert_eq!(
            address_of("/_elohim/csp-report/sha256-abc").as_deref(),
            Some("sha256-abc")
        );
        for bad in [
            "/_elohim/csp-report/",
            "/_elohim/csp-report/%2e%2e",
            "/_elohim/csp-report/a/b",
            "/_elohim/csp-report/a%2Fb",
            "/_elohim/csp-reportx",
        ] {
            assert_eq!(address_of(bad), None, "{bad}");
        }
    }

    #[test]
    fn reports_name_their_documents_and_origins_are_observed() {
        let legacy =
            br#"{"csp-report":{"document-uri":"https://apps.elohim.host/apps/quiz-1/index.html"}}"#;
        assert_eq!(
            report_documents(legacy).unwrap(),
            vec!["https://apps.elohim.host/apps/quiz-1/index.html"]
        );
        let modern = br#"[{"type":"csp-violation","body":{"documentURL":"https://apps.elohim.host/apps/quiz-1/x.html"}}]"#;
        assert_eq!(report_documents(modern).unwrap().len(), 1);
        assert!(report_documents(b"{").is_none());
        assert!(report_documents(br#"{"not":"a report"}"#).is_none());
        assert!(report_documents(b"42").is_none());

        assert_eq!(
            document_identifier("https://apps.elohim.host/apps/quiz-1/index.html?x=1").as_deref(),
            Some("quiz-1")
        );
        assert_eq!(document_identifier("https://elohim.host/epr/quiz-1"), None);

        let mut h = HeaderMap::new();
        h.insert(
            "referer",
            HeaderValue::from_static("https://apps.elohim.host/apps/quiz-1/i.html"),
        );
        assert_eq!(
            observed_origin(&h).as_deref(),
            Some("https://apps.elohim.host")
        );
        h.insert("origin", HeaderValue::from_static("null"));
        assert_eq!(
            observed_origin(&h).as_deref(),
            Some("https://apps.elohim.host")
        );
        h.insert("origin", HeaderValue::from_static("https://evil.example"));
        assert_eq!(observed_origin(&h).as_deref(), Some("https://evil.example"));
    }

    #[test]
    fn the_relay_limits_each_client_per_address_in_bounded_memory() {
        let start = std::time::Instant::now();
        let window = std::time::Duration::from_secs(60);
        let mut l = ClientLimiter::default();
        let key = |c: &str, a: &str| (c.to_string(), a.to_string());
        assert!(l.admit(key("a", "x"), start, 2, window, 3));
        assert!(l.admit(key("a", "x"), start, 2, window, 3));
        assert!(!l.admit(key("a", "x"), start, 2, window, 3));
        // Another client, or another address, has its own allowance.
        assert!(l.admit(key("b", "x"), start, 2, window, 3));
        assert!(l.admit(key("a", "y"), start, 2, window, 3));
        // The table is full: a new key is refused rather than growing it…
        assert!(!l.admit(key("c", "z"), start, 2, window, 3));
        // …until the windows expire and are pruned.
        assert!(l.admit(key("c", "z"), start + window, 2, window, 3));
        assert!(l.windows.len() <= 3);
    }

    #[tokio::test]
    async fn the_relay_stamps_provenance_and_forwards_verbatim_to_one_peer() {
        use wiremock::matchers::{body_bytes, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let storage = MockServer::start().await;
        let matching = br#"{"csp-report":{"document-uri":"https://apps.elohim.host/apps/sha256-abcdef123456/index.html"}}"#;
        let elsewhere = br#"[{"type":"csp-violation","body":{"documentURL":"https://apps.elohim.host/apps/someone-else/index.html"}}]"#;
        Mock::given(method("POST"))
            .and(path("/api/v1/epr/sha256-abcdef123456/signals"))
            .and(header("content-type", "application/csp-report"))
            .and(header(RELAYED_BY_HEADER, "alpha"))
            .and(header(OBSERVED_ORIGIN_HEADER, "https://apps.elohim.host"))
            .and(header(DOCUMENT_MATCH_HEADER, "true"))
            .and(body_bytes(matching.to_vec()))
            .respond_with(ResponseTemplate::new(202))
            .expect(1)
            .mount(&storage)
            .await;
        // A document naming other content is forwarded WITH that fact, never
        // refused at the edge; the client's own X-Elohim-* claims are dropped.
        Mock::given(method("POST"))
            .and(path("/api/v1/epr/sha256-abcdef123456/signals"))
            .and(header("content-type", "application/reports+json"))
            .and(header(RELAYED_BY_HEADER, "alpha"))
            .and(header(DOCUMENT_MATCH_HEADER, "false"))
            .respond_with(ResponseTemplate::new(429))
            .expect(1)
            .mount(&storage)
            .await;
        let state = state_for(&storage.uri()).await;
        let at = "/_elohim/csp-report/sha256-abcdef123456";
        let post = |ct: &str, body: &[u8], forged: bool| {
            let mut b = hyper::Request::builder()
                .method("POST")
                .uri(at)
                .header("content-type", ct)
                .header("origin", "https://apps.elohim.host");
            if forged {
                b = b
                    .header(RELAYED_BY_HEADER, "someone-else")
                    .header(DOCUMENT_MATCH_HEADER, "true");
            }
            b.body(Full::new(Bytes::from(body.to_vec()))).unwrap()
        };
        let resp = handle_csp_report(
            state.clone(),
            post("application/csp-report", matching, false),
            peer(1),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let resp = handle_csp_report(
            state.clone(),
            post("application/reports+json", elsewhere, true),
            peer(1),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let received = storage.received_requests().await.unwrap();
        assert!(received
            .iter()
            .all(|r| r.headers.get(RELAYED_BY_HEADER).unwrap() == "alpha"));
    }

    #[tokio::test]
    async fn malformed_reports_are_refused_before_the_limiter() {
        use wiremock::MockServer;
        let storage = MockServer::start().await;
        let state = state_for(&storage.uri()).await;
        let at = "/_elohim/csp-report/sha256-flood";
        let post = |ct: &str, body: &[u8]| {
            hyper::Request::builder()
                .method("POST")
                .uri(at)
                .header("content-type", ct)
                .body(Full::new(Bytes::from(body.to_vec())))
                .unwrap()
        };
        // Far more malformed bodies than any allowance: each is a 400 and none
        // spends the client's allowance.
        for _ in 0..(CSP_REPORTS_PER_CLIENT * 2) {
            let resp =
                handle_csp_report(state.clone(), post("application/json", b"{"), peer(9)).await;
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        }
        let resp = handle_csp_report(state.clone(), post("text/plain", b"{}"), peer(9)).await;
        assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        // A real report from the same client still goes through.
        let report = br#"{"csp-report":{"document-uri":"https://a/apps/sha256-flood/i.html"}}"#;
        let resp = handle_csp_report(
            state.clone(),
            post("application/csp-report", report),
            peer(9),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        // …until THAT client exhausts its own allowance for this address.
        for _ in 1..CSP_REPORTS_PER_CLIENT {
            handle_csp_report(
                state.clone(),
                post("application/csp-report", report),
                peer(9),
            )
            .await;
        }
        let resp = handle_csp_report(
            state.clone(),
            post("application/csp-report", report),
            peer(9),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        // Another client is unaffected.
        let resp = handle_csp_report(state, post("application/csp-report", report), peer(10)).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let resp = handle_csp_report(
            state_for(&storage.uri()).await,
            post("application/json", b"{}"),
            peer(9),
        )
        .await;
        // `{}` is JSON but not a report.
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    /// sha2-256 of the empty byte string, in both spellings.
    const EMPTY_CID: &str = "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku";
    const EMPTY_SHA: &str =
        "sha256-e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[tokio::test]
    async fn a_slug_document_matches_its_cid_path_through_the_cache() {
        use clap::Parser;
        let args = crate::config::Args::parse_from(["doorway", "--listen", "127.0.0.1:0"]);
        let mut state = AppState::new(args);
        let cache = Arc::new(crate::cache::AppFileCacheService::new(
            &crate::db::mongo::MongoClient::unconnected_for_test(),
            "t".to_string(),
        ));
        // The doorway already holds this slug's head (as the legacy spelling).
        cache.set_slug_head("epr-app-mva5", EMPTY_SHA).await;
        state.app_file_cache = Some(cache);
        let doc = "http://apps.elohim.localhost:8888/apps/epr-app-mva5/index.html".to_string();
        assert_eq!(document_match(&state, &[doc], EMPTY_CID).await, Some(true));
        // An unrelated slug: nothing cached connects it — false, not a guess.
        let other = "http://apps.elohim.localhost:8888/apps/someone-else/index.html".to_string();
        assert_eq!(
            document_match(&state, &[other], EMPTY_CID).await,
            Some(false)
        );
        // No document named at all: no fact to stamp.
        assert_eq!(document_match(&state, &[], EMPTY_CID).await, None);
    }

    #[tokio::test]
    async fn a_cid_document_matches_a_sha256_path() {
        let state = state_for("http://127.0.0.1:9").await;
        let doc = format!("https://apps.elohim.host/apps/{EMPTY_CID}/index.html");
        assert_eq!(document_match(&state, &[doc], EMPTY_SHA).await, Some(true));
        assert!(same_address(EMPTY_CID, EMPTY_SHA));
        assert!(!same_address(EMPTY_CID, "sha256-abc"));
        assert!(!same_address("quiz-1", "quiz-2"));
    }
}
