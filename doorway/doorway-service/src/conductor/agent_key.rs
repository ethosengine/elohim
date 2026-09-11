//! The one canonical string form of a Holochain agent key, and the one place
//! that normalizes a legacy form back onto it.
//!
//! # Why this module exists
//!
//! A doorway hands agent keys to three different kinds of reader: its own
//! registry (a `DashMap` keyed by the string), a person's JWT (an opaque claim
//! that only has to round-trip), and **elohim-storage's grant surface**, which
//! does `AgentPubKey::try_from(recipient)` and refuses anything else
//! (`elohim/elohim-storage/src/api/compute_grants.rs`). Only the third reader
//! has an opinion — and its opinion is the protocol's: the canonical agent
//! identity string is the HoloHash multibase form, `u` + base64url-nopad of the
//! raw 39 bytes, i.e. `uhCAk…`.
//!
//! The provisioner used to emit BARE base64 (`hCAk…`, no `u`), which every
//! pass-through reader tolerated and the grant surface refused with
//! `"recipient must be a Holochain agent key"` — a 500 on every hosted
//! registration's promise leg. Bare base64 is the drift, not the contract.
//!
//! # The two functions, and why both
//!
//! [`canonical_agent_key`] is for PRODUCERS holding raw bytes: there is exactly
//! one right answer and no decision to make. [`normalize_agent_key`] is for
//! READERS handed a string of unknown vintage — a Mongo row written before this
//! module existed, a JWT minted by an older doorway, an operator's hand-typed
//! admin query. It maps any recognizable encoding of a 39-byte key onto the
//! canonical form and **passes anything else through untouched**.
//!
//! Untouched is deliberate. Dev-mode mints placeholder identities that are not
//! keys at all (`uhCAk-dev-mode-agent-key`), and an admin lookup of a string
//! that is not a key should miss, not match something else. A normalizer that
//! "fixed" unrecognized input would turn an honest absence into a wrong answer.

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;

/// Raw byte length of a Holochain `AgentPubKey` (3-byte prefix + 32-byte core +
/// 4-byte location). A decode that yields any other length is not a key, and is
/// left alone rather than re-encoded into a plausible-looking wrong answer.
pub const AGENT_PUB_KEY_RAW_LEN: usize = 39;

/// The multibase tag Holochain's `HoloHashB64` writes and `AgentPubKey::try_from`
/// requires. Its absence IS the bug this module closes.
const MULTIBASE_TAG: char = 'u';

/// The canonical string for a raw agent key: `u` + base64url-nopad(39 bytes).
///
/// This is what `holo_hash`'s own `Display`/`HoloHashB64` produces, so a string
/// from here parses back through `AgentPubKey::try_from` by construction. Every
/// producer that holds raw key bytes must use this and nothing else.
pub fn canonical_agent_key(raw: &[u8]) -> String {
    format!("{MULTIBASE_TAG}{}", URL_SAFE_NO_PAD.encode(raw))
}

/// Decode a bare (untagged) base64 agent key in any of the four encodings a
/// doorway has historically written, returning `Some(raw)` ONLY at the exact
/// agent-key length.
///
/// The alphabet is tried url-safe first because that is what the provisioner and
/// discovery wrote; `STANDARD` is included because the startup agent-discovery
/// walk registers that encoding too. Padding variants are accepted on read and
/// never written.
fn decode_bare(s: &str) -> Option<Vec<u8>> {
    // Concrete engines, not `dyn Engine`: base64's trait has generic methods and
    // is not dyn-compatible, so the alphabets are tried by value.
    let attempts = [
        URL_SAFE_NO_PAD.decode(s),
        URL_SAFE.decode(s),
        STANDARD_NO_PAD.decode(s),
        STANDARD.decode(s),
    ];
    attempts
        .into_iter()
        .flatten()
        .find(|raw| raw.len() == AGENT_PUB_KEY_RAW_LEN)
}

