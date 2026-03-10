/**
 * @elohim/storage-client
 *
 * TypeScript client for elohim-storage sync API
 *
 * @example
 * ```typescript
 * import { StorageClient, AutomergeSync } from '@elohim/storage-client';
 *
 * // Create client
 * const client = new StorageClient({
 *   baseUrl: 'http://localhost:8080',
 *   appId: 'lamad',
 * });
 *
 * // List documents
 * const { documents } = await client.listDocuments({ prefix: 'graph' });
 *
 * // Use Automerge sync helper
 * const sync = new AutomergeSync(client);
 * let doc = await sync.load('graph:my-doc');
 * doc = Automerge.change(doc, d => { d.title = 'Updated'; });
 * await sync.save('graph:my-doc', doc);
 * ```
 */

// Types
export {
  StorageConfig,
  StorageError,
  DocumentInfo,
  ListDocumentsResponse,
  GetDocumentResponse,
  GetHeadsResponse,
  GetChangesResponse,
  ApplyChangesResponse,
  BlobResult,
  BlobManifest,
  ListOptions,
} from './types';

// HTTP Client
export { StorageClient } from './client';

// Automerge Sync
export { AutomergeSync, SyncResult, createSync } from './sync';
