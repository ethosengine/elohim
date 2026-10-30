//! `ChromeContext` — the PRODUCER side of the omni element's `ctx` contract.
//!
//! The shape is `elohim/sdk/schemas/v1/views/chrome-context.schema.json`
//! (version 1); the schema-contract test in `lib.rs` validates serialized
//! contexts against it. The element (`omni-element.js`) is the CONSUMER.
//! This struct is the single typed producer both callers (the doorway's
//! `build_chrome_context_json`, and any future producer) construct and
//! serialize — closing the same class of drift that shipped the resilience
//! snapshot bug (a hand-built `serde_json::json!` and the element's reads
//! silently diverging). See `lib.rs`'s structural contract test, which
//! mirrors `resilience_mapper_speaks_the_snapshot_contract`.
//!
//! `slug` and `authenticated` are the only fields the doorway supplies today
//! (it is the sole current producer); every other field is `None` by
//! default so [`ChromeContext::to_json`] on a minimally-populated instance
//! emits exactly today's island shape (`{"slug":…,"authenticated":…}`) —
//! byte-stable, because `#[serde(skip_serializing_if = "Option::is_none")]`
//! omits absent optionals rather than serializing a null/zero value the
//! element would otherwise (mis)interpret as an authored signal.

use serde::Serialize;

/// Island shape version this crate produces (`chrome-context.schema.json`).
pub const CHROME_CONTEXT_VERSION: u32 = 1;

/// A single nav-link entry (`ctx.navBack` / `ctx.navForward`). The element
/// reads `.href` (routed through its own `safeHref` scheme allowlist before
/// rendering) and `.label` (defaulted to `''` if absent).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavLink {
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// The producer side of the omni element's `ctx` contract
/// (`elohim/elohim-chrome-asset/src/omni-element.js`).
///
/// Every field mirrors a `ctx.<field>` read in the element JS. Keep the two
/// in sync: adding or removing a `ctx.<field>` read there without a matching
/// change here (and vice versa) is exactly the drift class the crate's
/// structural contract test (`chrome_context_speaks_the_producer_contract`
/// in `lib.rs`) is built to catch.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromeContext {
    /// The EPR address. Always supplied — the element falls back to
    /// `window.location`-derived resolution only when no context (inline or
    /// fetched) is available at all.
    pub slug: String,
    /// Whether this request/session carried a credential. Gates the account
    /// link (`data-omni-account`); the element cannot determine this
    /// client-side, so a producer must supply it explicitly (default
    /// `false`, matching `Default`).
    pub authenticated: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The ambient theme (`theme`): a bare colour scheme, or the record's
    /// theme hint object (schema `anyOf: string | $defs/themeHint`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<ChromeTheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_marker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_env: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_theme_toggle: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nav_back: Option<NavLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nav_forward: Option<NavLink>,
    /// The fair-trade receipt affordance (`ctx.receipt`): the sentence the
    /// chrome leads with and the route that reads the whole thing back.
    ///
    /// `None` — and therefore ABSENT from the island — whenever the response
    /// being framed accounted for no exchange. A chrome that always rendered a
    /// receipt line would be asserting an exchange nobody recorded, which is the
    /// same untruth as a synthesized credit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<ReceiptLink>,

    // ── v1 (`elohim/sdk/schemas/v1/views/chrome-context.schema.json`) ───────
    // All `None` by default so a pre-v1 context stays byte-stable. A v1
    // producer sets `version`, `viewer` and `medium` together (see
    // [`ChromeContext::v1`]).
    /// Island shape version (`1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// Who is looking. Supersedes `authenticated`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer: Option<Viewer>,
    /// Which medium mounted the chrome.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<ChromeMedium>,
    /// Base URL the chrome resolves projection fetches against (absent =
    /// same-origin).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readings_base: Option<String>,
    /// Present only on a content shell.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ContentIsland>,
}

/// `theme`: the schema's `anyOf: string | $defs/themeHint`, serialized
/// untagged — a bare string for [`ChromeTheme::Scheme`], an object for
/// [`ChromeTheme::Hint`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ChromeTheme {
    /// A bare colour scheme (`light` | `dark` | anything else = auto).
    Scheme(String),
    /// The theme hint a record declares (`$defs/themeHint`).
    Hint(ThemeHint),
}

impl From<&str> for ChromeTheme {
    fn from(scheme: &str) -> Self {
        Self::Scheme(scheme.to_string())
    }
}

impl From<String> for ChromeTheme {
    fn from(scheme: String) -> Self {
        Self::Scheme(scheme)
    }
}

