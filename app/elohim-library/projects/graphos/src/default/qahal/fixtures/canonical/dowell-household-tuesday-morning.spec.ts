import { describe, expect, it } from 'vitest';
import { dowellHouseholdTuesdayMorning } from './dowell-household-tuesday-morning';

describe('dowell-household-tuesday-morning scene fixture', () => {
  const scene = dowellHouseholdTuesdayMorning;

  it('uses the Dowell household qahal id + label + archetype', () => {
    expect(scene.id).toBe('dowell-household');
    expect(scene.qahalLabel).toContain('Dowell');
    expect(scene.qahalArchetype).toBe('household');
    expect(scene.qahalIcon).toBe('🏠');
  });

  it('binds the DOWELL_HOUSEHOLD_RUBRIC', () => {
    expect(scene.rubric.qahalId).toBe('dowell-household');
    expect(scene.rubric.standingHonors).toContain('care contributed');
  });

  it('includes James as a child member', () => {
    const james = scene.members.find((m) => m.name.includes('James'));
    expect(james).toBeDefined();
    expect(james?.capabilityTier).toBe('child');
  });

  it('includes Gertrude as elder_under_guardianship', () => {
    const gertrude = scene.members.find((m) => m.name.includes('Gertrude'));
    expect(gertrude).toBeDefined();
    expect(gertrude?.capabilityTier).toBe('elder_under_guardianship');
  });

  it('encodes the sick-James moment in the stream', () => {
    const jamesEvent = scene.streamEvents.find((e) =>
      /james.*sick|sick.*james/i.test(e.content)
    );
    expect(jamesEvent).toBeDefined();
  });

  it("includes Sheila's soup event in the stream", () => {
    const soupEvent = scene.streamEvents.find(
      (e) => /sheila/i.test(e.content) && /soup/i.test(e.content)
    );
    expect(soupEvent).toBeDefined();
  });

  it("includes Gertrude's check-in event in the stream", () => {
    const checkIn = scene.streamEvents.find(
      (e) => /gertrude/i.test(e.content) && /check/i.test(e.content)
    );
    expect(checkIn).toBeDefined();
  });

  it("includes the co-steward observation 'the household is steady'", () => {
    expect(scene.coStewardObservation).toMatch(/household.*steady/i);
  });

  it('has at least 3 pending acknowledgments per the narrative', () => {
    expect(scene.pendingAcknowledgments.length).toBeGreaterThanOrEqual(3);
  });

  it('includes at least one curated EPR', () => {
    expect(scene.curatedEprs.length).toBeGreaterThan(0);
  });

  it('includes the cofc-congregation as an otherQahal', () => {
    expect(scene.otherQahals.some((q) => q.id === 'cofc-congregation')).toBe(true);
  });
});
