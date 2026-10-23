//! Peer-native chrome for `/apps`: which answer a request for an html5-app's
//! files gets on the person's own peer — the chromed shell, the raw bytes, or a
//! redirect — decided from Fetch Metadata and the Host the browser used.
//!
//! Pure (no I/O): `http.rs` reads the headers, resolves the record, and acts on
//! the [`AppsDecision`]. Every request WITHOUT Fetch Metadata (curl, the doorway's
//! own fetches, an older browser, a non-trustworthy origin the browser sends no
//! `Sec-Fetch-*` to) is [`AppsDecision::Raw`] — the existing `/apps` behaviour,
//! byte-identical.
//!
//! **Isolation without DNS.** A browser treats two loopback names as two
//! origins, so the peer pairs them the way a public name pairs with
//! `apps.<name>`:
//! - `localhost[:port]` is the CHROME origin: a page navigation there gets the
//!   shell (`render_content_shell`, medium `peer-native`), which frames the
//!   content from `127.0.0.1[:port]` — `isolation.boundary = "origin-pair"`.
//! - `127.0.0.1[:port]` is the CONTENT origin: framed loads are served raw with
//!   `frame-ancestors <chrome origin>`, `no-store` on HTML, `Vary` on Fetch
//!   Metadata and a report endpoint; a top-level navigation there redirects to
//!   the chrome origin's dedicated shell path.
//! - any other Host has no pair: the shell frames the content from its own
//!   origin and the chrome withholds `allow-same-origin` — `"opaque-sandbox"`.
//!
//! The pairing is one-directional on purpose. If either name could host the
//! chrome, content running on `127.0.0.1` could open a shell on `127.0.0.1` and
//! script that chrome (its grants, its DOM) as same-origin. One fixed chrome
//! name keeps content and chrome on different origins in every tab.
//!
//! **No loop.** The redirect target is [`SHELL_PREFIX`]`/apps/…`, which renders
//! the shell whatever Fetch Metadata says; the shell's frame is requested with
//! `Sec-Fetch-Dest: iframe`, which is never a navigation.

use hyper::HeaderMap;

/// The dedicated shell path: `/_elohim/shell/apps/<id>/<file>` always renders
/// the shell (regardless of Fetch Metadata) on the chrome origin.
pub const SHELL_PREFIX: &str = "/_elohim/shell";

/// The loopback name that hosts the chrome.
pub const CHROME_HOST: &str = "localhost";
/// The loopback name that hosts content.
pub const CONTENT_HOST: &str = "127.0.0.1";

/// The Reporting API endpoint name storage's content origin declares — the
/// chrome's web-edge policy owns the name (`elohim_chrome_asset::policy`).
pub use elohim_chrome_asset::REPORT_ENDPOINT_NAME;

/// `Vary` on every Fetch-Metadata-decided answer.
pub const FETCH_METADATA_VARY: &str = "Sec-Fetch-Dest, Sec-Fetch-Mode, Sec-Fetch-Site";

/// What the browser said the request is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchKind {
    /// No `Sec-Fetch-Dest` — not a browser that speaks Fetch Metadata here.
    Unmarked,
    /// `Sec-Fetch-Dest: document` + `Sec-Fetch-Mode: navigate`.
    Navigation,
    /// `Sec-Fetch-Dest: iframe | frame`.
    Frame,
    /// Anything else (scripts, images, fetches, workers…).
    Subresource,
}

/// Classify a request by its Fetch Metadata.
pub fn fetch_kind(headers: &HeaderMap) -> FetchKind {
    let get = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim().to_ascii_lowercase())
    };
    match get("sec-fetch-dest").as_deref() {
        None | Some("") => FetchKind::Unmarked,
        Some("document") if get("sec-fetch-mode").as_deref() == Some("navigate") => {
            FetchKind::Navigation
        }
        Some("iframe" | "frame") => FetchKind::Frame,
        Some(_) => FetchKind::Subresource,
    }
}

/// Which side of the loopback pair the Host names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRole {
    /// `localhost[:port]`.
    Chrome { port: Option<u16> },
    /// `127.0.0.1[:port]`.
    Content { port: Option<u16> },
    /// Any other Host (or none): no pair.
    Unpaired,
}

