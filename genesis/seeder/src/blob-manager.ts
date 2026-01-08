/**
 * Blob Manager - Content extraction, hashing, and management
 *
 * Handles the separation of metadata (DNA) from content blobs (projection cache).
 *
 * Content Architecture:
 * - DNA stores metadata with blob_hash references
 * - holochain-cache-core stores and serves actual blob content
 * - Clients use ContentResolver to find content across tiers
 *
 * Blob-extractable formats:
 * - html5-app: Zip files with entry_point
 * - perseus-quiz-json: Large quiz JSON
 * - gherkin: Feature file content (usually small, may stay inline)
 *
 * Usage:
 *   const manager = new BlobManager({ doorwayUrl: 'https://doorway.example.com' });
 *   const result = await manager.processContent(content);
 *   // result.metadata -> seed to DNA
 *   // result.blob -> push to projection cache
 */

import * as fs from 'fs';
import * as path from 'path';
import * as crypto from 'crypto';

// =============================================================================
// Types
// =============================================================================

export interface BlobMetadata {
  /** SHA256 hash of the blob content */
  hash: string;
  /** Size in bytes */
  sizeBytes: number;
  /** MIME type */
  mimeType: string;
  /** Entry point for compound content (e.g., "index.html" for html5-app) */
  entryPoint?: string;
  /** Fallback URL if resolution fails */
  fallbackUrl?: string;
}

export interface ProcessedContent {
  /** Metadata to store in DNA (blob_hash reference instead of inline content) */
  metadata: Record<string, unknown>;
  /** Blob to push to projection cache (null if content stays inline) */
  blob: Buffer | null;
  /** Blob metadata (null if content stays inline) */
  blobMetadata: BlobMetadata | null;
  /** Whether content was extracted to blob */
  extracted: boolean;
}

export interface BlobManagerConfig {
  /** Doorway URL for pushing blobs */
  doorwayUrl: string;
  /** API key for doorway authentication */
  apiKey?: string;
  /** Minimum size (bytes) to extract as blob (default: 10KB) */
  minBlobSize?: number;
  /** Directory for cached blobs */
  cacheDir?: string;
  /** Dry run - don't actually push blobs */
  dryRun?: boolean;
}

export interface ContentFile {
  id: string;
  contentType?: string;
  contentFormat?: string;
  title?: string;
  description?: string;
  content?: string | object;
  blob_hash?: string;
  blob_url?: string;
  entry_point?: string;
  fallback_url?: string;
  [key: string]: unknown;
}

// Formats that should be extracted as blobs
const BLOB_FORMATS = ['html5-app', 'perseus-quiz-json'];

// MIME types by format
const FORMAT_MIME_TYPES: Record<string, string> = {
  'html5-app': 'application/zip',
  'perseus-quiz-json': 'application/json',
  'gherkin': 'text/plain',
  'markdown': 'text/markdown',
  'plaintext': 'text/plain',
};

// =============================================================================
// Blob Manager
// =============================================================================

export class BlobManager {
  private config: Required<BlobManagerConfig>;
  private blobCache: Map<string, BlobMetadata> = new Map();

  constructor(config: BlobManagerConfig) {
    this.config = {
      doorwayUrl: config.doorwayUrl,
      apiKey: config.apiKey || '',
      minBlobSize: config.minBlobSize || 10 * 1024, // 10KB default
      cacheDir: config.cacheDir || '.blob-cache',
      dryRun: config.dryRun || false,
    };

    // Ensure cache directory exists
    if (!fs.existsSync(this.config.cacheDir)) {
      fs.mkdirSync(this.config.cacheDir, { recursive: true });
    }
  }

