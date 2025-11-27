import {
  ContentNode,
  ContentFormat,
  ContentMetadata,
  ContentRelationship,
  ContentRelationshipType,
  ContentGraph,
  ContentGraphMetadata,
  ContentType,
  ContentReach
} from './content-node.model';

describe('ContentNode Model', () => {
  describe('ContentNode interface', () => {
    it('should create valid content node with required fields', () => {
      const node: ContentNode = {
        id: 'node-1',
        contentType: 'epic',
        title: 'Test Epic',
        description: 'A test content node',
        content: '# Test Epic\n\nContent here',
        contentFormat: 'markdown',
        tags: ['test', 'documentation'],
        sourcePath: '/docs/node-1.md',
        relatedNodeIds: ['node-2'],
        metadata: {}
      };

      expect(node.id).toBe('node-1');
      expect(node.contentType).toBe('epic');
      expect(node.contentFormat).toBe('markdown');
      expect(node.tags).toContain('test');
    });

    it('should support all valid content types', () => {
      const contentTypes: ContentType[] = [
        'epic', 'feature', 'scenario', 'concept',
        'simulation', 'video', 'assessment', 'organization',
        'book-chapter', 'tool'
      ];

      contentTypes.forEach(type => {
        const node: ContentNode = {
          id: `node-${type}`,
          contentType: type,
          title: 'Test',
          description: 'Test',
          content: 'Test content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {}
        };

        expect(node.contentType).toBe(type);
      });
    });

    it('should support all content formats', () => {
      const formats: ContentFormat[] = [
        'markdown', 'html5-app', 'video-embed', 'video-file',
        'quiz-json', 'external-link', 'epub', 'gherkin', 'html', 'plaintext'
      ];

      formats.forEach(format => {
        const node: ContentNode = {
          id: `node-${format}`,
          contentType: 'concept',
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

    it('should support optional trust fields', () => {
      const node: ContentNode = {
        id: 'node-1',
        contentType: 'epic',
        title: 'Trusted Content',
        description: 'Content with trust fields',
        content: 'Content',
        contentFormat: 'markdown',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
        authorId: 'author-123',
        reach: 'community',
        trustScore: 0.85,
        activeAttestationIds: ['att-1', 'att-2']
      };

      expect(node.authorId).toBe('author-123');
      expect(node.reach).toBe('community');
      expect(node.trustScore).toBe(0.85);
      expect(node.activeAttestationIds).toContain('att-1');
    });

    it('should support all reach levels', () => {
      const reachLevels: ContentReach[] = [
        'private', 'invited', 'local', 'community', 'federated', 'commons'
      ];

      reachLevels.forEach(reach => {
        const node: ContentNode = {
          id: `node-${reach}`,
          contentType: 'concept',
          title: 'Test',
          description: 'Test',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          reach
        };

        expect(node.reach).toBe(reach);
      });
    });

    it('should support optional timestamps as strings', () => {
      const now = new Date().toISOString();
      const node: ContentNode = {
        id: 'node-1',
        contentType: 'feature',
        title: 'Feature',
        description: 'Test feature',
        content: 'Content',
        contentFormat: 'gherkin',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
        createdAt: now,
        updatedAt: now
      };

      expect(node.createdAt).toBe(now);
      expect(node.updatedAt).toBe(now);
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

      expect(metadata.category).toBe('Architecture');
      expect(metadata.authors).toContain('Author 1');
      expect(metadata.version).toBe('1.0.0');
      expect(metadata.status).toBe('published');
      expect(metadata.priority).toBe(5);
    });

    it('should support custom fields via index signature', () => {
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

    it('should support embedding strategy options', () => {
      const metadata: ContentMetadata = {
        embedStrategy: 'iframe',
        requiredCapabilities: ['webgl', 'audio'],
        securityPolicy: {
          sandbox: ['allow-scripts'],
          csp: "default-src 'self'"
        }
      };

      expect(metadata.embedStrategy).toBe('iframe');
      expect(metadata.requiredCapabilities).toContain('webgl');
      expect(metadata.securityPolicy?.sandbox).toContain('allow-scripts');
    });
  });

  describe('ContentRelationship interface', () => {
    it('should create valid relationship', () => {
      const relationship: ContentRelationship = {
        id: 'rel-1',
        sourceNodeId: 'node-1',
        targetNodeId: 'node-2',
        relationshipType: ContentRelationshipType.DESCRIBES
      };

      expect(relationship.id).toBe('rel-1');
      expect(relationship.sourceNodeId).toBe('node-1');
      expect(relationship.targetNodeId).toBe('node-2');
      expect(relationship.relationshipType).toBe(ContentRelationshipType.DESCRIBES);
    });

    it('should support optional metadata', () => {
      const relationship: ContentRelationship = {
        id: 'rel-1',
        sourceNodeId: 'node-1',
        targetNodeId: 'node-2',
        relationshipType: ContentRelationshipType.VALIDATES,
        metadata: { weight: 0.8, description: 'Strong validation' }
      };

      expect(relationship.metadata?.['weight']).toBe(0.8);
      expect(relationship.metadata?.['description']).toBe('Strong validation');
    });
  });

  describe('ContentRelationshipType enum', () => {
    it('should have all relationship types', () => {
      expect(ContentRelationshipType.CONTAINS).toBe('CONTAINS');
      expect(ContentRelationshipType.BELONGS_TO).toBe('BELONGS_TO');
      expect(ContentRelationshipType.DESCRIBES).toBe('DESCRIBES');
      expect(ContentRelationshipType.IMPLEMENTS).toBe('IMPLEMENTS');
      expect(ContentRelationshipType.VALIDATES).toBe('VALIDATES');
      expect(ContentRelationshipType.RELATES_TO).toBe('RELATES_TO');
      expect(ContentRelationshipType.REFERENCES).toBe('REFERENCES');
      expect(ContentRelationshipType.DEPENDS_ON).toBe('DEPENDS_ON');
      expect(ContentRelationshipType.REQUIRES).toBe('REQUIRES');
      expect(ContentRelationshipType.FOLLOWS).toBe('FOLLOWS');
    });
  });

  describe('ContentGraph interface', () => {
    it('should create valid content graph structure', () => {
      const nodes = new Map<string, ContentNode>();
      const relationships = new Map<string, ContentRelationship>();
      const nodesByType = new Map<string, Set<string>>();
      const nodesByTag = new Map<string, Set<string>>();
      const nodesByCategory = new Map<string, Set<string>>();
      const adjacency = new Map<string, Set<string>>();
      const reverseAdjacency = new Map<string, Set<string>>();

      const metadata: ContentGraphMetadata = {
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
      expect(graph.metadata.version).toBe('1.0');
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

    it('should maintain adjacency lists for traversal', () => {
      const adjacency = new Map<string, Set<string>>();
      adjacency.set('node-1', new Set(['node-2', 'node-3']));

      const reverseAdjacency = new Map<string, Set<string>>();
      reverseAdjacency.set('node-2', new Set(['node-1']));

      expect(adjacency.get('node-1')?.has('node-2')).toBe(true);
      expect(reverseAdjacency.get('node-2')?.has('node-1')).toBe(true);
    });
  });

  describe('ContentGraphMetadata interface', () => {
    it('should track graph statistics', () => {
      const metadata: ContentGraphMetadata = {
        nodeCount: 100,
        relationshipCount: 250,
        lastUpdated: new Date('2025-01-01'),
        version: '2.0'
      };

      expect(metadata.nodeCount).toBe(100);
      expect(metadata.relationshipCount).toBe(250);
      expect(metadata.version).toBe('2.0');
      expect(metadata.lastUpdated instanceof Date).toBe(true);
    });
  });
});
