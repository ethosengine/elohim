/**
 * How this doorway's device approval page reaches the node that holds the
 * person's key, and leaves for a terminal. The page itself — wire, rules,
 * memory, sequencing — is the shared piece in `elohim-imagodei/device-consent`,
 * which both sign-in portals mount; this file supplies only what is the
 * doorway's own: its session travels as a bearer token, not a cookie.
 *
 * Injected so specs can answer the calls without a network.
 */

import { InjectionToken, inject } from '@angular/core';

import {
  createDeviceConsentClient,
  type DeviceConsentClient,
} from 'elohim-imagodei/device-consent';

import { DoorwaySessionTokenStore } from './doorway-session-token.store';

export interface DeviceConsentPort {
  client: DeviceConsentClient;
  /** Hand the code to the asking terminal's listener on this machine (top-level navigation). */
  handBack(url: string): void;
}

export const DEVICE_CONSENT_PORT = new InjectionToken<DeviceConsentPort>('DEVICE_CONSENT_PORT', {
  providedIn: 'root',
  factory: () => {
    const tokens = inject(DoorwaySessionTokenStore);
    return {
      client: createDeviceConsentClient({
        headers: (): Record<string, string> => {
          const token = tokens.get()?.token;
          return token ? { Authorization: `Bearer ${token}` } : {};
        },
      }),
      handBack: url => globalThis.location.assign(url),
    };
  },
});
