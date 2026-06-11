import { describe, it, expect } from 'vitest';
import { earnedReach } from '../seed-sqlite.js';
import { REACH_OPENNESS } from '../generated/schema-enums.js';

describe('earnedReach (inverted burden)', () => {
  it('defaults unauthored content to private', () => {
    expect(earnedReach({ authored: undefined, advisory: undefined })).toBe('private');
  });
  it('honors an authored value above the default', () => {
    expect(earnedReach({ authored: 'intimate', advisory: undefined })).toBe('intimate');
  });
  it('raises to the more-open of authored vs archetype advisory', () => {
    expect(earnedReach({ authored: 'private', advisory: 'commons' })).toBe('commons');
    expect(earnedReach({ authored: 'commons', advisory: 'community' })).toBe('commons');
  });
  it('HARD-FAILS on a non-canonical reach value (no silent coalesce)', () => {
    expect(() => earnedReach({ authored: 'invited', advisory: undefined })).toThrow(/non-canonical reach/i);
  });
  it('uses the generated ordinal, not a local copy', () => {
    expect(REACH_OPENNESS.private).toBe(1);
    expect(REACH_OPENNESS.commons).toBe(8);
  });
});
