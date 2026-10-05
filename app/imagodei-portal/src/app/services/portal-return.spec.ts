import { describe, expect, it } from 'vitest';

import { RETURN_TO_PARAM, safePortalReturn, signInUrl } from './portal-return.js';

const BASE = '/auth/portal/';

describe('safePortalReturn — where sign-in may bring a person back to', () => {
  it('accepts a page of this portal', () => {
    const page = '/auth/portal/consent/device?request=eyJhIjoxfQ';
    expect(safePortalReturn(page, BASE)).toBe(page);
  });

  it.each([
    ['another origin', 'https://evil.example/auth/portal/'],
    ['a protocol-relative host', '//evil.example/auth/portal/'],
    ['a backslash host', '/\\evil.example'],
    ['a page outside the portal', '/admin'],
    ['a relative path', 'consent/device'],
    ['nothing', null],
  ])('refuses %s', (_label, value) => {
    expect(safePortalReturn(value, BASE)).toBeNull();
  });
});

describe('signInUrl', () => {
  it('carries the page to come back to, encoded', () => {
    const here = '/auth/portal/consent/device?request=a&b=c';
    const url = signInUrl(BASE, here);
    expect(url.startsWith(`${BASE}?${RETURN_TO_PARAM}=`)).toBe(true);
    expect(new URLSearchParams(url.slice(url.indexOf('?'))).get(RETURN_TO_PARAM)).toBe(here);
  });
});
