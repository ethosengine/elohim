import { describe, expect, it } from 'vitest';

import { BUNDLE_EDITORS, editInBundle, openInBundle } from './bundle-lens';

const claims = [
  {
    bundle: 'lamad',
    claims: [
      { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
    ],
  },
];

describe('openInBundle', () => {
  it('mints the claiming bundle mount for a claimed type', () => {
    expect(
      openInBundle('path', 'foundations-christian-technology', claims, { lamad: '/lamad' })
    ).toEqual({
      href: '/lamad/path/foundations-christian-technology',
      bundleName: 'Lamad',
    });
  });

  it('returns null for an unclaimed type', () => {
    expect(
      openInBundle('collective', 'evolution-of-trust', claims, { lamad: '/lamad' })
    ).toBeNull();
  });

  it('encodes the id', () => {
    expect(openInBundle('path', 'a b', claims, { lamad: '/lamad' })?.href).toBe(
      '/lamad/path/a%20b'
    );
  });
});

describe('editInBundle', () => {
  it("mints the editing bundle's editor under its mount", () => {
    expect(editInBundle('evolution-of-trust')).toEqual({
      href: '/lamad/resource/evolution-of-trust/edit',
      bundleName: 'Lamad',
    });
    expect(editInBundle('a b', BUNDLE_EDITORS, { lamad: '/learn' })?.href).toBe(
      '/learn/resource/a%20b/edit'
    );
  });

  it('never offers an editor for a generated path-step id', () => {
    expect(editInBundle('path-foundations-step-3')).toBeNull();
    expect(editInBundle('path-elohim-protocol-step-12')).toBeNull();
    expect(editInBundle('path-overview')).not.toBeNull();
  });

  it('offers nothing with no editor declared', () => {
    expect(editInBundle('evolution-of-trust', {})).toBeNull();
  });
});
