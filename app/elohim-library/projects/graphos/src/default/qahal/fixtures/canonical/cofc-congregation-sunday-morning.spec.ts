import { describe, expect, it } from 'vitest';
import { cofcCongregationSundayMorning } from './cofc-congregation-sunday-morning';

describe('cofc-congregation-sunday-morning scene fixture', () => {
  const scene = cofcCongregationSundayMorning;

  it('uses the CofC congregation qahal id + congregation archetype', () => {
    expect(scene.id).toBe('cofc-congregation');
    expect(scene.qahalArchetype).toBe('congregation');
    expect(scene.qahalIcon).toBe('⛪');
  });

  it('binds the COFC_CONGREGATION_RUBRIC with 4 elder stewards', () => {
    expect(scene.rubric.qahalId).toBe('cofc-congregation');
    expect(scene.rubric.configuredBy).toHaveLength(4);
    expect(scene.rubric.configuredBy).toContain('brother-cal');
  });

  it('includes Brother Cal as a steward', () => {
    const cal = scene.members.find((m) => m.id === 'brother-cal');
    expect(cal).toBeDefined();
    expect(cal?.standingTier).toBe('steward');
  });

  it('references the Romans 12 sermon series in the stream', () => {
    expect(scene.streamEvents.some((e) => /romans 12/i.test(e.content))).toBe(true);
  });

  it('mentions the youth retreat needing drivers', () => {
    expect(
      scene.streamEvents.some((e) => /youth.*retreat/i.test(e.content) || /drivers?/i.test(e.content))
    ).toBe(true);
  });

  it('co-steward observation references rising reach OR life-groups at threshold', () => {
    expect(scene.coStewardObservation).toMatch(/(reach.*rising|life.?groups?.*threshold|cohesion)/i);
  });
});
