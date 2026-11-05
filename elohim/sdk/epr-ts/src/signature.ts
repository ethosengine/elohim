/**
 * The signed transcript of a signature envelope — the TS twin of `signing_transcript` in
 * elohim/epr/src/signature.rs:
 *
 *   domain ‖ u16be(epoch) ‖ u16be(scheme) ‖ key ‖ body
 *
 * `key` is the canonical multikey bytes (minimal unsigned-varint codec, then the key).
 */
export function signingTranscript(
  domain: string,
  epoch: number,
  scheme: number,
  key: Uint8Array,
  body: Uint8Array,
): Uint8Array {
  const d = new TextEncoder().encode(domain);
  const out = new Uint8Array(d.length + 4 + key.length + body.length);
  out.set(d, 0);
  out[d.length] = (epoch >> 8) & 0xff;
  out[d.length + 1] = epoch & 0xff;
  out[d.length + 2] = (scheme >> 8) & 0xff;
  out[d.length + 3] = scheme & 0xff;
  out.set(key, d.length + 4);
  out.set(body, d.length + 4 + key.length);
  return out;
}

/** Multicodec of an Ed25519 public key, varint-encoded as `ed 01`. */
export const ED25519_PUB_MULTICODEC = 0xed;

/** The Ed25519 multikey bytes: `ed 01 ‖ publicKey`. */
export function ed25519Multikey(publicKey: Uint8Array): Uint8Array {
  const out = new Uint8Array(2 + publicKey.length);
  out[0] = 0xed;
  out[1] = 0x01;
  out.set(publicKey, 2);
  return out;
}
