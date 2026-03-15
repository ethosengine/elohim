/* eslint-disable @typescript-eslint/require-await -- Phase 1: mock stubs returning Promise<T> without async work */
import { Injectable } from '@angular/core';

import { ContentMetadata, ContentNode } from '../../lamad/models/content-node.model';
import { DEFAULT_BOARD_COLUMNS } from '../models/work-project.model';

const CONTENT_FORMAT = 'text' as const;
const WORK_STORY_TYPE = 'work-story' as const;
const MOCK_CREATED_AT = '2026-01-01T00:00:00Z';
const MOCK_PROJECT_ID = 'proj-household-collective';

// Household member IDs
const MATTHEW = 'matthew';
const JESSICA = 'jessica';
const JAMES = 'james';

const MOCK_PROJECTS: ContentNode[] = [
  {
    id: MOCK_PROJECT_ID,
    contentType: 'work-project',
    title: 'Matthew, Jessica & James',
    description: 'Shared household tasks and projects for the family collective.',
    content: '',
    contentFormat: CONTENT_FORMAT,
    tags: ['household', 'family'],
    relatedNodeIds: [],
    metadata: {
      columns: DEFAULT_BOARD_COLUMNS,
      visibility: 'private',
      memberIds: [MATTHEW, JESSICA, JAMES],
    },
    createdAt: MOCK_CREATED_AT,
    updatedAt: MOCK_CREATED_AT,
  },
];

const MOCK_STORIES: ContentNode[] = [
  {
    id: 'story-groceries-weekly',
    contentType: WORK_STORY_TYPE,
    title: 'Weekly grocery run',
    description: 'Pick up groceries for the week — check the shared list first.',
    content: '',
    contentFormat: CONTENT_FORMAT,
    tags: ['food', 'recurring'],
    relatedNodeIds: [],
    metadata: {
      projectId: MOCK_PROJECT_ID,
      status: 'todo',
      visibility: 'private',
      priority: 'high',
      assigneeId: JESSICA,
      cadence: {
        interval: 'weekly',
        resetToStatus: 'todo',
        nextOccurrence: '2026-03-22T00:00:00Z',
      },
    } as unknown as ContentMetadata,
    createdAt: MOCK_CREATED_AT,
    updatedAt: MOCK_CREATED_AT,
  },
  {
    id: 'story-back-fence-repair',
    contentType: WORK_STORY_TYPE,
    title: 'Repair the back fence',
    description: 'Two panels came loose after the last storm. Needs new hardware.',
    content: '',
    contentFormat: CONTENT_FORMAT,
    tags: ['maintenance', 'yard'],
    relatedNodeIds: [],
    metadata: {
      projectId: MOCK_PROJECT_ID,
      status: 'backlog',
      visibility: 'private',
      priority: 'high',
      assigneeId: MATTHEW,
      storyPoints: 5,
    } as unknown as ContentMetadata,
    createdAt: MOCK_CREATED_AT,
    updatedAt: MOCK_CREATED_AT,
  },
  {
    id: 'story-james-reading',
    contentType: WORK_STORY_TYPE,
    title: 'Reading practice with James',
    description: '20 minutes of reading practice each evening before bed.',
    content: '',
    contentFormat: CONTENT_FORMAT,
    tags: ['education', 'recurring'],
    relatedNodeIds: [],
    metadata: {
      projectId: MOCK_PROJECT_ID,
      status: 'in-progress',
      visibility: 'private',
      priority: 'medium',
      assigneeId: MATTHEW,
      cadence: {
        interval: 'daily',
        resetToStatus: 'todo',
        nextOccurrence: '2026-03-16T21:00:00Z',
      },
    } as unknown as ContentMetadata,
    createdAt: MOCK_CREATED_AT,
    updatedAt: MOCK_CREATED_AT,
  },
];

@Injectable({ providedIn: 'root' })
export class AvodahApiService {
  async getProjects(): Promise<ContentNode[]> {
    return [...MOCK_PROJECTS];
  }

  async getStoriesForProject(projectId: string): Promise<ContentNode[]> {
    return MOCK_STORIES.filter(
      s => (s.metadata as Record<string, unknown>)['projectId'] === projectId
    );
  }

  async updateStoryStatus(storyId: string, status: string): Promise<void> {
    const story = MOCK_STORIES.find(s => s.id === storyId);
    if (story) {
      (story.metadata as Record<string, unknown>)['status'] = status;
    }
  }
}
