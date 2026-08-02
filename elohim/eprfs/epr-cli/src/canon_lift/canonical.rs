//! Canonical JSON, byte-identical to the pin oracle.
//!
//! The `.claude/epr-meta` registries pin every row with a `sha256` over ONE canonicalization —
//! `_lib.epr_meta.policy_content_hash()`, which is exactly:
//!
//! ```python
//! body = {k: v for k, v in row.items() if k not in {"contentHash", "status", "superseded_by"}}
//! json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
//! ```
//!
//! The brit lift must produce those SAME bytes or it is a rewrite, not a move (the P0.4 gate,
//! `elohim/epr/tests/concern_canon_liftability.rs`). So this module re-implements CPython's
//! `json.dumps` on that exact flag set — not serde_json's, which differs in two ways that matter:
//! serde_json emits non-ASCII literally (no `ensure_ascii`) and leaves `U+007F` unescaped, while
//! CPython's `ESCAPE_ASCII = re.compile(r'([\\"]|[^\ -~])')` escapes everything outside
//! `0x20..=0x7E`. The registries are full of `·`, `—` and `≠`, so that difference is not
//! theoretical: reaching for `serde_json::to_string` here would silently mint a different address
//! for nearly every row.
//!
//! Key order: CPython's `sort_keys=True` sorts by codepoint; Rust's `str::cmp` is UTF-8 byte
//! order, which is the same order for all of Unicode (UTF-8 is order-preserving).
//!
//! FAIL-CLOSED on anything whose two renderings could disagree. A float would have to match
//! CPython's `repr()` (`1.0` vs Rust's `1`, `1e+20` vs `100000000000000000000`) — a divergence
//! that would move a CID with no visible cause — so a float is an ERROR, not a best-effort
//! encoding. No registry row carries one today; if one appears, this says so out loud.

use serde_yaml::Value;

/// Keys the pin deliberately excludes: lifecycle state that moves without the semantics moving.
/// Mirrors `_lib.epr_meta._HASH_EXCLUDE_KEYS` exactly.
pub const HASH_EXCLUDE_KEYS: [&str; 3] = ["contentHash", "status", "superseded_by"];

/// A value the pin oracle's canonicalization cannot be reproduced for, byte for byte.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CanonError {
    #[error("non-string mapping key at `{0}` — `json.dumps` would coerce it; refusing to guess")]
    NonStringKey(String),
    #[error(
        "float at `{0}` — CPython `repr()` and Rust `{{}}` disagree on float rendering, so the \
         canonical bytes (and therefore the CID) would silently diverge from the sha256 pin"
    )]
    Float(String),
    #[error("YAML tag at `{0}` — a tagged scalar has no `json.dumps` equivalent")]
    Tagged(String),
}

/// The canonical body of one registry row: the row minus [`HASH_EXCLUDE_KEYS`], rendered as the
/// pin oracle's canonical JSON. Pure ASCII by construction, so the `String` and its bytes are
/// the same thing.
pub fn canonical_body(row: &serde_yaml::Mapping) -> Result<Vec<u8>, CanonError> {
    let mut body = row.clone();
    for key in HASH_EXCLUDE_KEYS {
        body.remove(Value::String(key.to_string()));
    }
    Ok(canonical_json(&Value::Mapping(body))?.into_bytes())
}

/// `json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)`.
pub fn canonical_json(value: &Value) -> Result<String, CanonError> {
    let mut out = String::new();
    write_value(value, "", &mut out)?;
    Ok(out)
}

fn write_value(value: &Value, path: &str, out: &mut String) -> Result<(), CanonError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                out.push_str(&i.to_string());
            } else if let Some(u) = n.as_u64() {
                out.push_str(&u.to_string());
            } else {
                return Err(CanonError::Float(path.to_string()));
            }
        }
        Value::String(s) => write_ascii_string(s, out),
        Value::Sequence(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, &format!("{path}[{i}]"), out)?;
            }
            out.push(']');
        }
        Value::Mapping(map) => {
            let mut entries: Vec<(&str, &Value)> = Vec::with_capacity(map.len());
            for (key, val) in map {
                let Value::String(key) = key else {
                    return Err(CanonError::NonStringKey(format!("{path}/{key:?}")));
                };
                entries.push((key.as_str(), val));
            }
            entries.sort_by(|a, b| a.0.cmp(b.0));
            out.push('{');
            for (i, (key, val)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_ascii_string(key, out);
                out.push(':');
                write_value(val, &format!("{path}/{key}"), out)?;
            }
            out.push('}');
        }
        Value::Tagged(tagged) => {
            return Err(CanonError::Tagged(format!("{path} (!{})", tagged.tag)));
        }
    }
    Ok(())
}

