// Canonical codec
export {
  encodeCanonical,
  decodeCanonical,
  cborItemLength,
  CborItemFramer,
  decodeCanonicalSequence,
} from './cbor.js';
export { computeCid, verifyCid } from './cid.js';
// The one TypeScript address verifier (rail R-T3)
export {
  verifyAddress,
  addressForm,
  RAW_CODEC,
  DAG_CBOR_CODEC,
  type AddressForm,
} from './address.js';
export { verifyEd25519 } from './proof.js';
export {
  contentAddressHex,
  sha256BlobHash,
  verifyBlobHash,
  type BlobHashVerdict,
} from './blob-hash.js';
export { canonicalEnvelopeBytes } from './envelope.js';
export { verifyEpr, type Epr, type VerifyError } from './epr.js';

// Generated wire types
export type { Coupling } from './generated/Coupling.js';
export type { Envelope } from './generated/Envelope.js';
export type { EprKind } from './generated/EprKind.js';
export type { Reach } from './generated/Reach.js';
export type { Signature } from './generated/Signature.js';
export type { CouplingLeg } from './generated/CouplingLeg.js';

// Frame/witness primitive (G1)
export type { WitnessedInteraction } from './generated/WitnessedInteraction.js';
export type { Magnitude } from './generated/Magnitude.js';
export type { FrameClassification } from './generated/FrameClassification.js';
export type { FrameEvidence } from './generated/FrameEvidence.js';
export type { ResolutionProvenance } from './generated/ResolutionProvenance.js';
export type { Span } from './generated/Span.js';
export type { FrameVerdict } from './generated/FrameVerdict.js';
export type { ComputeSource } from './generated/ComputeSource.js';
export type { Hop } from './generated/Hop.js';
export type { ReaVerb } from './generated/ReaVerb.js';
export type { WitnessId } from './generated/WitnessId.js';
