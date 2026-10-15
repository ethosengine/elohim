import { describe, expect, it } from 'vitest';

import { legacyResourceRedirect } from './legacy-resource-redirect';

function redirect(id: string, fragment: string | null, queryParams = {}): string {
  return legacyResourceRedirect({ params: { resourceId: id }, queryParams, fragment });
}

describe('legacyResourceRedirect', () => {
  it('mints the universal address for a bare legacy link', () => {
    expect(redirect('evolution-of-trust', null)).toBe('/epr/evolution-of-trust');
  });

  it("maps the retired viewer's tab fragments onto the legs that carry them", () => {
    expect(redirect('evolution-of-trust', 'network')).toBe('/epr/evolution-of-trust#holds');
    expect(redirect('evolution-of-trust', 'governance')).toBe('/epr/evolution-of-trust#governed');
    expect(redirect('evolution-of-trust', 'trust')).toBe('/epr/evolution-of-trust#from');
    expect(redirect('evolution-of-trust', 'content')).toBe('/epr/evolution-of-trust');
  });

  it('passes leg anchors through and drops fragments that name no leg', () => {
    for (const leg of ['holds', 'lives', 'governed', 'from', 'here']) {
      expect(redirect('succession', leg)).toBe(`/epr/succession#${leg}`);
    }
    expect(redirect('succession', 'step/3')).toBe('/epr/succession');
  });

  it('keeps query params and encodes the id', () => {
    expect(redirect('a b', 'network', { via: 'alpha', tag: ['x', 'y'] })).toBe(
      '/epr/a%20b?via=alpha&tag=x&tag=y#holds'
    );
  });
});
