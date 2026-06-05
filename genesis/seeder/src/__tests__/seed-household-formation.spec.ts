import { describe, it, expect, vi, afterEach } from 'vitest';
import {
  buildHouseholdCharter, buildCeremonyCustodyInput, HOUSEHOLD_MEMBERS,
  resolveExistingCollectiveCid,
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

describe('resolveExistingCollectiveCid', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns the cid when collectiveCid is present and starts with collective:', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ collectiveCid: 'collective:uhCkkABCDEF' }),
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBe('collective:uhCkkABCDEF');
  });

  it('returns null when collectiveCid is null', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ collectiveCid: null }),
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBeNull();
  });

  it('returns null on a non-OK response', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBeNull();
  });

  it('returns null when fetch throws', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('network error')));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBeNull();
  });

  it('returns null when cid does not start with collective:', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ collectiveCid: 'uhCkkBadPrefix' }),
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBeNull();
  });

  it('also handles the snake_case collective_cid field', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ collective_cid: 'collective:uhCkkSNAKE' }),
    }));
    const result = await resolveExistingCollectiveCid('https://doorway.example.host');
    expect(result).toBe('collective:uhCkkSNAKE');
  });
});
