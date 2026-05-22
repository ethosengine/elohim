import { describe, expect, it } from 'vitest';
import { hardinsLifeGroupTuesdayEvening } from './hardins-life-group-tuesday-evening';

describe('hardins-life-group-tuesday-evening scene fixture', () => {
  const scene = hardinsLifeGroupTuesdayEvening;

  it('uses the life-group qahal id + archetype', () => {
    expect(scene.id).toBe('hardins-life-group');
    expect(scene.qahalArchetype).toBe('life-group');
  });

  it('binds the LIFE_GROUP_RUBRIC', () => {
    expect(scene.rubric.qahalId).toBe('hardins-life-group');
  });

  it('references John Hardin as host steward', () => {
    expect(scene.members.some((m) => m.id === 'john-hardin')).toBe(true);
  });

  it('references the Romans 12 v1 discussion in the stream', () => {
    expect(scene.streamEvents.some((e) => /romans 12.*1|verse 1/i.test(e.content))).toBe(true);
  });

  it('references hosting accumulation OR friction-gradient surfacing in stream OR co-steward observation', () => {
    const combined = scene.streamEvents.map((e) => e.content).join(' ') + scene.coStewardObservation;
    expect(combined).toMatch(/(host.*accum|twenty-three|friction-gradient|host.*tuesday)/i);
  });
});