impl From<ThemeHint> for ChromeTheme {
    fn from(hint: ThemeHint) -> Self {
        Self::Hint(hint)
    }
}

/// `$defs/themeHint`. Every member optional; absent members are omitted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeHint {
    /// `light` | `dark` | `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
    /// The page's theme-color, as declared by the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,
    /// Chrome palette tokens for the scheme named by `color_scheme`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<std::collections::BTreeMap<String, String>>,
}

impl ThemeHint {
    /// `true` when the hint names nothing (nothing to emit).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.color_scheme.is_none() && self.theme_color.is_none() && self.tokens.is_none()
    }
}

/// Who is looking (`viewer.kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewerKind {
    /// The person's own key is on this device.
    DeviceAgent,
    /// A validated session on a doorway that hosts their agent.
    HostedSession,
    /// No validated identity.
    Visitor,
}

/// `viewer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Viewer {
    pub kind: ViewerKind,
}

/// Which medium mounted the chrome (`medium`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChromeMedium {
    Web2Doorway,
    PeerNative,
    Desktop,
}

/// `content.mode`: data, never inferred from bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustMode {
    Trustful,
    Trustless,
}

/// `content.witness`: green once a conductor other than the record's author
/// holds the record as valid; amber until then (the serving peer derives it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WitnessState {
    Amber,
    Green,
}

/// `content.isolation.boundary`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IsolationBoundary {
    OriginPair,
    SchemePerRecord,
    SeparateWebview,
    OpaqueSandbox,
    None,
}

/// `content.delivery.path[].transport`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryTransport {
    Iroh,
    Libp2p,
    Http,
}

/// Which party in the path reported a hop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HopReporter {
    #[serde(rename = "self")]
    SelfPeer,
    Holder,
    Doorway,
}

/// `content.delivery.checked.by`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckedBy {
    Peer,
    Worker,
}

/// The holders reading (`content.holders`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HolderReading {
    pub holders: u32,
    pub target: u32,
    /// The fold's own sentence (`FeltStatusView.headline`), when it came with one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headline: Option<String>,
    /// RFC 3339 time the reading was taken.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_of: Option<String>,
}

/// `content.frame`: what the chrome builds the iframe from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContentFrame {
    pub src: String,
    pub title: String,
}

/// `content.isolation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Isolation {
    pub boundary: IsolationBoundary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_origin: Option<String>,
}

/// One hop of `content.delivery.path`, tagged on `role`. A peer is named by
/// its key only; labels come from the person's own roster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum DeliveryHop {
    #[serde(rename = "self")]
    SelfPeer {
        #[serde(rename = "reportedBy")]
        reported_by: HopReporter,
    },
    Holder {
        peer: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        transport: Option<DeliveryTransport>,
        #[serde(rename = "reportedBy")]
        reported_by: HopReporter,
    },
    Doorway {
        name: String,
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        did: Option<String>,
        #[serde(rename = "reportedBy")]
        reported_by: HopReporter,
    },
}

/// `content.delivery.checked`: present only when the verifier reported it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeliveryCheck {
    pub by: CheckedBy,
    pub against: String,
}

/// `content.delivery`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContentDelivery {
    /// Ordered from the person outward; at least one hop.
    pub path: Vec<DeliveryHop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<DeliveryCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
}

/// `content`: present only on a content shell. `capabilities` are frame
/// capability ids (`frame-capability.schema.json`), validated by the
/// schema-contract test rather than by a dependency on lamad-types.
/// `holders` serializes as `null` when no reading was available (the key is
/// required: zero holders is `holders: 0`, never null).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentIsland {
    pub record_id: String,
    pub content_address: String,
    pub format: String,
    pub entry: String,
    pub mode: TrustMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach: Option<String>,
    pub witness: WitnessState,
    pub capabilities: Vec<String>,
    pub holders: Option<HolderReading>,
    pub frame: ContentFrame,
    pub isolation: Isolation,
    pub delivery: ContentDelivery,
}

