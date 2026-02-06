/**
 * Storage client configuration
 */
export interface StorageConfig {
  /** Base URL for elohim-storage HTTP API */
  baseUrl: string;
  /** Application ID for namespacing */
  appId: string;
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
  doc_id: string;
  /** Document type (e.g., "graph", "path", "personal") */
  doc_type: string;
  /** Number of changes in the document */
  change_count: number;
  /** Last modified timestamp (Unix millis) */
  last_modified: number;
  /** Current heads (hex-encoded change hashes) */
  heads: string[];
}

/**
 * Response from list documents endpoint
 */
export interface ListDocumentsResponse {
  /** Application ID */
  app_id: string;
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
  /** Application ID */
  app_id: string;
  /** Document ID */
  doc_id: string;
  /** Current heads (hex-encoded change hashes) */
  heads: string[];
}

/**
 * Response from get heads endpoint
 */
export interface GetHeadsResponse {
  /** Application ID */
  app_id: string;
  /** Document ID */
  doc_id: string;
  /** Current heads (hex-encoded change hashes) */
  heads: string[];
}

/**
 * Response from get changes endpoint
 */
export interface GetChangesResponse {
  /** Application ID */
  app_id: string;
  /** Document ID */
  doc_id: string;
  /** Changes as base64-encoded blobs */
  changes: string[];
  /** New heads after applying these changes */
  new_heads: string[];
}

/**
 * Response from apply changes endpoint
 */
export interface ApplyChangesResponse {
  /** Application ID */
  app_id: string;
  /** Document ID */
  doc_id: string;
  /** New heads after applying changes */
  new_heads: string[];
}

/**
 * Blob storage result
 */
export interface BlobResult {
  /** Content identifier (CID) */
  cid: string;
  /** SHA256 hash */
  hash: string;
  /** Size in bytes */
  size_bytes: number;
  /** Whether blob already existed */
  already_existed: boolean;
}

/**
 * Blob manifest (for sharded blobs)
 */
export interface BlobManifest {
  /** Blob hash */
  blob_hash: string;
  /** Blob CID */
  blob_cid?: string;
  /** MIME type */
  mime_type: string;
  /** Total size in bytes */
  total_size: number;
  /** Shard size */
  shard_size: number;
  /** Encoding method */
  encoding: string;
  /** List of shard hashes */
  shard_hashes: string[];
  /** Data parity shards */
  data_parity: [number, number];
  /** Reach level */
  reach: string;
}

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