/// Map any recognizable encoding of an agent key onto the canonical `uhCAk…`
/// form; pass everything else through byte-for-byte.
///
/// Tolerant by design — see the module note. Three outcomes, and the third is
/// the load-bearing one:
///
/// 1. already canonical (`u` + a decodable 39-byte body) → returned unchanged,
/// 2. bare base64 of 39 bytes (`hCAk…`, either alphabet) → re-emitted canonical,
/// 3. anything else (placeholders, truncated fixtures, non-keys) → unchanged.
///
/// Idempotent: `normalize(normalize(x)) == normalize(x)` for every input.
pub fn normalize_agent_key(key: &str) -> String {
    if let Some(body) = key.strip_prefix(MULTIBASE_TAG) {
        // A tagged string is already claiming to be canonical. Re-encoding a
        // decodable body would be a no-op and re-encoding an undecodable one
        // would corrupt a placeholder, so tagged input is never rewritten.
        let _ = body;
        return key.to_string();
    }
    match decode_bare(key) {
        Some(raw) => canonical_agent_key(&raw),
        None => key.to_string(),
    }
}

/// Is this string a recognizable encoding of an agent key at all?
///
/// Guards the alias scan on the registry's miss path: a string that is not a key
/// in ANY encoding can have no alias, so scanning for one would burn the map for
/// a guaranteed miss. Placeholders (`uhCAk-dev-mode-agent-key`) answer `false`.
pub fn is_agent_key_form(key: &str) -> bool {
    match key.strip_prefix(MULTIBASE_TAG) {
        Some(body) => decode_bare(body).is_some(),
        None => decode_bare(key).is_some(),
    }
}

/// Every string form of one raw key that a reader might hold, canonical FIRST.
///
/// The registry deliberately indexes an agent under all of these so a JWT or a
/// Mongo row written by any doorway vintage still routes. This returns forms,
/// not identities — capacity accounting must never count its length (that is
/// exactly the double-count this module's sibling fix closes).
pub fn lookup_forms(raw: &[u8]) -> Vec<String> {
    let mut forms = vec![canonical_agent_key(raw)];
    for extra in [URL_SAFE_NO_PAD.encode(raw), STANDARD.encode(raw)] {
        if !forms.contains(&extra) {
            forms.push(extra);
        }
    }
    forms
}

/// The raw 39 bytes behind ANY spelling of a HoloHash string, or `None` when the
/// string is not a HoloHash in any encoding.
///
/// This is the ONE decoder. Every reader that needs BYTES rather than a string
/// goes through it, so no caller has to pick a base64 alphabet — picking one is
/// exactly how `worker::zome_call` came to decode with `STANDARD` what
/// `services::discovery` encodes with `URL_SAFE_NO_PAD`, which fails on the ~80%
/// of keys whose url-safe spelling contains a `-` or `_`.
///
/// It is defined as *normalize, then read the canonical body*, so it cannot
/// disagree with [`normalize_agent_key`] about what a string means: if the
/// normalizer maps two spellings onto one canonical form, this returns identical
/// bytes for both, by construction rather than by two implementations agreeing.
///
/// An `AgentPubKey` and a `DnaHash` share this length (39 raw bytes) and this
/// multibase tag — only the 3-byte prefix differs (`uhCAk…` vs `uhC0k…`) — so a
/// cell id's BOTH halves decode here.
pub fn decode_key_bytes(key: &str) -> Option<Vec<u8>> {
    let canonical = normalize_agent_key(key);
    let body = canonical.strip_prefix(MULTIBASE_TAG)?;
    URL_SAFE_NO_PAD
        .decode(body)
        .ok()
        .filter(|raw| raw.len() == AGENT_PUB_KEY_RAW_LEN)
}

