import {
  DocumentGraph,
  GraphMetadata,
  GraphQuery,
  GraphTraversalResult,
  SerializableGraph,
  GraphJSONLD
} from './document-graph.model';
import { DocumentNode, NodeType } from './document-node.model';
import { EpicNode } from './epic-node.model';
import { FeatureNode } from './feature-node.model';
import { ScenarioNode } from './scenario-node.model';
import { NodeRelationship, RelationshipType } from './node-relationship.model';

describe('DocumentGraph Model', () => {
  describe('DocumentGraph interface', () => {
    it('should create valid document graph', () => {
      const nodes = new Map<string, DocumentNode>();
      const relationships = new Map<string, NodeRelationship>();
      const epics = new Map<string, EpicNode>();
      const features = new Map<string, FeatureNode>();
      const scenarios = new Map<string, ScenarioNode>();
      const nodesByTag = new Map<string, Set<string>>();
      const nodesByCategory = new Map<string, Set<string>>();
      const adjacency = new Map<string, Set<string>>();
      const reverseAdjacency = new Map<string, Set<string>>();

      const metadata: GraphMetadata = {
        nodeCount: 0,
        relationshipCount: 0,
        lastBuilt: new Date(),
        sources: {
          epicPath: '/docs',
          featurePath: '/features'
        },
        stats: {
          epicCount: 0,
          featureCount: 0,
          scenarioCount: 0,
          averageConnectionsPerNode: 0
        }
      };

      const graph: DocumentGraph = {
        nodes,
        relationships,
        nodesByType: { epics, features, scenarios },
        nodesByTag,
        nodesByCategory,
        adjacency,
        reverseAdjacency,
        metadata
      };

      expect(graph.nodes.size).toBe(0);
      expect(graph.metadata.nodeCount).toBe(0);
      expect(graph.metadata.sources.epicPath).toBe('/docs');
    });

    it('should organize nodes by type', () => {
      const epics = new Map<string, EpicNode>();
      const features = new Map<string, FeatureNode>();
      const scenarios = new Map<string, ScenarioNode>();

      const epic: EpicNode = {
        id: 'epic-1',
        type: NodeType.EPIC,
        title: 'Epic',
        description: 'Desc',
        tags: [],
        sourcePath: '/docs/epic.md',
        content: 'Content',
        relatedNodeIds: [],
        featureIds: [],
        relatedEpicIds: [],
        markdownContent: 'Markdown',
        sections: [],
        metadata: {}
      };

      epics.set('epic-1', epic);

      expect(epics.size).toBe(1);
      expect(epics.get('epic-1')?.type).toBe(NodeType.EPIC);
    });

    it('should organize nodes by tag', () => {
      const nodesByTag = new Map<string, Set<string>>();
      nodesByTag.set('authentication', new Set(['feature-1', 'feature-2']));
      nodesByTag.set('security', new Set(['feature-1']));

      expect(nodesByTag.get('authentication')?.size).toBe(2);
      expect(nodesByTag.get('security')?.has('feature-1')).toBe(true);
    });

    it('should maintain bidirectional adjacency', () => {
      const adjacency = new Map<string, Set<string>>();
      const reverseAdjacency = new Map<string, Set<string>>();

      adjacency.set('epic-1', new Set(['feature-1', 'feature-2']));
      reverseAdjacency.set('feature-1', new Set(['epic-1']));
      reverseAdjacency.set('feature-2', new Set(['epic-1']));

      expect(adjacency.get('epic-1')?.has('feature-1')).toBe(true);
      expect(reverseAdjacency.get('feature-1')?.has('epic-1')).toBe(true);
    });
  });

  describe('GraphMetadata interface', () => {
    it('should track comprehensive statistics', () => {
      const metadata: GraphMetadata = {
        nodeCount: 150,
        relationshipCount: 300,
        lastBuilt: new Date('2025-01-01'),
        sources: {
          epicPath: '/docs',
          featurePath: '/cypress/e2e/features'
        },
        stats: {
          epicCount: 20,
          featureCount: 50,
          scenarioCount: 80,
          averageConnectionsPerNode: 2.0
        }
      };

      expect(metadata.nodeCount).toBe(150);
      expect(metadata.stats.epicCount).toBe(20);
      expect(metadata.stats.averageConnectionsPerNode).toBe(2.0);
    });

    it('should track source paths', () => {
      const metadata: GraphMetadata = {
        nodeCount: 10,
        relationshipCount: 15,
        lastBuilt: new Date(),
        sources: {
          epicPath: '/custom/docs',
          featurePath: '/custom/features'
        },
        stats: {
          epicCount: 5,
          featureCount: 5,
          scenarioCount: 0,
          averageConnectionsPerNode: 1.5
        }
      };

      expect(metadata.sources.epicPath).toBe('/custom/docs');
      expect(metadata.sources.featurePath).toBe('/custom/features');
    });
  });

  describe('GraphQuery interface', () => {
    it('should create basic query', () => {
      const query: GraphQuery = {
        startNodeId: 'epic-1'
      };

      expect(query.startNodeId).toBe('epic-1');
    });

    it('should filter by node types', () => {
      const query: GraphQuery = {
        nodeTypes: ['epic', 'feature']
      };

      expect(query.nodeTypes).toContain('epic');
      expect(query.nodeTypes).toContain('feature');
    });

    it('should filter by tags', () => {
      const query: GraphQuery = {
        tags: ['authentication', 'security']
      };

      expect(query.tags?.length).toBe(2);
    });

    it('should filter by categories', () => {
      const query: GraphQuery = {
        categories: ['core', 'infrastructure']
      };

      expect(query.categories).toContain('core');
    });

    it('should limit traversal depth', () => {
      const query: GraphQuery = {
        startNodeId: 'epic-1',
        maxDepth: 3
      };

      expect(query.maxDepth).toBe(3);
    });

    it('should filter by relationship types', () => {
      const query: GraphQuery = {
        relationshipTypes: ['describes', 'implements']
      };

      expect(query.relationshipTypes).toContain('describes');
    });

    it('should support search text', () => {
      const query: GraphQuery = {
        searchText: 'authentication'
      };

      expect(query.searchText).toBe('authentication');
    });
  });

  describe('GraphTraversalResult interface', () => {
    it('should contain traversal results', () => {
      const nodes: DocumentNode[] = [];
      const relationships: NodeRelationship[] = [];
      const paths = new Map<string, string[]>();
      const depths = new Map<string, number>();

      paths.set('node-1', ['root', 'node-1']);
      depths.set('node-1', 1);

      const result: GraphTraversalResult = {
        nodes,
        relationships,
        paths,
        depths
      };

      expect(result.paths.get('node-1')).toEqual(['root', 'node-1']);
      expect(result.depths.get('node-1')).toBe(1);
    });

    it('should track multiple paths', () => {
      const paths = new Map<string, string[]>();
      paths.set('node-1', ['root', 'node-1']);
      paths.set('node-2', ['root', 'node-1', 'node-2']);
      paths.set('node-3', ['root', 'node-3']);

      expect(paths.get('node-2')?.length).toBe(3);
      expect(paths.get('node-3')?.length).toBe(2);
    });

    it('should track depths correctly', () => {
      const depths = new Map<string, number>();
      depths.set('root', 0);
      depths.set('level-1-node', 1);
      depths.set('level-2-node', 2);

      expect(depths.get('root')).toBe(0);
      expect(depths.get('level-2-node')).toBe(2);
    });
  });

  describe('SerializableGraph interface', () => {
    it('should create serializable graph', () => {
      const nodes: DocumentNode[] = [];
      const relationships: NodeRelationship[] = [];
      const metadata: GraphMetadata = {
        nodeCount: 0,
        relationshipCount: 0,
        lastBuilt: new Date(),
        sources: { epicPath: '', featurePath: '' },
        stats: {
          epicCount: 0,
          featureCount: 0,
          scenarioCount: 0,
          averageConnectionsPerNode: 0
        }
      };

      const serializable: SerializableGraph = {
        nodes,
        relationships,
        metadata,
        version: '1.0.0'
      };

      expect(serializable.version).toBe('1.0.0');
      expect(serializable.nodes.length).toBe(0);
    });
  });

  describe('GraphJSONLD interface', () => {
    it('should create JSON-LD representation', () => {
      const jsonLD: GraphJSONLD = {
        '@context': {
          '@vocab': 'http://schema.org/',
          epic: 'http://elohim.host/schema/epic',
          feature: 'http://elohim.host/schema/feature',
          scenario: 'http://elohim.host/schema/scenario',
          describes: 'http://elohim.host/schema/describes',
          implements: 'http://elohim.host/schema/implements'
        },
        '@graph': [
          {
            '@id': 'epic-1',
            '@type': 'epic',
            title: 'Test Epic'
          }
        ]
      };

      expect(jsonLD['@context']['@vocab']).toBe('http://schema.org/');
      expect(jsonLD['@graph'].length).toBe(1);
      expect(jsonLD['@graph'][0]['@id']).toBe('epic-1');
    });

    it('should include custom vocabulary', () => {
      const jsonLD: GraphJSONLD = {
        '@context': {
          '@vocab': 'http://custom.org/',
          epic: 'http://custom.org/epic',
          feature: 'http://custom.org/feature',
          scenario: 'http://custom.org/scenario',
          describes: 'http://custom.org/describes',
          implements: 'http://custom.org/implements'
        },
        '@graph': []
      };

      expect(jsonLD['@context'].epic).toBe('http://custom.org/epic');
      expect(jsonLD['@context'].feature).toBe('http://custom.org/feature');
    });
  });
});
