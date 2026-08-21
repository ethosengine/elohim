/**
 * Unit tests for the pure CONDUCTOR_URLS-parsing + expectedHumanId helpers in
 * enumerate-agent-peer-bindings.ts.
 *
 * Regression: this script used to `.split(',')` CONDUCTOR_URLS raw, so a
 * named entry (`matthew=ws://localhost:4445` — the local `just mesh`
 * convention seed-conductor-identities.ts/peer-id.ts::parseNamedCsv
 * introduced) was handed whole to `new URL()` downstream in toAdminUrl and
 * rejected as an Invalid URL. It also computed `expectedHumanId` against a
 * throwaway single-entry `{name: null, url}` list, which can only ever
 * match hostname-affinity — never a named entry.
 */
import { describe, expect, it } from 'vitest';

import { expectedHumanId, parseEnumerateConductorUrls } from '../enumerate-agent-peer-bindings.js';
import { parseConductorUrls } from '../seed-conductor-identities.js';

describe('parseEnumerateConductorUrls', () => {
  it('resolves named CONDUCTOR_URLS entries (local just-mesh form) without throwing', () => {
    const entries = parseEnumerateConductorUrls(
      'matthew=ws://localhost:4445,jessica=ws://localhost:4455,james=ws://localhost:4465',
    );
    expect(entries.map(e => e.url)).toEqual([
      'ws://localhost:4445',
      'ws://localhost:4455',
      'ws://localhost:4465',
    ]);
    expect(entries.map(e => e.name)).toEqual(['matthew', 'jessica', 'james']);
  });

  it('passes bare hostname-affine URLs through with name: null', () => {
    const entries = parseEnumerateConductorUrls('ws://elohim-adam-alpha:4445');
    expect(entries).toEqual([{ name: null, url: 'ws://elohim-adam-alpha:4445' }]);
  });

  it('empty string resolves to an empty list', () => {
    expect(parseEnumerateConductorUrls('')).toEqual([]);
  });
});

describe('expectedHumanId', () => {
  it('resolves a named entry to its human id', () => {
    const entries = parseConductorUrls(
      'matthew=ws://localhost:4445,jessica=ws://localhost:4455,james=ws://localhost:4465',
    );
    const humanIds = ['human-matthew-manager', 'human-jessica-spouse', 'human-james-son'];
    expect(expectedHumanId('ws://localhost:4445', humanIds, entries)).toBe(
      'human-matthew-manager',
    );
    expect(expectedHumanId('ws://localhost:4465', humanIds, entries)).toBe('human-james-son');
  });

  it('resolves a hostname-affine bare URL to its human id', () => {
    const entries = parseConductorUrls('ws://elohim-adam-alpha:4445');
    expect(expectedHumanId('ws://elohim-adam-alpha:4445', ['human-adam-node'], entries)).toBe(
      'human-adam-node',
    );
  });

  it('returns null when no human matches the url', () => {
    const entries = parseConductorUrls('matthew=ws://localhost:4445');
    expect(expectedHumanId('ws://localhost:4445', ['human-jessica-spouse'], entries)).toBeNull();
  });
});
