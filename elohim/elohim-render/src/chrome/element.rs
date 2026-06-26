//! Content-addressed runtime-served EPR omnibar ELEMENT for the native chrome.
//!
//! The hand-written, self-contained vanilla `omni-element.js` is baked into the
//! binary via `include_str!` and content-addressed by its `sha256`. The doorway
//! serves it at `/chrome/omni-element.{hash}.js` with immutable cache headers;
//! any page (the doorway SSR shell, a `/deliver` CSR page, the Tauri static
//! `index.html`) references it via a single `<script src>` pointing at
//! [`element_script_path`], so the referenced path and the served route never
//! diverge — no staleness window.
//!
//! Unlike the older `omni-enhance.js` (which enhanced server-rendered markup),
//! the element is render + theme + behavior in ONE file: it self-mounts the
//! omnibar, acquires the wrapped EPR's context (inline-injected OR fetched),
//! renders the rich `protocol-omni` markup, applies the EPR theme, and wires the
//! behavior — all client-side, identically in every context. It therefore
//! SUPERSEDES the enhance script; `enhance.rs` now delegates to this module so
//! the runtime serves a single content-addressed asset that is the full element.
//!
//! Hashing convention mirrors `enhance.rs` / `bootstrap.rs`
//! (`format!("{:x}", Sha256::digest)`). The filename hash is bare lowercase hex
//! (a clean content address); [`element_js_hash`] exposes that same hex.

use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// The hand-written, self-contained vanilla element script, baked at compile
/// time. Self-mounts, acquires EPR context, renders + themes + wires behavior.
pub const ELEMENT_JS: &str = include_str!("omni-element.js");

/// The script's bare lowercase-hex `sha256`, computed once on first use.
fn element_js_hash_cell() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| format!("{:x}", Sha256::digest(ELEMENT_JS.as_bytes())))
}

/// The element bytes the doorway `/chrome/` route serves.
#[must_use]
pub fn element_js_bytes() -> &'static [u8] {
    ELEMENT_JS.as_bytes()
}

/// The element's content address — bare lowercase-hex `sha256` of [`ELEMENT_JS`].
#[must_use]
pub fn element_js_hash() -> &'static str {
    element_js_hash_cell()
}

/// The content-addressed URL path the doorway serves the element at, e.g.
/// `/chrome/omni-element.<sha256hex>.js`. Any page splices a `<script src>`
/// pointing here; the doorway `/chrome/` route serves the bytes at exactly this
/// path.
#[must_use]
pub fn element_script_path() -> String {
    format!("/chrome/omni-element.{}.js", element_js_hash())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_matches_script_bytes() {
        let expected = format!("{:x}", Sha256::digest(ELEMENT_JS.as_bytes()));
        assert_eq!(element_js_hash(), expected);
    }

    #[test]
    fn script_path_carries_the_hash() {
        let path = element_script_path();
        assert!(path.starts_with("/chrome/omni-element."), "{path}");
        assert!(path.ends_with(".js"), "{path}");
        assert!(path.contains(element_js_hash()), "{path}");
    }

    #[test]
    fn hash_is_lowercase_hex_64_chars() {
        let h = element_js_hash();
        assert_eq!(h.len(), 64, "sha256 hex is 64 chars: {h}");
        assert!(
            h.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "hash must be lowercase hex: {h}"
        );
    }

    #[test]
    fn bytes_match_const() {
        assert_eq!(element_js_bytes(), ELEMENT_JS.as_bytes());
    }

    #[test]
    fn script_is_self_contained_and_bounded() {
        // The element is render + theme + behavior in one file: larger than the
        // enhance-only script, but still a single small asset. The bounds catch
        // accidental bloat, not a hard limit.
        let len = ELEMENT_JS.len();
        assert!(len > 3000, "element unexpectedly small: {len} bytes");
        assert!(len < 60_000, "element unexpectedly large: {len} bytes");
    }

    #[test]
    fn element_carries_the_self_mount_and_context_contract() {
        // The element MUST self-mount (#elohim-omni), read an inline context
        // island OR fetch the content node, and resolve the landing slug. These
        // string anchors guard the contract sibling tasks depend on.
        assert!(
            ELEMENT_JS.contains("elohim-omni"),
            "missing omni container id"
        );
        assert!(
            ELEMENT_JS.contains("elohim-omni-context"),
            "missing inline-context island id"
        );
        assert!(
            ELEMENT_JS.contains("/db/content/"),
            "missing content-node fetch fallback"
        );
        assert!(
            ELEMENT_JS.contains("elohim-host-landing"),
            "missing landing-slug resolution"
        );
        // Behavior contract (absorbed from omni-enhance.js).
        assert!(
            ELEMENT_JS.contains("elohim-theme-changed"),
            "missing theme-changed event"
        );
        assert!(
            ELEMENT_JS.contains("/api/v1/resilience/"),
            "missing lazy resilience fetch"
        );
    }
}
