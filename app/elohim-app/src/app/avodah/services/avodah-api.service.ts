import { Injectable } from '@angular/core';

import { ContentNode } from '../../lamad/models/content-node.model';
import { DEFAULT_BOARD_COLUMNS } from '../models/work-project.model';

const MOCK_PROJECT_ID = 'proj-household-2026';

const MOCK_PROJECTS: ContentNode[] = [
  {
    id: MOCK_PROJECT_ID,
    contentType: 'work-project',
    title: 'Household 2026',
    description: 'Running tasks and projects for the household.',
    content: '',
    contentFormat: 'text',
    tags: ['household'],
    relatedNodeIds: [],
    metadata: {
      columns: DEFAULT_BOARD_COLUMNS,
      visibility: 'private',
    },
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  },
];

const MOCK_STORIES: ContentNode[] = [
  {
    id: 'story-trash-weekly',
    contentType: 'work-story',
    title: 'Take out the trash',
    description: 'Weekly recurring chore.',
    content: '',
    contentFormat: 'text',
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
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  },
  {
    id: 'story-faucet-fix',
    contentType: 'work-story',
    title: 'Fix the kitchen faucet',
    description: 'The kitchen faucet is dripping and needs repair.',
    content: '',
    contentFormat: 'text',
    tags: ['maintenance'],
    relatedNodeIds: [],
    metadata: {
      projectId: MOCK_PROJECT_ID,
      status: 'backlog',
      visibility: 'private',
      priority: 'high',
      storyPoints: 3,
    },
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  },
  {
    id: 'story-cook-meals',
    contentType: 'work-story',
    title: 'Cook meals for the week',
    description: 'Batch cook and prepare meals for the coming week.',
    content: '',
    contentFormat: 'text',
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
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  },
];

@Injectable({ providedIn: 'root' })
export class AvodahApiService {
  getProjects(): Promise<ContentNode[]> {
    return Promise.resolve([...MOCK_PROJECTS]);
  }

  getStoriesForProject(projectId: string): Promise<ContentNode[]> {
    const stories = MOCK_STORIES.filter(
      s => (s.metadata as Record<string, unknown>)['projectId'] === projectId,
    );
    return Promise.resolve(stories);
  }

  updateStoryStatus(storyId: string, status: string): Promise<void> {
    const story = MOCK_STORIES.find(s => s.id === storyId);
    if (story) {
      (story.metadata as Record<string, unknown>)['status'] = status;
    }
    return Promise.resolve();
  }
}
