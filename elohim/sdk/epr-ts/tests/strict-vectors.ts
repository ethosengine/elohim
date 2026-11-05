// Derivation of `elohim/epr/tests/vectors/ed25519_strict.json`: Ed25519 signatures on which a
// cofactored or small-order-tolerant verifier and ed25519-dalek's `verify_strict` disagree, plus
// controls both accept. Every vector signs the transcript-v1 bytes of
// ("elohim:test:v0:", epoch 0, scheme 0, ed01 ‖ A, body), so the Rust parity test
// (elohim/epr/tests/ed25519_strict.rs) checks them through `SignatureEnvelope::check` too.
//
// Scalars are fixed: x = SHA-512("elohim:strict-vector:" ‖ name) mod L. T8 is an order-8 point.
import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha512";

import { ed25519Multikey, signingTranscript } from "../src/signature";

ed.etc.sha512Sync = (...m: Uint8Array[]) => sha512(ed.etc.concatBytes(...m));

const { Point } = ed;
type P = InstanceType<typeof Point>;
const L = ed.CURVE.n;
export const STRICT_DOMAIN = "elohim:test:v0:";

/** An order-8 point (curve25519-dalek's EIGHT_TORSION[1]). */
export const T8_HEX =
  "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05";
/** The identity, (0, 1). */
const IDENTITY_HEX =
  "0100000000000000000000000000000000000000000000000000000000000000";
/** (0, −1): y = p − 1, order 2. The review's small-order R. */
const MINUS_ONE_HEX =
  "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f";

export interface StrictVector {
  label: string;
  about: string;
  public_key_hex: string;
  body_hex: string;
  transcript_hex: string;
  signature_hex: string;
  /** What `verify_strict` (and verifyEd25519) must answer. */
  strict: "accept" | "reject";
}

const hex = (b: Uint8Array) => Buffer.from(b).toString("hex");
const unhex = (h: string) => Uint8Array.from(Buffer.from(h, "hex"));

function numberLE(bytes: Uint8Array): bigint {
  let n = 0n;
  for (let i = bytes.length - 1; i >= 0; i--) n = (n << 8n) | BigInt(bytes[i]);
  return n;
}

function le32(n: bigint): Uint8Array {
  const out = new Uint8Array(32);
  for (let i = 0; i < 32; i++) out[i] = Number((n >> BigInt(8 * i)) & 0xffn);
  return out;
}

const mod = (n: bigint) => ((n % L) + L) % L;
const scalar = (name: string) =>
  mod(
    numberLE(sha512(new TextEncoder().encode(`elohim:strict-vector:${name}`))),
  );

function transcript(pk: Uint8Array, body: Uint8Array): Uint8Array {
  return signingTranscript(STRICT_DOMAIN, 0, 0, ed25519Multikey(pk), body);
}

function challenge(r: Uint8Array, pk: Uint8Array, msg: Uint8Array): bigint {
  return mod(numberLE(sha512(ed.etc.concatBytes(r, pk, msg))));
}

function vector(
  label: string,
  about: string,
  pk: Uint8Array,
  body: Uint8Array,
  r: Uint8Array,
  s: bigint,
  strict: StrictVector["strict"],
): StrictVector {
  return {
    label,
    about,
    public_key_hex: hex(pk),
    body_hex: hex(body),
    transcript_hex: hex(transcript(pk, body)),
    signature_hex: hex(ed.etc.concatBytes(r, le32(s))),
    strict,
  };
}

/** A Schnorr signature by `a` with nonce point `Rp` (nonce scalar `r`) on a body chosen so that
 * `k mod 8` satisfies `want`: the body is "abc" followed by a counter byte. */
function signWith(
  a: bigint,
  pkPoint: P,
  r: bigint,
  Rp: P,
  want: (k: bigint) => boolean,
): { pk: Uint8Array; body: Uint8Array; R: Uint8Array; s: bigint } {
  const pk = pkPoint.toBytes();
  const R = Rp.toBytes();
  for (let i = 0; i < 256; i++) {
    const body = Uint8Array.of(0x61, 0x62, 0x63, i);
    const k = challenge(R, pk, transcript(pk, body));
    if (want(k)) return { pk, body, R, s: mod(r + k * a) };
  }
  throw new Error("no body met the challenge condition");
}