  /**
   * Process content file and determine if blob extraction is needed.
   *
   * @param content - The content file object
   * @param contentDir - Directory containing content files (for resolving blob files)
   * @returns ProcessedContent with metadata and optional blob
   */
  async processContent(content: ContentFile, contentDir: string): Promise<ProcessedContent> {
    const format = content.contentFormat || 'markdown';

    // Check if this format should be extracted as blob
    if (!BLOB_FORMATS.includes(format)) {
      return {
        metadata: content,
        blob: null,
        blobMetadata: null,
        extracted: false,
      };
    }

    // Check if content already has a blob reference
    if (content.blob_hash) {
      return {
        metadata: content,
        blob: null,
        blobMetadata: null,
        extracted: false, // Already has reference
      };
    }

    // Extract blob based on format
    let blob: Buffer;
    let entryPoint: string | undefined;

    if (format === 'html5-app') {
      // HTML5 app: look for zip file or blob file
      const blobResult = await this.extractHtml5AppBlob(content, contentDir);
      if (!blobResult) {
        // No blob to extract, keep as inline
        return {
          metadata: content,
          blob: null,
          blobMetadata: null,
          extracted: false,
        };
      }
      blob = blobResult.blob;
      entryPoint = blobResult.entryPoint || 'index.html';
    } else if (format === 'perseus-quiz-json') {
      // Perseus quiz: serialize content as JSON blob
      const blobResult = this.extractPerseusQuizBlob(content);
      if (!blobResult) {
        return {
          metadata: content,
          blob: null,
          blobMetadata: null,
          extracted: false,
        };
      }
      blob = blobResult;
    } else {
      return {
        metadata: content,
        blob: null,
        blobMetadata: null,
        extracted: false,
      };
    }

    // Check minimum size
    if (blob.length < this.config.minBlobSize) {
      return {
        metadata: content,
        blob: null,
        blobMetadata: null,
        extracted: false,
      };
    }

    // Compute hash
    const hash = this.computeHash(blob);
    const mimeType = FORMAT_MIME_TYPES[format] || 'application/octet-stream';

    const blobMetadata: BlobMetadata = {
      hash,
      sizeBytes: blob.length,
      mimeType,
      entryPoint,
      fallbackUrl: content.fallback_url as string | undefined,
    };

    // Create metadata with blob reference (remove inline content)
    const metadata: Record<string, unknown> = {
      ...content,
      blob_hash: hash,
      blob_url: `${this.config.doorwayUrl}/store/${hash}`,
    };

    if (entryPoint) {
      metadata.entry_point = entryPoint;
    }

    // Remove inline content from metadata
    delete metadata.content;

    // Cache blob locally
    this.cacheBlob(hash, blob);
    this.blobCache.set(hash, blobMetadata);

    return {
      metadata,
      blob,
      blobMetadata,
      extracted: true,
    };
  }

  /**
   * Extract blob from HTML5 app content.
   * Looks for (in order):
   * 1. metadata.localZipPath - relative to genesis directory
   * 2. A corresponding zip file in the content directory ({id}.zip)
   * 3. A blob_file reference in the content
   */
  private async extractHtml5AppBlob(
    content: ContentFile,
    contentDir: string
  ): Promise<{ blob: Buffer; entryPoint?: string; appId?: string } | null> {
    // Get appId from content.content object (for html5-app format)
    const contentObj = typeof content.content === 'object' ? content.content as Record<string, unknown> : null;
    const appId = contentObj?.appId as string | undefined;
    const entryPoint = contentObj?.entryPoint as string | undefined || (content.entry_point as string) || 'index.html';

    // Try metadata.localZipPath first (relative to genesis directory)
    const metadata = content.metadata as Record<string, unknown> | undefined;
    if (metadata?.localZipPath) {
      // Genesis directory is parent of seeder directory
      const genesisDir = path.resolve(path.dirname(path.dirname(contentDir)));
      const zipPath = path.join(genesisDir, metadata.localZipPath as string);
      if (fs.existsSync(zipPath)) {
        console.log(`   📦 Found ZIP via metadata.localZipPath: ${metadata.localZipPath}`);
        return {
          blob: fs.readFileSync(zipPath),
          entryPoint,
          appId,
        };
      }
    }

    // Try to find a zip file with same ID in content directory
    const zipPath = path.join(contentDir, `${content.id}.zip`);
    if (fs.existsSync(zipPath)) {
      return {
        blob: fs.readFileSync(zipPath),
        entryPoint,
        appId,
      };
    }

    // Try blob_file reference
    if (content.blob_file) {
      const blobPath = path.join(contentDir, content.blob_file as string);
      if (fs.existsSync(blobPath)) {
        return {
          blob: fs.readFileSync(blobPath),
          entryPoint,
          appId,
        };
      }
    }

    // No local blob found
    return null;
  }

