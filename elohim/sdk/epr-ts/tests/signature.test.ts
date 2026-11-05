// Transcript v1 and the strict wire reader: the TS twin of elohim/epr/src/signature.rs.
import { readFileSync } from "node:fs";
import { join } from "node:path";

import * as ed from "@noble/ed25519";
import { describe, expect, it } from "vitest";

import { verifyEpr, type Epr } from "../src/epr";
import type { Envelope } from "../src/generated/Envelope";
import {
  ed25519Multikey,
  MalformedSignatureError,
  readSignatureEnvelope,
  signingTranscript,
  strictBytes,
} from "../src/signature";

const DOMAIN = "elohim:test:v0:";
const enc = (s: string) => new TextEncoder().encode(s);
const hex = (b: Uint8Array) => Buffer.from(b).toString("hex");
const unhex = (h: string) => Uint8Array.from(Buffer.from(h, "hex"));
const cat = (...parts: Uint8Array[]) => ed.etc.concatBytes(...parts);

// The envelope vector, pinned identically in elohim/epr/tests/signature_struct.rs.
const SEED = new Uint8Array(32).fill(42);
const VECTOR_PUBLIC_KEY =
  "197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61";
const VECTOR_TRANSCRIPT =
  "000f656c6f68696d3a746573743a76303a000000000022ed01197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61616263";
const VECTOR_SIG =
  "e70b241c0e9a88696849cc40901f4856da3fab9909dda129358f56e0d15a27947fef04767fa469767eff6d2c24117f1ba691a3cc1edf761a5d711c0a582af90e";

describe("signingTranscript — transcript v1", () => {
  it("matches the shared envelope vector", async () => {
    const pk = await ed.getPublicKeyAsync(SEED);
    expect(hex(pk)).toBe(VECTOR_PUBLIC_KEY);
    const t = signingTranscript(DOMAIN, 0, 0, ed25519Multikey(pk), enc("abc"));
    expect(hex(t)).toBe(VECTOR_TRANSCRIPT);
    expect(hex(await ed.signAsync(t, SEED))).toBe(VECTOR_SIG);
  });

  it("frames every length: u16be(len d) ‖ d ‖ epoch ‖ scheme ‖ u16be(len k) ‖ k ‖ body", () => {
    const t = signingTranscript(
      "d",
      0x0102,
      0x0304,
      new Uint8Array([9, 9]),
      enc("zz"),
    );
    expect(Array.from(t)).toEqual([
      0, 1, 0x64, 1, 2, 3, 4, 0, 2, 9, 9, 0x7a, 0x7a,
    ]);
  });

  it("transcript collision regression: (D, K, ':'‖P‖B) and (D‖P‖':', K, B) differ", () => {
    const K = ed25519Multikey(unhex(VECTOR_PUBLIC_KEY));
    const P = cat(new Uint8Array(4), K);
    const B = enc("abc");
    // The unframed rule collided on exactly this pair.
    const unframed = (d: Uint8Array, k: Uint8Array, b: Uint8Array) =>
      cat(d, new Uint8Array(4), k, b);
    expect(hex(unframed(enc(DOMAIN), K, cat(enc(":"), P, B)))).toBe(
      hex(unframed(cat(enc(DOMAIN), P, enc(":")), K, B)),
    );
    const domainB = cat(enc(DOMAIN), P, enc(":"));
    const a = signingTranscript(DOMAIN, 0, 0, K, cat(enc(":"), P, B));
    const b = signingTranscript(domainB, 0, 0, K, B);
    expect(hex(a)).not.toBe(hex(b));
  });

  it("key-length collision regression: (D, K‖x, B) and (D, K, x‖B) differ", () => {
    const K = ed25519Multikey(unhex(VECTOR_PUBLIC_KEY));
    const a = signingTranscript(DOMAIN, 0, 0, cat(K, enc("x")), enc("abc"));
    const b = signingTranscript(DOMAIN, 0, 0, K, enc("xabc"));
    expect(hex(a)).not.toBe(hex(b));
  });

  it("refuses an epoch or scheme that is not a u16, never aliasing it", () => {
    const K = ed25519Multikey(unhex(VECTOR_PUBLIC_KEY));
    for (const bad of [
      65536,
      -1,
      1.5,
      Number.NaN,
      Number.POSITIVE_INFINITY,
      "0" as unknown as number,
    ]) {
      expect(
        () => signingTranscript(DOMAIN, bad, 0, K, enc("abc")),
        `epoch ${bad}`,
      ).toThrow(MalformedSignatureError);
      expect(
        () => signingTranscript(DOMAIN, 0, bad, K, enc("abc")),
        `scheme ${bad}`,
      ).toThrow(MalformedSignatureError);
    }
    expect(
      hex(signingTranscript(DOMAIN, 65535, 65535, K, enc("abc"))),
    ).toContain("ffffffff");
  });

  it("refuses a domain or key longer than 65535 bytes", () => {
    expect(() =>
      signingTranscript("x".repeat(65536), 0, 0, new Uint8Array(34), enc("")),
    ).toThrow(MalformedSignatureError);
    expect(() =>
      signingTranscript(DOMAIN, 0, 0, new Uint8Array(65536), enc("")),
    ).toThrow(MalformedSignatureError);
    expect(
      signingTranscript(DOMAIN, 0, 0, new Uint8Array(65535), enc("")).length,
    ).toBe(2 + 15 + 6 + 65535);
  });
});

