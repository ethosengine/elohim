/**
 * TauriAuthService - Native OAuth Handler for Tauri
 *
 * Handles OAuth flow for Tauri native app:
 * 1. Listens for 'oauth-callback' event from Tauri deep link handler
 * 2. Exchanges authorization code for access token
 * 3. Calls doorway's /auth/native-handoff to get identity info
 * 4. Creates local session in elohim-storage SQLite
 *
 * This service should be initialized at app startup in Tauri environments.
 */

import { Injectable, inject, signal, computed } from '@angular/core';
import { Router } from '@angular/router';

// @coverage: 92.2% (2026-02-05)

import { environment } from '../../../environments/environment';

import { AuthService } from './auth.service';
import { DoorwayRegistryService } from './doorway-registry.service';

/** Tauri global interface for event listening and IPC */
declare global {
  interface Window {
    __TAURI__?: {
      event: {
        listen: <T>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>;
      };
      core: {
        invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
      };
    };
  }
}

/** OAuth callback payload from Tauri deep link */
interface OAuthCallbackPayload {
  code: string;
  state?: string;
  url: string;
}

/** Deep link error payload */
interface DeepLinkError {
  message: string;
  url: string;
}

/** Native handoff response from doorway */
interface NativeHandoffResponse {
  humanId: string;
  identifier: string;
  agentPubKey: string;
  doorwayId: string;
  doorwayUrl: string;
  displayName?: string;
  profileImageHash?: string;
  bootstrapUrl?: string;
}

/** Local session stored in elohim-storage */
interface LocalSession {
  id: string;
  humanId: string;
  agentPubKey: string;
  doorwayUrl: string;
  doorwayId?: string;
  identifier: string;
  displayName?: string;
  profileImageHash?: string;
  isActive: number;
  createdAt: string;
  updatedAt: string;
  lastSyncedAt?: string;
  bootstrapUrl?: string;
}

/** Result from doorway_login IPC command */
interface DoorwayLoginResult {
  humanId: string;
  identifier: string;
  agentPubKey: string;
  doorwayId: string;
  conductorId?: string;
  hasKeyBundle: boolean;
  needsRestart: boolean;
  isSteward: boolean;
}

/** Result from doorway_unlock IPC command */
interface DoorwayUnlockResult {
  identifier: string;
  isSteward: boolean;
}

/** Result from doorway_status IPC command */
interface DoorwayStatus {
  connected: boolean;
  doorwayUrl?: string;
  identifier?: string;
  agentPubKey?: string;
  hasIdentity: boolean;
  isSteward: boolean;
  hasKeyBundle: boolean;
}

export type TauriAuthStatus = 'idle' | 'checking' | 'needs_login' | 'needs_unlock' | 'authenticated' | 'error';
export type GraduationStatus = 'idle' | 'confirming' | 'confirmed' | 'error';

@Injectable({
  providedIn: 'root',
})
export class TauriAuthService {
  private readonly router = inject(Router);
  private readonly authService = inject(AuthService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);

  // State signals
  readonly status = signal<TauriAuthStatus>('idle');
  readonly errorMessage = signal<string>('');
  readonly currentSession = signal<LocalSession | null>(null);

  /** Session data held before unlock — not yet promoted to currentSession */
  readonly pendingSession = signal<LocalSession | null>(null);

  // Stewardship signal — driven by doorway.json via doorway_status IPC
  readonly isSteward = signal(false);

  // Graduation signals
  readonly graduationStatus = signal<GraduationStatus>('idle');
  readonly graduationError = signal<string>('');

  // Computed state
  readonly isAuthenticated = computed(() => this.status() === 'authenticated');
  readonly needsLogin = computed(() => this.status() === 'needs_login');
  readonly needsUnlock = computed(() => this.status() === 'needs_unlock');
  readonly isTauri = computed(() => this.isTauriEnvironment());

  /** Whether this user is eligible to graduate (Tauri + authenticated) */
  readonly isGraduationEligible = computed(
    () => this.isTauri() && this.isAuthenticated() && this.status() === 'authenticated'
  );

