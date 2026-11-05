import type { Envelope } from './generated/Envelope.js';
import { canonicalEnvelopeBytes } from './envelope.js';
import { computeCid } from './cid.js';
import { verifyEd25519 } from './proof.js';
import { ed25519Multikey, signingTranscript } from './signature.js';
import { CID } from 'multiformats/cid';

export interface Epr {
  envelope: Envelope;
  payload: Uint8Array;
}

/** The domain an EPR envelope's signature is made under (elohim/epr/src/epr.rs). */
export const EPR_SIGNATURE_DOMAIN = 'elohim:epr:v1:';
/** The EPR envelope's signature epoch. */
export const EPR_SIGNATURE_EPOCH = 0;

export type VerifyError =
  | { kind: 'cid-mismatch'; message: string }
  | { kind: 'scheme-held'; message: string; scheme: number }
  | { kind: 'signature-invalid'; message: string };

/** Parse a CID that may arrive as a string or byte array. */
function parseCidField(raw: unknown): CID {
  if (typeof raw === 'string') return CID.parse(raw);
  if (raw instanceof Uint8Array) return CID.decode(raw);
  if (Array.isArray(raw)) return CID.decode(new Uint8Array(raw));
  throw new Error(`Cannot parse CID from ${JSON.stringify(raw)}`);
}

/** Parse signature bytes that may arrive in various forms. */
function parseSignatureBytes(sigRaw: unknown): Uint8Array | null {
  if (sigRaw instanceof Uint8Array) return sigRaw;
  if (Array.isArray(sigRaw)) return new Uint8Array(sigRaw);
  if (sigRaw && typeof sigRaw === 'object') {
    const obj = sigRaw as Record<string, unknown>;
    if (Array.isArray(obj['data'])) {
      return new Uint8Array(obj['data'] as number[]);
    }
    return new Uint8Array(Object.values(obj) as number[]);
  }
  return null;
}

/** Verify an Epr against the signer's public key. */
export async function verifyEpr(
  epr: Epr,
  publicKey: Uint8Array,
): Promise<{ ok: true } | { ok: false; error: VerifyError }> {
  const canonical = await canonicalEnvelopeBytes(epr.envelope, epr.payload);
  const derived = await computeCid(canonical);
  const claimed = parseCidField(epr.envelope.cid as unknown);
  if (derived.toString() !== claimed.toString()) {
    return {
      ok: false,
      error: { kind: 'cid-mismatch', message: `derived ${derived} != claimed ${claimed}` },
    };
  }
  const env = epr.envelope.proof.envelope;
  // An unknown scheme is HELD, never Invalid: neither authentic nor forged to this reader.
  if (env.scheme !== 0) {
    return {
      ok: false,
      error: {
        kind: 'scheme-held',
        message: `signature scheme ${env.scheme} is held: not implemented by this reader`,
        scheme: env.scheme,
      },
    };
  }

  const keyBytes = parseSignatureBytes(env.key as unknown);
  const sigBytes = parseSignatureBytes(env.sig as unknown);
  if (keyBytes === null || sigBytes === null) {
    return {
      ok: false,
      error: { kind: 'signature-invalid', message: 'key or signature bytes not parseable' },
    };
  }
  // Scheme 0: the key must be the Ed25519 multikey (ed 01 ‖ key) of the expected signer.
  const expectedKey = ed25519Multikey(publicKey);
  if (keyBytes.length !== expectedKey.length || !keyBytes.every((b, i) => b === expectedKey[i])) {
    return {
      ok: false,
      error: { kind: 'signature-invalid', message: 'envelope key is not the expected signer' },
    };
  }

  const transcript = signingTranscript(EPR_SIGNATURE_DOMAIN, EPR_SIGNATURE_EPOCH, 0, keyBytes, canonical);
  const ok = await verifyEd25519(publicKey, transcript, sigBytes);
  return ok
    ? { ok: true }
    : { ok: false, error: { kind: 'signature-invalid', message: 'signature verification failed' } };
}
