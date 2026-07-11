//! `ChromeContext` — the PRODUCER side of the omni element's `ctx` contract.
//!
//! The element (`omni-element.js`, the CONSUMER) reads up to 11 camelCase
//! fields off the inline JSON context island via `ctx.<field>` accessors in
//! `mount(ctx)` / `buildMarkup(ctx)` / `contextFromContentNode(slug, node)`:
//! `slug`, `title`, `theme`, `buildMarker`, `envTier`, `showEnv`,
//! `authenticated`, `accountHref`, `showThemeToggle`, `navBack`, `navForward`.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
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
}

impl ChromeContext {
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
            theme: Some("dark".to_string()),
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
    }

    #[test]
    fn default_context_serializes_to_minimal_island() {
        let ctx = ChromeContext::default();
        assert_eq!(ctx.to_json(), r#"{"slug":"","authenticated":false}"#);
    }
}
