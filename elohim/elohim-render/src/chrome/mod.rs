//! Native runtime chrome — the omnibar composed (spliced) around the V8-rendered
//! Angular body.
//!
//! Phase 2 (revised) lifts the EPR omnibar out of the Angular app bundle and
//! out of the Rust SSR-splice into a runtime-served, self-contained CLIENT
//! ELEMENT (`element` / `omni-element.js`) — themed from the wrapped EPR's
//! declared `metadata.theme` tokens with a base-palette fallback. The element
//! self-mounts and renders client-side identically in every context (browser
//! SSR shell, `/deliver` CSR, the Tauri static SPA).
//!
//! - [`theme`] — the theme inputs (the `--omni-*` token shape + base palette).
//! - [`omnibar`] — the Rust markup/style renderer (the design source-of-truth
//!   the element JS faithfully ports).
//! - [`element`] — the content-addressed served element bytes/hash/path.
//! - [`enhance`] — SUPERSEDED; delegates to [`element`] so the runtime serves a
//!   single content-addressed asset that is the full element.
//!
//! Spec: `genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md` §4.3
//! Plan: `genesis/docs/superpowers/plans/2026-06-26-native-chrome-omnibar-plan.md`

pub mod element;
pub mod enhance;
pub mod omnibar;
pub mod theme;

pub use element::{element_js_bytes, element_js_hash, element_script_path, ELEMENT_JS};
pub use enhance::{enhance_js_bytes, enhance_js_hash, enhance_script_path, ENHANCE_JS};
pub use omnibar::{html_escape, render_omnibar_markup, render_omnibar_style, ChromeInput};
pub use theme::{base_palette, BasePalette, ColorScheme, Theme, ThemeTokens};