  // Event unsubscribe callbacks
  private unsubscribeOAuthCallback?: () => void;
  private unsubscribeDeepLinkError?: () => void;

  /**
   * Check if running in Tauri environment.
   */
  isTauriEnvironment(): boolean {
    // eslint-disable-next-line unicorn/prefer-global-this -- window is correct here: Tauri extends Window, not globalThis
    return typeof window !== 'undefined' && '__TAURI__' in window;
  }

  /**
   * Get the elohim-storage base URL.
   */
  private getStorageUrl(): string {
    return environment.client?.storageUrl ?? 'http://localhost:8090';
  }

  /**
   * Initialize Tauri auth - check for existing session.
   *
   * Called at app startup to detect first-run vs returning user.
   */
  async initialize(): Promise<void> {
    if (!this.isTauriEnvironment()) {
      return;
    }

    this.status.set('checking');

    try {
      // Check for existing local session, retrying on network errors
      // (sidecar may still be starting up)
      const session = await this.getActiveSessionWithRetry();

      if (session) {
        // Session exists — check if user must prove identity first
        const doorwayStatus = await this.getDoorwayStatus();
        if (doorwayStatus?.hasKeyBundle) {
          // Key bundle present: gate on password unlock before granting access
          this.pendingSession.set(session);
          this.status.set('needs_unlock');
        } else {
          // Standalone mode (no key bundle): auto-authenticate as before
          this.currentSession.set(session);
          this.status.set('authenticated');
          this.authService.setTauriSession(session);
          await this.refreshStewardshipStatus();
        }
      } else {
        this.status.set('needs_login');
        // Clear stale doorway selection — IPC (doorway.json) is source of truth in Tauri
        this.doorwayRegistry.clearSelection();
      }

      // Set up event listeners for OAuth callback
      await this.setupEventListeners();
    } catch (err) {
      this.status.set('error');
      this.errorMessage.set(err instanceof Error ? err.message : 'Initialization failed');
    }
  }

  /**
   * Retry getActiveSession with exponential backoff for network errors.
   * Sidecar may take 1-3s to start; 404 (no session) returns null immediately.
   */
  private async getActiveSessionWithRetry(): Promise<LocalSession | null> {
    const backoffMs = [1000, 2000, 4000];

    for (let attempt = 0; attempt <= backoffMs.length; attempt++) {
      try {
        return await this.getActiveSession();
      } catch (err) {
        // Only retry network errors (TypeError from fetch)
        if (!(err instanceof TypeError) || attempt === backoffMs.length) {
          throw err;
        }
        console.warn(
          `[TauriAuthService] Sidecar not ready (attempt ${attempt + 1}/${backoffMs.length + 1}), retrying in ${backoffMs[attempt]}ms`
        );
        await new Promise(resolve => setTimeout(resolve, backoffMs[attempt]));
      }
    }

    // Unreachable, but satisfies TypeScript
    return null;
  }

