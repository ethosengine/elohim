//! Native chrome theme — the palette inputs for the runtime-rendered omnibar.
//!
//! The omnibar (Phase 2 native chrome) is themed from the wrapped EPR content
//! node's `metadata.theme` JSON bag. When that is absent the renderer falls back
//! to [`BASE_PALETTE`] — the exact RGBA lifted verbatim from the originating
//! Angular component `protocol-omni.component.css` (`:host` for light, the
//! `@media (prefers-color-scheme: dark)` block for dark).
//!
//! Boundary discipline: the JSON/serde surface is camelCase (`colorScheme`,
//! `envRing`); inside Rust the fields are snake_case. `theme` is metadata on the
//! existing landing content node — NOT a new DHT entity or table.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md` §4.3
//! Plan: `genesis/docs/superpowers/plans/2026-06-26-native-chrome-omnibar-plan.md` (Task 1)

use serde::{Deserialize, Serialize};

/// How the chrome resolves its color scheme.
///
/// `Auto` defers to `prefers-color-scheme` (the OS scheme) as the no-preference
/// fallback; `Light`/`Dark` pin it explicitly. Mirrors the CSS theme-authority
/// contract (`html[data-theme]` wins over the media block).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    Light,
    Dark,
    #[default]
    Auto,
}

/// The seven `--omni-*` token values that paint the omnibar chrome.
///
/// Each field is the raw CSS value string (an `rgba(…)`, a hex color, or a
/// `box-shadow` declaration) bound 1:1 to a `--omni-*` custom property by the
/// renderer (Task 2). Stored as `String` so the seed can carry any valid CSS
/// value without the substrate needing to parse colors.
///
/// camelCase on the JSON boundary (`envRing`), snake_case inside Rust
/// (`env_ring`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeTokens {
    /// `--omni-bg`
    pub bg: String,
    /// `--omni-fg`
    pub fg: String,
    /// `--omni-muted`
    pub muted: String,
    /// `--omni-border`
    pub border: String,
    /// `--omni-accent`
    pub accent: String,
    /// `--omni-shadow`
    pub shadow: String,
    /// `--omni-env-ring`
    pub env_ring: String,
}

/// The EPR-declared theme, deserialized from `metadata.theme`.
///
/// Absent on the content node ⇒ the renderer uses [`BASE_PALETTE`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub color_scheme: ColorScheme,
    pub tokens: ThemeTokens,
}

/// A complete light+dark fallback palette.
///
/// [`BASE_PALETTE`] is the canonical instance, lifted verbatim from
/// `protocol-omni.component.css`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasePalette {
    pub light: ThemeTokens,
    pub dark: ThemeTokens,
}

impl BasePalette {
    /// The token set for a resolved (concrete) scheme. `Auto` resolves to
    /// `light` here — the renderer emits both light and dark rules and lets
    /// `prefers-color-scheme` pick at paint time, so the static default is the
    /// light set.
    #[must_use]
    pub fn tokens_for(&self, scheme: ColorScheme) -> &ThemeTokens {
        match scheme {
            ColorScheme::Dark => &self.dark,
            ColorScheme::Light | ColorScheme::Auto => &self.light,
        }
    }
}

