/** Portable signed grants. Decoding never substitutes for conductor verification. */
import { decodeHashFromBase64, encodeHashToBase64 } from '@holochain/client';

export interface HeadDelegationDocument {
  grantor: string;
  delegate: string;
  scope: string;
  validUntil: number;
  rootActionHash: string;
  dnaHash: string;
  /** Native grant-issuance action; absent only on historical v2 receipts. */
  issuanceActionHash?: string;
  signature: string;
  acceptance?: {
    headActionHash: string;
    witnessActionHash: string;
    acceptedAt: number;
    signature: string;
  };
}

export interface HeadDelegationWire {
  payload: {
    grantor: Uint8Array;
    delegate: Uint8Array;
    scope: string;
    valid_until: number;
    root_action_hash: Uint8Array;
    dna_hash: Uint8Array;
    issuance_action_hash?: Uint8Array | null;
  };
  signature: Uint8Array;
  acceptance: {
    head_action_hash: Uint8Array;
    witness_action_hash: Uint8Array;
    accepted_at: number;
    signature: Uint8Array;
  } | null;
}

/** Preserve the conductor's complete signed statement in a portable receipt. */
export function delegationDocument(grant: HeadDelegationWire): HeadDelegationDocument {
  const document: HeadDelegationDocument = {
    grantor: encodeHashToBase64(grant.payload.grantor),
    delegate: encodeHashToBase64(grant.payload.delegate),
    scope: grant.payload.scope,
    validUntil: grant.payload.valid_until,
    rootActionHash: encodeHashToBase64(grant.payload.root_action_hash),
    dnaHash: encodeHashToBase64(grant.payload.dna_hash),
    ...(grant.payload.issuance_action_hash
      ? { issuanceActionHash: encodeHashToBase64(grant.payload.issuance_action_hash) }
      : {}),
    signature: Buffer.from(grant.signature).toString('base64'),
    acceptance: grant.acceptance
      ? {
          headActionHash: encodeHashToBase64(grant.acceptance.head_action_hash),
          witnessActionHash: encodeHashToBase64(grant.acceptance.witness_action_hash),
          acceptedAt: grant.acceptance.accepted_at,
          signature: Buffer.from(grant.acceptance.signature).toString('base64'),
        }
      : undefined,
  };
  delegationWire(document);
  return document;
}

export function delegationWire(grant: HeadDelegationDocument): HeadDelegationWire {
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
      ...(grant.issuanceActionHash
        ? { issuance_action_hash: decodeHashFromBase64(grant.issuanceActionHash) }
        : {}),
    },
    signature: signature(grant.signature),
    acceptance: grant.acceptance
      ? {
          head_action_hash: decodeHashFromBase64(grant.acceptance.headActionHash),
          witness_action_hash: decodeHashFromBase64(grant.acceptance.witnessActionHash),
          accepted_at: grant.acceptance.acceptedAt,
          signature: signature(grant.acceptance.signature),
        }
      : null,
  };
}
