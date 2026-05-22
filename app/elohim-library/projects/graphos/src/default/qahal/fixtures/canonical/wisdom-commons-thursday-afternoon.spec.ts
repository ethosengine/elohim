import { describe, expect, it } from 'vitest';
import { wisdomCommonsThursdayAfternoon } from './wisdom-commons-thursday-afternoon';

describe('wisdom-commons-thursday-afternoon scene fixture', () => {
  const scene = wisdomCommonsThursdayAfternoon;

  it('uses the wisdom-commons qahal id + archetype', () => {
    expect(scene.id).toBe('wisdom-commons');
    expect(scene.qahalArchetype).toBe('wisdom-commons');
    expect(scene.qahalIcon).toBe('🌳');
  });

  it('references the Arkansas sister congregation', () => {
    expect(scene.streamEvents.some((e) => /arkansas/i.test(e.content))).toBe(true);
  });

  it('references peer council convening or REA reconciliation', () => {
    const combined = scene.streamEvents.map((e) => e.content).join(' ');
    expect(combined).toMatch(/(peer council|reconciliation|witness)/i);
  });

  it('includes the Arkansas elder as a peer member', () => {
    expect(scene.members.some((m) => m.id === 'arkansas-elder')).toBe(true);
  });

  it('binds the WISDOM_COMMONS_RUBRIC', () => {
    expect(scene.rubric.qahalId).toBe('wisdom-commons');
  });
});
