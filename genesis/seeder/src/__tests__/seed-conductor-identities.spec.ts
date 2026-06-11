/**
 * Unit tests for the pure conductor-affinity + identity helpers in
 * seed-conductor-identities.ts.
 *
 * These encode the genesis #1119 founder-FATAL lessons:
 *   - exists-checks must compare HUMAN IDS, not truthiness (the old check
 *     declared every human "exists" because matthew's conductor had SOME
 *     human on it);
 *   - a human binds only to THEIR pod under the `elohim-<name>-<env>`
 *     service convention (first-reachable-wins lets one profile squat
 *     every probe).
 */
import { describe, expect, it } from 'vitest';

import {
  conductorUrlForHuman,
  extractHumanId,
  humanShortName,
  urlsAreNameAffine,
} from '../seed-conductor-identities.js';

const ALPHA_URLS = [
  'ws://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:4445',
  'ws://elohim-adam-alpha.elohim-alpha.svc.cluster.local:4445',
  'ws://elohim-jessica-alpha.elohim-alpha.svc.cluster.local:4445',
];

describe('humanShortName', () => {
  it('extracts the first token after the human- prefix (Jenkinsfile rule)', () => {
    expect(humanShortName('human-matthew-manager')).toBe('matthew');
    expect(humanShortName('human-adam-firstman')).toBe('adam');
    expect(humanShortName('human-jessica-spouse')).toBe('jessica');
    expect(humanShortName('human-james-son')).toBe('james');
  });
});

describe('urlsAreNameAffine', () => {
  it('recognizes the elohim-<name>-<env> service convention', () => {
    expect(urlsAreNameAffine(ALPHA_URLS)).toBe(true);
  });

  it('rejects local-dev URL sets (legacy walk applies there)', () => {
    expect(urlsAreNameAffine(['ws://localhost:4445'])).toBe(false);
    expect(urlsAreNameAffine([])).toBe(false);
  });
});

describe('conductorUrlForHuman', () => {
  it('binds each human to exactly their own pod', () => {
    expect(conductorUrlForHuman('human-matthew-manager', ALPHA_URLS)).toContain(
      'elohim-matthew-alpha'
    );
    expect(conductorUrlForHuman('human-jessica-spouse', ALPHA_URLS)).toContain(
      'elohim-jessica-alpha'
    );
  });

  it('returns null for humans with no deployed conductor (eve/pete/terrance)', () => {
    expect(conductorUrlForHuman('human-eve-firstwoman', ALPHA_URLS)).toBeNull();
    expect(conductorUrlForHuman('human-pete-pastor', ALPHA_URLS)).toBeNull();
    expect(conductorUrlForHuman('human-james-son', ALPHA_URLS)).toBeNull();
  });

  it('does not cross-match on name prefixes', () => {
    const urls = ['ws://elohim-jess-alpha.svc:4445'];
    expect(conductorUrlForHuman('human-jessica-spouse', urls)).toBeNull();
  });
});

describe('extractHumanId', () => {
  it('reads the flat HumanOutput shape', () => {
    expect(extractHumanId({ id: 'human-adam-firstman', display_name: 'Adam' })).toBe(
      'human-adam-firstman'
    );
  });

  it('reads the wrapped { human: { id } } shape', () => {
    expect(extractHumanId({ human: { id: 'human-matthew-manager' } })).toBe(
      'human-matthew-manager'
    );
  });

  it('returns undefined for null / empty / malformed results', () => {
    expect(extractHumanId(null)).toBeUndefined();
    expect(extractHumanId(undefined)).toBeUndefined();
    expect(extractHumanId({})).toBeUndefined();
    expect(extractHumanId({ human: null })).toBeUndefined();
    expect(extractHumanId({ human: { id: 42 } })).toBeUndefined();
    expect(extractHumanId('human-adam-firstman')).toBeUndefined();
  });
});
