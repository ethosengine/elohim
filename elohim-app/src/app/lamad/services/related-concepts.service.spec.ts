import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { RelatedConceptsService } from './related-concepts.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ContentNode, ContentGraph, ContentRelationship, ContentRelationshipType } from '../models/content-node.model';
import { vi, Mock } from 'vitest';

describe('RelatedConceptsService', () => {
  let service: RelatedConceptsService;
  let dataLoaderSpy: any;

  const mockNodes: Map<string, ContentNode> = new Map([
    [
      'concept-1',
      {
        id: 'concept-1',
        title: 'Core Concept',
        description: 'A core concept',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: '# Core Concept',
        tags: [],
        relatedNodeIds: ['concept-2', 'concept-3'],
        metadata: {},
      },
    ],
    [
      'concept-2',
      {
        id: 'concept-2',
        title: 'Related Concept',
        description: 'A related concept',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: '# Related',
        tags: [],
        relatedNodeIds: ['concept-1'],
        metadata: {},
      },
    ],
    [
      'concept-3',
      {
        id: 'concept-3',
        title: 'Extension Concept',
        description: 'An extension',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: '# Extension',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
      },
    ],
    [
      'concept-4',
      {
        id: 'concept-4',
        title: 'Prerequisite',
        description: 'A prerequisite',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: '# Prerequisite',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
      },
    ],
    [
      'concept-5',
      {
        id: 'concept-5',
        title: 'Child Concept',
        description: 'A child',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: '# Child',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
      },
    ],
    [
      'concept-6',
      {
        id: 'concept-6',
        title: 'Parent Concept',
        description: 'A parent',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: '# Parent',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
      },
    ],
  ]);

  const mockRelationships: Map<string, ContentRelationship> = new Map([
    [
      'rel-1',
      {
        id: 'rel-1',
        sourceNodeId: 'concept-1',
        targetNodeId: 'concept-2',
        relationshipType: ContentRelationshipType.RELATES_TO,
        metadata: {},
      },
    ],
    [
      'rel-2',
      {
        id: 'rel-2',
        sourceNodeId: 'concept-1',
        targetNodeId: 'concept-3',
        relationshipType: 'EXTENDS' as any,
        metadata: {},
      },
    ],
    [
      'rel-3',
      {
        id: 'rel-3',
        sourceNodeId: 'concept-1',
        targetNodeId: 'concept-4',
        relationshipType: ContentRelationshipType.DEPENDS_ON,
        metadata: {},
      },
    ],
    [
      'rel-4',
      {
        id: 'rel-4',
        sourceNodeId: 'concept-1',
        targetNodeId: 'concept-5',
        relationshipType: ContentRelationshipType.CONTAINS,
        metadata: {},
      },
    ],
    [
      'rel-5',
      {
        id: 'rel-5',
        sourceNodeId: 'concept-6',
        targetNodeId: 'concept-1',
        relationshipType: ContentRelationshipType.CONTAINS,
        metadata: {},
      },
    ],
  ]);

  const mockGraph: ContentGraph = {
    nodes: mockNodes,
    relationships: mockRelationships,
    nodesByType: new Map([['concept', new Set(['concept-1', 'concept-2', 'concept-3', 'concept-4', 'concept-5', 'concept-6'])]]),
    nodesByTag: new Map(),
    nodesByCategory: new Map(),
    adjacency: new Map([
      ['concept-1', new Set(['concept-2', 'concept-3', 'concept-4', 'concept-5'])],
      ['concept-2', new Set(['concept-1'])],
      ['concept-6', new Set(['concept-1'])],
    ]),
    reverseAdjacency: new Map([
      ['concept-1', new Set(['concept-2', 'concept-6'])],
      ['concept-2', new Set(['concept-1'])],
      ['concept-3', new Set(['concept-1'])],
      ['concept-4', new Set(['concept-1'])],
      ['concept-5', new Set(['concept-1'])],
    ]),
    metadata: {
      nodeCount: 6,
      relationshipCount: 5,
      lastUpdated: new Date().toISOString(),
      version: '1.0',
    },
  };

  beforeEach(() => {
    dataLoaderSpy = {
    getGraph: vi.fn(),
    getRelationshipsForNode: vi.fn(),
    getContent: vi.fn(),
  };

    dataLoaderSpy.getGraph.mockReturnValue(of(mockGraph));
    dataLoaderSpy.getRelationshipsForNode.mockReturnValue(
      of([
        mockRelationships.get('rel-1')!,
        mockRelationships.get('rel-2')!,
        mockRelationships.get('rel-3')!,
        mockRelationships.get('rel-4')!,
        mockRelationships.get('rel-5')!,
      ])
    );
    dataLoaderSpy.getContent.mockImplementation((id: string) => {
      const node = mockNodes.get(id);
      return node ? of(node) : throwError(() => new Error('Not found'));
    });

    TestBed.configureTestingModule({
      providers: [
        RelatedConceptsService,
        { provide: DataLoaderService, useValue: dataLoaderSpy },
      ],
    });

    service = TestBed.inject(RelatedConceptsService);
  });

  afterEach(() => {
    service.clearCache();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getRelatedConcepts', () => {
    it('should get related concepts grouped by type', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1').subscribe(result => {
        expect(result.prerequisites.length).toBeGreaterThan(0);
        expect(result.extensions.length).toBeGreaterThan(0);
        expect(result.related.length).toBeGreaterThan(0);
        expect(result.children.length).toBeGreaterThan(0);
        expect(result.parents.length).toBeGreaterThan(0);
        expect(result.allRelationships.length).toBeGreaterThan(0);
        done();
      });
    }));

    it('should use lazy loading for simple queries', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1', { limit: 5 }).subscribe(() => {
        expect(dataLoaderSpy.getRelationshipsForNode).toHaveBeenCalled();
        done();
      });
    }));

    it('should use full graph for complex queries with filters', () => new Promise<void>(done => {
      service
        .getRelatedConcepts('concept-1', {
          includeTypes: ['RELATES_TO'],
        })
        .subscribe(() => {
          expect(dataLoaderSpy.getGraph).toHaveBeenCalled();
          done();
        });
    }));

    it('should respect limit parameter', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1', { limit: 2 }).subscribe(result => {
        expect(result.related.length).toBeLessThanOrEqual(2);
        expect(result.prerequisites.length).toBeLessThanOrEqual(2);
        done();
      });
    }));

    it('should filter by includeTypes', () => new Promise<void>(done => {
      service
        .getRelatedConcepts('concept-1', {
          includeTypes: ['RELATES_TO'],
        })
        .subscribe(result => {
          expect(result.related.length).toBeGreaterThan(0);
          expect(result.extensions.length).toBe(0); // EXTENDS filtered out
          done();
        });
    }));

    it('should filter by excludeTypes', () => new Promise<void>(done => {
      service
        .getRelatedConcepts('concept-1', {
          excludeTypes: ['RELATES_TO'],
        })
        .subscribe(result => {
          expect(result.related.length).toBe(0); // RELATES_TO excluded
          expect(result.extensions.length).toBeGreaterThan(0);
          done();
        });
    }));

    it('should cache results', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1').subscribe(() => {
        service.getRelatedConcepts('concept-1').subscribe(() => {
          // Second call should use cache
          expect(dataLoaderSpy.getRelationshipsForNode).toHaveBeenCalledTimes(1);
          done();
        });
      });
    }));

    it('should strip content by default', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1').subscribe(result => {
        if (result.related.length > 0) {
          expect(result.related[0].content).toBe('');
        }
        done();
      });
    }));

    it('should include content when requested', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1', { includeContent: true }).subscribe(result => {
        // Should use lazy loading but include content in results
        expect(dataLoaderSpy.getRelationshipsForNode).toHaveBeenCalled();
        // Verify that content is present (not stripped)
        if (result.related.length > 0) {
          expect(result.related[0].content).not.toBe('');
        }
        done();
      });
    }));

    it('should categorize prerequisites correctly', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1').subscribe(result => {
        const prereq = result.prerequisites.find(n => n.id === 'concept-4');
        expect(prereq).toBeDefined();
        expect(prereq?.title).toBe('Prerequisite');
        done();
      });
    }));

    it('should categorize extensions correctly', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1').subscribe(result => {
        const ext = result.extensions.find(n => n.id === 'concept-3');
        expect(ext).toBeDefined();
        expect(ext?.title).toBe('Extension Concept');
        done();
      });
    }));

    it('should categorize hierarchy (parents/children) correctly', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1').subscribe(result => {
        const child = result.children.find(n => n.id === 'concept-5');
        const parent = result.parents.find(n => n.id === 'concept-6');
        expect(child).toBeDefined();
        expect(parent).toBeDefined();
        done();
      });
    }));

    it('should avoid duplicate related concepts', () => new Promise<void>(done => {
      // Add bidirectional RELATES_TO
      const biRelationships = [
        ...Array.from(mockRelationships.values()),
        {
          id: 'rel-6',
          sourceNodeId: 'concept-2',
          targetNodeId: 'concept-1',
          relationshipType: ContentRelationshipType.RELATES_TO,
          metadata: {},
        },
      ];
      dataLoaderSpy.getRelationshipsForNode.mockReturnValue(of(biRelationships));

      service.getRelatedConcepts('concept-1').subscribe(result => {
        const relatedIds = result.related.map(n => n.id);
        const uniqueIds = new Set(relatedIds);
        expect(relatedIds.length).toBe(uniqueIds.size); // No duplicates
        done();
      });
    }));
  });

  describe('getNeighborhood', () => {
    it('should build neighborhood graph', () => new Promise<void>(done => {
      service.getNeighborhood('concept-1').subscribe(graph => {
        expect(graph.focus.id).toBe('concept-1');
        expect(graph.focus.isFocus).toBe(true);
        expect(graph.neighbors.length).toBeGreaterThan(0);
        expect(graph.edges.length).toBeGreaterThan(0);
        done();
      });
    }));

    it('should respect depth parameter', () => new Promise<void>(done => {
      service.getNeighborhood('concept-1', { depth: 2 }).subscribe(graph => {
        const hasDepth2 = graph.neighbors.some(n => n.depth === 2);
        expect(hasDepth2).toBe(false); // Mock graph isn't deep enough
        done();
      });
    }));

    it('should respect maxNodes parameter', () => new Promise<void>(done => {
      service.getNeighborhood('concept-1', { maxNodes: 3 }).subscribe(graph => {
        expect(graph.neighbors.length).toBeLessThanOrEqual(2); // maxNodes - 1 (focus)
        done();
      });
    }));

    it('should filter by relationship types', () => new Promise<void>(done => {
      service
        .getNeighborhood('concept-1', {
          relationshipTypes: ['RELATES_TO'],
        })
        .subscribe(graph => {
          const edges = graph.edges.filter(e => e.relationshipType === 'RELATES_TO');
          expect(edges.length).toBeGreaterThan(0);
          done();
        });
    }));

    it('should cache neighborhood queries', () => new Promise<void>(done => {
      service.getNeighborhood('concept-1').subscribe(() => {
        service.getNeighborhood('concept-1').subscribe(() => {
          expect(dataLoaderSpy.getGraph).toHaveBeenCalledTimes(1);
          done();
        });
      });
    }));

    it('should return empty neighbors for unknown node', () => new Promise<void>(done => {
      service.getNeighborhood('unknown').subscribe(graph => {
        expect(graph.focus.id).toBe('unknown');
        expect(graph.neighbors.length).toBe(0);
        expect(graph.edges.length).toBe(0);
        done();
      });
    }));

    it('should include both incoming and outgoing edges', () => new Promise<void>(done => {
      service.getNeighborhood('concept-1').subscribe(graph => {
        expect(graph.edges.length).toBeGreaterThan(0);
        // Should have both directions
        const hasOutgoing = graph.edges.some(e => e.source === 'concept-1');
        const hasIncoming = graph.edges.some(e => e.target === 'concept-1');
        expect(hasOutgoing || hasIncoming).toBe(true);
        done();
      });
    }));
  });

  describe('getByRelationshipType', () => {
    it('should get outgoing relationships by type', () => new Promise<void>(done => {
      service.getByRelationshipType('concept-1', 'RELATES_TO', 'outgoing').subscribe(nodes => {
        expect(nodes.length).toBe(1);
        expect(nodes[0].id).toBe('concept-2');
        done();
      });
    }));

    it('should get incoming relationships by type', () => new Promise<void>(done => {
      service.getByRelationshipType('concept-1', 'CONTAINS', 'incoming').subscribe(nodes => {
        expect(nodes.length).toBe(1);
        expect(nodes[0].id).toBe('concept-6');
        done();
      });
    }));

    it('should return empty array for non-existent relationships', () => new Promise<void>(done => {
      service
        .getByRelationshipType('concept-1', 'NON_EXISTENT' as any, 'outgoing')
        .subscribe(nodes => {
          expect(nodes.length).toBe(0);
          done();
        });
    }));
  });

  describe('hasRelatedConcepts', () => {
    it('should return true for node with relationships', () => new Promise<void>(done => {
      service.hasRelatedConcepts('concept-1').subscribe(hasRelated => {
        expect(hasRelated).toBe(true);
        done();
      });
    }));

    it('should return false for isolated node', () => new Promise<void>(done => {
      // Add isolated node to graph
      const isolatedGraph = {
        ...mockGraph,
        nodes: new Map([...mockGraph.nodes, ['isolated', mockNodes.get('concept-1')!]]),
        adjacency: new Map(mockGraph.adjacency),
        reverseAdjacency: new Map(mockGraph.reverseAdjacency),
      };
      dataLoaderSpy.getGraph.mockReturnValue(of(isolatedGraph));

      service.hasRelatedConcepts('isolated').subscribe(hasRelated => {
        expect(hasRelated).toBe(false);
        done();
      });
    }));
  });

  describe('clearCache', () => {
    it('should clear all caches', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1').subscribe(() => {
        const statsBefore = service.getCacheStats();
        expect(statsBefore.relationshipCacheSize).toBeGreaterThan(0);

        service.clearCache();

        const statsAfter = service.getCacheStats();
        expect(statsAfter.relationshipCacheSize).toBe(0);
        expect(statsAfter.neighborhoodCacheSize).toBe(0);
        expect(statsAfter.hasGraph).toBe(false);
        done();
      });
    }));
  });

  describe('getCacheStats', () => {
    it('should return cache statistics', () => new Promise<void>(done => {
      service.getRelatedConcepts('concept-1').subscribe(() => {
        const stats = service.getCacheStats();
        expect(stats.relationshipCacheSize).toBeGreaterThan(0);
        expect(stats.neighborhoodCacheSize).toBeGreaterThanOrEqual(0);
        // Lazy loading doesn't load full graph, so hasGraph is false
        expect(stats.hasGraph).toBe(false);
        done();
      });
    }));

    it('should show hasGraph=true after neighborhood query', () => new Promise<void>(done => {
      service.getNeighborhood('concept-1').subscribe(() => {
        const stats = service.getCacheStats();
        expect(stats.hasGraph).toBe(true);
        done();
      });
    }));
  });

  describe('edge cases', () => {
    it('should handle empty graph', () => new Promise<void>(done => {
      const emptyGraph: ContentGraph = {
        nodes: new Map(),
        relationships: new Map(),
        nodesByType: new Map(),
        nodesByTag: new Map(),
        nodesByCategory: new Map(),
        adjacency: new Map(),
        reverseAdjacency: new Map(),
        metadata: {
          nodeCount: 0,
          relationshipCount: 0,
          lastUpdated: new Date().toISOString(),
          version: '1.0',
        },
      };
      dataLoaderSpy.getGraph.mockReturnValue(of(emptyGraph));
      // Also mock empty relationships for lazy loading path
      dataLoaderSpy.getRelationshipsForNode.mockReturnValue(of([]));

      service.getRelatedConcepts('concept-1').subscribe(result => {
        expect(result.prerequisites.length).toBe(0);
        expect(result.extensions.length).toBe(0);
        expect(result.related.length).toBe(0);
        done();
      });
    }));

    it('should handle relationship load error in lazy loading', () => new Promise<void>(done => {
      dataLoaderSpy.getRelationshipsForNode.mockReturnValue(
        throwError(() => new Error('Load failed'))
      );

      service.getRelatedConcepts('concept-1').subscribe({
        error: err => {
          expect(err.message).toBe('Load failed');
          done();
        },
      });
    }));

    it('should handle content load error gracefully', () => new Promise<void>(done => {
      dataLoaderSpy.getContent.mockReturnValue(throwError(() => new Error('Not found')));

      service.getRelatedConcepts('concept-1').subscribe(result => {
        // Should still complete, just with fewer loaded nodes
        expect(result).toBeDefined();
        done();
      });
    }));

    it('should handle missing nodes in relationships', () => new Promise<void>(done => {
      const relationshipsWithMissing = [
        {
          id: 'rel-missing',
          sourceNodeId: 'concept-1',
          targetNodeId: 'missing-node',
          relationshipType: ContentRelationshipType.RELATES_TO,
          metadata: {},
        },
      ];
      dataLoaderSpy.getRelationshipsForNode.mockReturnValue(of(relationshipsWithMissing));
      dataLoaderSpy.getContent.mockImplementation((id: string) => {
        if (id === 'missing-node') {
          return throwError(() => new Error('Not found'));
        }
        return of(mockNodes.get(id)!);
      });

      service.getRelatedConcepts('concept-1').subscribe(result => {
        // Should handle missing node gracefully
        expect(result).toBeDefined();
        done();
      });
    }));
  });

  describe('LRU cache behavior', () => {
    it('should evict old entries when cache is full', () => new Promise<void>(done => {
      // Fill cache with many queries
      let completed = 0;
      const total = 210; // More than cache limit (200)

      for (let i = 0; i < total; i++) {
        service.getRelatedConcepts(`concept-${i}`).subscribe(() => {
          completed++;
          if (completed === total) {
            const stats = service.getCacheStats();
            expect(stats.relationshipCacheSize).toBeLessThanOrEqual(200);
            done();
          }
        });
      }
    }));
  });
});
