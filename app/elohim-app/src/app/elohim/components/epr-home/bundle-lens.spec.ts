import { describe, expect, it } from 'vitest';

import { openInBundle } from './bundle-lens';

const claims = [
  { bundle: 'lamad', claims: [{ contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } }] },
];

describe('openInBundle', () => {
  it('mints the claiming bundle mount for a claimed type', () => {
    expect(openInBundle('path', 'foundations-christian-technology', claims, { lamad: '/lamad' })).toEqual({
      href: '/lamad/path/foundations-christian-technology',
      bundleName: 'Lamad',
    });
  });

  it('returns null for an unclaimed type', () => {
    expect(openInBundle('collective', 'evolution-of-trust', claims, { lamad: '/lamad' })).toBeNull();
  });

  it('encodes the id', () => {
    expect(openInBundle('path', 'a b', claims, { lamad: '/lamad' })?.href).toBe('/lamad/path/a%20b');
  });
});
