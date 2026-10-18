import { describe, expect, it } from 'vitest';
import { acceptance, burndown, presentation, type Reading } from './view-reading';
import type { Subject } from './view-contract';
function subject(id: string, status: string, assertions?: string[]): Subject {
  return {
    identity: id,
    source: `${id}.md`,
    version: 'source',
    fields: { phase: status },
    relationships: {},
    readings: {
      evidence: assertions
        ? {
            state: 'available',
            value: { assertions: assertions.map(acceptance => ({ acceptance })) },
          }
        : { state: 'unavailable' },
    },
  };
}
function reading(sequence: number, subjects: Subject[], terminal = ['done']): Reading {
  return {
    observation: {
      sequence,
      observedAt: `2026-10-13T10:0${sequence}:00Z`,
      declarationCid: `d${sequence}`,
      snapshotCid: `s${sequence}`,
    },
    projection: {
      version: 1,
      purpose: 'fixture',
      declaration: 'view.json',
      declaration_cid: `d${sequence}`,
      method: 'native',
      reader_versions: {},
      subjects,
      sources: {},
      input_versions: {},
      governance: {},
      diagnostics: [],
    },
    presentation: presentation({
      titleField: '$identity',
      statusField: 'phase',
      terminalValues: terminal,
      reconciliationReading: 'evidence',
    }),
  };
}
describe('declared baseline burndown', () => {
  it('separates authored, accepted, unknown, added and missing; revalidation can reopen work', () => {
    const baseline = reading(1, [
      subject('a', 'open'),
      subject('b', 'done', ['accepted']),
      subject('c', 'open', ['produced']),
    ]);
    const second = reading(2, [
      subject('a', 'done', ['accepted']),
      subject('b', 'done', ['needs-revalidation']),
      subject('new', 'done', ['accepted']),
    ]);
    const points = burndown([baseline, second], baseline);
    expect(points[0]).toMatchObject({
      authoredRemaining: 2,
      acceptanceMin: 1,
      acceptanceMax: 2,
      unknown: 1,
    });
    expect(points[1]).toMatchObject({
      authoredRemaining: 1,
      acceptanceMin: 1,
      acceptanceMax: 2,
      unknown: 1,
      missing: 1,
      additions: 1,
    });
    expect(baseline.projection.subjects[1].readings).toEqual({
      evidence: { state: 'available', value: { assertions: [{ acceptance: 'accepted' }] } },
    });
  });
  it('requires a nonempty current assertion set and retains declaration semantics per observation', () => {
    expect(acceptance(subject('a', 'done', []), 'evidence')).toBe('unknown');
    expect(acceptance(subject('a', 'done', ['accepted', 'produced']), 'evidence')).toBe(
      'remaining'
    );
    const first = reading(1, [subject('a', 'done')]);
    const second = reading(2, [subject('a', 'done')], ['harvested']);
    expect(burndown([first, second], first).map(p => [p.authoredRemaining, p.available])).toEqual([
      [0, false],
      [1, false],
    ]);
    expect(burndown([first, second], second)).toHaveLength(1);
  });
});
