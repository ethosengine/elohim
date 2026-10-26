//! `file://` URIs to and from absolute paths.
//!
//! Clients may percent-encode a path differently from each other and from the server, so every
//! comparison in this crate goes through [`path_key`]: the decoded absolute path.

use std::path::{Path, PathBuf};

/// Encode an absolute path as a `file://` URI. Unreserved characters and `/` stay literal;
/// every other byte is percent-encoded.
#[must_use]
pub fn path_to_uri(path: &Path) -> String {
    let text = path.to_string_lossy();
    let mut uri = String::from("file://");
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

/// Decode a `file://` URI into an absolute path. `None` for any other scheme or a malformed
/// escape.
#[must_use]
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return None;
    }
    let bytes = rest.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = rest.get(index + 1..index + 3)?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok().map(PathBuf::from)
}

/// The comparison key of a URI: its decoded path, or the URI itself for a non-file scheme.
#[must_use]
pub fn path_key(uri: &str) -> String {
    uri_to_path(uri).map_or_else(|| uri.to_owned(), |p| p.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_paths_with_reserved_characters() {
        let path = Path::new("/projects/a b/c:d/é.rs");
        let uri = path_to_uri(path);
        assert_eq!(uri, "file:///projects/a%20b/c%3Ad/%C3%A9.rs");
        assert_eq!(uri_to_path(&uri).as_deref(), Some(path));
    }

    #[test]
    fn differently_encoded_uris_share_one_key() {
        assert_eq!(
            path_key("file:///x/c%3Ad.rs"),
            path_key("file:///x/c:d.rs"),
            "an encoded and a literal colon name the same file"
        );
        assert_eq!(path_key("untitled:1"), "untitled:1");
        assert!(uri_to_path("file:///bad%zz").is_none());
        assert_eq!(
            uri_to_path("file://localhost/a").as_deref(),
            Some(Path::new("/a"))
        );
    }
}
