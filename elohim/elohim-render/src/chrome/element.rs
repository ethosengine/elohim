//! Content-addressed runtime-served EPR omnibar ELEMENT — re-export shim.
//!
//! The element bytes/hash/path helpers (and the `omni-element.js` source) now
//! live in the **light** `elohim-chrome-asset` crate, whose only dependency is
//! `sha2` — so serving the static element does NOT require compiling V8.
//! `elohim-render` hard-depends on `deno_core` (the SSR-splice renderers in
//! `omnibar.rs` / `theme.rs`), so the served-asset accessors were lifted out to
//! keep the doorway's and the storage sidecar's `/chrome/` route off the V8
//! path. See `elohim-chrome-asset` for the implementation and rationale.
//!
//! This module re-exports them verbatim so every existing
//! `elohim_render::chrome::element::*` / `elohim_render::element_*` caller (the
//! doorway `routes/chrome.rs`, the enhance-supersession shim) is unchanged.

pub use elohim_chrome_asset::{
    element_js_bytes, element_js_hash, element_script_path, escape_json_for_script, inject_element,
    CONTEXT_SCRIPT_ID, ELEMENT_JS, STABLE_ELEMENT_PATH,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexports_resolve_to_the_light_crate() {
        // The shim forwards the same content address the light crate computes,
        // so the doorway's referenced <script src> and the served bytes agree.
        assert_eq!(element_js_hash(), elohim_chrome_asset::element_js_hash());
        assert_eq!(element_script_path(), elohim_chrome_asset::element_script_path());
        assert_eq!(element_js_bytes(), elohim_chrome_asset::element_js_bytes());
        assert_eq!(STABLE_ELEMENT_PATH, "/chrome/omni-element.js");
        // inject_element is reachable through elohim-render too (doorway uses it).
        let out = inject_element("<body></body>", "{}");
        assert!(out.contains("id=\"elohim-omni-context\""));
    }
}
