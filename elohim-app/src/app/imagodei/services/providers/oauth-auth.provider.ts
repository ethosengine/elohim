/**
 * OAuth Authentication Provider.
 *
 * Implements OAuth 2.0 Authorization Code flow for doorway authentication.
 * Used when users click a doorway in the picker to sign in.
 *
 * Flow:
 * 1. User clicks doorway in DoorwayPickerComponent
 * 2. initiateLogin() redirects browser to doorway's /auth/authorize
 * 3. User authenticates on doorway's /threshold/login page
 * 4. Doorway redirects back with authorization code
 * 5. handleCallback() exchanges code for JWT token
 *
 * This enables the thin-federated architecture where elohim-app
 * works with ANY doorway (like choosing a Mastodon instance).
 */

import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject, signal } from '@angular/core';

// @coverage: 61.1% (2026-02-05)

import { firstValueFrom } from 'rxjs';

import {
  type AuthProvider,
  type AuthCredentials,
  type AuthResult,
  type AuthFailure,
  type OAuthCredentials,
} from '../../models/auth.model';
import { DoorwayRegistryService } from '../doorway-registry.service';

// =============================================================================
// OAuth Types
// =============================================================================

/** OAuth state stored in sessionStorage for CSRF protection */
interface OAuthState {
  state: string;
  doorwayUrl: string;
  redirectUri: string;
  codeVerifier?: string; // For PKCE (future)
  timestamp: number;
}

/** Response from POST /auth/token */
interface OAuthTokenResponse {
  accessToken: string;
  tokenType: string;
  expiresIn: number;
  refreshToken?: string;
  humanId: string;
  agentPubKey: string;
  identifier: string;
  doorwayId?: string;
  doorwayUrl?: string;
}

/** Error response from OAuth endpoints */
interface OAuthErrorResponse {
  error: string;
  errorDescription?: string;
  state?: string;
}

// Storage key for OAuth state
const OAUTH_STATE_KEY = 'elohim-oauth-state';

// =============================================================================
// Provider Implementation
// =============================================================================

@Injectable({ providedIn: 'root' })
export class OAuthAuthProvider implements AuthProvider {
  readonly type = 'oauth' as const;

