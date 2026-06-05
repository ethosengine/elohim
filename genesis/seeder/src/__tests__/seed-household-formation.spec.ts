import { describe, it, expect } from 'vitest';
import {
  buildHouseholdCharter, buildCeremonyCustodyInput, HOUSEHOLD_MEMBERS,
} from '../seed-household-formation.js';

describe('buildHouseholdCharter', () => {
  it('declares household kind, rubric, and the family-dowell slug alias', () => {
    const charter = JSON.parse(buildHouseholdCharter());
    expect(charter.kind).toBe('household');
    expect(charter.rubric).toBe('recognition-of-given');
    expect(charter.slugAlias).toBe('family-dowell');
  });
});

describe('buildCeremonyCustodyInput', () => {
  it('builds a snake_case zome input with ceremony provenance and collective scope', () => {
    const input = buildCeremonyCustodyInput({
      providerHumanId: 'human-jessica-spouse', providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager', receiverArchetype: 'desktop',
      blobHash: 'sha256-deadbeef', blobSizeBytes: 64,
      collectiveCid: 'collective:uhCkkFAKE',
    });
    expect(input.action).toBe('custody-blob');
    expect(input.provider.startsWith('12D3KooW')).toBe(true);
    expect(input.in_scope_of).toEqual(['collective:uhCkkFAKE']);
    expect(JSON.parse(input.metadata_json!).seedGeneration).toBe('ceremony');
    expect(JSON.parse(input.metadata_json!).providerHumanId).toBe('human-jessica-spouse');
  });
});

describe('HOUSEHOLD_MEMBERS', () => {
  it('is the canonical triad with the founder first', () => {
    expect(HOUSEHOLD_MEMBERS.map(m => m.humanId)).toEqual([
      'human-matthew-manager', 'human-jessica-spouse', 'human-james-student',
    ]);
    expect(HOUSEHOLD_MEMBERS[2].minor).toBe(true);
  });
});