/// Read the Host header's role.
pub fn host_role(host: Option<&str>) -> HostRole {
    let Some(host) = host.map(str::trim).filter(|h| !h.is_empty()) else {
        return HostRole::Unpaired;
    };
    let (name, port) = match host.rsplit_once(':') {
        Some((name, port)) if !name.contains(':') => match port.parse::<u16>() {
            Ok(p) => (name, Some(p)),
            Err(_) => return HostRole::Unpaired,
        },
        _ => (host, None),
    };
    match name.to_ascii_lowercase().as_str() {
        CHROME_HOST => HostRole::Chrome { port },
        CONTENT_HOST => HostRole::Content { port },
        _ => HostRole::Unpaired,
    }
}

fn origin(host: &str, port: Option<u16>) -> String {
    match port {
        Some(p) => format!("http://{host}:{p}"),
        None => format!("http://{host}"),
    }
}

/// The chrome origin of a loopback pair.
pub fn chrome_origin(port: Option<u16>) -> String {
    origin(CHROME_HOST, port)
}

/// The content origin of a loopback pair.
pub fn content_origin(port: Option<u16>) -> String {
    origin(CONTENT_HOST, port)
}

/// The signal-intake path for a record (`POST /api/v1/epr/{id}/signals`), with
/// the id percent-encoded as one path segment.
pub fn signals_path(record_id: &str) -> String {
    let mut seg = String::with_capacity(record_id.len());
    for b in record_id.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b':') {
            seg.push(b as char);
        } else {
            seg.push_str(&format!("%{b:02X}"));
        }
    }
    format!("/api/v1/epr/{seg}/signals")
}

/// How the shell isolates the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellIsolation {
    /// Content framed from the paired content origin.
    OriginPair { content_origin: String },
    /// Content framed from the shell's own origin; the chrome withholds
    /// `allow-same-origin`.
    OpaqueSandbox,
}

/// The answer `/apps` gives one request for an html5-app record's files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppsDecision {
    /// The existing raw serve, untouched.
    Raw,
    /// The raw serve plus the content-origin headers.
    RawContent {
        /// The `frame-ancestors` source list.
        frame_ancestors: String,
        /// Absolute Reporting API endpoint, when the origin is one the browser
        /// will report to (loopback).
        report_endpoint: Option<String>,
    },
    /// The chromed shell; the frame loads `frame_src`.
    Shell {
        isolation: ShellIsolation,
        frame_src: String,
    },
    /// A redirect (302) to `location`.
    Redirect { location: String },
}

/// Decide. `apps_path_and_query` is `/apps/<id>/<file>[?query]`; `via_shell_path`
/// is true when the request came in on [`SHELL_PREFIX`]; `record_id` names the
/// html5-app record the path resolved to.
pub fn decide(
    kind: FetchKind,
    role: &HostRole,
    via_shell_path: bool,
    apps_path_and_query: &str,
    record_id: &str,
) -> AppsDecision {
    let wants_shell = via_shell_path || kind == FetchKind::Navigation;
    match role {
        HostRole::Chrome { port } => {
            if wants_shell {
                let content = content_origin(*port);
                AppsDecision::Shell {
                    frame_src: format!("{content}{apps_path_and_query}"),
                    isolation: ShellIsolation::OriginPair {
                        content_origin: content,
                    },
                }
            } else if kind == FetchKind::Frame {
                // Content is never framed on the chrome origin.
                AppsDecision::Redirect {
                    location: format!("{}{apps_path_and_query}", content_origin(*port)),
                }
            } else {
                AppsDecision::Raw
            }
        }
        HostRole::Content { port } => {
            if wants_shell {
                AppsDecision::Redirect {
                    location: format!(
                        "{}{SHELL_PREFIX}{apps_path_and_query}",
                        chrome_origin(*port)
                    ),
                }
            } else if kind == FetchKind::Unmarked {
                AppsDecision::Raw
            } else {
                AppsDecision::RawContent {
                    frame_ancestors: chrome_origin(*port),
                    report_endpoint: Some(format!(
                        "{}{}",
                        content_origin(*port),
                        signals_path(record_id)
                    )),
                }
            }
        }
        HostRole::Unpaired => {
            if wants_shell {
                AppsDecision::Shell {
                    frame_src: apps_path_and_query.to_string(),
                    isolation: ShellIsolation::OpaqueSandbox,
                }
            } else if kind == FetchKind::Frame {
                AppsDecision::RawContent {
                    frame_ancestors: "'self'".to_string(),
                    report_endpoint: None,
                }
            } else {
                AppsDecision::Raw
            }
        }
    }
}

