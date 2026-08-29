/**
 * Authentication Service for Hosted Human Login/Logout.
 *
 * This service manages authentication state and coordinates with auth providers
 * to handle login, logout, and token management.
 *
 * Design:
 * - Uses signals for reactive state management
 * - Supports multiple auth providers through the AuthProvider interface
 * - Persists auth state via BrowserSessionTokenStore (same localStorage keys
 *   as always; in-memory on the SSR server path)
 * - Delegates doorway session walking (token refresh, expiry-checked session
 *   restore) to @elohim/identity's DoorwaySessionClient — the consolidated,
 *   schema-contract-pinned auth wire surface
 * - Integrates with IdentityService for Holochain identity coordination
 * - Doorway-aware: Exposes selected doorway info for UI components
 *
 * Edge vs walking split (auth-wire plan 2026-06-10, Task 2):
 * - EDGE (stays here): the AuthProvider registry + provider selection,
 *   refresh TIMER scheduling + isRefreshing guard, the AuthState signal,
 *   and auth base-URL derivation (the client receives a plain origin —
 *   no connection-strategy logic enters the client)
 * - WALKING (delegated): the /auth/refresh HTTP call and all token
 *   bookkeeping go through DoorwaySessionClient + the token store
 *
 * Doorway Integration:
 * - Users select a doorway (fediverse-style gateway) at registration
 * - Selected doorway is used for all auth operations
 * - Providers access doorway URL through DoorwayRegistryService
 *
 * Usage:
 * 1. Register providers on app initialization
 * 2. Call restoreSession() on app startup to recover existing auth
 * 3. Use login() / logout() for authentication flows
 */

import { Injectable, signal, computed, inject } from '@angular/core';

// @coverage: 93.5% (2026-02-24)

import { DoorwaySessionClient, DoorwaySessionError } from '@elohim/identity';

import { workspaceDoorwayUrl } from '@workspace/runtime';

import { environment } from '../../../environments/environment';
import {
  type AuthState,
  type AuthProvider,
  type AuthProviderType,
  type AuthCredentials,
  type RegisterCredentials,
  type AuthResult,
  type AuthFailure,
  type AuthErrorCode,
  INITIAL_AUTH_STATE,
  parseExpiryDate,
} from '../models/auth.model';

import { BrowserSessionTokenStore } from './browser-session-token.store';
import { DoorwayRegistryService } from './doorway-registry.service';

// =============================================================================
// Service
// =============================================================================

/** Valid wire error codes — used to pass doorway envelope codes through. */
const AUTH_ERROR_CODES: ReadonlySet<string> = new Set([
  'INVALID_CREDENTIALS',
  'USER_EXISTS',
  'IDENTITY_EXISTS',
  'NOT_ENABLED',
  'VALIDATION_ERROR',
  'NETWORK_ERROR',
  'TOKEN_EXPIRED',
]);

@Injectable({ providedIn: 'root' })
export class AuthService {
  // ==========================================================================
  // Dependencies
  // ==========================================================================

  private readonly doorwayRegistry = inject(DoorwayRegistryService);

  /** SSR-guarded session persistence (same localStorage keys as always). */
  private readonly sessionStore = inject(BrowserSessionTokenStore);

  // ==========================================================================
  // State
  // ==========================================================================

  /** Core authentication state signal */
  private readonly authSignal = signal<AuthState>(INITIAL_AUTH_STATE);

  /** Registered authentication providers */
  private readonly providers = new Map<AuthProviderType, AuthProvider>();

  /** Token refresh timer */
  private refreshTimer: ReturnType<typeof setTimeout> | null = null;

  /** Guard against re-entrant token refresh */
  private isRefreshing = false;

  /** Doorway session client (wire walking); rebuilt when the baseUrl changes. */
  private sessionClient: DoorwaySessionClient | null = null;

  /** baseUrl the cached session client was built for. */
  private sessionClientBaseUrl: string | null = null;

  // ==========================================================================
  // Public Signals (read-only)
  // ==========================================================================

  /** Complete authentication state */
  readonly auth = this.authSignal.asReadonly();

  /** Whether user is authenticated */
  readonly isAuthenticated = computed(() => this.authSignal().isAuthenticated);

  /** Current JWT token */
  readonly token = computed(() => this.authSignal().token);

  /** Holochain human ID */
  readonly humanId = computed(() => this.authSignal().humanId);

  /** Holochain agent public key */
  readonly agentPubKey = computed(() => this.authSignal().agentPubKey);

  /** User identifier (email/username) */
  readonly identifier = computed(() => this.authSignal().identifier);

  /** Auth provider type */
  readonly provider = computed(() => this.authSignal().provider);

