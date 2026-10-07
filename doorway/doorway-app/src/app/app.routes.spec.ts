import { describe, expect, it } from 'vitest';

import { routes } from './app.routes';
import { authGuard } from './core/guards/auth.guard';

const route = (path: string) => routes.find(r => r.path === path);

describe('app routes — where sign-in is asked for', () => {
  it('lets the device approval page ask the doorway before any sign-in redirect', () => {
    // The page probes `POST /auth/consent/view` first and orders sign-in
    // itself; a route guard here would send a person through sign-in to a
    // page that then cannot approve.
    expect(route('consent/device')?.canActivate).toBeUndefined();
  });

  it.each(['dashboard', 'account'])('keeps %s behind sign-in', path => {
    expect(route(path)?.canActivate).toEqual([authGuard]);
  });
});
