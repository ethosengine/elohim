// AUTO-GENERATED from avodah manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run avodah:codegen

export interface WorkStoryMeta {
  projectId?: string;
  status?: 'backlog' | 'todo' | 'in-progress' | 'review' | 'done';
  visibility?: 'private' | 'community' | 'exchange';
  priority?: 'low' | 'medium' | 'high' | 'urgent';
  storyPoints?: number;
  assigneeId?: string;
  /** Lamad content IDs that must be mastered before this task can be started */
  attestationGates?: string[];
  /** Shefa ServiceRequest ID — set when published to exchange */
  exchangeRequestId?: string;
  cadence?: {
    interval?: 'daily' | 'weekly' | 'monthly' | 'custom';
    customIntervalDays?: number;
    resetToStatus?: 'backlog' | 'todo';
    nextOccurrence?: string;
  };
  [key: string]: unknown;
}

export interface WorkProjectMeta {
  columns?: { id?: string; name?: string; color?: string; isTerminal?: boolean }[];
  visibility?: 'private' | 'community';
  memberIds?: string[];
  [key: string]: unknown;
}
