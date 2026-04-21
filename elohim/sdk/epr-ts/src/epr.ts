import type { Envelope } from './generated/Envelope';
import { canonicalEnvelopeBytes } from './envelope';
import { computeCid } from './cid';
import { verifyEd25519 } from './proof';
import { CID } from 'multiformats/cid';

export interface Epr {
  envelope: Envelope;
  payload: Uint8Array;
}

export type VerifyError =
  | { kind: 'cid-mismatch'; message: string }
  | { kind: 'algorithm-unsupported'; message: string }
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
  if (epr.envelope.proof.algorithm !== 'ed25519') {
    return {
      ok: false,
      error: {
        kind: 'algorithm-unsupported',
        message: `unsupported algorithm: ${epr.envelope.proof.algorithm}`,
      },
    };
  }

  const sigBytes = parseSignatureBytes(epr.envelope.proof.signature as unknown);
  if (sigBytes === null) {
    return {
      ok: false,
      error: { kind: 'signature-invalid', message: 'signature bytes not parseable' },
    };
  }

  const ok = await verifyEd25519(publicKey, canonical, sigBytes);
  return ok
    ? { ok: true }
    : { ok: false, error: { kind: 'signature-invalid', message: 'signature verification failed' } };
}
