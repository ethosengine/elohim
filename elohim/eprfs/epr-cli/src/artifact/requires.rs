//! Canonical Requires bytes. Semantics/admission belong to host_contract, not put.
use std::path::Path;

use eprfs_core::BlobCid;
use eprfs_peer::Bytes;

use super::{ArtifactError, ArtifactResult};

/// Sorted object keys, compact UTF-8, unchanged arrays and only plain-decimal u64 numbers.
/// Absence means no claim, not "requires nothing". The document remains local in part 1.
pub(super) fn read_requires(path: &Path) -> ArtifactResult<(BlobCid, Bytes)> {
    let input = std::fs::read(path)?;
    check_numbers(&input)?;
    let mut value: serde_json::Value = serde_json::from_slice(&input)?;
    if !value.is_array() {
        return Err(ArtifactError::InvalidArguments(
            "--requires must contain a JSON array".into(),
        ));
    }
    value.sort_all_objects();
    let bytes = Bytes::from(serde_json::to_vec(&value)?);
    Ok((BlobCid::compute_bytes(&bytes), bytes))
}

/// Tracks JSON Pointer components while scanning original tokens. This is only a lexical
/// number check: serde_json still validates the entire grammar afterwards. Inspecting Value
/// would lose exponent spelling, negative zero and numbers overwritten by duplicate keys.
enum Container {
    Array(usize),
    Object { key: String, expecting_key: bool },
}

fn check_numbers(input: &[u8]) -> ArtifactResult<()> {
    let mut stack = Vec::new();
    let mut cursor = 0;
    while let Some(&byte) = input.get(cursor) {
        match byte {
            b'"' => {
                let start = cursor;
                cursor += 1;
                while let Some(&next) = input.get(cursor) {
                    cursor += 1;
                    if next == b'\\' {
                        cursor = (cursor + 1).min(input.len());
                    } else if next == b'"' {
                        break;
                    }
                }
                if let Some(Container::Object { key, expecting_key }) = stack.last_mut() {
                    if *expecting_key {
                        *key = serde_json::from_slice(&input[start..cursor])?;
                        *expecting_key = false;
                    }
                }
                continue;
            }
            b'[' => stack.push(Container::Array(0)),
            b'{' => stack.push(Container::Object {
                key: String::new(),
                expecting_key: true,
            }),
            b']' | b'}' => {
                stack.pop();
            }
            b',' => match stack.last_mut() {
                Some(Container::Array(index)) => *index += 1,
                Some(Container::Object { expecting_key, .. }) => *expecting_key = true,
                None => {}
            },
            b'0'..=b'9' | b'-' | b'+' | b'.' => {
                let start = cursor;
                while input
                    .get(cursor)
                    .is_some_and(|b| !b.is_ascii_whitespace() && !b",]}:".contains(b))
                {
                    cursor += 1;
                }
                let number = String::from_utf8_lossy(&input[start..cursor]);
                if !number.bytes().all(|b| b.is_ascii_digit())
                    || (number.len() > 1 && number.starts_with('0'))
                    || number.parse::<u64>().is_err()
                {
                    let path = stack
                        .iter()
                        .map(|part| match part {
                            Container::Array(index) => format!("/{index}"),
                            Container::Object { key, .. } => {
                                format!("/{}", key.replace('~', "~0").replace('/', "~1"))
                            }
                        })
                        .collect();
                    return Err(ArtifactError::RequiresNumber {
                        path,
                        number: number.into_owned(),
                    });
                }
                continue;
            }
            _ => {}
        }
        cursor += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_requires_plain_decimal_u64_vectors_with_typed_paths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("requires.json");
        for number in ["0", "1", "18446744073709551615"] {
            let input = format!(r#"[{{"predicate":{{"value":[{number}]}}}}]"#);
            std::fs::write(&path, &input).unwrap();
            assert_eq!(read_requires(&path).unwrap().1.as_ref(), input.as_bytes());
        }
        for number in [
            "1.0",
            "1e0",
            "-0",
            "-1",
            "01",
            "18446744073709551616",
            "+1",
            "1E0",
        ] {
            std::fs::write(
                &path,
                format!(r#"[{{"predicate":{{"value":[{number}]}}}}]"#),
            )
            .unwrap();
            match read_requires(&path).unwrap_err() {
                ArtifactError::RequiresNumber {
                    path,
                    number: found,
                } => {
                    assert_eq!(path, "/0/predicate/value/0");
                    assert_eq!(found, number);
                }
                other => panic!("expected typed number refusal, got {other}"),
            }
        }
    }

    #[test]
    fn artifact_requires_number_scan_preserves_strings_paths_and_duplicate_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("requires.json");
        std::fs::write(&path, r#"[{"text":"1e0 \\"},"01","-0",{"a/~":[0,1e0]}]"#).unwrap();
        assert!(
            matches!(read_requires(&path), Err(ArtifactError::RequiresNumber { path, .. }) if path == "/3/a~1~0/1")
        );
        std::fs::write(&path, r#"[{"n":1.0,"n":1}]"#).unwrap();
        assert!(
            matches!(read_requires(&path), Err(ArtifactError::RequiresNumber { path, .. }) if path == "/0/n")
        );
        std::fs::write(&path, r#"["1.0","-0","01","18446744073709551616"]"#).unwrap();
        assert!(read_requires(&path).is_ok());
    }
}
