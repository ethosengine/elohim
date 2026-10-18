import { describe, expect, it } from 'vitest';
import { claims, collection, evidenceSummary, movement, remainingActions } from './work-progress';
import { presentation, type Reading } from './view-reading';
import type { Subject } from './view-contract';
function subject(acceptance?: string): Subject {
  return {
    identity: 'tasks:one',
    source: 'one.md',
    version: 'v1',
    fields: { $group: 'tasks', $claims: [{ text: 'DELTA done', line: 7 }] },
    relationships: {},
    readings: {
      evidence: acceptance
        ? {
            state: 'available',
            value: {
              assertions: [
                {
                  gap_id: 'g1',
                  acceptance,
                  remaining_action: 'Revalidate source',
                  commitments: [{ fulfillments: ['production'], technical_reviews: ['review'] }],
                },
              ],
            },
          }
        : { state: 'unavailable' },
    },
  };
}
function reading(s: Subject): Reading {
  return {
    observation: { sequence: 1, observedAt: '2026-10-13', snapshotCid: 's', declarationCid: 'd' },
    presentation: presentation(undefined),
    projection: {
      version: 2,
      purpose: 'Work',
      declaration: 'view.json',
      declaration_cid: 'd',
      method: 'composition',
      reader_versions: { reader: 'v1' },
      subjects: [s],
      sources: {},
      input_versions: {},
      governance: {},
      diagnostics: [],
      composition: {
        authority: 'Local',
        views: ['Progress'],
        links: [],
        groups: [
          { id: 'tasks', label: 'Tasks', role: 'work', presentation: presentation(undefined) },
        ],
      },
    },
  };
}
describe('domain-neutral progress', () => {
  it('does not promote claims, missing evidence or first capture into verified progress', () => {
    const s = subject();
    const r = reading(s);
    expect(movement(s, r)).toBe('Baseline established');
    expect(evidenceSummary(s)).toBe('Structured evidence unavailable');
    expect(claims(s)[0].text).toBe('DELTA done');
    expect(movement(s, r, r)).toBe('Unchanged reading');
  });
  it('distinguishes acceptance, regression, coverage loss and interpretation changes', () => {
    const before = reading(subject('remaining'));
    const accepted = subject('accepted');
    expect(movement(accepted, reading(accepted), before)).toBe('New verified acceptance');
    expect(evidenceSummary(accepted)).toBe('1 produced · 1 reviewed · 1/1 accepted assertions');
    expect(movement(subject('revalidation-required'), before, reading(accepted))).toBe(
      'Acceptance reopened'
    );
    expect(movement(subject(), before, reading(accepted))).toBe('Evidence coverage changed');
    const changed = reading(accepted);
    changed.projection.reader_versions = { reader: 'v2' };
    expect(movement(accepted, changed, before)).toBe('Interpretation changed — review evidence');
    expect(remainingActions(subject('revalidation-required'))).toEqual(['Revalidate source']);
  });
  it('tracks native orphan commitments without gap labels or intents', () => {
    const before = subject('remaining');
    before.readings = {
      evidence: {
        state: 'available',
        value: {
          assertions: [
            {
              intent: null,
              gap_id: null,
              acceptance: 'remaining',
              commitments: [{ commitment: 'commitment-cid' }],
            },
          ],
        },
      },
    };
    const after = structuredClone(before);
    after.readings = {
      evidence: {
        state: 'available',
        value: {
          assertions: [
            {
              intent: null,
              gap_id: null,
              acceptance: 'accepted',
              commitments: [{ commitment: 'commitment-cid' }],
            },
          ],
        },
      },
    };
    expect(movement(after, reading(after), reading(before))).toBe('New verified acceptance');
  });
  it('keeps a homogeneous cohort and includes claims as authored changes only', () => {
    const r = reading(subject());
    r.projection.subjects.push({
      ...subject(),
      identity: 'goals:one',
      fields: { $group: 'goals' },
    });
    expect(collection(r, 'tasks').projection.subjects.map(s => s.identity)).toEqual(['tasks:one']);
    const next = subject();
    next.fields['$claims'] = [{ text: 'DELTA new', line: 8 }];
    expect(movement(next, reading(next), r)).toBe('Authored claims changed');
  });
});
