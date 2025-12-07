import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { KnowledgeMapService } from './knowledge-map.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ElohimAgentService } from '@app/elohim/services/elohim-agent.service';
import { KnowledgeMapType, MasteryLevel } from '../models/knowledge-map.model';

describe('KnowledgeMapService', () => {
  let service: KnowledgeMapService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let elohimServiceSpy: jasmine.SpyObj<ElohimAgentService>;

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', ['getContent', 'getPath']);
    const elohimServiceSpyObj = jasmine.createSpyObj('ElohimAgentService', ['invoke']);

    TestBed.configureTestingModule({
      providers: [
        KnowledgeMapService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: ElohimAgentService, useValue: elohimServiceSpyObj }
      ]
    });

    service = TestBed.inject(KnowledgeMapService);
    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    elohimServiceSpy = TestBed.inject(ElohimAgentService) as jasmine.SpyObj<ElohimAgentService>;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getMapIndex', () => {
    it('should return index with demo maps', (done) => {
      service.getMapIndex().subscribe(index => {
        expect(index).toBeTruthy();
        expect(index.maps.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should include lastUpdated and totalCount', (done) => {
      service.getMapIndex().subscribe(index => {
        expect(index.lastUpdated).toBeTruthy();
        expect(index.totalCount).toBe(index.maps.length);
        done();
      });
    });
  });

  describe('getMyMaps', () => {
    it('should return maps owned by current agent', (done) => {
      service.getMyMaps().subscribe(maps => {
        expect(maps.length).toBeGreaterThan(0);
        maps.forEach(map => {
          expect(map.ownerId).toBe('demo-learner');
        });
        done();
      });
    });
  });

  describe('getMapsByType', () => {
    it('should filter by domain type', (done) => {
      service.getMapsByType('domain').subscribe(maps => {
        expect(maps.length).toBeGreaterThan(0);
        maps.forEach(map => {
          expect(map.mapType).toBe('domain');
        });
        done();
      });
    });

    it('should filter by collective type', (done) => {
      service.getMapsByType('collective').subscribe(maps => {
        expect(maps.length).toBeGreaterThan(0);
        maps.forEach(map => {
          expect(map.mapType).toBe('collective');
        });
        done();
      });
    });

    it('should return empty array for person type (no demo person maps)', (done) => {
      service.getMapsByType('person').subscribe(maps => {
        expect(maps.length).toBe(0);
        done();
      });
    });
  });

  describe('getMap', () => {
    it('should return existing map', (done) => {
      service.getMap('map-domain-elohim-protocol').subscribe(map => {
        expect(map).toBeTruthy();
        expect(map!.id).toBe('map-domain-elohim-protocol');
        done();
      });
    });

    it('should return null for non-existent map', (done) => {
      service.getMap('non-existent').subscribe(map => {
        expect(map).toBeNull();
        done();
      });
    });
  });

  describe('getDomainMap', () => {
    it('should return domain map with type narrowing', (done) => {
      service.getDomainMap('map-domain-elohim-protocol').subscribe(map => {
        expect(map).toBeTruthy();
        expect(map!.mapType).toBe('domain');
        expect(map!.contentGraphId).toBeTruthy();
        done();
      });
    });

    it('should return null for non-domain map', (done) => {
      service.getDomainMap('map-collective-learners').subscribe(map => {
        expect(map).toBeNull();
        done();
      });
    });
  });

  describe('getPersonMap', () => {
    it('should return null for non-person map', (done) => {
      service.getPersonMap('map-domain-elohim-protocol').subscribe(map => {
        expect(map).toBeNull();
        done();
      });
    });
  });

  describe('getCollectiveMap', () => {
    it('should return collective map with type narrowing', (done) => {
      service.getCollectiveMap('map-collective-learners').subscribe(map => {
        expect(map).toBeTruthy();
        expect(map!.mapType).toBe('collective');
        expect(map!.members).toBeDefined();
        done();
      });
    });

    it('should return null for non-collective map', (done) => {
      service.getCollectiveMap('map-domain-elohim-protocol').subscribe(map => {
        expect(map).toBeNull();
        done();
      });
    });
  });

  describe('createDomainMap', () => {
    it('should create a new domain map', (done) => {
      service.createDomainMap({
        title: 'Test Domain Map',
        contentGraphId: 'test-graph'
      }).subscribe(map => {
        expect(map.id).toContain('map-domain-');
        expect(map.mapType).toBe('domain');
        expect(map.title).toBe('Test Domain Map');
        expect(map.contentGraphId).toBe('test-graph');
        expect(map.visibility).toBe('private');
        done();
      });
    });

    it('should respect visibility parameter', (done) => {
      service.createDomainMap({
        title: 'Public Map',
        contentGraphId: 'test-graph',
        visibility: 'public'
      }).subscribe(map => {
        expect(map.visibility).toBe('public');
        done();
      });
    });
  });

  describe('createPersonMap', () => {
    it('should create a new person map', (done) => {
      service.createPersonMap({
        title: 'Test Person Map',
        subjectAgentId: 'subject-123',
        subjectName: 'Test Person',
        relationshipType: 'colleague'
      }).subscribe(map => {
        expect(map.id).toContain('map-person-');
        expect(map.mapType).toBe('person');
        expect(map.title).toBe('Test Person Map');
        expect(map.relationshipType).toBe('colleague');
        expect(map.subject.subjectId).toBe('subject-123');
        done();
      });
    });
  });

  describe('createCollectiveMap', () => {
    it('should create a new collective map', (done) => {
      service.createCollectiveMap({
        title: 'Test Collective Map',
        organizationId: 'org-123',
        organizationName: 'Test Org'
      }).subscribe(map => {
        expect(map.id).toContain('map-collective-');
        expect(map.mapType).toBe('collective');
        expect(map.title).toBe('Test Collective Map');
        expect(map.members.length).toBe(1);
        expect(map.members[0].role).toBe('steward');
        done();
      });
    });

    it('should set default governance', (done) => {
      service.createCollectiveMap({
        title: 'Test Map',
        organizationId: 'org-123',
        organizationName: 'Test Org'
      }).subscribe(map => {
        expect(map.governance.approvalModel).toBe('steward-only');
        expect(map.governance.membershipControl).toBe('steward-only');
        done();
      });
    });

    it('should respect custom governance', (done) => {
      service.createCollectiveMap({
        title: 'Test Map',
        organizationId: 'org-123',
        organizationName: 'Test Org',
        governance: {
          approvalModel: 'consensus',
          membershipControl: 'member-invite'
        }
      }).subscribe(map => {
        expect(map.governance.approvalModel).toBe('consensus');
        expect(map.governance.membershipControl).toBe('member-invite');
        done();
      });
    });
  });

  describe('addNode', () => {
    it('should add node to map', (done) => {
      const nodeData = {
        category: 'test',
        title: 'Test Node',
        content: 'Test content',
        affinity: 0.5,
        relatedNodeIds: [],
        tags: ['test']
      };

      service.addNode('map-domain-elohim-protocol', nodeData).subscribe(node => {
        expect(node.id).toBeTruthy();
        expect(node.title).toBe('Test Node');
        done();
      });
    });

    it('should return error for non-existent map', (done) => {
      service.addNode('non-existent', {
        category: 'test',
        title: 'Test',
        content: '',
        affinity: 0,
        relatedNodeIds: [],
        tags: []
      }).subscribe({
        error: err => {
          expect(err.code).toBe('NOT_FOUND');
          done();
        }
      });
    });
  });

  describe('updateNode', () => {
    it('should update existing node', (done) => {
      // First get the map to find an existing node
      service.getDomainMap('map-domain-elohim-protocol').subscribe(map => {
        const existingNodeId = map!.nodes[0].id;

        service.updateNode('map-domain-elohim-protocol', existingNodeId, {
          title: 'Updated Title'
        }).subscribe(node => {
          expect(node.title).toBe('Updated Title');
          done();
        });
      });
    });

    it('should return error for non-existent node', (done) => {
      service.updateNode('map-domain-elohim-protocol', 'non-existent-node', {
        title: 'Updated'
      }).subscribe({
        error: err => {
          expect(err.code).toBe('NOT_FOUND');
          done();
        }
      });
    });
  });

  describe('removeNode', () => {
    it('should remove node from map', (done) => {
      // First add a node
      service.addNode('map-domain-elohim-protocol', {
        category: 'test',
        title: 'To Remove',
        content: '',
        affinity: 0,
        relatedNodeIds: [],
        tags: []
      }).subscribe(addedNode => {
        // Then remove it
        service.removeNode('map-domain-elohim-protocol', addedNode.id).subscribe(() => {
          // Verify it's gone
          service.getDomainMap('map-domain-elohim-protocol').subscribe(map => {
            const found = map!.nodes.find(n => n.id === addedNode.id);
            expect(found).toBeFalsy();
            done();
          });
        });
      });
    });
  });

  describe('updateMastery', () => {
    it('should update mastery level for content node', (done) => {
      const level: MasteryLevel = 'apply';
      service.updateMastery('map-domain-elohim-protocol', 'test-content', level).subscribe(() => {
        service.getDomainMap('map-domain-elohim-protocol').subscribe(map => {
          expect(map!.masteryLevels.get('test-content')).toBe(level);
          done();
        });
      });
    });

    it('should return error for non-domain map', (done) => {
      const level: MasteryLevel = 'apply';
      service.updateMastery('map-collective-learners', 'test-content', level).subscribe({
        error: err => {
          expect(err.code).toBe('NOT_FOUND');
          done();
        }
      });
    });
  });

  describe('requestConsent', () => {
    it('should handle consent request for person map', (done) => {
      // First create a person map
      service.createPersonMap({
        title: 'Test Person Map',
        subjectAgentId: 'subject-123',
        subjectName: 'Test Person',
        relationshipType: 'colleague'
      }).subscribe(map => {
        service.requestConsent(map.id, 'shared-only').subscribe(() => {
          // Request should complete without error
          expect(true).toBeTrue();
          done();
        });
      });
    });

    it('should return error for non-person map', (done) => {
      service.requestConsent('map-domain-elohim-protocol', 'shared-only').subscribe({
        error: err => {
          expect(err.code).toBe('NOT_FOUND');
          done();
        }
      });
    });
  });

  describe('grantConsent', () => {
    it('should return error when not the subject', (done) => {
      // Create a person map about someone else
      service.createPersonMap({
        title: 'About Someone',
        subjectAgentId: 'other-person',
        subjectName: 'Other Person',
        relationshipType: 'colleague'
      }).subscribe(map => {
        service.grantConsent(map.id, {
          granted: true,
          scope: 'shared-only',
          grantedAt: new Date().toISOString(),
          transparencyLevel: 'categories-only'
        }).subscribe({
          error: err => {
            expect(err.code).toBe('UNAUTHORIZED');
            done();
          }
        });
      });
    });
  });

  describe('requestElohimSynthesis', () => {
    it('should return suggestions for valid map', (done) => {
      elohimServiceSpy.invoke.and.returnValue(of({
        requestId: 'test-request',
        elohimId: 'test-elohim',
        status: 'fulfilled',
        payload: {
          type: 'knowledge-map-update'
        },
        respondedAt: new Date().toISOString(),
        constitutionalReasoning: {
          principlesConsidered: [],
          decision: 'approved',
          rationale: 'test'
        }
      } as any));

      service.requestElohimSynthesis('map-domain-elohim-protocol').subscribe(suggestions => {
        expect(suggestions.length).toBeGreaterThan(0);
        expect(suggestions[0].operation).toBe('add-node');
        done();
      });
    });

    it('should return empty array for rejected response', (done) => {
      elohimServiceSpy.invoke.and.returnValue(of({
        requestId: 'test-request',
        elohimId: 'test-elohim',
        status: 'rejected',
        reason: 'Not available',
        respondedAt: new Date().toISOString(),
        constitutionalReasoning: {
          principlesConsidered: [],
          decision: 'rejected',
          rationale: 'test'
        }
      } as any));

      service.requestElohimSynthesis('map-domain-elohim-protocol').subscribe(suggestions => {
        expect(suggestions.length).toBe(0);
        done();
      });
    });

    it('should return error for non-existent map', (done) => {
      service.requestElohimSynthesis('non-existent').subscribe({
        error: err => {
          expect(err.code).toBe('NOT_FOUND');
          done();
        }
      });
    });
  });

  describe('setCurrentAgent', () => {
    it('should update current agent ID', (done) => {
      service.setCurrentAgent('new-agent');
      service.getMyMaps().subscribe(maps => {
        // Should no longer see demo-learner's maps
        const demoLearnerMaps = maps.filter(m => m.ownerId === 'demo-learner');
        expect(demoLearnerMaps.length).toBe(0);
        done();
      });
    });
  });

  describe('visibility and access control', () => {
    it('should allow owner to view private map', (done) => {
      service.getMap('map-domain-elohim-protocol').subscribe(map => {
        expect(map).toBeTruthy();
        done();
      });
    });

    it('should allow members to view collective map', (done) => {
      service.getMap('map-collective-learners').subscribe(map => {
        expect(map).toBeTruthy();
        done();
      });
    });
  });

  describe('affinity calculation', () => {
    it('should recalculate affinity when adding node', (done) => {
      service.getDomainMap('map-domain-elohim-protocol').subscribe(initialMap => {
        const initialAffinity = initialMap!.overallAffinity;

        service.addNode('map-domain-elohim-protocol', {
          category: 'test',
          title: 'High Affinity',
          content: '',
          affinity: 1.0, // Maximum affinity
          relatedNodeIds: [],
          tags: []
        }).subscribe(() => {
          service.getDomainMap('map-domain-elohim-protocol').subscribe(updatedMap => {
            // Affinity should change
            expect(updatedMap!.overallAffinity).not.toBe(initialAffinity);
            done();
          });
        });
      });
    });
  });
});

// Import tap operator for getMyMaps test
import { tap } from 'rxjs/operators';
