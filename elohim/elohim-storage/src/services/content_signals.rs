//! Content-address signal intake — `POST /api/v1/epr/{id}/signals`.
//!
//! The one storage-declared route that records what content was seen doing that
//! its record did not declare. A browser raises a Content-Security-Policy
//! violation report (trustful content is served with a Report-Only policy, so
//! it is observed, not blocked) against a content origin; the report reaches
//! this route either directly (storage's own content origin names it in
//! `report-uri` / `Reporting-Endpoints`) or through a doorway, which relays the
//! browser body verbatim to this ONE route and never fans out.
//!
//! Each accepted report becomes one `elohim:undeclared-behaviour` observation
//! (payload: `elohim/sdk/schemas/v1/feedback-signals/undeclared-behaviour.schema.json`):
//! subject = the content address, observer = this peer's own agent key, appended
//! to this peer's observation log through the observation manager. The log is
//! the source of truth and the `observations` row its projection; the
//! observation plane — not this route — carries it to other peers. Read back
//! through `GET /api/v1/observations/by-subject`.
//!
//! What this route is NOT: a verdict. The report is unauthenticated browser
//! evidence; it carries no standing impact and debits no one. So nothing about
//! the report's provenance refuses it: the facts the path carried — who relayed
//! it ([`SignalEnvelope`]: `reported_via`, `relayed_by`, `observed_origin`) and
//! whether its document named this content (`document_match`) — are RECORDED
//! beside it for the gates to weigh. It does refuse a subject the peer cannot
//! name: the path must resolve to a servable record, so a stranger cannot mint
//! observations about arbitrary strings. And it refuses a record the caller may
//! not read at its reach (the chrome route's own reach gate — so an anonymous
//! browser report reaches commons/public content only), or whose reach is
//! narrower than the observation's ([`UNDECLARED_BEHAVIOUR_REACH`]): neither
//! learns the record's content address, and the observation plane never carries
//! an address further than its record's reach.
//!
//! Bodies accepted (the doorway forwards them unchanged):
//! - `application/reports+json` — Reporting API, an array of reports; only
//!   `type: "csp-violation"` entries are recorded;
//! - `application/csp-report` — the legacy `{"csp-report": {…}}` body;
//! - `application/json` — a schema-shaped intent (`directive`, `blocked_uri?`,
//!   `disposition?`, `reported_by?`, `document_uri?`) from the chrome element or
//!   an inspector.
//!
//! Rate limit: [`MAX_SIGNALS_PER_WINDOW`] per content address per
//! [`SIGNAL_WINDOW`], of which one source (a relaying doorway, or a direct
//! client address) may take at most [`MAX_SIGNALS_PER_SOURCE`]. A report past
//! either limit is not recorded (429). A body that does not parse is refused
//! before the limiter, so it never consumes the allowance.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use diesel::prelude::*;
use serde_json::Value;

use crate::db::diesel_schema::observations;
use crate::error::StorageError;
use crate::observation::manager::ObservationManagerBackend;
use crate::observation::wire::Observation;

/// The signal kind this route records (`signal_kind` in the payload; declared in
/// the elohim domain manifest's `signalKinds`, with no debit weight).
pub const UNDECLARED_BEHAVIOUR: &str = "undeclared-behaviour";

/// The observation kind each report is recorded as — declared in
/// `elohim/sdk/domains/elohim/manifest.json` `observation_kinds`
/// (`attestation-feeding`, reach `community`).
pub const UNDECLARED_BEHAVIOUR_OBSERVATION: &str = "elohim:undeclared-behaviour";

/// The reach the [`UNDECLARED_BEHAVIOUR_OBSERVATION`] kind is declared at in the
/// manifest — how far the observation plane carries the subject's content
/// address. The intake refuses a record whose own reach is narrower
/// (`api::epr::resolve_signal_subject`); a test holds this equal to the
/// manifest's declaration.
pub const UNDECLARED_BEHAVIOUR_REACH: &str = "community";

/// Observation kinds this intake alone records. Their payload carries facts the
/// intake stamps from the path a report arrived by (`reported_via`,
/// `relayed_by`, `observed_origin`, `document_match`), so the generic self-row
/// route `POST /api/v1/observations` refuses them: a caller there would assert
/// those facts instead of the path carrying them. The kind is declared in the
/// manifest (so the observation plane carries it and payloads validate) whether
/// or not the content chrome is on; this list is what keeps that declaration
/// from widening the generic route.
pub const INTAKE_ONLY_OBSERVATION_KINDS: &[&str] = &[UNDECLARED_BEHAVIOUR_OBSERVATION];

