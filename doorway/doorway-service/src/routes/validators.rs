//! Shared response-validator helpers for mutable routes (story 6.1 —
//! "Validators, not caching, on mutable routes").
//!
//! A CDN will eventually stand in front of doorways. A mutable response (an
//! EPR-head declaration, the shell HTML entry) must never become
//! CDN-cacheable-as-fresh, but it CAN be made cheaply REVALIDATABLE: a strong
//! `ETag` over the exact bytes served, honoured on the way back in via
//! `If-None-Match`, so a repeat ask that changed nothing costs a `304` with no
//! body instead of the full representation. This module holds the ONE matcher
//! every such route shares, so the RFC 9110 comma-list header shape and the
//! `*` wildcard are handled correctly everywhere a validator is checked —
//! never duplicated per route. First landed inline in `auth_discovery.rs`
//! (`/.well-known/elohim-auth`); moved here so `routes::epr` and the shell
//! HTML entry (`server/http.rs::projected_shell_response`) can reuse it
//! byte-for-byte rather than re-deriving the RFC 9110 list semantics.

/// Does an `If-None-Match` header match our current ETag?
///
/// The header is a comma-separated LIST (RFC 9110 §13.1.2) and `*` matches any
/// current representation, so a naive string equality answers 200 to a
/// well-formed conditional request and the validator silently never fires.
pub fn etag_matches(if_none_match: &str, current: &str) -> bool {
    if_none_match
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == "*" || candidate == current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matching_validator_is_recognised_in_every_legal_header_shape() {
        let tag = "\"abc123\"";
        assert!(etag_matches(tag, tag), "the simple case must match");
        assert!(
            etag_matches("*", tag),
            "`*` matches any current representation"
        );
        assert!(
            etag_matches("\"other\", \"abc123\"", tag),
            "If-None-Match is a LIST — a naive equality check answers 200 to a well-formed \
             conditional request and the validator never fires"
        );
        assert!(
            etag_matches("W/\"abc123\", \"other\"", "W/\"abc123\""),
            "a weak validator in list form must still be matched by exact string equality \
             against the current (also-weak) tag"
        );
        assert!(
            !etag_matches("\"stale\"", tag),
            "a stale validator must NOT match"
        );
        assert!(!etag_matches("", tag), "an empty header must not match");
    }
}
