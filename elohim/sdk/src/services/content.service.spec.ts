/**
 * Content Service Tests
 *
 * Tests for ContentService covering:
 * - CRUD operations
 * - Query methods
 * - Bulk operations with batching
 * - Statistics and aggregations
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ContentService } from './content.service.js';
import { ZomeClient } from '../client/zome-client.js';
import { BatchExecutor } from '../client/batch-executor.js';
import type { ContentOutput, CreateContentInput } from '../types.js';

// Mock dependencies
vi.mock('../client/zome-client.js');
vi.mock('../client/batch-executor.js');

describe('ContentService', () => {
  let service: ContentService;
  let mockClient: any;
  let mockBatchExecutor: any;

  const mockContentOutput: ContentOutput = {
    action_hash: new Uint8Array([1, 2, 3]),
    entry_hash: new Uint8Array([4, 5, 6]),
    content: {
      id: 'test-content',
      content_type: 'concept',
      title: 'Test Content',
      description: 'Test description',
      content: 'Test content body',
      content_format: 'markdown',
      tags: ['test'],
      source_path: null,
      related_node_ids: [],
      author_id: null,
      reach: 'commons',
      trust_score: 1.0,
      metadata_json: '{}',
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();

    // Mock ZomeClient
    mockClient = {
      createContent: vi.fn(),
      getContentById: vi.fn(),
      getContentByType: vi.fn(),
      getContentByTag: vi.fn(),
      getMyContent: vi.fn(),
      getContentStats: vi.fn(),
      bulkCreateContent: vi.fn(),
    };

    // Mock BatchExecutor
    mockBatchExecutor = {
      bulkCreateContent: vi.fn(),
    };

    // Setup mocks
    (ZomeClient as any).mockImplementation(() => mockClient);
    (BatchExecutor as any).mockImplementation(() => mockBatchExecutor);

    service = new ContentService(mockClient);
  });

  describe('create()', () => {
    it('should create single content entry', async () => {
      const input: CreateContentInput = {
        id: 'test-1',
        content_type: 'concept',
        title: 'Test',
        description: 'Desc',
        content: 'Body',
        content_format: 'markdown',
        tags: [],
        related_node_ids: [],
        reach: 'commons',
        metadata_json: '{}',
      };

      mockClient.createContent.mockResolvedValue(mockContentOutput);

      const result = await service.create(input);

      expect(result).toEqual(mockContentOutput);
      expect(mockClient.createContent).toHaveBeenCalledWith(input);
    });

    it('should propagate creation errors', async () => {
      const input: CreateContentInput = {
        id: 'test-1',
        content_type: 'concept',
        title: 'Test',
        description: 'Desc',
        content: 'Body',
        content_format: 'markdown',
        tags: [],
        related_node_ids: [],
        reach: 'commons',
        metadata_json: '{}',
      };

      mockClient.createContent.mockRejectedValue(new Error('Validation failed'));

      await expect(service.create(input)).rejects.toThrow('Validation failed');
    });
  });

  describe('bulkCreate()', () => {
    it('should delegate to batch executor', async () => {
      const contents: CreateContentInput[] = [
        {
          id: 'test-1',
          content_type: 'concept',
          title: 'Test 1',
          description: 'Desc',
          content: 'Body',
          content_format: 'markdown',
          tags: [],
          related_node_ids: [],
          reach: 'commons',
          metadata_json: '{}',
        },
      ];

      const expectedResult = {
        success: ['hash-1'],
        errors: [],
        totalProcessed: 1,
        totalBatches: 1,
      };

      mockBatchExecutor.bulkCreateContent.mockResolvedValue(expectedResult);

      const result = await service.bulkCreate(contents);

      expect(result).toEqual(expectedResult);
      expect(mockBatchExecutor.bulkCreateContent).toHaveBeenCalledWith(contents, undefined);
    });

    it('should pass import ID to batch executor', async () => {
      const contents: CreateContentInput[] = [];
      mockBatchExecutor.bulkCreateContent.mockResolvedValue({
        success: [],
        errors: [],
        totalProcessed: 0,
        totalBatches: 0,
      });

      await service.bulkCreate(contents, 'test-import');

      expect(mockBatchExecutor.bulkCreateContent).toHaveBeenCalledWith(contents, 'test-import');
    });
  });

  describe('getById()', () => {
    it('should retrieve content by ID', async () => {
      mockClient.getContentById.mockResolvedValue(mockContentOutput);

      const result = await service.getById('test-content');

      expect(result).toEqual(mockContentOutput);
      expect(mockClient.getContentById).toHaveBeenCalledWith('test-content');
    });

    it('should return null for non-existent content', async () => {
      mockClient.getContentById.mockResolvedValue(null);

      const result = await service.getById('non-existent');

      expect(result).toBeNull();
    });
  });

  describe('exists()', () => {
    it('should return true when content exists', async () => {
      mockClient.getContentById.mockResolvedValue(mockContentOutput);

      const result = await service.exists('test-content');

      expect(result).toBe(true);
    });

    it('should return false when content does not exist', async () => {
      mockClient.getContentById.mockResolvedValue(null);

      const result = await service.exists('non-existent');

      expect(result).toBe(false);
    });
  });

  describe('getByType()', () => {
    it('should retrieve content by type', async () => {
      const contents = [mockContentOutput];
      mockClient.getContentByType.mockResolvedValue(contents);

      const result = await service.getByType('concept');

      expect(result).toEqual(contents);
      expect(mockClient.getContentByType).toHaveBeenCalledWith('concept', undefined);
    });

    it('should pass limit parameter', async () => {
      mockClient.getContentByType.mockResolvedValue([]);

      await service.getByType('concept', 10);

      expect(mockClient.getContentByType).toHaveBeenCalledWith('concept', 10);
    });

    it('should return empty array when no content found', async () => {
      mockClient.getContentByType.mockResolvedValue([]);

      const result = await service.getByType('non-existent-type');

      expect(result).toEqual([]);
    });
  });

  describe('getByTag()', () => {
    it('should retrieve content by tag', async () => {
      const contents = [mockContentOutput];
      mockClient.getContentByTag.mockResolvedValue(contents);

      const result = await service.getByTag('test-tag');

      expect(result).toEqual(contents);
      expect(mockClient.getContentByTag).toHaveBeenCalledWith('test-tag');
    });
  });

  describe('getMine()', () => {
    it('should retrieve content created by current agent', async () => {
      const contents = [mockContentOutput];
      mockClient.getMyContent.mockResolvedValue(contents);

      const result = await service.getMine();

      expect(result).toEqual(contents);
      expect(mockClient.getMyContent).toHaveBeenCalled();
    });
  });

  describe('query()', () => {
    it('should query by type only', async () => {
      const contents = [mockContentOutput];
      mockClient.getContentByType.mockResolvedValue(contents);

      const result = await service.query({ content_type: 'concept' });

      expect(result).toEqual(contents);
      expect(mockClient.getContentByType).toHaveBeenCalledWith('concept', undefined);
    });

    it('should query by tags only', async () => {
      const contents = [mockContentOutput];
      mockClient.getContentByTag.mockResolvedValue(contents);

      const result = await service.query({ tags: ['test'] });

      expect(result).toEqual(contents);
      expect(mockClient.getContentByTag).toHaveBeenCalledWith('test');
    });

    it('should filter by multiple tags', async () => {
      const content1 = {
        ...mockContentOutput,
        content: { ...mockContentOutput.content, tags: ['tag1', 'tag2'] },
      };
      const content2 = {
        ...mockContentOutput,
        content: { ...mockContentOutput.content, tags: ['tag1'] },
      };

      mockClient.getContentByTag.mockResolvedValue([content1, content2]);

      const result = await service.query({ tags: ['tag1', 'tag2'] });

      // Should only return content1 which has both tags
      expect(result).toHaveLength(1);
      expect(result[0].content.tags).toContain('tag1');
      expect(result[0].content.tags).toContain('tag2');
    });

    it('should combine type and tag queries', async () => {
      const contents = [mockContentOutput];
      mockClient.getContentByType.mockResolvedValue(contents);

      const result = await service.query({
        content_type: 'concept',
        tags: ['test'],
      });

      expect(result).toHaveLength(1);
    });

    it('should apply limit to results', async () => {
      const contents = Array(10).fill(mockContentOutput);
      mockClient.getContentByType.mockResolvedValue(contents);

      const result = await service.query({ content_type: 'concept', limit: 5 });

      expect(result).toHaveLength(5);
    });

    it('should return empty array when no matches', async () => {
      mockClient.getContentByType.mockResolvedValue([]);

      const result = await service.query({ content_type: 'non-existent' });

      expect(result).toEqual([]);
    });
  });

  describe('getStats()', () => {
    it('should retrieve content statistics', async () => {
      const stats = {
        total_count: 10,
        by_type: {
          concept: 5,
          lesson: 3,
          practice: 2,
        },
      };

      mockClient.getContentStats.mockResolvedValue(stats);

      const result = await service.getStats();

      expect(result).toEqual(stats);
      expect(mockClient.getContentStats).toHaveBeenCalled();
    });
  });

  describe('getContentTypes()', () => {
    it('should return list of content types', async () => {
      const stats = {
        total_count: 10,
        by_type: {
          concept: 5,
          lesson: 3,
          practice: 2,
        },
      };

      mockClient.getContentStats.mockResolvedValue(stats);

      const result = await service.getContentTypes();

      expect(result).toEqual(['concept', 'lesson', 'practice']);
    });

    it('should return empty array when no content types', async () => {
      mockClient.getContentStats.mockResolvedValue({
        total_count: 0,
        by_type: {},
      });

      const result = await service.getContentTypes();

      expect(result).toEqual([]);
    });
  });

  describe('countByType()', () => {
    it('should return count for existing type', async () => {
      const stats = {
        total_count: 10,
        by_type: {
          concept: 5,
        },
      };

      mockClient.getContentStats.mockResolvedValue(stats);

      const result = await service.countByType('concept');

      expect(result).toBe(5);
    });

    it('should return 0 for non-existent type', async () => {
      const stats = {
        total_count: 10,
        by_type: {
          concept: 5,
        },
      };

      mockClient.getContentStats.mockResolvedValue(stats);

      const result = await service.countByType('non-existent');

      expect(result).toBe(0);
    });
  });
});
