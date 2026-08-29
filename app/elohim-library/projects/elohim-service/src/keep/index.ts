/**
 * Keep — the client control surface for identity, storage, keys and recovery.
 *
 * Framework-free by rule, enforced by `keep-boundary.spec.ts`. The Angular
 * wiring for anything here lives in `src/angular/`, never in this directory.
 *
 * Today: the peer register (slice 1) and Answer<T> + the discovery reader
 * (slice 2). Still to come per the design's slice order: openKeep/Custodian,
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

export {
  absent,
  answerFromStatus,
  docRejected,
  isPresent,
  present,
  unreachable,
  unreachableFromError,
  valueOr,
} from './answer.js';

export type {
  Answer,
  AnswerAbsent,
  AnswerPresent,
  AnswerReason,
  AnswerUnreachable,
} from './answer.js';

export {
  CLIENT_AUTH_PATHS,
  DISCOVERY_PATH,
  isOriginRelative,
  pathDrift,
  portalUrl,
  readAuthDiscovery,
  rejectionsIn,
} from './discovery.js';

export type { PathDrift, ReadDiscoveryOptions } from './discovery.js';
