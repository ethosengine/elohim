// AUTO-GENERATED from avodah manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run avodah:codegen

export interface WorkStoryMeta {
  status?: 'backlog' | 'todo' | 'in-progress' | 'review' | 'done';
  priority?: 'low' | 'medium' | 'high' | 'urgent';
  visibility?: 'private' | 'community' | 'exchange';
  cadence?: { frequency?: 'daily' | 'weekly' | 'monthly' | 'custom'; resetBehavior?: 'reset-to-backlog' | 'reset-to-todo' | 'archive' };
  /** Lamad content IDs that must be mastered before this task can be started */
  attestationGates?: string[];
  /** Whether to publish to shefa exchange as a service request */
  exchangePublish?: boolean;
  projectId?: string;
  assignedTo?: string;
  [key: string]: unknown;
}

export interface WorkProjectMeta {
  visibility?: 'private' | 'community';
  columns?: { id?: string; label?: string; isTerminal?: boolean }[];
  members?: string[];
  defaultCadence?: string;
  [key: string]: unknown;
}
