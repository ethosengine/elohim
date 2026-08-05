//! Canonical JSON shared by policy-pin evaluation and the canon lift.
//!
//! The Python pin oracle is:
//!
//! ```python
//! body = {k: v for k, v in row.items() if k not in {"contentHash", "status", "superseded_by"}}
//! json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
//! ```
//!
//! `serde_json` is not byte-identical: it emits non-ASCII literally and leaves U+007F
//! unescaped. Registry rows contain characters such as `·`, `—`, and `≠`, so policy
//! evaluation and content-addressing must share this implementation rather than approximating it.

use serde_yaml::Value;

/// Lifecycle keys excluded from the semantic policy-row pin.
pub const HASH_EXCLUDE_KEYS: [&str; 3] = ["contentHash", "status", "superseded_by"];

/// A value whose Python canonical representation cannot be reproduced safely.
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

/// Canonical bytes for one registry row, excluding lifecycle-only keys.
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
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                out.push_str(&value.to_string());
            } else if let Some(value) = number.as_u64() {
                out.push_str(&value.to_string());
            } else {
                return Err(CanonError::Float(path.to_string()));
            }
        }
        Value::String(value) => write_ascii_string(value, out),
        Value::Sequence(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(item, &format!("{path}[{index}]"), out)?;
            }
            out.push(']');
        }
        Value::Mapping(map) => {
            let mut entries: Vec<(&str, &Value)> = Vec::with_capacity(map.len());
            for (key, value) in map {
                let Value::String(key) = key else {
                    return Err(CanonError::NonStringKey(format!("{path}/{key:?}")));
                };
                entries.push((key.as_str(), value));
            }
            entries.sort_by(|left, right| left.0.cmp(right.0));
            out.push('{');
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_ascii_string(key, out);
                out.push(':');
                write_value(value, &format!("{path}/{key}"), out)?;
            }
            out.push('}');
        }
        Value::Tagged(tagged) => {
            return Err(CanonError::Tagged(format!("{path} (!{})", tagged.tag)));
        }
    }
    Ok(())
}

fn write_ascii_string(value: &str, out: &mut String) {
    out.push('"');
    for character in value.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ' '..='~' => out.push(character),
            _ => {
                let codepoint = character as u32;
                if codepoint < 0x1_0000 {
                    out.push_str(&format!("\\u{codepoint:04x}"));
                } else {
                    let codepoint = codepoint - 0x1_0000;
                    let high = 0xd800 | ((codepoint >> 10) & 0x3ff);
                    let low = 0xdc00 | (codepoint & 0x3ff);
                    out.push_str(&format!("\\u{high:04x}\\u{low:04x}"));
                }
            }
        }
    }
    out.push('"');
}

/// Lowercase, zero-padded hexadecimal.
pub fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(yaml: &str) -> String {
        canonical_json(&serde_yaml::from_str::<Value>(yaml).unwrap()).unwrap()
    }

    fn escaped(codepoint: u32) -> String {
        format!("\\u{codepoint:04x}")
    }

    #[test]
    fn keys_sort_and_separators_are_compact() {
        assert_eq!(
            json("b: 2\na: 1\nc: {z: 1, y: 2}"),
            r#"{"a":1,"b":2,"c":{"y":2,"z":1}}"#
        );
    }

    #[test]
    fn non_ascii_and_del_are_escaped_like_python_ensure_ascii() {
        assert_eq!(
            json("s: \"a \u{b7} b \u{2014} c\""),
            format!(r#"{{"s":"a {} b {} c"}}"#, escaped(0xb7), escaped(0x2014))
        );
        assert_eq!(
            json(r#"s: "\x7f""#),
            format!(r#"{{"s":"{}"}}"#, escaped(0x7f))
        );
    }

    #[test]
    fn astral_characters_become_surrogate_pairs() {
        assert_eq!(
            json(r#"s: "\U0001F600""#),
            format!(r#"{{"s":"{}{}"}}"#, escaped(0xd83d), escaped(0xde00))
        );
    }

    #[test]
    fn a_float_fails_closed() {
        let error =
            canonical_json(&serde_yaml::from_str::<Value>("a: {b: 1.5}").unwrap()).unwrap_err();
        assert_eq!(error, CanonError::Float("/a/b".into()));
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
}
