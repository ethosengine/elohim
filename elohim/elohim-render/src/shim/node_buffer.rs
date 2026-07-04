//! Rust half of the Node `Buffer` shim.
//!
//! The Angular server bundle (via bundled `ws` / `multiformats` / libp2p code)
//! uses a Node `Buffer` in two shapes, BOTH of which must resolve to ONE real
//! class:
//!
//! - a **global** `Buffer` -- e.g. `ws`'s `EMPTY_BUFFER = Buffer.alloc(0)` and
//!   `Buffer.from([0,0,255,255])` evaluate at module-eval time (top-level consts)
//!   during the SSR render's module-graph load. A bare deno_core runtime has no
//!   `Buffer` global, so this was the render wall
//!   (`ReferenceError: Buffer is not defined` at the ws chunk).
//! - a named export `Buffer` from the `buffer` / `node:buffer` module -- e.g.
//!   `import { Buffer } from "buffer"` in multiformats/libp2p chunks. That import
//!   is served by [`crate::shim::node_builtins`], whose `("buffer","Buffer")`
//!   member is now REAL and simply re-exports `globalThis.Buffer` (one source of
//!   truth: the global installed by the JS half here).
//!
//! # Architecture: a sibling ext (like [`crate::shim::node_crypto`])
//!
//! The GLOBAL must exist at runtime-init time (before ANY bundle module body runs
//! -- the ws const is module-eval-time). deno_core extension `js` files run at
//! isolate construction, so the global is installed there: [`node_buffer.js`] is
//! served via `js = [dir "src/shim", "node_buffer.js"]` (NOT as a loader module),
//! exactly the init-time install path [`crate::shim::node_globals`] uses. The
//! `buffer` MODULE re-exports that global rather than defining a second class, so
//! `import { Buffer }` and the global `Buffer` are the SAME constructor and
//! `x instanceof Buffer` is coherent across both surfaces.
//!
//! # Why Rust ops back base64 / hex (but not utf8)
//!
//! `Buffer` is a `Uint8Array` subclass; utf8 round-trips through deno_core's
//! existing `Deno.core.encode` / `Deno.core.decode` (op_encode/op_decode) and
//! latin1 through `Deno.core.encodeBinaryString`. deno_core core provides NO
//! base64 or hex primitive (verified against deno_core 0.339 `01_core.js`: only
//! encode/decode/encodeBinaryString are exposed; no `atob`/`btoa`). Rather than
//! hand-roll a base64 alphabet in JS (subtle padding/whitespace bugs are exactly
//! the silent-wrong class this rung is meant to avoid), the four codecs are
//! byte-exact pure-Rust ops, unit-tested here independent of V8 -- mirroring how
//! `node_crypto`'s `createHash` is a real sha2 op, not a JS fabrication.
//!
//! # Used-surface discipline (mirrors node_builtins / node_crypto)
//!
//! Implemented for REAL is exactly the surface the bundle exercises at
//! module-eval / SSR-render time:
//!
//! Static: `from(string,enc | ArrayBuffer[,off,len] | TypedArray/Buffer | Array)`,
//! `alloc(size[,fill[,enc]])`, `allocUnsafe`, `allocUnsafeSlow`, `concat`,
//! `isBuffer`, `byteLength`, `isEncoding`.
//! Instance: `toString(enc[,start[,end]])`, `slice`/`subarray` (memory-SHARING,
//! per Node -- NOT Uint8Array's copying `slice`), `equals`, `copy`, `fill`,
//! `write`, `toJSON`, and the inherited `length` / `byteOffset` / `buffer`.
//! Encodings: `utf8`/`utf-8`, `hex`, `base64`, `base64url`, `latin1`/`binary`,
//! `ascii`. (`ucs2`/`utf16le` are NOT in the bundle's used surface -- absent, per
//! the honesty rule.)
//!
//! LOUD, NAMED stubs (used by the bundle but off the SSR boot path -- the binary
//! wire-protocol read/write integer family lives in `ws` frame framing
//! (`Sender.frame` -> `writeUInt16BE`) and protobuf varint code, which runs at
//! CONNECTION time, never during a single-shot SSR render -- the same class as
//! `zlib`'s connection-time loud stubs). Getting Node's exact `writeUIntBE`
//! offset/return semantics subtly wrong would be silent-wrong on a live wire, so
//! the honest bound is a throw naming the method + this file, NOT a guessed impl.
//! Installed on `Buffer.prototype` so a call on a real `Buffer` instance is loud;
//! a call on a non-Buffer (a custom protobuf `Writer`) uses that object's own
//! method and never consults this prototype.
//!
//! # WARNING: pure ASCII
//!
//! `node_buffer.js` is served through the `js = [dir ...]` macro, which panics at
//! const-eval on any non-ASCII byte. Keep every literal ASCII.

