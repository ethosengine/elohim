import type { Envelope } from "./generated/Envelope.js";
import { canonicalEnvelopeBytes } from "./envelope.js";
import { computeCid } from "./cid.js";
import { verifyEd25519 } from "./proof.js";
import {
  ed25519Multikey,
  MalformedSignatureError,
  readSignatureEnvelope,
  signingTranscript,
  type WireSignatureEnvelope,
} from "./signature.js";
import { CID } from "multiformats/cid";

export interface Epr {
  envelope: Envelope;
  payload: Uint8Array;
}

/** The domain an EPR envelope's signature is made under (elohim/epr/src/epr.rs). */
export const EPR_SIGNATURE_DOMAIN = "elohim:epr:v1:";
/** The EPR envelope's signature epoch. */
export const EPR_SIGNATURE_EPOCH = 0;
/** The one signature scheme this reader implements: strict Ed25519. */
const SCHEME_ED25519 = 0;

/**
 * Why an EPR did not verify.
 *
 * - `malformed`: the envelope, its proof or the key handed in is not structurally an EPR here
 *   (a missing field, a scheme or epoch that is not a `u16`, a byte array with an element that is
 *   not an integer in 0..255). Never confused with `scheme-held`.
 * - `cid-mismatch`: the canonical bytes do not hash to the claimed CID.
 * - `scheme-held`: a structurally valid proof under a scheme this reader does not implement;
 *   neither authentic nor forged here.
 * - `signature-invalid`: scheme 0, and the key is not the expected signer's or the signature does
 *   not hold.
 */
export type VerifyError =
  | { kind: "malformed"; message: string }
  | { kind: "cid-mismatch"; message: string }
  | { kind: "scheme-held"; message: string; scheme: number }
  | { kind: "signature-invalid"; message: string };

type Result = { ok: true } | { ok: false; error: VerifyError };

function malformed(message: string): Result {
  return { ok: false, error: { kind: "malformed", message } };
}

/** Parse a CID that may arrive as a string or byte array. */
function parseCidField(raw: unknown): CID {
  if (typeof raw === "string") return CID.parse(raw);
  if (raw instanceof Uint8Array) return CID.decode(raw);
  if (Array.isArray(raw)) return CID.decode(new Uint8Array(raw));
  throw new Error(`Cannot parse CID from ${JSON.stringify(raw)}`);
}

/** The proof's signature envelope, read strictly; a message when it is not one. */
function readProofEnvelope(envelope: unknown): WireSignatureEnvelope | string {
  if (envelope === null || typeof envelope !== "object")
    return "envelope must be an object";
  const proof = (envelope as Record<string, unknown>)["proof"];
  if (proof === null || typeof proof !== "object" || Array.isArray(proof)) {
    return "envelope.proof must be an object";
  }
  if (!Object.prototype.hasOwnProperty.call(proof, "signer")) {
    return "envelope.proof is missing 'signer'";
  }
  try {
    return readSignatureEnvelope(
      (proof as Record<string, unknown>)["envelope"],
    );
  } catch (e) {
    if (e instanceof MalformedSignatureError)
      return `envelope.proof.envelope: ${e.message}`;
    throw e;
  }
}

/**
 * Verify an Epr against the signer's public key. Checks, in order: structure (`malformed`), the
 * claimed CID (`cid-mismatch`), the scheme (`scheme-held` for a valid unknown code), then the key
 * and signature (`signature-invalid`).
 */
export async function verifyEpr(
  epr: Epr,
  publicKey: Uint8Array,
): Promise<Result> {
  if (!(publicKey instanceof Uint8Array) || publicKey.length !== 32) {
    return malformed("public key must be 32 bytes");
  }
  const env = readProofEnvelope(epr?.envelope);
  if (typeof env === "string") return malformed(env);

  let canonical: Uint8Array;
  let derived: CID;
  let claimed: CID;
  try {
    canonical = await canonicalEnvelopeBytes(epr.envelope, epr.payload);
    derived = await computeCid(canonical);
    claimed = parseCidField(epr.envelope.cid as unknown);
  } catch (e) {
    return malformed(
      `envelope does not encode: ${e instanceof Error ? e.message : String(e)}`,
    );
  }
  if (derived.toString() !== claimed.toString()) {
    return {
      ok: false,
      error: {
        kind: "cid-mismatch",
        message: `derived ${derived} != claimed ${claimed}`,
      },
    };
  }

  // An unknown scheme is HELD, never Invalid: neither authentic nor forged to this reader.
  if (env.scheme !== SCHEME_ED25519) {
    return {
      ok: false,
      error: {
        kind: "scheme-held",
        message: `signature scheme ${env.scheme} is held: not implemented by this reader`,
        scheme: env.scheme,
      },
    };
  }

  // Scheme 0: the key must be the Ed25519 multikey (ed 01 ‖ key) of the expected signer.
  const expectedKey = ed25519Multikey(publicKey);
  if (
    env.key.length !== expectedKey.length ||
    !env.key.every((b, i) => b === expectedKey[i])
  ) {
    return {
      ok: false,
      error: {
        kind: "signature-invalid",
        message: "envelope key is not the expected signer",
      },
    };
  }

  const transcript = signingTranscript(
    EPR_SIGNATURE_DOMAIN,
    EPR_SIGNATURE_EPOCH,
    SCHEME_ED25519,
    env.key,
    canonical,
  );
  const ok = await verifyEd25519(publicKey, transcript, env.sig);
  return ok
    ? { ok: true }
    : {
        ok: false,
        error: {
          kind: "signature-invalid",
          message: "signature verification failed",
        },
      };
}
