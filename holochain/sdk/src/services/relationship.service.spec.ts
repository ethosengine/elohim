/**
 * Relationship Service Tests
 *
 * Tests for RelationshipService covering:
 * - Relationship CRUD operations
 * - Type-specific helpers (relatesTo, contains, dependsOn)
 * - Graph traversal
 * - Bulk operations
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { RelationshipService } from './relationship.service.js';
import { ZomeClient } from '../client/zome-client.js';
import { BatchExecutor } from '../client/batch-executor.js';
import { RelationshipTypes, InferenceSources } from '../types.js';
import type { RelationshipOutput, ContentOutput, CreateRelationshipInput } from '../types.js';

// Mock dependencies
vi.mock('../client/zome-client.js');
vi.mock('../client/batch-executor.js');

describe('RelationshipService', () => {
  let service: RelationshipService;
  let mockClient: any;
  let mockBatchExecutor: any;

  const mockRelationship: RelationshipOutput = {
    action_hash: new Uint8Array([1, 2, 3]),
    relationship: {
      source_id: 'concept-1',
      target_id: 'concept-2',
      relationship_type: 'relates_to',
      confidence: 0.9,
      inference_source: 'explicit',
      created_at: '2024-01-01T00:00:00Z',
    },
  };

  const mockContent: ContentOutput = {
    action_hash: new Uint8Array([4, 5, 6]),
    entry_hash: new Uint8Array([7, 8, 9]),
    content: {
      id: 'concept-1',
      content_type: 'concept',
      title: 'Test Concept',
      description: 'Test',
      content: 'Body',
      content_format: 'markdown',
      tags: [],
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

    mockClient = {
      createRelationship: vi.fn(),
      getRelationships: vi.fn(),
      queryRelatedContent: vi.fn(),
      getContentGraph: vi.fn(),
      getContentById: vi.fn(),
    };

    mockBatchExecutor = {
      bulkCreateRelationships: vi.fn(),
    };

    (ZomeClient as any).mockImplementation(() => mockClient);
    (BatchExecutor as any).mockImplementation(() => mockBatchExecutor);

    service = new RelationshipService(mockClient);
  });

  describe('create()', () => {
    it('should create relationship', async () => {
      const input: CreateRelationshipInput = {
        source_id: 'concept-1',
        target_id: 'concept-2',
        relationship_type: 'relates_to',
        confidence: 0.9,
        inference_source: 'explicit',
      };

      mockClient.createRelationship.mockResolvedValue(mockRelationship);

      const result = await service.create(input);

      expect(result).toEqual(mockRelationship);
      expect(mockClient.createRelationship).toHaveBeenCalledWith(input);
    });

    it('should propagate creation errors', async () => {
      const input: CreateRelationshipInput = {
        source_id: 'nonexistent',
        target_id: 'concept-2',
        relationship_type: 'relates_to',
        confidence: 0.9,
        inference_source: 'explicit',
      };

      mockClient.createRelationship.mockRejectedValue(new Error('Source not found'));

      await expect(service.create(input)).rejects.toThrow('Source not found');
    });
  });

  describe('createExplicit()', () => {
    it('should create explicit relationship with defaults', async () => {
      mockClient.createRelationship.mockResolvedValue(mockRelationship);

      await service.createExplicit('concept-1', 'concept-2', 'relates_to');

      expect(mockClient.createRelationship).toHaveBeenCalledWith({
        source_id: 'concept-1',
        target_id: 'concept-2',
        relationship_type: 'relates_to',
        confidence: 1.0,
        inference_source: InferenceSources.EXPLICIT,
      });
    });

    it('should accept custom confidence', async () => {
      mockClient.createRelationship.mockResolvedValue(mockRelationship);

      await service.createExplicit('concept-1', 'concept-2', 'relates_to', 0.7);

      expect(mockClient.createRelationship).toHaveBeenCalledWith(
        expect.objectContaining({ confidence: 0.7 })
      );
    });
  });

  describe('Type-Specific Helpers', () => {
    describe('relatesTo()', () => {
      it('should create RELATES_TO relationship', async () => {
        mockClient.createRelationship.mockResolvedValue(mockRelationship);

        await service.relatesTo('concept-1', 'concept-2');

        expect(mockClient.createRelationship).toHaveBeenCalledWith(
          expect.objectContaining({
            relationship_type: RelationshipTypes.RELATES_TO,
            confidence: 0.8,
          })
        );
      });

      it('should accept custom confidence', async () => {
        mockClient.createRelationship.mockResolvedValue(mockRelationship);

        await service.relatesTo('concept-1', 'concept-2', 0.95);

        expect(mockClient.createRelationship).toHaveBeenCalledWith(
          expect.objectContaining({ confidence: 0.95 })
        );
      });
    });

    describe('contains()', () => {
      it('should create CONTAINS relationship', async () => {
        mockClient.createRelationship.mockResolvedValue(mockRelationship);

        await service.contains('path-1', 'step-1');

        expect(mockClient.createRelationship).toHaveBeenCalledWith(
          expect.objectContaining({
            source_id: 'path-1',
            target_id: 'step-1',
            relationship_type: RelationshipTypes.CONTAINS,
            confidence: 1.0,
          })
        );
      });
    });

    describe('dependsOn()', () => {
      it('should create DEPENDS_ON relationship', async () => {
        mockClient.createRelationship.mockResolvedValue(mockRelationship);

        await service.dependsOn('concept-2', 'concept-1');

        expect(mockClient.createRelationship).toHaveBeenCalledWith(
          expect.objectContaining({
            source_id: 'concept-2',
            target_id: 'concept-1',
            relationship_type: RelationshipTypes.DEPENDS_ON,
            confidence: 1.0,
          })
        );
      });
    });
  });

  describe('bulkCreate()', () => {
    it('should delegate to batch executor', async () => {
      const relationships: CreateRelationshipInput[] = [
        {
          source_id: 'concept-1',
          target_id: 'concept-2',
          relationship_type: 'relates_to',
          confidence: 0.9,
          inference_source: 'explicit',
        },
      ];

      const expectedResult = {
        success: [mockRelationship],
        errors: [],
        totalProcessed: 1,
        totalBatches: 1,
      };

      mockBatchExecutor.bulkCreateRelationships.mockResolvedValue(expectedResult);

      const result = await service.bulkCreate(relationships);

      expect(result).toEqual(expectedResult);
      expect(mockBatchExecutor.bulkCreateRelationships).toHaveBeenCalledWith(relationships);
    });
  });

  describe('getAll()', () => {
    it('should retrieve all relationships for content', async () => {
      mockClient.getRelationships.mockResolvedValue([mockRelationship]);

      const result = await service.getAll('concept-1');

      expect(result).toEqual([mockRelationship]);
      expect(mockClient.getRelationships).toHaveBeenCalledWith({
        content_id: 'concept-1',
        direction: 'both',
      });
    });
  });

  describe('getOutgoing()', () => {
    it('should retrieve outgoing relationships', async () => {
      mockClient.getRelationships.mockResolvedValue([mockRelationship]);

      const result = await service.getOutgoing('concept-1');

      expect(result).toEqual([mockRelationship]);
      expect(mockClient.getRelationships).toHaveBeenCalledWith({
        content_id: 'concept-1',
        direction: 'outgoing',
      });
    });
  });

  describe('getIncoming()', () => {
    it('should retrieve incoming relationships', async () => {
      mockClient.getRelationships.mockResolvedValue([mockRelationship]);

      const result = await service.getIncoming('concept-2');

      expect(result).toEqual([mockRelationship]);
      expect(mockClient.getRelationships).toHaveBeenCalledWith({
        content_id: 'concept-2',
        direction: 'incoming',
      });
    });
  });

  describe('getRelatedContent()', () => {
    it('should retrieve related content', async () => {
      mockClient.queryRelatedContent.mockResolvedValue([mockContent]);

      const result = await service.getRelatedContent('concept-1');

      expect(result).toEqual([mockContent]);
      expect(mockClient.queryRelatedContent).toHaveBeenCalledWith({
        content_id: 'concept-1',
        relationship_types: undefined,
      });
    });

    it('should filter by relationship types', async () => {
      mockClient.queryRelatedContent.mockResolvedValue([mockContent]);

      const types = ['relates_to', 'depends_on'];
      await service.getRelatedContent('concept-1', types);

      expect(mockClient.queryRelatedContent).toHaveBeenCalledWith({
        content_id: 'concept-1',
        relationship_types: types,
      });
    });
  });

  describe('getGraph()', () => {
    it('should retrieve content graph with default depth', async () => {
      const mockGraph = {
        root: mockContent,
        nodes: [mockContent],
        edges: [mockRelationship],
      };

      mockClient.getContentGraph.mockResolvedValue(mockGraph);

      const result = await service.getGraph('concept-1');

      expect(result).toEqual(mockGraph);
      expect(mockClient.getContentGraph).toHaveBeenCalledWith({
        content_id: 'concept-1',
        depth: 1,
      });
    });

    it('should support custom depth', async () => {
      mockClient.getContentGraph.mockResolvedValue({
        root: mockContent,
        nodes: [],
        edges: [],
      });

      await service.getGraph('concept-1', 3);

      expect(mockClient.getContentGraph).toHaveBeenCalledWith({
        content_id: 'concept-1',
        depth: 3,
      });
    });
  });

  describe('exists()', () => {
    it('should return true when relationship exists', async () => {
      mockClient.getRelationships.mockResolvedValue([mockRelationship]);

      const result = await service.exists('concept-1', 'concept-2');

      expect(result).toBe(true);
    });

    it('should return false when relationship does not exist', async () => {
      mockClient.getRelationships.mockResolvedValue([]);

      const result = await service.exists('concept-1', 'concept-3');

      expect(result).toBe(false);
    });

    it('should filter by type when provided', async () => {
      const relationships = [
        mockRelationship,
        {
          ...mockRelationship,
          relationship: {
            ...mockRelationship.relationship,
            relationship_type: 'contains',
          },
        },
      ];

      mockClient.getRelationships.mockResolvedValue(relationships);

      const result = await service.exists('concept-1', 'concept-2', 'contains');

      expect(result).toBe(true);
    });

    it('should return false when type does not match', async () => {
      mockClient.getRelationships.mockResolvedValue([mockRelationship]);

      const result = await service.exists('concept-1', 'concept-2', 'contains');

      expect(result).toBe(false);
    });
  });

  describe('getChildren()', () => {
    it('should retrieve content with CONTAINS relationships', async () => {
      mockClient.queryRelatedContent.mockResolvedValue([mockContent]);

      const result = await service.getChildren('path-1');

      expect(result).toEqual([mockContent]);
      expect(mockClient.queryRelatedContent).toHaveBeenCalledWith({
        content_id: 'path-1',
        relationship_types: [RelationshipTypes.CONTAINS],
      });
    });
  });

  describe('getParents()', () => {
    it('should retrieve parent content via reverse CONTAINS lookup', async () => {
      const incomingRels = [
        {
          ...mockRelationship,
          relationship: {
            ...mockRelationship.relationship,
            relationship_type: RelationshipTypes.CONTAINS,
            source_id: 'parent-1',
            target_id: 'child-1',
          },
        },
      ];

      mockClient.getRelationships.mockResolvedValue(incomingRels);
      mockClient.getContentById.mockResolvedValue(mockContent);

      const result = await service.getParents('child-1');

      expect(result).toHaveLength(1);
      expect(mockClient.getContentById).toHaveBeenCalledWith('parent-1');
    });

    it('should filter out non-CONTAINS relationships', async () => {
      const incomingRels = [
        {
          ...mockRelationship,
          relationship: {
            ...mockRelationship.relationship,
            relationship_type: 'relates_to', // Not CONTAINS
            source_id: 'related-1',
            target_id: 'child-1',
          },
        },
      ];

      mockClient.getRelationships.mockResolvedValue(incomingRels);

      const result = await service.getParents('child-1');

      expect(result).toHaveLength(0);
      expect(mockClient.getContentById).not.toHaveBeenCalled();
    });

    it('should handle missing parent content gracefully', async () => {
      const incomingRels = [
        {
          ...mockRelationship,
          relationship: {
            ...mockRelationship.relationship,
            relationship_type: RelationshipTypes.CONTAINS,
            source_id: 'nonexistent',
            target_id: 'child-1',
          },
        },
      ];

      mockClient.getRelationships.mockResolvedValue(incomingRels);
      mockClient.getContentById.mockResolvedValue(null);

      const result = await service.getParents('child-1');

      expect(result).toHaveLength(0);
    });
  });
});
