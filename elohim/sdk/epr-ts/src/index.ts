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

// Frame/witness primitive (G1)
export type { WitnessedInteraction } from './generated/WitnessedInteraction';
export type { Magnitude } from './generated/Magnitude';
export type { FrameClassification } from './generated/FrameClassification';
export type { FrameEvidence } from './generated/FrameEvidence';
export type { ResolutionProvenance } from './generated/ResolutionProvenance';
export type { Span } from './generated/Span';
export type { FrameVerdict } from './generated/FrameVerdict';
export type { ComputeSource } from './generated/ComputeSource';
export type { Hop } from './generated/Hop';
export type { ReaVerb } from './generated/ReaVerb';
export type { WitnessId } from './generated/WitnessId';
