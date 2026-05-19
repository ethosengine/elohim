/**
 * GraphQL surface for the storage-client package.
 *
 * Re-exports the viewer query documents + response type interfaces. Consumers
 * call `POST /api/v1/graphql` directly with these query strings; this module
 * intentionally ships no client library.
 */

export * from './queries';