  /** Whether an auth operation is in progress */
  readonly isLoading = computed(() => this.authSignal().isLoading);

  /** Current error message */
  readonly error = computed(() => this.authSignal().error);

  /** Token expiration time */
  readonly expiresAt = computed(() => this.authSignal().expiresAt);

  // ==========================================================================
  // Doorway Signals (delegated to registry)
  // ==========================================================================

  /** Selected doorway for authentication */
  readonly selectedDoorway = this.doorwayRegistry.selected;

  /** Selected doorway URL */
  readonly doorwayUrl = this.doorwayRegistry.selectedUrl;

  /** Whether a doorway has been selected */
  readonly hasDoorway = this.doorwayRegistry.hasSelection;

  // ==========================================================================
  // Constructor
  // ==========================================================================

  constructor() {
    // Attempt to restore session on service init
    this.restoreSession();
  }

  // ==========================================================================
  // Provider Management
  // ==========================================================================

  /**
   * Register an authentication provider.
   *
   * @param provider - Provider instance to register
   */
  registerProvider(provider: AuthProvider): void {
    this.providers.set(provider.type, provider);
  }

  /**
   * Get a registered provider by type.
   *
   * @param type - Provider type
   * @returns Provider instance or undefined
   */
  getProvider(type: AuthProviderType): AuthProvider | undefined {
    return this.providers.get(type);
  }

  /**
   * Check if a provider is registered.
   *
   * @param type - Provider type
   * @returns True if provider is registered
   */
  hasProvider(type: AuthProviderType): boolean {
    return this.providers.has(type);
  }

  // ==========================================================================
  // Authentication Operations
  // ==========================================================================

  /**
   * Login with credentials.
   *
   * @param type - Provider type to use
   * @param credentials - Authentication credentials
   * @returns Authentication result
   */
  async login(type: AuthProviderType, credentials: AuthCredentials): Promise<AuthResult> {
    const provider = this.providers.get(type);

    if (!provider) {
      const result: AuthResult = {
        success: false,
        error: `Authentication provider '${type}' not registered`,
        code: 'NOT_ENABLED',
      };
      return result;
    }

    this.updateState({ isLoading: true, error: null });

    try {
      const result = await provider.login(credentials);

      if (result.success) {
        this.handleAuthSuccess(result, type);
      } else {
        this.updateState({
          isLoading: false,
          error: result.error,
        });
      }

      return result;
    } catch (err) {
      const error = err instanceof Error ? err.message : 'Login failed';
      this.updateState({ isLoading: false, error });
      return { success: false, error, code: 'NETWORK_ERROR' };
    }
  }

  /**
   * Register authentication credentials for a Holochain identity.
   *
   * @param type - Provider type to use
   * @param credentials - Registration credentials including Holochain identity
   * @returns Authentication result
   */
  async register(type: AuthProviderType, credentials: RegisterCredentials): Promise<AuthResult> {
    const provider = this.providers.get(type);

    if (!provider) {
      return {
        success: false,
        error: `Authentication provider '${type}' not registered`,
        code: 'NOT_ENABLED',
      };
    }

    if (!provider.register) {
      return {
        success: false,
        error: `Provider '${type}' does not support registration`,
        code: 'NOT_ENABLED',
      };
    }

    this.updateState({ isLoading: true, error: null });

    try {
      const result = await provider.register(credentials);

      if (result.success) {
        this.handleAuthSuccess(result, type);
      } else {
        this.updateState({
          isLoading: false,
          error: result.error,
        });
      }

      return result;
    } catch (err) {
      const error = err instanceof Error ? err.message : 'Registration failed';
      this.updateState({ isLoading: false, error });
      return { success: false, error, code: 'NETWORK_ERROR' };
    }
  }

  /**
   * Logout and clear authentication state.
   */
  async logout(): Promise<void> {
    const provider = this.provider();

    // Call provider logout if available
    if (provider) {
      const providerInstance = this.providers.get(provider);
      if (providerInstance) {
        try {
          await providerInstance.logout();
        } catch {
          // Provider logout failed silently
        }
      }
    }

    // Clear timer
    this.clearRefreshTimer();

    // Clear persisted state (same key set clearPersistedAuth always removed)
    this.sessionStore.clear();

    // Reset state
    this.updateState(INITIAL_AUTH_STATE);
  }

