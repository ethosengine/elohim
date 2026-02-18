/**
 * Authentication Service for Hosted Human Login/Logout.
 *
 * This service manages authentication state and coordinates with auth providers
 * to handle login, logout, and token management.
 *
 * Design:
 * - Uses signals for reactive state management
 * - Supports multiple auth providers through the AuthProvider interface
 * - Persists auth state in localStorage for session recovery
 * - Integrates with IdentityService for Holochain identity coordination
 * - Doorway-aware: Exposes selected doorway info for UI components
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

// @coverage: 93.6% (2026-02-05)

import { SIGNING_CREDENTIALS_KEY } from '../../elohim/models/holochain-connection.model';
import {
  type AuthState,
  type AuthProvider,
  type AuthProviderType,
  type AuthCredentials,
  type RegisterCredentials,
  type AuthResult,
  INITIAL_AUTH_STATE,
  AUTH_TOKEN_KEY,
  AUTH_PROVIDER_KEY,
  AUTH_EXPIRY_KEY,
  AUTH_IDENTIFIER_KEY,
  AUTH_HUMAN_ID_KEY,
  AUTH_AGENT_PUB_KEY_KEY,
  AUTH_INSTALLED_APP_ID_KEY,
  parseExpiryDate,
  isTokenExpiringSoon,
} from '../models/auth.model';
import { DOORWAY_CACHE_KEY } from '../models/doorway.model';

import { DoorwayRegistryService } from './doorway-registry.service';

// =============================================================================
// Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class AuthService {
  // ==========================================================================
  // Dependencies
  // ==========================================================================

  private readonly doorwayRegistry = inject(DoorwayRegistryService);

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

    // Clear persisted state
    this.clearPersistedAuth();

    // Reset state
    this.updateState(INITIAL_AUTH_STATE);
  }

  /**
   * Refresh the current token.
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

    try {
      const result = await provider.refreshToken(currentToken);

      if (result.success) {
        this.handleAuthSuccess(result, providerType);
      }

      return result;
    } catch (err) {
      const error = err instanceof Error ? err.message : 'Token refresh failed';
      return { success: false, error, code: 'NETWORK_ERROR' };
    }
  }

  // ==========================================================================
  // Session Management
  // ==========================================================================

  /**
   * Restore session from localStorage.
   *
   * @returns True if session was restored
   */
  restoreSession(): boolean {
    try {
      const token = localStorage.getItem(AUTH_TOKEN_KEY);
      const provider = localStorage.getItem(AUTH_PROVIDER_KEY) as AuthProviderType | null;
      const expiresAtStr = localStorage.getItem(AUTH_EXPIRY_KEY);
      const identifier = localStorage.getItem(AUTH_IDENTIFIER_KEY);
      const humanId = localStorage.getItem(AUTH_HUMAN_ID_KEY);
      const agentPubKey = localStorage.getItem(AUTH_AGENT_PUB_KEY_KEY);

      if (!token || !provider) {
        return false;
      }

      const expiresAt = parseExpiryDate(expiresAtStr);

      // Check if token is expired
      if (isTokenExpiringSoon(expiresAt, 0)) {
        this.clearPersistedAuth();
        return false;
      }

      this.updateState({
        isAuthenticated: true,
        token,
        provider,
        expiresAt,
        identifier,
        humanId,
        agentPubKey,
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
    return !!localStorage.getItem(AUTH_TOKEN_KEY);
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

    // Persist to localStorage
    this.persistAuth(
      result.token,
      provider,
      result.expiresAt,
      result.identifier,
      result.humanId,
      result.agentPubKey
    );

    // Store installed app ID for multi-conductor routing
    if (result.installedAppId) {
      localStorage.setItem(AUTH_INSTALLED_APP_ID_KEY, result.installedAppId);
    }

    // Schedule token refresh
    this.scheduleRefresh(expiresAt);
  }

  /**
   * Persist auth state to localStorage.
   */
  private persistAuth(
    token: string,
    provider: AuthProviderType,
    expiresAt: string | number,
    identifier: string,
    humanId?: string,
    agentPubKey?: string
  ): void {
    localStorage.setItem(AUTH_TOKEN_KEY, token);
    localStorage.setItem(AUTH_PROVIDER_KEY, provider);
    // Store as string - parseExpiryDate handles both formats on restore
    localStorage.setItem(AUTH_EXPIRY_KEY, String(expiresAt));
    localStorage.setItem(AUTH_IDENTIFIER_KEY, identifier);
    if (humanId) {
      localStorage.setItem(AUTH_HUMAN_ID_KEY, humanId);
    }
    if (agentPubKey) {
      localStorage.setItem(AUTH_AGENT_PUB_KEY_KEY, agentPubKey);
    }
  }

  /**
   * Clear persisted auth state.
   */
  private clearPersistedAuth(): void {
    localStorage.removeItem(AUTH_TOKEN_KEY);
    localStorage.removeItem(AUTH_PROVIDER_KEY);
    localStorage.removeItem(AUTH_EXPIRY_KEY);
    localStorage.removeItem(AUTH_IDENTIFIER_KEY);
    localStorage.removeItem(AUTH_HUMAN_ID_KEY);
    localStorage.removeItem(AUTH_AGENT_PUB_KEY_KEY);
    localStorage.removeItem(AUTH_INSTALLED_APP_ID_KEY);
    localStorage.removeItem(SIGNING_CREDENTIALS_KEY);
    localStorage.removeItem(DOORWAY_CACHE_KEY);
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
