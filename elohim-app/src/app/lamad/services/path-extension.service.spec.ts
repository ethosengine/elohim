import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { PathExtensionService } from './path-extension.service';
import { PathService } from './path.service';
import { LearningPath, PathStep } from '../models/learning-path.model';
import { PathExtension } from '../models/path-extension.model';

describe('PathExtensionService', () => {
  let service: PathExtensionService;
  let pathServiceSpy: jasmine.SpyObj<PathService>;

  const mockPath: LearningPath = {
    id: 'test-path',
    version: '1.0.0',
    title: 'Test Path',
    description: 'A test path',
    purpose: 'Testing',
    createdBy: 'test-user',
    contributors: [],
    createdAt: '2025-01-01T00:00:00.000Z',
    updatedAt: '2025-01-01T00:00:00.000Z',
    difficulty: 'beginner',
    estimatedDuration: '1 hour',
    tags: ['test'],
    visibility: 'public',
    steps: [
      { resourceId: 'resource-1', stepTitle: 'Step 1', stepNarrative: 'First step' },
      { resourceId: 'resource-2', stepTitle: 'Step 2', stepNarrative: 'Second step' },
      { resourceId: 'resource-3', stepTitle: 'Step 3', stepNarrative: 'Third step' },
    ] as PathStep[],
  };

  beforeEach(() => {
    const pathServiceSpyObj = jasmine.createSpyObj('PathService', ['getPath']);

    TestBed.configureTestingModule({
      providers: [PathExtensionService, { provide: PathService, useValue: pathServiceSpyObj }],
    });

    pathServiceSpy = TestBed.inject(PathService) as jasmine.SpyObj<PathService>;
    pathServiceSpy.getPath.and.returnValue(of(mockPath));

    service = TestBed.inject(PathExtensionService);
    service.setCurrentAgent('demo-learner');
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // Extension Discovery
  // =========================================================================

  describe('getExtensionIndex', () => {
    it('should return extension index', done => {
      service.getExtensionIndex().subscribe(index => {
        expect(index).toBeDefined();
        expect(index.totalCount).toBeGreaterThanOrEqual(0);
        expect(index.extensions).toBeDefined();
        done();
      });
    });

    it('should include demo extension', done => {
      service.getExtensionIndex().subscribe(index => {
        const demoExt = index.extensions.find(e => e.id === 'ext-demo-elohim-path');
        expect(demoExt).toBeDefined();
        done();
      });
    });
  });

  describe('getMyExtensions', () => {
    it('should return only extensions by current agent', done => {
      service.getMyExtensions().subscribe(extensions => {
        for (const ext of extensions) {
          expect(ext.extendedBy).toBe('demo-learner');
        }
        done();
      });
    });
  });

  describe('getExtensionsForPath', () => {
    it('should filter extensions by base path', done => {
      service.getExtensionsForPath('elohim-protocol').subscribe(extensions => {
        for (const ext of extensions) {
          expect(ext.basePathId).toBe('elohim-protocol');
        }
        done();
      });
    });
  });

  describe('getExtension', () => {
    it('should return null for non-existent extension', done => {
      service.getExtension('non-existent').subscribe(ext => {
        expect(ext).toBeNull();
        done();
      });
    });

    it('should return extension when authorized', done => {
      service.getExtension('ext-demo-elohim-path').subscribe(ext => {
        expect(ext).not.toBeNull();
        expect(ext?.id).toBe('ext-demo-elohim-path');
        done();
      });
    });

    it('should error for unauthorized access to private extension', done => {
      // Create private extension as different user
      service.setCurrentAgent('other-user');
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Private Extension',
          visibility: 'private',
        })
        .subscribe(ext => {
          // Try to access as original user
          service.setCurrentAgent('demo-learner');

          service.getExtension(ext.id).subscribe({
            error: err => {
              expect(err.code).toBe('UNAUTHORIZED');
              done();
            },
          });
        });
    });
  });

  // =========================================================================
  // Extension Creation
  // =========================================================================

  describe('createExtension', () => {
    it('should create new extension', done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'My Extension',
        })
        .subscribe(ext => {
          expect(ext.id).toBeDefined();
          expect(ext.basePathId).toBe('test-path');
          expect(ext.title).toBe('My Extension');
          expect(ext.extendedBy).toBe('demo-learner');
          expect(ext.visibility).toBe('private');
          done();
        });
    });

    it('should set version from base path', done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'My Extension',
        })
        .subscribe(ext => {
          expect(ext.basePathVersion).toBe('1.0.0');
          done();
        });
    });

    it('should error when base path not found', done => {
      pathServiceSpy.getPath.and.returnValue(of(null as any));

      service
        .createExtension({
          basePathId: 'non-existent',
          title: 'My Extension',
        })
        .subscribe({
          error: err => {
            expect(err.code).toBe('NOT_FOUND');
            done();
          },
        });
    });

    it('should respect visibility parameter', done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Public Extension',
          visibility: 'public',
        })
        .subscribe(ext => {
          expect(ext.visibility).toBe('public');
          done();
        });
    });
  });

  describe('forkExtension', () => {
    it('should create fork of existing extension', done => {
      service
        .forkExtension('ext-demo-elohim-path', {
          title: 'My Forked Extension',
        })
        .subscribe(forked => {
          expect(forked.id).not.toBe('ext-demo-elohim-path');
          expect(forked.title).toBe('My Forked Extension');
          expect(forked.forkedFrom).toBe('ext-demo-elohim-path');
          expect(forked.visibility).toBe('private');
          done();
        });
    });

    it('should copy annotations from original', done => {
      service.forkExtension('ext-demo-elohim-path').subscribe(forked => {
        expect(forked.annotations.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should error for non-existent extension', done => {
      service.forkExtension('non-existent').subscribe({
        error: err => {
          expect(err.code).toBe('NOT_FOUND');
          done();
        },
      });
    });
  });

  // =========================================================================
  // Extension Modifications
  // =========================================================================

  describe('addInsertion', () => {
    let testExtensionId: string;

    beforeEach(done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
        .subscribe(ext => {
          testExtensionId = ext.id;
          done();
        });
    });

    it('should add insertion to extension', done => {
      const newStep: PathStep = {
        order: 0,
        resourceId: 'new-resource',
        stepTitle: 'New Step',
        stepNarrative: 'Added step',
        learningObjectives: ['Test objective'],
        optional: false,
        completionCriteria: ['View content'],
      };

      service
        .addInsertion(testExtensionId, 0, [newStep], 'Adding extra content')
        .subscribe(insertion => {
          expect(insertion.id).toBeDefined();
          expect(insertion.afterStepIndex).toBe(0);
          expect(insertion.steps.length).toBe(1);
          expect(insertion.rationale).toBe('Adding extra content');
          done();
        });
    });

    it('should error for non-existent extension', done => {
      service.addInsertion('non-existent', 0, [], 'test').subscribe({
        error: err => {
          expect(err.code).toBe('NOT_FOUND');
          done();
        },
      });
    });

    it('should error for unauthorized edit', done => {
      service.setCurrentAgent('other-user');

      service.addInsertion(testExtensionId, 0, [], 'test').subscribe({
        error: err => {
          expect(err.code).toBe('UNAUTHORIZED');
          done();
        },
      });
    });
  });

  describe('addAnnotation', () => {
    let testExtensionId: string;

    beforeEach(done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
        .subscribe(ext => {
          testExtensionId = ext.id;
          done();
        });
    });

    it('should add annotation to extension', done => {
      service.addAnnotation(testExtensionId, 0, 'insight', 'My insight').subscribe(annotation => {
        expect(annotation.id).toBeDefined();
        expect(annotation.stepIndex).toBe(0);
        expect(annotation.type).toBe('insight');
        expect(annotation.content).toBe('My insight');
        done();
      });
    });

    it('should include additional resources', done => {
      service
        .addAnnotation(testExtensionId, 1, 'note', 'Extra reading', {
          additionalResources: [
            { title: 'Extra Resource', resourceId: 'extra-1', description: 'More depth' },
          ],
        })
        .subscribe(annotation => {
          expect(annotation.additionalResources).toBeDefined();
          expect(annotation.additionalResources?.length).toBe(1);
          done();
        });
    });
  });

  describe('addReorder', () => {
    let testExtensionId: string;

    beforeEach(done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
        .subscribe(ext => {
          testExtensionId = ext.id;
          done();
        });
    });

    it('should add reorder to extension', done => {
      service.addReorder(testExtensionId, 0, 2, 'Better flow').subscribe(reorder => {
        expect(reorder.id).toBeDefined();
        expect(reorder.fromIndex).toBe(0);
        expect(reorder.toIndex).toBe(2);
        expect(reorder.rationale).toBe('Better flow');
        done();
      });
    });
  });

  describe('addExclusion', () => {
    let testExtensionId: string;

    beforeEach(done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
        .subscribe(ext => {
          testExtensionId = ext.id;
          done();
        });
    });

    it('should add exclusion to extension', done => {
      service
        .addExclusion(testExtensionId, 1, 'already-mastered', 'I know this already')
        .subscribe(exclusion => {
          expect(exclusion.id).toBeDefined();
          expect(exclusion.stepIndex).toBe(1);
          expect(exclusion.reason).toBe('already-mastered');
          expect(exclusion.notes).toBe('I know this already');
          done();
        });
    });
  });

  describe('removeModification', () => {
    let testExtensionId: string;
    let insertionId: string;

    beforeEach(done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
        .subscribe(ext => {
          testExtensionId = ext.id;

          service
            .addInsertion(testExtensionId, 0, [
              {
                order: 0,
                resourceId: 'test',
                stepTitle: 'Test',
                stepNarrative: 'Test',
                learningObjectives: ['Test'],
                optional: false,
                completionCriteria: ['View'],
              },
            ])
            .subscribe(insertion => {
              insertionId = insertion.id;
              done();
            });
        });
    });

    it('should remove modification from extension', done => {
      service.removeModification(testExtensionId, insertionId).subscribe(() => {
        service.getExtension(testExtensionId).subscribe(ext => {
          expect(ext?.insertions.find(i => i.id === insertionId)).toBeUndefined();
          done();
        });
      });
    });
  });

  // =========================================================================
  // Apply Extension
  // =========================================================================

  describe('applyExtension', () => {
    let testExtensionId: string;

    beforeEach(done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
        .subscribe(ext => {
          testExtensionId = ext.id;
          done();
        });
    });

    it('should apply extension to get effective steps', done => {
      service.applyExtension(testExtensionId).subscribe(result => {
        expect(result.effectiveSteps).toBeDefined();
        expect(result.effectiveSteps.length).toBe(mockPath.steps.length);
        expect(result.warnings).toBeDefined();
        done();
      });
    });

    it('should include inserted steps', done => {
      service
        .addInsertion(testExtensionId, 0, [
          {
            order: 0,
            resourceId: 'inserted',
            stepTitle: 'Inserted Step',
            stepNarrative: 'New step',
            learningObjectives: ['Test'],
            optional: false,
            completionCriteria: ['View'],
          },
        ])
        .subscribe(() => {
          service.applyExtension(testExtensionId).subscribe(result => {
            expect(result.effectiveSteps.length).toBe(mockPath.steps.length + 1);
            done();
          });
        });
    });

    it('should exclude excluded steps', done => {
      service.addExclusion(testExtensionId, 1, 'not-relevant').subscribe(() => {
        service.applyExtension(testExtensionId).subscribe(result => {
          expect(result.effectiveSteps.length).toBe(mockPath.steps.length - 1);
          done();
        });
      });
    });

    it('should warn on version mismatch', done => {
      const modifiedPath = { ...mockPath, version: '2.0.0' };
      pathServiceSpy.getPath.and.returnValue(of(modifiedPath));

      service.applyExtension(testExtensionId).subscribe(result => {
        const versionWarning = result.warnings.find(w => w.type === 'version-mismatch');
        expect(versionWarning).toBeDefined();
        done();
      });
    });
  });

  // =========================================================================
  // Upstream Proposals
  // =========================================================================

  describe('submitUpstreamProposal', () => {
    let testExtensionId: string;

    beforeEach(done => {
      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
        .subscribe(ext => {
          testExtensionId = ext.id;
          done();
        });
    });

    it('should submit upstream proposal', done => {
      service.submitUpstreamProposal(testExtensionId).subscribe(proposal => {
        expect(proposal.status).toBe('submitted');
        expect(proposal.submittedAt).toBeDefined();
        done();
      });
    });

    it('should error for non-existent extension', done => {
      service.submitUpstreamProposal('non-existent').subscribe({
        error: err => {
          expect(err.code).toBe('NOT_FOUND');
          done();
        },
      });
    });
  });

  // =========================================================================
  // Collaborative Paths
  // =========================================================================

  describe('enableCollaboration', () => {
    beforeEach(() => {
      // Mock path ownership
      const ownedPath = { ...mockPath, createdBy: 'demo-learner' };
      pathServiceSpy.getPath.and.returnValue(of(ownedPath));
    });

    it('should enable collaboration on owned path', done => {
      service.enableCollaboration('test-path', 'open').subscribe(collab => {
        expect(collab.pathId).toBe('test-path');
        expect(collab.collaborationType).toBe('open');
        expect(collab.roles.get('demo-learner')).toBe('owner');
        done();
      });
    });

    it('should error when not path owner', done => {
      const otherPath = { ...mockPath, createdBy: 'other-user' };
      pathServiceSpy.getPath.and.returnValue(of(otherPath));

      service.enableCollaboration('test-path', 'open').subscribe({
        error: err => {
          expect(err.code).toBe('UNAUTHORIZED');
          done();
        },
      });
    });
  });

  describe('addCollaborator', () => {
    beforeEach(done => {
      const ownedPath = { ...mockPath, createdBy: 'demo-learner' };
      pathServiceSpy.getPath.and.returnValue(of(ownedPath));

      service.enableCollaboration('test-path', 'open').subscribe(() => done());
    });

    it('should add collaborator to path', done => {
      service.addCollaborator('test-path', 'new-collaborator', 'editor').subscribe(() => {
        service.getCollaborativePath('test-path').subscribe(collab => {
          expect(collab?.roles.get('new-collaborator')).toBe('editor');
          done();
        });
      });
    });

    it('should error for non-collaborative path', done => {
      service.addCollaborator('non-collab-path', 'user', 'viewer').subscribe({
        error: err => {
          expect(err.code).toBe('NOT_FOUND');
          done();
        },
      });
    });
  });

  describe('submitProposal', () => {
    beforeEach(done => {
      const ownedPath = { ...mockPath, createdBy: 'demo-learner' };
      pathServiceSpy.getPath.and.returnValue(of(ownedPath));

      service.enableCollaboration('test-path', 'open').subscribe(() => done());
    });

    it('should submit proposal to collaborative path', done => {
      service
        .submitProposal('test-path', 'add-step', { stepIndex: 0 }, 'Need more content')
        .subscribe(proposal => {
          expect(proposal.id).toBeDefined();
          expect(proposal.status).toBe('pending');
          expect(proposal.changeType).toBe('add-step');
          done();
        });
    });
  });

  // =========================================================================
  // Agent Management
  // =========================================================================

  describe('setCurrentAgent', () => {
    it('should change current agent', done => {
      service.setCurrentAgent('new-agent');

      service
        .createExtension({
          basePathId: 'test-path',
          title: 'Test',
        })
        .subscribe(ext => {
          expect(ext.extendedBy).toBe('new-agent');
          done();
        });
    });
  });
});
