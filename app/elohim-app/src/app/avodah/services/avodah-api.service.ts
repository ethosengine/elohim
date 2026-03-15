/* eslint-disable @typescript-eslint/require-await -- Phase 1: mock stubs returning Promise<T> without async work */
import { Injectable } from '@angular/core';

import { ContentNode } from '../../lamad/models/content-node.model';
import { DEFAULT_BOARD_COLUMNS } from '../models/work-project.model';

const CONTENT_FORMAT = 'text' as const;
const WORK_STORY_TYPE = 'work-story' as const;
const MOCK_CREATED_AT = '2026-01-01T00:00:00Z';
const MOCK_PROJECT_ID = 'proj-household-2026';

const MOCK_PROJECTS: ContentNode[] = [
  {
    id: MOCK_PROJECT_ID,
    contentType: 'work-project',
    title: 'Household 2026',
    description: 'Running tasks and projects for the household.',
    content: '',
    contentFormat: CONTENT_FORMAT,
    tags: ['household'],
    relatedNodeIds: [],
    metadata: {
      columns: DEFAULT_BOARD_COLUMNS,
      visibility: 'private',
    },
    createdAt: MOCK_CREATED_AT,
    updatedAt: MOCK_CREATED_AT,
  },
];

const MOCK_STORIES: ContentNode[] = [
  {
    id: 'story-trash-weekly',
    contentType: WORK_STORY_TYPE,
    title: 'Take out the trash',
    description: 'Weekly recurring chore.',
    content: '',
    contentFormat: CONTENT_FORMAT,
    tags: ['chore', 'recurring'],
    relatedNodeIds: [],
    metadata: {
      projectId: MOCK_PROJECT_ID,
      status: 'todo',
      visibility: 'private',
      priority: 'medium',
      cadence: {
        interval: 'weekly',
        resetToStatus: 'todo',
        nextOccurrence: '2026-03-22T00:00:00Z',
      },
    },
    createdAt: MOCK_CREATED_AT,
    updatedAt: MOCK_CREATED_AT,
  },
  {
    id: 'story-faucet-fix',
    contentType: WORK_STORY_TYPE,
    title: 'Fix the kitchen faucet',
    description: 'The kitchen faucet is dripping and needs repair.',
    content: '',
    contentFormat: CONTENT_FORMAT,
    tags: ['maintenance'],
    relatedNodeIds: [],
    metadata: {
      projectId: MOCK_PROJECT_ID,
      status: 'backlog',
      visibility: 'private',
      priority: 'high',
      storyPoints: 3,
    },
    createdAt: MOCK_CREATED_AT,
    updatedAt: MOCK_CREATED_AT,
  },
  {
    id: 'story-cook-meals',
    contentType: WORK_STORY_TYPE,
    title: 'Cook meals for the week',
    description: 'Batch cook and prepare meals for the coming week.',
    content: '',
    contentFormat: CONTENT_FORMAT,
    tags: ['food', 'recurring'],
    relatedNodeIds: [],
    metadata: {
      projectId: MOCK_PROJECT_ID,
      status: 'in-progress',
      visibility: 'community',
      priority: 'medium',
      cadence: {
        interval: 'weekly',
        resetToStatus: 'todo',
        nextOccurrence: '2026-03-22T00:00:00Z',
      },
    },
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
