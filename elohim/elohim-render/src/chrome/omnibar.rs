//! Native omnibar chrome renderer — markup + themed `<style>`, in Rust.
//!
//! A faithful port of the Angular `app-protocol-omnibar` component
//! (`app/elohim-app/src/app/elohim/components/protocol-omnibar/`) into native
//! runtime chrome. The omnibar leaves the per-EPR Angular bundle and becomes
//! EPR-agnostic chrome the runtime composes (splices) around the V8-rendered
//! Angular body (Task 3). This module owns the two halves of that chrome:
//!
//! - [`render_omnibar_style`] — the scoped `<style>` binding the seven
//!   `--omni-*` custom properties from the EPR's [`Theme`] tokens (or
//!   [`base_palette`] when absent), and the color-scheme behavior: `Auto`
//!   emits a light `:root` default + a `@media (prefers-color-scheme: dark)`
//!   override; `Light`/`Dark` pin one set. The client theme toggle
//!   (`html[data-theme=…]`) still wins via the `[data-theme]` selector hooks.
//! - [`render_omnibar_markup`] — the collapsed pill + the expanded bar, with
//!   server-known values (truncated EPR id, build marker) rendered inline and
//!   behavior expressed as `data-omni-action` / `data-omni-resilience-slug`
//!   hooks that `omni-enhance.js` (Task 5) wires. The markup references only
//!   `var(--omni-*, <fallback>)` — never raw color.
//!
//! Every interpolated value is HTML-escaped ([`html_escape`]) — a server-side
//! splice of attacker-influenceable content (title/description/slug/marker) is
//! the one new XSS surface this layer introduces.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md` §4.1, §4.4
//! Plan: `genesis/docs/superpowers/plans/2026-06-26-native-chrome-omnibar-plan.md` (Task 2)

use super::theme::{base_palette, ColorScheme, Theme, ThemeTokens};

/// Server-known inputs to the native omnibar chrome.
///
/// These are the values the runtime can paint without JS: the EPR slug (→ the
/// truncated address chip), the served build marker, and the EPR's declared
/// theme. Dynamic detail (resilience tier, stewards, prev/next nav) stays
/// client-enhanced in `omni-enhance.js` (Task 5) — the markup emits hooks, not
/// data, for those.
///
/// Borrowed `&str` fields: the caller (the composition layer, Task 3) builds
/// this transiently per request from the fetched content node; nothing here
/// outlives the render call.
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
    /// rendered (the chrome stays valid without it).
    pub build_marker: Option<&'a str>,
    /// The EPR-declared theme. Absent ⇒ the [`base_palette`] fallback.
    pub theme: Option<Theme>,
}

