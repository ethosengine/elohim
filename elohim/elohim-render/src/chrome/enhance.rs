//! Content-addressed progressive-enhancement script for the native runtime
//! chrome.
//!
//! The hand-written `omni-enhance.js` is baked into the binary via `include_str!`
//! and content-addressed by its `sha256`. The doorway serves it at
//! `/chrome/omni-enhance.{hash}.js` with immutable cache headers; the
//! `ComposingRenderer` (Task 3) references the SAME hash via
//! [`enhance_script_path`] so the spliced `<script src>` always matches the
//! served route — no staleness window.
//!
//! Hashing convention mirrors `bootstrap.rs` (`format!("{:x}", Sha256::digest)`).
//! The filename hash is bare lowercase hex (a clean content address); the
//! `enhance_js_hash()` accessor exposes that same hex.

use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// The hand-written vanilla enhancement script, baked at compile time.
pub const ENHANCE_JS: &str = include_str!("omni-enhance.js");

/// The script's bare lowercase-hex `sha256`, computed once on first use.
fn enhance_js_hash_cell() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| format!("{:x}", Sha256::digest(ENHANCE_JS.as_bytes())))
}

/// The script bytes the doorway `/chrome/` route serves.
#[must_use]
pub fn enhance_js_bytes() -> &'static [u8] {
    ENHANCE_JS.as_bytes()
}

/// The script's content address — bare lowercase-hex `sha256` of [`ENHANCE_JS`].
#[must_use]
pub fn enhance_js_hash() -> &'static str {
    enhance_js_hash_cell()
}

/// The content-addressed URL path the doorway serves the script at, e.g.
/// `/chrome/omni-enhance.<sha256hex>.js`. Task 3 splices a `<script src>`
/// pointing here; Task 5's route serves the bytes at exactly this path.
#[must_use]
pub fn enhance_script_path() -> String {
    format!("/chrome/omni-enhance.{}.js", enhance_js_hash())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_matches_script_bytes() {
        let expected = format!("{:x}", Sha256::digest(ENHANCE_JS.as_bytes()));
        assert_eq!(enhance_js_hash(), expected);
    }

    #[test]
    fn script_path_carries_the_hash() {
        let path = enhance_script_path();
        assert!(path.starts_with("/chrome/omni-enhance."), "{path}");
        assert!(path.ends_with(".js"), "{path}");
        assert!(path.contains(enhance_js_hash()), "{path}");
    }

    #[test]
    fn hash_is_lowercase_hex_64_chars() {
        let h = enhance_js_hash();
        assert_eq!(h.len(), 64, "sha256 hex is 64 chars: {h}");
        assert!(
            h.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "hash must be lowercase hex: {h}"
        );
    }

    #[test]
    fn bytes_match_const() {
        assert_eq!(enhance_js_bytes(), ENHANCE_JS.as_bytes());
    }

    #[test]
    fn script_is_nonempty_and_bounded() {
        // Sanity: the baked script is present and within the ~3-5KB design budget
        // (generous upper bound to catch accidental bloat, not a hard limit).
        let len = ENHANCE_JS.len();
        assert!(len > 1000, "script unexpectedly small: {len} bytes");
        assert!(len < 12_000, "script unexpectedly large: {len} bytes");
    }
}
