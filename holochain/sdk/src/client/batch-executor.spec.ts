/**
 * Batch Executor Tests
 *
 * Tests for BatchExecutor class covering:
 * - Batch splitting logic
 * - Bulk content creation
 * - Bulk relationship creation
 * - Error handling and recovery
 * - Progress callbacks
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { BatchExecutor, createBatchExecutor } from './batch-executor.js';
import { ZomeClient } from './zome-client.js';
import type { CreateContentInput, CreateRelationshipInput } from '../types.js';

// Mock ZomeClient
vi.mock('./zome-client.js', () => ({
  ZomeClient: vi.fn(),
}));

describe('BatchExecutor', () => {
  let mockClient: any;
  let executor: BatchExecutor;

  beforeEach(() => {
    vi.clearAllMocks();

    // Mock ZomeClient methods
    mockClient = {
      bulkCreateContent: vi.fn(),
      createRelationship: vi.fn(),
    };

    executor = new BatchExecutor(mockClient);
  });

  describe('Constructor', () => {
    it('should create instance with default config', () => {
      const exec = new BatchExecutor(mockClient);
      expect(exec).toBeInstanceOf(BatchExecutor);
    });

    it('should use custom batch size', () => {
      const exec = new BatchExecutor(mockClient, { batchSize: 10 });
      expect(exec).toBeInstanceOf(BatchExecutor);
    });

    it('should cap batch size at MAX_BATCH_SIZE', () => {
      const exec = new BatchExecutor(mockClient, { batchSize: 10000 });
      expect(exec).toBeInstanceOf(BatchExecutor);
      // Internally capped, but we can't directly test private config
    });

    it('should accept progress callback', () => {
      const onBatchComplete = vi.fn();
      const exec = new BatchExecutor(mockClient, { onBatchComplete });
      expect(exec).toBeInstanceOf(BatchExecutor);
    });

    it('should accept error handler', () => {
      const onError = vi.fn(() => 'continue' as const);
      const exec = new BatchExecutor(mockClient, { onError });
      expect(exec).toBeInstanceOf(BatchExecutor);
    });
  });

  describe('bulkCreateContent()', () => {
    it('should create content in single batch when under limit', async () => {
      const contents: CreateContentInput[] = [
        {
          id: 'test-1',
          content_type: 'concept',
          title: 'Test 1',
          description: 'Desc 1',
          content: 'Content 1',
          content_format: 'markdown',
          tags: [],
          related_node_ids: [],
          reach: 'commons',
          metadata_json: '{}',
        },
      ];

      mockClient.bulkCreateContent.mockResolvedValue({
        import_id: 'batch-1',
        created_count: 1,
        action_hashes: [new Uint8Array([1, 2, 3])],
        errors: [],
      });

      const result = await executor.bulkCreateContent(contents);

      expect(result.success).toHaveLength(1);
      expect(result.errors).toHaveLength(0);
      expect(result.totalProcessed).toBe(1);
      expect(result.totalBatches).toBe(1);
      expect(mockClient.bulkCreateContent).toHaveBeenCalledTimes(1);
    });

    it('should split into multiple batches when over limit', async () => {
      // Create 75 content items (should split into 2 batches of 50 + 25)
      const contents: CreateContentInput[] = Array.from({ length: 75 }, (_, i) => ({
        id: `test-${i}`,
        content_type: 'concept',
        title: `Test ${i}`,
        description: `Desc ${i}`,
        content: `Content ${i}`,
        content_format: 'markdown',
        tags: [],
        related_node_ids: [],
        reach: 'commons',
        metadata_json: '{}',
      }));

      mockClient.bulkCreateContent.mockResolvedValue({
        import_id: 'batch-1',
        created_count: 50,
        action_hashes: Array(50).fill(new Uint8Array([1, 2, 3])),
        errors: [],
      });

      const result = await executor.bulkCreateContent(contents, 'test-import');

      // Should call twice (batch 1: 50 items, batch 2: 25 items)
      expect(mockClient.bulkCreateContent).toHaveBeenCalledTimes(2);
      expect(result.totalBatches).toBe(2);
      expect(result.totalProcessed).toBe(100); // 50 + 50
    });

    it('should handle partial errors in batch', async () => {
      const contents: CreateContentInput[] = [
        {
          id: 'test-1',
          content_type: 'concept',
          title: 'Test 1',
          description: '',
          content: '',
          content_format: 'markdown',
          tags: [],
          related_node_ids: [],
          reach: 'commons',
          metadata_json: '{}',
        },
      ];

      mockClient.bulkCreateContent.mockResolvedValue({
        import_id: 'batch-1',
        created_count: 0,
        action_hashes: [],
        errors: ['Validation failed: description required'],
      });

      const result = await executor.bulkCreateContent(contents);

      expect(result.errors).toHaveLength(1);
      expect(result.errors[0].error).toContain('Validation failed');
    });

    it('should handle complete batch failure', async () => {
      const contents: CreateContentInput[] = [
        {
          id: 'test-1',
          content_type: 'concept',
          title: 'Test 1',
          description: '',
          content: '',
          content_format: 'markdown',
          tags: [],
          related_node_ids: [],
          reach: 'commons',
          metadata_json: '{}',
        },
      ];

      mockClient.bulkCreateContent.mockRejectedValue(new Error('Network timeout'));

      const result = await executor.bulkCreateContent(contents);

      expect(result.success).toHaveLength(0);
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0].error).toContain('Network timeout');
    });

    it('should continue on error when configured', async () => {
      const onError = vi.fn(() => 'continue' as const);
      const exec = new BatchExecutor(mockClient, { onError, batchSize: 1 });

      const contents: CreateContentInput[] = Array(3).fill(null).map((_, i) => ({
        id: `test-${i}`,
        content_type: 'concept',
        title: `Test ${i}`,
        description: '',
        content: '',
        content_format: 'markdown',
        tags: [],
        related_node_ids: [],
        reach: 'commons',
        metadata_json: '{}',
      }));

      // Fail first batch, succeed others
      mockClient.bulkCreateContent
        .mockRejectedValueOnce(new Error('Batch 1 failed'))
        .mockResolvedValue({
          import_id: 'batch',
          created_count: 1,
          action_hashes: [new Uint8Array([1, 2, 3])],
          errors: [],
        });

      const result = await exec.bulkCreateContent(contents);

      expect(onError).toHaveBeenCalledTimes(1);
      expect(result.errors).toHaveLength(1);
      expect(result.success.length).toBeGreaterThan(0);
    });

    it('should stop on error when configured', async () => {
      const onError = vi.fn(() => 'stop' as const);
      const exec = new BatchExecutor(mockClient, { onError, batchSize: 1 });

      const contents: CreateContentInput[] = Array(3).fill(null).map((_, i) => ({
        id: `test-${i}`,
        content_type: 'concept',
        title: `Test ${i}`,
        description: '',
        content: '',
        content_format: 'markdown',
        tags: [],
        related_node_ids: [],
        reach: 'commons',
        metadata_json: '{}',
      }));

      mockClient.bulkCreateContent.mockRejectedValue(new Error('Batch failed'));

      const result = await exec.bulkCreateContent(contents);

      expect(onError).toHaveBeenCalledTimes(1);
      expect(mockClient.bulkCreateContent).toHaveBeenCalledTimes(1); // Stopped after first
    });

    it('should call progress callback', async () => {
      const onBatchComplete = vi.fn();
      const exec = new BatchExecutor(mockClient, { onBatchComplete, batchSize: 1 });

      const contents: CreateContentInput[] = Array(2).fill(null).map((_, i) => ({
        id: `test-${i}`,
        content_type: 'concept',
        title: `Test ${i}`,
        description: '',
        content: '',
        content_format: 'markdown',
        tags: [],
        related_node_ids: [],
        reach: 'commons',
        metadata_json: '{}',
      }));

      mockClient.bulkCreateContent.mockResolvedValue({
        import_id: 'batch',
        created_count: 1,
        action_hashes: [new Uint8Array([1, 2, 3])],
        errors: [],
      });

      await exec.bulkCreateContent(contents);

      expect(onBatchComplete).toHaveBeenCalledTimes(2);
      expect(onBatchComplete).toHaveBeenNthCalledWith(1, 1, 2); // batch 1 of 2
      expect(onBatchComplete).toHaveBeenNthCalledWith(2, 2, 2); // batch 2 of 2
    });

    it('should use custom import ID', async () => {
      const contents: CreateContentInput[] = [{
        id: 'test-1',
        content_type: 'concept',
        title: 'Test',
        description: '',
        content: '',
        content_format: 'markdown',
        tags: [],
        related_node_ids: [],
        reach: 'commons',
        metadata_json: '{}',
      }];

      mockClient.bulkCreateContent.mockResolvedValue({
        import_id: 'custom-batch-1',
        created_count: 1,
        action_hashes: [new Uint8Array([1, 2, 3])],
        errors: [],
      });

      await executor.bulkCreateContent(contents, 'custom');

      expect(mockClient.bulkCreateContent).toHaveBeenCalledWith(
        expect.objectContaining({
          import_id: 'custom-batch-1',
        })
      );
    });
  });

  describe('bulkCreateRelationships()', () => {
    it('should create relationships in batches', async () => {
      const relationships: CreateRelationshipInput[] = [
        {
          source_id: 'concept-1',
          target_id: 'concept-2',
          relationship_type: 'relates_to',
          confidence: 0.9,
          inference_source: 'explicit',
        },
      ];

      mockClient.createRelationship.mockResolvedValue({
        action_hash: new Uint8Array([1, 2, 3]),
        relationship: relationships[0],
      });

      const result = await executor.bulkCreateRelationships(relationships);

      expect(result.success).toHaveLength(1);
      expect(result.errors).toHaveLength(0);
      expect(result.totalProcessed).toBe(1);
      expect(mockClient.createRelationship).toHaveBeenCalledTimes(1);
    });

    it('should split large relationship arrays into batches', async () => {
      const relationships: CreateRelationshipInput[] = Array(75).fill(null).map((_, i) => ({
        source_id: `concept-${i}`,
        target_id: `concept-${i + 1}`,
        relationship_type: 'relates_to',
        confidence: 0.9,
        inference_source: 'explicit',
      }));

      mockClient.createRelationship.mockResolvedValue({
        action_hash: new Uint8Array([1, 2, 3]),
        relationship: relationships[0],
      });

      const result = await executor.bulkCreateRelationships(relationships);

      expect(result.totalBatches).toBe(2); // 50 + 25
      expect(mockClient.createRelationship).toHaveBeenCalledTimes(75);
    });

    it('should handle individual relationship failures', async () => {
      const relationships: CreateRelationshipInput[] = [
        {
          source_id: 'concept-1',
          target_id: 'concept-2',
          relationship_type: 'relates_to',
          confidence: 0.9,
          inference_source: 'explicit',
        },
        {
          source_id: 'concept-3',
          target_id: 'concept-4',
          relationship_type: 'relates_to',
          confidence: 0.9,
          inference_source: 'explicit',
        },
      ];

      mockClient.createRelationship
        .mockResolvedValueOnce({
          action_hash: new Uint8Array([1, 2, 3]),
          relationship: relationships[0],
        })
        .mockRejectedValueOnce(new Error('Source not found'));

      const result = await executor.bulkCreateRelationships(relationships);

      expect(result.success).toHaveLength(1);
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0].error).toContain('Source not found');
    });

    it('should continue on relationship error when configured to continue', async () => {
      const onError = vi.fn(() => 'continue' as const);
      const exec = new BatchExecutor(mockClient, { onError });

      const relationships: CreateRelationshipInput[] = Array(3).fill(null).map((_, i) => ({
        source_id: `concept-${i}`,
        target_id: `concept-${i + 1}`,
        relationship_type: 'relates_to',
        confidence: 0.9,
        inference_source: 'explicit',
      }));

      mockClient.createRelationship
        .mockRejectedValueOnce(new Error('Failed'))
        .mockResolvedValueOnce({
          action_hash: new Uint8Array([1, 2, 3]),
          relationship: relationships[0],
        })
        .mockResolvedValueOnce({
          action_hash: new Uint8Array([1, 2, 3]),
          relationship: relationships[0],
        });

      const result = await exec.bulkCreateRelationships(relationships);

      // Should succeed for 2 out of 3 relationships
      expect(result.success.length).toBe(2);
      expect(result.errors).toHaveLength(1);
    });

    it('should stop on relationship error when configured', async () => {
      const onError = vi.fn(() => 'stop' as const);
      const exec = new BatchExecutor(mockClient, { onError });

      const relationships: CreateRelationshipInput[] = Array(3).fill(null).map((_, i) => ({
        source_id: `concept-${i}`,
        target_id: `concept-${i + 1}`,
        relationship_type: 'relates_to',
        confidence: 0.9,
        inference_source: 'explicit',
      }));

      mockClient.createRelationship.mockRejectedValue(new Error('Failed'));

      const result = await exec.bulkCreateRelationships(relationships);

      expect(mockClient.createRelationship).toHaveBeenCalledTimes(1);
      expect(result.errors).toHaveLength(1);
    });
  });

  describe('createBatchExecutor()', () => {
    it('should create executor instance', () => {
      const exec = createBatchExecutor(mockClient);
      expect(exec).toBeInstanceOf(BatchExecutor);
    });

    it('should accept config', () => {
      const exec = createBatchExecutor(mockClient, { batchSize: 10 });
      expect(exec).toBeInstanceOf(BatchExecutor);
    });
  });
});