/// Whether `kind` is recorded only by this intake ([`INTAKE_ONLY_OBSERVATION_KINDS`]).
pub fn is_intake_only_observation_kind(kind: &str) -> bool {
    INTAKE_ONLY_OBSERVATION_KINDS.contains(&kind)
}

/// The largest body the route reads (64 KiB — a Reporting API batch).
pub const MAX_SIGNAL_BODY_BYTES: usize = 64 * 1024;

/// Reports recorded per content address per [`SIGNAL_WINDOW`].
pub const MAX_SIGNALS_PER_WINDOW: u32 = 30;

/// Of those, the most one source may take — so one doorway or one client
/// cannot spend a content address's whole allowance.
pub const MAX_SIGNALS_PER_SOURCE: u32 = 10;

/// The rate-limit window.
pub const SIGNAL_WINDOW: Duration = Duration::from_secs(60);

/// Reports read from one request body, at most.
pub const MAX_REPORTS_PER_BODY: usize = 20;

/// Header a relaying doorway names itself in (its federation id).
pub const RELAYED_BY_HEADER: &str = "x-elohim-relayed-by";
/// Header carrying the `Origin` (or `Referer` origin) the browser sent the doorway.
pub const OBSERVED_ORIGIN_HEADER: &str = "x-elohim-observed-origin";
/// Header carrying the doorway's own check: did the report's document name this
/// content address (`true` | `false`)?
pub const DOCUMENT_MATCH_HEADER: &str = "x-elohim-document-match";

/// Distinct keys the limiter tracks before it prunes (bounded memory).
const MAX_TRACKED_KEYS: usize = 4096;

const MAX_DIRECTIVE_LEN: usize = 128;
const MAX_BLOCKED_URI_LEN: usize = 512;
const MAX_ENVELOPE_FIELD_LEN: usize = 256;

/// Who observed the behaviour (`reported_by`).
const REPORTERS: [&str; 3] = ["browser", "chrome", "inspector"];

/// One report, reduced to the schema's fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedSignal {
    pub directive: String,
    pub blocked_uri: Option<String>,
    pub disposition: Option<String>,
    pub reported_by: String,
    /// The document the report was raised in (`documentURL` / `document-uri`) —
    /// read only to check it against the subject; never recorded.
    pub document_url: Option<String>,
}

/// What the path a report arrived by says about it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignalEnvelope {
    /// The relaying doorway's federation id; `None` for a direct report.
    pub relayed_by: Option<String>,
    /// The origin the browser sent (`Origin`, else the `Referer`'s origin).
    pub observed_origin: Option<String>,
    /// A relaying doorway's own document check (direct reports are checked here).
    pub relayed_document_match: Option<bool>,
    /// The direct client's address (the rate-limit share key for direct reports).
    pub client: Option<String>,
}

impl SignalEnvelope {
    /// `"doorway"` when a doorway relayed the report, else `"direct"`.
    pub fn reported_via(&self) -> &'static str {
        if self.relayed_by.is_some() {
            "doorway"
        } else {
            "direct"
        }
    }

    /// The rate-limit share key: the relaying doorway, or the direct client.
    pub fn source_key(&self) -> String {
        match (&self.relayed_by, &self.client) {
            (Some(doorway), _) => format!("doorway:{doorway}"),
            (None, Some(client)) => format!("direct:{client}"),
            (None, None) => "direct:unknown".to_string(),
        }
    }
}

fn header_text(headers: &hyper::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.chars().take(MAX_ENVELOPE_FIELD_LEN).collect())
}

/// The origin (`scheme://host[:port]`) of a URL, or `None`.
fn origin_of(url: &str) -> Option<String> {
    let u = url::Url::parse(url).ok()?;
    let origin = u.origin();
    origin.is_tuple().then(|| origin.ascii_serialization())
}

/// Read the envelope from the request: the doorway's relay headers when present,
/// else the direct request's own `Origin` / `Referer`.
pub fn envelope_from_headers(
    headers: &hyper::HeaderMap,
    client: Option<std::net::IpAddr>,
) -> SignalEnvelope {
    let relayed_by = header_text(headers, RELAYED_BY_HEADER);
    let observed_origin = if relayed_by.is_some() {
        header_text(headers, OBSERVED_ORIGIN_HEADER)
    } else {
        header_text(headers, "origin")
            .filter(|o| o != "null")
            .or_else(|| header_text(headers, "referer").and_then(|r| origin_of(&r)))
    };
    let relayed_document_match = if relayed_by.is_some() {
        match header_text(headers, DOCUMENT_MATCH_HEADER).as_deref() {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => None,
        }
    } else {
        None
    };
    SignalEnvelope {
        relayed_by,
        observed_origin,
        relayed_document_match,
        client: client.map(|ip| ip.to_string()),
    }
}

