export interface BoardColumn {
  id: string;
  name: string;
  color?: string;
  /** Terminal columns trigger cadence reset when story moves here */
  isTerminal?: boolean;
}

/** Structured payload stored in ContentNode.metadata for work-project nodes */
export interface WorkProjectMeta {
  columns: BoardColumn[];
  visibility: 'private' | 'community';
  memberIds?: string[];
}

export const DEFAULT_BOARD_COLUMNS: BoardColumn[] = [
  { id: 'backlog', name: 'Backlog', color: '#64748b' },
  { id: 'todo', name: 'To Do', color: '#6366f1' },
  { id: 'in-progress', name: 'In Progress', color: '#f59e0b' },
  { id: 'review', name: 'Review', color: '#8b5cf6' },
  { id: 'done', name: 'Done', color: '#10b981', isTerminal: true },
];

export function parseWorkProjectMeta(raw: Record<string, unknown>): WorkProjectMeta {
  return {
    columns: DEFAULT_BOARD_COLUMNS,
    visibility: 'private',
    ...raw,
  } as WorkProjectMeta;
}
