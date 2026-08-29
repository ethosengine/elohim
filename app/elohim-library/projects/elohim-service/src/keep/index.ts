/**
 * Keep — the client control surface for identity, storage, keys and recovery.
 *
 * Framework-free by rule, enforced by `keep-boundary.spec.ts`. The Angular
 * wiring for anything here lives in `src/angular/`, never in this directory.
 *
 * Today this exports the peer register only. See the design's slice order for
 * what joins it: Answer<T> and the discovery reader, then openKeep/Custodian,
 * then Witnesses.
 */

export {
  FederationPeerResolver,
  identityOf,
  peerOf,
  resolutionOf,
  trustOf,
} from './peer-register.js';

export type {
  FederationDoorwayRow,
  FederationDoorwaysResponseShape,
  FederationPeerResolverOptions,
  KeepPeer,
  PeerTrust,
} from './peer-register.js';
