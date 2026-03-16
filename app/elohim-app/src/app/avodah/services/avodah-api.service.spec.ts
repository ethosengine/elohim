import { describe, it, expect, beforeEach, vi } from 'vitest';
import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { StorageApiService } from '../../elohim/services/storage-api.service';
import { AvodahApiService } from './avodah-api.service';

const MOCK_PROJECT_VIEW = {
  id: 'proj-1',
  appId: 'lamad',
  contentType: 'work-project',
  title: 'Test Project',
  description: null,
  contentFormat: 'text',
  contentBody: null,
  blobHash: null,
  blobCid: null,
  contentSizeBytes: null,
  reach: 'private',
  validationStatus: 'approved',
  createdBy: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  metadata: { columns: [], visibility: 'private', memberIds: [] },
  tags: ['household'],
};

const MOCK_STORY_VIEW = {
  id: 'story-1',
  appId: 'lamad',
  contentType: 'work-story',
  title: 'Fix the fence',
  description: null,
  contentFormat: 'text',
  contentBody: null,
  blobHash: null,
  blobCid: null,
  contentSizeBytes: null,
  reach: 'private',
  validationStatus: 'approved',
  createdBy: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  metadata: { projectId: 'proj-1', status: 'todo', visibility: 'private', priority: 'high' },
  tags: [],
};

describe('AvodahApiService', () => {
  let service: AvodahApiService;
  let storageSpy: {
    getContents: ReturnType<typeof vi.fn>;
    updateContent: ReturnType<typeof vi.fn>;
    createContent: ReturnType<typeof vi.fn>;
    createEconomicEvent: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    storageSpy = {
      getContents: vi.fn(),
      updateContent: vi.fn().mockReturnValue(of(MOCK_STORY_VIEW)),
      createContent: vi.fn().mockReturnValue(of(MOCK_PROJECT_VIEW)),
      createEconomicEvent: vi.fn().mockReturnValue(of({})),
    };

    TestBed.configureTestingModule({
      providers: [
        AvodahApiService,
        { provide: StorageApiService, useValue: storageSpy },
      ],
    });
    service = TestBed.inject(AvodahApiService);
  });

  it('getProjects fetches work-project content type', async () => {
    storageSpy.getContents.mockReturnValue(of([MOCK_PROJECT_VIEW]));
    const projects = await service.getProjects();
    expect(storageSpy.getContents).toHaveBeenCalledWith({ contentType: 'work-project' });
    expect(projects[0].contentType).toBe('work-project');
    expect(projects[0].id).toBe('proj-1');
  });

  it('getStoriesForProject fetches work-story and filters by projectId', async () => {
    storageSpy.getContents.mockReturnValue(of([MOCK_STORY_VIEW]));
    const stories = await service.getStoriesForProject('proj-1');
    expect(storageSpy.getContents).toHaveBeenCalledWith({ contentType: 'work-story' });
    expect(stories).toHaveLength(1);
    expect(stories[0].id).toBe('story-1');
  });

  it('getStoriesForProject excludes stories from other projects', async () => {
    const otherStory = {
      ...MOCK_STORY_VIEW,
      id: 'story-other',
      metadata: { ...MOCK_STORY_VIEW.metadata, projectId: 'proj-99' },
    };
    storageSpy.getContents.mockReturnValue(of([MOCK_STORY_VIEW, otherStory]));
    const stories = await service.getStoriesForProject('proj-1');
    expect(stories).toHaveLength(1);
    expect(stories[0].id).toBe('story-1');
  });

  it('updateStoryStatus patches metadata.status', async () => {
    await service.updateStoryStatus('story-1', 'in-progress');
    expect(storageSpy.updateContent).toHaveBeenCalledWith('story-1', {
      metadata: { status: 'in-progress' },
    });
  });

  it('updateStoryStatus does NOT emit economic event for non-terminal status', async () => {
    await service.updateStoryStatus('story-1', 'in-progress', false);
    expect(storageSpy.createEconomicEvent).not.toHaveBeenCalled();
  });

  it('updateStoryStatus emits economic event when isTerminal=true', async () => {
    await service.updateStoryStatus('story-1', 'done', true);
    expect(storageSpy.updateContent).toHaveBeenCalledWith('story-1', {
      metadata: { status: 'done' },
    });
    expect(storageSpy.createEconomicEvent).toHaveBeenCalledWith(
      expect.objectContaining({ action: 'work', contentId: 'story-1' }),
    );
  });

  it('updateStoryField patches via storageApi', async () => {
    await service.updateStoryField('story-1', { title: 'New title' });
    expect(storageSpy.updateContent).toHaveBeenCalledWith('story-1', { title: 'New title' });
  });

  it('createStory creates a work-story via storageApi', async () => {
    storageSpy.createContent.mockReturnValue(of(MOCK_STORY_VIEW));
    const result = await service.createStory('proj-1', 'New task', 'todo');
    expect(storageSpy.createContent).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'New task',
        contentType: 'work-story',
        metadata: expect.objectContaining({ projectId: 'proj-1', status: 'todo' }),
      }),
    );
    expect(result.id).toBe('story-1');
  });
});
