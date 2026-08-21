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
 *
 * And the local-mesh regression this file adds coverage for: on a loopback
 * CONDUCTOR_URLS list (`ws://localhost:4445,...`), NONE of the URLs are
 * hostname-affine (no `elohim-<name>-` segment), so the legacy
 * first-reachable-wins walk applied to EVERY human — including ones with no
 * conductor in the list at all (adam) — and cast them onto whichever pod
 * answered first. The fix is the `name=url` CONDUCTOR_URLS convention
 * (`parseConductorUrls`): once ANY entry carries an explicit name, the whole
 * list is name-affine and a human with no matching name is SKIPPED, never
 * cast onto someone else's conductor.
 */
import { describe, expect, it } from 'vitest';

import {
  conductorUrlForHuman,
  extractHumanId,
  humanShortName,
  parseConductorUrls,
  resolveCandidateUrls,
  urlsAreNameAffine,
} from '../seed-conductor-identities.js';

const ALPHA_URLS = parseConductorUrls(
  [
    'ws://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:4445',
    'ws://elohim-adam-alpha.elohim-alpha.svc.cluster.local:4445',
    'ws://elohim-jessica-alpha.elohim-alpha.svc.cluster.local:4445',
  ].join(',')
);

describe('humanShortName', () => {
  it('extracts the first token after the human- prefix (Jenkinsfile rule)', () => {
    expect(humanShortName('human-matthew-manager')).toBe('matthew');
    expect(humanShortName('human-adam-firstman')).toBe('adam');
    expect(humanShortName('human-jessica-spouse')).toBe('jessica');
    expect(humanShortName('human-james-son')).toBe('james');
  });
});

describe('parseConductorUrls', () => {
  it('parses unnamed (bare URL) entries — today\'s behaviour', () => {
    expect(parseConductorUrls('ws://localhost:4445,ws://localhost:4455')).toEqual([
      { name: null, url: 'ws://localhost:4445' },
      { name: null, url: 'ws://localhost:4455' },
    ]);
  });

  it('parses named `name=url` entries — same convention as PEER_STORAGE_URLS', () => {
    expect(
      parseConductorUrls('matthew=ws://localhost:4445,jessica=ws://localhost:4455')
    ).toEqual([
      { name: 'matthew', url: 'ws://localhost:4445' },
      { name: 'jessica', url: 'ws://localhost:4455' },
    ]);
  });

  it('allows a mixed list of named and unnamed entries', () => {
    expect(parseConductorUrls('matthew=ws://localhost:4445,ws://localhost:4455')).toEqual([
      { name: 'matthew', url: 'ws://localhost:4445' },
      { name: null, url: 'ws://localhost:4455' },
    ]);
  });

  it('trims whitespace and drops empty entries', () => {
    expect(parseConductorUrls(' matthew=ws://localhost:4445 , , ws://localhost:4455 ')).toEqual([
      { name: 'matthew', url: 'ws://localhost:4445' },
      { name: null, url: 'ws://localhost:4455' },
    ]);
  });

  it('returns an empty list for an empty string', () => {
    expect(parseConductorUrls('')).toEqual([]);
  });
});

describe('urlsAreNameAffine', () => {
  it('recognizes the elohim-<name>-<env> service convention (hostname-affine)', () => {
    expect(urlsAreNameAffine(ALPHA_URLS)).toBe(true);
  });

  it('rejects fully unnamed local-dev URL sets (legacy walk applies there)', () => {
    expect(urlsAreNameAffine(parseConductorUrls('ws://localhost:4445'))).toBe(false);
    expect(urlsAreNameAffine([])).toBe(false);
  });

  it('treats an explicit name=url entry as name-affine even on loopback URLs', () => {
    expect(urlsAreNameAffine(parseConductorUrls('matthew=ws://localhost:4445'))).toBe(true);
  });

  it('is affine when even ONE entry in a mixed list carries a name', () => {
    expect(
      urlsAreNameAffine(parseConductorUrls('matthew=ws://localhost:4445,ws://localhost:4455'))
    ).toBe(true);
  });
});

describe('conductorUrlForHuman', () => {
  it('binds each human to exactly their own pod (hostname convention)', () => {
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

  it('does not cross-match on name prefixes (hostname convention)', () => {
    const urls = parseConductorUrls('ws://elohim-jess-alpha.svc:4445');
    expect(conductorUrlForHuman('human-jessica-spouse', urls)).toBeNull();
  });

  it('binds each human to their own pod via an explicit name=url entry (local mesh)', () => {
    const mesh = parseConductorUrls(
      'matthew=ws://localhost:4445,jessica=ws://localhost:4455,james=ws://localhost:4465'
    );
    expect(conductorUrlForHuman('human-matthew-manager', mesh)).toBe('ws://localhost:4445');
    expect(conductorUrlForHuman('human-jessica-spouse', mesh)).toBe('ws://localhost:4455');
    expect(conductorUrlForHuman('human-james-son', mesh)).toBe('ws://localhost:4465');
  });

  it('returns null for a human with no matching name in a named local-mesh list (adam)', () => {
    const mesh = parseConductorUrls(
      'matthew=ws://localhost:4445,jessica=ws://localhost:4455,james=ws://localhost:4465'
    );
    expect(conductorUrlForHuman('human-adam-firstman', mesh)).toBeNull();
  });

  it('does not cross-match named entries on short-name prefixes', () => {
    const mesh = parseConductorUrls('jess=ws://localhost:4455');
    expect(conductorUrlForHuman('human-jessica-spouse', mesh)).toBeNull();
  });
});

describe('resolveCandidateUrls', () => {
  it('narrows to exactly the human\'s own URL when the list is name-affine', () => {
    const mesh = parseConductorUrls(
      'matthew=ws://localhost:4445,jessica=ws://localhost:4455,james=ws://localhost:4465'
    );
    expect(resolveCandidateUrls('human-matthew-manager', mesh)).toEqual(['ws://localhost:4445']);
  });

  it('the genesis #1119 / local-mesh regression: NEVER falls back to the full list for a human with no matching entry, even though the list has affine entries elsewhere (adam on the local mesh)', () => {
    const mesh = parseConductorUrls(
      'matthew=ws://localhost:4445,jessica=ws://localhost:4455,james=ws://localhost:4465'
    );
    expect(resolveCandidateUrls('human-adam-firstman', mesh)).toEqual([]);
  });

  it('walks the FULL list first-reachable-wins only when NO entry in the list is named or hostname-affine', () => {
    const mesh = parseConductorUrls('ws://localhost:4445,ws://localhost:4455,ws://localhost:4465');
    expect(resolveCandidateUrls('human-adam-firstman', mesh)).toEqual([
      'ws://localhost:4445',
      'ws://localhost:4455',
      'ws://localhost:4465',
    ]);
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
