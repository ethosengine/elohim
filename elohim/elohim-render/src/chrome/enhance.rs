//! Content-addressed progressive-enhancement script for the native runtime
//! chrome — SUPERSEDED by the self-contained element ([`super::element`]).
//!
//! Historically this module baked a small `omni-enhance.js` that progressively
//! enhanced server-rendered omnibar markup. The Phase-2 (revised) pivot replaced
//! the SSR-splice with a runtime-served, self-contained CLIENT ELEMENT
//! (`omni-element.js`) that self-mounts, renders, themes, AND wires the same
//! behavior — so there is no separate server-rendered markup left to "enhance."
//!
//! The element FULLY ABSORBS the enhance behavior (the `data-omni-*` action
//! hooks, the `elohim-theme` persistence, the `elohim-theme-changed` event, the
//! lazy `/api/v1/resilience/{slug}` fetch). To keep the runtime coherent — ONE
//! served script that is the full element — these accessors now DELEGATE to
//! [`super::element`]: `enhance_js_bytes()`/`enhance_js_hash()` return the
//! element's bytes/hash, and `enhance_script_path()` returns the element's
//! content-addressed path. Any existing caller that referenced the "enhance"
//! script therefore transparently serves and references the element.
//!
//! Prefer the `element_*` accessors in new code; the `enhance_*` names are kept
//! only so in-flight references keep resolving to the (now superseding) element.

use super::element;

/// The served enhancement script — now the self-contained element bytes.
///
/// Kept as a `const` alias of [`element::ELEMENT_JS`] for any caller that
/// referenced the baked source directly.
pub const ENHANCE_JS: &str = element::ELEMENT_JS;

/// The served script bytes — the element bytes (supersession).
#[must_use]
pub fn enhance_js_bytes() -> &'static [u8] {
    element::element_js_bytes()
}

/// The served script's content address — the element's `sha256` (supersession).
#[must_use]
pub fn enhance_js_hash() -> &'static str {
    element::element_js_hash()
}

/// The content-addressed URL path the doorway serves — the element's path
/// (`/chrome/omni-element.<sha256hex>.js`). The enhance path is no longer a
/// distinct asset; it resolves to the element so there is a single served file.
#[must_use]
pub fn enhance_script_path() -> String {
    element::element_script_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enhance_now_delegates_to_the_element() {
        // Supersession invariant: the enhance accessors return the element's
        // bytes/hash/path verbatim — one served script that is the full element.
        assert_eq!(ENHANCE_JS, element::ELEMENT_JS);
        assert_eq!(enhance_js_bytes(), element::element_js_bytes());
        assert_eq!(enhance_js_hash(), element::element_js_hash());
        assert_eq!(enhance_script_path(), element::element_script_path());
    }

    #[test]
    fn served_path_is_the_element_path() {
        // The path now carries the element filename (supersession), not the old
        // omni-enhance filename — so the served asset is unambiguously the element.
        let path = enhance_script_path();
        assert!(path.starts_with("/chrome/omni-element."), "{path}");
        assert!(path.ends_with(".js"), "{path}");
    }
}
