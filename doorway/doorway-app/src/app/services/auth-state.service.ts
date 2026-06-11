import { Injectable, inject, signal, computed } from '@angular/core';
import { Router } from '@angular/router';
import { firstValueFrom } from 'rxjs';

import { DoorwaySessionClient } from '@elohim/identity';

import { DoorwayAdminService } from './doorway-admin.service';
import { DoorwaySessionTokenStore } from './doorway-session-token.store';
import { AccountResponse } from '../models/doorway.model';
import { environment } from '../../environments/environment';

/**
 * Auth display state for the operator dashboard.
 *
 * Edge vs walking split (auth-wire plan 2026-06-10, Task 3):
 * - EDGE (stays here): the account signal + display computeds
 *   (isSteward/modeLabel/initials), the URL session_token detection and
 *   history cleanup, the GET /auth/account probe (an account-surface view,
 *   not part of the session client's wire surface), and router navigation.
 * - WALKING (delegated): GET /auth/exchange-session and POST /auth/logout go
 *   through @elohim/identity's DoorwaySessionClient — the consolidated,
 *   schema-contract-pinned auth wire surface. All token persistence goes
 *   through DoorwaySessionTokenStore (the SAME legacy `doorway_auth_token`
 *   raw-JWT localStorage key the auth interceptor reads).
 */
@Injectable({ providedIn: 'root' })
export class AuthStateService {
  private readonly adminService = inject(DoorwayAdminService);
  private readonly router = inject(Router);

  /** Session persistence — same legacy localStorage key as always. */
  private readonly tokenStore = inject(DoorwaySessionTokenStore);

  private readonly baseUrl = environment.doorwayUrl ?? '';

  /**
   * Doorway session wire walking. Bearer comes from the token store; these
   * fetch calls do not pass through the Angular auth interceptor.
   */
  private readonly sessionClient = new DoorwaySessionClient({
    baseUrl: this.baseUrl,
    tokenStore: this.tokenStore,
  });

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
      // Check for session transfer token in URL (cross-app handoff).
      // This may store a token before we check below.
      await this.handleSessionTokenIfPresent();

      // Anonymous-page guard: if no stored token exists at this point, there is
      // no credential to send and the server will 401. Probing unconditionally
      // produced a console-visible "Failed to load resource: 401" on anonymous
      // pages such as /threshold/login even though catchError swallowed the
      // RxJS error — the browser network layer logs the response before RxJS
      // processes it. Skip the probe and resolve as unauthenticated immediately.
      if (!this.sessionClient.token) {
        this._account.set(null);
        return;
      }

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
    // Clear local state FIRST to ensure UX responsiveness (pre-clear, then
    // server courtesy call). The stored token is cleared by the client's finally
    // block below, but clearing local signals before the network call ensures
    // the UI is responsive even if the server logout hangs.
    this._account.set(null);
    this.router.navigate(['/']);

    // Fire the server logout with a 30-second timeout. The server logout is
    // a stateless courtesy per the client docstring — its failure or timeout
    // must not block UX. The session client clears the stored token in its
    // finally block even if the request fails.
    try {
      await Promise.race([
        this.sessionClient.logout().catch(() => undefined),
        new Promise(res => setTimeout(res, 30_000))
      ]);
    } catch {
      // Race never rejects, but catch for safety.
    }
  }

  /**
   * Store a JWT token (used after login or session exchange).
   */
  storeToken(token: string): void {
    this.tokenStore.setToken(token);
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
      // Walks GET /auth/exchange-session and persists the resulting session
      // via the token store (same legacy localStorage key).
      await this.sessionClient.exchangeSessionToken(sessionToken);
    } catch {
      // Exchange failed (expired, consumed, or invalid token).
      // User will see the unauthenticated state and can log in manually.
    }
  }
}