/// Build the base palette.
///
/// A function rather than a `const`/`static` because [`ThemeTokens`] holds
/// owned `String`s (the CSS value strings cannot be `const`). Cheap to call;
/// the renderer constructs it once on the fallback path.
///
/// Values lifted verbatim from `protocol-omni.component.css`:
/// - light: the `:host` block (lines 20-27)
/// - dark: the `@media (prefers-color-scheme: dark)` block (lines 38-46)
#[must_use]
pub fn base_palette() -> BasePalette {
    BasePalette {
        light: ThemeTokens {
            bg: "rgba(255, 255, 255, 0.92)".to_string(),
            fg: "rgba(20, 22, 30, 0.96)".to_string(),
            muted: "rgba(20, 22, 30, 0.55)".to_string(),
            border: "rgba(20, 22, 30, 0.14)".to_string(),
            accent: "rgba(20, 22, 30, 0.96)".to_string(),
            shadow: "0 1px 6px rgba(0, 0, 0, 0.08)".to_string(),
            env_ring: "#d97706".to_string(),
        },
        dark: ThemeTokens {
            bg: "rgba(22, 23, 28, 0.92)".to_string(),
            fg: "rgba(232, 234, 240, 0.96)".to_string(),
            muted: "rgba(232, 234, 240, 0.55)".to_string(),
            border: "rgba(232, 234, 240, 0.16)".to_string(),
            accent: "rgba(232, 234, 240, 0.96)".to_string(),
            shadow: "0 1px 6px rgba(0, 0, 0, 0.35)".to_string(),
            env_ring: "#d97706".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (c) The camelCase JSON boundary parses into snake_case Rust:
    /// `colorScheme` → `color_scheme`, `envRing` → `env_ring`.
    #[test]
    fn theme_json_shape_deserializes_with_camelcase_boundary() {
        // Exactly the shape seeded under `metadata.theme` in
        // genesis/data/lamad/content/elohim-host-landing.json (light palette).
        let json = r##"{
            "colorScheme": "auto",
            "tokens": {
                "bg": "rgba(255, 255, 255, 0.92)",
                "fg": "rgba(20, 22, 30, 0.96)",
                "muted": "rgba(20, 22, 30, 0.55)",
                "border": "rgba(20, 22, 30, 0.14)",
                "accent": "rgba(20, 22, 30, 0.96)",
                "shadow": "0 1px 6px rgba(0, 0, 0, 0.08)",
                "envRing": "#d97706"
            }
        }"##;

        let theme: Theme = serde_json::from_str(json).expect("metadata.theme must deserialize");

        // (a) shape deserializes; camelCase boundary mapped to snake_case fields.
        assert_eq!(theme.color_scheme, ColorScheme::Auto);
        assert_eq!(theme.tokens.bg, "rgba(255, 255, 255, 0.92)");
        assert_eq!(theme.tokens.env_ring, "#d97706");
        // The seeded light tokens match the base-palette light set verbatim.
        assert_eq!(theme.tokens, base_palette().light);
    }

    /// camelCase round-trips on serialize (snake_case never leaks to JSON).
    #[test]
    fn theme_serializes_back_to_camelcase() {
        let theme = Theme {
            color_scheme: ColorScheme::Auto,
            tokens: base_palette().light,
        };
        let json = serde_json::to_string(&theme).expect("serialize");
        assert!(json.contains("\"colorScheme\":\"auto\""), "json: {json}");
        assert!(json.contains("\"envRing\":"), "json: {json}");
        assert!(
            !json.contains("color_scheme") && !json.contains("env_ring"),
            "snake_case must not leak to JSON: {json}"
        );
    }

    /// All three colorScheme variants parse.
    #[test]
    fn color_scheme_variants_parse() {
        assert_eq!(
            serde_json::from_str::<ColorScheme>("\"light\"").unwrap(),
            ColorScheme::Light
        );
        assert_eq!(
            serde_json::from_str::<ColorScheme>("\"dark\"").unwrap(),
            ColorScheme::Dark
        );
        assert_eq!(
            serde_json::from_str::<ColorScheme>("\"auto\"").unwrap(),
            ColorScheme::Auto
        );
    }

    /// (b) BASE_PALETTE light + dark are both populated (all 7 tokens non-empty),
    /// and carry the verbatim values lifted from protocol-omni.component.css.
    #[test]
    fn base_palette_light_and_dark_populated() {
        let palette = base_palette();

        for tokens in [&palette.light, &palette.dark] {
            for field in [
                &tokens.bg,
                &tokens.fg,
                &tokens.muted,
                &tokens.border,
                &tokens.accent,
                &tokens.shadow,
                &tokens.env_ring,
            ] {
                assert!(!field.is_empty(), "every base-palette token must be set");
            }
        }

        // Verbatim spot-checks against the CSS source.
        assert_eq!(palette.light.bg, "rgba(255, 255, 255, 0.92)");
        assert_eq!(palette.light.shadow, "0 1px 6px rgba(0, 0, 0, 0.08)");
        assert_eq!(palette.dark.bg, "rgba(22, 23, 28, 0.92)");
        assert_eq!(palette.dark.shadow, "0 1px 6px rgba(0, 0, 0, 0.35)");
        // env-ring is scheme-invariant in the source CSS.
        assert_eq!(palette.light.env_ring, palette.dark.env_ring);
        assert_eq!(palette.light.env_ring, "#d97706");
    }

    /// `tokens_for` resolves Auto→light, Dark→dark, Light→light.
    #[test]
    fn tokens_for_resolves_scheme() {
        let palette = base_palette();
        assert_eq!(palette.tokens_for(ColorScheme::Light), &palette.light);
        assert_eq!(palette.tokens_for(ColorScheme::Auto), &palette.light);
        assert_eq!(palette.tokens_for(ColorScheme::Dark), &palette.dark);
    }
}
