import { buildContentNode } from './content-node-builder';
import { ContentNode } from '../models/content-node.model';

describe('content-node-builder', () => {
  describe('buildContentNode', () => {
    it('should build a complete content node with default values', () => {
      const config = {
        id: 'test-id',
        contentType: 'epic' as const,
        title: 'Test Epic',
        description: 'Test description',
        content: 'Test content',
        contentFormat: 'markdown' as const,
        tags: ['test', 'epic'],
        sourcePath: '/path/to/file.md',
        relatedNodeIds: ['related-1', 'related-2'],
        metadata: { key: 'value' }
      };

      const result = buildContentNode(config);

      expect(result.id).toBe('test-id');
      expect(result.contentType).toBe('epic');
      expect(result.title).toBe('Test Epic');
      expect(result.description).toBe('Test description');
      expect(result.content).toBe('Test content');
      expect(result.contentFormat).toBe('markdown');
      expect(result.tags).toEqual(['test', 'epic']);
      expect(result.sourcePath).toBe('/path/to/file.md');
      expect(result.relatedNodeIds).toEqual(['related-1', 'related-2']);
      expect(result.metadata).toEqual({ key: 'value' });
      expect(result.reach).toBe('commons');
      expect(result.createdAt).toBeDefined();
      expect(result.updatedAt).toBeDefined();
      expect(result.createdAt).toBe(result.updatedAt);
    });

    it('should use provided reach value', () => {
      const config = {
        id: 'test-id',
        contentType: 'reference' as const,
        title: 'Test',
        description: 'Test',
        content: 'Test',
        contentFormat: 'markdown' as const,
        tags: [],
        sourcePath: '/path',
        relatedNodeIds: [],
        metadata: {},
        reach: 'private' as const
      };

      const result = buildContentNode(config);
      expect(result.reach).toBe('private');
    });

    it('should use provided timestamp values', () => {
      const createdAt = '2024-01-01T00:00:00.000Z';
      const updatedAt = '2024-01-02T00:00:00.000Z';

      const config = {
        id: 'test-id',
        contentType: 'scenario' as const,
        title: 'Test',
        description: 'Test',
        content: 'Test',
        contentFormat: 'gherkin' as const,
        tags: [],
        sourcePath: '/path',
        relatedNodeIds: [],
        metadata: {},
        createdAt,
        updatedAt
      };

      const result = buildContentNode(config);
      expect(result.createdAt).toBe(createdAt);
      expect(result.updatedAt).toBe(updatedAt);
    });

    it('should handle gherkin content format', () => {
      const config = {
        id: 'scenario-id',
        contentType: 'scenario' as const,
        title: 'Test Scenario',
        description: 'Scenario desc',
        content: 'Given...When...Then...',
        contentFormat: 'gherkin' as const,
        tags: ['scenario', 'test'],
        sourcePath: '/scenarios/test.feature',
        relatedNodeIds: [],
        metadata: {}
      };

      const result = buildContentNode(config);
      expect(result.contentFormat).toBe('gherkin');
    });

    it('should handle empty arrays and objects', () => {
      const config = {
        id: 'empty-id',
        contentType: 'role' as const,
        title: 'Empty',
        description: 'Empty',
        content: '',
        contentFormat: 'markdown' as const,
        tags: [],
        sourcePath: '/path',
        relatedNodeIds: [],
        metadata: {}
      };

      const result = buildContentNode(config);
      expect(result.tags).toEqual([]);
      expect(result.relatedNodeIds).toEqual([]);
      expect(result.metadata).toEqual({});
    });

    it('should preserve metadata object reference', () => {
      const metadata = { custom: 'field', nested: { value: 123 } };
      const config = {
        id: 'test-id',
        contentType: 'epic' as const,
        title: 'Test',
        description: 'Test',
        content: 'Test',
        contentFormat: 'markdown' as const,
        tags: [],
        sourcePath: '/path',
        relatedNodeIds: [],
        metadata
      };

      const result = buildContentNode(config);
      expect(result.metadata).toBe(metadata);
      expect(result.metadata).toEqual({ custom: 'field', nested: { value: 123 } });
    });
  });
});
