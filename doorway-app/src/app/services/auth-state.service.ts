import { Injectable, inject, signal, computed } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Router } from '@angular/router';
import { firstValueFrom } from 'rxjs';

import { DoorwayAdminService } from './doorway-admin.service';
import { AccountResponse } from '../models/doorway.model';
import { environment } from '../../environments/environment';

const AUTH_TOKEN_KEY = 'doorway_auth_token';

@Injectable({ providedIn: 'root' })
export class AuthStateService {
  private readonly adminService = inject(DoorwayAdminService);
  private readonly http = inject(HttpClient);
  private readonly router = inject(Router);

  private readonly baseUrl = environment.doorwayUrl ?? '';

  private readonly _account = signal<AccountResponse | null>(null);
  private readonly _isLoading = signal(true);

  readonly account = this._account.asReadonly();
  readonly isLoading = this._isLoading.asReadonly();
  readonly isAuthenticated = computed(() => this._account() !== null);
  readonly isSteward = computed(() => this._account()?.isSteward ?? false);
  readonly modeLabel = computed<'Steward' | 'Hosted'>(() =>
    this._account()?.isSteward ? 'Steward' : 'Hosted'
  );
  readonly initials = computed(() => {
    const id = this._account()?.identifier;
    return id ? id.charAt(0).toUpperCase() : '?';
  });

  async init(): Promise<void> {
    this._isLoading.set(true);
    try {
      // Check for session transfer token in URL (cross-app handoff)
      await this.handleSessionTokenIfPresent();

      const account = await firstValueFrom(this.adminService.getAccount());
      this._account.set(account);
    } finally {
      this._isLoading.set(false);
    }
  }

  async refresh(): Promise<void> {
    const account = await firstValueFrom(this.adminService.getAccount());
    this._account.set(account);
  }

  async logout(): Promise<void> {
    await firstValueFrom(this.adminService.logout());
    localStorage.removeItem(AUTH_TOKEN_KEY);
    this._account.set(null);
    this.router.navigate(['/']);
  }

  /**
   * Store a JWT token (used after login or session exchange).
   */
  storeToken(token: string): void {
    localStorage.setItem(AUTH_TOKEN_KEY, token);
  }

  /**
   * Check if a session_token query parameter is present in the URL.
   * If so, exchange it for a JWT and store it. This enables seamless
   * navigation from elohim-app to doorway-app without re-login.
   */
  private async handleSessionTokenIfPresent(): Promise<void> {
    const params = new URLSearchParams(window.location.search);
    const sessionToken = params.get('session_token');
    if (!sessionToken) return;

    // Clean the URL to remove the session_token param
    params.delete('session_token');
    const cleanSearch = params.toString();
    const cleanUrl =
      window.location.pathname + (cleanSearch ? `?${cleanSearch}` : '') + window.location.hash;
    window.history.replaceState({}, '', cleanUrl);

    try {
      const response = await firstValueFrom(
        this.http.get<{ token: string }>(
          `${this.baseUrl}/auth/exchange-session?session_token=${encodeURIComponent(sessionToken)}`
        )
      );
      if (response?.token) {
        this.storeToken(response.token);
      }
    } catch {
      // Exchange failed (expired, consumed, or invalid token).
      // User will see the unauthenticated state and can log in manually.
    }
  }
}
