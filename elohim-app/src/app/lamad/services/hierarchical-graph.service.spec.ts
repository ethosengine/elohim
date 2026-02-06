import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { HierarchicalGraphService } from './hierarchical-graph.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import { LearningPath, PathChapter, PathModule, PathSection } from '../models/learning-path.model';
import { ClusterNode, ClusterGraphData } from '../models/cluster-graph.model';
import { ContentNode } from '../models/content-node.model';

describe('HierarchicalGraphService', () => {
  let service: HierarchicalGraphService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let affinitySpy: jasmine.SpyObj<AffinityTrackingService>;

  const mockPath: LearningPath = {
    id: 'test-path',
    version: '1.0.0',
    title: 'Test Learning Path',
    description: 'A hierarchical path for testing',
    purpose: 'Testing',
    createdBy: 'test-user',
    contributors: [],
    createdAt: '2025-01-01T00:00:00.000Z',
    updatedAt: '2025-01-01T00:00:00.000Z',
    difficulty: 'intermediate',
    estimatedDuration: '5 hours',
    tags: ['test', 'hierarchical'],
    visibility: 'public',
    steps: [],
    chapters: [
      {
        id: 'chapter-1',
        title: 'Introduction',
        description: 'Getting started',
        order: 1,
        modules: [
          {
            id: 'module-1-1',
            title: 'Basics',
            description: 'Basic concepts',
            order: 1,
            sections: [
              {
                id: 'section-1-1-1',
                title: 'Core Concepts',
                description: 'Core concepts section',
                order: 1,
                conceptIds: ['concept-1', 'concept-2', 'concept-3'],
              },
              {
                id: 'section-1-1-2',
                title: 'Advanced Topics',
                description: 'Advanced topics',
                order: 2,
                conceptIds: ['concept-4', 'concept-5'],
              },
            ],
          },
          {
            id: 'module-1-2',
            title: 'Intermediate',
            description: 'Intermediate topics',
            order: 2,
            sections: [
              {
                id: 'section-1-2-1',
                title: 'Practice',
                description: 'Practice section',
                order: 1,
                conceptIds: ['concept-6', 'concept-7'],
              },
            ],
          },
        ],
      },
      {
        id: 'chapter-2',
        title: 'Advanced',
        description: 'Advanced topics',
        order: 2,
        modules: [
          {
            id: 'module-2-1',
            title: 'Expert Level',
            description: 'Expert level',
            order: 1,
            sections: [
              {
                id: 'section-2-1-1',
                title: 'Mastery',
                description: 'Mastery section',
                order: 1,
                conceptIds: ['concept-8', 'concept-9', 'concept-10'],
              },
            ],
          },
        ],
      },
    ],
  };

  const mockContentNodes: Map<string, ContentNode> = new Map([
    [
      'concept-1',
      {
        id: 'concept-1',
        title: 'First Concept',
        description: 'The first concept',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: '# First Concept',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
      },
    ],
    [
      'concept-2',
      {
        id: 'concept-2',
        title: 'Second Concept',
        description: 'The second concept',
        contentType: 'concept',
        contentFormat: 'markdown',
        content: '# Second Concept',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
      },
    ],
  ]);

  beforeEach(() => {
    dataLoaderSpy = jasmine.createSpyObj('DataLoaderService', [
      'getPathHierarchy',
      'getClusterConcepts',
      'getClusterConnections',
    ]);
    affinitySpy = jasmine.createSpyObj('AffinityTrackingService', ['getAffinity']);

    // Default return values
    dataLoaderSpy.getPathHierarchy.and.returnValue(of(mockPath));
    dataLoaderSpy.getClusterConcepts.and.returnValue(of(mockContentNodes));
    dataLoaderSpy.getClusterConnections.and.returnValue(
      of({
        clusterId: 'default',
        totalConnections: 0,
        outgoingByCluster: new Map(),
        incomingByCluster: new Map(),
      })
    );
    affinitySpy.getAffinity.and.returnValue(0.5);

    TestBed.configureTestingModule({
      providers: [
        HierarchicalGraphService,
        { provide: DataLoaderService, useValue: dataLoaderSpy },
        { provide: AffinityTrackingService, useValue: affinitySpy },
      ],
    });

    service = TestBed.inject(HierarchicalGraphService);
  });

  afterEach(() => {
    service.reset();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('initializeFromPath', () => {
    it('should initialize cluster graph from learning path', done => {
      service.initializeFromPath('test-path').subscribe(graph => {
        expect(graph).toBeDefined();
        expect(graph.root.id).toBe('test-path');
        expect(graph.root.title).toBe('Test Learning Path');
        expect(graph.root.clusterType).toBe('path');
        expect(graph.clusters.size).toBeGreaterThan(0);
        done();
      });
    });

    it('should create chapter clusters', done => {
      service.initializeFromPath('test-path').subscribe(graph => {
        const chapter1 = graph.clusters.get('chapter-1');
        expect(chapter1).toBeDefined();
        expect(chapter1?.clusterType).toBe('chapter');
        expect(chapter1?.title).toBe('Introduction');
        expect(chapter1?.childClusterIds.length).toBe(2); // 2 modules
        done();
      });
    });

    it('should calculate total concept count', done => {
      service.initializeFromPath('test-path').subscribe(graph => {
        // 3 + 2 + 2 + 3 = 10 total concepts
        expect(graph.root.totalConceptCount).toBe(10);
        done();
      });
    });

    it('should cache graph for same path', done => {
      service.initializeFromPath('test-path').subscribe(graph1 => {
        service.initializeFromPath('test-path').subscribe(graph2 => {
          // Should be the same cached observable
          expect(dataLoaderSpy.getPathHierarchy).toHaveBeenCalledTimes(1);
          done();
        });
      });
    });

    it('should reload graph for different path', done => {
      service.initializeFromPath('test-path').subscribe(() => {
        dataLoaderSpy.getPathHierarchy.and.returnValue(
          of({ ...mockPath, id: 'other-path' })
        );
        service.initializeFromPath('other-path').subscribe(() => {
          expect(dataLoaderSpy.getPathHierarchy).toHaveBeenCalledTimes(2);
          done();
        });
      });
    });

    it('should handle path load error gracefully', done => {
      dataLoaderSpy.getPathHierarchy.and.returnValue(
        throwError(() => new Error('Load failed'))
      );

      service.initializeFromPath('test-path').subscribe(graph => {
        expect(graph.root.id).toBe('empty');
        expect(graph.clusters.size).toBe(0);
        done();
      });
    });

    it('should build concept-to-cluster mapping', done => {
      service.initializeFromPath('test-path').subscribe(() => {
        // Mapping is private, but we can test indirectly via getClusterConnections
        expect((service as any).conceptToClusterMap.size).toBeGreaterThan(0);
        done();
      });
    });
  });

  describe('expandCluster', () => {
    beforeEach(done => {
      service.initializeFromPath('test-path').subscribe(() => done());
    });

    it('should expand chapter to show modules', done => {
      service.expandCluster('chapter-1').subscribe(result => {
        expect(result.clusterId).toBe('chapter-1');
        expect(result.children.length).toBe(2); // 2 modules
        expect(result.children[0].clusterType).toBe('module');
        expect(result.edges.length).toBe(2); // 2 containment edges
        done();
      });
    });

    it('should expand module to show sections', done => {
      service.expandCluster('module-1-1').subscribe(result => {
        expect(result.clusterId).toBe('module-1-1');
        expect(result.children.length).toBe(2); // 2 sections
        expect(result.children[0].clusterType).toBe('section');
        done();
      });
    });

    it('should expand section to show concepts', done => {
      service.expandCluster('section-1-1-1').subscribe(result => {
        expect(result.clusterId).toBe('section-1-1-1');
        expect(result.children.length).toBe(3); // section-1-1-1 has 3 concept IDs
        expect(result.children[0].isCluster).toBe(false);
        done();
      });
    });

    it('should cache expanded children', done => {
      service.expandCluster('chapter-1').subscribe(() => {
        service.expandCluster('chapter-1').subscribe(() => {
          // Should use cache, not reload
          expect(service.isExpanded('chapter-1')).toBe(true);
          done();
        });
      });
    });

    it('should mark cluster as expanded', done => {
      expect(service.isExpanded('chapter-1')).toBe(false);
      service.expandCluster('chapter-1').subscribe(() => {
        expect(service.isExpanded('chapter-1')).toBe(true);
        done();
      });
    });

    it('should return empty result for nonexistent cluster', done => {
      service.expandCluster('nonexistent').subscribe(result => {
        expect(result.children.length).toBe(0);
        expect(result.edges.length).toBe(0);
        done();
      });
    });

    it('should load concepts from dataLoader for sections', done => {
      service.expandCluster('section-1-1-1').subscribe(() => {
        expect(dataLoaderSpy.getClusterConcepts).toHaveBeenCalledWith(['concept-1', 'concept-2', 'concept-3']);
        done();
      });
    });

    it('should handle concept load error', done => {
      dataLoaderSpy.getClusterConcepts.and.returnValue(
        throwError(() => new Error('Concept load failed'))
      );

      service.expandCluster('section-1-1-1').subscribe(result => {
        expect(result.children.length).toBe(0);
        done();
      });
    });

    it('should create child edges for expanded cluster', done => {
      service.expandCluster('chapter-1').subscribe(result => {
        expect(result.edges.length).toBe(2);
        expect(result.edges[0].type).toBe('CONTAINS');
        expect(result.edges[0].source).toBe('chapter-1');
        done();
      });
    });
  });

  describe('collapseCluster', () => {
    beforeEach(done => {
      service.initializeFromPath('test-path').subscribe(() => {
        service.expandCluster('chapter-1').subscribe(() => done());
      });
    });

    it('should collapse expanded cluster', () => {
      expect(service.isExpanded('chapter-1')).toBe(true);
      service.collapseCluster('chapter-1');
      expect(service.isExpanded('chapter-1')).toBe(false);
    });

    it('should collapse descendant clusters', done => {
      service.expandCluster('module-1-1').subscribe(() => {
        expect(service.isExpanded('module-1-1')).toBe(true);

        service.collapseCluster('chapter-1');

        expect(service.isExpanded('chapter-1')).toBe(false);
        expect(service.isExpanded('module-1-1')).toBe(false);
        done();
      });
    });
  });

  describe('getVisibleNodes', () => {
    beforeEach(done => {
      service.initializeFromPath('test-path').subscribe(() => done());
    });

    it('should return root and chapters by default', done => {
      service.initializeFromPath('test-path').subscribe(graph => {
        const visible = service.getVisibleNodes(graph);
        expect(visible.length).toBeGreaterThan(0);
        expect(visible.find(n => n.id === 'test-path')).toBeDefined(); // root
        expect(visible.find(n => n.id === 'chapter-1')).toBeDefined();
        done();
      });
    });

    it('should include expanded cluster children', done => {
      service.initializeFromPath('test-path').subscribe(graph => {
        service.expandCluster('chapter-1').subscribe(() => {
          const visible = service.getVisibleNodes(graph);
          expect(visible.find(n => n.id === 'module-1-1')).toBeDefined();
          expect(visible.find(n => n.id === 'module-1-2')).toBeDefined();
          done();
        });
      });
    });

    it('should not include collapsed cluster children', done => {
      service.initializeFromPath('test-path').subscribe(graph => {
        const visible = service.getVisibleNodes(graph);
        expect(visible.find(n => n.id === 'module-1-1')).toBeUndefined();
        done();
      });
    });
  });

  describe('getVisibleEdges', () => {
    beforeEach(done => {
      service.initializeFromPath('test-path').subscribe(() => done());
    });

    it('should return edges between visible nodes', done => {
      service.initializeFromPath('test-path').subscribe(graph => {
        service.expandCluster('chapter-1').subscribe(() => {
          const visible = service.getVisibleNodes(graph);
          const visibleIds = new Set(visible.map(n => n.id));
          const edges = service.getVisibleEdges(visibleIds);

          expect(edges.length).toBeGreaterThan(0);
          expect(edges[0].type).toBe('NEXT'); // Progression edges
          done();
        });
      });
    });

    it('should create progression edges for siblings', done => {
      service.initializeFromPath('test-path').subscribe(graph => {
        service.expandCluster('chapter-1').subscribe(() => {
          const visible = service.getVisibleNodes(graph);
          const visibleIds = new Set(visible.map(n => n.id));
          const edges = service.getVisibleEdges(visibleIds);

          const moduleEdge = edges.find(
            e => e.source === 'module-1-1' && e.target === 'module-1-2'
          );
          expect(moduleEdge).toBeDefined();
          expect(moduleEdge?.type).toBe('NEXT');
          done();
        });
      });
    });
  });

  describe('getClusterConnections', () => {
    beforeEach(done => {
      service.initializeFromPath('test-path').subscribe(() => done());
    });

    it('should get connections for a cluster', done => {
      const mockConnections = {
        clusterId: 'section-1-1-1',
        totalConnections: 3,
        outgoingByCluster: new Map([
          [
            'other-cluster',
            {
              sourceClusterId: 'section-1-1-1',
              targetClusterId: 'other-cluster',
              connectionCount: 3,
              relationshipTypes: ['RELATES_TO', 'DEPENDS_ON'],
            },
          ],
        ]),
        incomingByCluster: new Map(),
      };
      dataLoaderSpy.getClusterConnections.and.returnValue(of(mockConnections));

      service.getClusterConnections('section-1-1-1').subscribe(connections => {
        expect(connections.length).toBe(1);
        expect(connections[0].targetClusterId).toBe('other-cluster');
        expect(connections[0].connectionCount).toBe(3);
        done();
      });
    });

    it('should cache cluster connections', done => {
      service.getClusterConnections('section-1-1-1').subscribe(() => {
        service.getClusterConnections('section-1-1-1').subscribe(() => {
          expect(dataLoaderSpy.getClusterConnections).toHaveBeenCalledTimes(1);
          done();
        });
      });
    });

    it('should exclude self-connections', done => {
      const mockConnections = {
        clusterId: 'section-1-1-1',
        totalConnections: 2,
        outgoingByCluster: new Map([
          [
            'section-1-1-1',
            {
              sourceClusterId: 'section-1-1-1',
              targetClusterId: 'section-1-1-1',
              connectionCount: 2,
              relationshipTypes: ['RELATES_TO'],
            },
          ],
        ]),
        incomingByCluster: new Map(),
      };
      dataLoaderSpy.getClusterConnections.and.returnValue(of(mockConnections));

      service.getClusterConnections('section-1-1-1').subscribe(connections => {
        expect(connections.length).toBe(0); // Self-connection excluded
        done();
      });
    });

    it('should return empty for nonexistent cluster', done => {
      service.getClusterConnections('nonexistent').subscribe(connections => {
        expect(connections.length).toBe(0);
        done();
      });
    });
  });

  describe('reset', () => {
    it('should clear all state', done => {
      service.initializeFromPath('test-path').subscribe(() => {
        service.expandCluster('chapter-1').subscribe(() => {
          expect(service.isExpanded('chapter-1')).toBe(true);

          service.reset();

          expect(service.isExpanded('chapter-1')).toBe(false);
          expect((service as any).currentGraph$).toBeNull();
          expect((service as any).currentPathId).toBeNull();
          done();
        });
      });
    });
  });

  describe('affinity calculation', () => {
    it('should calculate cluster affinity from children', done => {
      affinitySpy.getAffinity.and.callFake((conceptId: string) => {
        const affinities: Record<string, number> = {
          'concept-1': 0.8,
          'concept-2': 0.6,
          'concept-3': 0.4,
        };
        return affinities[conceptId] ?? 0.5;
      });

      service.initializeFromPath('test-path').subscribe(graph => {
        const section = graph.clusters.get('section-1-1-1');
        expect(section?.affinityScore).toBeCloseTo((0.8 + 0.6 + 0.4) / 3, 2);
        done();
      });
    });

    it('should determine cluster state based on affinity', done => {
      affinitySpy.getAffinity.and.returnValue(0.9);

      service.initializeFromPath('test-path').subscribe(graph => {
        const section = graph.clusters.get('section-1-1-1');
        expect(section?.state).toBe('proficient');
        done();
      });
    });

    it('should mark first chapter as recommended if unseen', done => {
      affinitySpy.getAffinity.and.returnValue(0);

      service.initializeFromPath('test-path').subscribe(graph => {
        const chapter1 = graph.clusters.get('chapter-1');
        expect(chapter1?.state).toBe('recommended');
        done();
      });
    });
  });

  describe('edge cases', () => {
    it('should handle path with no chapters', done => {
      const emptyPath = { ...mockPath, chapters: [] };
      dataLoaderSpy.getPathHierarchy.and.returnValue(of(emptyPath));

      service.initializeFromPath('empty-path').subscribe(graph => {
        expect(graph.root.childClusterIds.length).toBe(0);
        expect(graph.root.totalConceptCount).toBe(0);
        done();
      });
    });

    it('should handle section with no concepts', done => {
      service.initializeFromPath('test-path').subscribe(() => {
        const emptySectionPath = {
          ...mockPath,
          chapters: [
            {
              ...mockPath.chapters![0],
              modules: [
                {
                  ...(mockPath.chapters![0].modules![0]),
                  sections: [
                    {
                      id: 'empty-section',
                      title: 'Empty',
                      description: '',
                      order: 1,
                      conceptIds: [],
                    },
                  ],
                },
              ],
            },
          ],
        };
        dataLoaderSpy.getPathHierarchy.and.returnValue(of(emptySectionPath));

        service.reset();
        service.initializeFromPath('empty-section-path').subscribe(() => {
          service.expandCluster('empty-section').subscribe(result => {
            expect(result.children.length).toBe(0);
            done();
          });
        });
      });
    });
  });
});