/// The names a document URL may use for the subject: its content address in
/// both spellings, its record id, and its bundle slug.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignalSubject {
    /// CIDv1 — the observation's subject.
    pub content_address: String,
    pub record_id: Option<String>,
    /// Every spelling a document path may name the subject by.
    pub names: Vec<String>,
}

/// Did the report's document name this subject? `None` when the report carried
/// no parseable document URL. The `/apps/<identifier>/…` segment is compared
/// when present; otherwise any path segment.
pub fn document_names_subject(document_url: Option<&str>, subject: &SignalSubject) -> Option<bool> {
    let url = url::Url::parse(document_url?).ok()?;
    let segments: Vec<&str> = url
        .path_segments()
        .map(|s| s.filter(|seg| !seg.is_empty()).collect())
        .unwrap_or_default();
    let named =
        |seg: &str| seg == subject.content_address || subject.names.iter().any(|n| n == seg);
    let matched = match segments.iter().position(|s| *s == "apps") {
        Some(i) => segments.get(i + 1).is_some_and(|s| named(s)),
        None => segments.iter().any(|s| named(s)),
    };
    Some(matched)
}

/// Fixed-window counters: per content address, and per (address, source).
#[derive(Debug, Default)]
pub struct SignalRateLimiter {
    per_address: HashMap<String, (Instant, u32)>,
    per_source: HashMap<(String, String), (Instant, u32)>,
}

fn window_count<K: std::hash::Hash + Eq + Clone>(
    map: &mut HashMap<K, (Instant, u32)>,
    key: &K,
    now: Instant,
) -> Option<u32> {
    if !map.contains_key(key) && map.len() >= MAX_TRACKED_KEYS {
        map.retain(|_, (start, _)| now.duration_since(*start) < SIGNAL_WINDOW);
        if map.len() >= MAX_TRACKED_KEYS {
            return None;
        }
    }
    let entry = map.entry(key.clone()).or_insert((now, 0));
    if now.duration_since(entry.0) >= SIGNAL_WINDOW {
        *entry = (now, 0);
    }
    Some(entry.1)
}

impl SignalRateLimiter {
    /// Admit one report for `address` from `source` at `now`, or refuse it.
    /// Both the address's allowance and the source's share must have room.
    pub fn allow(&mut self, address: &str, source: &str, now: Instant) -> bool {
        let address_key = address.to_string();
        let source_key = (address.to_string(), source.to_string());
        let (Some(by_address), Some(by_source)) = (
            window_count(&mut self.per_address, &address_key, now),
            window_count(&mut self.per_source, &source_key, now),
        ) else {
            return false;
        };
        if by_address >= MAX_SIGNALS_PER_WINDOW || by_source >= MAX_SIGNALS_PER_SOURCE {
            return false;
        }
        if let Some(e) = self.per_address.get_mut(&address_key) {
            e.1 += 1;
        }
        if let Some(e) = self.per_source.get_mut(&source_key) {
            e.1 += 1;
        }
        true
    }
}

/// The process-wide limiter.
pub fn rate_limiter() -> &'static Mutex<SignalRateLimiter> {
    static LIMITER: OnceLock<Mutex<SignalRateLimiter>> = OnceLock::new();
    LIMITER.get_or_init(|| Mutex::new(SignalRateLimiter::default()))
}

