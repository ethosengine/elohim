/**
 * Device approval wire calls — the one place the portal talks to
 * /auth/consent/*. Typed request and response, no other logic: when the
 * backend changes, this file is where it changes.
 *
 * Status of the backend (keep in view when reading the types):
 * - POST /auth/consent/view — exists in the peer runtime
 *   (elohim-storage services/device_consent.rs, ConsentView); the doorway
 *   does not mount it yet.
 * - POST /auth/consent/agree — proposed shape; not implemented anywhere yet.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { firstValueFrom } from 'rxjs';

import type { WitnessStep } from '../models/witness-step';

/** The two acts a device can ask its person to agree to. */
export type DeviceAct = 'device.enroll' | 'device.bind-root';

/**
 * The terminal's GrantRequest, exactly as its link carried it. The portal
 * passes it back unchanged and never reads its fields.
 */
export type GrantRequestJson = Readonly<Record<string, unknown>>;

/** 200 from POST /auth/consent/view (consent_grant::ConsentView). */
export interface ConsentViewResponse {
  clientId: string;
  label: string;
  deviceFingerprint: string;
  deviceRootFingerprint?: string;
  askedActs: DeviceAct[];
}

/** Body of POST /auth/consent/agree. */
export interface ConsentAgreeRequest {
  request: GrantRequestJson;
  agreedActs: DeviceAct[];
}

/** Where the one-time code goes: shown to paste, or handed to a terminal on this machine. */
export type ConsentReturnTarget =
  | { kind: 'display'; value: string }
  | { kind: 'redirect'; url: string };

/** 200 from POST /auth/consent/agree. `expiresAt` is epoch milliseconds. */
export interface ConsentAgreeResponse {
  returnTarget: ConsentReturnTarget;
  expiresAt: number;
  /**
   * PROPOSED, optional — no backend sends it yet. Who else took part in
   * securing this approval (the device's own signature, peers that recorded
   * it, …), as the runtime observed them. The portal shows them after its own
   * "your doorway signed this" step, unchanged and in the order given.
   */
  witnesses?: WitnessStep[];
}

/** 4xx body from either call. */
export interface ConsentRefusalBody {
  error: string;
  code: string;
}

@Injectable({ providedIn: 'root' })
export class DeviceConsentService {
  private readonly http = inject(HttpClient);

  /** What the device is asking for, or a 4xx {@link ConsentRefusalBody}. */
  view(request: GrantRequestJson): Promise<ConsentViewResponse> {
    return firstValueFrom(this.http.post<ConsentViewResponse>('/auth/consent/view', request));
  }

  /** Sign the agreement as the signed-in person and get the code's destination. */
  agree(body: ConsentAgreeRequest): Promise<ConsentAgreeResponse> {
    return firstValueFrom(this.http.post<ConsentAgreeResponse>('/auth/consent/agree', body));
  }

  /** Hand the code to the asking terminal's listener on this machine (top-level navigation). */
  handBack(url: string): void {
    globalThis.location.assign(url);
  }
}