/// CPython's `py_encode_basestring_ascii`: escape `\` and `"`, use the short forms for the five
/// named control characters, `\u00xx` for the rest below `0x20`, pass through `0x20..=0x7E`
/// literally, and `\uXXXX` (surrogate pair above the BMP) for EVERYTHING at or above `0x7F`.
fn write_ascii_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ' '..='~' => out.push(c),
            _ => {
                let n = c as u32;
                if n < 0x1_0000 {
                    out.push_str(&format!("\\u{n:04x}"));
                } else {
                    let n = n - 0x1_0000;
                    let hi = 0xd800 | ((n >> 10) & 0x3ff);
                    let lo = 0xdc00 | (n & 0x3ff);
                    out.push_str(&format!("\\u{hi:04x}\\u{lo:04x}"));
                }
            }
        }
    }
    out.push('"');
}

/// Lowercase hex of a byte slice. Hand-rolled rather than pulling a `hex` dependency into a
/// tooling crate for sixteen characters — the same call `eprfs_core::BlobCid::short_fingerprint`
/// makes.
pub fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(yaml: &str) -> String {
        canonical_json(&serde_yaml::from_str::<Value>(yaml).unwrap()).unwrap()
    }

    #[test]
    fn keys_sort_and_separators_are_compact() {
        assert_eq!(
            json("b: 2\na: 1\nc: {z: 1, y: 2}"),
            r#"{"a":1,"b":2,"c":{"y":2,"z":1}}"#
        );
    }

    /// The three expectations below build their `\\uXXXX` escapes programmatically. Writing the
    /// escape text literally would make the test source itself carry the character it is
    /// asserting is NOT emitted literally — which is exactly the confusion this module exists to
    /// prevent, and which round-trips badly through editors that normalize escapes.
    fn esc(codepoint: u32) -> String {
        format!("\\u{codepoint:04x}")
    }

    #[test]
    fn non_ascii_is_escaped_like_ensure_ascii() {
        // U+00B7 and U+2014 both appear verbatim in the live canon rows; serde_json would emit
        // them literally and mint a different address for nearly every row.
        assert_eq!(
            json("s: \"a \u{b7} b \u{2014} c\""),
            format!(r#"{{"s":"a {} b {} c"}}"#, esc(0xb7), esc(0x2014))
        );
    }

    #[test]
    fn del_and_control_characters_follow_cpython_not_serde_json() {
        // U+007F is outside CPython's `[\ -~]` pass-through range and IS escaped there, while
        // serde_json leaves it literal. This is the second `ensure_ascii` divergence.
        assert_eq!(json(r#"s: "\x7f""#), format!(r#"{{"s":"{}"}}"#, esc(0x7f)));
        // A control character with no short form takes `\u00xx`; `\b` and `\f` take theirs.
        assert_eq!(
            json(r#"s: "\x01\b\f""#),
            format!(r#"{{"s":"{}\b\f"}}"#, esc(1))
        );
    }

    #[test]
    fn astral_characters_become_surrogate_pairs() {
        assert_eq!(
            json(r#"s: "\U0001F600""#),
            format!(r#"{{"s":"{}{}"}}"#, esc(0xd83d), esc(0xde00))
        );
    }

    #[test]
    fn quotes_and_backslashes_escape() {
        assert_eq!(json(r#"s: "a\"b\\c""#), r#"{"s":"a\"b\\c"}"#);
    }

    #[test]
    fn scalars_render_like_json_dumps() {
        assert_eq!(
            json("a: true\nb: false\nc: null\nd: 0\ne: -7"),
            r#"{"a":true,"b":false,"c":null,"d":0,"e":-7}"#
        );
        assert_eq!(json("a: [1, two, [3]]"), r#"{"a":[1,"two",[3]]}"#);
    }

    #[test]
    fn a_float_fails_closed_rather_than_guessing_cpython_repr() {
        let err =
            canonical_json(&serde_yaml::from_str::<Value>("a: {b: 1.5}").unwrap()).unwrap_err();
        assert_eq!(err, CanonError::Float("/a/b".into()));
    }

    #[test]
    fn canonical_body_drops_exactly_the_lifecycle_keys() {
        let row: serde_yaml::Mapping = serde_yaml::from_str(
            "id: x\nversion: 1\ncontentHash: sha256:deadbeef\nstatus: superseded\n\
             superseded_by: y@2\ntitle: T",
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(canonical_body(&row).unwrap()).unwrap(),
            r#"{"id":"x","title":"T","version":1}"#
        );
    }

    #[test]
    fn hex_is_lowercase_and_zero_padded() {
        assert_eq!(hex_lower(&[0x00, 0x0f, 0xff]), "000fff");
    }
}
