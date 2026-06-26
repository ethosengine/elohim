//! Native omnibar chrome renderer — markup + themed `<style>`, in Rust.
//!
//! A faithful port of the Angular `app-protocol-omni` component
//! (`app/elohim-app/src/app/elohim/components/protocol-omni/`) — the rich MAIN
//! landing omnibar — into native runtime chrome. (The earlier port mistakenly
//! mirrored the narrow `/deliver`-route `protocol-omnibar` sibling; this is the
//! re-port from the canonical `protocol-omni`.) The omnibar leaves the per-EPR
//! Angular bundle and becomes EPR-agnostic chrome the runtime composes (splices)
//! around the V8-rendered Angular body (Task 3). This module owns the two halves
//! of that chrome:
//!
//! - [`render_omnibar_style`] — the scoped `<style>` binding the seven
//!   `--omni-*` custom properties from the EPR's [`Theme`] tokens (or
//!   [`base_palette`] when absent), and the color-scheme behavior: `Auto`
//!   emits a light `:root` default + a `@media (prefers-color-scheme: dark)`
//!   override; `Light`/`Dark` pin one set. The client theme toggle
//!   (`html[data-theme=…]`) still wins via the `[data-theme]` selector hooks.
//! - [`render_omnibar_markup`] — the collapsed chip (the `elohim-protocol`
//!   pill) + the expanded toolbar, with server-known values (truncated EPR id,
//!   build marker, env tier, nav targets) rendered inline and behavior
//!   expressed as `data-omni-action` / `data-omni-resilience-slug` hooks that
//!   `omni-enhance.js` (Task 5) wires. The markup references only
//!   `var(--omni-*, <fallback>)` — never raw color.
//!
//! Affordances ported from `protocol-omni` (vs the previous narrow port): the
//! `elohim-protocol` collapsed pill label, the env-context badge (tier + short
//! build id, rendered when a build marker is present), the back/forward nav
//! links (plain `<a href>`, progressive enhancement), the account link, the
//! theme toggle, the resilience glyph, and the EPR id chip + copy.
//!
//! Every interpolated value is HTML-escaped ([`html_escape`]) — a server-side
//! splice of attacker-influenceable content (title/description/slug/marker/tier
//! /nav labels) is the one new XSS surface this layer introduces.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md` §4.1, §4.4
//! Plan: `genesis/docs/superpowers/plans/2026-06-26-native-chrome-omnibar-plan.md` (Task 2)

use super::theme::{base_palette, ColorScheme, Theme, ThemeTokens};

/// A back/forward nav target the runtime can paint as a plain `<a href>`.
///
/// `protocol-omni` gates its nav buttons on `ProtocolNavigationService` (a
/// client-side history primitive). Server-side we can only paint a nav
/// affordance when the composition layer (Task 3) supplies a concrete target;
/// otherwise the slot is omitted and `omni-enhance.js` (or a future client
/// router) may add it. We deliberately do NOT invent a client router here.
#[derive(Debug, Clone)]
pub struct NavTarget<'a> {
    /// Destination href (e.g. `/resource/<cid>`). Escaped before splicing.
    pub href: &'a str,
    /// Human label (e.g. the prior EPR title). Escaped before splicing.
    pub label: &'a str,
}

/// Server-known inputs to the native omnibar chrome.
///
/// These are the values the runtime can paint without JS: the EPR slug (→ the
/// truncated address chip), the served build marker, the serving-environment
/// tier, optional nav targets, and the EPR's declared theme. Dynamic detail
/// (the live resilience snapshot) stays client-enhanced in `omni-enhance.js`
/// (Task 5) — the markup emits a neutral glyph + a `data-omni-resilience-slug`
/// hook, not data, for that.
///
/// Borrowed `&str` fields: the caller (the composition layer, Task 3) builds
/// this transiently per request from the fetched content node; nothing here
/// outlives the render call. New fields carry sensible defaults via
/// [`ChromeInput::for_slug`] so Task 3 can populate what it can and leave the
/// rest off.
#[derive(Debug, Clone)]
pub struct ChromeInput<'a> {
    /// The EPR slug / content address — rendered as the truncated id chip.
    pub slug: &'a str,
    /// EPR title (used for the chip's accessible label / tooltip).
    pub title: &'a str,
    /// EPR description (reserved for `<head>` composition; escaped here for
    /// any inline use).
    pub description: &'a str,
    /// The served build marker (e.g. the short blob hash). Absent ⇒ no marker
    /// rendered (the chrome stays valid without it). Mirrors `protocol-omni`'s
    /// `shortBuildId` — only the first 7 chars are shown in the env badge.
    pub build_marker: Option<&'a str>,
    /// The EPR-declared theme. Absent ⇒ the [`base_palette`] fallback.
    pub theme: Option<Theme>,
    /// Render the env-context badge (tier + short build id). Mirrors
    /// `protocol-omni`'s `showEnvContext` opt-in (the trust surface never
    /// cries wolf). The badge is only painted when this is `true` AND a
    /// non-production tier is supplied — see [`ChromeInput::env_visible`].
    pub show_env: bool,
    /// The serving-environment tier (e.g. `dev`, `alpha`). `production` (or
    /// absent) suppresses the env badge even when `show_env` is set, matching
    /// `protocol-omni`'s `envVisible` (tier present && tier !== 'production').
    pub env_tier: Option<&'a str>,
    /// Whether a household/account session is active. When `true` the account
    /// link is rendered live; when `false` it is still server-rendered but
    /// carries `hidden` + `data-omni-account` so JS may reveal it (progressive
    /// enhancement — the server cannot always know the auth state).
    pub authenticated: bool,
    /// Where the account link points. Mirrors `protocol-omni`'s `accountHref`
    /// (default `/account`).
    pub account_href: &'a str,
    /// Render the theme-toggle button. Mirrors `protocol-omni`'s
    /// `showThemeToggle` opt-in.
    pub show_theme_toggle: bool,
    /// In-network back target, if the composition layer knows one. `None` ⇒ no
    /// back affordance (no invented client router).
    pub nav_back: Option<NavTarget<'a>>,
    /// In-network forward target, if known. `None` ⇒ no forward affordance.
    pub nav_forward: Option<NavTarget<'a>>,
}

