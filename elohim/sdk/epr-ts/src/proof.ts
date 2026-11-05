import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha512";

// @noble/ed25519 v2 requires setting the sha-512 hash provider for sync operations
ed.etc.sha512Sync = (...m: Uint8Array[]) => sha512(ed.etc.concatBytes(...m));

const { Point } = ed;
/** The order of the prime-order subgroup, L = 2^252 + 27742317777372353535851937790883648493. */
const L = ed.CURVE.n;

function numberLE(bytes: Uint8Array): bigint {
  let n = 0n;
  for (let i = bytes.length - 1; i >= 0; i--) n = (n << 8n) | BigInt(bytes[i]);
  return n;
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a[i] ^ b[i];
  return diff === 0;
}

/**
 * Strict Ed25519 verification — the twin of ed25519-dalek 2's `VerifyingKey::verify_strict`, the
 * one policy both languages enforce:
 *
 * 1. `S` (the signature's second half, little-endian) is canonical: `S < L`.
 * 2. `A` (the public key) and `R` (the signature's first half) decompress as dalek's
 *    `CompressedEdwardsY::decompress` does: the y coordinate is read from the low 255 bits and
 *    reduced mod p, and a zero x with the sign bit set is accepted as x = 0 (noble's
 *    `Point.fromBytes(bytes, true)` is exactly that rule).
 * 3. Neither `A` nor `R` is of small order (in the 8-torsion subgroup).
 * 4. `k = SHA-512(R_bytes ‖ A_bytes ‖ M) mod L` over the bytes AS RECEIVED, and the **uncofactored**
 *    equation holds: `[S]B − [k]A` compresses to exactly `R_bytes`.
 *
 * Noble's `verify(…, { zip215: false })` is not this: it checks the cofactored equation
 * `[8]([S]B − R − [k]A) = 0` and never rejects a small-order `R`, so it admits signatures dalek
 * refuses (`tests/vectors/ed25519_strict.json`).
 */
export function verifyEd25519Strict(
  publicKey: Uint8Array,
  message: Uint8Array,
  signature: Uint8Array,
): boolean {
  if (!(publicKey instanceof Uint8Array) || !(signature instanceof Uint8Array))
    return false;
  if (publicKey.length !== 32 || signature.length !== 64) return false;
  const rBytes = signature.subarray(0, 32);
  const s = numberLE(signature.subarray(32, 64));
  if (s >= L) return false;
  let A: InstanceType<typeof Point>;
  let R: InstanceType<typeof Point>;
  try {
    A = Point.fromBytes(publicKey, true);
    R = Point.fromBytes(rBytes, true);
  } catch {
    return false;
  }
  if (A.isSmallOrder() || R.isSmallOrder()) return false;
  const k =
    numberLE(sha512(ed.etc.concatBytes(rBytes, publicKey, message))) % L;
  const expectedR = Point.BASE.multiply(s, false).add(
    A.multiply(k, false).negate(),
  );
  // R is not small-order, so an identity here cannot equal it (and noble cannot encode one).
  if (expectedR.is0()) return false;
  return sameBytes(expectedR.toBytes(), rBytes);
}

/** Verify an Ed25519 signature under the strict policy of `verifyEd25519Strict`. */
export async function verifyEd25519(
  publicKey: Uint8Array,
  message: Uint8Array,
  signature: Uint8Array,
): Promise<boolean> {
  try {
    return verifyEd25519Strict(publicKey, message, signature);
  } catch {
    return false;
  }
}
