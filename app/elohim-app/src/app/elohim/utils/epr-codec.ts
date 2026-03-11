/**
 * EPR IPLD Codec — DAG-CBOR encoding for EPR Heads in the browser.
 *
 * Provides encode/decode between TypeScript EprHead objects and DAG-CBOR bytes,
 * plus CID computation with the correct IPLD codec (0x71 = dag-cbor).
 *
 * ## Multicodec Constants
 *
 * EPR defines private-use multicodec values (matching Rust epr_codec.rs):
 * - `0x300001` — EPR Head
 * - `0x300002` — EPR Document
 * - `0x300003` — EPR Relationship
 *
 * See: protocol-specification.md Appendix F
 */

import * as dagCbor from '@ipld/dag-cbor';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

import type { EprHead } from '../models/epr-head.model';

// ── Multicodec Constants ────────────────────────────────────────────────────

/** DAG-CBOR multicodec (standard IPLD codec) */
export const DAG_CBOR_CODEC = 0x71;

/** EPR Head multicodec (private-use range) */
export const EPR_HEAD_CODEC = 0x300001;

/** EPR Document multicodec (private-use range) */
export const EPR_DOCUMENT_CODEC = 0x300002;

/** EPR Relationship multicodec (private-use range) */
export const EPR_RELATIONSHIP_CODEC = 0x300003;

// ── Encode / Decode ─────────────────────────────────────────────────────────

/**
 * Encode an EPR Head as DAG-CBOR and compute its CIDv1 (codec 0x71).
 *
 * Returns the raw CBOR bytes and the CID.
 */
export async function encodeEprHead(head: EprHead): Promise<{ bytes: Uint8Array; cid: CID }> {
  const bytes = dagCbor.encode(head);
  const hash = await sha256.digest(bytes);
  const cid = CID.create(1, DAG_CBOR_CODEC, hash);
  return { bytes, cid };
}

/**
 * Decode an EPR Head from bytes, auto-detecting JSON (0x7B) vs DAG-CBOR.
 *
 * This enables backward compatibility: existing JSON-encoded heads can be
 * decoded alongside new DAG-CBOR heads.
 */
export function decodeEprHead(bytes: Uint8Array): EprHead {
  if (bytes.length === 0) {
    throw new Error('Empty input');
  }

  if (bytes[0] === 0x7b) {
    // JSON (starts with '{')
    const text = new TextDecoder().decode(bytes);
    return JSON.parse(text) as EprHead;
  }

  // DAG-CBOR
  return dagCbor.decode<EprHead>(bytes);
}

/**
 * Check if a CID uses the DAG-CBOR codec (0x71).
 */
export function isDagCborCid(cid: CID): boolean {
  return cid.code === DAG_CBOR_CODEC;
}

/**
 * Check if a CID string represents a DAG-CBOR encoded document.
 *
 * DAG-CBOR CIDs start with 'bafyr' in base32 encoding (vs 'bafkrei' for raw).
 */
export function isDagCborCidString(cidStr: string): boolean {
  return cidStr.startsWith('bafyr');
}
