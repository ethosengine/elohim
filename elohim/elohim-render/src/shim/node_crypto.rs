//! Rust half of the Node `crypto` builtin shim + the WebCrypto global.
//!
//! Two ops live here:
//!
//! - `op_node_crypto_hash` backs the Node `createHash(...).digest()` shim with a
//!   real sha2 implementation. The JS half (`node_crypto.js`) is served as a
//!   module by [`crate::shim::loader::NodeShimLoader`] for the `crypto` /
//!   `node:crypto` specifiers.
//! - `op_elohim_get_random_values` backs the WebCrypto `crypto.getRandomValues`
//!   global with REAL OS entropy (`getrandom`, i.e. the Linux `getrandom(2)`
//!   syscall on this native build). The JS half (`node_webcrypto.js`) is an
//!   INIT-TIME global installer (served via `js = [dir ...]`, like
//!   [`crate::shim::node_globals`] / [`crate::shim::node_buffer`]) that installs
//!   `globalThis.crypto` BEFORE any bundle module evaluates -- a jsbn/brorand-style
//!   CSPRNG probe in the libp2p/multiformats chunk feature-detects
//!   `globalThis.crypto.getRandomValues` at module-eval time and throws
//!   "No secure random number generator found" when it is absent.
//!
//! # One source of truth for `crypto`
//!
//! `globalThis.crypto` (installed by `node_webcrypto.js`) and the `node:crypto`
//! module's `webcrypto` export are the SAME object: `node_crypto.js` binds
//! `webcrypto = globalThis.crypto`. So there is exactly one WebCrypto object across
//! the global surface and the module surface, and `crypto.subtle` (a loud, named
//! stub for now -- see below) is likewise single-sourced.
//!
//! # Entropy honesty (non-negotiable for a crypto path)
//!
//! `getRandomValues` MUST be real entropy or absent -- NEVER a PRNG masquerade.
//! This op backs libp2p / multiformats key + nonce generation on the render path,
//! where a predictable stream would be a security defect, not a rendering quirk.
//! `getrandom::fill` reads the OS CSPRNG and errors (rather than degrading) if the
//! source is unavailable.
//!
//! # Surface discipline
//!
//! Intentionally minimal, mirroring [`crate::shim::node_builtins`]: `createHash`
//! (sha256 / sha512) and `getRandomValues` / `randomUUID` are the only crypto
//! primitives proven to execute on the SSR landing render path. `crypto.subtle.*`
//! is a set of LOUD, NAMED stubs -- a render-path call throws a message naming the
//! operation, so an unproven path fails clearly instead of silently. New
//! algorithms / ops are added deliberately as render paths prove they need them --
//! never speculatively stubbed with fake output.

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

/// Pure entropy core: return `len` bytes from the OS CSPRNG.
///
/// REAL entropy from the OS CSPRNG via `getrandom::fill` -- never a PRNG. On this
/// native build (`RUSTFLAGS=""`) that is the Linux `getrandom(2)` syscall. An
/// unavailable entropy source is a hard error, not a silent degrade to
/// predictable bytes. Enforces the WebCrypto 65536-byte per-call quota. Kept free
/// of deno_core types so it is directly unit-testable without a V8 isolate.
fn random_bytes(len: usize) -> Result<Vec<u8>, String> {
    // Defense in depth: the JS shim already rejects len > 65536 (WebCrypto quota),
    // but never trust the boundary -- an unbounded allocation from a JS-supplied
    // length is the kind of footgun this rung exists to avoid.
    if len > 65536 {
        return Err(
            "elohim-render webcrypto shim: getRandomValues length exceeds the \
             65536-byte quota"
                .to_string(),
        );
    }
    let mut buf = vec![0u8; len];
    getrandom::fill(&mut buf).map_err(|e| {
        format!("elohim-render webcrypto shim: OS entropy source (getrandom) failed: {e}")
    })?;
    Ok(buf)
}

/// Return `len` bytes of cryptographically-secure OS entropy.
///
/// Backs the WebCrypto `crypto.getRandomValues(typedArray)` global (and, through
/// it, `crypto.randomUUID()`). The JS half (`node_webcrypto.js`) copies the
/// returned bytes into the caller's typed array in place and enforces the
/// WebCrypto 65536-byte per-call quota, so `len` here is already bounded; it is
/// re-bounded defensively in [`random_bytes`] regardless. An unavailable entropy
/// source surfaces as a named `JsErrorBox`, never predictable bytes.
#[op2]
#[buffer]
fn op_elohim_get_random_values(#[number] len: usize) -> Result<Vec<u8>, JsErrorBox> {
    random_bytes(len).map_err(JsErrorBox::generic)
}

deno_core::extension!(
    node_crypto_ext,
    ops = [op_node_crypto_hash, op_elohim_get_random_values],
    js = [dir "src/shim", "node_webcrypto.js"],
);

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

    /// `random_bytes` returns exactly the requested length (WebCrypto fills the
    /// whole typed array), including the zero-length edge.
    #[test]
    fn random_bytes_preserves_length() {
        for len in [0usize, 1, 4, 16, 32, 256, 65536] {
            let bytes = random_bytes(len).expect("entropy ok");
            assert_eq!(bytes.len(), len, "returns exactly {len} bytes");
        }
    }

    /// Two independent 32-byte draws must differ. A constant/zero-filled result
    /// (the failure mode where the op silently returns un-randomized bytes) would
    /// make these equal. Statistical smoke: the odds of two real 32-byte CSPRNG
    /// draws colliding are ~2^-256.
    #[test]
    fn random_bytes_are_non_constant_across_calls() {
        let a = random_bytes(32).expect("entropy ok");
        let b = random_bytes(32).expect("entropy ok");
        assert_ne!(a, b, "two 32-byte CSPRNG draws must differ");
        // And guard the specific all-zero degrade explicitly.
        assert_ne!(
            a,
            vec![0u8; 32],
            "draw must not be all-zero (un-filled) bytes"
        );
    }

    /// A draw beyond the WebCrypto 65536-byte quota is a named error, not an
    /// unbounded allocation.
    #[test]
    fn random_bytes_rejects_over_quota() {
        let err = random_bytes(65537).expect_err("over-quota must error");
        assert!(err.contains("quota"), "error names the quota: {err}");
    }
}