  /**
   * Refresh the current token.
   *
   * The provider capability gate stays at the service edge (provider
   * selection); the HTTP walking itself is delegated to the composed
   * DoorwaySessionClient (`POST /auth/refresh`, bearer from the token store).
   *
   * @returns New authentication result
   */
  async refreshToken(): Promise<AuthResult> {
    const currentToken = this.token();
    const providerType = this.provider();

    if (!currentToken || !providerType) {
      return {
        success: false,
        error: 'No active session to refresh',
        code: 'TOKEN_EXPIRED',
      };
    }

    const provider = this.providers.get(providerType);
    if (!provider?.refreshToken) {
      return {
        success: false,
        error: 'Token refresh not supported',
        code: 'NOT_ENABLED',
      };
    }

    const baseUrl = this.deriveAuthBaseUrl();
    if (!baseUrl) {
      return {
        success: false,
        error: 'Holochain configuration not available',
        code: 'NETWORK_ERROR',
      };
    }

    try {
      const response = await this.getSessionClient(baseUrl).refresh();

      const result = {
        success: true as const,
        token: response.token,
        humanId: response.humanId,
        agentPubKey: response.agentPubKey,
        expiresAt: response.expiresAt,
        identifier: response.identifier,
        installedAppId: response.installedAppId,
      };

      this.handleAuthSuccess(result, providerType);
      return result;
    } catch (err) {
      return this.refreshFailure(err);
    }
  }

  // ==========================================================================
  // Session Management
  // ==========================================================================

  /**
   * Restore session from the persisted token store.
   *
   * Delegates the expiry-checked read to DoorwaySessionClient.restoreSession()
   * (purely local — clears the stored session when expired); the provider
   * type and the AuthState hydration stay at the service edge.
   *
   * @returns True if session was restored
   */
  restoreSession(): boolean {
    try {
      const provider = this.sessionStore.getProviderType();
      const client = this.getSessionClient(this.deriveAuthBaseUrl() ?? '');
      const stored = client.restoreSession();

      if (!stored || !provider) {
        return false;
      }

      const expiresAt = new Date(stored.expiresAt * 1000);

      this.updateState({
        isAuthenticated: true,
        token: stored.token,
        provider,
        expiresAt,
        // '' is the token store's absent-value sentinel — map back to null
        identifier: stored.identifier || null,
        humanId: stored.humanId || null,
        agentPubKey: stored.agentPubKey || null,
        isLoading: false,
        error: null,
      });

      // Schedule token refresh
      this.scheduleRefresh(expiresAt);

      return true;
    } catch {
      // Session restore failure is non-critical - user can login again
      return false;
    }
  }

  /**
   * Check if there's a stored session (without restoring).
   *
   * @returns True if session data exists
   */
  hasStoredSession(): boolean {
    return this.sessionStore.get() !== null;
  }

  // ==========================================================================
  // Private Helpers
  // ==========================================================================

  /**
   * Handle successful authentication.
   */
  private handleAuthSuccess(
    result: AuthResult & { success: true },
    provider: AuthProviderType
  ): void {
    const expiresAt = parseExpiryDate(result.expiresAt);

    // Update state
    this.updateState({
      isAuthenticated: true,
      token: result.token,
      humanId: result.humanId,
      agentPubKey: result.agentPubKey,
      expiresAt,
      provider,
      identifier: result.identifier,
      isLoading: false,
      error: null,
    });

    // Persist via the session token store (same localStorage keys as before;
    // also seeds the bearer source DoorwaySessionClient refreshes from)
    this.sessionStore.set({
      token: result.token,
      humanId: result.humanId,
      agentPubKey: result.agentPubKey,
      identifier: result.identifier,
      expiresAt: expiresAt ? Math.floor(expiresAt.getTime() / 1000) : 0,
      installedAppId: result.installedAppId,
    });
    this.sessionStore.setProviderType(provider);

    // Schedule token refresh
    this.scheduleRefresh(expiresAt);
  }

  /**
   * Map a thrown refresh error onto the AuthResult failure contract,
   * preserving pre-migration semantics: doorway envelope code/message when
   * present, provider-equivalent status mapping otherwise, NETWORK_ERROR
   * for network-level failures.
   */
  private refreshFailure(err: unknown): AuthFailure {
    if (err instanceof DoorwaySessionError) {
      return {
        success: false,
        error: err.message,
        code: this.mapRefreshErrorCode(err),
      };
    }

    const error = err instanceof Error ? err.message : 'Token refresh failed';
    return { success: false, error, code: 'NETWORK_ERROR' };
  }

