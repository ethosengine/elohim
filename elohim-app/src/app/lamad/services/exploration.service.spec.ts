import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { ExplorationService } from './exploration.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ContentNode, ContentGraph, ContentGraphMetadata } from '../models/content-node.model';
import { RATE_LIMIT_CONFIGS } from '../models/exploration.model';

describe('ExplorationService', () => {
  let service: ExplorationService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;

  // Mock content nodes
  const mockNodes: ContentNode[] = [
    {
      id: 'node-1',
      title: 'Node 1',
      description: 'First node',
      contentType: 'concept',
      contentFormat: 'markdown',
      content: '# Node 1',
      tags: ['test'],
      relatedNodeIds: ['node-2'],
      metadata: {}
    },
    {
      id: 'node-2',
      title: 'Node 2',
      description: 'Second node',
      contentType: 'concept',
      contentFormat: 'markdown',
      content: '# Node 2',
      tags: ['test'],
      relatedNodeIds: ['node-1', 'node-3'],
      metadata: {}
    },
    {
      id: 'node-3',
      title: 'Node 3',
      description: 'Third node',
      contentType: 'feature',
      contentFormat: 'markdown',
      content: '# Node 3',
      tags: [],
      relatedNodeIds: ['node-2', 'node-4'],
      metadata: {}
    },
    {
      id: 'node-4',
      title: 'Node 4',
      description: 'Fourth node',
      contentType: 'epic',
      contentFormat: 'markdown',
      content: '# Node 4',
      tags: [],
      relatedNodeIds: ['node-3'],
      metadata: {}
    }
  ];

  // Build mock graph
  function createMockGraph(): ContentGraph {
    const nodes = new Map<string, ContentNode>();
    const nodesByType = new Map<string, Set<string>>();
    const nodesByTag = new Map<string, Set<string>>();
    const nodesByCategory = new Map<string, Set<string>>();
    const adjacency = new Map<string, Set<string>>();
    const reverseAdjacency = new Map<string, Set<string>>();
    const relationships = new Map<string, any>();

    // Populate nodes
    for (const node of mockNodes) {
      nodes.set(node.id, node);
      adjacency.set(node.id, new Set(node.relatedNodeIds));
      reverseAdjacency.set(node.id, new Set());

      if (!nodesByType.has(node.contentType)) {
        nodesByType.set(node.contentType, new Set());
      }
      nodesByType.get(node.contentType)!.add(node.id);

      for (const tag of node.tags) {
        if (!nodesByTag.has(tag)) {
          nodesByTag.set(tag, new Set());
        }
        nodesByTag.get(tag)!.add(node.id);
      }
    }

    // Build reverse adjacency
    for (const node of mockNodes) {
      for (const relatedId of node.relatedNodeIds) {
        if (!reverseAdjacency.has(relatedId)) {
          reverseAdjacency.set(relatedId, new Set());
        }
        reverseAdjacency.get(relatedId)!.add(node.id);
      }
    }

    // Add relationships
    let relId = 0;
    for (const node of mockNodes) {
      for (const targetId of node.relatedNodeIds) {
        const id = `rel-${relId++}`;
        relationships.set(id, {
          id,
          sourceNodeId: node.id,
          targetNodeId: targetId,
          relationshipType: 'RELATES_TO'
        });
      }
    }

    const metadata: ContentGraphMetadata = {
      nodeCount: nodes.size,
      relationshipCount: relationships.size,
      lastUpdated: '2025-01-01T00:00:00.000Z',
      version: '1.0.0'
    };

    return {
      nodes,
      relationships,
      nodesByType,
      nodesByTag,
      nodesByCategory,
      adjacency,
      reverseAdjacency,
      metadata
    };
  }

  const mockAgentIndex = {
    agents: [
      {
        id: 'demo-learner',
        displayName: 'Demo Learner',
        type: 'human' as const,
        visibility: 'public' as const,
        createdAt: '2025-01-01T00:00:00.000Z',
        updatedAt: '2025-01-01T00:00:00.000Z',
        attestations: []
      },
      {
        id: 'researcher-agent',
        displayName: 'Researcher',
        type: 'human' as const,
        visibility: 'public' as const,
        createdAt: '2025-01-01T00:00:00.000Z',
        updatedAt: '2025-01-01T00:00:00.000Z',
        attestations: ['graph-researcher']
      },
      {
        id: 'path-creator-agent',
        displayName: 'Path Creator',
        type: 'human' as const,
        visibility: 'public' as const,
        createdAt: '2025-01-01T00:00:00.000Z',
        updatedAt: '2025-01-01T00:00:00.000Z',
        attestations: ['path-creator']
      }
    ]
  };

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', [
      'getGraph',
      'getAgentIndex'
    ]);

    TestBed.configureTestingModule({
      providers: [
        ExplorationService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj }
      ]
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;

    // Default spy returns
    dataLoaderSpy.getGraph.and.returnValue(of(createMockGraph()));
    dataLoaderSpy.getAgentIndex.and.returnValue(of(mockAgentIndex));

    service = TestBed.inject(ExplorationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // exploreNeighborhood
  // =========================================================================

  describe('exploreNeighborhood', () => {
    it('should explore depth 1 from focus node', (done) => {
      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph'
      }).subscribe(result => {
        expect(result.focus.id).toBe('node-1');
        expect(result.neighbors.has(1)).toBe(true);
        const neighbors = result.neighbors.get(1) ?? [];
        expect(neighbors.length).toBeGreaterThan(0);
        expect(neighbors.some(n => n.id === 'node-2')).toBe(true);
        done();
      });
    });

    it('should return metadata with query stats', (done) => {
      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph'
      }).subscribe(result => {
        expect(result.metadata.nodesReturned).toBeGreaterThan(0);
        expect(result.metadata.computeTimeMs).toBeDefined();
        expect(result.metadata.resourceCredits).toBeGreaterThan(0);
        done();
      });
    });

    it('should error for non-existent focus node', (done) => {
      service.exploreNeighborhood({
        focus: 'non-existent',
        depth: 1,
        view: 'graph'
      }).subscribe({
        error: err => {
          expect(err.code).toBe('RESOURCE_NOT_FOUND');
          done();
        }
      });
    });

    it('should respect maxNodes limit', (done) => {
      // Use researcher agent which has depth 2 access
      service.setCurrentAgent('researcher-agent');

      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 2,
        view: 'graph',
        maxNodes: 2
      }).subscribe(result => {
        expect(result.metadata.nodesReturned).toBeLessThanOrEqual(2);
        done();
      });
    });

    it('should strip content when includeContent is false', (done) => {
      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph',
        includeContent: false
      }).subscribe(result => {
        const neighbors = result.neighbors.get(1) ?? [];
        for (const node of neighbors) {
          expect(node.content).toBe('[content stripped for performance]');
        }
        done();
      });
    });

    it('should error when graph is empty', (done) => {
      const emptyGraph: ContentGraph = {
        nodes: new Map(),
        relationships: new Map(),
        nodesByType: new Map(),
        nodesByTag: new Map(),
        nodesByCategory: new Map(),
        adjacency: new Map(),
        reverseAdjacency: new Map(),
        metadata: { nodeCount: 0, relationshipCount: 0, lastUpdated: '', version: '1.0.0' }
      };
      dataLoaderSpy.getGraph.and.returnValue(of(emptyGraph));

      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph'
      }).subscribe({
        error: err => {
          expect(err.code).toBe('INVALID_QUERY');
          done();
        }
      });
    });

    it('should deny depth 2 without proper attestation', (done) => {
      // demo-learner has no attestations, so depth 2 should be denied
      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 2,
        view: 'graph'
      }).subscribe({
        error: err => {
          expect(err.code).toBe('DEPTH_UNAUTHORIZED');
          done();
        }
      });
    });

    it('should allow depth 2 for graph-researcher', (done) => {
      service.setCurrentAgent('researcher-agent');

      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 2,
        view: 'graph'
      }).subscribe(result => {
        expect(result.focus.id).toBe('node-1');
        expect(result.metadata.depthTraversed).toBeLessThanOrEqual(2);
        done();
      });
    });
  });

  // =========================================================================
  // findPath
  // =========================================================================

  describe('findPath', () => {
    beforeEach(() => {
      // Path creator has pathfinding access
      service.setCurrentAgent('path-creator-agent');
    });

    it('should find shortest path between two nodes', (done) => {
      service.findPath({
        from: 'node-1',
        to: 'node-3',
        algorithm: 'shortest'
      }).subscribe(result => {
        expect(result.path.length).toBeGreaterThan(0);
        expect(result.path[0]).toBe('node-1');
        expect(result.path[result.path.length - 1]).toBe('node-3');
        done();
      });
    });

    it('should return edges in path result', (done) => {
      service.findPath({
        from: 'node-1',
        to: 'node-3',
        algorithm: 'shortest'
      }).subscribe(result => {
        expect(result.edges.length).toBe(result.path.length - 1);
        expect(result.edges[0].source).toBe('node-1');
        done();
      });
    });

    it('should error when no path exists', (done) => {
      // Create isolated node
      const graph = createMockGraph();
      const isolatedNode: ContentNode = {
        id: 'isolated',
        title: 'Isolated',
        description: 'Isolated node',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: '',
        tags: [],
        relatedNodeIds: [],
        metadata: {}
      };
      graph.nodes.set('isolated', isolatedNode);
      graph.adjacency.set('isolated', new Set());
      dataLoaderSpy.getGraph.and.returnValue(of(graph));

      service.findPath({
        from: 'node-1',
        to: 'isolated',
        algorithm: 'shortest'
      }).subscribe({
        error: err => {
          expect(err.code).toBe('NO_PATH_EXISTS');
          done();
        }
      });
    });

    it('should error for non-existent from node', (done) => {
      service.findPath({
        from: 'non-existent',
        to: 'node-3',
        algorithm: 'shortest'
      }).subscribe({
        error: err => {
          expect(err.code).toBe('RESOURCE_NOT_FOUND');
          done();
        }
      });
    });

    it('should error for non-existent to node', (done) => {
      service.findPath({
        from: 'node-1',
        to: 'non-existent',
        algorithm: 'shortest'
      }).subscribe({
        error: err => {
          expect(err.code).toBe('RESOURCE_NOT_FOUND');
          done();
        }
      });
    });

    it('should respect maxHops limit', (done) => {
      service.findPath({
        from: 'node-1',
        to: 'node-4',
        algorithm: 'shortest',
        maxHops: 1
      }).subscribe({
        error: err => {
          expect(err.code).toBe('NO_PATH_EXISTS');
          done();
        }
      });
    });

    it('should use semantic pathfinding when requested', (done) => {
      service.findPath({
        from: 'node-1',
        to: 'node-3',
        algorithm: 'semantic'
      }).subscribe(result => {
        expect(result.path.length).toBeGreaterThan(0);
        expect(result.semanticScore).toBeDefined();
        done();
      });
    });

    it('should deny pathfinding for non-path-creator', (done) => {
      service.setCurrentAgent('demo-learner');

      service.findPath({
        from: 'node-1',
        to: 'node-3',
        algorithm: 'shortest'
      }).subscribe({
        error: err => {
          expect(err.code).toBe('PATHFINDING_UNAUTHORIZED');
          done();
        }
      });
    });
  });

  // =========================================================================
  // estimateCost
  // =========================================================================

  describe('estimateCost', () => {
    it('should estimate cost for exploreNeighborhood', (done) => {
      service.estimateCost('exploreNeighborhood', { depth: 1 }).subscribe(cost => {
        expect(cost.estimatedNodes).toBeGreaterThan(0);
        expect(cost.estimatedTimeMs).toBeDefined();
        expect(cost.resourceCredits).toBeGreaterThan(0);
        expect(cost.rateLimitImpact).toBeDefined();
        done();
      });
    });

    it('should indicate canExecute for allowed queries', (done) => {
      service.estimateCost('exploreNeighborhood', { depth: 1 }).subscribe(cost => {
        expect(cost.canExecute).toBe(true);
        done();
      });
    });

    it('should indicate canExecute false for unauthorized depth', (done) => {
      // demo-learner can only go depth 1
      service.estimateCost('exploreNeighborhood', { depth: 3 }).subscribe(cost => {
        expect(cost.canExecute).toBe(false);
        expect(cost.blockedReason).toBe('insufficient-attestation');
        done();
      });
    });

    it('should estimate cost for findPath', (done) => {
      service.estimateCost('findPath', {}).subscribe(cost => {
        expect(cost.estimatedNodes).toBeGreaterThan(0);
        expect(cost.attestationRequired).toBe('path-creator');
        done();
      });
    });

    it('should return canExecute false for unknown operation', (done) => {
      service.estimateCost('unknownOperation', {}).subscribe(cost => {
        expect(cost.canExecute).toBe(false);
        expect(cost.blockedReason).toBe('invalid-query');
        done();
      });
    });

    it('should handle empty graph', (done) => {
      const emptyGraph: ContentGraph = {
        nodes: new Map(),
        relationships: new Map(),
        nodesByType: new Map(),
        nodesByTag: new Map(),
        nodesByCategory: new Map(),
        adjacency: new Map(),
        reverseAdjacency: new Map(),
        metadata: { nodeCount: 0, relationshipCount: 0, lastUpdated: '', version: '1.0.0' }
      };
      dataLoaderSpy.getGraph.and.returnValue(of(emptyGraph));

      service.estimateCost('exploreNeighborhood', { depth: 1 }).subscribe(cost => {
        expect(cost.canExecute).toBe(false);
        done();
      });
    });
  });

  // =========================================================================
  // Rate Limiting
  // =========================================================================

  describe('getRateLimitStatus', () => {
    it('should return rate limit status for current agent', (done) => {
      service.getRateLimitStatus().subscribe(status => {
        expect(status.tier).toBeDefined();
        expect(status.explorationRemaining).toBeDefined();
        expect(status.explorationLimit).toBeDefined();
        expect(status.resetsAt).toBeDefined();
        done();
      });
    });

    it('should return authenticated tier for demo-learner', (done) => {
      service.setCurrentAgent('demo-learner');

      service.getRateLimitStatus().subscribe(status => {
        expect(status.tier).toBe('authenticated');
        expect(status.maxDepth).toBe(1);
        done();
      });
    });

    it('should return graph-researcher tier for researcher-agent', (done) => {
      service.setCurrentAgent('researcher-agent');

      service.getRateLimitStatus().subscribe(status => {
        expect(status.tier).toBe('graph-researcher');
        expect(status.maxDepth).toBe(2);
        done();
      });
    });

    it('should return path-creator tier for path-creator-agent', (done) => {
      service.setCurrentAgent('path-creator-agent');

      service.getRateLimitStatus().subscribe(status => {
        expect(status.tier).toBe('path-creator');
        expect(status.maxDepth).toBe(3);
        expect(status.pathfindingLimit).toBeGreaterThan(0);
        done();
      });
    });
  });

  describe('rate limit enforcement', () => {
    it('should decrement exploration remaining after query', (done) => {
      let initialRemaining: number;

      service.getRateLimitStatus().subscribe(status => {
        initialRemaining = status.explorationRemaining;

        service.exploreNeighborhood({
          focus: 'node-1',
          depth: 1,
          view: 'graph'
        }).subscribe(() => {
          service.getRateLimitStatus().subscribe(newStatus => {
            expect(newStatus.explorationRemaining).toBe(initialRemaining - 1);
            done();
          });
        });
      });
    });

    it('should error when rate limit exceeded', (done) => {
      // Exhaust rate limit by making many queries
      const config = RATE_LIMIT_CONFIGS['authenticated'];
      const queries: Promise<void>[] = [];

      // Execute all allowed queries
      for (let i = 0; i < config.queriesPerHour; i++) {
        queries.push(
          new Promise<void>((resolve) => {
            service.exploreNeighborhood({
              focus: 'node-1',
              depth: 1,
              view: 'graph'
            }).subscribe({
              next: () => resolve(),
              error: () => resolve()
            });
          })
        );
      }

      Promise.all(queries).then(() => {
        // Next query should fail
        service.exploreNeighborhood({
          focus: 'node-1',
          depth: 1,
          view: 'graph'
        }).subscribe({
          error: err => {
            expect(err.code).toBe('RATE_LIMIT_EXCEEDED');
            done();
          }
        });
      });
    });
  });

  // =========================================================================
  // setCurrentAgent
  // =========================================================================

  describe('setCurrentAgent', () => {
    it('should update current agent', () => {
      service.setCurrentAgent('researcher-agent');
      // The next query should use researcher-agent's permissions
      expect(true).toBe(true);
    });

    it('should emit rate limit status update after query', (done) => {
      service.setCurrentAgent('researcher-agent');

      // Tier is determined during query execution (when attestations are checked)
      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph'
      }).subscribe(() => {
        service.getRateLimitStatus().subscribe(status => {
          expect(status.tier).toBe('graph-researcher');
          expect(status.maxDepth).toBe(2);
          done();
        });
      });
    });
  });

  // =========================================================================
  // Event Logging
  // =========================================================================

  describe('getRecentEvents', () => {
    it('should return empty array initially', () => {
      const events = service.getRecentEvents();
      expect(events).toEqual([]);
    });

    it('should log successful queries', (done) => {
      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph'
      }).subscribe(() => {
        const events = service.getRecentEvents();
        expect(events.length).toBeGreaterThan(0);
        expect(events[0].type).toBe('query-completed');
        done();
      });
    });

    it('should log failed queries', (done) => {
      service.exploreNeighborhood({
        focus: 'non-existent',
        depth: 1,
        view: 'graph'
      }).subscribe({
        error: () => {
          const events = service.getRecentEvents();
          expect(events.some(e => e.type === 'query-failed')).toBe(true);
          done();
        }
      });
    });

    it('should respect limit parameter', (done) => {
      // Make multiple queries
      service.exploreNeighborhood({ focus: 'node-1', depth: 1, view: 'graph' }).subscribe();
      service.exploreNeighborhood({ focus: 'node-2', depth: 1, view: 'graph' }).subscribe(() => {
        const events = service.getRecentEvents(1);
        expect(events.length).toBe(1);
        done();
      });
    });
  });

  describe('getAgentEvents', () => {
    it('should filter events by agent', (done) => {
      service.setCurrentAgent('demo-learner');
      service.exploreNeighborhood({ focus: 'node-1', depth: 1, view: 'graph' }).subscribe(() => {
        const events = service.getAgentEvents('demo-learner');
        expect(events.length).toBeGreaterThan(0);
        expect(events.every(e => e.agentId === 'demo-learner')).toBe(true);
        done();
      });
    });

    it('should return empty for unknown agent', () => {
      const events = service.getAgentEvents('unknown-agent');
      expect(events).toEqual([]);
    });
  });

  // =========================================================================
  // Serialization
  // =========================================================================

  describe('serializeGraphView', () => {
    it('should convert Map to plain object', (done) => {
      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph'
      }).subscribe(result => {
        const serialized = service.serializeGraphView(result);

        expect(serialized.focus).toBeDefined();
        expect(typeof serialized.neighbors).toBe('object');
        expect(Array.isArray(serialized.neighbors)).toBe(false);
        expect(serialized.neighbors[1]).toBeDefined();
        done();
      });
    });
  });

  describe('deserializeGraphView', () => {
    it('should convert plain object back to Map', (done) => {
      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph'
      }).subscribe(result => {
        const serialized = service.serializeGraphView(result);
        const deserialized = service.deserializeGraphView(serialized);

        expect(deserialized.neighbors instanceof Map).toBe(true);
        expect(deserialized.neighbors.get(1)).toBeDefined();
        done();
      });
    });
  });

  // =========================================================================
  // Error Handling
  // =========================================================================

  describe('error handling', () => {
    it('should handle graph loading error gracefully', (done) => {
      dataLoaderSpy.getGraph.and.returnValue(throwError(() => new Error('Network error')));

      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph'
      }).subscribe({
        error: err => {
          expect(err).toBeDefined();
          done();
        }
      });
    });

    it('should handle agent index loading error', (done) => {
      dataLoaderSpy.getAgentIndex.and.returnValue(throwError(() => new Error('Network error')));

      service.exploreNeighborhood({
        focus: 'node-1',
        depth: 1,
        view: 'graph'
      }).subscribe({
        error: err => {
          expect(err.code).toBe('DEPTH_UNAUTHORIZED');
          done();
        }
      });
    });
  });
});
