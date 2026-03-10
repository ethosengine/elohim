/**
 * Elohim Interfaces — Abstract contracts for protocol services.
 *
 * These interfaces enable inversion of control:
 * - IBlobFetcher + BLOB_FETCHER token → swap blob retrieval strategy
 * - IEprUriResolver → pure URI resolution (no network)
 * - IEprContentResolver → async content/head resolution
 * - IStorageApi + STORAGE_API token → swap storage backend
 * - IDataLoader + DATA_LOADER token → swap content loading orchestration
 * - ICustodianSelector + CUSTODIAN_SELECTOR token → swap custodian selection
 */

export { BLOB_FETCHER } from './blob-fetcher.interface';
export type { IBlobFetcher } from './blob-fetcher.interface';
export { CUSTODIAN_SELECTOR } from './custodian.interface';
export type { ICustodianSelector } from './custodian.interface';
export { DATA_LOADER } from './data-loader.interface';
export type { IDataLoader } from './data-loader.interface';
export type { IEprUriResolver, IEprContentResolver } from './epr-resolver.interface';
export { STORAGE_API } from './storage-api.interface';
export type {
  IStorageApi,
  ContentFilters,
  PathFilters,
  RelationshipFilters,
} from './storage-api.interface';
export { STORAGE_WRITER } from './storage-writer.interface';
export type {
  IStorageWriter,
  CreatePresenceInput,
  CreateMasteryInput,
  CreateEventInput,
} from './storage-writer.interface';
export { LEARNER_BACKEND } from './learner-backend.interface';
export type { ILearnerBackend } from './learner-backend.interface';
export { GOVERNANCE } from './governance.interface';
export type { IGovernance } from './governance.interface';
export { CONTENT_ATTESTATION } from './content-attestation.interface';
export type { IContentAttestation } from './content-attestation.interface';