  /**
   * Extract perseus quiz content as blob.
   */
  private extractPerseusQuizBlob(content: ContentFile): Buffer | null {
    if (!content.content) {
      return null;
    }

    // Serialize content to JSON
    const contentStr = typeof content.content === 'string'
      ? content.content
      : JSON.stringify(content.content, null, 2);

    return Buffer.from(contentStr, 'utf-8');
  }

  /**
   * Compute SHA256 hash of blob content.
   */
  computeHash(data: Buffer): string {
    const hash = crypto.createHash('sha256').update(data).digest('hex');
    return `sha256-${hash}`;
  }

  /**
   * Cache blob to local filesystem.
   */
  private cacheBlob(hash: string, data: Buffer): void {
    const cachePath = path.join(this.config.cacheDir, hash);
    fs.writeFileSync(cachePath, data);
  }

  /**
   * Get cached blob by hash.
   */
  getCachedBlob(hash: string): Buffer | null {
    const cachePath = path.join(this.config.cacheDir, hash);
    if (fs.existsSync(cachePath)) {
      return fs.readFileSync(cachePath);
    }
    return null;
  }

  /**
   * Get blob metadata by hash.
   */
  getBlobMetadata(hash: string): BlobMetadata | undefined {
    return this.blobCache.get(hash);
  }

  /**
   * Get all cached blob hashes.
   */
  getCachedBlobHashes(): string[] {
    return Array.from(this.blobCache.keys());
  }

  /**
   * Get statistics about cached blobs.
   */
  getStats(): {
    totalBlobs: number;
    totalBytes: number;
    byFormat: Record<string, number>;
  } {
    let totalBytes = 0;
    const byFormat: Record<string, number> = {};

    for (const [, metadata] of Array.from(this.blobCache.entries())) {
      totalBytes += metadata.sizeBytes;
      const format = metadata.mimeType;
      byFormat[format] = (byFormat[format] || 0) + 1;
    }

    return {
      totalBlobs: this.blobCache.size,
      totalBytes,
      byFormat,
    };
  }
}

// =============================================================================
// Validation Functions
// =============================================================================

/**
 * Validate blob hash format.
 */
export function isValidBlobHash(hash: string): boolean {
  return /^sha256-[a-f0-9]{64}$/.test(hash);
}

/**
 * Validate content has required blob fields for its format.
 */
export function validateBlobReferences(content: ContentFile): {
  valid: boolean;
  errors: string[];
  warnings: string[];
} {
  const errors: string[] = [];
  const warnings: string[] = [];
  const format = content.contentFormat;

  if (!format || !BLOB_FORMATS.includes(format)) {
    return { valid: true, errors, warnings };
  }

  // For blob formats, should have either inline content OR blob reference
  const hasInlineContent = content.content !== undefined && content.content !== null;
  const hasBlobRef = content.blob_hash !== undefined && content.blob_hash !== null;

  if (!hasInlineContent && !hasBlobRef) {
    errors.push(`${format} content must have either 'content' or 'blob_hash'`);
  }

  if (hasBlobRef) {
    if (!isValidBlobHash(content.blob_hash!)) {
      errors.push(`Invalid blob_hash format: ${content.blob_hash}`);
    }

    // Warn if no fallback URL for external content
    if (!content.fallback_url) {
      warnings.push(`No fallback_url for blob content (recommended for resilience)`);
    }
  }

  // html5-app should have entry_point
  if (format === 'html5-app' && hasBlobRef && !content.entry_point) {
    warnings.push(`html5-app should have 'entry_point' (defaulting to 'index.html')`);
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

export default BlobManager;