impl<'a> ChromeInput<'a> {
    /// Construct a minimal input for a slug with all the rich affordances at
    /// their sensible defaults (env hidden, unauthenticated, `/account`, theme
    /// toggle shown, no nav). Task 3 mutates the fields it can populate.
    #[must_use]
    pub fn for_slug(slug: &'a str) -> Self {
        Self {
            slug,
            title: "",
            description: "",
            build_marker: None,
            theme: None,
            show_env: false,
            env_tier: None,
            authenticated: false,
            account_href: "/account",
            show_theme_toggle: true,
            nav_back: None,
            nav_forward: None,
        }
    }

    /// The resolved color scheme: the theme's declared scheme, or `Auto` when
    /// no theme is supplied (emit both light + dark, let the OS pick).
    #[must_use]
    pub fn color_scheme(&self) -> ColorScheme {
        self.theme
            .as_ref()
            .map_or(ColorScheme::Auto, |t| t.color_scheme)
    }

    /// Whether the env-context badge should paint. Mirrors `protocol-omni`'s
    /// `envVisible`: opted in (`show_env`) AND a non-`production` tier present.
    #[must_use]
    pub fn env_visible(&self) -> bool {
        self.show_env
            && self
                .env_tier
                .is_some_and(|t| !t.is_empty() && t != "production")
    }

    /// The short build id for the env badge — the first 7 chars of the build
    /// marker, mirroring `protocol-omni`'s `shortBuildId`.
    fn short_build_id(&self) -> Option<String> {
        self.build_marker
            .map(|m| m.chars().take(7).collect::<String>())
    }
}

/// HTML-escape the five significant characters for text and double-quoted
/// attribute contexts: `&`, `<`, `>`, `"`, `'`.
///
/// `&` is escaped first so the entity ampersands the function itself emits are
/// not double-escaped. This is the XSS guard for every interpolated value the
/// chrome splices into the served document.
#[must_use]
pub fn html_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Truncate a content address the way `protocol-omni`'s `shortCid` getter does:
/// `<=20` chars passes through; longer becomes `head6…tail6` (joined by the
/// ASCII `...` so the markup stays pure-ASCII; the original used `…`).
#[must_use]
fn truncate_address(address: &str) -> String {
    let count = address.chars().count();
    if count <= 20 {
        return address.to_string();
    }
    let head: String = address.chars().take(6).collect();
    let tail: String = address.chars().skip(count - 6).collect();
    format!("{head}...{tail}")
}

/// Bind a single `--omni-*` declaration line from a token value.
fn var_line(name: &str, value: &str) -> String {
    format!("  --omni-{name}: {value};\n")
}

/// Emit the seven `--omni-*` custom-property declarations for one token set.
fn token_block(tokens: &ThemeTokens) -> String {
    let mut block = String::new();
    block.push_str(&var_line("bg", &tokens.bg));
    block.push_str(&var_line("fg", &tokens.fg));
    block.push_str(&var_line("muted", &tokens.muted));
    block.push_str(&var_line("border", &tokens.border));
    block.push_str(&var_line("accent", &tokens.accent));
    block.push_str(&var_line("shadow", &tokens.shadow));
    block.push_str(&var_line("env-ring", &tokens.env_ring));
    block
}

/// Resolve the (light, dark) token sets the style should bind.
///
/// When the EPR supplies a [`Theme`], its tokens occupy the slot matching the
/// theme's scheme and the [`base_palette`] supplies the *other* slot (so a
/// light-only EPR theme still gets a sensible dark override under `Auto`). When
/// no theme is supplied, both slots come from the base palette.
fn resolve_token_sets(input: &ChromeInput) -> (ThemeTokens, ThemeTokens) {
    let base = base_palette();
    match &input.theme {
        None => (base.light, base.dark),
        Some(theme) => match theme.color_scheme {
            // The EPR's tokens are its dark set; base supplies the light default.
            ColorScheme::Dark => (base.light, theme.tokens.clone()),
            // The EPR's tokens are its light set; base supplies the dark override.
            ColorScheme::Light | ColorScheme::Auto => (theme.tokens.clone(), base.dark),
        },
    }
}

