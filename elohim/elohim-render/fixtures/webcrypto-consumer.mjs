// Golden fixture: exercises the WebCrypto global installed at isolate init by
// node_webcrypto.js -- `globalThis.crypto.getRandomValues` (real OS entropy) and
// `crypto.randomUUID()`. Renders a space-separated `key:bool` string the Rust
// test asserts on, so each behaviour is proven end-to-end through a real V8
// isolate (not just at the Rust op level).
export function render(_url) {
  const c = globalThis.crypto;

  // In-place fill returns the SAME array (WebCrypto contract) and preserves length.
  const a = new Uint8Array(32);
  const ret = c.getRandomValues(a);
  const inPlaceSameRef = ret === a;
  const lenPreserved = a.length === 32 && ret.length === 32;

  // Two independent 32-byte draws must differ, and neither is all-zero (the
  // un-filled / PRNG-zero degrade this rung exists to prevent).
  const b = new Uint8Array(32);
  c.getRandomValues(b);
  let differ = false;
  for (let i = 0; i < 32; i++) {
    if (a[i] !== b[i]) { differ = true; break; }
  }
  let aAllZero = true;
  for (let i = 0; i < 32; i++) {
    if (a[i] !== 0) { aAllZero = false; break; }
  }

  // Wider element type: a Uint32Array is filled across its whole byte range.
  const u32 = new Uint32Array(4);
  c.getRandomValues(u32);
  let u32NonZero = false;
  for (let i = 0; i < 4; i++) {
    if (u32[i] !== 0) { u32NonZero = true; break; }
  }

  // Zero-length is a no-op returning the same (empty) array.
  const empty = new Uint8Array(0);
  const emptyOk = c.getRandomValues(empty) === empty;

  // randomUUID is a well-formed RFC 4122 v4 UUID (version nibble 4, variant 8/9/a/b),
  // and two calls differ.
  const uuid = c.randomUUID();
  const uuidFormatOk =
    /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(uuid);
  const uuidUnique = c.randomUUID() !== c.randomUUID();

  return (
    `inPlaceSameRef:${inPlaceSameRef} ` +
    `lenPreserved:${lenPreserved} ` +
    `differ:${differ} ` +
    `notAllZero:${!aAllZero} ` +
    `u32NonZero:${u32NonZero} ` +
    `emptyOk:${emptyOk} ` +
    `uuidFormatOk:${uuidFormatOk} ` +
    `uuidUnique:${uuidUnique}`
  );
}
