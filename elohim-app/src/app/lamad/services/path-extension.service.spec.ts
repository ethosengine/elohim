import { TestBed } from '@angular/core/testing';
import { firstValueFrom } from 'rxjs';
import { of, throwError } from 'rxjs';
import { PathExtensionService } from './path-extension.service';
import { PathService } from './path.service';
import { LearningPath, PathStep } from '../models/learning-path.model';
import { PathExtension } from '../models/path-extension.model';
import { vi, Mock } from 'vitest';

describe('PathExtensionService', () => {
  let service: PathExtensionService;
  let pathServiceSpy: any;

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
    const pathServiceSpyObj = {
      getPath: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [PathExtensionService, { provide: PathService, useValue: pathServiceSpyObj }],
    });

    pathServiceSpy = TestBed.inject(PathService) as { [K in keyof PathService]?: Mock };
    pathServiceSpy.getPath.mockReturnValue(of(mockPath));

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
    it('should return extension index', async () => {
      const index = await firstValueFrom(service.getExtensionIndex());
      expect(index).toBeDefined();
      expect(index.totalCount).toBeGreaterThanOrEqual(0);
      expect(index.extensions).toBeDefined();
    });

    it('should include demo extension', async () => {
      const index = await firstValueFrom(service.getExtensionIndex());
      const demoExt = index.extensions.find(e => e.id === 'ext-demo-elohim-path');
      expect(demoExt).toBeDefined();
    });
  });

  describe('getMyExtensions', () => {
    it('should return only extensions by current agent', async () => {
      const extensions = await firstValueFrom(service.getMyExtensions());
      for (const ext of extensions) {
        expect(ext.extendedBy).toBe('demo-learner');
      }
    });
  });

  describe('getExtensionsForPath', () => {
    it('should filter extensions by base path', async () => {
      const extensions = await firstValueFrom(service.getExtensionsForPath('elohim-protocol'));
      for (const ext of extensions) {
        expect(ext.basePathId).toBe('elohim-protocol');
      }
    });
  });

  describe('getExtension', () => {
    it('should return null for non-existent extension', async () => {
      const ext = await firstValueFrom(service.getExtension('non-existent'));
      expect(ext).toBeNull();
    });

    it('should return extension when authorized', async () => {
      const ext = await firstValueFrom(service.getExtension('ext-demo-elohim-path'));
      expect(ext).not.toBeNull();
      expect(ext?.id).toBe('ext-demo-elohim-path');
    });

    it('should error for unauthorized access to private extension', async () => {
      // Create private extension as different user
      service.setCurrentAgent('other-user');
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Private Extension',
          visibility: 'private',
        })
      );

      // Try to access as original user
      service.setCurrentAgent('demo-learner');

      await expect(firstValueFrom(service.getExtension(ext.id))).rejects.toMatchObject({
        code: 'UNAUTHORIZED',
      });
    });
  });

  // =========================================================================
  // Extension Creation
  // =========================================================================

  describe('createExtension', () => {
    it('should create new extension', async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'My Extension',
        })
      );
      expect(ext.id).toBeDefined();
      expect(ext.basePathId).toBe('test-path');
      expect(ext.title).toBe('My Extension');
      expect(ext.extendedBy).toBe('demo-learner');
      expect(ext.visibility).toBe('private');
    });

    it('should set version from base path', async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'My Extension',
        })
      );
      expect(ext.basePathVersion).toBe('1.0.0');
    });

    it('should error when base path not found', async () => {
      pathServiceSpy.getPath.mockReturnValue(of(null as any));

      await expect(
        firstValueFrom(
          service.createExtension({
            basePathId: 'non-existent',
            title: 'My Extension',
          })
        )
      ).rejects.toMatchObject({ code: 'NOT_FOUND' });
    });

    it('should respect visibility parameter', async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Public Extension',
          visibility: 'public',
        })
      );
      expect(ext.visibility).toBe('public');
    });
  });

  describe('forkExtension', () => {
    it('should create fork of existing extension', async () => {
      const forked = await firstValueFrom(
        service.forkExtension('ext-demo-elohim-path', {
          title: 'My Forked Extension',
        })
      );
      expect(forked.id).not.toBe('ext-demo-elohim-path');
      expect(forked.title).toBe('My Forked Extension');
      expect(forked.forkedFrom).toBe('ext-demo-elohim-path');
      expect(forked.visibility).toBe('private');
    });

    it('should copy annotations from original', async () => {
      const forked = await firstValueFrom(service.forkExtension('ext-demo-elohim-path'));
      expect(forked.annotations.length).toBeGreaterThan(0);
    });

    it('should error for non-existent extension', async () => {
      await expect(
        firstValueFrom(service.forkExtension('non-existent'))
      ).rejects.toMatchObject({ code: 'NOT_FOUND' });
    });
  });

  // =========================================================================
  // Extension Modifications
  // =========================================================================

  describe('addInsertion', () => {
    let testExtensionId: string;

    beforeEach(async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
      );
      testExtensionId = ext.id;
    });

    it('should add insertion to extension', async () => {
      const newStep: PathStep = {
        order: 0,
        resourceId: 'new-resource',
        stepTitle: 'New Step',
        stepNarrative: 'Added step',
        learningObjectives: ['Test objective'],
        optional: false,
        completionCriteria: ['View content'],
      };

      const insertion = await firstValueFrom(
        service.addInsertion(testExtensionId, 0, [newStep], 'Adding extra content')
      );
      expect(insertion.id).toBeDefined();
      expect(insertion.afterStepIndex).toBe(0);
      expect(insertion.steps.length).toBe(1);
      expect(insertion.rationale).toBe('Adding extra content');
    });

    it('should error for non-existent extension', async () => {
      await expect(
        firstValueFrom(service.addInsertion('non-existent', 0, [], 'test'))
      ).rejects.toMatchObject({ code: 'NOT_FOUND' });
    });

    it('should error for unauthorized edit', async () => {
      service.setCurrentAgent('other-user');

      await expect(
        firstValueFrom(service.addInsertion(testExtensionId, 0, [], 'test'))
      ).rejects.toMatchObject({ code: 'UNAUTHORIZED' });
    });
  });

  describe('addAnnotation', () => {
    let testExtensionId: string;

    beforeEach(async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
      );
      testExtensionId = ext.id;
    });

    it('should add annotation to extension', async () => {
      const annotation = await firstValueFrom(
        service.addAnnotation(testExtensionId, 0, 'insight', 'My insight')
      );
      expect(annotation.id).toBeDefined();
      expect(annotation.stepIndex).toBe(0);
      expect(annotation.type).toBe('insight');
      expect(annotation.content).toBe('My insight');
    });

    it('should include additional resources', async () => {
      const annotation = await firstValueFrom(
        service.addAnnotation(testExtensionId, 1, 'note', 'Extra reading', {
          additionalResources: [
            { title: 'Extra Resource', resourceId: 'extra-1', description: 'More depth' },
          ],
        })
      );
      expect(annotation.additionalResources).toBeDefined();
      expect(annotation.additionalResources?.length).toBe(1);
    });
  });

  describe('addReorder', () => {
    let testExtensionId: string;

    beforeEach(async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
      );
      testExtensionId = ext.id;
    });

    it('should add reorder to extension', async () => {
      const reorder = await firstValueFrom(
        service.addReorder(testExtensionId, 0, 2, 'Better flow')
      );
      expect(reorder.id).toBeDefined();
      expect(reorder.fromIndex).toBe(0);
      expect(reorder.toIndex).toBe(2);
      expect(reorder.rationale).toBe('Better flow');
    });
  });

  describe('addExclusion', () => {
    let testExtensionId: string;

    beforeEach(async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
      );
      testExtensionId = ext.id;
    });

    it('should add exclusion to extension', async () => {
      const exclusion = await firstValueFrom(
        service.addExclusion(testExtensionId, 1, 'already-mastered', 'I know this already')
      );
      expect(exclusion.id).toBeDefined();
      expect(exclusion.stepIndex).toBe(1);
      expect(exclusion.reason).toBe('already-mastered');
      expect(exclusion.notes).toBe('I know this already');
    });
  });

  describe('removeModification', () => {
    let testExtensionId: string;
    let insertionId: string;

    beforeEach(async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
      );
      testExtensionId = ext.id;

      const insertion = await firstValueFrom(
        service.addInsertion(testExtensionId, 0, [
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
      );
      insertionId = insertion.id;
    });

    it('should remove modification from extension', async () => {
      await firstValueFrom(service.removeModification(testExtensionId, insertionId));
      const ext = await firstValueFrom(service.getExtension(testExtensionId));
      expect(ext?.insertions.find(i => i.id === insertionId)).toBeUndefined();
    });
  });

  // =========================================================================
  // Apply Extension
  // =========================================================================

  describe('applyExtension', () => {
    let testExtensionId: string;

    beforeEach(async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
      );
      testExtensionId = ext.id;
    });

    it('should apply extension to get effective steps', async () => {
      const result = await firstValueFrom(service.applyExtension(testExtensionId));
      expect(result.effectiveSteps).toBeDefined();
      expect(result.effectiveSteps.length).toBe(mockPath.steps.length);
      expect(result.warnings).toBeDefined();
    });

    it('should include inserted steps', async () => {
      await firstValueFrom(
        service.addInsertion(testExtensionId, 0, [
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
      );
      const result = await firstValueFrom(service.applyExtension(testExtensionId));
      expect(result.effectiveSteps.length).toBe(mockPath.steps.length + 1);
    });

    it('should exclude excluded steps', async () => {
      await firstValueFrom(service.addExclusion(testExtensionId, 1, 'not-relevant'));
      const result = await firstValueFrom(service.applyExtension(testExtensionId));
      expect(result.effectiveSteps.length).toBe(mockPath.steps.length - 1);
    });

    it('should warn on version mismatch', async () => {
      const modifiedPath = { ...mockPath, version: '2.0.0' };
      pathServiceSpy.getPath.mockReturnValue(of(modifiedPath));

      const result = await firstValueFrom(service.applyExtension(testExtensionId));
      const versionWarning = result.warnings.find(w => w.type === 'version-mismatch');
      expect(versionWarning).toBeDefined();
    });
  });

  // =========================================================================
  // Upstream Proposals
  // =========================================================================

  describe('submitUpstreamProposal', () => {
    let testExtensionId: string;

    beforeEach(async () => {
      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Test Extension',
        })
      );
      testExtensionId = ext.id;
    });

    it('should submit upstream proposal', async () => {
      const proposal = await firstValueFrom(service.submitUpstreamProposal(testExtensionId));
      expect(proposal.status).toBe('submitted');
      expect(proposal.submittedAt).toBeDefined();
    });

    it('should error for non-existent extension', async () => {
      await expect(
        firstValueFrom(service.submitUpstreamProposal('non-existent'))
      ).rejects.toMatchObject({ code: 'NOT_FOUND' });
    });
  });

  // =========================================================================
  // Collaborative Paths
  // =========================================================================

  describe('enableCollaboration', () => {
    beforeEach(() => {
      // Mock path ownership
      const ownedPath = { ...mockPath, createdBy: 'demo-learner' };
      pathServiceSpy.getPath.mockReturnValue(of(ownedPath));
    });

    it('should enable collaboration on owned path', async () => {
      const collab = await firstValueFrom(service.enableCollaboration('test-path', 'open'));
      expect(collab.pathId).toBe('test-path');
      expect(collab.collaborationType).toBe('open');
      expect(collab.roles.get('demo-learner')).toBe('owner');
    });

    it('should error when not path owner', async () => {
      const otherPath = { ...mockPath, createdBy: 'other-user' };
      pathServiceSpy.getPath.mockReturnValue(of(otherPath));

      await expect(
        firstValueFrom(service.enableCollaboration('test-path', 'open'))
      ).rejects.toMatchObject({ code: 'UNAUTHORIZED' });
    });
  });

  describe('addCollaborator', () => {
    beforeEach(async () => {
      const ownedPath = { ...mockPath, createdBy: 'demo-learner' };
      pathServiceSpy.getPath.mockReturnValue(of(ownedPath));

      await firstValueFrom(service.enableCollaboration('test-path', 'open'));
    });

    it('should add collaborator to path', async () => {
      await firstValueFrom(service.addCollaborator('test-path', 'new-collaborator', 'editor'));
      const collab = await firstValueFrom(service.getCollaborativePath('test-path'));
      expect(collab?.roles.get('new-collaborator')).toBe('editor');
    });

    it('should error for non-collaborative path', async () => {
      await expect(
        firstValueFrom(service.addCollaborator('non-collab-path', 'user', 'viewer'))
      ).rejects.toMatchObject({ code: 'NOT_FOUND' });
    });
  });

  describe('submitProposal', () => {
    beforeEach(async () => {
      const ownedPath = { ...mockPath, createdBy: 'demo-learner' };
      pathServiceSpy.getPath.mockReturnValue(of(ownedPath));

      await firstValueFrom(service.enableCollaboration('test-path', 'open'));
    });

    it('should submit proposal to collaborative path', async () => {
      const proposal = await firstValueFrom(
        service.submitProposal('test-path', 'add-step', { stepIndex: 0 }, 'Need more content')
      );
      expect(proposal.id).toBeDefined();
      expect(proposal.status).toBe('pending');
      expect(proposal.changeType).toBe('add-step');
    });
  });

  // =========================================================================
  // Agent Management
  // =========================================================================

  describe('setCurrentAgent', () => {
    it('should change current agent', async () => {
      service.setCurrentAgent('new-agent');

      const ext = await firstValueFrom(
        service.createExtension({
          basePathId: 'test-path',
          title: 'Test',
        })
      );
      expect(ext.extendedBy).toBe('new-agent');
    });
  });
});
