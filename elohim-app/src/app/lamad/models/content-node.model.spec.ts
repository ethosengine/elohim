import {
  ContentNode,
  ContentFormat,
  ContentMetadata,
  ContentRelationship,
  RelationshipType,
  ContentGraph,
  GraphMetadata
} from './content-node.model';

describe('ContentNode Model', () => {
  describe('ContentNode interface', () => {
    it('should create valid content node', () => {
      const node: ContentNode = {
        id: 'node-1',
        contentType: 'epic',
        title: 'Test Epic',
        description: 'A test content node',
        content: '# Test Epic\n\nContent here',
        contentFormat: 'markdown',
        tags: ['test', 'documentation'],
        relatedNodeIds: ['node-2'],
        metadata: {}
      };

      expect(node.id).toBe('node-1');
      expect(node.contentType).toBe('epic');
      expect(node.contentFormat).toBe('markdown');
      expect(node.tags).toContain('test');
    });

    it('should support all content formats', () => {
      const formats: ContentFormat[] = ['markdown', 'gherkin', 'html', 'plaintext'];

      formats.forEach(format => {
        const node: ContentNode = {
          id: `node-${format}`,
          contentType: 'test',
          title: 'Test',
          description: 'Test',
          content: 'Test content',
          contentFormat: format,
          tags: [],
          relatedNodeIds: [],
          metadata: {}
        };

        expect(node.contentFormat).toBe(format);
      });
    });

    it('should support optional source path', () => {
      const node: ContentNode = {
        id: 'node-1',
        contentType: 'documentation',
        title: 'Doc',
        description: 'Documentation node',
        content: 'Content',
        contentFormat: 'markdown',
        tags: [],
        sourcePath: '/docs/test.md',
        relatedNodeIds: [],
        metadata: {}
      };

      expect(node.sourcePath).toBe('/docs/test.md');
    });

    it('should support optional timestamps', () => {
      const createdAt = new Date('2025-01-01');
      const updatedAt = new Date('2025-01-15');

      const node: ContentNode = {
        id: 'node-1',
        contentType: 'article',
        title: 'Article',
        description: 'Test article',
        content: 'Content',
        contentFormat: 'html',
        tags: [],
        relatedNodeIds: [],
        createdAt,
        updatedAt,
        metadata: {}
      };

      expect(node.createdAt).toEqual(createdAt);
      expect(node.updatedAt).toEqual(updatedAt);
    });
  });

  describe('ContentMetadata interface', () => {
    it('should support standard metadata fields', () => {
      const metadata: ContentMetadata = {
        category: 'Architecture',
        authors: ['Author 1', 'Author 2'],
        version: '1.0.0',
        status: 'published',
        priority: 5
      };

      expect(metadata['category']).toBe('Architecture');
      expect(metadata['authors']).toContain('Author 1');
      expect(metadata['version']).toBe('1.0.0');
      expect(metadata['status']).toBe('published');
      expect(metadata['priority']).toBe(5);
    });

    it('should support custom fields', () => {
      const metadata: ContentMetadata = {
        customField1: 'value1',
        customField2: 123,
        customField3: true,
        nestedObject: { key: 'value' }
      };

      expect(metadata['customField1']).toBe('value1');
      expect(metadata['customField2']).toBe(123);
      expect(metadata['customField3']).toBe(true);
      expect(metadata['nestedObject']).toEqual({ key: 'value' });
    });
  });

  describe('ContentRelationship interface', () => {
    it('should create valid relationship', () => {
      const relationship: ContentRelationship = {
        id: 'rel-1',
        sourceNodeId: 'node-1',
        targetNodeId: 'node-2',
        relationshipType: RelationshipType.DESCRIBES
      };

      expect(relationship.id).toBe('rel-1');
      expect(relationship.sourceNodeId).toBe('node-1');
      expect(relationship.targetNodeId).toBe('node-2');
      expect(relationship.relationshipType).toBe(RelationshipType.DESCRIBES);
    });

    it('should support optional metadata', () => {
      const relationship: ContentRelationship = {
        id: 'rel-1',
        sourceNodeId: 'node-1',
        targetNodeId: 'node-2',
        relationshipType: RelationshipType.VALIDATES,
        metadata: { weight: 0.8, description: 'Strong validation' }
      };

      expect(relationship.metadata?.['weight']).toBe(0.8);
      expect(relationship.metadata?.['description']).toBe('Strong validation');
    });
  });

  describe('RelationshipType enum', () => {
    it('should have all relationship types', () => {
      expect(RelationshipType.CONTAINS).toBe('CONTAINS');
      expect(RelationshipType.BELONGS_TO).toBe('BELONGS_TO');
      expect(RelationshipType.DESCRIBES).toBe('DESCRIBES');
      expect(RelationshipType.IMPLEMENTS).toBe('IMPLEMENTS');
      expect(RelationshipType.VALIDATES).toBe('VALIDATES');
      expect(RelationshipType.RELATES_TO).toBe('RELATES_TO');
      expect(RelationshipType.REFERENCES).toBe('REFERENCES');
      expect(RelationshipType.DEPENDS_ON).toBe('DEPENDS_ON');
      expect(RelationshipType.REQUIRES).toBe('REQUIRES');
      expect(RelationshipType.FOLLOWS).toBe('FOLLOWS');
    });
  });

  describe('ContentGraph interface', () => {
    it('should create valid content graph', () => {
      const nodes = new Map<string, ContentNode>();
      const relationships = new Map<string, ContentRelationship>();
      const nodesByType = new Map<string, Set<string>>();
      const nodesByTag = new Map<string, Set<string>>();
      const nodesByCategory = new Map<string, Set<string>>();
      const adjacency = new Map<string, Set<string>>();
      const reverseAdjacency = new Map<string, Set<string>>();

      const metadata: GraphMetadata = {
        nodeCount: 0,
        relationshipCount: 0,
        lastUpdated: new Date(),
        version: '1.0'
      };

      const graph: ContentGraph = {
        nodes,
        relationships,
        nodesByType,
        nodesByTag,
        nodesByCategory,
        adjacency,
        reverseAdjacency,
        metadata
      };

      expect(graph.nodes.size).toBe(0);
      expect(graph.relationships.size).toBe(0);
      expect(graph.metadata['version']).toBe('1.0');
    });

    it('should organize nodes by type', () => {
      const nodesByType = new Map<string, Set<string>>();
      nodesByType.set('epic', new Set(['epic-1', 'epic-2']));
      nodesByType.set('feature', new Set(['feature-1']));

      expect(nodesByType.get('epic')?.size).toBe(2);
      expect(nodesByType.get('feature')?.size).toBe(1);
    });

    it('should organize nodes by tag', () => {
      const nodesByTag = new Map<string, Set<string>>();
      nodesByTag.set('authentication', new Set(['node-1', 'node-2']));
      nodesByTag.set('security', new Set(['node-1']));

      expect(nodesByTag.get('authentication')?.has('node-1')).toBe(true);
      expect(nodesByTag.get('security')?.has('node-1')).toBe(true);
    });

    it('should maintain adjacency lists', () => {
      const adjacency = new Map<string, Set<string>>();
      adjacency.set('node-1', new Set(['node-2', 'node-3']));

      const reverseAdjacency = new Map<string, Set<string>>();
      reverseAdjacency.set('node-2', new Set(['node-1']));

      expect(adjacency.get('node-1')?.has('node-2')).toBe(true);
      expect(reverseAdjacency.get('node-2')?.has('node-1')).toBe(true);
    });
  });

  describe('GraphMetadata interface', () => {
    it('should track graph statistics', () => {
      const metadata: GraphMetadata = {
        nodeCount: 100,
        relationshipCount: 250,
        lastUpdated: new Date('2025-01-01'),
        version: '2.0'
      };

      expect(metadata['nodeCount']).toBe(100);
      expect(metadata['relationshipCount']).toBe(250);
      expect(metadata['version']).toBe('2.0');
    });
  });
});
