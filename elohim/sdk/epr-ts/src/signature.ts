/**
 * The signature envelope's signed transcript and its strict wire reader — the TS twin of
 * `signing_transcript` and `SignatureEnvelope` in elohim/epr/src/signature.rs.
 *
 * Transcript version 1:
 *
 *   u16be(len(domain)) ‖ domain ‖ u16be(epoch) ‖ u16be(scheme) ‖ u16be(len(key)) ‖ key ‖ body
 *
 * `domain` is the object type's registered tag (a string is taken as its UTF-8 bytes), `key` the canonical multikey bytes
 * (minimal unsigned-varint codec, then the key) and `body` the object's exact signed bytes. Every
 * variable-length field before `body` is length-prefixed, so the transcript is injective.
 *
 * Integers are never coerced: an epoch or scheme that is not an integer in 0..65535, or a byte
 * that is not an integer in 0..255, is malformed — `65536` does not alias `0`, `"0"` is not `0`,
 * and `256` does not wrap to `0` the way `new Uint8Array([256])` would.
 */

/** A transcript field that cannot be framed, or an envelope that is not one on the wire. */
export class MalformedSignatureError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MalformedSignatureError";
  }
}

const U16_MAX = 0xffff;

/** True iff `value` is an integer in 0..65535 (a `u16`), with no coercion. */
export function isU16(value: unknown): value is number {
  return (
    typeof value === "number" &&
    Number.isInteger(value) &&
    value >= 0 &&
    value <= U16_MAX
  );
}

function requireU16(name: string, value: unknown): number {
  if (!isU16(value)) {
    throw new MalformedSignatureError(`${name} must be an integer in 0..65535`);
  }
  return value;
}

function u16be(out: Uint8Array, at: number, value: number): void {
  out[at] = (value >> 8) & 0xff;
  out[at + 1] = value & 0xff;
}

/**
 * The transcript-v1 bytes of `(domain, epoch, scheme, key, body)`. Throws
 * `MalformedSignatureError` for an epoch or scheme outside 0..65535 or a domain or key longer than
 * 65535 bytes.
 */
export function signingTranscript(
  domain: string | Uint8Array,
  epoch: number,
  scheme: number,
  key: Uint8Array,
  body: Uint8Array,
): Uint8Array {
  requireU16("epoch", epoch);
  requireU16("scheme", scheme);
  const d =
    typeof domain === "string" ? new TextEncoder().encode(domain) : domain;
  if (d.length > U16_MAX)
    throw new MalformedSignatureError("domain longer than 65535 bytes");
  if (key.length > U16_MAX)
    throw new MalformedSignatureError("key longer than 65535 bytes");
  const out = new Uint8Array(2 + d.length + 6 + key.length + body.length);
  let at = 0;
  u16be(out, at, d.length);
  at += 2;
  out.set(d, at);
  at += d.length;
  u16be(out, at, epoch);
  u16be(out, at + 2, scheme);
  u16be(out, at + 4, key.length);
  at += 6;
  out.set(key, at);
  at += key.length;
  out.set(body, at);
  return out;
}

/** A signature envelope whose fields passed the strict wire reader. */
export interface WireSignatureEnvelope {
  scheme: number;
  key: Uint8Array;
  sig: Uint8Array;
}

/**
 * A byte string as it arrives: a `Uint8Array`, or a JSON array whose every element is an integer
 * in 0..255. Anything else — a string, an object, a fractional or out-of-range element — is
 * `null`, never coerced.
 */
export function strictBytes(raw: unknown): Uint8Array | null {
  if (raw instanceof Uint8Array) return raw;
  if (!Array.isArray(raw)) return null;
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) {
    const b: unknown = raw[i];
    if (typeof b !== "number" || !Number.isInteger(b) || b < 0 || b > 0xff)
      return null;
    out[i] = b;
  }
  return out;
}

/**
 * Read a signature envelope from the wire: `{ scheme, key, sig }`, every field mandatory, `scheme`
 * an integer in 0..65535, `key` and `sig` byte strings (`strictBytes`). Throws
 * `MalformedSignatureError` naming the first field that is not one. A structurally valid envelope
 * under a scheme this reader does not implement is returned as is: deciding Held is the caller's.
 */
export function readSignatureEnvelope(raw: unknown): WireSignatureEnvelope {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) {
    throw new MalformedSignatureError("signature envelope must be an object");
  }
  const obj = raw as Record<string, unknown>;
  for (const field of ["scheme", "key", "sig"]) {
    if (!Object.prototype.hasOwnProperty.call(obj, field)) {
      throw new MalformedSignatureError(
        `signature envelope is missing '${field}'`,
      );
    }
  }
  const scheme = requireU16("scheme", obj["scheme"]);
  const key = strictBytes(obj["key"]);
  if (key === null) {
    throw new MalformedSignatureError(
      "key must be a byte array of integers in 0..255",
    );
  }
  const sig = strictBytes(obj["sig"]);
  if (sig === null) {
    throw new MalformedSignatureError(
      "sig must be a byte array of integers in 0..255",
    );
  }
  if (key.length > U16_MAX)
    throw new MalformedSignatureError("key longer than 65535 bytes");
  return { scheme, key, sig };
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
