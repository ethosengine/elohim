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
  deviceBinding?: string;
  exercise?: { requester: string; executor: string; policy: string };
  signature: string;
  acceptance?: {
    headActionHash: string;
    witnessActionHash: string;
    acceptedAt: number;
    deviceWitnessActionHash?: string;
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
    device_binding?: Uint8Array | null;
    exercise?: { requester: Uint8Array; executor: Uint8Array; policy: string } | null;
  };
  signature: Uint8Array;
  acceptance: {
    head_action_hash: Uint8Array;
    witness_action_hash: Uint8Array;
    accepted_at: number;
    device_witness_action_hash?: Uint8Array | null;
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
    ...(grant.payload.device_binding
      ? { deviceBinding: encodeHashToBase64(grant.payload.device_binding) }
      : {}),
    ...(grant.payload.exercise
      ? {
          exercise: {
            requester: encodeHashToBase64(grant.payload.exercise.requester),
            executor: encodeHashToBase64(grant.payload.exercise.executor),
            policy: grant.payload.exercise.policy,
          },
        }
      : {}),
    acceptance: grant.acceptance
      ? {
          headActionHash: encodeHashToBase64(grant.acceptance.head_action_hash),
          witnessActionHash: encodeHashToBase64(grant.acceptance.witness_action_hash),
          acceptedAt: grant.acceptance.accepted_at,
          ...(grant.acceptance.device_witness_action_hash
            ? {
                deviceWitnessActionHash: encodeHashToBase64(
                  grant.acceptance.device_witness_action_hash
                ),
              }
            : {}),
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
      ...(grant.deviceBinding ? { device_binding: decodeHashFromBase64(grant.deviceBinding) } : {}),
      ...(grant.exercise
        ? {
            exercise: {
              requester: decodeHashFromBase64(grant.exercise.requester),
              executor: decodeHashFromBase64(grant.exercise.executor),
              policy: grant.exercise.policy,
            },
          }
        : {}),
    },
    signature: signature(grant.signature),
    acceptance: grant.acceptance
      ? {
          head_action_hash: decodeHashFromBase64(grant.acceptance.headActionHash),
          witness_action_hash: decodeHashFromBase64(grant.acceptance.witnessActionHash),
          accepted_at: grant.acceptance.acceptedAt,
          ...(grant.acceptance.deviceWitnessActionHash
            ? {
                device_witness_action_hash: decodeHashFromBase64(
                  grant.acceptance.deviceWitnessActionHash
                ),
              }
            : {}),
          signature: signature(grant.acceptance.signature),
        }
      : null,
  };
}

/** Private discovery hints, never a registry or substitute for native preflight. */
export interface CanonicalRootHint {
  root: string;
  rootAuthor: string;
}
export function canonicalRootHints(value: unknown): Record<string, CanonicalRootHint> {
  const rows = (value as { canonicalRoots?: unknown } | null)?.canonicalRoots;
  if (!Array.isArray(rows)) throw new Error('Canonical roots must contain a canonicalRoots array');
  const result: Record<string, CanonicalRootHint> = Object.create(null) as Record<
    string,
    CanonicalRootHint
  >;
  for (const row of rows as unknown[]) {
    const item = row as { id?: unknown; root?: unknown; rootAuthor?: unknown } | null;
    if (
      !item ||
      typeof item.id !== 'string' ||
      !item.id ||
      typeof item.root !== 'string' ||
      typeof item.rootAuthor !== 'string' ||
      Object.hasOwn(result, item.id)
    )
      throw new Error('Invalid or duplicate canonical root hint');
    for (const [text, prefix] of [
      [item.root, 41],
      [item.rootAuthor, 32],
    ] as const) {
      const bytes = decodeHashFromBase64(text);
      if (bytes.length !== 39 || bytes[0] !== 132 || bytes[1] !== prefix || bytes[2] !== 36)
        throw new Error('Canonical hint must name a native Create action and author key');
    }
    result[item.id] = { root: item.root, rootAuthor: item.rootAuthor };
  }
  return result;
}

/** Verify the exact native Create lineage. Canonical-ID authority is checked
 * separately by preflight_head_publication, never inferred from this read. */
export async function verifyRootHint(
  conductor: Pick<import('./steward-conductor.js').Conductor, 'call'>,
  id: string,
  hint: CanonicalRootHint,
  local = false
): Promise<void> {
  const lineage = await conductor.call<{
    content_id: string;
    referenced_action_hash: Uint8Array;
    root_action_hash: Uint8Array;
    root_author: Uint8Array;
    truncated: boolean;
  }>('get_content_lineage', { action_hash: decodeHashFromBase64(hint.root), local });
  if (
    lineage.content_id !== id ||
    lineage.truncated ||
    encodeHashToBase64(lineage.referenced_action_hash) !== hint.root ||
    encodeHashToBase64(lineage.root_action_hash) !== hint.root ||
    encodeHashToBase64(lineage.root_author) !== hint.rootAuthor
  )
    throw new Error(`Incomplete or mismatched native root hint for ${id}`);
}