/// The chrome's receipt affordance (`ctx.receipt`). The element reads `.href`
/// (through its own `safeHref` allowlist), `.sentence` and `.label`.
///
/// The SENTENCE is what a person reads; the HREF is the protocol form, one
/// request away. Putting the precise form behind a link rather than in the
/// island is the whole "never the first thing said" discipline — and it keeps
/// the shell serve free of a ledger read it would otherwise have to make on the
/// hot path.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptLink {
    /// Where the full receipt is read from — relative, so it resolves against
    /// whichever doorway framed the page.
    pub href: String,
    /// The words a friend would use, shown on hover / read by a screen reader.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentence: Option<String>,
    /// The link text. Absent lets the element use its own default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl ChromeContext {
    /// A v1 context for `slug`: sets `version`, `viewer`, `medium`, and derives
    /// the deprecated `authenticated` from the viewer so the pre-v1 element
    /// keeps working for one release.
    #[must_use]
    pub fn v1(slug: impl Into<String>, viewer: ViewerKind, medium: ChromeMedium) -> Self {
        Self {
            slug: slug.into(),
            authenticated: !matches!(viewer, ViewerKind::Visitor),
            version: Some(CHROME_CONTEXT_VERSION),
            viewer: Some(Viewer { kind: viewer }),
            medium: Some(medium),
            ..Self::default()
        }
    }

    /// Serialize to the wire JSON string the inline context island carries
    /// (the value `inject_element` escapes and splices into the
    /// `<script type="application/json" id="elohim-omni-context">` island).
    ///
    /// Infallible in practice (every field is a `String`/`bool`/`Option` of
    /// those — never a float or non-string map key), but we fall back to
    /// `"{}"` defensively rather than panic if serialization ever did fail.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_context_is_byte_stable_with_todays_shape() {
        let ctx = ChromeContext {
            slug: "abc".to_string(),
            authenticated: true,
            ..Default::default()
        };
        assert_eq!(ctx.to_json(), r#"{"slug":"abc","authenticated":true}"#);
    }

    #[test]
    fn optional_fields_round_trip_when_present() {
        let ctx = ChromeContext {
            slug: "abc".to_string(),
            authenticated: false,
            title: Some("Title".to_string()),
            theme: Some("dark".into()),
            build_marker: Some("deadbee".to_string()),
            env_tier: Some("staging".to_string()),
            show_env: Some(true),
            account_href: Some("/account".to_string()),
            show_theme_toggle: Some(false),
            nav_back: Some(NavLink {
                href: "/back".to_string(),
                label: Some("Back".to_string()),
            }),
            nav_forward: Some(NavLink {
                href: "/forward".to_string(),
                label: None,
            }),
            receipt: Some(ReceiptLink {
                href: "/api/v1/receipt/community-garden-club".to_string(),
                sentence: Some("Someone kept this ready for you.".to_string()),
                label: None,
            }),
            ..Default::default()
        };
        let v: serde_json::Value = serde_json::from_str(&ctx.to_json()).unwrap();
        assert_eq!(v["slug"], "abc");
        assert_eq!(v["authenticated"], false);
        assert_eq!(v["title"], "Title");
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["buildMarker"], "deadbee");
        assert_eq!(v["envTier"], "staging");
        assert_eq!(v["showEnv"], true);
        assert_eq!(v["accountHref"], "/account");
        assert_eq!(v["showThemeToggle"], false);
        assert_eq!(v["navBack"]["href"], "/back");
        assert_eq!(v["navBack"]["label"], "Back");
        assert_eq!(v["navForward"]["href"], "/forward");
        assert!(v["navForward"].get("label").is_none());
        assert_eq!(
            v["receipt"]["href"],
            "/api/v1/receipt/community-garden-club"
        );
        assert_eq!(v["receipt"]["sentence"], "Someone kept this ready for you.");
        assert!(
            v["receipt"].get("label").is_none(),
            "an absent label lets the element use its own default rather than an empty one"
        );
    }

    #[test]
    fn default_context_serializes_to_minimal_island() {
        let ctx = ChromeContext::default();
        assert_eq!(ctx.to_json(), r#"{"slug":"","authenticated":false}"#);
    }

    #[test]
    fn theme_serializes_as_a_string_or_a_hint_object() {
        let scheme = ChromeContext {
            theme: Some("dark".into()),
            ..Default::default()
        };
        let v: serde_json::Value = serde_json::from_str(&scheme.to_json()).unwrap();
        assert_eq!(v["theme"], "dark");

        let hint = ChromeContext {
            theme: Some(
                ThemeHint {
                    color_scheme: Some("dark".into()),
                    theme_color: Some("#112233".into()),
                    tokens: None,
                }
                .into(),
            ),
            ..Default::default()
        };
        let v: serde_json::Value = serde_json::from_str(&hint.to_json()).unwrap();
        assert_eq!(
            v["theme"],
            serde_json::json!({ "colorScheme": "dark", "themeColor": "#112233" })
        );
        assert!(ThemeHint::default().is_empty());
    }
}
