// elohim-render WebCrypto global: `globalThis.crypto` with getRandomValues,
// randomUUID, and a loud, named `subtle`.
//
// WHY this is an INIT-TIME global (not a loader module): the Angular server
// bundle's libp2p/multiformats chunk carries a jsbn/brorand-style CSPRNG probe
// that runs at MODULE-EVAL time (a top-level IIFE), and its FIRST strategy is a
// feature-detect of `globalThis.crypto && typeof crypto.getRandomValues ==
// "function"`. A bare deno_core runtime has no `crypto` global (Web Crypto lives
// in Deno's web extension, not core), so the probe fell through every strategy
// and threw "No secure random number generator found". Installing the global here
// -- at isolate construction, before ANY bundle module body runs -- makes the
// probe's first strategy succeed.
//
// ENTROPY HONESTY (non-negotiable): getRandomValues is backed by the Rust op
// `op_elohim_get_random_values`, which reads the OS CSPRNG (getrandom). It is REAL
// entropy or a hard error -- NEVER a JS PRNG masquerade. This backs libp2p /
// multiformats key + nonce generation on the render path, where a predictable
// stream is a security defect, not a rendering quirk.
//
// ONE SOURCE OF TRUTH: `node_crypto.js` (the `node:crypto` module shim) binds its
// `webcrypto` export to `globalThis.crypto`, so the global object installed here
// and `import { webcrypto } from "node:crypto"` are the SAME object -- crypto.subtle
// is single-sourced across both surfaces.
//
// SURFACE DISCIPLINE (mirrors node_crypto / node_buffer / node_builtins):
// getRandomValues + randomUUID are real. `crypto.subtle.*` is a set of LOUD, NAMED
// stubs: a render-path call throws a message naming the operation + this file, so
// an unproven crypto path fails clearly instead of silently. Any key-bearing
// subtle op reaching the SSR render path is a DESIGN SMELL (p2p identity init
// should be guarded behind isPlatformServer in the app) -- the loud throw makes
// that visible rather than papering over it with a fabricated key.
//
// WARNING: pure ASCII only. This file is served through the `js = [dir ...]` macro,
// which panics at const-eval on any non-ASCII byte. Keep every literal ASCII.
((globalThis) => {
  const core = Deno.core;

  // WebCrypto getRandomValues: fill `typedArray` IN PLACE with OS entropy and
  // return the SAME array (the WebCrypto contract). Rejects non-integer typed
  // arrays and enforces the 65536-byte per-call quota, matching the spec's
  // TypeMismatchError / QuotaExceededError shape (as plain errors -- the bundle's
  // probe only cares that the call succeeds and yields random bytes).
  function getRandomValues(typedArray) {
    if (typedArray == null || !ArrayBuffer.isView(typedArray)) {
      throw new TypeError(
        "crypto.getRandomValues: argument must be an integer-typed TypedArray"
      );
    }
    const ctorName =
      typedArray.constructor && typedArray.constructor.name
        ? typedArray.constructor.name
        : "";
    // WebCrypto permits only integer typed arrays; floats and DataView throw.
    if (
      ctorName === "Float16Array" ||
      ctorName === "Float32Array" ||
      ctorName === "Float64Array" ||
      ctorName === "DataView"
    ) {
      throw new TypeError(
        "crypto.getRandomValues: '" +
          ctorName +
          "' is not an integer typed array (TypeMismatchError)"
      );
    }
    const byteLength = typedArray.byteLength;
    if (byteLength > 65536) {
      throw new Error(
        "crypto.getRandomValues: requested " +
          byteLength +
          " bytes exceeds the 65536-byte quota (QuotaExceededError)"
      );
    }
    if (byteLength === 0) return typedArray;
    const bytes = core.ops.op_elohim_get_random_values(byteLength);
    // Copy into a byte-view over the SAME backing buffer so the caller's array is
    // filled in place regardless of its element width (Uint32Array etc.).
    const view = new Uint8Array(
      typedArray.buffer,
      typedArray.byteOffset,
      byteLength
    );
    view.set(bytes);
    return typedArray;
  }

  // WebCrypto randomUUID: an RFC 4122 version-4 UUID built from 16 bytes of the
  // SAME OS entropy source (cheap; no extra dependency). Version nibble -> 4,
  // variant bits -> 10xx.
  function randomUUID() {
    const b = core.ops.op_elohim_get_random_values(16);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex = "";
    for (let i = 0; i < 16; i++) {
      hex += b[i].toString(16).padStart(2, "0");
    }
    return (
      hex.slice(0, 8) +
      "-" +
      hex.slice(8, 12) +
      "-" +
      hex.slice(12, 16) +
      "-" +
      hex.slice(16, 20) +
      "-" +
      hex.slice(20, 32)
    );
  }

  // Loud, named subtle stub. A render-path call throws a message naming the
  // operation + this file so the next thing to implement (or, for key ops, the
  // app-side isPlatformServer guard to add) is unambiguous.
  function subtleNotImplemented(name) {
    return function subtleStub() {
      throw new Error(
        "elohim-render webcrypto shim: crypto.subtle." +
          name +
          "() is not implemented. The SSR server bundle invoked it during render. " +
          "If this is a digest, implement it in elohim/elohim-render/src/shim/" +
          "node_webcrypto.js backed by the sha2 op. If it is key material " +
          "(importKey / generateKey / sign / deriveKey / ...), that is a DESIGN " +
          "SMELL: SSR should not perform key operations -- guard p2p identity init " +
          "behind isPlatformServer in app/elohim-app rather than shimming it here."
      );
    };
  }

  const subtle = {
    digest: subtleNotImplemented("digest"),
    encrypt: subtleNotImplemented("encrypt"),
    decrypt: subtleNotImplemented("decrypt"),
    sign: subtleNotImplemented("sign"),
    verify: subtleNotImplemented("verify"),
    generateKey: subtleNotImplemented("generateKey"),
    importKey: subtleNotImplemented("importKey"),
    exportKey: subtleNotImplemented("exportKey"),
    deriveKey: subtleNotImplemented("deriveKey"),
    deriveBits: subtleNotImplemented("deriveBits"),
    wrapKey: subtleNotImplemented("wrapKey"),
    unwrapKey: subtleNotImplemented("unwrapKey"),
  };

  // Single WebCrypto object. `node_crypto.js` re-exports THIS as its `webcrypto`
  // named/default member (one source of truth).
  const cryptoGlobal = {
    getRandomValues,
    randomUUID,
    subtle,
  };

  // Install only if absent (a bare deno_core runtime has no `crypto` global).
  // Guarding avoids clobbering a real Web Crypto implementation should a future
  // deno_core ship one; the one-source-of-truth test asserts the binding held.
  if (typeof globalThis.crypto === "undefined") {
    globalThis.crypto = cryptoGlobal;
  }
})(globalThis);
