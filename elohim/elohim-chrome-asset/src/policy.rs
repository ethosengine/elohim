//! The chrome's web-edge content policy: the Content-Security-Policy a peer or
//! a doorway attaches to RAW content served on a content origin, so the browser
//! can report what the content does beyond what its record declares.
//!
//! One pure function, [`content_policy`], keyed by the content's trust mode, so
//! every medium that serves raw content (the storage peer's own content origin,
//! a doorway's `apps.<name>`) emits the byte-identical policy:
//!
//! - **trustful** (the record came through the protocol's gates): the content is
//!   OBSERVED, not blocked — `Content-Security-Policy-Report-Only`. A gated app
//!   keeps working; anything it reaches outside its own origin and its declared
//!   `sources` becomes a browser report, recorded as an `undeclared-behaviour`
//!   signal against its content address.
//! - **trustless** (no protocol record): the same baseline ENFORCED as a
//!   `Content-Security-Policy`, without `'unsafe-inline'` / `'unsafe-eval'` —
//!   the strict profile. Trustful never adapts to it.
//!
//! `frame-ancestors` is deliberately NOT part of this policy: browsers ignore it
//! in a Report-Only policy, so the producer keeps it in its own enforced
//! `Content-Security-Policy` header beside this one.
//!
//! Why a reportable policy at all: a framed app touching `parent.document`
//! throws a `SecurityError` that no browser reports anywhere. Only a policy
//! the browser checks produces a report the network can hear.

use crate::TrustMode;

/// The Reporting API endpoint name a content origin declares in
/// `Reporting-Endpoints` and the policy names in `report-to`.
pub const REPORT_ENDPOINT_NAME: &str = "elohim-signals";

/// One response header: its name and value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentPolicy {
    /// `Content-Security-Policy-Report-Only` (trustful) or
    /// `Content-Security-Policy` (trustless).
    pub header_name: &'static str,
    pub value: String,
}

/// A declared source is used only when it is a bare origin token a CSP source
/// list can carry: no whitespace, `;`, `,` or quotes (which would split or
/// escape the directive), and an explicit scheme. The record's parser
/// (lamad-types `normalize_origin`) already admits only origins; this is the
/// edge's own guard against a producer that skipped it.
fn is_source_token(source: &str) -> bool {
    !source.is_empty()
        && source.contains("://")
        && !source
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, ';' | ',' | '\'' | '"'))
}