  /** Status→code mapping mirroring the password provider's getHttpErrorCode. */
  private mapRefreshErrorCode(err: DoorwaySessionError): AuthErrorCode {
    if (err.code && AUTH_ERROR_CODES.has(err.code)) {
      return err.code as AuthErrorCode;
    }
    switch (err.status) {
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

  /**
   * Doorway base URL for session walking. The service owns this derivation —
   * the client receives a plain origin (no strategy logic in the client).
   * Mirrors PasswordAuthProvider.getAuthBaseUrl() priority:
   * PROVEN doorway selection > this workspace's doorway > environment authUrl
   * > adminUrl. Bearer tokens ride this URL, so the environment is the default
   * and a selection only overrides it with proof.
   *
   * @returns Base URL, or null when no configuration is available
   */
  private deriveAuthBaseUrl(): string | null {
    // A doorway selection overrides the environment ONLY when it carries proof:
    // `selectedUrl()` is null until DoorwayRegistryService has verified it.
    const doorwayUrl = this.doorwayRegistry.selectedUrl();
    if (doorwayUrl) {
      return doorwayUrl;
    }

    // A development workspace serves the doorway at a sibling origin, resolved
    // by the vendor fence (`app/workspace-runtime/`). Returns null off-workspace.
    const workspaceDoorway = workspaceDoorwayUrl();
    if (workspaceDoorway) {
      return workspaceDoorway;
    }

    // Explicit authUrl (local dev or production)
    if (environment.holochain?.authUrl) {
      return environment.holochain.authUrl;
    }

    // Fall back to deriving from adminUrl (WS → HTTP)
    const adminUrl = environment.holochain?.adminUrl;
    if (!adminUrl) {
      return null;
    }

    let httpUrl = adminUrl.replace(/^wss:/, 'https:').replace(/^ws:/, 'http:');
    try {
      const parsed = new URL(httpUrl);
      httpUrl = `${parsed.protocol}//${parsed.host}`;
    } catch {
      // If URL parsing fails, just use the converted URL
    }
    return httpUrl;
  }

  /**
   * Lazily build (and cache per baseUrl) the composed DoorwaySessionClient.
   * The token store is the shared persistence seam: the service writes
   * sessions there on auth success, the client reads the bearer from it.
   */
  private getSessionClient(baseUrl: string): DoorwaySessionClient {
    if (!this.sessionClient || this.sessionClientBaseUrl !== baseUrl) {
      this.sessionClient = new DoorwaySessionClient({
        baseUrl,
        // Late-bound platform fetch (browser and SSR Node both provide it)
        fetchImpl: async (input, init) => globalThis.fetch(input, init),
        tokenStore: this.sessionStore,
      });
      this.sessionClientBaseUrl = baseUrl;
    }
    return this.sessionClient;
  }

  /**
   * Schedule automatic token refresh.
   */
  private scheduleRefresh(expiresAt: Date | null): void {
    this.clearRefreshTimer();

    if (!expiresAt) return;

    // Refresh 5 minutes before expiry
    const refreshTime = expiresAt.getTime() - Date.now() - 5 * 60 * 1000;

    if (refreshTime <= 0) {
      // Token already expiring soon - but guard against refresh loop
      if (this.isRefreshing) {
        return;
      }

      this.isRefreshing = true;
      void this.refreshToken().finally(() => {
        this.isRefreshing = false;
      });
      return;
    }

    this.refreshTimer = setTimeout(() => {
      void this.refreshToken();
    }, refreshTime);
  }

  /**
   * Clear the refresh timer.
   */
  private clearRefreshTimer(): void {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = null;
    }
  }

  /**
   * Update authentication state (partial update).
   */
  private updateState(partial: Partial<AuthState>): void {
    this.authSignal.update(current => ({
      ...current,
      ...partial,
    }));
  }

  /**
   * Clear any error state.
   */
  clearError(): void {
    this.updateState({ error: null });
  }

  /**
   * Set authentication state from an external auth result.
   * Used by OAuth callback to complete the login flow.
   *
   * @param result - Successful auth result from OAuth provider
   * @param providerType - The provider type that was used (default: oauth)
   */
  setAuthFromResult(result: AuthResult, providerType: AuthProviderType = 'oauth'): void {
    if (!result.success) {
      this.updateState({
        isLoading: false,
        error: result.error,
      });
      return;
    }

    this.handleAuthSuccess(result, providerType);
  }

  /**
   * Set authentication state from a Tauri local session.
   * Used by TauriAuthService after OAuth handoff or session restoration.
   *
   * @param session - Local session from elohim-storage
   */
  setTauriSession(session: {
    humanId: string;
    agentPubKey: string;
    doorwayUrl: string;
    identifier: string;
    displayName?: string;
  }): void {
    this.updateState({
      isAuthenticated: true,
      humanId: session.humanId,
      agentPubKey: session.agentPubKey,
      identifier: session.identifier,
      // Tauri sessions don't use JWT tokens - local session is the auth
      token: null,
      expiresAt: null,
      provider: 'tauri' as AuthProviderType,
      isLoading: false,
      error: null,
    });
  }
}
