import { InjectionToken } from '@angular/core';
import { signAccountBind } from 'elohim-imagodei';
import { createIdentityStandingClient } from 'elohim-imagodei/identity-standing';
import { sessionKeys } from 'elohim-imagodei/session-key';
import { portalBase } from '../device-approval/device-approval-port';
import { signInUrl } from '../services/portal-return';

export const ACCOUNT_BIND_PORT = new InjectionToken('ACCOUNT_BIND_PORT', {
  providedIn: 'root',
  factory: () => ({
    standing: () => createIdentityStandingClient().standing(),
    hasSessionKey: async () => !!(await sessionKeys().current()),
    sign: (challenge: string) =>
      signAccountBind({ prove: () => sessionKeys().current() }, challenge),
    signIn: () => {
      const { pathname, search } = globalThis.location;
      globalThis.location.assign(signInUrl(portalBase(), pathname + search));
    },
    navigate: (url: string) => globalThis.location.assign(url),
  }),
});