/// Render the scoped `<style>` for the omnibar chrome.
///
/// Authority chain (mirrors `protocol-omni.component.css` + the
/// `html[data-theme]` contract):
/// 1. `:root` carries the light defaults (or the pinned set for explicit
///    `Light`/`Dark`).
/// 2. For `Auto`, `@media (prefers-color-scheme: dark) :root:not([data-theme])`
///    overrides with the dark set — the OS preference is the no-`data-theme`
///    fallback (the `:host-context([data-theme])` blocks in the Angular source
///    win over the media block; we mirror that with `:not([data-theme])` here
///    so the explicit override always beats the OS scheme).
/// 3. `:root[data-theme='light']` / `:root[data-theme='dark']` always win — the
///    client theme toggle persists `html[data-theme]` and these selectors let
///    it override both the default and the media query.
///
/// The static layout rules (positions, radii, the markup's structural CSS) are
/// emitted alongside, all painted through `var(--omni-*)`.
#[must_use]
pub fn render_omnibar_style(input: &ChromeInput) -> String {
    let (light, dark) = resolve_token_sets(input);
    let scheme = input.color_scheme();

    let mut style = String::new();
    style.push_str("<style id=\"elohim-omni-style\">\n");

    // (1) :root default — the light set for Auto/Light, the dark set for Dark.
    let root_tokens = match scheme {
        ColorScheme::Dark => &dark,
        ColorScheme::Light | ColorScheme::Auto => &light,
    };
    style.push_str(":root {\n");
    style.push_str(&token_block(root_tokens));
    style.push_str("}\n");

    // (2) Auto only: prefers-color-scheme dark override with the dark set.
    if scheme == ColorScheme::Auto {
        style.push_str("@media (prefers-color-scheme: dark) {\n");
        style.push_str("  :root:not([data-theme]) {\n");
        for line in token_block(&dark).lines() {
            style.push_str("  ");
            style.push_str(line);
            style.push('\n');
        }
        style.push_str("  }\n");
        style.push_str("}\n");
    }

    // (3) data-theme override hooks — the client toggle always wins.
    style.push_str(":root[data-theme='light'] {\n");
    style.push_str(&token_block(&light));
    style.push_str("}\n");
    style.push_str(":root[data-theme='dark'] {\n");
    style.push_str(&token_block(&dark));
    style.push_str("}\n");

    // Structural CSS — every paint goes through var(--omni-*).
    style.push_str(OMNI_LAYOUT_CSS);

    style.push_str("</style>");
    style
}

/// Structural / layout CSS for the omnibar markup. Ported from
/// `protocol-omni.component.css`; every color/shadow/border now reads a
/// `--omni-*` custom property (with the verbatim original value as the
/// `var()` fallback so the bar paints even if a token is unset).
///
/// Container contract: the renderer keeps the `#elohim-omni` /
/// `[data-omni-state]` show/hide hooks that `omni-enhance.js` toggles (the
/// Angular source used `*ngIf` on `expanded()`; server-side we render both the
/// collapsed pill group and the expanded toolbar and let `[data-omni-state]`
/// pick which is visible). The class vocabulary follows `protocol-omni`
/// (`.omni-chip`, `.omni-toolbar`, `.omni-epr`, `.omni-env`, …).
const OMNI_LAYOUT_CSS: &str = "\
#elohim-omni { position: fixed; inset: 0 0 auto 0; z-index: 2147483000; \
font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', \
Roboto, Helvetica, Arial, sans-serif; font-size: 12px; line-height: 1.2; \
pointer-events: none; }
#elohim-omni .omni-chip, #elohim-omni .omni-toolbar { pointer-events: auto; }
#elohim-omni .omni-pill-group { display: block; }
#elohim-omni .omni-chip { position: absolute; top: 0.45rem; left: 50%; \
transform: translateX(-50%); display: inline-flex; align-items: center; gap: 0.4rem; \
padding: 0.28rem 0.7rem; background: var(--omni-bg, rgba(255,255,255,0.92)); \
color: var(--omni-fg, rgba(20,22,30,0.96)); \
border: 1px solid var(--omni-border, rgba(20,22,30,0.14)); border-radius: 999px; \
box-shadow: var(--omni-shadow, 0 1px 6px rgba(0,0,0,0.08)); backdrop-filter: blur(8px); \
font: inherit; cursor: pointer; }
#elohim-omni .omni-chip-env { box-shadow: 0 0 0 2px var(--omni-env-ring, #d97706), \
var(--omni-shadow, 0 1px 6px rgba(0,0,0,0.08)); }
#elohim-omni .omni-glyph { font-size: 13px; line-height: 1; \
color: var(--omni-accent, rgba(20,22,30,0.96)); }
#elohim-omni .omni-label { color: var(--omni-fg, rgba(20,22,30,0.96)); }
#elohim-omni .omni-toolbar { position: absolute; top: 0; left: 0; right: 0; \
display: none; align-items: center; gap: 0.75rem; padding: 0.5rem 1rem; \
background: var(--omni-bg, rgba(255,255,255,0.92)); \
color: var(--omni-fg, rgba(20,22,30,0.96)); \
border-bottom: 1px solid var(--omni-border, rgba(20,22,30,0.14)); \
box-shadow: var(--omni-shadow, 0 1px 6px rgba(0,0,0,0.08)); backdrop-filter: blur(10px); }
#elohim-omni[data-omni-state='expanded'] .omni-toolbar { display: flex; }
#elohim-omni[data-omni-state='expanded'] .omni-pill-group { display: none; }
#elohim-omni .omni-toolbar button, #elohim-omni .omni-toolbar a { \
background: transparent; border: 1px solid var(--omni-border, rgba(20,22,30,0.14)); \
border-radius: 6px; padding: 0.25rem 0.6rem; font: inherit; \
color: var(--omni-fg, rgba(20,22,30,0.96)); text-decoration: none; cursor: pointer; }
#elohim-omni .omni-toolbar button:hover, #elohim-omni .omni-toolbar a:hover { \
border-color: var(--omni-accent, rgba(20,22,30,0.96)); }
#elohim-omni .omni-epr-group { display: inline-flex; align-items: center; gap: 0.35rem; }
#elohim-omni .omni-epr { display: inline-flex; align-items: center; gap: 0.5rem; }
#elohim-omni .omni-epr-label { color: var(--omni-muted, rgba(20,22,30,0.55)); }
#elohim-omni .omni-epr-value { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
#elohim-omni .omni-env { display: inline-flex; align-items: center; gap: 0.35rem; \
border: 1px solid var(--omni-env-ring, #d97706); }
#elohim-omni .omni-env-tier { text-transform: uppercase; font-weight: 700; \
font-size: 10px; letter-spacing: 0.04em; }
#elohim-omni .omni-env-build { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
#elohim-omni .omni-resilience { display: inline-flex; align-items: center; \
color: var(--omni-fg, rgba(20,22,30,0.96)); padding: 0 0.25rem; }
#elohim-omni .omni-resilience-neutral { font-size: 12px; line-height: 1; }
#elohim-omni .omni-collapse { margin-left: auto; }
";