use deno_core::op2;

/// The standard and URL-safe base64 alphabets (RFC 4648).
const B64_STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const HEX_LC: &[u8; 16] = b"0123456789abcdef";

/// Encode `data` as base64. `url_safe` selects the `-_` alphabet and, per Node's
/// `base64url`, emits NO `=` padding; standard base64 pads to a multiple of 4.
///
/// Pure (V8-free) so it is unit-tested directly below.
fn base64_encode(data: &[u8], url_safe: bool) -> String {
    let alphabet = if url_safe { B64_URL } else { B64_STD };
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(alphabet[((n >> 18) & 63) as usize] as char);
        out.push(alphabet[((n >> 12) & 63) as usize] as char);
        match chunk.len() {
            1 => {
                if !url_safe {
                    out.push('=');
                    out.push('=');
                }
            }
            2 => {
                out.push(alphabet[((n >> 6) & 63) as usize] as char);
                if !url_safe {
                    out.push('=');
                }
            }
            _ => {
                out.push(alphabet[((n >> 6) & 63) as usize] as char);
                out.push(alphabet[(n & 63) as usize] as char);
            }
        }
    }
    out
}

/// Decode a base64 string. Lenient like Node: accepts BOTH the standard (`+/`)
/// and URL-safe (`-_`) alphabets (so one decoder serves `base64` and
/// `base64url`), treats `=` as an end-of-data terminator, and skips any other
/// non-alphabet byte (whitespace/newlines in wrapped base64). Well-formed inputs
/// -- the content hashes / keys the bundle decodes -- round-trip exactly.
fn base64_decode(s: &str) -> Vec<u8> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' | b'-' => Some(62),
            b'/' | b'_' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3 + 3);
    let mut acc: u32 = 0;
    let mut bits: u8 = 0;
    for &c in s.as_bytes() {
        if c == b'=' {
            break;
        }
        let v = match val(c) {
            Some(v) => v,
            None => continue,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/// Encode `data` as lowercase hex (Node's `hex` encoding output).
fn hex_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 2);
    for &b in data {
        out.push(HEX_LC[(b >> 4) as usize] as char);
        out.push(HEX_LC[(b & 0x0f) as usize] as char);
    }
    out
}

/// Decode a hex string. Like Node: decode whole byte-pairs, TRUNCATE at the first
/// non-hex character or a trailing odd nibble (never throw -- Node's `hex`
/// decoder silently truncates).
fn hex_decode(s: &str) -> Vec<u8> {
    fn nib(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        match (nib(bytes[i]), nib(bytes[i + 1])) {
            (Some(h), Some(l)) => {
                out.push((h << 4) | l);
                i += 2;
            }
            _ => break,
        }
    }
    out
}

/// Base64-encode `data`; `url_safe` picks the URL-safe alphabet with no padding.
/// Backs the JS shim's `toString("base64" | "base64url")`.
#[op2]
#[string]
fn op_elohim_base64_encode(#[buffer] data: &[u8], url_safe: bool) -> String {
    base64_encode(data, url_safe)
}