  /**
   * Set up Tauri event listeners for OAuth callbacks.
   */
  private async setupEventListeners(): Promise<void> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.event) {
      return;
    }

    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    const { listen } = window.__TAURI__.event;

    // Listen for OAuth callback from deep link
    this.unsubscribeOAuthCallback = await listen<OAuthCallbackPayload>(
      'oauth-callback',
      // eslint-disable-next-line @typescript-eslint/no-misused-promises -- callback must be async to await OAuth flow
      async event => {
        await this.handleOAuthCallback(event.payload);
      }
    );

    // Listen for deep link errors
    this.unsubscribeDeepLinkError = await listen<DeepLinkError>('deep-link-error', event => {
      this.status.set('error');
      this.errorMessage.set(event.payload.message);
    });

    // Drain any OAuth callbacks that arrived before Angular was ready (cold-start)
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (window.__TAURI__?.core) {
      try {
        const pending =
          // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
          await window.__TAURI__.core.invoke<OAuthCallbackPayload[]>('get_pending_deep_links');
        for (const payload of pending) {
          await this.handleOAuthCallback(payload);
        }
      } catch (err) {
        if (err instanceof Error) {
          console.warn('[TauriAuthService] Failed to drain pending deep links:', err.message);
        }
      }
    }
  }

  /**
   * Clean up event listeners.
   */
  destroy(): void {
    this.unsubscribeOAuthCallback?.();
    this.unsubscribeDeepLinkError?.();
  }

  /**
   * Get the active local session from elohim-storage.
   */
  async getActiveSession(): Promise<LocalSession | null> {
    const storageUrl = this.getStorageUrl();

    try {
      const response = await fetch(`${storageUrl}/session`);

      if (response.status === 404) {
        return null;
      }

      if (!response.ok) {
        throw new Error(`Session API error: ${response.status}`);
      }

      return (await response.json()) as LocalSession;
    } catch (err) {
      // Network errors (sidecar not ready) should propagate for retry logic.
      // TypeError from fetch = network-level failure (connection refused, DNS, etc.)
      if (err instanceof TypeError) {
        throw err;
      }
      // Non-network errors (e.g. JSON parse) are non-critical
      if (err instanceof Error) {
        console.warn('[TauriAuthService] Failed to retrieve session:', err.message);
      }
      return null;
    }
  }

  /**
   * Handle OAuth callback from Tauri deep link.
   */
  private async handleOAuthCallback(payload: OAuthCallbackPayload): Promise<void> {
    this.status.set('checking');

    try {
      // Get the selected doorway URL
      const selectedDoorway = this.doorwayRegistry.selected();
      if (!selectedDoorway) {
        throw new Error('No doorway selected');
      }

      const doorwayUrl = selectedDoorway.doorway.url;

      // Step 1: Exchange code for token
      const tokenResponse = await fetch(`${doorwayUrl}/auth/token`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
        body: new URLSearchParams({
          grant_type: 'authorization_code',
          code: payload.code,
          redirect_uri: 'elohim://auth/callback',
        }),
      });

      if (!tokenResponse.ok) {
        const error = await tokenResponse.text();
        throw new Error(`Token exchange failed: ${error}`);
      }

      const tokenData = (await tokenResponse.json()) as { access_token: string };
      const accessToken = tokenData.access_token;

      // Step 2: Get native handoff info
      const handoffResponse = await fetch(`${doorwayUrl}/auth/native-handoff`, {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      });

      if (!handoffResponse.ok) {
        const error = await handoffResponse.text();
        throw new Error(`Native handoff failed: ${error}`);
      }

      const handoff: NativeHandoffResponse = await handoffResponse.json();

      // Step 3: Create local session with doorway-provisioned agent key
      const session = await this.createSession({
        humanId: handoff.humanId,
        agentPubKey: handoff.agentPubKey,
        doorwayUrl: handoff.doorwayUrl,
        doorwayId: handoff.doorwayId,
        identifier: handoff.identifier,
        displayName: handoff.displayName,
        profileImageHash: handoff.profileImageHash,
        bootstrapUrl: handoff.bootstrapUrl,
      });

      this.currentSession.set(session);
      this.status.set('authenticated');

      // Update auth service
      this.authService.setTauriSession(session);

      // Navigate to main app
      void this.router.navigate(['/lamad']);
    } catch (err) {
      this.status.set('error');
      this.errorMessage.set(err instanceof Error ? err.message : 'Authentication failed');
    }
  }

  /**
   * Create a new local session in elohim-storage.
   */
  private async createSession(input: {
    humanId: string;
    agentPubKey: string;
    doorwayUrl: string;
    doorwayId?: string;
    identifier: string;
    displayName?: string;
    profileImageHash?: string;
    bootstrapUrl?: string;
  }): Promise<LocalSession> {
    const storageUrl = this.getStorageUrl();

    const body = {
      humanId: input.humanId,
      agentPubKey: input.agentPubKey,
      doorwayUrl: input.doorwayUrl,
      doorwayId: input.doorwayId,
      identifier: input.identifier,
      displayName: input.displayName,
      profileImageHash: input.profileImageHash,
      bootstrapUrl: input.bootstrapUrl,
    };

    const response = await fetch(`${storageUrl}/session`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to create session: ${error}`);
    }

    return (await response.json()) as LocalSession;
  }

  /**
   * Confirm stewardship — graduate from hosted to app-steward.
   *
   * Calls the Tauri IPC command which:
   * 1. Decrypts the key bundle with the user's password
   * 2. Signs the stewardship challenge
   * 3. Calls doorway's POST /auth/confirm-stewardship
   * 4. Doorway retires the conductor cell and marks user as steward
   *
   * @param password - The user's doorway password (used to decrypt key bundle)
   * @returns true on success, false on failure
   */
  async confirmStewardship(password: string): Promise<boolean> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) {
      this.graduationStatus.set('error');
      this.graduationError.set('Tauri IPC not available');
      return false;
    }

    this.graduationStatus.set('confirming');
    this.graduationError.set('');

    try {
      // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
      await window.__TAURI__.core.invoke('doorway_confirm_stewardship', { password });

      // Update auth state — identity is now local
      this.status.set('authenticated');
      this.graduationStatus.set('confirmed');

      return true;
    } catch (err) {
      this.graduationStatus.set('error');
      this.graduationError.set(err instanceof Error ? err.message : String(err));
      return false;
    }
  }

  /**
   * Login with password via Tauri IPC (doorway_login).
   *
   * Used by first-time users who selected a doorway and are entering credentials.
   * The Rust side handles: login -> handoff -> key decryption -> store save -> session creation.
   */
  async loginWithPassword(
    doorwayUrl: string,
    identifier: string,
    password: string
  ): Promise<{
    success: boolean;
    needsRestart: boolean;
    isSteward: boolean;
    error?: string;
  }> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) {
      return { success: false, needsRestart: false, isSteward: false, error: 'Tauri IPC not available' };
    }

    try {
      // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
      const result = await window.__TAURI__.core.invoke<DoorwayLoginResult>('doorway_login', {
        url: doorwayUrl,
        identifier,
        password,
      });

      this.isSteward.set(result.isSteward);

      // Session was already created by Rust side — refresh our local state
      const session = await this.getActiveSession();
      if (session) {
        this.currentSession.set(session);
        this.status.set('authenticated');
        this.authService.setTauriSession(session);
      }

      return {
        success: true,
        needsRestart: result.needsRestart,
        isSteward: result.isSteward,
      };
    } catch (err) {
      return {
        success: false,
        needsRestart: false,
        isSteward: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  /**
   * Unlock local identity with password (no network required).
   *
   * Used by returning users who already have doorway.json with an encrypted key bundle.
   * Decrypts the key bundle locally to prove identity.
   */
  async unlockWithPassword(password: string): Promise<{
    success: boolean;
    isSteward: boolean;
    error?: string;
  }> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) {
      return { success: false, isSteward: false, error: 'Tauri IPC not available' };
    }

    try {
      // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
      const result = await window.__TAURI__.core.invoke<DoorwayUnlockResult>(
        'doorway_unlock',
        { password }
      );

      this.isSteward.set(result.isSteward);

      // Promote pending session if available (set during initialize lock gate)
      const pending = this.pendingSession();
      if (pending) {
        this.currentSession.set(pending);
        this.pendingSession.set(null);
        this.authService.setTauriSession(pending);
      } else {
        // Fallback: fetch session from storage
        const session = await this.getActiveSessionWithRetry();
        if (session) {
          this.currentSession.set(session);
          this.authService.setTauriSession(session);
        }
      }
      this.status.set('authenticated');

      return { success: true, isSteward: result.isSteward };
    } catch (err) {
      return {
        success: false,
        isSteward: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  /**
   * Get doorway status from Tauri (reads doorway.json store).
   */
  async getDoorwayStatus(): Promise<DoorwayStatus | null> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) {
      return null;
    }

    try {
      // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
      return await window.__TAURI__.core.invoke<DoorwayStatus>('doorway_status');
    } catch {
      return null;
    }
  }

  /**
   * Refresh stewardship status from doorway.json via IPC.
   */
  private async refreshStewardshipStatus(): Promise<void> {
    const status = await this.getDoorwayStatus();
    if (status) {
      this.isSteward.set(status.isSteward);
    }
  }

  /**
   * Logout - delete local session and redirect to doorway picker.
   */
  async logout(): Promise<void> {
    const storageUrl = this.getStorageUrl();

    try {
      await fetch(`${storageUrl}/session`, { method: 'DELETE' });
    } catch (err) {
      // Session deletion failure is non-critical - user can still logout locally
      // This can happen if sidecar is not running or network is unavailable
      if (err instanceof Error) {
        console.warn('[TauriAuthService] Failed to delete session:', err.message);
      }
    }

    // Clear Rust doorway.json credentials (bootstrap URLs, agent key, etc.)
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (window.__TAURI__?.core) {
      try {
        // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
        await window.__TAURI__.core.invoke('doorway_logout');
      } catch (err) {
        if (err instanceof Error) {
          console.warn('[TauriAuthService] Failed to clear doorway store:', err.message);
        }
      }
    }

    this.currentSession.set(null);
    this.status.set('needs_login');
    void this.authService.logout();
    void this.router.navigate(['/identity']);
  }

  /**
   * Navigate to doorway picker for login.
   */
  navigateToLogin(): void {
    void this.router.navigate(['/identity']);
  }

  // ==========================================================================
  // Multi-Account Management
  // ==========================================================================

  /**
   * List all saved accounts from doorway.json.
   */
  async listAccounts(): Promise<AccountSummary[]> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) {
      return [];
    }

    try {
      // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
      return await window.__TAURI__.core.invoke<AccountSummary[]>('doorway_list_accounts');
    } catch {
      return [];
    }
  }

  /**
   * Switch active account by humanId. Requires app restart.
   */
  async switchAccount(humanId: string): Promise<{ needsRestart: boolean }> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) {
      return { needsRestart: false };
    }

    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    return await window.__TAURI__.core.invoke<{ needsRestart: boolean }>(
      'doorway_switch_account',
      { humanId }
    );
  }

  // ==========================================================================
  // Account Lifecycle
  // ==========================================================================

  /**
   * Soft lock — delete session, return to lock screen.
   * Does NOT clear identity data. User can unlock with password.
   */
  async lock(): Promise<void> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (window.__TAURI__?.core) {
      try {
        // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
        await window.__TAURI__.core.invoke('doorway_lock');
      } catch (err) {
        if (err instanceof Error) {
          console.warn('[TauriAuthService] Lock failed:', err.message);
        }
      }
    }

    this.currentSession.set(null);
    this.pendingSession.set(null);
    this.status.set('needs_unlock');
    void this.router.navigate(['/identity/login']);
  }

  /**
   * Remove a specific account by humanId.
   */
  async removeAccount(humanId: string): Promise<void> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) return;

    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    await window.__TAURI__.core.invoke('doorway_remove_account', { humanId });
  }

  /**
   * Reset all accounts — clears all identity data.
   */
  async resetAll(): Promise<void> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) return;

    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    await window.__TAURI__.core.invoke('doorway_reset');

    this.currentSession.set(null);
    this.pendingSession.set(null);
    this.status.set('needs_login');
    void this.authService.logout();
    void this.router.navigate(['/identity']);
  }

  /**
   * Deregister identity from doorway (revoke + remove locally).
   */
  async deregister(password: string): Promise<{ success: boolean; error?: string }> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) {
      return { success: false, error: 'Tauri IPC not available' };
    }

    try {
      // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
      await window.__TAURI__.core.invoke('doorway_deregister', { password });
      return { success: true };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  /**
   * Emergency wipe — delete all local data and exit app.
   */
  async emergencyWipe(): Promise<void> {
    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    if (!window.__TAURI__?.core) return;

    // eslint-disable-next-line unicorn/prefer-global-this -- Tauri API is on window
    await window.__TAURI__.core.invoke('doorway_emergency_wipe');
  }
}

/** Summary of a saved account (from Rust IPC) */
export interface AccountSummary {
  humanId: string;
  identifier: string;
  doorwayUrl: string;
  displayName?: string;
  isSteward: boolean;
  isActive: boolean;
}
