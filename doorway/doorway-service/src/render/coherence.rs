//! Shell coherence — is the head this doorway is about to serve *deliverable
//! through this doorway*?
//!
//! ## The defect this closes (measured 2026-09-08)
//!
//! The 2026-09-04 rails made a shell prove it was FETCHED BY the head the
//! doorway declares (`head_bound`). That is a provenance proof, not a
//! deliverability proof: on 2026-09-08 both fleet doorways served `/` from the
//! warm cache at browser head `sha256-6899…` for 15 hours after storage had
//! moved to `sha256-e0e2f7…`. The page named `main-AVSOD6V6.js`, which 404'd
//! through the doorway, while `main-SQRMM2WZ.js` — the new head's entry — served
//! 200 on the slug path. Every rail held; the visitor got a blank page.
//!
//! ## The cure
//!
//! `AtHead` now needs a second proof: **the entry script the shell names
//! resolves through THIS doorway for THAT head**. The storage peer already
//! judges exactly this from the bundle bytes inside its extraction walk
//! (`app_deliverability::judge_deliverability`, 2026-09-05) and stamps the
//! verdict on its `/apps/` surfaces, so the primary probe is one bounded
//! `HEAD /apps/{head}/_capability` reading `X-Deliverability`
//! (`boots` | `broken` | `not-judged`) + `X-Deliverability-Reason`.
//!
//! **`HEAD` is only correct for `_capability`.** `/apps/{id}/{file}` is
//! GET-only in `elohim-storage/src/http.rs` (the `(Method::HEAD, …)` arm covers
//! `_capability` alone), so a `HEAD /apps/{head}/main-X.js` probe returns 404
//! for a file that GETs 200 — the same content-addressed HEAD/GET asymmetry the
//! blob route carries. The fallback probe therefore uses `GET` with
//! `Range: bytes=0-0`, never `HEAD`.
//!
//! ## Memo semantics (why they are asymmetric)
//!
//! - **Coherent is PERMANENT for a head.** A head is content-addressed: the
//!   bytes behind `sha256-e0e2…` cannot change, so one confirmation is
//!   confirmation forever. This is also what keeps a catch-up window from
//!   demoting a shell that was already proven — an unreachable upstream can
//!   never un-prove what it already proved.
//! - **Incoherent / unreachable is TTL'd** ([`COHERENCE_RECHECK_SECS`]): the
//!   asset may simply not be extracted yet, so the verdict is re-probed on the
//!   next interval and the shell converges on its own. Between probes the last
//!   verdict stands, so an incoherent head costs at most one probe per interval,
//!   not one per request.

use std::sync::Arc;
use std::time::Duration;

/// Why a shell is not confirmed at the declared head. Rendered on the wire as
/// `x-elohim-bundle: behind;<reason>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehindReason {
    /// The head's bundle does not hold an asset the shell names.
    MissingAsset(String),
    /// The doorway's projection declares no head for this app, so nothing can
    /// be confirmed against one.
    HeadUnknown,
    /// The coherence probe could not be made or answered.
    StorageUnreachable,
    /// Bytes in hand, but they are not proof of the declared head (a different
    /// head, or an archive doc with no `head_bound` marker).
    StaleProjection,
}

impl BehindReason {
    /// The reason token, as it appears after `behind;`.
    pub fn as_wire(&self) -> String {
        match self {
            BehindReason::MissingAsset(file) => format!("missing-asset:{file}"),
            BehindReason::HeadUnknown => "head-unknown".to_string(),
            BehindReason::StorageUnreachable => "storage-unreachable".to_string(),
            BehindReason::StaleProjection => "stale-projection".to_string(),
        }
    }

    /// The bounded reason CLASS (the segment before the first `:`), for metrics
    /// and logs — `missing-asset:main-ABC.js` is unbounded cardinality.
    pub fn class(&self) -> &'static str {
        match self {
            BehindReason::MissingAsset(_) => "missing-asset",
            BehindReason::HeadUnknown => "head-unknown",
            BehindReason::StorageUnreachable => "storage-unreachable",
            BehindReason::StaleProjection => "stale-projection",
        }
    }
}

/// A coherence judgement for one head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoherenceVerdict {
    /// The entry the shell names resolves through this doorway for this head.
    Coherent,
    /// It does not, or could not be shown to.
    Incoherent(BehindReason),
}

impl CoherenceVerdict {
    pub fn is_coherent(&self) -> bool {
        matches!(self, CoherenceVerdict::Coherent)
    }

    /// Whether this verdict may be memoised permanently. Only a confirmation
    /// may: a head is content-addressed, so it cannot become incoherent — but a
    /// missing asset can arrive (extraction lag) and an unreachable peer can
    /// return.
    pub fn is_permanent(&self) -> bool {
        self.is_coherent()
    }
}

/// Seconds a NEGATIVE coherence verdict stands before it is re-probed.
///
/// The positive verdict is permanent (see the module doc), so this bounds only
/// the retry rate of a head that has never been confirmed. Matching the shell
/// upgrade interval keeps one cadence for "how often does this doorway re-ask
/// the upstream about a shell it cannot confirm".
pub const COHERENCE_RECHECK_SECS: u64 = 30;

