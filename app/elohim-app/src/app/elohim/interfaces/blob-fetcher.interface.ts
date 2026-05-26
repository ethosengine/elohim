/**
 * IBlobFetcher — Abstract interface for CID-verified blob retrieval.
 *
 * Slice 2.1c: Interface + token re-exported from @elohim/service so that
 * lamad services can inject via token without a cross-pillar import.
 *
 * The factory (HeliaFetchService) is registered explicitly in elohim-app's
 * app.config.ts via useClass, since the factory cannot live in the shared
 * library (it references an elohim-app-internal service).
 */

// Re-export interface + token from shared SDK surface
export type { IBlobFetcher } from '@elohim/service';
export { BLOB_FETCHER } from '@elohim/service';

// Re-export concrete impl for app.config.ts registration
export { HeliaFetchService } from '../services/helia-fetch.service';
