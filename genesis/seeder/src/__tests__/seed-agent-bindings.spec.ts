/**
 * Unit tests for the pure conductor-routing helpers in seed-agent-bindings.ts.
 *
 * These encode the elohim-genesis #1380–#1386 lesson: the bindings seeder
 * walked CONDUCTOR_URLS with first-reachable-wins, and because EVERY alpha
 * conductor installs an app whose id starts with the `elohim` prefix, the
 * walk always stopped at the first url — `elohim-adam-alpha`. Every human's
 * bindings were therefore attempted on adam's pod, so one saturated
 * conductor produced a 7/7 stage failure while jessica's and james's pods
 * answered call_zome fine in the same build.
 *
 * This is the same first-reachable-wins trap the genesis #1119 founder-FATAL
 * fix removed from seed-conductor-identities.ts; the bindings seeder was the
 * sibling that never got the backport. These tests exist so it cannot drift
 * back a second time.
 */
import { describe, expect, it } from 'vitest';

import {
  candidateConductorUrls,
  isZomeTimeout,
} from '../seed-agent-bindings.js';

const ALPHA_URLS = [
  // adam is FIRST on purpose — this is the ordering that made the bug.
  'ws://elohim-adam-alpha.elohim-alpha.svc.cluster.local:4445',
  'ws://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:4445',
  'ws://elohim-jessica-alpha.elohim-alpha.svc.cluster.local:4445',
  'ws://elohim-james-alpha.elohim-alpha.svc.cluster.local:4445',
  'ws://elohim-gertrude-alpha.elohim-alpha.svc.cluster.local:4445',
  'ws://elohim-susan-alpha.elohim-alpha.svc.cluster.local:4445',
  'ws://elohim-eve-alpha.elohim-alpha.svc.cluster.local:4445',
];

describe('candidateConductorUrls — name-affine targeting', () => {
  it('resolves each human to THEIR OWN pod, never the first url in the list', () => {
    expect(candidateConductorUrls('human-matthew-manager', ALPHA_URLS)).toEqual([
      'ws://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:4445',
    ]);
    expect(candidateConductorUrls('human-jessica-spouse', ALPHA_URLS)).toEqual([
      'ws://elohim-jessica-alpha.elohim-alpha.svc.cluster.local:4445',
    ]);
    expect(candidateConductorUrls('human-eve-firstwoman', ALPHA_URLS)).toEqual([
      'ws://elohim-eve-alpha.elohim-alpha.svc.cluster.local:4445',
    ]);
  });

  it('the regression guard: no non-adam human may resolve to adam', () => {
    const adam = 'ws://elohim-adam-alpha.elohim-alpha.svc.cluster.local:4445';
    for (const humanId of [
      'human-matthew-manager',
      'human-jessica-spouse',
      'human-james-son',
      'human-eve-firstwoman',
      'human-gertrude-grandma',
      'human-susan-household',
    ]) {
      expect(candidateConductorUrls(humanId, ALPHA_URLS)).not.toContain(adam);
    }
  });

  it('adam still resolves to adam', () => {
    expect(candidateConductorUrls('human-adam-firstman', ALPHA_URLS)).toEqual([
      'ws://elohim-adam-alpha.elohim-alpha.svc.cluster.local:4445',
    ]);
  });

  it('returns an empty list (→ skip, not failure) for a human with no pod', () => {
    // pete and terrance have no elohim-pete-*/elohim-terrance-* conductor;
    // the old walk charged them as failures against adam's pod.
    expect(candidateConductorUrls('human-pete-pastor', ALPHA_URLS)).toEqual([]);
    expect(candidateConductorUrls('human-terrance-tutor', ALPHA_URLS)).toEqual([]);
  });

  it('keeps the legacy walk for non-affine url sets (local dev)', () => {
    const localUrls = ['ws://localhost:4445'];
    expect(candidateConductorUrls('human-pete-pastor', localUrls)).toEqual(localUrls);
    expect(candidateConductorUrls('human-adam-firstman', localUrls)).toEqual(localUrls);
  });

  it('an empty CONDUCTOR_URLS yields no candidates', () => {
    expect(candidateConductorUrls('human-adam-firstman', [])).toEqual([]);
  });
});

describe('isZomeTimeout', () => {
  it('recognizes the holochain client timeout verbatim from the CI logs', () => {
    expect(isZomeTimeout('Request timed out in 60000 ms: call_zome')).toBe(true);
    expect(isZomeTimeout('desktop: Request timed out in 60000 ms: call_zome')).toBe(true);
  });

  it('does not swallow real zome errors (which must keep trying archetypes)', () => {
    expect(
      isZomeTimeout("signer mismatch: caller 'uhCAk…' does not match Agent EPR"),
    ).toBe(false);
    expect(isZomeTimeout('Admin connect failed (ws://…:4444): ECONNREFUSED')).toBe(false);
    expect(isZomeTimeout('')).toBe(false);
  });
});
