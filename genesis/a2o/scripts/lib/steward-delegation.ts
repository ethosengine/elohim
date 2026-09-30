/** Portable signed grants. Decoding never substitutes for conductor verification. */
import { decodeHashFromBase64 } from '@holochain/client';

export interface HeadDelegationDocument {
  grantor: string;
  delegate: string;
  scope: string;
  validUntil: number;
  rootActionHash: string;
  dnaHash: string;
  signature: string;
  acceptance?: { headActionHash: string; acceptedAt: number; signature: string };
}

export function delegationWire(grant: HeadDelegationDocument): unknown {
  const signature = (value: string): Uint8Array => {
    const bytes = Buffer.from(value, 'base64');
    if (bytes.length !== 64 || bytes.toString('base64') !== value)
      throw new Error('delegation signature must be a canonical base64 Ed25519 signature');
    return new Uint8Array(bytes);
  };
  if (!Number.isSafeInteger(grant.validUntil) || grant.validUntil <= 0)
    throw new Error('delegation expiry must be an integer timestamp in microseconds');
  return {
    payload: {
      grantor: decodeHashFromBase64(grant.grantor),
      delegate: decodeHashFromBase64(grant.delegate),
      scope: grant.scope,
      valid_until: grant.validUntil,
      root_action_hash: decodeHashFromBase64(grant.rootActionHash),
      dna_hash: decodeHashFromBase64(grant.dnaHash),
    },
    signature: signature(grant.signature),
    acceptance: grant.acceptance
      ? {
          head_action_hash: decodeHashFromBase64(grant.acceptance.headActionHash),
          accepted_at: grant.acceptance.acceptedAt,
          signature: signature(grant.acceptance.signature),
        }
      : null,
  };
}