/// Per-probe ceiling. A coherence probe sits on the `/` hot path behind a warm
/// serve we can already answer with, so it must never gamble a browser
/// navigation on a slow peer.
pub const COHERENCE_PROBE_TIMEOUT_SECS: u64 = 2;

/// Judges whether a head is deliverable through this doorway.
///
/// A trait so the warm-shell decisions stay unit-testable without a live
/// storage peer — the same mocking seam shape [`crate::render::warm_shell::ShellArchive`]
/// uses.
#[async_trait::async_trait]
pub trait CoherenceOracle: Send + Sync {
    /// `entry` is the asset the shell names (parsed from its own bytes), used
    /// only by the fallback probe when the peer returns no verdict of its own.
    async fn judge(&self, head: &str, entry: Option<&str>) -> CoherenceVerdict;
}

/// The live oracle: ask the storage peer for its own deliverability verdict,
/// and fall back to probing the entry asset when it has none.
pub struct StorageCoherenceProbe {
    storage_base: String,
    client: reqwest::Client,
}

impl StorageCoherenceProbe {
    pub fn new(storage_base_url: String) -> Self {
        Self {
            storage_base: storage_base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(COHERENCE_PROBE_TIMEOUT_SECS))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn shared(storage_base_url: String) -> Arc<dyn CoherenceOracle> {
        Arc::new(Self::new(storage_base_url))
    }
}

#[async_trait::async_trait]
impl CoherenceOracle for StorageCoherenceProbe {
    async fn judge(&self, head: &str, entry: Option<&str>) -> CoherenceVerdict {
        // Primary: the peer's own verdict, judged from the bundle bytes. HEAD is
        // ROUTED for `_capability` (and only for it) — see the module doc.
        let capability_url = format!("{}/apps/{}/_capability", self.storage_base, head);
        match self.client.head(&capability_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let verdict = header_str(&resp, "x-deliverability");
                match verdict.as_deref() {
                    Some("boots") => return CoherenceVerdict::Coherent,
                    Some("broken") => {
                        let reason = header_str(&resp, "x-deliverability-reason");
                        return CoherenceVerdict::Incoherent(parse_reason(
                            reason.as_deref(),
                            entry,
                        ));
                    }
                    // `not-judged`, or a peer too old to stamp the header: fall
                    // through to the asset probe rather than demote a head the
                    // peer simply has no opinion about.
                    _ => {}
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::debug!(
                    target: "doorway::ssr",
                    head = %head,
                    error = %e,
                    "shell coherence: capability probe failed"
                );
                return CoherenceVerdict::Incoherent(BehindReason::StorageUnreachable);
            }
        }

        // Fallback: does the entry asset itself resolve for this head? GET with
        // a one-byte Range — `/apps/{id}/{file}` is GET-only, so a HEAD here
        // 404s for a file that GETs 200.
        let Some(entry) = entry else {
            // Nothing to probe and no verdict from the peer: unconfirmed, not
            // disproven. Treated as unreachable so it is re-probed, never
            // memoised as a permanent pass.
            return CoherenceVerdict::Incoherent(BehindReason::StorageUnreachable);
        };
        let asset_url = format!("{}/apps/{}/{}", self.storage_base, head, entry);
        match self
            .client
            .get(&asset_url)
            .header("Range", "bytes=0-0")
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => CoherenceVerdict::Coherent,
            Ok(resp) if resp.status() == reqwest::StatusCode::NOT_FOUND => {
                CoherenceVerdict::Incoherent(BehindReason::MissingAsset(entry.to_string()))
            }
            Ok(_) => CoherenceVerdict::Incoherent(BehindReason::StorageUnreachable),
            Err(_) => CoherenceVerdict::Incoherent(BehindReason::StorageUnreachable),
        }
    }
}

