// verifyEd25519 is ed25519-dalek's `verify_strict`, not noble's RFC 8032 mode: the shared vectors
// in elohim/epr/tests/vectors/ed25519_strict.json hold signatures that noble's cofactored
// `verify(…, { zip215: false })` admits and dalek refuses, and the Rust parity test
// (elohim/epr/tests/ed25519_strict.rs) answers every vector the same way.
import { existsSync, readFileSync, writeFileSync } from "node:fs";

import * as ed from "@noble/ed25519";
import { describe, expect, it } from "vitest";

import { verifyEd25519, verifyEd25519Strict } from "../src/proof";
import { ed25519Multikey, signingTranscript } from "../src/signature";
import {
  deriveStrictVectors,
  STRICT_DOMAIN,
  T8_HEX,
  type StrictVector,
} from "./strict-vectors";

const FIXTURE = new URL(
  "../../../epr/tests/vectors/ed25519_strict.json",
  import.meta.url,
);
const unhex = (h: string) => Uint8Array.from(Buffer.from(h, "hex"));

function fixture(): StrictVector[] {
  const derived = deriveStrictVectors();
  // Compare-only: the file is written only when absent and explicitly asked for.
  if (!existsSync(FIXTURE) && process.env["WRITE_STRICT_VECTORS"] === "1") {
    writeFileSync(FIXTURE, `${JSON.stringify(derived, null, 2)}\n`);
  }
  return JSON.parse(readFileSync(FIXTURE, "utf8")) as StrictVector[];
}

const byLabel = (vs: StrictVector[], label: string) => {
  const v = vs.find((x) => x.label === label);
  if (!v) throw new Error(`no vector ${label}`);
  return v;
};

describe("strict Ed25519 — the shared vectors", () => {
  const vectors = fixture();

  it("the pinned file is exactly the in-repo derivation", () => {
    expect(vectors).toEqual(deriveStrictVectors());
    expect(vectors.length).toBe(8);
  });

  it("T8 is an order-8 point", () => {
    const t = ed.Point.fromBytes(unhex(T8_HEX), true);
    expect(t.isSmallOrder()).toBe(true);
    expect(t.double().double().is0()).toBe(false);
    expect(t.double().double().double().is0()).toBe(true);
  });

  it("every vector signs its transcript-v1 bytes", () => {
    for (const v of vectors) {
      const t = signingTranscript(
        STRICT_DOMAIN,
        0,
        0,
        ed25519Multikey(unhex(v.public_key_hex)),
        unhex(v.body_hex),
      );
      expect(Buffer.from(t).toString("hex"), v.label).toBe(v.transcript_hex);
    }
  });

  it("answers every vector as verify_strict does", async () => {
    for (const v of vectors) {
      const ok = await verifyEd25519(
        unhex(v.public_key_hex),
        unhex(v.transcript_hex),
        unhex(v.signature_hex),
      );
      expect(ok, v.label).toBe(v.strict === "accept");
    }
  });

  it("MUST REJECT the review vector: A = B, R = (0,-1) = ecff…7f, S = SHA512(R‖A‖M) mod L", () => {
    const v = byLabel(vectors, "small-order-r-cofactored-only");
    expect(v.public_key_hex).toBe(
      Buffer.from(ed.Point.BASE.toBytes()).toString("hex"),
    );
    expect(v.signature_hex.slice(0, 64)).toBe(
      "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    );
    const [pk, msg, sig] = [
      unhex(v.public_key_hex),
      unhex(v.transcript_hex),
      unhex(v.signature_hex),
    ];
    // The hazard: noble's RFC 8032 mode (the previous implementation) admits it.
    expect(ed.verify(sig, msg, pk, { zip215: false })).toBe(true);
    expect(verifyEd25519Strict(pk, msg, sig)).toBe(false);
  });

  it("rejects small-order R, mixed-torsion A and mixed-torsion R that noble admits", () => {
    for (const label of [
      "small-order-r-cofactored-only",
      "small-order-r-equation-holds",
      "mixed-torsion-a",
      "mixed-torsion-r",
    ]) {
      const v = byLabel(vectors, label);
      const [pk, msg, sig] = [
        unhex(v.public_key_hex),
        unhex(v.transcript_hex),
        unhex(v.signature_hex),
      ];
      expect(
        ed.verify(sig, msg, pk, { zip215: false }),
        `${label}: noble admits`,
      ).toBe(true);
      expect(verifyEd25519Strict(pk, msg, sig), label).toBe(false);
    }
  });

  it("accepts a mixed-torsion key whose challenge kills the torsion, as dalek does", () => {
    const v = byLabel(vectors, "mixed-torsion-a-k-multiple-of-8");
    const pk = unhex(v.public_key_hex);
    expect(ed.Point.fromBytes(pk, true).isTorsionFree()).toBe(false);
    expect(
      verifyEd25519Strict(pk, unhex(v.transcript_hex), unhex(v.signature_hex)),
    ).toBe(true);
  });

  it("refuses short, long and non-byte inputs without throwing", async () => {
    const v = byLabel(vectors, "valid-torsion-free");
    const [pk, msg, sig] = [
      unhex(v.public_key_hex),
      unhex(v.transcript_hex),
      unhex(v.signature_hex),
    ];
    expect(await verifyEd25519(pk, msg, sig)).toBe(true);
    expect(await verifyEd25519(pk.subarray(1), msg, sig)).toBe(false);
    expect(await verifyEd25519(pk, msg, sig.subarray(1))).toBe(false);
    expect(await verifyEd25519(pk, msg, new Uint8Array([...sig, 0]))).toBe(
      false,
    );
    const notAPoint = pk.slice();
    notAPoint.fill(0xff);
    expect(await verifyEd25519(notAPoint, msg, sig)).toBe(false);
  });
});