/// The media type of a request body, lowercased, parameters dropped.
fn media_type(content_type: Option<&str>) -> String {
    content_type
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

/// Parse a request body into reports, by its media type. An unsupported media
/// type or unparseable body is `InvalidInput` (400).
pub fn parse_signal_body(
    content_type: Option<&str>,
    body: &[u8],
) -> Result<Vec<ParsedSignal>, StorageError> {
    if body.len() > MAX_SIGNAL_BODY_BYTES {
        return Err(StorageError::InvalidInput(format!(
            "signal body is {} bytes; the limit is {MAX_SIGNAL_BODY_BYTES}",
            body.len()
        )));
    }
    let value: Value = serde_json::from_slice(body)
        .map_err(|e| StorageError::InvalidInput(format!("signal body is not JSON: {e}")))?;
    let media = media_type(content_type);
    let signals: Vec<ParsedSignal> = match media.as_str() {
        "application/reports+json" => {
            let reports = value.as_array().ok_or_else(|| {
                StorageError::InvalidInput("application/reports+json body must be an array".into())
            })?;
            reports
                .iter()
                .take(MAX_REPORTS_PER_BODY)
                .filter(|r| r.get("type").and_then(Value::as_str) == Some("csp-violation"))
                .filter_map(|r| {
                    let b = r.get("body")?;
                    signal_from(
                        b.get("effectiveDirective")
                            .or_else(|| b.get("violatedDirective")),
                        b.get("blockedURL").or_else(|| b.get("blockedURI")),
                        b.get("disposition"),
                        b.get("documentURL")
                            .or_else(|| b.get("documentURI"))
                            .or_else(|| r.get("url")),
                        "browser",
                    )
                })
                .collect()
        }
        "application/csp-report" => {
            let b = value.get("csp-report").ok_or_else(|| {
                StorageError::InvalidInput("application/csp-report body lacks `csp-report`".into())
            })?;
            signal_from(
                b.get("effective-directive")
                    .or_else(|| b.get("violated-directive")),
                b.get("blocked-uri"),
                b.get("disposition"),
                b.get("document-uri"),
                "browser",
            )
            .into_iter()
            .collect()
        }
        "application/json" | "" => {
            let reporter = value
                .get("reported_by")
                .and_then(Value::as_str)
                .unwrap_or("chrome");
            if !REPORTERS.contains(&reporter) {
                return Err(StorageError::InvalidInput(format!(
                    "reported_by must be one of {REPORTERS:?}"
                )));
            }
            signal_from(
                value.get("directive"),
                value.get("blocked_uri"),
                value.get("disposition"),
                value.get("document_uri"),
                reporter,
            )
            .into_iter()
            .collect()
        }
        other => {
            return Err(StorageError::InvalidInput(format!(
                "unsupported signal media type `{other}` (accepted: application/reports+json, \
                 application/csp-report, application/json)"
            )))
        }
    };
    Ok(signals)
}

fn signal_from(
    directive: Option<&Value>,
    blocked: Option<&Value>,
    disposition: Option<&Value>,
    document: Option<&Value>,
    reported_by: &str,
) -> Option<ParsedSignal> {
    // A legacy `violated-directive` carries the policy too (`script-src 'self'`):
    // keep only the directive name.
    let directive: String = directive
        .and_then(Value::as_str)?
        .split_whitespace()
        .next()?
        .chars()
        .take(MAX_DIRECTIVE_LEN)
        .collect();
    if directive.is_empty() {
        return None;
    }
    Some(ParsedSignal {
        directive,
        blocked_uri: blocked.and_then(Value::as_str).and_then(reduce_blocked_uri),
        disposition: disposition
            .and_then(Value::as_str)
            .filter(|d| matches!(*d, "enforce" | "report"))
            .map(str::to_string),
        reported_by: reported_by.to_string(),
        document_url: document
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(str::to_string),
    })
}

/// Reduce what the content reached for to scheme, host and path — query and
/// fragment dropped so a report cannot carry a token — or keep a CSP keyword
/// (`inline`, `eval`, …) / a bare scheme (`data`, `blob`).
pub fn reduce_blocked_uri(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let reduced = match url::Url::parse(raw) {
        Ok(u) if u.has_host() => {
            let mut s = format!("{}://{}", u.scheme(), u.host_str().unwrap_or(""));
            if let Some(port) = u.port() {
                s.push_str(&format!(":{port}"));
            }
            s.push_str(u.path());
            s
        }
        Ok(u) => u.scheme().to_string(),
        Err(_) => raw
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect(),
    };
    let reduced: String = reduced.chars().take(MAX_BLOCKED_URI_LEN).collect();
    (!reduced.is_empty()).then_some(reduced)
}

/// The observation payload for one report (schema `undeclared-behaviour`).
pub fn signal_payload(
    signal: &ParsedSignal,
    subject: &SignalSubject,
    envelope: &SignalEnvelope,
    observed_at: &str,
) -> Value {
    let mut payload = serde_json::json!({
        "signal_kind": UNDECLARED_BEHAVIOUR,
        "content_address": subject.content_address,
        "directive": signal.directive,
        "observed_at": observed_at,
        "reported_by": signal.reported_by,
        "reported_via": envelope.reported_via(),
    });
    let obj = payload.as_object_mut().expect("json object");
    let mut put = |key: &str, value: Option<Value>| {
        if let Some(v) = value {
            obj.insert(key.to_string(), v);
        }
    };
    put("record_id", subject.record_id.clone().map(Value::String));
    put("blocked_uri", signal.blocked_uri.clone().map(Value::String));
    put("disposition", signal.disposition.clone().map(Value::String));
    put("relayed_by", envelope.relayed_by.clone().map(Value::String));
    put(
        "observed_origin",
        envelope.observed_origin.clone().map(Value::String),
    );
    // A relaying doorway's check is recorded as it reported it; a direct report
    // is checked here. Neither refuses the report.
    let document_match = if envelope.relayed_by.is_some() {
        envelope.relayed_document_match
    } else {
        document_names_subject(signal.document_url.as_deref(), subject)
    };
    put("document_match", document_match.map(Value::Bool));
    payload
}

/// Outcome of one intake call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntakeOutcome {
    pub recorded: usize,
    pub rate_limited: usize,
}