/// Base64-decode `s` (lenient, both alphabets). Backs `Buffer.from(s, "base64" |
/// "base64url")`.
#[op2]
#[buffer]
fn op_elohim_base64_decode(#[string] s: &str) -> Vec<u8> {
    base64_decode(s)
}

/// Hex-encode `data` (lowercase). Backs the JS shim's `toString("hex")`.
#[op2]
#[string]
fn op_elohim_hex_encode(#[buffer] data: &[u8]) -> String {
    hex_encode(data)
}

/// Hex-decode `s` (truncating on the first invalid/odd nibble). Backs
/// `Buffer.from(s, "hex")`.
#[op2]
#[buffer]
fn op_elohim_hex_decode(#[string] s: &str) -> Vec<u8> {
    hex_decode(s)
}

deno_core::extension!(
    node_buffer_ext,
    ops = [
        op_elohim_base64_encode,
        op_elohim_base64_decode,
        op_elohim_hex_encode,
        op_elohim_hex_decode,
    ],
    js = [dir "src/shim", "node_buffer.js"],
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_all_pad_lengths() {
        // 0,1,2 trailing bytes exercise the three padding cases.
        for input in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            let enc = base64_encode(input, false);
            assert_eq!(base64_decode(&enc), input, "std round-trip for {input:?}");
        }
    }

    #[test]
    fn base64_known_vectors() {
        // RFC 4648 test vectors.
        assert_eq!(base64_encode(b"", false), "");
        assert_eq!(base64_encode(b"f", false), "Zg==");
        assert_eq!(base64_encode(b"fo", false), "Zm8=");
        assert_eq!(base64_encode(b"foo", false), "Zm9v");
        assert_eq!(base64_encode(b"foob", false), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba", false), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar", false), "Zm9vYmFy");
    }

    #[test]
    fn base64url_no_padding_and_urlsafe_alphabet() {
        // 0xff,0xef,0xfe -> "/+/+" in std, "_-_-"... build a case that yields +//.
        // Bytes chosen so std encoding contains + and /.
        let data = [0xfb_u8, 0xff, 0xbf];
        let std = base64_encode(&data, false);
        let url = base64_encode(&data, true);
        assert!(std.contains('+') || std.contains('/'), "std uses +/: {std}");
        assert!(
            !url.contains('+') && !url.contains('/'),
            "url-safe avoids +/: {url}"
        );
        assert!(!url.contains('='), "base64url has no padding: {url}");
        // Both decode back to the same bytes via the lenient decoder.
        assert_eq!(base64_decode(&std), data);
        assert_eq!(base64_decode(&url), data);
    }

    #[test]
    fn base64_decode_skips_whitespace() {
        assert_eq!(base64_decode("Zm9v\nYmFy"), b"foobar");
        assert_eq!(base64_decode("Zm9v YmFy"), b"foobar");
    }

    #[test]
    fn hex_round_trips_and_is_lowercase() {
        let data = [0x00_u8, 0x0f, 0xa5, 0xff, 0xde, 0xad, 0xbe, 0xef];
        let enc = hex_encode(&data);
        assert_eq!(enc, "000fa5ffdeadbeef");
        assert_eq!(hex_decode(&enc), data);
    }

    #[test]
    fn hex_decode_accepts_uppercase_and_truncates_odd_and_invalid() {
        assert_eq!(hex_decode("DEADBEEF"), [0xde, 0xad, 0xbe, 0xef]);
        // Odd trailing nibble is dropped (Node truncates the incomplete byte).
        assert_eq!(hex_decode("dead0"), [0xde, 0xad]);
        // Truncates at the first non-hex character.
        assert_eq!(hex_decode("deadZZ00"), [0xde, 0xad]);
        assert_eq!(hex_decode("z"), Vec::<u8>::new());
    }
}
