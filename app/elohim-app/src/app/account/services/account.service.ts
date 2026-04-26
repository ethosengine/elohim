/**
 * Account Service — wraps GET /api/v1/account.
 *
 * Fetches the aggregate AccountView for the authenticated human and exposes it
 * as reactive signals. Thin UI binding: it shapes what the account pillar
 * displays; domain truth lives in elohim-storage.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal, computed } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import type { AccountServiceState } from '../models/account.model';
import type { AccountView } from '@app/generated/account-view';

@Injectable({ providedIn: 'root' })
export class AccountService {
  private readonly http = inject(HttpClient);

  private readonly state = signal<AccountServiceState>({
    account: null,
    loading: false,
    error: null,
  });

  // ---------------------------------------------------------------------------
  // Public signals (read-only projections)
  // ---------------------------------------------------------------------------

  readonly account = computed(() => this.state().account);
  readonly loading = computed(() => this.state().loading);
  readonly error = computed(() => this.state().error);

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  /**
   * Refresh the AccountView from the backend.
   * Called by the accountGuard on every route activation.
   */
  async refresh(): Promise<void> {
    this.state.update(s => ({ ...s, loading: true, error: null }));
    try {
      const account = await firstValueFrom(this.http.get<AccountView>('/api/v1/account'));
      this.state.set({ account, loading: false, error: null });
    } catch (e) {
      this.state.update(s => ({
        ...s,
        loading: false,
        error: e instanceof Error ? e.message : String(e),
      }));
    }
  }

  /**
   * Clear cached account state (e.g., on logout).
   */
  clear(): void {
    this.state.set({ account: null, loading: false, error: null });
  }
}
