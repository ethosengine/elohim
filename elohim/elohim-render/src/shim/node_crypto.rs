//! Rust half of the Node `crypto` builtin shim.
//!
//! Exposes a single op, `op_node_crypto_hash`, that backs the shim's
//! `createHash(...).digest()` with a real sha2 implementation. The JS half
//! (`node_crypto.js`) is served as a module by [`crate::shim::loader::NodeShimLoader`]
//! for the `crypto` / `node:crypto` specifiers; it is embedded here so the op and
//! the code that calls it live together.
//!
//! Surface discipline: this is intentionally minimal. `createHash` (sha256 /
//! sha512) is the only crypto primitive proven to execute on the SSR landing
//! render path. New algorithms / ops are added deliberately as render paths
//! prove they need them -- never speculatively stubbed with fake output.

use deno_core::op2;
use deno_error::JsErrorBox;

/// The JS source of the `crypto` shim module, served by `NodeShimLoader` for the
/// `crypto` / `node:crypto` specifiers.
pub(crate) const CRYPTO_SHIM_JS: &str = include_str!("node_crypto.js");

/// Pure hash core: digest `data` with `algorithm`, returning the raw bytes.
///
/// Only the algorithms the bundle actually uses are implemented (sha256,
/// sha512); any other algorithm is a named error rather than a silently
/// wrong-length or fabricated digest. Kept free of deno_core types so it is
/// directly unit-testable without a V8 isolate.
fn hash_bytes(algorithm: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    use sha2::{Digest, Sha256, Sha512};
    match algorithm {
        "sha256" => Ok(Sha256::digest(data).to_vec()),
        "sha512" => Ok(Sha512::digest(data).to_vec()),
        other => Err(format!(
            "elohim-render crypto shim: hash algorithm '{other}' is not implemented \
             (sha256 / sha512 only)"
        )),
    }
}

/// Hash `data` with `algorithm` and return the raw digest bytes.
///
/// Backs the JS shim's `createHash(algorithm).update(data).digest()`.
#[op2]
#[buffer]
fn op_node_crypto_hash(
    #[string] algorithm: &str,
    #[buffer] data: &[u8],
) -> Result<Vec<u8>, JsErrorBox> {
    hash_bytes(algorithm, data).map_err(JsErrorBox::generic)
}

deno_core::extension!(node_crypto_ext, ops = [op_node_crypto_hash],);

#[cfg(test)]
mod tests {
    use super::*;

    /// sha256("hello") -- the canonical vector the render-level golden test also
    /// asserts, verified here at the hash-core level independent of V8.
    #[test]
    fn hashes_sha256_known_vector() {
        let digest = hash_bytes("sha256", b"hello").expect("sha256 ok");
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn hashes_sha512_len() {
        let digest = hash_bytes("sha512", b"hello").expect("sha512 ok");
        assert_eq!(digest.len(), 64, "sha512 digest is 64 bytes");
    }

    #[test]
    fn unknown_algorithm_errors_named() {
        let err = hash_bytes("md5", b"hello").expect_err("md5 unsupported");
        assert!(err.contains("md5"), "error names the algorithm: {err}");
        assert!(
            err.contains("not implemented"),
            "error is a clear not-implemented message: {err}"
        );
    }
}