/// The `Content-Security-Policy` value for raw content on a content origin.
pub fn content_csp(frame_ancestors: &str, record_id: &str, has_report_to: bool) -> String {
    let mut csp = format!(
        "frame-ancestors {frame_ancestors}; report-uri {}",
        signals_path(record_id)
    );
    if has_report_to {
        csp.push_str(&format!("; report-to {REPORT_ENDPOINT_NAME}"));
    }
    csp
}

/// Apply the content-origin headers to a raw response's header map: the
/// enforced `frame-ancestors` (Content-Security-Policy), the chrome's web-edge
/// content policy for `mode` over the record's declared `sources`
/// (`elohim_chrome_asset::content_policy` — Report-Only for trustful content,
/// so it is observed, not blocked), the report endpoint, `no-store` on HTML and
/// `Vary` on Fetch Metadata.
pub fn apply_content_headers(
    headers: &mut HeaderMap,
    frame_ancestors: &str,
    report_endpoint: Option<&str>,
    record_id: &str,
    mode: elohim_chrome_asset::TrustMode,
    sources: &[String],
) {
    use hyper::header::{HeaderName, HeaderValue, CACHE_CONTROL, CONTENT_TYPE, VARY};
    // No `report-to` on either policy: with it present, browsers ignore `report-uri` and
    // queue the report in the Reporting API for about a minute. Signals go by `report-uri`,
    // which is delivered promptly; `Reporting-Endpoints` stays declared and harmless.
    let _ = report_endpoint;
    let csp = content_csp(frame_ancestors, record_id, false);
    if let Ok(v) = HeaderValue::from_str(&csp) {
        headers.insert("content-security-policy", v);
    }
    let policy = elohim_chrome_asset::content_policy(mode, sources, &signals_path(record_id), None);
    if let (Ok(name), Ok(value)) = (
        HeaderName::from_bytes(policy.header_name.as_bytes()),
        HeaderValue::from_str(&policy.value),
    ) {
        if name == "content-security-policy" {
            // Trustless: the enforced policy joins frame-ancestors as a second
            // CSP header (browsers enforce every policy they receive).
            headers.append(name, value);
        } else {
            headers.insert(name, value);
        }
    }
    if let Some(endpoint) = report_endpoint {
        if let Ok(v) = HeaderValue::from_str(&format!("{REPORT_ENDPOINT_NAME}=\"{endpoint}\"")) {
            headers.insert("reporting-endpoints", v);
        }
    }
    let is_html = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|t| t.to_ascii_lowercase().starts_with("text/html"));
    if is_html {
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    headers.insert(VARY, HeaderValue::from_static(FETCH_METADATA_VARY));
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::HeaderValue;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(*k, HeaderValue::from_static(v));
        }
        h
    }

    const NAV: &[(&str, &str)] = &[
        ("sec-fetch-dest", "document"),
        ("sec-fetch-mode", "navigate"),
    ];
    const FRAME: &[(&str, &str)] = &[("sec-fetch-dest", "iframe"), ("sec-fetch-mode", "navigate")];
    const SCRIPT: &[(&str, &str)] = &[("sec-fetch-dest", "script"), ("sec-fetch-mode", "no-cors")];
    const P: &str = "/apps/evolution-of-trust/index.html?level=2";

    #[test]
    fn fetch_kind_reads_dest_and_mode() {
        assert_eq!(fetch_kind(&HeaderMap::new()), FetchKind::Unmarked);
        assert_eq!(fetch_kind(&headers(NAV)), FetchKind::Navigation);
        assert_eq!(fetch_kind(&headers(FRAME)), FetchKind::Frame);
        assert_eq!(fetch_kind(&headers(SCRIPT)), FetchKind::Subresource);
        assert_eq!(
            fetch_kind(&headers(&[
                ("sec-fetch-dest", "document"),
                ("sec-fetch-mode", "cors")
            ])),
            FetchKind::Subresource,
            "a document fetch that is not a navigation is not a page open"
        );
    }

    #[test]
    fn host_roles_pair_the_two_loopback_names() {
        assert_eq!(
            host_role(Some("localhost:8090")),
            HostRole::Chrome { port: Some(8090) }
        );
        assert_eq!(
            host_role(Some("LOCALHOST")),
            HostRole::Chrome { port: None }
        );
        assert_eq!(
            host_role(Some("127.0.0.1:8090")),
            HostRole::Content { port: Some(8090) }
        );
        assert_eq!(host_role(Some("192.168.1.4:8090")), HostRole::Unpaired);
        assert_eq!(host_role(Some("[::1]:8090")), HostRole::Unpaired);
        assert_eq!(host_role(None), HostRole::Unpaired);
        assert_eq!(host_role(Some("localhost:notaport")), HostRole::Unpaired);
    }

    #[test]
    fn unmarked_requests_keep_todays_raw_serve_everywhere() {
        for role in [
            HostRole::Chrome { port: Some(8090) },
            HostRole::Content { port: Some(8090) },
            HostRole::Unpaired,
        ] {
            assert_eq!(
                decide(FetchKind::Unmarked, &role, false, P, "r"),
                AppsDecision::Raw
            );
        }
    }

    #[test]
    fn navigation_on_the_chrome_origin_frames_from_the_content_origin() {
        let d = decide(
            FetchKind::Navigation,
            &HostRole::Chrome { port: Some(8090) },
            false,
            P,
            "r",
        );
        assert_eq!(
            d,
            AppsDecision::Shell {
                isolation: ShellIsolation::OriginPair {
                    content_origin: "http://127.0.0.1:8090".into()
                },
                frame_src: format!("http://127.0.0.1:8090{P}"),
            },
            "query string preserved into the frame"
        );
        // A frame on the chrome origin is sent to the content origin.
        assert_eq!(
            decide(
                FetchKind::Frame,
                &HostRole::Chrome { port: Some(8090) },
                false,
                P,
                "r"
            ),
            AppsDecision::Redirect {
                location: format!("http://127.0.0.1:8090{P}")
            }
        );
        assert_eq!(
            decide(
                FetchKind::Subresource,
                &HostRole::Chrome { port: Some(8090) },
                false,
                P,
                "r"
            ),
            AppsDecision::Raw
        );
    }

    #[test]
    fn content_origin_redirects_navigations_to_a_shell_path_that_always_renders() {
        let content = HostRole::Content { port: Some(8090) };
        let d = decide(FetchKind::Navigation, &content, false, P, "r");
        let AppsDecision::Redirect { location } = d else {
            panic!("navigation on the content origin must redirect: {d:?}")
        };
        assert_eq!(location, format!("http://localhost:8090{SHELL_PREFIX}{P}"));
        // Following it: the dedicated path on the chrome origin renders the
        // shell even for a request that carries no Fetch Metadata — no loop.
        assert!(matches!(
            decide(
                FetchKind::Unmarked,
                &HostRole::Chrome { port: Some(8090) },
                true,
                P,
                "r"
            ),
            AppsDecision::Shell { .. }
        ));
        // The dedicated path on the content origin still goes to the chrome.
        assert!(matches!(
            decide(FetchKind::Unmarked, &content, true, P, "r"),
            AppsDecision::Redirect { .. }
        ));
    }

    #[test]
    fn framed_and_subresource_loads_on_the_content_origin_carry_content_headers() {
        for kind in [FetchKind::Frame, FetchKind::Subresource] {
            assert_eq!(
                decide(
                    kind,
                    &HostRole::Content { port: Some(8090) },
                    false,
                    P,
                    "evolution-of-trust"
                ),
                AppsDecision::RawContent {
                    frame_ancestors: "http://localhost:8090".into(),
                    report_endpoint: Some(
                        "http://127.0.0.1:8090/api/v1/epr/evolution-of-trust/signals".into()
                    ),
                }
            );
        }
    }

    #[test]
    fn unpaired_hosts_fall_back_to_an_opaque_sandbox_on_one_origin() {
        assert_eq!(
            decide(FetchKind::Navigation, &HostRole::Unpaired, false, P, "r"),
            AppsDecision::Shell {
                isolation: ShellIsolation::OpaqueSandbox,
                frame_src: P.into(),
            }
        );
        assert_eq!(
            decide(FetchKind::Frame, &HostRole::Unpaired, false, P, "r"),
            AppsDecision::RawContent {
                frame_ancestors: "'self'".into(),
                report_endpoint: None,
            }
        );
        assert_eq!(
            decide(FetchKind::Subresource, &HostRole::Unpaired, false, P, "r"),
            AppsDecision::Raw
        );
    }

    #[test]
    fn content_headers_pin_frame_ancestors_reporting_and_html_no_store() {
        let mut h = headers(&[
            ("content-type", "text/html"),
            ("cache-control", "public, max-age=3600"),
        ]);
        apply_content_headers(
            &mut h,
            "http://localhost:8090",
            Some("http://127.0.0.1:8090/api/v1/epr/x/signals"),
            "x",
            elohim_chrome_asset::TrustMode::Trustful,
            &["https://fonts.example.org".to_string()],
        );
        assert_eq!(
            h.get("content-security-policy").unwrap(),
            "frame-ancestors http://localhost:8090; report-uri /api/v1/epr/x/signals"
        );
        assert_eq!(
            h.get("reporting-endpoints").unwrap(),
            "elohim-signals=\"http://127.0.0.1:8090/api/v1/epr/x/signals\""
        );
        assert_eq!(
            h.get("content-security-policy-report-only").unwrap(),
            "default-src 'self' blob: data: 'unsafe-inline' 'unsafe-eval' https://fonts.example.org; \
             report-uri /api/v1/epr/x/signals",
            "trustful content is observed against its declared sources, not blocked"
        );
        assert_eq!(h.get("cache-control").unwrap(), "no-store");
        assert_eq!(h.get("vary").unwrap(), FETCH_METADATA_VARY);

        // A script keeps its cache policy; only raw HTML is no-store.
        let mut js = headers(&[
            ("content-type", "application/javascript"),
            ("cache-control", "public, max-age=3600"),
        ]);
        apply_content_headers(
            &mut js,
            "'self'",
            None,
            "x",
            elohim_chrome_asset::TrustMode::Trustful,
            &[],
        );
        assert_eq!(js.get("cache-control").unwrap(), "public, max-age=3600");
        assert!(js.get("reporting-endpoints").is_none());
        assert_eq!(
            js.get("content-security-policy").unwrap(),
            "frame-ancestors 'self'; report-uri /api/v1/epr/x/signals"
        );
        assert_eq!(
            js.get("content-security-policy-report-only").unwrap(),
            "default-src 'self' blob: data: 'unsafe-inline' 'unsafe-eval'; report-uri /api/v1/epr/x/signals"
        );

        // Trustless: the strict profile is ENFORCED, beside frame-ancestors.
        let mut strict = headers(&[("content-type", "text/html")]);
        apply_content_headers(
            &mut strict,
            "'self'",
            None,
            "x",
            elohim_chrome_asset::TrustMode::Trustless,
            &[],
        );
        let enforced: Vec<&str> = strict
            .get_all("content-security-policy")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        assert_eq!(enforced.len(), 2);
        assert!(enforced[1].starts_with("default-src 'self' blob: data:; object-src 'none'"));
        assert!(strict.get("content-security-policy-report-only").is_none());
    }

    #[test]
    fn signals_path_encodes_the_record_id_as_one_segment() {
        assert_eq!(
            signals_path("evolution-of-trust"),
            "/api/v1/epr/evolution-of-trust/signals"
        );
        assert_eq!(signals_path("a b/c"), "/api/v1/epr/a%20b%2Fc/signals");
    }
}