/// Every string form of the identity a READER's string names, canonical first.
///
/// The sibling of [`lookup_forms`] for callers holding a string rather than raw
/// bytes — an operator's hand-typed admin query, a Mongo filter, a JWT claim.
/// Widening an equality match to an `$in` over this is what lets a row written
/// by a pre-canonical doorway be found by its canonical spelling and the reverse.
///
/// Always a SUPERSET of the plain equality it replaces: the caller's own string
/// is present whatever it is, so a string that is not a key (a dev-mode
/// placeholder) still matches its own row exactly and nothing else.
pub fn lookup_forms_of(key: &str) -> Vec<String> {
    match decode_key_bytes(key) {
        Some(raw) => {
            let mut forms = lookup_forms(&raw);
            if !forms.iter().any(|form| form == key) {
                forms.push(key.to_string());
            }
            forms
        }
        None => vec![key.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A REAL 39-byte agent key, minted the way a conductor mints one.
    ///
    /// Hand-rolled bytes are not enough: `AgentPubKey::try_from` verifies the
    /// 4-byte DHT location trailer against the 32-byte core and answers
    /// `BadChecksum` otherwise — so a fixture built by hand would prove the
    /// normalizer against a string the grant surface would refuse anyway.
    fn raw_key(fill: u8) -> Vec<u8> {
        holo_hash::AgentPubKey::from_raw_32(vec![fill; 32])
            .get_raw_39()
            .to_vec()
    }

    #[test]
    fn canonical_form_is_multibase_tagged_and_parses_as_an_agent_key() {
        let canonical = canonical_agent_key(&raw_key(7));
        assert!(
            canonical.starts_with("uhCAk"),
            "canonical form must carry the multibase tag: {canonical}"
        );
        holo_hash::AgentPubKey::try_from(canonical.as_str())
            .expect("canonical form must parse through the same call the grant surface makes");
    }

    #[test]
    fn bare_base64_is_normalized_onto_the_canonical_form() {
        let raw = raw_key(9);
        let bare = URL_SAFE_NO_PAD.encode(&raw);
        assert!(
            bare.starts_with("hCAk"),
            "fixture is the drifted form: {bare}"
        );
        assert_eq!(normalize_agent_key(&bare), canonical_agent_key(&raw));
        holo_hash::AgentPubKey::try_from(normalize_agent_key(&bare).as_str())
            .expect("a normalized legacy key must satisfy the grant surface");
    }

    #[test]
    fn standard_alphabet_bare_keys_normalize_too() {
        // The startup discovery walk registers STANDARD-encoded keys; a fleet row
        // in that form must still resolve.
        let raw = raw_key(0xFB); // forces + and / into the STANDARD alphabet
        let bare = STANDARD.encode(&raw);
        assert_eq!(normalize_agent_key(&bare), canonical_agent_key(&raw));
    }

    #[test]
    fn an_already_canonical_key_is_returned_unchanged() {
        let canonical = canonical_agent_key(&raw_key(3));
        assert_eq!(normalize_agent_key(&canonical), canonical);
    }

    #[test]
    fn normalization_is_idempotent() {
        let raw = raw_key(42);
        let once = normalize_agent_key(&URL_SAFE_NO_PAD.encode(&raw));
        assert_eq!(normalize_agent_key(&once), once);
    }

    #[test]
    fn a_non_key_string_is_passed_through_untouched() {
        // Dev-mode placeholders and fixtures are NOT keys. Rewriting them would
        // turn an honest miss into a wrong match.
        for not_a_key in [
            "uhCAk-dev-mode-agent-key",
            "uhCAk...",
            "uhCAk-test",
            "human-matthew-manager",
            "",
        ] {
            assert_eq!(
                normalize_agent_key(not_a_key),
                not_a_key,
                "non-key input must survive byte-for-byte"
            );
        }
    }

    #[test]
    fn a_wrong_length_decode_is_not_treated_as_a_key() {
        // 32 bytes decodes cleanly as base64 but is not an agent key.
        let short = URL_SAFE_NO_PAD.encode(vec![0u8; 32]);
        assert_eq!(normalize_agent_key(&short), short);
    }

    #[test]
    fn is_agent_key_form_recognizes_both_vintages_and_refuses_placeholders() {
        let raw = raw_key(11);
        assert!(is_agent_key_form(&canonical_agent_key(&raw)));
        assert!(is_agent_key_form(&URL_SAFE_NO_PAD.encode(&raw)));
        assert!(is_agent_key_form(&STANDARD.encode(&raw)));
        for not_a_key in [
            "uhCAk-dev-mode-agent-key",
            "uhCAk...",
            "",
            "human-matthew-manager",
        ] {
            assert!(
                !is_agent_key_form(not_a_key),
                "{not_a_key} is not a key in any encoding"
            );
        }
    }

    #[test]
    fn every_spelling_decodes_to_the_same_raw_bytes() {
        // The property the worker's cell_id depends on: whichever vintage of
        // string a reader holds, the BYTES it hands the conductor are identical.
        let raw = raw_key(0xFB); // url-safe spelling contains '-' and '_'
        for spelling in [
            canonical_agent_key(&raw),
            URL_SAFE_NO_PAD.encode(&raw),
            STANDARD.encode(&raw),
        ] {
            assert_eq!(
                decode_key_bytes(&spelling).as_deref(),
                Some(raw.as_slice()),
                "spelling {spelling} must decode to the one identity's bytes"
            );
        }
    }

    #[test]
    fn decode_key_bytes_refuses_a_non_key_rather_than_guessing() {
        for not_a_key in [
            "uhCAk-dev-mode-agent-key",
            "uhCAk...",
            "human-matthew-manager",
            "",
        ] {
            assert!(
                decode_key_bytes(not_a_key).is_none(),
                "{not_a_key} has no bytes to hand a conductor"
            );
        }
        // Decodable base64 at the wrong length is still not a key.
        assert!(decode_key_bytes(&URL_SAFE_NO_PAD.encode(vec![0u8; 32])).is_none());
    }

    #[test]
    fn lookup_forms_of_a_string_names_one_identity_in_every_spelling() {
        let raw = raw_key(0xFB);
        let canonical = canonical_agent_key(&raw);
        let legacy = URL_SAFE_NO_PAD.encode(&raw);

        // Asked canonically, the legacy spelling is among the forms — and asked
        // in the legacy spelling, the canonical one is. This is what makes a
        // pre-canonical Mongo row reachable from a canonical admin query.
        assert!(lookup_forms_of(&canonical).contains(&legacy));
        assert!(lookup_forms_of(&legacy).contains(&canonical));
        assert_eq!(lookup_forms_of(&canonical)[0], canonical);
        assert_eq!(lookup_forms_of(&legacy)[0], canonical);
    }

    #[test]
    fn lookup_forms_of_always_contains_the_callers_own_string() {
        // The widening must be a SUPERSET of the equality it replaces: whatever a
        // caller passes, a row spelled exactly that way is still matched.
        let raw = raw_key(5);
        for spelling in [
            canonical_agent_key(&raw),
            URL_SAFE_NO_PAD.encode(&raw),
            STANDARD.encode(&raw),
            "uhCAk-dev-mode-agent-key".to_string(),
            "human-matthew-manager".to_string(),
        ] {
            assert!(
                lookup_forms_of(&spelling).contains(&spelling),
                "{spelling} must still match its own row"
            );
        }
    }

    #[test]
    fn lookup_forms_of_a_non_key_matches_only_itself() {
        // A placeholder has no other spelling; widening it must not start
        // matching unrelated rows.
        assert_eq!(
            lookup_forms_of("uhCAk-dev-mode-agent-key"),
            vec!["uhCAk-dev-mode-agent-key".to_string()]
        );
    }

    #[test]
    fn lookup_forms_of_two_different_keys_never_overlap() {
        let a = lookup_forms_of(&canonical_agent_key(&raw_key(1)));
        let b = lookup_forms_of(&canonical_agent_key(&raw_key(2)));
        assert!(
            !a.iter().any(|form| b.contains(form)),
            "widening must never make one identity's query match another's row"
        );
    }

    #[test]
    fn lookup_forms_lead_with_canonical_and_keep_legacy_encodings() {
        let raw = raw_key(0xFB);
        let forms = lookup_forms(&raw);
        assert_eq!(forms[0], canonical_agent_key(&raw));
        assert!(forms.contains(&URL_SAFE_NO_PAD.encode(&raw)));
        assert!(forms.contains(&STANDARD.encode(&raw)));
        // Distinct forms only — a duplicate here would silently inflate any
        // caller that mistook form-count for agent-count.
        let mut sorted = forms.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), forms.len());
    }
}
