import { describe, it, expect } from 'vitest';
import { parseWorkStoryMeta } from './work-story.model';
import { DEFAULT_BOARD_COLUMNS, parseWorkProjectMeta } from './work-project.model';

describe('parseWorkStoryMeta', () => {
  it('returns defaults when metadata is empty', () => {
    const result = parseWorkStoryMeta({});
    expect(result.status).toBe('backlog');
    expect(result.visibility).toBe('private');
    expect(result.priority).toBe('medium');
  });

  it('preserves cadence when present', () => {
    const cadence = { interval: 'weekly' as const, resetToStatus: 'todo' as const, nextOccurrence: '2026-03-22' };
    const result = parseWorkStoryMeta({ cadence });
    expect(result.cadence?.interval).toBe('weekly');
  });
});

describe('DEFAULT_BOARD_COLUMNS', () => {
  it('has five columns ending with a terminal done column', () => {
    expect(DEFAULT_BOARD_COLUMNS).toHaveLength(5);
    const last = DEFAULT_BOARD_COLUMNS[DEFAULT_BOARD_COLUMNS.length - 1];
    expect(last.isTerminal).toBe(true);
    expect(last.id).toBe('done');
  });
});

describe('parseWorkProjectMeta', () => {
  it('returns defaults when metadata is empty', () => {
    const result = parseWorkProjectMeta({});
    expect(result.visibility).toBe('private');
    expect(result.columns).toEqual(DEFAULT_BOARD_COLUMNS);
  });

  it('overrides visibility when provided', () => {
    const result = parseWorkProjectMeta({ visibility: 'community' });
    expect(result.visibility).toBe('community');
  });

  it('overrides columns when provided', () => {
    const custom = [{ id: 'open', name: 'Open' }, { id: 'closed', name: 'Closed', isTerminal: true }];
    const result = parseWorkProjectMeta({ columns: custom });
    expect(result.columns).toHaveLength(2);
    expect(result.columns[1].isTerminal).toBe(true);
  });

  it('includes memberIds when provided', () => {
    const result = parseWorkProjectMeta({ memberIds: ['abc', 'def'] });
    expect(result.memberIds).toEqual(['abc', 'def']);
  });
});
