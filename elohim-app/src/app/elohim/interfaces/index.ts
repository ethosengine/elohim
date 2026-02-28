/**
 * Elohim Interfaces — Abstract contracts for protocol services.
 *
 * These interfaces enable inversion of control:
 * - IBlobFetcher + BLOB_FETCHER token → swap blob retrieval strategy
 * - IEprUriResolver → pure URI resolution (no network)
 * - IEprContentResolver → async content/head resolution
 */

export { BLOB_FETCHER } from './blob-fetcher.interface';
export type { IBlobFetcher } from './blob-fetcher.interface';
export type { IEprUriResolver, IEprContentResolver } from './epr-resolver.interface';
