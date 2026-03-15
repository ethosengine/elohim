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

/** Structured payload stored in ContentNode.metadata for work-story nodes */
export interface WorkStoryMeta {
  projectId: string;
  status: WorkStoryStatus;
  visibility: WorkVisibility;
  priority: WorkPriority;
  storyPoints?: number;
  assigneeId?: string;
  /** lamad ContentNode IDs required to bid/accept this story */
  attestationGates?: string[];
  /** shefa ServiceRequest ID — set when published to exchange */
  exchangeRequestId?: string;
  cadence?: WorkCadence;
}

const DEFAULTS: WorkStoryMeta = {
  projectId: '',
  status: 'backlog',
  visibility: 'private',
  priority: 'medium',
};

export function parseWorkStoryMeta(raw: Record<string, unknown>): WorkStoryMeta {
  return { ...DEFAULTS, ...raw } as WorkStoryMeta;
}