export function deriveStrictVectors(): StrictVector[] {
  const B = Point.BASE;
  const T8 = Point.fromBytes(unhex(T8_HEX), true);
  const a = scalar("a");
  const r = scalar("r");
  const A = B.multiply(a);
  const any = () => true;
  const out: StrictVector[] = [];

  const valid = signWith(a, A, r, B.multiply(r), any);
  out.push(
    vector(
      "valid-torsion-free",
      "A = aB, R = rB, S = r + ka: both equations hold.",
      valid.pk,
      valid.body,
      valid.R,
      valid.s,
      "accept",
    ),
  );
  out.push(
    vector(
      "non-canonical-s",
      "The valid signature with S + L: S must be < L.",
      valid.pk,
      valid.body,
      valid.R,
      valid.s + L,
      "reject",
    ),
  );

  {
    // The review's vector: A = B, R = (0, −1), S = k. Only the cofactored equation holds.
    const pk = B.toBytes();
    const R = unhex(MINUS_ONE_HEX);
    const body = new TextEncoder().encode("abc");
    const k = challenge(R, pk, transcript(pk, body));
    out.push(
      vector(
        "small-order-r-cofactored-only",
        "A = B, R = (0,-1) of order 2, S = SHA512(R||A||M) mod L.",
        pk,
        body,
        R,
        k,
        "reject",
      ),
    );
  }
  {
    // A = aB + T2 and R = T2 = (0, −1), S = ka with k odd: [S]B − [k]A = −[k]T2 = T2 = R exactly,
    // so the uncofactored equation holds and only the explicit small-order R check refuses it.
    const T2 = Point.fromBytes(unhex(MINUS_ONE_HEX), true);
    const pk = B.multiply(a).add(T2).toBytes();
    const R = unhex(MINUS_ONE_HEX);
    let found: StrictVector | null = null;
    for (let i = 0; i < 256 && found === null; i++) {
      const body = Uint8Array.of(0x61, 0x62, 0x63, i);
      const k = challenge(R, pk, transcript(pk, body));
      if (k % 2n === 1n) {
        found = vector(
          "small-order-r-equation-holds",
          "A = aB + T2, R = T2 = (0,-1), S = ka with k odd: [S]B - [k]A = R exactly; only the small-order R check refuses.",
          pk,
          body,
          R,
          mod(k * a),
          "reject",
        );
      }
    }
    if (found === null) throw new Error("no body gave an odd challenge");
    out.push(found);
  }
  {
    // A = identity, R = identity, S = 0: the equation holds for every message.
    const pk = unhex(IDENTITY_HEX);
    out.push(
      vector(
        "small-order-a-identity",
        "A = identity, R = identity, S = 0: the equation holds; A is small-order.",
        pk,
        new TextEncoder().encode("abc"),
        unhex(IDENTITY_HEX),
        0n,
        "reject",
      ),
    );
  }
  const Amixed = A.add(T8);
  const mixedA = signWith(a, Amixed, r, B.multiply(r), (k) => k % 8n !== 0n);
  out.push(
    vector(
      "mixed-torsion-a",
      "A = aB + T8, R = rB, S = r + ka, k mod 8 != 0: only the cofactored equation holds.",
      mixedA.pk,
      mixedA.body,
      mixedA.R,
      mixedA.s,
      "reject",
    ),
  );
  const mixedA8 = signWith(a, Amixed, r, B.multiply(r), (k) => k % 8n === 0n);
  out.push(
    vector(
      "mixed-torsion-a-k-multiple-of-8",
      "A = aB + T8, k mod 8 == 0: [k]T8 vanishes, the uncofactored equation holds; dalek accepts a mixed-torsion key.",
      mixedA8.pk,
      mixedA8.body,
      mixedA8.R,
      mixedA8.s,
      "accept",
    ),
  );
  const mixedR = signWith(a, A, r, B.multiply(r).add(T8), any);
  out.push(
    vector(
      "mixed-torsion-r",
      "A = aB, R = rB + T8, S = r + ka: only the cofactored equation holds.",
      mixedR.pk,
      mixedR.body,
      mixedR.R,
      mixedR.s,
      "reject",
    ),
  );
  return out;
}
