/**
 * Real byte-profile CIDs (bafkr4i…) for test fixtures. Deterministic: each is
 * the CID of a fixed label, so assertions can compare exact strings without a
 * hand-typed (and therefore wrong-length) placeholder.
 */
import { byteCid } from '../byte-cid.js';

const cidOf = (label: string): string => byteCid(Buffer.from(label, 'utf8'));

export const CID_A = cidOf('fixture-a');
export const CID_B = cidOf('fixture-b');
export const CID_C = cidOf('fixture-c');
export const CID_D = cidOf('fixture-d');
export const CID_E = cidOf('fixture-e');
export const CID_F = cidOf('fixture-f');
