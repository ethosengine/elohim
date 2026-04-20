// Re-export WorkStoryMeta from generated types (schema is source of truth)
export type { WorkStoryMeta } from '../generated/metadata-types';

import type { ContentMetadata } from '../../lamad/models/content-node.model';
import type { WorkStoryMeta } from '../generated/metadata-types';

export type WorkStoryStatus = 'backlog' | 'todo' | 'in-progress' | 'review' | 'done';
export type WorkVisibility = 'private' | 'community' | 'exchange';
export type WorkPriority = 'low' | 'medium' | 'high' | 'urgent';
export type CadenceInterval = 'daily' | 'weekly' | 'monthly' | 'custom';

export interface WorkCadence {
  interval: CadenceInterval;
  customIntervalDays?: number;
  resetToStatus: 'backlog' | 'todo';
  nextOccurrence: string; // ISO date
}

const DEFAULTS: WorkStoryMeta = {
  projectId: '',
  status: 'backlog',
  visibility: 'private',
  priority: 'medium',
};

export function parseWorkStoryMeta(raw: ContentMetadata): WorkStoryMeta {
  return { ...DEFAULTS, ...raw } as WorkStoryMeta;
}
