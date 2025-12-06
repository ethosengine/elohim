import { TestBed } from '@angular/core/testing';
import { ContentEditorService, ContentDraft, SaveResult } from './content-editor.service';
import { ContentFormatRegistryService } from './content-format-registry.service';
import { ContentNode } from '../../models/content-node.model';
import { ContentFormatPlugin, DEFAULT_EDITOR_CONFIG } from '../interfaces/content-format-plugin.interface';

describe('ContentEditorService', () => {
  let service: ContentEditorService;
  let mockRegistry: jasmine.SpyObj<ContentFormatRegistryService>;

  const createMockNode = (overrides: Partial<ContentNode> = {}): ContentNode => ({
    id: 'test-node-1',
    title: 'Test Node',
    description: 'Test description',
    content: '# Test Content',
    contentType: 'epic',
    contentFormat: 'markdown',
    tags: ['test', 'demo'],
    relatedNodeIds: [],
    metadata: {},
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    ...overrides
  });

  beforeEach(() => {
    mockRegistry = jasmine.createSpyObj('ContentFormatRegistryService', [
      'getPlugin',
      'getEditorComponent',
      'getEditorConfig'
    ]);
    mockRegistry.getPlugin.and.returnValue(undefined);
    mockRegistry.getEditorComponent.and.returnValue(null);
    mockRegistry.getEditorConfig.and.returnValue(DEFAULT_EDITOR_CONFIG);

    TestBed.configureTestingModule({
      providers: [
        ContentEditorService,
        { provide: ContentFormatRegistryService, useValue: mockRegistry }
      ]
    });
    service = TestBed.inject(ContentEditorService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('canEdit', () => {
    it('should return true for regular content nodes', () => {
      const node = createMockNode({ id: 'epic-intro-1' });
      expect(service.canEdit(node)).toBeTrue();
    });

    it('should return false for null node', () => {
      expect(service.canEdit(null)).toBeFalse();
    });

    it('should return false for path-prefixed IDs (dynamically generated)', () => {
      const node = createMockNode({ id: 'path-123-step-0' });
      expect(service.canEdit(node)).toBeFalse();
    });

    it('should return true for path content type without path- prefix', () => {
      const node = createMockNode({ id: 'learning-path-1', contentType: 'path' });
      expect(service.canEdit(node)).toBeTrue();
    });

    it('should return false for nodes with path- prefix regardless of content type', () => {
      const node = createMockNode({ id: 'path-abc-step-2', contentType: 'epic' });
      expect(service.canEdit(node)).toBeFalse();
    });
  });

  describe('canEditFormat', () => {
    it('should return true when editor component is available', () => {
      const mockEditor = {} as any;
      mockRegistry.getEditorComponent.and.returnValue(mockEditor);

      expect(service.canEditFormat('markdown')).toBeTrue();
      expect(mockRegistry.getEditorComponent).toHaveBeenCalledWith('markdown');
    });

    it('should return false when no editor component available', () => {
      mockRegistry.getEditorComponent.and.returnValue(null);

      expect(service.canEditFormat('unknown')).toBeFalse();
    });
  });

  describe('createDraft', () => {
    it('should create a draft from a content node', () => {
      const node = createMockNode();

      const draft = service.createDraft(node);

      expect(draft.id).toMatch(/^draft-\d+-\w+$/);
      expect(draft.originalNodeId).toBe(node.id);
      expect(draft.content.title).toBe(node.title);
      expect(draft.content.description).toBe(node.description);
      expect(draft.content.content).toBe(node.content);
      expect(draft.content.contentFormat).toBe(node.contentFormat);
      expect(draft.content.tags).toEqual(node.tags);
      expect(draft.isDirty).toBeFalse();
    });

    it('should create unique draft IDs', () => {
      const node = createMockNode();

      const draft1 = service.createDraft(node);
      const draft2 = service.createDraft(node);

      expect(draft1.id).not.toBe(draft2.id);
    });

    it('should store draft in internal map', () => {
      const node = createMockNode();

      const draft = service.createDraft(node);
      const retrieved = service.getDraft(draft.id);

      expect(retrieved).toBe(draft);
    });
  });

  describe('createNewDraft', () => {
    it('should create a draft for new content', () => {
      const draft = service.createNewDraft('markdown');

      expect(draft.originalNodeId).toBeNull();
      expect(draft.content.title).toBe('Untitled');
      expect(draft.content.content).toBe('');
      expect(draft.content.contentFormat).toBe('markdown');
      expect(draft.isDirty).toBeTrue(); // New content is always dirty
    });

    it('should use provided initial data', () => {
      const draft = service.createNewDraft('gherkin', {
        title: 'My Feature',
        content: 'Feature: Test',
        description: 'A test feature'
      });

      expect(draft.content.title).toBe('My Feature');
      expect(draft.content.content).toBe('Feature: Test');
      expect(draft.content.description).toBe('A test feature');
    });
  });

  describe('getDraft', () => {
    it('should return draft by ID', () => {
      const node = createMockNode();
      const draft = service.createDraft(node);

      const retrieved = service.getDraft(draft.id);

      expect(retrieved).toBe(draft);
    });

    it('should return undefined for unknown ID', () => {
      const retrieved = service.getDraft('nonexistent');

      expect(retrieved).toBeUndefined();
    });
  });

  describe('updateDraft', () => {
    it('should update draft with changes and mark as dirty', () => {
      const node = createMockNode();
      const draft = service.createDraft(node);

      const updated = service.updateDraft(draft.id, { title: 'New Title' });

      expect(updated).toBeDefined();
      expect(updated!.content.title).toBe('New Title');
      expect(updated!.isDirty).toBeTrue();
    });

    it('should preserve unchanged fields', () => {
      const node = createMockNode();
      const draft = service.createDraft(node);
      const originalDescription = draft.content.description;

      service.updateDraft(draft.id, { title: 'New Title' });

      expect(draft.content.description).toBe(originalDescription);
    });

    it('should return undefined for unknown draft ID', () => {
      const updated = service.updateDraft('nonexistent', { title: 'New Title' });

      expect(updated).toBeUndefined();
    });

    it('should allow updating multiple fields at once', () => {
      const node = createMockNode();
      const draft = service.createDraft(node);

      service.updateDraft(draft.id, {
        title: 'New Title',
        description: 'New Description',
        content: 'New Content',
        tags: ['new', 'tags']
      });

      expect(draft.content.title).toBe('New Title');
      expect(draft.content.description).toBe('New Description');
      expect(draft.content.content).toBe('New Content');
      expect(draft.content.tags).toEqual(['new', 'tags']);
    });
  });

  describe('deleteDraft', () => {
    it('should remove draft from internal storage', () => {
      const node = createMockNode();
      const draft = service.createDraft(node);

      const deleted = service.deleteDraft(draft.id);

      expect(deleted).toBeTrue();
      expect(service.getDraft(draft.id)).toBeUndefined();
    });

    it('should return false if draft does not exist', () => {
      const deleted = service.deleteDraft('nonexistent');

      expect(deleted).toBeFalse();
    });
  });

  describe('getAllDrafts', () => {
    it('should return all drafts', () => {
      const node1 = createMockNode({ id: 'node-1' });
      const node2 = createMockNode({ id: 'node-2' });
      service.createDraft(node1);
      service.createDraft(node2);

      const drafts = service.getAllDrafts();

      expect(drafts.length).toBe(2);
    });

    it('should return empty array when no drafts', () => {
      const drafts = service.getAllDrafts();

      expect(drafts).toEqual([]);
    });
  });

  describe('hasDraft', () => {
    it('should return true when node has dirty draft', () => {
      const node = createMockNode({ id: 'node-1' });
      const draft = service.createDraft(node);
      service.updateDraft(draft.id, { title: 'Changed' });

      expect(service.hasDraft('node-1')).toBeTrue();
    });

    it('should return false when node has clean draft', () => {
      const node = createMockNode({ id: 'node-1' });
      service.createDraft(node);

      expect(service.hasDraft('node-1')).toBeFalse();
    });

    it('should return false when no draft exists', () => {
      expect(service.hasDraft('nonexistent')).toBeFalse();
    });
  });

  describe('validateContent', () => {
    it('should return warning when no plugin available', async () => {
      mockRegistry.getPlugin.and.returnValue(undefined);

      const result = await service.validateContent('unknown', '# Content');

      expect(result.valid).toBeTrue();
      expect(result.warnings.length).toBeGreaterThan(0);
      expect(result.warnings[0].code).toBe('NO_PLUGIN');
    });

    it('should use plugin validation when available', async () => {
      const mockPlugin: Partial<ContentFormatPlugin> = {
        canValidate: true,
        validate: jasmine.createSpy('validate').and.returnValue(
          Promise.resolve({ valid: true, errors: [], warnings: [] })
        )
      };
      mockRegistry.getPlugin.and.returnValue(mockPlugin as ContentFormatPlugin);

      const result = await service.validateContent('markdown', '# Content');

      expect(result.valid).toBeTrue();
      expect(mockPlugin.validate).toHaveBeenCalled();
    });

    it('should return valid when plugin cannot validate', async () => {
      const mockPlugin: Partial<ContentFormatPlugin> = {
        canValidate: false
      };
      mockRegistry.getPlugin.and.returnValue(mockPlugin as ContentFormatPlugin);

      const result = await service.validateContent('markdown', '# Content');

      expect(result.valid).toBeTrue();
      expect(result.errors).toEqual([]);
    });
  });

  describe('saveContent', () => {
    it('should mark draft as saved', (done) => {
      const node = createMockNode();
      const draft = service.createDraft(node);
      service.updateDraft(draft.id, { title: 'Changed' });

      service.saveContent(draft.id).subscribe({
        next: (result: SaveResult) => {
          expect(result.success).toBeTrue();
          expect(draft.isDirty).toBeFalse();
          done();
        },
        error: done.fail
      });
    });

    it('should return error for unknown draft', (done) => {
      service.saveContent('nonexistent').subscribe({
        next: () => done.fail('Should have thrown'),
        error: (err) => {
          expect(err.message).toContain('Draft not found');
          done();
        }
      });
    });

    it('should return node ID in save result', (done) => {
      const node = createMockNode();
      const draft = service.createDraft(node);

      service.saveContent(draft.id).subscribe({
        next: (result: SaveResult) => {
          expect(result.nodeId).toBe(node.id);
          done();
        },
        error: done.fail
      });
    });
  });

  describe('exportContent', () => {
    it('should return content as-is when no plugin', async () => {
      mockRegistry.getPlugin.and.returnValue(undefined);

      const result = await service.exportContent({
        title: 'Test',
        content: '# Content',
        contentFormat: 'unknown',
        contentType: 'concept',
        description: '',
        tags: [],
        metadata: {}
      });

      expect(result).toBe('# Content');
    });

    it('should use plugin export when available', async () => {
      const mockPlugin: Partial<ContentFormatPlugin> = {
        export: jasmine.createSpy('export').and.returnValue(Promise.resolve('exported'))
      };
      mockRegistry.getPlugin.and.returnValue(mockPlugin as ContentFormatPlugin);

      const result = await service.exportContent({
        title: 'Test',
        content: '# Content',
        contentFormat: 'markdown',
        contentType: 'concept',
        description: '',
        tags: [],
        metadata: {}
      });

      expect(result).toBe('exported');
      expect(mockPlugin.export).toHaveBeenCalled();
    });

    it('should JSON stringify object content when no plugin', async () => {
      mockRegistry.getPlugin.and.returnValue(undefined);
      const contentObj = { key: 'value' };

      const result = await service.exportContent({
        title: 'Test',
        content: contentObj as any,
        contentFormat: 'unknown',
        contentType: 'concept',
        description: '',
        tags: [],
        metadata: {}
      });

      expect(result).toBe(JSON.stringify(contentObj, null, 2));
    });
  });
});
