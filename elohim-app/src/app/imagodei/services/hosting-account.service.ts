/**
 * Hosting Account Service
 *
 * Fetches the hosted human's account details from the doorway's
 * GET /auth/account endpoint. Provides resource usage signals
 * for display in the profile hosting section.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { AuthService } from './auth.service';
import { DoorwayRegistryService } from './doorway-registry.service';

import type { HostingAccount } from '../models/hosting-account.model';

@Injectable({ providedIn: 'root' })
export class HostingAccountService {
  // ===========================================================================
  // Dependencies
  // ===========================================================================

  private readonly http = inject(HttpClient);
  private readonly authService = inject(AuthService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);

  // ===========================================================================
  // State
  // ===========================================================================

  private readonly accountSignal = signal<HostingAccount | null>(null);
  private readonly loadingSignal = signal(false);
  private readonly errorSignal = signal<string | null>(null);

  // ===========================================================================
  // Public Signals (read-only)
  // ===========================================================================

  readonly account = this.accountSignal.asReadonly();
  readonly isLoading = this.loadingSignal.asReadonly();
  readonly error = this.errorSignal.asReadonly();

  // ===========================================================================
  // Operations
  // ===========================================================================

  async loadAccount(): Promise<HostingAccount | null> {
    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    const token = this.authService.token();

    if (!doorwayUrl || !token) {
      this.errorSignal.set('Not authenticated or no doorway selected');
      return null;
    }

    this.loadingSignal.set(true);
    this.errorSignal.set(null);

    try {
      const account = await firstValueFrom(
        this.http.get<HostingAccount>(`${doorwayUrl}/auth/account`, {
          headers: {
            Authorization: `Bearer ${token}`,
          },
        })
      );

      this.accountSignal.set(account);
      return account;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load hosting account';
      this.errorSignal.set(message);
      return null;
    } finally {
      this.loadingSignal.set(false);
    }
  }

  clearAccount(): void {
    this.accountSignal.set(null);
    this.errorSignal.set(null);
  }
}