fn header_str(resp: &reqwest::Response, name: &str) -> Option<String> {
    resp.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Map storage's `X-Deliverability-Reason` onto this doorway's reason
/// vocabulary. Storage already speaks `missing-asset:<file>`; anything else it
/// reports is carried through as a missing-asset naming what it named, so the
/// wire never loses the peer's own diagnosis.
fn parse_reason(storage_reason: Option<&str>, entry: Option<&str>) -> BehindReason {
    match storage_reason {
        Some(r) => match r.split_once(':') {
            Some(("missing-asset", file)) => BehindReason::MissingAsset(file.to_string()),
            _ => BehindReason::MissingAsset(r.to_string()),
        },
        None => BehindReason::MissingAsset(entry.unwrap_or("unknown").to_string()),
    }
}

/// The entry script a shell names — the one asset whose absence makes the page
/// blank. Prefers a `main-*.js` (the Angular browser build's entry) and falls
/// back to the first same-bundle script.
///
/// Pure and deliberately not an HTML parser: a shell is a build artifact with a
/// stable shape, and pulling in a DOM parser for the `/` hot path buys nothing.
pub fn entry_script(html: &str) -> Option<String> {
    let scripts = shell_assets(html);
    scripts
        .iter()
        .find(|s| {
            let file = s.rsplit('/').next().unwrap_or(s);
            file.starts_with("main-") && file.ends_with(".js")
        })
        .or_else(|| scripts.first())
        .cloned()
}

/// Every same-bundle asset the shell names: `<script src>` and
/// `<link rel="modulepreload" href>`. Absolute URLs and parent-escaping paths
/// are dropped — they are not this head's bytes, so they say nothing about it.
pub fn shell_assets(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (tag, attr) in [("<script", "src=\""), ("<link", "href=\"")] {
        let mut rest = html;
        while let Some(pos) = rest.find(tag) {
            rest = &rest[pos + tag.len()..];
            let Some(end) = rest.find('>') else { break };
            let element = &rest[..end];
            if tag == "<link" && !element.contains("modulepreload") {
                continue;
            }
            if let Some(v) = element.find(attr) {
                let value = &element[v + attr.len()..];
                if let Some(q) = value.find('"') {
                    let asset = normalize_asset(&value[..q]);
                    if let Some(asset) = asset {
                        if !out.contains(&asset) {
                            out.push(asset);
                        }
                    }
                }
            }
        }
    }
    out
}

/// A bundle-relative asset path, or `None` when the reference is not this
/// head's bytes (absolute URL, protocol-relative, or `..`-escaping).
fn normalize_asset(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.starts_with("http://")
        || raw.starts_with("https://")
        || raw.starts_with("//")
        || raw.starts_with("data:")
        || raw.contains("..")
    {
        return None;
    }
    Some(raw.trim_start_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHELL: &str = r#"<!doctype html><html><head>
        <base href="/"><link rel="modulepreload" href="chunk-ABC.js">
        <script src="polyfills-XYZ.js" type="module"></script>
        <script src="main-SQRMM2WZ.js" type="module"></script>
        <script src="https://cdn.example/analytics.js"></script>
        </head><body><app-root></app-root></body></html>"#;

    #[test]
    fn the_entry_script_is_the_main_bundle_not_merely_the_first_script() {
        // polyfills comes FIRST in the document; the asset whose absence blanks
        // the page is `main-*`.
        assert_eq!(
            entry_script(SHELL).as_deref(),
            Some("main-SQRMM2WZ.js"),
            "the 2026-09-08 blank page named main-AVSOD6V6.js — that is the probe target"
        );
    }

    #[test]
    fn assets_exclude_cross_origin_and_escaping_references() {
        let assets = shell_assets(SHELL);
        assert!(assets.contains(&"main-SQRMM2WZ.js".to_string()));
        assert!(assets.contains(&"polyfills-XYZ.js".to_string()));
        assert!(assets.contains(&"chunk-ABC.js".to_string()));
        assert!(
            !assets.iter().any(|a| a.contains("cdn.example")),
            "a cross-origin script says nothing about THIS head's bytes"
        );
    }

    #[test]
    fn a_stylesheet_link_is_not_a_modulepreload() {
        let html =
            r#"<link rel="stylesheet" href="styles-A.css"><script src="main-B.js"></script>"#;
        let assets = shell_assets(html);
        assert_eq!(assets, vec!["main-B.js".to_string()]);
    }

    #[test]
    fn a_shell_with_no_scripts_names_no_entry() {
        assert_eq!(entry_script("<html><body>hi</body></html>"), None);
    }

    #[test]
    fn reason_tokens_are_the_declared_vocabulary() {
        assert_eq!(
            BehindReason::MissingAsset("main-X.js".into()).as_wire(),
            "missing-asset:main-X.js"
        );
        assert_eq!(BehindReason::HeadUnknown.as_wire(), "head-unknown");
        assert_eq!(
            BehindReason::StorageUnreachable.as_wire(),
            "storage-unreachable"
        );
        assert_eq!(BehindReason::StaleProjection.as_wire(), "stale-projection");
        // The metrics label stays bounded even though the wire value is not.
        assert_eq!(
            BehindReason::MissingAsset("main-X.js".into()).class(),
            "missing-asset"
        );
    }

    #[test]
    fn storages_own_reason_is_carried_through_not_re_derived() {
        assert_eq!(
            parse_reason(Some("missing-asset:main-A.js"), Some("main-B.js")),
            BehindReason::MissingAsset("main-A.js".into()),
            "the peer judged the bundle bytes; its file name wins over ours"
        );
        assert_eq!(
            parse_reason(None, Some("main-B.js")),
            BehindReason::MissingAsset("main-B.js".into())
        );
    }

    #[test]
    fn only_a_confirmation_may_be_memoised_forever() {
        assert!(CoherenceVerdict::Coherent.is_permanent());
        assert!(
            !CoherenceVerdict::Incoherent(BehindReason::StorageUnreachable).is_permanent(),
            "an unreachable peer returns; a missing asset can still be extracted"
        );
        assert!(
            !CoherenceVerdict::Incoherent(BehindReason::MissingAsset("x".into())).is_permanent()
        );
    }
}
