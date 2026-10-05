/**
 * How the native portal's device approval page reaches the node that holds
 * the person's key — this node, the one serving the portal — and how it
 * leaves for sign-in or for a terminal. The page itself is the shared piece in
 * `elohim-imagodei/device-consent`; this file supplies only what is this
 * host's own: its session is the node's cookie, and sign-in is this portal's
 * own first page.
 *
 * Injected so specs can answer the calls without a network.
 */

import { InjectionToken } from '@angular/core';
import {
  createDeviceConsentClient,
  type DeviceConsentClient,
} from 'elohim-imagodei/device-consent';

import { signInUrl } from '../services/portal-return.js';

export interface DeviceApprovalPort {
  client: DeviceConsentClient;
  /** Send the person to this portal's sign-in and straight back to this page. */
  signIn(): void;
  /** Hand the code to the asking terminal's listener on this machine (top-level navigation). */
  handBack(url: string): void;
}

/** The portal's own base path (its `<base href>`), e.g. `/auth/portal/`. */
export function portalBase(): string {
  return new URL('.', globalThis.document.baseURI).pathname;
}

export const DEVICE_APPROVAL_PORT = new InjectionToken<DeviceApprovalPort>('DEVICE_APPROVAL_PORT', {
  providedIn: 'root',
  factory: () => ({
    client: createDeviceConsentClient(),
    signIn: () => {
      const { pathname, search } = globalThis.location;
      globalThis.location.assign(signInUrl(portalBase(), pathname + search));
    },
    handBack: url => globalThis.location.assign(url),
  }),
});