/// Record each report as an `undeclared-behaviour` observation by `observer`
/// (this peer's agent key) about `subject.content_address`, with the envelope's
/// facts beside it. Reports past the per-address limit or the source's share are
/// counted, not recorded.
#[allow(
    clippy::too_many_arguments,
    reason = "the intake's write takes the observation backend, the connection, the observer, the subject, the envelope, the parsed reports, both clocks and the limiter as separate inputs so a test can pin each one"
)]
pub async fn record_signals(
    manager: &ObservationManagerBackend,
    conn: &mut SqliteConnection,
    observer: &str,
    subject: &SignalSubject,
    envelope: &SignalEnvelope,
    signals: &[ParsedSignal],
    now_unix: i64,
    now_rfc3339: &str,
    limiter: &Mutex<SignalRateLimiter>,
) -> Result<IntakeOutcome, StorageError> {
    let mut outcome = IntakeOutcome {
        recorded: 0,
        rate_limited: 0,
    };
    let source = envelope.source_key();
    for signal in signals {
        let admitted = limiter
            .lock()
            .map(|mut l| l.allow(&subject.content_address, &source, Instant::now()))
            .unwrap_or(false);
        if !admitted {
            outcome.rate_limited += 1;
            continue;
        }
        let last_seq: Option<i64> = observations::table
            .filter(observations::observer_cid.eq(observer))
            .select(diesel::dsl::max(observations::seq))
            .first(conn)
            .map_err(|e| StorageError::Internal(format!("observation seq read failed: {e}")))?;
        let payload = signal_payload(signal, subject, envelope, now_rfc3339);
        if let Some(registry) = manager.kind_registry() {
            registry
                .validate_payload(UNDECLARED_BEHAVIOUR_OBSERVATION, &payload)
                .map_err(StorageError::InvalidInput)?;
        }
        let observation = Observation {
            observer_cid: observer.to_string(),
            log_cid: String::new(),
            log_offset: 0,
            observed_at: now_unix,
            seq: u64::try_from(last_seq.unwrap_or(0)).unwrap_or(0) + 1,
            observation_kind: UNDECLARED_BEHAVIOUR_OBSERVATION.to_string(),
            subject_cid: Some(subject.content_address.clone()),
            subject_kind: Some("content".to_string()),
            payload_json: payload.to_string(),
            observer_household_cid: None,
            observer_collective_cid: None,
            observer_region: None,
            observer_archetype: None,
            observer_compute_class: None,
            signature: Vec::new(),
        };
        manager
            .append_local(conn, observation)
            .await
            .map_err(|e| StorageError::Internal(format!("signal record failed: {e}")))?;
        outcome.recorded += 1;
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::HeaderValue;

    fn subject() -> SignalSubject {
        SignalSubject {
            content_address: "bafkreiexample".into(),
            record_id: Some("evolution-of-trust".into()),
            names: vec![
                "bafkreiexample".into(),
                "sha256-abc".into(),
                "evolution-of-trust".into(),
                "evo".into(),
            ],
        }
    }

    fn browser(directive: &str) -> ParsedSignal {
        ParsedSignal {
            directive: directive.into(),
            reported_by: "browser".into(),
            ..Default::default()
        }
    }

    #[test]
    fn reports_json_keeps_only_csp_violations() {
        let body = serde_json::json!([
            {"type": "csp-violation", "url": "http://127.0.0.1:8090/apps/x/index.html",
             "body": {"effectiveDirective": "connect-src",
                      "blockedURL": "https://tracker.example/collect?id=SECRET#frag",
                      "disposition": "enforce"}},
            {"type": "deprecation", "body": {"id": "x"}},
            {"type": "csp-violation", "body": {"effectiveDirective": "script-src-elem",
                                                "blockedURL": "inline"}}
        ]);
        let parsed = parse_signal_body(
            Some("application/reports+json"),
            body.to_string().as_bytes(),
        )
        .unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].directive, "connect-src");
        assert_eq!(
            parsed[0].blocked_uri.as_deref(),
            Some("https://tracker.example/collect"),
            "query and fragment never recorded"
        );
        assert_eq!(parsed[0].disposition.as_deref(), Some("enforce"));
        assert_eq!(parsed[0].reported_by, "browser");
        assert_eq!(
            parsed[0].document_url.as_deref(),
            Some("http://127.0.0.1:8090/apps/x/index.html"),
            "the report's url stands in when the body names no document"
        );
        assert_eq!(parsed[1].blocked_uri.as_deref(), Some("inline"));
    }

    #[test]
    fn legacy_csp_report_reads_the_directive_name_only() {
        let body = serde_json::json!({"csp-report": {
            "document-uri": "http://127.0.0.1:8090/apps/x/index.html",
            "violated-directive": "frame-ancestors http://localhost:8090",
            "blocked-uri": "http://evil.example:8080/frame",
        }});
        let parsed = parse_signal_body(
            Some("application/csp-report; charset=utf-8"),
            body.to_string().as_bytes(),
        )
        .unwrap();
        assert_eq!(
            parsed,
            vec![ParsedSignal {
                directive: "frame-ancestors".into(),
                blocked_uri: Some("http://evil.example:8080/frame".into()),
                disposition: None,
                reported_by: "browser".into(),
                document_url: Some("http://127.0.0.1:8090/apps/x/index.html".into()),
            }]
        );
    }

    #[test]
    fn json_intent_validates_the_reporter_and_rejects_other_media() {
        let ok = parse_signal_body(
            Some("application/json"),
            br#"{"directive":"popups","reported_by":"inspector"}"#,
        )
        .unwrap();
        assert_eq!(ok[0].reported_by, "inspector");
        assert!(parse_signal_body(
            Some("application/json"),
            br#"{"directive":"popups","reported_by":"doorway"}"#
        )
        .is_err());
        assert!(parse_signal_body(Some("text/plain"), b"{}").is_err());
        assert!(parse_signal_body(Some("application/reports+json"), b"{}").is_err());
        let big = vec![b' '; MAX_SIGNAL_BODY_BYTES + 1];
        assert!(parse_signal_body(Some("application/json"), &big).is_err());
    }

    /// What a browser sends for a violation of the content origin's
    /// `Content-Security-Policy-Report-Only` (trustful content is observed, not
    /// blocked): `disposition: "report"`, in both the Reporting API and the
    /// legacy body. Both parse and reduce exactly like an enforced report.
    #[test]
    fn report_only_violations_parse_in_both_forms() {
        let reports = serde_json::json!([{
            "type": "csp-violation",
            "age": 3,
            "url": "http://127.0.0.1:8090/apps/evo/index.html",
            "user_agent": "Mozilla/5.0",
            "body": {
                "documentURL": "http://127.0.0.1:8090/apps/evo/index.html",
                "blockedURL": "https://cdn.undeclared.example/lib.js?v=1&token=SECRET",
                "effectiveDirective": "script-src-elem",
                "originalPolicy": "default-src 'self' blob: data: 'unsafe-inline' 'unsafe-eval'; report-uri /api/v1/epr/evo/signals; report-to elohim-signals",
                "disposition": "report",
                "statusCode": 200
            }
        }]);
        let a = parse_signal_body(
            Some("application/reports+json"),
            reports.to_string().as_bytes(),
        )
        .unwrap();
        let legacy = serde_json::json!({"csp-report": {
            "document-uri": "http://127.0.0.1:8090/apps/evo/index.html",
            "violated-directive": "script-src-elem",
            "effective-directive": "script-src-elem",
            "original-policy": "default-src 'self'; report-uri /api/v1/epr/evo/signals",
            "disposition": "report",
            "blocked-uri": "https://cdn.undeclared.example/lib.js?v=1&token=SECRET",
            "status-code": 200
        }});
        let b = parse_signal_body(
            Some("application/csp-report"),
            legacy.to_string().as_bytes(),
        )
        .unwrap();
        let expected = ParsedSignal {
            directive: "script-src-elem".into(),
            blocked_uri: Some("https://cdn.undeclared.example/lib.js".into()),
            disposition: Some("report".into()),
            reported_by: "browser".into(),
            document_url: Some("http://127.0.0.1:8090/apps/evo/index.html".into()),
        };
        assert_eq!(a, vec![expected.clone()]);
        assert_eq!(b, vec![expected]);
    }

    #[test]
    fn blocked_uri_reduction() {
        assert_eq!(
            reduce_blocked_uri("data:image/png;base64,AAAA").as_deref(),
            Some("data")
        );
        assert_eq!(reduce_blocked_uri("eval").as_deref(), Some("eval"));
        assert_eq!(reduce_blocked_uri("  ").as_deref(), None);
    }

    #[test]
    fn limiter_shares_an_addresss_window_between_sources() {
        let mut limiter = SignalRateLimiter::default();
        let t0 = Instant::now();
        for _ in 0..MAX_SIGNALS_PER_SOURCE {
            assert!(limiter.allow("bafkreia", "doorway:alpha", t0));
        }
        assert!(
            !limiter.allow("bafkreia", "doorway:alpha", t0),
            "one source cannot take more than its share"
        );
        // Other sources fill the rest of the address's allowance.
        let others = MAX_SIGNALS_PER_WINDOW - MAX_SIGNALS_PER_SOURCE;
        for i in 0..others {
            assert!(limiter.allow("bafkreia", &format!("direct:10.0.0.{i}"), t0));
        }
        assert!(
            !limiter.allow("bafkreia", "direct:10.0.0.250", t0),
            "the address's whole allowance is spent"
        );
        assert!(
            limiter.allow("bafkreib", "doorway:alpha", t0),
            "limits are per content address"
        );
        assert!(
            limiter.allow("bafkreia", "doorway:alpha", t0 + SIGNAL_WINDOW),
            "a new window"
        );
    }

    #[test]
    fn envelope_reads_the_doorway_relay_or_the_direct_request() {
        let mut relayed = hyper::HeaderMap::new();
        relayed.insert(RELAYED_BY_HEADER, HeaderValue::from_static("doorway-alpha"));
        relayed.insert(
            OBSERVED_ORIGIN_HEADER,
            HeaderValue::from_static("https://apps.alpha.elohim.host"),
        );
        relayed.insert(DOCUMENT_MATCH_HEADER, HeaderValue::from_static("false"));
        relayed.insert(
            "origin",
            HeaderValue::from_static("https://ignored.example"),
        );
        let e = envelope_from_headers(&relayed, Some("10.0.0.9".parse().unwrap()));
        assert_eq!(e.reported_via(), "doorway");
        assert_eq!(e.relayed_by.as_deref(), Some("doorway-alpha"));
        assert_eq!(
            e.observed_origin.as_deref(),
            Some("https://apps.alpha.elohim.host"),
            "a relayed report records what the doorway observed"
        );
        assert_eq!(e.relayed_document_match, Some(false));
        assert_eq!(e.source_key(), "doorway:doorway-alpha");

        let mut direct = hyper::HeaderMap::new();
        direct.insert(
            "referer",
            HeaderValue::from_static("http://127.0.0.1:8090/apps/evo/index.html?x=1"),
        );
        let d = envelope_from_headers(&direct, Some("127.0.0.1".parse().unwrap()));
        assert_eq!(d.reported_via(), "direct");
        assert_eq!(d.observed_origin.as_deref(), Some("http://127.0.0.1:8090"));
        assert_eq!(d.relayed_document_match, None);
        assert_eq!(d.source_key(), "direct:127.0.0.1");
    }

    #[test]
    fn direct_reports_are_checked_against_the_subject_and_never_refused() {
        let s = subject();
        let direct = SignalEnvelope::default();
        let mut on_ours = browser("connect-src");
        on_ours.document_url = Some("http://127.0.0.1:8090/apps/evo/index.html".into());
        let p = signal_payload(&on_ours, &s, &direct, "t");
        assert_eq!(p["document_match"], true);
        assert_eq!(p["reported_via"], "direct");

        let mut elsewhere = browser("connect-src");
        elsewhere.document_url = Some("http://127.0.0.1:8090/apps/other-app/index.html".into());
        let p = signal_payload(&elsewhere, &s, &direct, "t");
        assert_eq!(p["document_match"], false, "recorded, not refused");

        let p = signal_payload(&browser("connect-src"), &s, &direct, "t");
        assert!(p.get("document_match").is_none(), "no document → no claim");

        // A doorway's verdict is recorded as reported, not re-derived.
        let relayed = SignalEnvelope {
            relayed_by: Some("doorway-alpha".into()),
            observed_origin: Some("https://apps.alpha.elohim.host".into()),
            relayed_document_match: Some(true),
            client: Some("10.0.0.9".into()),
        };
        let p = signal_payload(&elsewhere, &s, &relayed, "t");
        assert_eq!(p["document_match"], true);
        assert_eq!(p["relayed_by"], "doorway-alpha");
        assert_eq!(p["reported_via"], "doorway");
    }

    #[test]
    fn payload_matches_the_feedback_signal_schema() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../sdk/schemas/v1/feedback-signals/undeclared-behaviour.schema.json"
        ))
        .unwrap();
        let mut signal = browser("connect-src");
        signal.blocked_uri = Some("https://x.example/a".into());
        signal.disposition = Some("report".into());
        signal.document_url = Some("https://apps.alpha.elohim.host/apps/evo/index.html".into());
        let payload = signal_payload(
            &signal,
            &subject(),
            &SignalEnvelope {
                relayed_by: Some("doorway-alpha".into()),
                observed_origin: Some("https://apps.alpha.elohim.host".into()),
                relayed_document_match: Some(true),
                client: None,
            },
            "2026-10-15T12:00:00Z",
        );
        let props = schema["properties"].as_object().unwrap();
        for key in payload.as_object().unwrap().keys() {
            assert!(props.contains_key(key), "payload key {key} not in schema");
        }
        for required in schema["required"].as_array().unwrap() {
            assert!(
                payload.get(required.as_str().unwrap()).is_some(),
                "{required}"
            );
        }
        assert_eq!(
            payload["signal_kind"],
            schema["properties"]["signal_kind"]["const"]
        );
        assert!(!payload.as_object().unwrap().contains_key("document_url"));
    }

    #[tokio::test]
    async fn record_signals_appends_observations_by_this_peer_about_the_address() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let manager = ObservationManagerBackend::new();
        let limiter = Mutex::new(SignalRateLimiter::default());
        let signals = vec![browser("connect-src"); 2];
        let out = record_signals(
            &manager,
            &mut conn,
            "uhCAkthispeer",
            &subject(),
            &SignalEnvelope::default(),
            &signals,
            1_760_000_000,
            "2026-10-15T12:00:00Z",
            &limiter,
        )
        .await
        .unwrap();
        assert_eq!(
            out,
            IntakeOutcome {
                recorded: 2,
                rate_limited: 0
            }
        );
        let rows = manager
            .query_by_subject(
                &mut conn,
                "bafkreiexample",
                UNDECLARED_BEHAVIOUR_OBSERVATION,
            )
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].observer_cid, "uhCAkthispeer");

        // Read back through the existing route the a2o glue uses:
        // GET /api/v1/observations/by-subject?subjectCid=<content address>&kind=…
        let registry = crate::services::observation_kinds::ObservationKindRegistry::embedded();
        let via_route = crate::api::observations::observations_by_subject(
            &mut conn,
            Some(&registry),
            &format!("subjectCid=bafkreiexample&kind={UNDECLARED_BEHAVIOUR_OBSERVATION}"),
        )
        .unwrap();
        assert_eq!(
            via_route.len(),
            2,
            "by-subject returns the recorded reports"
        );
        assert_eq!(via_route[0].subject_cid.as_deref(), Some("bafkreiexample"));
    }

    #[test]
    fn the_embedded_manifest_declares_the_kind_and_accepts_the_payload() {
        let registry = crate::services::observation_kinds::ObservationKindRegistry::embedded();
        let decl = registry
            .get(UNDECLARED_BEHAVIOUR_OBSERVATION)
            .expect("elohim manifest declares elohim:undeclared-behaviour");
        assert_eq!(decl.retention_class, "attestation-feeding");
        assert_eq!(
            decl.reach, UNDECLARED_BEHAVIOUR_REACH,
            "the intake's reach floor is the reach the manifest declares the kind at"
        );
        assert!(
            INTAKE_ONLY_OBSERVATION_KINDS
                .iter()
                .all(|kind| registry.get(kind).is_some()),
            "every intake-only kind is manifest-declared"
        );
        let mut full_signal = browser("connect-src");
        full_signal.blocked_uri = Some("https://x.example/a".into());
        full_signal.disposition = Some("enforce".into());
        let full = signal_payload(
            &full_signal,
            &subject(),
            &SignalEnvelope {
                relayed_by: Some("doorway-alpha".into()),
                observed_origin: Some("https://apps.alpha.elohim.host".into()),
                relayed_document_match: Some(false),
                client: None,
            },
            "2026-10-15T12:00:00Z",
        );
        registry
            .validate_payload(UNDECLARED_BEHAVIOUR_OBSERVATION, &full)
            .unwrap();
        let minimal_subject = SignalSubject {
            record_id: None,
            ..subject()
        };
        let minimal = signal_payload(
            &browser("frame-ancestors"),
            &minimal_subject,
            &SignalEnvelope::default(),
            "2026-10-15T12:00:00Z",
        );
        registry
            .validate_payload(UNDECLARED_BEHAVIOUR_OBSERVATION, &minimal)
            .unwrap();
    }

    #[tokio::test]
    async fn recording_is_validated_by_the_wired_registry() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let manager = ObservationManagerBackend::new().with_kind_registry(std::sync::Arc::new(
            crate::services::observation_kinds::ObservationKindRegistry::embedded(),
        ));
        let mut signal = browser("script-src-elem");
        signal.blocked_uri = Some("https://cdn.undeclared.example/lib.js".into());
        signal.disposition = Some("report".into());
        signal.document_url = Some("http://127.0.0.1:8090/apps/evo/index.html".into());
        let out = record_signals(
            &manager,
            &mut conn,
            "uhCAkthispeer",
            &subject(),
            &SignalEnvelope::default(),
            &[signal],
            1_760_000_000,
            "2026-10-15T12:00:00Z",
            &Mutex::new(SignalRateLimiter::default()),
        )
        .await
        .unwrap();
        assert_eq!(out.recorded, 1);
    }
}
