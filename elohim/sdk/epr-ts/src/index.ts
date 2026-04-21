// Canonical codec
export { encodeCanonical, decodeCanonical } from './cbor';
export { computeCid, verifyCid } from './cid';
export { verifyEd25519 } from './proof';
export { canonicalEnvelopeBytes } from './envelope';
export { verifyEpr, type Epr, type VerifyError } from './epr';

// Generated wire types
export type { Coupling } from './generated/Coupling';
export type { Envelope } from './generated/Envelope';
export type { EprKind } from './generated/EprKind';
export type { Reach } from './generated/Reach';
export type { Signature } from './generated/Signature';
export type { CouplingLeg } from './generated/CouplingLeg';