  private readonly http = inject(HttpClient);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);

  /** Whether an OAuth flow is in progress */
  readonly isFlowInProgress = signal(false);

  /**
   * OAuth login is different - it redirects to the doorway.
   * This method handles the callback with the authorization code.
   */
  async login(credentials: AuthCredentials): Promise<AuthResult> {
    if (credentials.type !== 'oauth') {
      return {
        success: false,
        error: 'Invalid credentials type for OAuth provider',
        code: 'VALIDATION_ERROR',
      };
    }

    const oauthCreds = credentials as OAuthCredentials;

    // The 'token' field contains the authorization code for OAuth
    // This is called from the callback handler
    return await this.exchangeCodeForToken(oauthCreds.provider, oauthCreds.token);
  }

  /**
   * Initiate OAuth login by redirecting to the doorway's authorize endpoint.
   *
   * @param doorwayUrl - URL of the doorway to authenticate with
   * @param returnUrl - URL to redirect back to after auth (default: current page)
   */
  initiateLogin(doorwayUrl: string, returnUrl?: string): void {
    const state = this.generateState();
    const redirectUri = returnUrl ?? `${window.location.origin}/auth/callback`;

    // Store state for verification on callback
    const oauthState: OAuthState = {
      state,
      doorwayUrl,
      redirectUri,
      timestamp: Date.now(),
    };
    sessionStorage.setItem(OAUTH_STATE_KEY, JSON.stringify(oauthState));

    // Build authorization URL
    const params = new URLSearchParams({
      clientId: 'elohim-app',
      redirectUri: redirectUri,
      responseType: 'code',
      state,
    });

    const authorizeUrl = `${doorwayUrl}/auth/authorize?${params.toString()}`;

    this.isFlowInProgress.set(true);

    // Redirect browser to doorway's authorize endpoint
    window.location.href = authorizeUrl;
  }

  /**
   * Handle the OAuth callback with authorization code.
   * Called from the auth callback component.
   *
   * @param code - Authorization code from doorway
   * @param state - State parameter for CSRF verification
   * @returns Authentication result
   */
  async handleCallback(code: string, state: string): Promise<AuthResult> {
    // Retrieve and verify stored state
    const storedStateJson = sessionStorage.getItem(OAUTH_STATE_KEY);
    if (!storedStateJson) {
      return {
        success: false,
        error: 'OAuth session not found. Please try again.',
        code: 'VALIDATION_ERROR',
      };
    }

    let storedState: OAuthState;
    try {
      storedState = JSON.parse(storedStateJson);
    } catch {
      sessionStorage.removeItem(OAUTH_STATE_KEY);
      return {
        success: false,
        error: 'Invalid OAuth session. Please try again.',
        code: 'VALIDATION_ERROR',
      };
    }

    // Verify state matches (CSRF protection)
    if (storedState.state !== state) {
      sessionStorage.removeItem(OAUTH_STATE_KEY);
      return {
        success: false,
        error: 'OAuth state mismatch. Possible CSRF attack.',
        code: 'VALIDATION_ERROR',
      };
    }

    // Check if state is expired (10 minutes)
    const stateAge = Date.now() - storedState.timestamp;
    if (stateAge > 10 * 60 * 1000) {
      sessionStorage.removeItem(OAUTH_STATE_KEY);
      return {
        success: false,
        error: 'OAuth session expired. Please try again.',
        code: 'TOKEN_EXPIRED',
      };
    }

    // Exchange code for token
    const result = await this.exchangeCodeForToken(
      storedState.doorwayUrl,
      code,
      storedState.redirectUri
    );

    // Clear stored state on success or permanent failure
    if (result.success || result.code !== 'NETWORK_ERROR') {
      sessionStorage.removeItem(OAUTH_STATE_KEY);
    }

    this.isFlowInProgress.set(false);

    return result;
  }

  /**
   * Exchange authorization code for access token.
   */
  private async exchangeCodeForToken(
    doorwayUrl: string,
    code: string,
    redirectUri?: string
  ): Promise<AuthResult> {
    const tokenUrl = `${doorwayUrl}/auth/token`;

    const body = {
      grantType: 'authorization_code',
      code,
      redirectUri: redirectUri ?? `${globalThis.location.origin}/auth/callback`,
      clientId: 'elohim-app',
    };

    try {
      const response = await firstValueFrom(
        this.http.post<OAuthTokenResponse>(tokenUrl, body, {
          headers: { 'Content-Type': 'application/json' },
        })
      );

      // Calculate expiry time
      const expiresAt = new Date(Date.now() + response.expiresIn * 1000);

      return {
        success: true,
        token: response.accessToken,
        humanId: response.humanId,
        agentPubKey: response.agentPubKey,
        expiresAt: expiresAt.toISOString(),
        identifier: response.identifier,
      };
    } catch (err) {
      return this.handleError(err);
    }
  }

  /**
   * Logout - clear any OAuth state.
   */
  logout(): Promise<void> {
    sessionStorage.removeItem(OAUTH_STATE_KEY);
    this.isFlowInProgress.set(false);
    return Promise.resolve();
  }

  /**
   * Refresh token using refreshToken (if available).
   */
  async refreshToken(token: string): Promise<AuthResult> {
    // OAuth typically uses refreshToken grant
    // For now, we redirect back to the doorway for re-authentication
    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (!doorwayUrl) {
      return {
        success: false,
        error: 'No doorway selected for token refresh',
        code: 'NETWORK_ERROR',
      };
    }

    // Try to refresh using the doorway's refresh endpoint
    const url = `${doorwayUrl}/auth/refresh`;

    try {
      const response = await firstValueFrom(
        this.http.post<OAuthTokenResponse>(
          url,
          {},
          {
            headers: {
              'Content-Type': 'application/json',
              Authorization: `Bearer ${token}`,
            },
          }
        )
      );

      const expiresAt = new Date(Date.now() + response.expiresIn * 1000);

      return {
        success: true,
        token: response.accessToken,
        humanId: response.humanId,
        agentPubKey: response.agentPubKey,
        expiresAt: expiresAt.toISOString(),
        identifier: response.identifier,
      };
    } catch (err) {
      return this.handleError(err);
    }
  }

  /**
   * Check if there's a pending OAuth callback.
   * Call this on app init to detect OAuth redirects.
   */
  hasPendingCallback(): boolean {
    const url = new URL(window.location.href);
    const code = url.searchParams.get('code');
    const state = url.searchParams.get('state');

    if (!code || !state) return false;

    // Also check we have stored state
    const storedState = sessionStorage.getItem(OAUTH_STATE_KEY);
    return storedState !== null;
  }

  /**
   * Get callback parameters from current URL.
   */
  getCallbackParams(): { code: string; state: string } | null {
    const url = new URL(window.location.href);
    const code = url.searchParams.get('code');
    const state = url.searchParams.get('state');
    const error = url.searchParams.get('error');

    if (error) {
      return null;
    }

    if (!code || !state) return null;

    return { code, state };
  }

  /**
   * Clear callback parameters from URL without navigation.
   */
  clearCallbackParams(): void {
    const url = new URL(window.location.href);
    url.searchParams.delete('code');
    url.searchParams.delete('state');
    url.searchParams.delete('error');
    url.searchParams.delete('errorDescription');

    // Replace current URL without navigation
    window.history.replaceState({}, '', url.toString());
  }

  /**
   * Generate a random state string for CSRF protection.
   */
  private generateState(): string {
    const array = new Uint8Array(32);
    crypto.getRandomValues(array);
    return Array.from(array, b => b.toString(16).padStart(2, '0')).join('');
  }

  /**
   * Handle HTTP errors and convert to AuthResult.
   */
  private handleError(err: unknown): AuthResult {
    if (err instanceof HttpErrorResponse) {
      const body = err.error as OAuthErrorResponse | undefined;

      // OAuth-specific error handling
      const error = body?.errorDescription ?? body?.error ?? this.getHttpErrorMessage(err.status);
      const code = this.getErrorCode(body?.error ?? '', err.status);

      return {
        success: false,
        error,
        code,
      };
    }

    return {
      success: false,
      error: err instanceof Error ? err.message : 'OAuth authentication failed',
      code: 'NETWORK_ERROR',
    };
  }

  /**
   * Get user-friendly error message for HTTP status.
   */
  private getHttpErrorMessage(status: number): string {
    switch (status) {
      case 400:
        return 'Invalid OAuth request';
      case 401:
        return 'Authorization code expired or invalid';
      case 403:
        return 'Access denied by doorway';
      case 404:
        return 'OAuth endpoint not found';
      default:
        return `OAuth request failed (${status})`;
    }
  }

  /**
   * Map OAuth error to auth error code.
   */
  private getErrorCode(oauthError: string, status: number): AuthFailure['code'] {
    switch (oauthError) {
      case 'invalid_grant':
      case 'invalid_code':
        return 'INVALID_CREDENTIALS';
      case 'expired_token':
      case 'invalid_token':
        return 'TOKEN_EXPIRED';
      case 'access_denied':
        return 'INVALID_CREDENTIALS';
      default:
        return status === 401 ? 'INVALID_CREDENTIALS' : 'NETWORK_ERROR';
    }
  }
}