impl ChromeInput<'_> {
    /// The resolved color scheme: the theme's declared scheme, or `Auto` when
    /// no theme is supplied (emit both light + dark, let the OS pick).
    #[must_use]
    pub fn color_scheme(&self) -> ColorScheme {
        self.theme
            .as_ref()
            .map_or(ColorScheme::Auto, |t| t.color_scheme)
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

/// Truncate a content address the way the Angular component's
/// `truncatedAddress` getter does: `<=16` chars passes through; longer becomes
/// `head7…tail6`. Mirrors `protocol-omnibar.component.ts`.
#[must_use]
fn truncate_address(address: &str) -> String {
    let count = address.chars().count();
    if count <= 16 {
        return address.to_string();
    }
    let head: String = address.chars().take(7).collect();
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
/// Authority chain (mirrors `protocol-omnibar.component.css` + the
/// `html[data-theme]` contract):
/// 1. `:root` carries the light defaults (or the pinned set for explicit
///    `Light`/`Dark`).
/// 2. For `Auto`, `@media (prefers-color-scheme: dark) :root { … }` overrides
///    with the dark set — the OS preference is the no-`data-theme` fallback.
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
/// `protocol-omnibar.component.css`; every color/shadow/border now reads a
/// `--omni-*` custom property (with the verbatim original value as the
/// `var()` fallback so the bar paints even if a token is unset).
const OMNI_LAYOUT_CSS: &str = "\
#elohim-omni { position: fixed; top: 0.625rem; right: 0.625rem; z-index: 9999; \
font-family: system-ui, sans-serif; }
#elohim-omni .omni-pill-group { display: flex; gap: 0.375rem; }
#elohim-omni .omni-pill { display: flex; align-items: center; gap: 0.25rem; \
padding: 0.3rem 0.5rem; background: var(--omni-bg, rgba(255,255,255,0.92)); \
border: 1px solid var(--omni-border, rgba(20,22,30,0.14)); border-radius: 16px; \
color: var(--omni-fg, rgba(20,22,30,0.96)); font-size: 0.6875rem; font-weight: 600; \
cursor: pointer; box-shadow: var(--omni-shadow, 0 1px 6px rgba(0,0,0,0.08)); \
backdrop-filter: blur(12px); transition: all 0.2s; }
#elohim-omni .omni-pill:hover { border-color: var(--omni-accent, rgba(20,22,30,0.96)); }
#elohim-omni .omni-pill-mark { font-family: serif; font-weight: 700; font-size: 0.75rem; \
color: var(--omni-accent, rgba(20,22,30,0.96)); }
#elohim-omni .omni-pill-reach { font-size: 0.5rem; opacity: 0.8; color: var(--omni-muted, rgba(20,22,30,0.55)); }
#elohim-omni .omni-expanded { display: none; align-items: center; gap: 0.5rem; \
padding: 0.375rem 0.625rem; background: var(--omni-bg, rgba(255,255,255,0.92)); \
border: 1px solid var(--omni-border, rgba(20,22,30,0.14)); border-radius: 10px; \
color: var(--omni-fg, rgba(20,22,30,0.96)); font-size: 0.6875rem; \
box-shadow: var(--omni-shadow, 0 1px 6px rgba(0,0,0,0.08)); backdrop-filter: blur(16px); \
max-width: calc(100vw - 1.25rem); }
#elohim-omni[data-omni-state='expanded'] .omni-expanded { display: flex; }
#elohim-omni[data-omni-state='expanded'] .omni-pill-group { display: none; }
#elohim-omni .omni-main { display: flex; align-items: center; gap: 0.5rem; \
overflow: hidden; white-space: nowrap; }
#elohim-omni .omni-address { display: inline-flex; align-items: center; gap: 0.25rem; \
color: var(--omni-accent, rgba(20,22,30,0.96)); text-decoration: none; \
font-family: 'SF Mono', 'Fira Code', monospace; font-size: 0.6875rem; }
#elohim-omni .omni-address-mark { font-family: serif; font-weight: 700; font-size: 0.75rem; \
color: var(--omni-accent, rgba(20,22,30,0.96)); }
#elohim-omni .omni-divider { width: 1px; height: 12px; \
background: var(--omni-border, rgba(20,22,30,0.14)); flex-shrink: 0; }
#elohim-omni .omni-reach { color: var(--omni-muted, rgba(20,22,30,0.55)); \
font-size: 0.625rem; flex-shrink: 0; }
#elohim-omni .omni-env-ring { display: inline-block; width: 0.5rem; height: 0.5rem; \
border-radius: 50%; box-shadow: 0 0 0 2px var(--omni-env-ring, #d97706); flex-shrink: 0; }
#elohim-omni .omni-marker { color: var(--omni-muted, rgba(20,22,30,0.55)); \
font-size: 0.5625rem; font-style: italic; }
#elohim-omni .omni-actions { display: flex; align-items: center; gap: 0.125rem; \
flex-shrink: 0; margin-left: 0.25rem; }
#elohim-omni .omni-btn { background: none; border: none; \
color: var(--omni-muted, rgba(20,22,30,0.55)); cursor: pointer; font-size: 0.75rem; \
padding: 0.125rem 0.25rem; border-radius: 4px; line-height: 1; transition: all 0.15s; }
#elohim-omni .omni-btn:hover { color: var(--omni-fg, rgba(20,22,30,0.96)); \
background: var(--omni-border, rgba(20,22,30,0.14)); }
#elohim-omni .omni-resilience { display: inline-flex; align-items: center; \
color: var(--omni-muted, rgba(20,22,30,0.55)); font-size: 0.625rem; flex-shrink: 0; }
@media (max-width: 640px) { #elohim-omni .omni-marker { display: none; } }
";

/// Render the omnibar markup: the collapsed pill (default) + the expanded bar.
///
/// Server-known values are rendered inline and HTML-escaped; behavior is
/// expressed as `data-omni-action` hooks (`toggle` / `copy` / `theme-toggle`)
/// and a `data-omni-resilience-slug` hook for the lazy resilience panel
/// (`omni-enhance.js` populates it on first expand — here it is a neutral
/// glyph placeholder). The container starts in the `pill` state; the script
/// flips `data-omni-state` to `expanded`.
///
/// `<a href>` affordances (inspect, account) are plain links that work with
/// zero JS — progressive enhancement.
#[must_use]
pub fn render_omnibar_markup(input: &ChromeInput) -> String {
    let slug_attr = html_escape(input.slug);
    let title_attr = html_escape(input.title);
    let truncated = html_escape(&truncate_address(input.slug));
    let inspect_href = format!("/epr/{slug_attr}");

    let mut html = String::new();
    html.push_str(&format!(
        "<div id=\"elohim-omni\" data-omni-state=\"pill\" \
data-omni-resilience-slug=\"{slug_attr}\">\n"
    ));

    // === COLLAPSED PILL (default) ===
    html.push_str("  <div class=\"omni-pill-group\">\n");
    html.push_str(&format!(
        "    <button type=\"button\" class=\"omni-pill\" data-omni-action=\"toggle\" \
aria-label=\"View protocol provenance for {title_attr}\">\n"
    ));
    html.push_str("      <span class=\"omni-pill-mark\">E</span>\n");
    // Neutral resilience glyph placeholder — JS replaces on first expand.
    html.push_str(
        "      <span class=\"omni-pill-reach\" data-omni-resilience-glyph aria-hidden=\"true\">\u{25CB}</span>\n",
    );
    html.push_str("    </button>\n");
    html.push_str("  </div>\n");

    // === EXPANDED BAR ===
    html.push_str("  <div class=\"omni-expanded\">\n");
    html.push_str("    <div class=\"omni-main\">\n");

    // EPR address chip (clickable -> inspect). Plain link; works with no JS.
    html.push_str(&format!(
        "      <a href=\"{inspect_href}\" class=\"omni-address\" \
title=\"Inspect EPR {title_attr}\">\n"
    ));
    html.push_str("        <span class=\"omni-address-mark\">E</span>\n");
    html.push_str(&format!("        <code>{truncated}</code>\n"));
    html.push_str("      </a>\n");

    // Copy affordance — behavior wired by omni-enhance.js.
    html.push_str(&format!(
        "      <button type=\"button\" class=\"omni-btn\" data-omni-action=\"copy\" \
data-omni-copy-value=\"{slug_attr}\" aria-label=\"Copy EPR address\">\u{1F4CB}</button>\n"
    ));

    html.push_str("      <span class=\"omni-divider\"></span>\n");

    // Env ring — a themed provenance dot (the env-ring token). Provenance-true,
    // not decorative: it reads the --omni-env-ring token.
    html.push_str(
        "      <span class=\"omni-env-ring\" title=\"Protocol-delivered\" aria-label=\"Protocol-delivered\"></span>\n",
    );

    // Lazy resilience panel placeholder — neutral glyph; JS fetches + populates.
    html.push_str(
        "      <span class=\"omni-resilience\" data-omni-resilience-glyph aria-label=\"Resilience standing\">\u{25CB}</span>\n",
    );

    // Build marker (server-known, inline) — only when present.
    if let Some(marker) = input.build_marker {
        let marker_text = html_escape(marker);
        html.push_str(&format!(
            "      <span class=\"omni-marker\">via {marker_text}</span>\n"
        ));
    }

    html.push_str("    </div>\n");

    // Actions area: theme toggle + collapse.
    html.push_str("    <div class=\"omni-actions\">\n");
    html.push_str(
        "      <button type=\"button\" class=\"omni-btn\" data-omni-action=\"theme-toggle\" \
aria-label=\"Toggle theme\">\u{25D0}</button>\n",
    );
    html.push_str(
        "      <button type=\"button\" class=\"omni-btn\" data-omni-action=\"toggle\" \
aria-label=\"Collapse omnibar\">\u{2715}</button>\n",
    );
    html.push_str("    </div>\n");

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
        ChromeInput {
            slug: "elohim-host-landing-0123456789abcdef",
            title: "Welcome",
            description: "The landing EPR",
            build_marker: marker,
            theme,
        }
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

    // ---- truncate_address ----

    #[test]
    fn truncate_address_passthrough_short() {
        assert_eq!(truncate_address("short"), "short");
        assert_eq!(truncate_address("0123456789abcdef"), "0123456789abcdef"); // exactly 16
    }

    #[test]
    fn truncate_address_truncates_long() {
        assert_eq!(truncate_address("0123456789abcdefghij"), "0123456...efghij");
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

    // ---- markup: with / without marker ----

    #[test]
    fn markup_with_marker_renders_it_inline() {
        let input = input_with(None, Some("a1b2c3d"));
        let html = render_omnibar_markup(&input);
        assert!(html.contains("via a1b2c3d"), "html: {html}");
        assert!(html.contains("class=\"omni-marker\""), "html: {html}");
    }

    #[test]
    fn markup_without_marker_omits_the_marker_span() {
        let input = input_with(None, None);
        let html = render_omnibar_markup(&input);
        assert!(
            !html.contains("class=\"omni-marker\""),
            "no marker span without a build marker: {html}"
        );
    }

    #[test]
    fn markup_renders_truncated_address_and_behavior_hooks() {
        let input = input_with(None, None);
        let html = render_omnibar_markup(&input);
        // Truncated EPR id (slug is > 16 chars).
        assert!(html.contains("elohim-...abcdef"), "html: {html}");
        // Behavior hooks present (wired by omni-enhance.js).
        assert!(html.contains("data-omni-action=\"toggle\""), "html: {html}");
        assert!(html.contains("data-omni-action=\"copy\""), "html: {html}");
        assert!(
            html.contains("data-omni-action=\"theme-toggle\""),
            "html: {html}"
        );
        assert!(html.contains("data-omni-resilience-slug="), "html: {html}");
        // Starts collapsed.
        assert!(html.contains("data-omni-state=\"pill\""), "html: {html}");
    }

    // ---- the var(--omni-*) surface + NO raw rgba in markup ----

    #[test]
    fn markup_references_only_css_vars_no_raw_rgba() {
        let input = input_with(Some(sample_theme(ColorScheme::Auto)), Some("deadbeef"));
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
        let input = ChromeInput {
            slug: "safe-slug-1234567890",
            title: "<script>alert('xss')</script>",
            description: "desc",
            build_marker: Some("<img src=x onerror=alert(1)>"),
            theme: None,
        };
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
        let input = ChromeInput {
            slug: "\"><script>alert(1)</script>",
            title: "t",
            description: "d",
            build_marker: None,
            theme: None,
        };
        let html = render_omnibar_markup(&input);
        // The slug is spliced into both an href and several attributes; none may
        // break out of their context.
        assert!(!html.contains("<script>"), "slug broke out: {html}");
        assert!(
            !html.contains("\"><script"),
            "slug attribute breakout: {html}"
        );
    }
}
