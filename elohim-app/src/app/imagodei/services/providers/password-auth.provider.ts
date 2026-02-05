/**
 * Password Authentication Provider.
 *
 * Implements the AuthProvider interface for email/username + password authentication.
 * Communicates with the doorway's /auth/* HTTP endpoints.
 *
 * Doorway-aware: Uses the selected doorway URL when available, falling back to
 * environment configuration for backwards compatibility.
 *
 * Usage:
 * 1. Inject and register with AuthService on app init
 * 2. Use for login/register flows via AuthService
 */

import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

// @coverage: 1.4% (2026-02-05)

import { firstValueFrom } from 'rxjs';

import { environment } from '../../../../environments/environment';
import {
  type AuthProvider,
  type AuthCredentials,
  type RegisterCredentials,
  type AuthResult,
  type AuthFailure,
  type AuthResponse,
  type AuthErrorResponse,
  type PasswordCredentials,
  type RegisterAuthRequest,
  type LoginRequest,
} from '../../models/auth.model';
import { DoorwayRegistryService } from '../doorway-registry.service';

// =============================================================================
// Provider Implementation
// =============================================================================

@Injectable({ providedIn: 'root' })
export class PasswordAuthProvider implements AuthProvider {
  readonly type = 'password' as const;

  private readonly http = inject(HttpClient);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);

  /**
   * Detect if running in Eclipse Che environment.
   */
  private isCheEnvironment(): boolean {
    return (
      window.location.hostname.includes('.devspaces.') ||
      window.location.hostname.includes('.code.ethosengine.com')
    );
  }

  /**
   * Get the Che hc-dev endpoint URL for auth.
   * The admin-proxy is exposed via the hc-dev endpoint on port 8888.
   */
  private getCheAuthUrl(): string | null {
    if (!this.isCheEnvironment()) return null;

    // Replace current endpoint suffix with hc-dev
    // e.g., ...-angular-dev.code.ethosengine.com -> ...-hc-dev.code.ethosengine.com
    const hostname = window.location.hostname.replace(/-angular-dev\./, '-hc-dev.');
    return `https://${hostname}`;
  }

  /**
   * Get the base URL for auth endpoints.
   *
   * Priority:
   * 1. Selected doorway URL (user's chosen identity provider)
   * 2. Eclipse Che: Use hc-dev endpoint (admin-proxy exposed via Che)
   * 3. Explicit authUrl from environment
   * 4. Derive from adminUrl by converting WS to HTTP
   */
  private getAuthBaseUrl(): string {
    // Check for selected doorway first (fediverse-style gateway)
    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (doorwayUrl) {
      return doorwayUrl;
    }

    // Check for Eclipse Che environment
    const cheAuthUrl = this.getCheAuthUrl();
    if (cheAuthUrl) {
      return cheAuthUrl;
    }

    // Use explicit authUrl if provided (for local dev or production)
    if (environment.holochain?.authUrl) {
      return environment.holochain.authUrl;
    }

    // Fall back to deriving from adminUrl
    const adminUrl = environment.holochain?.adminUrl;
    if (!adminUrl) {
      throw new Error('Holochain configuration not available');
    }

    // Convert WebSocket URL to HTTP URL
    // wss://holochain-dev.elohim.host -> https://holochain-dev.elohim.host
    // ws://localhost:8080 -> http://localhost:8080
    let httpUrl = adminUrl.replace(/^wss:/, 'https:').replace(/^ws:/, 'http:');

    // Remove any path and trailing slash
    try {
      const parsed = new URL(httpUrl);
      httpUrl = `${parsed.protocol}//${parsed.host}`;
    } catch {
      // If URL parsing fails, just use the converted URL
    }

    return httpUrl;
  }

  /**
   * Get headers for auth requests.
   */
  private getHeaders(): Record<string, string> {
    return {
      'Content-Type': 'application/json',
    };
  }

  /**
   * Login with email/username and password.
   */
  async login(credentials: AuthCredentials): Promise<AuthResult> {
    if (credentials.type !== 'password') {
      return {
        success: false,
        error: 'Invalid credentials type for password provider',
        code: 'VALIDATION_ERROR',
      };
    }

    const passwordCreds = credentials as PasswordCredentials;
    const url = `${this.getAuthBaseUrl()}/auth/login`;

    const body: LoginRequest = {
      identifier: passwordCreds.identifier,
      password: passwordCreds.password,
    };

    try {
      const response = await firstValueFrom(
        this.http.post<AuthResponse>(url, body, {
          headers: this.getHeaders(),
        })
      );

      return {
        success: true,
        token: response.token,
        humanId: response.humanId,
        agentPubKey: response.agentPubKey,
        expiresAt: response.expiresAt,
        identifier: response.identifier,
      };
    } catch (err) {
      return this.handleError(err);
    }
  }

  /**
   * Register with doorway (creates identity + credentials atomically).
   *
   * Doorway handles:
   * 1. Creating Holochain identity via imagodei zome
   * 2. Storing auth credentials in MongoDB
   * 3. Returning JWT + profile
   */
  async register(credentials: RegisterCredentials): Promise<AuthResult> {
    const url = `${this.getAuthBaseUrl()}/auth/register`;

    const body: RegisterAuthRequest = {
      identifier: credentials.identifier,
      identifierType: credentials.identifierType,
      password: credentials.password,
      // Profile fields - doorway creates identity
      displayName: credentials.displayName,
      bio: credentials.bio,
      affinities: credentials.affinities ?? [],
      profileReach: credentials.profileReach ?? 'public',
      location: credentials.location,
      // Legacy fields (optional - for external registration)
      humanId: credentials.humanId,
      agentPubKey: credentials.agentPubKey,
    };

    try {
      const response = await firstValueFrom(
        this.http.post<AuthResponse>(url, body, {
          headers: this.getHeaders(),
        })
      );

      return {
        success: true,
        token: response.token,
        humanId: response.humanId,
        agentPubKey: response.agentPubKey,
        expiresAt: response.expiresAt,
        identifier: response.identifier,
      };
    } catch (err) {
      return this.handleError(err);
    }
  }

  /**
   * Logout - primarily client-side for password auth.
   */
  async logout(): Promise<void> {
    // Password auth doesn't require server-side logout
    // Token invalidation is handled by expiry
    // We could optionally call /auth/logout for token blacklisting
  }

  /**
   * Refresh an expiring token.
   */
  async refreshToken(token: string): Promise<AuthResult> {
    const url = `${this.getAuthBaseUrl()}/auth/refresh`;

    try {
      const response = await firstValueFrom(
        this.http.post<AuthResponse>(
          url,
          {},
          {
            headers: {
              ...this.getHeaders(),
              Authorization: `Bearer ${token}`,
            },
          }
        )
      );

      return {
        success: true,
        token: response.token,
        humanId: response.humanId,
        agentPubKey: response.agentPubKey,
        expiresAt: response.expiresAt,
        identifier: response.identifier,
      };
    } catch (err) {
      return this.handleError(err);
    }
  }

  /**
   * Get current user info from token.
   * Useful for verifying a restored session.
   */
  async getCurrentUser(token: string): Promise<{
    humanId: string;
    agentPubKey: string;
    identifier: string;
  } | null> {
    const url = `${this.getAuthBaseUrl()}/auth/me`;

    try {
      return await firstValueFrom(
        this.http.get<{ humanId: string; agentPubKey: string; identifier: string }>(url, {
          headers: {
            ...this.getHeaders(),
            Authorization: `Bearer ${token}`,
          },
        })
      );
    } catch {
      return null;
    }
  }

  /**
   * Handle HTTP errors and convert to AuthResult.
   */
  private handleError(err: unknown): AuthResult {
    if (err instanceof HttpErrorResponse) {
      const body = err.error as AuthErrorResponse | undefined;

      // Use error message from response body if available
      const error = body?.error ?? this.getHttpErrorMessage(err.status);
      const code = body?.code ?? this.getHttpErrorCode(err.status);

      return {
        success: false,
        error,
        code,
      };
    }

    return {
      success: false,
      error: err instanceof Error ? err.message : 'Authentication failed',
      code: 'NETWORK_ERROR',
    };
  }

  /**
   * Get user-friendly error message for HTTP status.
   */
  private getHttpErrorMessage(status: number): string {
    switch (status) {
      case 400:
        return 'Invalid request';
      case 401:
        return 'Invalid credentials';
      case 403:
        return 'Access denied';
      case 404:
        return 'Authentication service not available';
      case 409:
        return 'Account already exists';
      case 500:
        return 'Server error. Please try again later.';
      case 501:
        return 'Password authentication is not enabled';
      default:
        return `Request failed (${status})`;
    }
  }

  /**
   * Get error code for HTTP status.
   */
  private getHttpErrorCode(status: number): AuthFailure['code'] {
    switch (status) {
      case 401:
        return 'INVALID_CREDENTIALS';
      case 409:
        return 'USER_EXISTS';
      case 501:
        return 'NOT_ENABLED';
      default:
        return 'NETWORK_ERROR';
    }
  }
}
