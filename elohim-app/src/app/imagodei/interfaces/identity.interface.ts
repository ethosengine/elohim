/**
 * IIdentityApi — Abstract interface for human identity data operations.
 *
 * Decouples identity CRUD from the transport layer (zome calls vs HTTP).
 * The default factory resolves to IdentityApiService which currently wraps
 * Holochain zome calls. When elohim-storage exposes /api/v1/identity endpoints,
 * the implementation switches to thin HTTP calls.
 *
 * Consumers inject the IDENTITY_API token; tests provide a mock.
 *
 * @example
 * ```typescript
 * export class SomeService {
 *   private readonly identityApi = inject(IDENTITY_API);
 *
 *   async loadProfile(): Promise<HumanProfile | null> {
 *     const result = await this.identityApi.getMyHuman();
 *     return result ? mapToProfile(result.human) : null;
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { IdentityApiService } from '../services/identity-api.service';

import type {
  HumanSessionResult,
  HumanUpdateResult,
  RegisterHumanPayload,
  UpdateHumanPayload,
} from '../models/identity.model';

/**
 * Abstract identity data operations — CRUD for human profiles.
 *
 * Implementations handle the transport (zome calls or HTTP) and wire format.
 * The identity state machine and auth orchestration live in IdentityService,
 * not here.
 */
export interface IIdentityApi {
  /** Create a new human identity */
  createHuman(payload: RegisterHumanPayload): Promise<HumanSessionResult>;

  /** Get the current human's identity and attestations */
  getMyHuman(): Promise<HumanSessionResult | null>;

  /** Update the current human's profile fields */
  updateHuman(payload: UpdateHumanPayload): Promise<HumanUpdateResult>;
}

/**
 * Injection token for the identity data API.
 *
 * Default factory resolves to IdentityApiService (zome calls, transitional).
 * Will switch to HTTP client when /api/v1/identity endpoints are ready.
 *
 * Override in tests:
 * ```typescript
 * { provide: IDENTITY_API, useValue: mockIdentityApi }
 * ```
 */
export const IDENTITY_API = new InjectionToken<IIdentityApi>('IdentityApi', {
  providedIn: 'root',
  factory: () => inject(IdentityApiService),
});