/// A Reporting API group name a directive can carry: a non-empty token.
fn is_group_token(group: &str) -> bool {
    !group.is_empty()
        && group
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// The policy for raw content of `mode`, allowing its own origin plus the
/// record's declared `sources`, reporting to `report_uri` and — when the
/// response also declares a `Reporting-Endpoints` group — to `report_to` (that
/// group's name, conventionally [`REPORT_ENDPOINT_NAME`]). A `report-to` naming a
/// group the response never declared would displace `report-uri` in browsers
/// that support both, so pass `None` then.
///
/// Where reports go is the producer's: the storage peer reports straight to its
/// signal intake (`/api/v1/epr/{id}/signals`); a doorway, whose apps origin the
/// browser cannot route to storage from, reports to its own relay. Everything
/// else in the text is identical in every producer.
#[must_use]
pub fn content_policy(
    mode: TrustMode,
    sources: &[String],
    report_uri: &str,
    report_to: Option<&str>,
) -> ContentPolicy {
    let mut sources_list = String::new();
    for source in sources
        .iter()
        .map(String::as_str)
        .filter(|s| is_source_token(s))
    {
        sources_list.push(' ');
        sources_list.push_str(source);
    }
    let (header_name, baseline) = match mode {
        TrustMode::Trustful => (
            "Content-Security-Policy-Report-Only",
            "default-src 'self' blob: data: 'unsafe-inline' 'unsafe-eval'",
        ),
        TrustMode::Trustless => ("Content-Security-Policy", "default-src 'self' blob: data:"),
    };
    let mut value = format!("{baseline}{sources_list}");
    if mode == TrustMode::Trustless {
        value.push_str("; object-src 'none'; base-uri 'none'");
    }
    value.push_str(&format!("; report-uri {report_uri}"));
    if let Some(group) = report_to.filter(|g| is_group_token(g)) {
        value.push_str(&format!("; report-to {group}"));
    }
    ContentPolicy { header_name, value }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> Vec<String> {
        vec![
            "https://fonts.example.org".to_string(),
            "wss://live.example.org:8443".to_string(),
        ]
    }

    #[test]
    fn trustful_content_is_observed_with_a_report_only_policy() {
        let p = content_policy(
            TrustMode::Trustful,
            &sources(),
            "/api/v1/epr/evo/signals",
            Some(REPORT_ENDPOINT_NAME),
        );
        assert_eq!(p.header_name, "Content-Security-Policy-Report-Only");
        assert_eq!(
            p.value,
            "default-src 'self' blob: data: 'unsafe-inline' 'unsafe-eval' \
             https://fonts.example.org wss://live.example.org:8443; \
             report-uri /api/v1/epr/evo/signals; report-to elohim-signals"
        );
        assert!(
            !p.value.contains("frame-ancestors"),
            "ignored in Report-Only; enforced elsewhere"
        );
    }

    #[test]
    fn trustless_content_gets_the_strict_profile_enforced() {
        let p = content_policy(
            TrustMode::Trustless,
            &sources(),
            "/api/v1/epr/x/signals",
            Some(REPORT_ENDPOINT_NAME),
        );
        assert_eq!(p.header_name, "Content-Security-Policy");
        assert_eq!(
            p.value,
            "default-src 'self' blob: data: https://fonts.example.org wss://live.example.org:8443; \
             object-src 'none'; base-uri 'none'; \
             report-uri /api/v1/epr/x/signals; report-to elohim-signals"
        );
        assert!(!p.value.contains("unsafe"));
    }

    #[test]
    fn a_source_that_could_split_the_directive_is_never_emitted() {
        let hostile = vec![
            "https://ok.example".to_string(),
            "https://evil.example; script-src *".to_string(),
            "'unsafe-hashes'".to_string(),
            "evil.example".to_string(),
            String::new(),
        ];
        let p = content_policy(TrustMode::Trustful, &hostile, "/r", None);
        assert!(p.value.starts_with(
            "default-src 'self' blob: data: 'unsafe-inline' 'unsafe-eval' https://ok.example; report-uri /r"
        ));
        assert_eq!(
            p.value.matches(';').count(),
            1,
            "only the report-uri separator"
        );
    }

    #[test]
    fn the_report_endpoint_is_the_producers_and_the_rest_is_identical() {
        let storage = content_policy(
            TrustMode::Trustful,
            &sources(),
            "/api/v1/epr/bafkreiabc/signals",
            Some(REPORT_ENDPOINT_NAME),
        );
        let doorway = content_policy(
            TrustMode::Trustful,
            &sources(),
            "/_elohim/csp-report/bafkreiabc",
            Some(REPORT_ENDPOINT_NAME),
        );
        let strip = |v: &str| v.split("; report-uri").next().unwrap().to_string();
        assert_eq!(strip(&storage.value), strip(&doorway.value));
        assert!(doorway
            .value
            .contains("report-uri /_elohim/csp-report/bafkreiabc"));
        assert_eq!(storage.header_name, doorway.header_name);
        // A group name that could split the directive is never emitted.
        let bad = content_policy(TrustMode::Trustful, &[], "/r", Some("g; script-src *"));
        assert!(!bad.value.contains("report-to"));
    }

    #[test]
    fn no_declared_sources_means_own_origin_only() {
        let p = content_policy(TrustMode::Trustful, &[], "/r", Some(REPORT_ENDPOINT_NAME));
        assert_eq!(
            p.value,
            "default-src 'self' blob: data: 'unsafe-inline' 'unsafe-eval'; report-uri /r; report-to elohim-signals"
        );
        let p = content_policy(TrustMode::Trustful, &[], "/r", None);
        assert_eq!(
            p.value, "default-src 'self' blob: data: 'unsafe-inline' 'unsafe-eval'; report-uri /r",
            "no declared endpoint → no report-to"
        );
    }
}
