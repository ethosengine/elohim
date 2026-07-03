// Node `crypto` builtin shim for the SSR runtime.
//
// The Angular server bundle statically imports Node's `crypto` builtin (bare
// `import ... from "crypto"`). deno_core's module loader cannot resolve a bare
// Node builtin, so every render used to die with a raw TypeError before this
// shim existed. This module is served by NodeShimLoader for the `crypto` /
// `node:crypto` specifiers.
//
// SURFACE DISCIPLINE (the bundle's Node-builtin surface -- extend deliberately,
// never auto-stub): only `createHash` is proven to execute on the landing
// render path (multiformats computes content-address digests via
// `createHash("sha256").update(bytes).digest()`), so only `createHash` is
// implemented for real -- backed by the Rust `op_node_crypto_hash` op (sha2).
// Every other member is a LOUD, NAMED stub: a call throws an error naming the
// member, so an unproven path fails clearly ("crypto.randomBytes is not
// implemented") instead of a cryptic "undefined is not a function". Implement
// more members deliberately, one at a time, as render paths prove they need them.
//
// WARNING: pure ASCII only (this file is embedded via include_str! and served as
// module source; keeping it ASCII matches the rest of src/shim and avoids any
// encoding surprises).

function toBytes(data) {
  if (data == null) return new Uint8Array(0);
  if (typeof data === "string") return new TextEncoder().encode(data);
  if (data instanceof Uint8Array) return data;
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (ArrayBuffer.isView(data)) {
    return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  }
  return new TextEncoder().encode(String(data));
}

function bytesToHex(bytes) {
  let out = "";
  for (let i = 0; i < bytes.length; i++) {
    out += bytes[i].toString(16).padStart(2, "0");
  }
  return out;
}

// Node `crypto.createHash(algorithm)` -> incremental Hash. Buffers the update
// chunks in JS and hashes once at digest() time via the Rust op (single-update
// is the only observed usage, so buffering is cheap and correct for multi-update
// too). Returns raw bytes (Uint8Array) for `digest()` with no encoding -- what
// multiformats expects -- or a hex string for `digest("hex")`.
function createHash(algorithm) {
  const algo = String(algorithm).toLowerCase();
  const chunks = [];
  const hash = {
    update(data, _inputEncoding) {
      chunks.push(toBytes(data));
      return hash;
    },
    digest(encoding) {
      let total = 0;
      for (let i = 0; i < chunks.length; i++) total += chunks[i].length;
      const joined = new Uint8Array(total);
      let offset = 0;
      for (let i = 0; i < chunks.length; i++) {
        joined.set(chunks[i], offset);
        offset += chunks[i].length;
      }
      const digest = Deno.core.ops.op_node_crypto_hash(algo, joined);
      if (encoding == null) return digest;
      if (encoding === "hex") return bytesToHex(digest);
      throw new Error(
        "elohim-render crypto shim: createHash().digest('" + encoding +
        "') encoding is not implemented (raw bytes or 'hex' only)."
      );
    },
  };
  return hash;
}

// Build a loud, named stub for a member we have not implemented yet. Accessing
// it is fine (feature-detection stays truthy-free is not a concern for this
// bundle, which only ever CALLS members); calling it throws with the member
// name so the failure points precisely at what to implement next.
function notImplemented(name) {
  return function notImplementedStub() {
    throw new Error(
      "elohim-render crypto shim: node crypto." + name + "() is not implemented. " +
      "The SSR server bundle invoked it during render. Implement it deliberately in " +
      "elohim/elohim-render/src/shim/node_crypto.{js,rs} -- the bundle's Node-builtin " +
      "surface, extend deliberately, never auto-stub."
    );
  };
}

// Named exports. ESM linking requires every `import { X } from "crypto"` in the
// bundle to find a matching named export, so these must EXIST even when the
// render path never reads them:
//  - createPrivateKey / createPublicKey / webcrypto: named-imported by the bundle
//    (proven via static grep) but not on the landing render path -> loud stubs.
//  - createHash: real (see above); also named for `import { createHash }`.
//  - randomBytes / randomUUID: common framework members exported as loud stubs so
//    a `import { randomUUID } from "crypto"` links and fails clearly ON CALL.
const createPrivateKey = notImplemented("createPrivateKey");
const createPublicKey = notImplemented("createPublicKey");
const randomBytes = notImplemented("randomBytes");
const randomUUID = notImplemented("randomUUID");
// bare deno_core exposes no Web Crypto global; expose whatever the isolate has
// (typically undefined) rather than fabricate one. The named export must exist
// for linking regardless of whether the render path reads it.
const webcrypto = (typeof globalThis !== "undefined" ? globalThis.crypto : undefined);

export {
  createHash,
  createPrivateKey,
  createPublicKey,
  randomBytes,
  randomUUID,
  webcrypto,
};

// Default export -- `import crypto from "crypto"` binds this object, and the
// bundle accesses members off it (e.g. `_r.createHash(...)`). Carries the real
// createHash plus loud stubs for every member observed in the bundle's static
// import graph, so a default-binding member call throws a named error instead of
// "undefined is not a function".
const cryptoShim = {
  createHash,
  createPrivateKey,
  createPublicKey,
  randomBytes,
  randomUUID,
  webcrypto,
  randomFillSync: notImplemented("randomFillSync"),
  createHmac: notImplemented("createHmac"),
  createCipheriv: notImplemented("createCipheriv"),
  createDecipheriv: notImplemented("createDecipheriv"),
  createSign: notImplemented("createSign"),
  createVerify: notImplemented("createVerify"),
  createECDH: notImplemented("createECDH"),
  generateKeyPair: notImplemented("generateKeyPair"),
  generateKeyPairSync: notImplemented("generateKeyPairSync"),
  diffieHellman: notImplemented("diffieHellman"),
  publicEncrypt: notImplemented("publicEncrypt"),
  privateDecrypt: notImplemented("privateDecrypt"),
  pbkdf2: notImplemented("pbkdf2"),
  pbkdf2Sync: notImplemented("pbkdf2Sync"),
  sign: notImplemented("sign"),
  verify: notImplemented("verify"),
  getHashes: notImplemented("getHashes"),
  getCiphers: notImplemented("getCiphers"),
};

export default cryptoShim;
