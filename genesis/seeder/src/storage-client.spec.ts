/**
 * Storage Client Tests
 *
 * Tests for StorageClient covering:
 * - Health checks
 * - Shard operations
 * - Blob upload and download
 * - Batch operations
 * - Error handling and retries
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  StorageClient,
  validateStorageNode,
  type StorageClientConfig,
  type ShardManifest,
} from './storage-client.js';
import * as crypto from 'crypto';
import { CID_A, CID_B, CID_C, CID_D, CID_E } from './__tests__/fixture-cids.js';

// Mock global fetch
global.fetch = vi.fn();

/** Storage's `PUT /blob/{cid}` answer (PutBlobResponseView) for an unstriped blob. */
function putAnswer(cid: string, len: number, mimeType = 'text/plain') {
  return {
    blobCid: cid,
    byteLen: len,
    mimeType,
    profile: {
      epoch: 0,
      chunker: { id: 'fastcdc-2020', min: 65536, avg: 262144, max: 1048576, level: 1 },
      erasure: { id: 'none', data: 0, parity: 0 },
    },
    chunks: [{ cid, len }],
    stripes: [],
    reach: 'commons',
    authorId: null,
    createdAt: '2026-11-06T00:00:00Z',
    verifiedAt: null,
  };
}

describe('StorageClient', () => {
  let client: StorageClient;
  let config: StorageClientConfig;

  beforeEach(() => {
    vi.clearAllMocks();

    config = {
      baseUrl: 'http://localhost:8090',
      timeout: 5000,
      retries: 1,
    };

    client = new StorageClient(config);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Constructor', () => {
    it('should create instance with provided config', () => {
      expect(client).toBeInstanceOf(StorageClient);
    });

    it('should remove trailing slash from baseUrl', () => {
      const clientWithSlash = new StorageClient({
        baseUrl: 'http://localhost:8090/',
      });
      expect(clientWithSlash).toBeInstanceOf(StorageClient);
    });

    it('should apply default config values', () => {
      const minimalClient = new StorageClient({ baseUrl: 'http://localhost:8090' });
      expect(minimalClient).toBeInstanceOf(StorageClient);
    });

    it('should support dry run mode', () => {
      const dryRunClient = new StorageClient({ baseUrl: 'http://localhost:8090', dryRun: true });
      expect(dryRunClient).toBeInstanceOf(StorageClient);
    });
  });

  describe('computeCid()', () => {
    it('should compute the byte-profile CID of data', () => {
      const data = Buffer.from('test data');
      const hash = StorageClient.computeCid(data);

      expect(hash).toMatch(/^bafkr4i[a-z2-7]{52}$/);
    });

    it('should produce consistent hashes for same data', () => {
      const data = Buffer.from('test data');
      const hash1 = StorageClient.computeCid(data);
      const hash2 = StorageClient.computeCid(data);

      expect(hash1).toBe(hash2);
    });

    it('should produce different hashes for different data', () => {
      const data1 = Buffer.from('test data 1');
      const data2 = Buffer.from('test data 2');
      const hash1 = StorageClient.computeCid(data1);
      const hash2 = StorageClient.computeCid(data2);

      expect(hash1).not.toBe(hash2);
    });

    it('should handle empty buffer', () => {
      const data = Buffer.from('');
      const hash = StorageClient.computeCid(data);

      expect(hash).toMatch(/^bafkr4i[a-z2-7]{52}$/);
    });
  });

  describe('checkHealth()', () => {
    it('should return healthy status on success', async () => {
      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => ({
          status: 'ok',
          blobs: 100,
          bytes: 1024000,
          manifests: 95,
        }),
      });

      const health = await client.checkHealth();

      expect(health.healthy).toBe(true);
      expect(health.blobs).toBe(100);
      expect(health.bytes).toBe(1024000);
      expect(health.manifests).toBe(95);
    });

    it('should return unhealthy on HTTP error', async () => {
      (global.fetch as any).mockResolvedValue({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
      });

      const health = await client.checkHealth();

      expect(health.healthy).toBe(false);
      expect(health.error).toContain('500');
    });

    it('should return unhealthy on network error', async () => {
      (global.fetch as any).mockRejectedValue(new Error('Network timeout'));

      const health = await client.checkHealth();

      expect(health.healthy).toBe(false);
      expect(health.error).toContain('Network timeout');
    });
  });

  describe('shardExists()', () => {
    it('should return true when shard exists', async () => {
      (global.fetch as any).mockResolvedValue({
        status: 200,
      });

      const exists = await client.shardExists(CID_A);

      expect(exists).toBe(true);
      expect(global.fetch).toHaveBeenCalledWith(
        `http://localhost:8090/shard/${CID_A}`,
        expect.objectContaining({ method: 'HEAD' })
      );
    });

    it('should return false when shard does not exist', async () => {
      (global.fetch as any).mockResolvedValue({
        status: 404,
      });

      const exists = await client.shardExists(CID_B);

      expect(exists).toBe(false);
    });

    it('should return false on network error', async () => {
      (global.fetch as any).mockRejectedValue(new Error('Network error'));

      const exists = await client.shardExists(CID_A);

      expect(exists).toBe(false);
    });
  });

  describe('pushShard()', () => {
    it('should upload shard successfully', async () => {
      const data = Buffer.from('test shard data');
      const hash = StorageClient.computeCid(data);

      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => ({
          cid: hash,
          size_bytes: data.length,
          already_existed: false,
        }),
      });

      const result = await client.pushShard(data);

      expect(result.success).toBe(true);
      expect(result.cid).toBe(hash);
      expect(result.sizeBytes).toBe(data.length);
      expect(result.alreadyExisted).toBe(false);
    });

    it('should handle already existing shard', async () => {
      const data = Buffer.from('test data');
      const hash = StorageClient.computeCid(data);

      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => ({
          cid: hash,
          size_bytes: data.length,
          already_existed: true,
        }),
      });

      const result = await client.pushShard(data);

      expect(result.success).toBe(true);
      expect(result.alreadyExisted).toBe(true);
    });

    it('should handle upload failure', async () => {
      const data = Buffer.from('test data');

      (global.fetch as any).mockResolvedValue({
        ok: false,
        status: 500,
        text: async () => 'Internal server error',
      });

      const result = await client.pushShard(data);

      expect(result.success).toBe(false);
      expect(result.error).toContain('500');
    });

    it('should handle network error', async () => {
      const data = Buffer.from('test data');

      (global.fetch as any).mockRejectedValue(new Error('Connection refused'));

      const result = await client.pushShard(data);

      expect(result.success).toBe(false);
      expect(result.error).toContain('Connection refused');
    });

    it('should use provided hash if given', async () => {
      const data = Buffer.from('test data');
      const expectedHash = CID_C;

      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => ({
          cid: expectedHash,
          size_bytes: data.length,
          already_existed: false,
        }),
      });

      const result = await client.pushShard(data, expectedHash);

      expect(result.cid).toBe(expectedHash);
    });

    it('should skip upload in dry run mode', async () => {
      const dryClient = new StorageClient({ ...config, dryRun: true });
      const data = Buffer.from('test data');

      const result = await dryClient.pushShard(data);

      expect(result.success).toBe(true);
      expect(global.fetch).not.toHaveBeenCalled();
    });
  });

  describe('pushBlob()', () => {
    it('should upload blob and return manifest', async () => {
      const data = Buffer.from('test blob data');
      const hash = StorageClient.computeCid(data);

      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => putAnswer(hash, data.length),
      });

      const result = await client.pushBlob(data, 'text/plain', 'commons');

      expect(result.success).toBe(true);
      expect(result.manifest).toBeDefined();
      expect(result.manifest?.blob_cid).toBe(hash);
      expect(result.manifest?.byte_len).toBe(data.length);
      expect(result.manifest?.chunks).toEqual([{ cid: hash, len: data.length }]);
      expect(result.manifest?.stripes).toEqual([]);
      expect(result.manifest?.reach).toBe('commons');
    });

    it('should handle blob upload failure', async () => {
      const data = Buffer.from('test data');

      (global.fetch as any).mockResolvedValue({
        ok: false,
        status: 413,
        text: async () => 'Payload too large',
      });

      const result = await client.pushBlob(data, 'text/plain');

      expect(result.success).toBe(false);
      expect(result.error).toContain('413');
    });

    it('should return mock manifest in dry run mode', async () => {
      const dryClient = new StorageClient({ ...config, dryRun: true });
      const data = Buffer.from('test data');

      const result = await dryClient.pushBlob(data, 'text/plain');

      expect(result.success).toBe(true);
      expect(result.manifest).toBeDefined();
      expect(global.fetch).not.toHaveBeenCalled();
    });

    it('should use default reach value', async () => {
      const data = Buffer.from('test data');

      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => putAnswer(CID_D, data.length),
      });

      const result = await client.pushBlob(data, 'text/plain');

      expect(result.manifest?.reach).toBe('commons');
    });

    it('does not surface any legacy second-hash field', async () => {
      (global.fetch as any).mockResolvedValueOnce({
        ok: true,
        status: 201,
        json: async () => putAnswer(CID_E, 4),
      });

      const result = await client.pushBlob(Buffer.from('test'), 'text/plain');
      expect(result.success).toBe(true);
      expect(Object.keys(result)).toEqual(['success', 'manifest']);
    });
  });

  describe('getShard()', () => {
    it('should retrieve shard data', async () => {
      const encoder = new TextEncoder();
      const encoded = encoder.encode('shard data');
      const arrayBuffer = encoded.buffer.slice(encoded.byteOffset, encoded.byteOffset + encoded.byteLength);

      (global.fetch as any).mockResolvedValue({
        ok: true,
        arrayBuffer: async () => arrayBuffer,
      });

      const result = await client.getShard(CID_A);

      expect(result).toBeInstanceOf(Buffer);
      expect(result?.toString()).toBe('shard data');
    });

    it('should return null when shard not found', async () => {
      (global.fetch as any).mockResolvedValue({
        ok: false,
        status: 404,
      });

      const result = await client.getShard(CID_B);

      expect(result).toBeNull();
    });

    it('should return null on network error', async () => {
      (global.fetch as any).mockRejectedValue(new Error('Network error'));

      const result = await client.getShard(CID_A);

      expect(result).toBeNull();
    });
  });

  describe('getManifest()', () => {
    it('should retrieve blob manifest', async () => {
      const mockManifest: ShardManifest = {
        blob_cid: CID_D,
        byte_len: 1024,
        mime_type: 'text/plain',
        chunker: { id: 'fastcdc-2020', min: 65536, avg: 262144, max: 1048576, level: 1 },
        chunks: [{ cid: CID_D, len: 1024 }],
        erasure: { id: 'none', data: 0, parity: 0 },
        stripes: [],
        reach: 'commons',
        author_id: null,
        created_at: new Date().toISOString(),
        verified_at: null,
      };

      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => mockManifest,
      });

      const result = await client.getManifest(CID_D);

      expect(result).toEqual(mockManifest);
    });

    it('should return null when manifest not found', async () => {
      (global.fetch as any).mockResolvedValue({
        ok: false,
        status: 404,
      });

      const result = await client.getManifest(CID_B);

      expect(result).toBeNull();
    });
  });

  describe('pushBlobs()', () => {
    it('should upload multiple blobs', async () => {
      const blobs = [
        { data: Buffer.from('blob1'), mimeType: 'text/plain' },
        { data: Buffer.from('blob2'), mimeType: 'text/plain' },
      ];

      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => putAnswer(CID_D, 5),
      });

      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      const result = await client.pushBlobs(blobs);

      expect(result.success).toBe(2);
      expect(result.failed).toBe(0);
      expect(result.manifests).toHaveLength(2);

      consoleSpy.mockRestore();
    });

    it('should handle partial failures', async () => {
      const blobs = [
        { data: Buffer.from('blob1'), mimeType: 'text/plain' },
        { data: Buffer.from('blob2'), mimeType: 'text/plain' },
      ];

      (global.fetch as any)
        .mockResolvedValueOnce({
          ok: true,
          json: async () => putAnswer(CID_D, 5),
        })
        .mockResolvedValueOnce({
          ok: false,
          status: 500,
          text: async () => 'Error',
        });

      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const result = await client.pushBlobs(blobs);

      expect(result.success).toBe(1);
      expect(result.failed).toBe(1);
      expect(result.errors).toHaveLength(1);

      consoleSpy.mockRestore();
      consoleErrorSpy.mockRestore();
    });
  });

  describe('Retry Logic', () => {
    it('should retry on network failure', async () => {
      const data = Buffer.from('test');
      const retryClient = new StorageClient({ ...config, retries: 2 });

      // Fail first, succeed second
      (global.fetch as any)
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({
            cid: CID_D,
            size_bytes: 4,
            already_existed: false,
          }),
        });

      const result = await retryClient.pushShard(data);

      expect(result.success).toBe(true);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    });

    it('should fail after max retries', async () => {
      const data = Buffer.from('test');
      const retryClient = new StorageClient({ ...config, retries: 2 });

      (global.fetch as any).mockRejectedValue(new Error('Persistent network error'));

      const result = await retryClient.pushShard(data);

      expect(result.success).toBe(false);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    });
  });
});

describe('validateStorageNode()', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should validate healthy storage node', async () => {
    (global.fetch as any).mockResolvedValue({
      ok: true,
      json: async () => ({
        status: 'ok',
        blobs: 50,
        bytes: 102400,
      }),
    });

    const result = await validateStorageNode('http://localhost:8090');

    expect(result.ready).toBe(true);
    expect(result.issues).toHaveLength(0);
    expect(result.stats?.blobs).toBe(50);
  });

  it('should report unhealthy storage node', async () => {
    (global.fetch as any).mockRejectedValue(new Error('Connection refused'));

    const result = await validateStorageNode('http://localhost:8090');

    expect(result.ready).toBe(false);
    expect(result.issues.length).toBeGreaterThan(0);
  });
});