/// Render the omnibar markup: the collapsed `elohim-protocol` chip (default) +
/// the expanded toolbar.
///
/// Server-known values are rendered inline and HTML-escaped; behavior is
/// expressed as `data-omni-action` hooks (`toggle` / `copy` / `theme-toggle`)
/// and a `data-omni-resilience-slug` hook for the lazy resilience panel
/// (`omni-enhance.js` populates the `data-omni-resilience-glyph` marks on first
/// expand — here it is a neutral glyph that makes no status claim). The
/// container starts in the `pill` state; the script flips `data-omni-state` to
/// `expanded`.
///
/// `<a href>` affordances (env link, back/forward nav, account) are plain links
/// that work with zero JS — progressive enhancement. Nav links render only when
/// the composition layer supplies a target; no client router is invented.
#[must_use]
pub fn render_omnibar_markup(input: &ChromeInput) -> String {
    let slug_attr = html_escape(input.slug);
    let title_attr = html_escape(input.title);
    let truncated = html_escape(&truncate_address(input.slug));
    let env_visible = input.env_visible();

    let mut html = String::new();
    html.push_str(&format!(
        "<div id=\"elohim-omni\" data-omni-state=\"pill\" \
data-omni-resilience-slug=\"{slug_attr}\">\n"
    ));

    // === COLLAPSED CHIP (default): the `elohim-protocol` pill ===
    html.push_str("  <div class=\"omni-pill-group\">\n");
    // .omni-chip-env mirrors protocol-omni's [class.omni-chip-env]="envVisible()".
    let chip_class = if env_visible {
        "omni-chip omni-chip-env"
    } else {
        "omni-chip"
    };
    html.push_str(&format!(
        "    <button type=\"button\" class=\"{chip_class}\" data-omni-action=\"toggle\" \
aria-label=\"Elohim Protocol \u{2014} click for details\">\n"
    ));
    html.push_str("      <span class=\"omni-glyph\" aria-hidden=\"true\">\u{2BC2}</span>\n");
    html.push_str("      <span class=\"omni-label\">elohim-protocol</span>\n");
    html.push_str("    </button>\n");
    html.push_str("  </div>\n");

    // === EXPANDED TOOLBAR ===
    html.push_str(
        "  <div class=\"omni-toolbar\" role=\"region\" aria-label=\"Protocol context\">\n",
    );

    // Back nav (server-known target only; plain link, no client router).
    if let Some(back) = &input.nav_back {
        let href = html_escape(back.href);
        let label = html_escape(back.label);
        html.push_str(&format!(
            "    <a href=\"{href}\" class=\"omni-nav omni-nav-back\" \
title=\"{label}\" aria-label=\"{label}\">\u{2190} {label}</a>\n"
        ));
    }

    // EPR group: copy-CID button + (optional) env-context badge.
    html.push_str("    <span class=\"omni-epr-group\">\n");
    html.push_str(&format!(
        "      <button type=\"button\" class=\"omni-epr\" data-omni-action=\"copy\" \
data-omni-copy-value=\"{slug_attr}\" title=\"Copy content identifier for {title_attr}\" \
aria-label=\"Copy content identifier\">\n"
    ));
    html.push_str("        <span class=\"omni-epr-label\">EPR</span>\n");
    html.push_str(&format!(
        "        <code class=\"omni-epr-value\">{truncated}</code>\n"
    ));
    html.push_str("      </button>\n");

    // Env-context badge — tier + short build id. Mirrors protocol-omni's
    // .omni-env link (gated on envVisible()).
    if env_visible {
        let tier = html_escape(input.env_tier.unwrap_or_default());
        html.push_str(&format!(
            "      <a href=\"/doorway/elohim\" class=\"omni-env\" \
title=\"Serving environment: {tier}\" aria-label=\"Serving environment: {tier}\">\n"
        ));
        html.push_str(&format!(
            "        <span class=\"omni-env-tier\">{tier}</span>\n"
        ));
        if let Some(build) = input.short_build_id() {
            let build = html_escape(&build);
            html.push_str(&format!(
                "        <code class=\"omni-env-build\">{build}</code>\n"
            ));
        }
        html.push_str("      </a>\n");
    }
    html.push_str("    </span>\n");

    // Resilience indicator — neutral glyph; JS swaps it on first expand via the
    // data-omni-resilience-glyph marks (keyed off data-omni-resilience-slug).
    html.push_str("    <span class=\"omni-resilience\" aria-label=\"Resilience indicator\">\n");
    html.push_str(
        "      <span class=\"omni-resilience-neutral\" data-omni-resilience-glyph \
title=\"Resilience snapshot unavailable\">\u{25C9}</span>\n",
    );
    html.push_str("    </span>\n");

    // Forward nav (server-known target only).
    if let Some(forward) = &input.nav_forward {
        let href = html_escape(forward.href);
        let label = html_escape(forward.label);
        html.push_str(&format!(
            "    <a href=\"{href}\" class=\"omni-nav omni-nav-forward\" \
title=\"{label}\" aria-label=\"{label}\">{label} \u{2192}</a>\n"
        ));
    }

    // Account link — always server-rendered; hidden (with the data-omni-account
    // hook) when the server doesn't know the session is authenticated, so JS may
    // reveal it. Mirrors protocol-omni's *ngIf="authenticated()".
    let account_href = html_escape(input.account_href);
    let hidden_attr = if input.authenticated { "" } else { " hidden" };
    html.push_str(&format!(
        "    <a href=\"{account_href}\" class=\"omni-account\" data-omni-account \
aria-label=\"Account\"{hidden_attr}>\u{25D0}</a>\n"
    ));

    // Theme toggle (opt-in) — wired by omni-enhance.js.
    if input.show_theme_toggle {
        html.push_str(
            "    <button type=\"button\" class=\"omni-theme\" \
data-omni-action=\"theme-toggle\" aria-label=\"Toggle theme\">\u{25D1}</button>\n",
        );
    }

    // Build marker (server-known, inline) — only when present AND not already
    // shown in the env badge (keeps the toolbar from repeating the build id).
    if let Some(marker) = input.build_marker {
        if !env_visible {
            let marker_text = html_escape(marker);
            html.push_str(&format!(
                "    <span class=\"omni-marker\">via {marker_text}</span>\n"
            ));
        }
    }

    // Collapse — flips data-omni-state back to pill via the toggle action.
    html.push_str(
        "    <button type=\"button\" class=\"omni-collapse\" data-omni-action=\"toggle\" \
aria-label=\"Collapse\">\u{00D7}</button>\n",
    );

    html.push_str("  </div>\n");
    html.push_str("</div>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chrome::theme::{ColorScheme, Theme, ThemeTokens};

    fn sample_theme(scheme: ColorScheme) -> Theme {
        Theme {
            color_scheme: scheme,
            tokens: ThemeTokens {
                bg: "rgba(1, 2, 3, 0.9)".to_string(),
                fg: "rgba(4, 5, 6, 0.9)".to_string(),
                muted: "rgba(7, 8, 9, 0.5)".to_string(),
                border: "rgba(10, 11, 12, 0.2)".to_string(),
                accent: "rgba(13, 14, 15, 0.9)".to_string(),
                shadow: "0 2px 9px rgba(0, 0, 0, 0.5)".to_string(),
                env_ring: "#abcdef".to_string(),
            },
        }
    }

    fn input_with(theme: Option<Theme>, marker: Option<&'static str>) -> ChromeInput<'static> {
        let mut input = ChromeInput::for_slug("elohim-host-landing-0123456789abcdef");
        input.title = "Welcome";
        input.description = "The landing EPR";
        input.build_marker = marker;
        input.theme = theme;
        input
    }

    // ---- html_escape ----

    #[test]
    fn html_escape_handles_all_five_significant_chars() {
        assert_eq!(
            html_escape(r#"<a href="x">&'</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#x27;&lt;/a&gt;"
        );
    }

    #[test]
    fn html_escape_does_not_double_escape_ampersand() {
        // & must become &amp; — not &amp;amp; — i.e. ampersand is handled first.
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("&lt;"), "&amp;lt;");
    }

    // ---- truncate_address (protocol-omni shortCid: <=20 passthrough) ----

    #[test]
    fn truncate_address_passthrough_short() {
        assert_eq!(truncate_address("short"), "short");
        // exactly 20 chars passes through.
        assert_eq!(
            truncate_address("01234567890123456789"),
            "01234567890123456789"
        );
    }

    #[test]
    fn truncate_address_truncates_long() {
        // 21+ chars → head6...tail6.
        assert_eq!(
            truncate_address("0123456789abcdefghijklmnop"),
            "012345...klmnop"
        );
    }

    // ---- env_visible / short_build_id ----

    #[test]
    fn env_visible_requires_optin_and_non_production_tier() {
        let mut input = input_with(None, Some("a1b2c3d4e5"));
        // Default: not opted in.
        assert!(!input.env_visible());
        // Opted in but no tier.
        input.show_env = true;
        assert!(!input.env_visible());
        // Opted in + production → suppressed.
        input.env_tier = Some("production");
        assert!(!input.env_visible());
        // Opted in + non-production → visible.
        input.env_tier = Some("alpha");
        assert!(input.env_visible());
    }

    #[test]
    fn short_build_id_takes_first_seven() {
        let input = input_with(None, Some("a1b2c3d4e5f6"));
        assert_eq!(input.short_build_id().as_deref(), Some("a1b2c3d"));
        let no_marker = input_with(None, None);
        assert_eq!(no_marker.short_build_id(), None);
    }

    // ---- style: theme present / absent ----

    #[test]
    fn style_theme_present_binds_theme_tokens() {
        let input = input_with(Some(sample_theme(ColorScheme::Light)), None);
        let css = render_omnibar_style(&input);
        // The EPR's light tokens are bound on :root.
        assert!(css.contains("--omni-bg: rgba(1, 2, 3, 0.9);"), "css: {css}");
        assert!(css.contains("--omni-env-ring: #abcdef;"), "css: {css}");
        // Dark override slot falls back to base_palette().dark.
        assert!(
            css.contains("--omni-bg: rgba(22, 23, 28, 0.92);"),
            "css: {css}"
        );
    }

    #[test]
    fn style_theme_absent_uses_base_palette() {
        let input = input_with(None, None);
        let css = render_omnibar_style(&input);
        // base_palette light on :root.
        assert!(
            css.contains("--omni-bg: rgba(255, 255, 255, 0.92);"),
            "css: {css}"
        );
        // base_palette dark in the data-theme='dark' + media block.
        assert!(
            css.contains("--omni-bg: rgba(22, 23, 28, 0.92);"),
            "css: {css}"
        );
    }

    // ---- style: colorScheme light / dark / auto ----

    #[test]
    fn style_auto_emits_both_default_and_media_dark() {
        let input = input_with(None, None);
        let css = render_omnibar_style(&input);
        assert!(
            css.contains("@media (prefers-color-scheme: dark)"),
            "auto must emit a prefers-color-scheme block: {css}"
        );
        // :root carries the light default for auto.
        assert!(css.contains(":root {\n  --omni-bg: rgba(255, 255, 255, 0.92);"));
        // The media block uses the dark set, scoped to :root:not([data-theme]).
        assert!(css.contains(":root:not([data-theme])"), "css: {css}");
    }

    #[test]
    fn style_explicit_light_pins_light_no_media_block() {
        let input = input_with(Some(sample_theme(ColorScheme::Light)), None);
        let css = render_omnibar_style(&input);
        assert!(
            !css.contains("@media (prefers-color-scheme: dark)"),
            "explicit light must NOT emit a media block: {css}"
        );
        // :root pinned to the theme's (light) tokens.
        assert!(
            css.contains(":root {\n  --omni-bg: rgba(1, 2, 3, 0.9);"),
            "css: {css}"
        );
    }

    #[test]
    fn style_explicit_dark_pins_dark_no_media_block() {
        let input = input_with(Some(sample_theme(ColorScheme::Dark)), None);
        let css = render_omnibar_style(&input);
        assert!(
            !css.contains("@media (prefers-color-scheme: dark)"),
            "explicit dark must NOT emit a media block: {css}"
        );
        // :root pinned to the theme's (dark) tokens.
        assert!(
            css.contains(":root {\n  --omni-bg: rgba(1, 2, 3, 0.9);"),
            "css: {css}"
        );
    }

    #[test]
    fn style_always_emits_data_theme_override_hooks() {
        // The client theme toggle (html[data-theme]) must always win.
        for scheme in [ColorScheme::Auto, ColorScheme::Light, ColorScheme::Dark] {
            let input = input_with(Some(sample_theme(scheme)), None);
            let css = render_omnibar_style(&input);
            assert!(
                css.contains(":root[data-theme='light']"),
                "missing light override hook ({scheme:?}): {css}"
            );
            assert!(
                css.contains(":root[data-theme='dark']"),
                "missing dark override hook ({scheme:?}): {css}"
            );
        }
    }

    // ---- markup: the collapsed `elohim-protocol` pill ----

    #[test]
    fn markup_renders_the_collapsed_elohim_protocol_pill() {
        let input = input_with(None, None);
        let html = render_omnibar_markup(&input);
        assert!(html.contains("class=\"omni-pill-group\""), "html: {html}");
        // The pill label is the rich protocol-omni signature.
        assert!(
            html.contains("<span class=\"omni-label\">elohim-protocol</span>"),
            "html: {html}"
        );
        // The chip glyph (⯂) is present.
        assert!(html.contains("\u{2BC2}"), "html: {html}");
        // Starts collapsed.
        assert!(html.contains("data-omni-state=\"pill\""), "html: {html}");
    }

    // ---- markup: env badge present/absent (the new affordance) ----

    #[test]
    fn markup_renders_env_badge_when_visible() {
        let mut input = input_with(None, Some("a1b2c3d4e5"));
        input.show_env = true;
        input.env_tier = Some("alpha");
        let html = render_omnibar_markup(&input);
        // The env badge link + tier + short build id all present.
        assert!(html.contains("class=\"omni-env\""), "html: {html}");
        assert!(
            html.contains("<span class=\"omni-env-tier\">alpha</span>"),
            "html: {html}"
        );
        assert!(
            html.contains("<code class=\"omni-env-build\">a1b2c3d</code>"),
            "html: {html}"
        );
        // The chip carries the env ring class when env is visible.
        assert!(html.contains("omni-chip-env"), "html: {html}");
    }

    #[test]
    fn markup_omits_env_badge_when_not_opted_in() {
        let input = input_with(None, Some("a1b2c3d4e5"));
        let html = render_omnibar_markup(&input);
        assert!(
            !html.contains("class=\"omni-env\""),
            "env badge must be absent without opt-in: {html}"
        );
        assert!(
            !html.contains("omni-chip-env"),
            "chip env-ring must be absent without env visibility: {html}"
        );
        // Without the env badge, the build marker shows inline as a fallback.
        assert!(html.contains("via a1b2c3d4e5"), "html: {html}");
    }

    // ---- markup: resilience glyph + slug hook (the JS contract) ----

    #[test]
    fn markup_renders_resilience_glyph_with_slug_hook() {
        let input = input_with(None, None);
        let html = render_omnibar_markup(&input);
        // The neutral resilience glyph carries the marker the JS swaps.
        assert!(html.contains("data-omni-resilience-glyph"), "html: {html}");
        assert!(
            html.contains("\u{25C9}"),
            "neutral resilience glyph: {html}"
        );
        // The container exposes the slug the JS keys the lazy fetch off.
        assert!(
            html.contains("data-omni-resilience-slug=\"elohim-host-landing-0123456789abcdef\""),
            "html: {html}"
        );
    }

    // ---- markup: theme-toggle + account affordances (new) ----

    #[test]
    fn markup_renders_theme_toggle_and_account() {
        let input = input_with(None, None);
        let html = render_omnibar_markup(&input);
        // Theme toggle present (opt-in defaults on via for_slug).
        assert!(
            html.contains("data-omni-action=\"theme-toggle\""),
            "html: {html}"
        );
        // Account link present; hidden when unauthenticated, with the JS hook.
        assert!(html.contains("class=\"omni-account\""), "html: {html}");
        assert!(html.contains("data-omni-account"), "html: {html}");
        assert!(
            html.contains("href=\"/account\""),
            "default account href: {html}"
        );
        assert!(
            html.contains("data-omni-account aria-label=\"Account\" hidden"),
            "account must be hidden when unauthenticated: {html}"
        );
    }

    #[test]
    fn markup_account_visible_when_authenticated() {
        let mut input = input_with(None, None);
        input.authenticated = true;
        input.account_href = "/me";
        let html = render_omnibar_markup(&input);
        assert!(html.contains("href=\"/me\""), "html: {html}");
        // No `hidden` attribute on the account link when authenticated.
        assert!(
            !html.contains("aria-label=\"Account\" hidden"),
            "authenticated account must not be hidden: {html}"
        );
    }

    #[test]
    fn markup_omits_theme_toggle_when_opted_out() {
        let mut input = input_with(None, None);
        input.show_theme_toggle = false;
        let html = render_omnibar_markup(&input);
        assert!(
            !html.contains("data-omni-action=\"theme-toggle\""),
            "theme toggle must be absent when opted out: {html}"
        );
    }

    // ---- markup: back/forward nav (new; server-known targets only) ----

    #[test]
    fn markup_renders_nav_links_when_targets_present() {
        let mut input = input_with(None, None);
        input.nav_back = Some(NavTarget {
            href: "/resource/back-cid",
            label: "Prior EPR",
        });
        input.nav_forward = Some(NavTarget {
            href: "/resource/fwd-cid",
            label: "Next EPR",
        });
        let html = render_omnibar_markup(&input);
        assert!(html.contains("href=\"/resource/back-cid\""), "html: {html}");
        assert!(html.contains("omni-nav-back"), "html: {html}");
        assert!(html.contains("href=\"/resource/fwd-cid\""), "html: {html}");
        assert!(html.contains("omni-nav-forward"), "html: {html}");
    }

    #[test]
    fn markup_omits_nav_links_without_targets() {
        let input = input_with(None, None);
        let html = render_omnibar_markup(&input);
        assert!(!html.contains("omni-nav-back"), "html: {html}");
        assert!(!html.contains("omni-nav-forward"), "html: {html}");
    }

    // ---- markup: behavior hooks the JS targets ----

    #[test]
    fn markup_renders_truncated_address_and_behavior_hooks() {
        let input = input_with(None, None);
        let html = render_omnibar_markup(&input);
        // Truncated EPR id (slug is > 20 chars): head6...tail6.
        assert!(html.contains("elohim...abcdef"), "html: {html}");
        // Behavior hooks present (wired by omni-enhance.js).
        assert!(html.contains("data-omni-action=\"toggle\""), "html: {html}");
        assert!(html.contains("data-omni-action=\"copy\""), "html: {html}");
        assert!(
            html.contains("data-omni-action=\"theme-toggle\""),
            "html: {html}"
        );
        assert!(html.contains("data-omni-resilience-slug="), "html: {html}");
        // Copy carries the value the JS writes to the clipboard.
        assert!(
            html.contains("data-omni-copy-value=\"elohim-host-landing-0123456789abcdef\""),
            "html: {html}"
        );
    }

    // ---- the var(--omni-*) surface + NO raw rgba in markup ----

    #[test]
    fn markup_references_only_css_vars_no_raw_rgba() {
        let mut input = input_with(Some(sample_theme(ColorScheme::Auto)), Some("deadbeef"));
        input.show_env = true;
        input.env_tier = Some("dev");
        input.authenticated = true;
        input.nav_back = Some(NavTarget {
            href: "/resource/b",
            label: "Back",
        });
        input.nav_forward = Some(NavTarget {
            href: "/resource/f",
            label: "Fwd",
        });
        let html = render_omnibar_markup(&input);
        // The markup must not carry raw color values — those live in the <style>.
        assert!(
            !html.contains("rgba("),
            "markup must not contain raw rgba(): {html}"
        );
        // It also must not bake the theme's hex env-ring inline.
        assert!(
            !html.contains("#abcdef"),
            "markup must not bake a theme hex inline: {html}"
        );
    }

    #[test]
    fn style_exposes_the_full_omni_var_surface() {
        let input = input_with(None, None);
        let css = render_omnibar_style(&input);
        for name in [
            "--omni-bg",
            "--omni-fg",
            "--omni-muted",
            "--omni-border",
            "--omni-accent",
            "--omni-shadow",
            "--omni-env-ring",
        ] {
            assert!(css.contains(name), "style missing {name}: {css}");
        }
        // The layout CSS paints through var(--omni-*).
        assert!(css.contains("var(--omni-bg,"), "css: {css}");
    }

    // ---- the XSS escape proof ----

    #[test]
    fn markup_escapes_a_script_bearing_title() {
        let mut input = ChromeInput::for_slug("safe-slug-1234567890");
        input.title = "<script>alert('xss')</script>";
        input.build_marker = Some("<img src=x onerror=alert(1)>");
        let html = render_omnibar_markup(&input);
        // The raw <script> must NOT appear; it must be entity-escaped.
        assert!(
            !html.contains("<script>"),
            "unescaped <script> leaked into markup: {html}"
        );
        assert!(
            html.contains("&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"),
            "title not escaped: {html}"
        );
        // The marker (also attacker-influenceable) is escaped.
        assert!(
            !html.contains("<img src=x"),
            "unescaped marker leaked: {html}"
        );
        assert!(
            html.contains("&lt;img src=x onerror=alert(1)&gt;"),
            "marker not escaped: {html}"
        );
    }

    #[test]
    fn markup_escapes_a_script_bearing_slug() {
        let mut input = ChromeInput::for_slug("\"><script>alert(1)</script>");
        input.title = "t";
        let html = render_omnibar_markup(&input);
        // The slug is spliced into both an href and several attributes; none may
        // break out of their context.
        assert!(!html.contains("<script>"), "slug broke out: {html}");
        assert!(
            !html.contains("\"><script"),
            "slug attribute breakout: {html}"
        );
    }

    #[test]
    fn markup_escapes_a_script_bearing_env_tier_and_nav_label() {
        let mut input = input_with(None, Some("deadbeef"));
        input.show_env = true;
        input.env_tier = Some("<script>alert('tier')</script>");
        input.nav_back = Some(NavTarget {
            href: "/r/\"><script>",
            label: "<script>nav</script>",
        });
        let html = render_omnibar_markup(&input);
        // Newly-interpolated values (tier, nav href/label) are also escaped.
        assert!(!html.contains("<script>"), "env/nav broke out: {html}");
        assert!(
            html.contains("&lt;script&gt;alert(&#x27;tier&#x27;)&lt;/script&gt;"),
            "env tier not escaped: {html}"
        );
        assert!(
            !html.contains("\"><script"),
            "nav href attribute breakout: {html}"
        );
    }
}
