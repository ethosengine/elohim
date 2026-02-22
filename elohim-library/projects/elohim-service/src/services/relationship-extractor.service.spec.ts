/**
 * Relationship Extractor Service Tests
 *
 * Tests for relationship extraction between content nodes.
 */

import {
  extractRelationships,
  buildRelationshipGraph,
  findConnectedComponents
} from './relationship-extractor.service';
import { ContentNode, ContentRelationshipType } from '../models/content-node.model';

describe('Relationship Extractor Service', () => {
  describe('extractRelationships', () => {
    it('should extract explicit relationships from relatedNodeIds', () => {
      const nodes: ContentNode[] = [
        {
          id: 'node-1',
          contentType: 'epic',
          title: 'Epic 1',
          description: 'Epic',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: ['node-2'],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'node-2',
          contentType: 'scenario',
          title: 'Scenario 1',
          description: 'Scenario',
          content: 'Content',
          contentFormat: 'gherkin',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes);

      expect(relationships.length).toBeGreaterThan(0);
      const explicitRel = relationships.find(
        r => r.sourceNodeId === 'node-1' && r.targetNodeId === 'node-2'
      );
      expect(explicitRel).toBeDefined();
    });

    it('should extract DERIVED_FROM relationships from metadata', () => {
      const nodes: ContentNode[] = [
        {
          id: 'source-1',
          contentType: 'source',
          title: 'Source',
          description: 'Source',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'derived-1',
          contentType: 'epic',
          title: 'Derived',
          description: 'Derived',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: { derivedFrom: 'source-1' },
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes);

      const derivedRel = relationships.find(
        r => r.relationshipType === ContentRelationshipType.DERIVED_FROM
      );
      expect(derivedRel).toBeDefined();
      expect(derivedRel?.sourceNodeId).toBe('derived-1');
      expect(derivedRel?.targetNodeId).toBe('source-1');
    });

    it('should extract path-based relationships for same epic', () => {
      const nodes: ContentNode[] = [
        {
          id: 'role-1',
          contentType: 'role',
          title: 'Role 1',
          description: 'Role',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: { epic: 'value-scanner', userType: 'adult' },
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'role-2',
          contentType: 'role',
          title: 'Role 2',
          description: 'Role',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: { epic: 'value-scanner', userType: 'adult' },
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes, {
        includePath: true,
        includeTags: false,
        includeContent: false
      });

      // Should find path-based relationship
      expect(relationships.length).toBeGreaterThan(0);
    });

    it('should skip path-based relationships for source nodes', () => {
      const nodes: ContentNode[] = [
        {
          id: 'source-1',
          contentType: 'source',
          title: 'Source 1',
          description: 'Source',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: { epic: 'value-scanner' },
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'source-2',
          contentType: 'source',
          title: 'Source 2',
          description: 'Source',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: { epic: 'value-scanner' },
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes, {
        includePath: true,
        includeTags: false,
        includeContent: false
      });

      // Should not create path-based relationships between source nodes
      const pathRels = relationships.filter(r => r.inferenceSource === 'path');
      expect(pathRels).toHaveLength(0);
    });

    it('should extract tag-based relationships', () => {
      const nodes: ContentNode[] = [
        {
          id: 'node-1',
          contentType: 'concept',
          title: 'Node 1',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: ['care-economy', 'value-recognition', 'community', 'trust'],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'node-2',
          contentType: 'concept',
          title: 'Node 2',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: ['care-economy', 'value-recognition', 'mutual-aid', 'trust'],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes, {
        includePath: false,
        includeTags: true,
        includeContent: false
      });

      // Should find tag-based relationships (shared meaningful tags)
      const tagRels = relationships.filter(r => r.inferenceSource === 'tag');
      expect(tagRels.length).toBeGreaterThan(0);
    });

    it('should skip common tags that do not indicate meaningful relationships', () => {
      const nodes: ContentNode[] = [
        {
          id: 'node-1',
          contentType: 'scenario',
          title: 'Node 1',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'gherkin',
          tags: ['scenario', 'feature', 'source'], // all common tags
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'node-2',
          contentType: 'scenario',
          title: 'Node 2',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'gherkin',
          tags: ['scenario', 'feature', 'documentation'],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes, {
        includePath: false,
        includeTags: true,
        includeContent: false
      });

      // Should not create relationships based only on common tags
      const tagRels = relationships.filter(r => r.inferenceSource === 'tag');
      expect(tagRels).toHaveLength(0);
    });

    it('should require at least 2 shared meaningful tags', () => {
      const nodes: ContentNode[] = [
        {
          id: 'node-1',
          contentType: 'concept',
          title: 'Node 1',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: ['unique-tag', 'common-tag'],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'node-2',
          contentType: 'concept',
          title: 'Node 2',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: ['different-tag', 'common-tag'],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes, {
        includePath: false,
        includeTags: true,
        includeContent: false
      });

      // Only 1 shared tag - should not meet threshold
      const tagRels = relationships.filter(r => r.inferenceSource === 'tag');
      expect(tagRels).toHaveLength(0);
    });

    it('should extract content-based relationships when enabled', () => {
      const nodes: ContentNode[] = [
        {
          id: 'node-1',
          contentType: 'epic',
          title: 'Value Scanner Epic',
          description: 'Epic about value scanning',
          content: 'This content references Value Scanner Epic',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'node-2',
          contentType: 'scenario',
          title: 'Scenario',
          description: 'Scenario',
          content: 'This scenario relates to Value Scanner Epic',
          contentFormat: 'gherkin',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes, {
        includePath: false,
        includeTags: false,
        includeContent: true
      });

      const contentRels = relationships.filter(r => r.inferenceSource === 'semantic');
      expect(contentRels.length).toBeGreaterThan(0);
    });

    it('should detect ID references in content', () => {
      const nodes: ContentNode[] = [
        {
          id: 'epic-value-scanner',
          contentType: 'epic',
          title: 'Value Scanner',
          description: 'Epic',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'scenario-1',
          contentType: 'scenario',
          title: 'Scenario',
          description: 'Scenario',
          content: 'This refers to epic-value-scanner',
          contentFormat: 'gherkin',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes, {
        includeContent: true
      });

      const idRef = relationships.find(
        r => r.sourceNodeId === 'scenario-1' && r.targetNodeId === 'epic-value-scanner'
      );
      expect(idRef).toBeDefined();
      expect(idRef?.relationshipType).toBe(ContentRelationshipType.REFERENCES);
    });

    it('should filter by minimum score', () => {
      const nodes: ContentNode[] = [
        {
          id: 'node-1',
          contentType: 'role',
          title: 'Role',
          description: 'Role',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: { epic: 'value-scanner' }, // score ~0.4
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'node-2',
          contentType: 'concept',
          title: 'Concept',
          description: 'Concept',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: { epic: 'value-scanner' },
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes, {
        includePath: true,
        minScore: 0.8 // High threshold
      });

      // Low-score path relationships should be filtered out
      expect(relationships.length).toBe(0);
    });

    it('should limit relationships per node', () => {
      // Create node with many potential relationships
      const nodes: ContentNode[] = [
        {
          id: 'hub-node',
          contentType: 'epic',
          title: 'Hub',
          description: 'Hub',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: Array.from({ length: 20 }, (_, i) => `node-${i}`),
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        ...Array.from({ length: 20 }, (_, i) => ({
          id: `node-${i}`,
          contentType: 'scenario' as const,
          title: `Node ${i}`,
          description: 'Desc',
          content: 'Content',
          contentFormat: 'gherkin' as const,
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }))
      ];

      const relationships = extractRelationships(nodes, {
        maxPerNode: 5
      });

      // Count relationships from hub-node
      const hubRels = relationships.filter(r => r.sourceNodeId === 'hub-node');
      expect(hubRels.length).toBeLessThanOrEqual(5);
    });

    it('should deduplicate bidirectional relationships', () => {
      const nodes: ContentNode[] = [
        {
          id: 'node-1',
          contentType: 'epic',
          title: 'Node 1',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: ['node-2'],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'node-2',
          contentType: 'epic',
          title: 'Node 2',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: ['node-1'],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes);

      // Should only have one relationship (deduplicated)
      const bothWays = relationships.filter(
        r =>
          (r.sourceNodeId === 'node-1' && r.targetNodeId === 'node-2') ||
          (r.sourceNodeId === 'node-2' && r.targetNodeId === 'node-1')
      );

      expect(bothWays.length).toBe(1);
    });

    it('should handle empty node array', () => {
      const relationships = extractRelationships([]);

      expect(relationships).toEqual([]);
    });

    it('should handle single node', () => {
      const nodes: ContentNode[] = [
        {
          id: 'lonely-node',
          contentType: 'concept',
          title: 'Lonely',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = extractRelationships(nodes);

      expect(relationships).toEqual([]);
    });
  });

  describe('buildRelationshipGraph', () => {
    it('should build bidirectional graph from relationships', () => {
      const nodes: ContentNode[] = [
        {
          id: 'node-1',
          contentType: 'epic',
          title: 'Node 1',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        },
        {
          id: 'node-2',
          contentType: 'scenario',
          title: 'Node 2',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'gherkin',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const relationships = [
        {
          id: 'rel-1',
          sourceNodeId: 'node-1',
          targetNodeId: 'node-2',
          relationshipType: ContentRelationshipType.RELATES_TO
        }
      ];

      const graph = buildRelationshipGraph(nodes, relationships);

      expect(graph.size).toBe(2);
      expect(graph.get('node-1')?.has('node-2')).toBe(true);
      expect(graph.get('node-2')?.has('node-1')).toBe(true); // bidirectional
    });

    it('should initialize all nodes even without relationships', () => {
      const nodes: ContentNode[] = [
        {
          id: 'isolated',
          contentType: 'concept',
          title: 'Isolated',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {},
          createdAt: '',
          updatedAt: ''
        }
      ];

      const graph = buildRelationshipGraph(nodes, []);

      expect(graph.size).toBe(1);
      expect(graph.get('isolated')).toBeDefined();
      expect(graph.get('isolated')?.size).toBe(0);
    });
  });

  describe('findConnectedComponents', () => {
    it('should find single connected component', () => {
      const graph = new Map([
        ['node-1', new Set(['node-2', 'node-3'])],
        ['node-2', new Set(['node-1', 'node-3'])],
        ['node-3', new Set(['node-1', 'node-2'])]
      ]);

      const components = findConnectedComponents(graph);

      expect(components).toHaveLength(1);
      expect(components[0].sort()).toEqual(['node-1', 'node-2', 'node-3']);
    });

    it('should find multiple connected components', () => {
      const graph = new Map([
        ['node-1', new Set(['node-2'])],
        ['node-2', new Set(['node-1'])],
        ['node-3', new Set(['node-4'])],
        ['node-4', new Set(['node-3'])]
      ]);

      const components = findConnectedComponents(graph);

      expect(components).toHaveLength(2);
      expect(components.some(c => c.includes('node-1') && c.includes('node-2'))).toBe(true);
      expect(components.some(c => c.includes('node-3') && c.includes('node-4'))).toBe(true);
    });

    it('should handle isolated nodes', () => {
      const graph = new Map<string, Set<string>>([
        ['isolated-1', new Set<string>()],
        ['isolated-2', new Set<string>()],
        ['connected-1', new Set(['connected-2'])],
        ['connected-2', new Set(['connected-1'])]
      ]);

      const components = findConnectedComponents(graph);

      expect(components).toHaveLength(3);
      expect(components.some(c => c.length === 1 && c[0] === 'isolated-1')).toBe(true);
      expect(components.some(c => c.length === 1 && c[0] === 'isolated-2')).toBe(true);
      expect(components.some(c => c.length === 2)).toBe(true);
    });

    it('should handle empty graph', () => {
      const graph = new Map();

      const components = findConnectedComponents(graph);

      expect(components).toEqual([]);
    });

    it('should handle large connected component', () => {
      // Create a chain: 1 -> 2 -> 3 -> ... -> 10
      const graph = new Map<string, Set<string>>();
      for (let i = 1; i <= 10; i++) {
        const neighbors = new Set<string>();
        if (i > 1) neighbors.add(`node-${i - 1}`);
        if (i < 10) neighbors.add(`node-${i + 1}`);
        graph.set(`node-${i}`, neighbors);
      }

      const components = findConnectedComponents(graph);

      expect(components).toHaveLength(1);
      expect(components[0]).toHaveLength(10);
    });
  });
});
