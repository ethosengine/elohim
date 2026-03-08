/**
 * IdentityApiService — Data boundary for human identity operations.
 *
 * Transitional implementation: wraps Holochain zome calls to imagodei DNA.
 * Target state: thin HTTP client calling /api/v1/identity/* via doorway,
 * matching the pattern established by PresenceApiService and StewardshipApiService.
 *
 * The Holochain DNA retains its role for cryptographic provenance (attestations,
 * agent keys, immutable proofs). Mutable profile data (display name, bio, etc.)
 * will move to elohim-storage's Diesel layer.
 */

import { Injectable, inject } from '@angular/core';

import { HolochainClientService } from '../../elohim/services/holochain-client.service';

import type { IIdentityApi } from '../interfaces/identity.interface';
import type {
  HumanSessionResult,
  HumanUpdateResult,
  RegisterHumanPayload,
  UpdateHumanPayload,
} from '../models/identity.model';

@Injectable({ providedIn: 'root' })
export class IdentityApiService implements IIdentityApi {
  private readonly holochainClient = inject(HolochainClientService);

  async createHuman(payload: RegisterHumanPayload): Promise<HumanSessionResult> {
    const result = await this.holochainClient.callZome<HumanSessionResult>({
      zomeName: 'imagodei',
      fnName: 'create_human',
      payload,
      roleName: 'imagodei',
    });

    if (!result.success || !result.data) {
      throw new Error(result.error ?? 'Failed to create human identity');
    }

    return result.data;
  }

  async getMyHuman(): Promise<HumanSessionResult | null> {
    const result = await this.holochainClient.callZome<HumanSessionResult | null>({
      zomeName: 'imagodei',
      fnName: 'get_my_human',
      payload: null,
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return result.data;
    }

    return null;
  }

  async updateHuman(payload: UpdateHumanPayload): Promise<HumanUpdateResult> {
    const result = await this.holochainClient.callZome<HumanUpdateResult>({
      zomeName: 'imagodei',
      fnName: 'update_human',
      payload,
      roleName: 'imagodei',
    });

    if (!result.success || !result.data) {
      throw new Error(result.error ?? 'Failed to update human profile');
    }

    return result.data;
  }
}
