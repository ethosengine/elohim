import type { PutBlobResponseView } from "./generated/PutBlobResponseView.js";

/**
 * Storage client configuration
 */
export interface StorageConfig {
  /** Base URL for elohim-storage HTTP API */
  baseUrl: string;
  /** Holochain app ID for namespacing */
  hAppId: string;
  /** Optional API key for authentication */
  apiKey?: string;
  /** Request timeout in milliseconds (default: 30000) */
  timeout?: number;
}

/**
 * Document metadata from list operations
 */
export interface DocumentInfo {
  /** Document ID */
  docId: string;
  /** Document type (e.g., "graph", "path", "personal") */
  docType: string;
  /** Number of changes in the document */
  changeCount: number;
  /** Last modified timestamp (Unix millis) */
  lastModified: number;
  /** Current heads (hex-encoded change hashes) */
  heads: string[];
}

/**
 * Response from list documents endpoint
 */
export interface ListDocumentsResponse {
  /** Holochain app ID */
  hAppId: string;
  /** List of documents */
  documents: DocumentInfo[];
  /** Total count (for pagination) */
  total: number;
  /** Pagination offset */
  offset: number;
  /** Pagination limit */
  limit: number;
}

/**
 * Response from get document endpoint
 */
export interface GetDocumentResponse {
  /** Holochain app ID */
  hAppId: string;
  /** Document ID */
  docId: string;
  /** Current heads (hex-encoded change hashes) */
  heads: string[];
}

/**
 * Response from get heads endpoint
 */
export interface GetHeadsResponse {
  /** Holochain app ID */
  hAppId: string;
  /** Document ID */
  docId: string;
  /** Current heads (hex-encoded change hashes) */
  heads: string[];
}

/**
 * Response from get changes endpoint
 */
export interface GetChangesResponse {
  /** Holochain app ID */
  hAppId: string;
  /** Document ID */
  docId: string;
  /** Changes as base64-encoded blobs */
  changes: string[];
  /** New heads after applying these changes */
  newHeads: string[];
}

/**
 * Response from apply changes endpoint
 */
export interface ApplyChangesResponse {
  /** Holochain app ID */
  hAppId: string;
  /** Document ID */
  docId: string;
  /** New heads after applying changes */
  newHeads: string[];
}

/**
 * Blob storage result
 */
export interface BlobResult {
  /** Byte CID (`bafkr4i…`, raw + BLAKE3-256) — the blob's one address */
  cid: string;
  /** Size in bytes */
  size_bytes: number;
  /** Whether blob already existed */
  already_existed: boolean;
}

/**
 * Blob manifest: the `PUT /blob/{cid}` response shape (camelCase, byte CID, declared byte
 * profile, chunks and erasure stripes). Generated from the Rust view; never hand-edited.
 */
export type BlobManifest = PutBlobResponseView;

/**
 * Options for list documents
 */
export interface ListOptions {
  /** Filter by document type prefix */
  prefix?: string;
  /** Pagination offset */
  offset?: number;
  /** Pagination limit */
  limit?: number;
}

/**
 * Storage client error
 */
export class StorageError extends Error {
  constructor(
    message: string,
    public statusCode?: number,
    public responseBody?: string
  ) {
    super(message);
    this.name = 'StorageError';
  }
}