describe("strict wire reader", () => {
  it("takes byte arrays only as integers in 0..255, never coerced", () => {
    expect(Array.from(strictBytes([0, 255]) ?? [])).toEqual([0, 255]);
    expect(strictBytes(new Uint8Array([1]))).toEqual(new Uint8Array([1]));
    for (const bad of [
      [256],
      [-1],
      [1.5],
      ["1"],
      [null],
      "abc",
      { 0: 1 },
      { type: "Buffer", data: [1] },
      null,
      undefined,
    ]) {
      expect(strictBytes(bad), JSON.stringify(bad)).toBeNull();
    }
  });

  it("requires every field and a u16 scheme", () => {
    const good = { scheme: 0, key: [1], sig: [2] };
    expect(readSignatureEnvelope(good).scheme).toBe(0);
    expect(readSignatureEnvelope({ ...good, scheme: 65535 }).scheme).toBe(
      65535,
    );
    const missing = (f: string) => {
      const o: Record<string, unknown> = { ...good };
      delete o[f];
      return o;
    };
    for (const bad of [
      missing("scheme"),
      missing("key"),
      missing("sig"),
      { ...good, scheme: -1 },
      { ...good, scheme: 1.5 },
      { ...good, scheme: "0" },
      { ...good, scheme: 65536 },
      { ...good, scheme: null },
      { ...good, key: [256] },
      { ...good, sig: [0, -1] },
      { ...good, key: "ed01" },
      null,
      [],
      "envelope",
    ]) {
      expect(() => readSignatureEnvelope(bad), JSON.stringify(bad)).toThrow(
        MalformedSignatureError,
      );
    }
  });
});

describe("verifyEpr — malformed is never held", () => {
  const vectors = JSON.parse(
    readFileSync(
      join(process.cwd(), "../../epr/tests/vectors/signed_eprs.json"),
      "utf-8",
    ),
  );
  const v = vectors[0];
  const pk = unhex(v.public_key_hex);
  const payload = unhex(v.payload_hex);
  const withProofEnvelope = (
    patch: (e: Record<string, unknown>) => void,
  ): Epr => {
    const envelope = structuredClone(v.envelope) as Envelope;
    patch(envelope.proof.envelope as unknown as Record<string, unknown>);
    return { envelope, payload };
  };

  it("verifies the unmodified vector", async () => {
    expect(
      await verifyEpr(
        withProofEnvelope(() => {}),
        pk,
      ),
    ).toEqual({ ok: true });
  });

  it("reports a missing, negative, fractional, string or out-of-range scheme as malformed", async () => {
    const cases: Array<[string, (e: Record<string, unknown>) => void]> = [
      ["missing", (e) => delete e["scheme"]],
      ["-1", (e) => (e["scheme"] = -1)],
      ["0.5", (e) => (e["scheme"] = 0.5)],
      ['"0"', (e) => (e["scheme"] = "0")],
      ["65536 (would alias 0)", (e) => (e["scheme"] = 65536)],
      ["null", (e) => (e["scheme"] = null)],
    ];
    for (const [name, patch] of cases) {
      const r = await verifyEpr(withProofEnvelope(patch), pk);
      expect(r.ok, name).toBe(false);
      if (!r.ok) expect(r.error.kind, name).toBe("malformed");
    }
  });

  it("reports out-of-range key or signature bytes as malformed, never coercing them", async () => {
    const plus256 = (f: string) => (e: Record<string, unknown>) => {
      e[f] = (e[f] as number[]).map((b) => b + 256);
    };
    const cases: Array<[string, (e: Record<string, unknown>) => void]> = [
      ["key + 256", plus256("key")],
      ["sig + 256", plus256("sig")],
      ["key missing", (e) => delete e["key"]],
      ["sig missing", (e) => delete e["sig"]],
      ["key as string", (e) => (e["key"] = "ed01")],
      [
        "sig fractional",
        (e) => (e["sig"] = (e["sig"] as number[]).map((b) => b + 0.5)),
      ],
      [
        "sig as Buffer JSON",
        (e) => (e["sig"] = { type: "Buffer", data: e["sig"] }),
      ],
    ];
    for (const [name, patch] of cases) {
      const r = await verifyEpr(withProofEnvelope(patch), pk);
      expect(r.ok, name).toBe(false);
      if (!r.ok) expect(r.error.kind, name).toBe("malformed");
    }
  });

  it("reports a missing proof, signer or envelope as malformed", async () => {
    for (const [name, patch] of [
      ["proof", (env: Record<string, unknown>) => delete env["proof"]],
      [
        "signer",
        (env: Record<string, unknown>) =>
          delete (env["proof"] as Record<string, unknown>)["signer"],
      ],
      [
        "envelope",
        (env: Record<string, unknown>) =>
          delete (env["proof"] as Record<string, unknown>)["envelope"],
      ],
    ] as const) {
      const envelope = structuredClone(v.envelope) as Record<string, unknown>;
      patch(envelope);
      const r = await verifyEpr(
        { envelope: envelope as unknown as Envelope, payload },
        pk,
      );
      expect(r.ok, name).toBe(false);
      if (!r.ok) expect(r.error.kind, name).toBe("malformed");
    }
  });

  it("holds only a structurally valid, unsupported scheme", async () => {
    for (const scheme of [1, 9, 65535]) {
      const r = await verifyEpr(
        withProofEnvelope((e) => (e["scheme"] = scheme)),
        pk,
      );
      expect(r).toEqual({
        ok: false,
        error: expect.objectContaining({ kind: "scheme-held", scheme }),
      });
    }
    // Garbage key/sig bytes under an unknown but valid scheme are still held: the bytes are
    // structurally bytes, and the scheme decides what they mean.
    const r = await verifyEpr(
      withProofEnvelope((e) => {
        e["scheme"] = 7;
        e["key"] = [1, 2, 3];
        e["sig"] = [];
      }),
      pk,
    );
    expect(r.ok === false && r.error.kind).toBe("scheme-held");
  });
});
