import { describe, expect, it } from 'vitest';
import type { Scene } from '../../../../designed/qahal/_lib/types';
import { householdMultiGeneration } from './household-multi-generation';

describe('household-multi-generation variation fixture', () => {
  const scene: Scene = householdMultiGeneration;

  it('conforms to the Scene type contract', () => {
    expect(scene.id).toBeTruthy();
    expect(scene.qahalLabel).toBeTruthy();
    expect(scene.qahalIcon).toBeTruthy();
    expect(['household', 'congregation', 'life-group', 'wisdom-commons']).toContain(scene.qahalArchetype);
    expect(scene.rubric).toBeDefined();
    expect(Array.isArray(scene.members)).toBe(true);
    expect(Array.isArray(scene.streamEvents)).toBe(true);
    expect(scene.computeTopology).toBeDefined();
    expect(typeof scene.coStewardObservation).toBe('string');
    expect(Array.isArray(scene.curatedEprs)).toBe(true);
    expect(Array.isArray(scene.externalLinks)).toBe(true);
    expect(Array.isArray(scene.pendingAcknowledgments)).toBe(true);
  });
});
