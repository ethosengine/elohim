//! Native runtime chrome — the omnibar composed (spliced) around the V8-rendered
//! Angular body.
//!
//! Phase 2 lifts the EPR omnibar out of the Angular app bundle into native Rust
//! chrome, themed from the wrapped EPR's declared `metadata.theme` tokens with a
//! base-palette fallback. This module owns the theme inputs (Task 1) and, in
//! later tasks, the markup/style renderer and the splice composition.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md` §4.3
//! Plan: `genesis/docs/superpowers/plans/2026-06-26-native-chrome-omnibar-plan.md`

pub mod omnibar;
pub mod theme;

pub use omnibar::{html_escape, render_omnibar_markup, render_omnibar_style, ChromeInput};
pub use theme::{base_palette, BasePalette, ColorScheme, Theme, ThemeTokens};
