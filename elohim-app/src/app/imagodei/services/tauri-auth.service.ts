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

import { environment } from '../../../environments/environment';

import { AuthService } from './auth.service';
import { DoorwayRegistryService } from './doorway-registry.service';

/** Tauri global interface for event listening */
declare global {
  interface Window {
    __TAURI__?: {
      event: {
        listen: <T>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>;
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

export type TauriAuthStatus = 'idle' | 'checking' | 'needs_login' | 'authenticated' | 'error';

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

  // Computed state
  readonly isAuthenticated = computed(() => this.status() === 'authenticated');
  readonly needsLogin = computed(() => this.status() === 'needs_login');
  readonly isTauri = computed(() => this.isTauriEnvironment());

  // Event unsubscribe callbacks
  private unsubscribeOAuthCallback?: () => void;
  private unsubscribeDeepLinkError?: () => void;

  /**
   * Check if running in Tauri environment.
   */
  isTauriEnvironment(): boolean {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  }

  /**
   * Get the elohim-storage base URL.
   */
  private getStorageUrl(): string {
    return environment.client?.storageUrl || 'http://localhost:8090';
  }

  /**
   * Initialize Tauri auth - check for existing session.
   *
   * Called at app startup to detect first-run vs returning user.
   */
  async initialize(): Promise<void> {
    if (!this.isTauriEnvironment()) {
      console.log('[TauriAuth] Not in Tauri environment, skipping');
      return;
    }

    console.log('[TauriAuth] Initializing...');
    this.status.set('checking');

    try {
      // Check for existing local session
      const session = await this.getActiveSession();

      if (session) {
        console.log('[TauriAuth] Found existing session:', session.identifier);
        this.currentSession.set(session);
        this.status.set('authenticated');

        // Update auth service with session info
        this.authService.setTauriSession(session);
      } else {
        console.log('[TauriAuth] No session found, user needs to login');
        this.status.set('needs_login');
      }

      // Set up event listeners for OAuth callback
      await this.setupEventListeners();
    } catch (err) {
      console.error('[TauriAuth] Initialization failed:', err);
      this.status.set('error');
      this.errorMessage.set(err instanceof Error ? err.message : 'Initialization failed');
    }
  }

  /**
   * Set up Tauri event listeners for OAuth callbacks.
   */
  private async setupEventListeners(): Promise<void> {
    if (!window.__TAURI__?.event) {
      console.warn('[TauriAuth] Tauri event API not available');
      return;
    }

    const { listen } = window.__TAURI__.event;

    // Listen for OAuth callback from deep link
    this.unsubscribeOAuthCallback = await listen<OAuthCallbackPayload>(
      'oauth-callback',
      async event => {
        console.log('[TauriAuth] OAuth callback received');
        await this.handleOAuthCallback(event.payload);
      }
    );

    // Listen for deep link errors
    this.unsubscribeDeepLinkError = await listen<DeepLinkError>('deep-link-error', event => {
      console.error('[TauriAuth] Deep link error:', event.payload.message);
      this.status.set('error');
      this.errorMessage.set(event.payload.message);
    });

    console.log('[TauriAuth] Event listeners registered');
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

      const session = await response.json();
      // Convert snake_case to camelCase
      return {
        id: session.id,
        humanId: session.humanId,
        agentPubKey: session.agentPubKey,
        doorwayUrl: session.doorwayUrl,
        doorwayId: session.doorwayId,
        identifier: session.identifier,
        displayName: session.displayName,
        profileImageHash: session.profileImageHash,
        isActive: session.isActive,
        createdAt: session.createdAt,
        updatedAt: session.updatedAt,
        lastSyncedAt: session.lastSyncedAt,
        bootstrapUrl: session.bootstrapUrl,
      };
    } catch (err) {
      console.warn('[TauriAuth] Failed to get session:', err);
      return null;
    }
  }

  /**
   * Handle OAuth callback from Tauri deep link.
   */
  private async handleOAuthCallback(payload: OAuthCallbackPayload): Promise<void> {
    console.log('[TauriAuth] Processing OAuth callback...');
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

      const tokenData = await tokenResponse.json();
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
      console.log('[TauriAuth] Native handoff received:', handoff.identifier);

      // Step 3: Generate local agent public key
      // In a real implementation, this would come from the Holochain conductor
      // For now, use a placeholder that will be updated when Holochain connects
      const agentPubKey = 'pending-' + crypto.randomUUID();

      // Step 4: Create local session
      const session = await this.createSession({
        humanId: handoff.humanId,
        agentPubKey,
        doorwayUrl: handoff.doorwayUrl,
        doorwayId: handoff.doorwayId,
        identifier: handoff.identifier,
        displayName: handoff.displayName,
        profileImageHash: handoff.profileImageHash,
        bootstrapUrl: handoff.bootstrapUrl,
      });

      console.log('[TauriAuth] Session created:', session.id);
      this.currentSession.set(session);
      this.status.set('authenticated');

      // Update auth service
      this.authService.setTauriSession(session);

      // Navigate to main app
      this.router.navigate(['/lamad']);
    } catch (err) {
      console.error('[TauriAuth] OAuth callback failed:', err);
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

    const session = await response.json();
    return {
      id: session.id,
      humanId: session.humanId,
      agentPubKey: session.agentPubKey,
      doorwayUrl: session.doorwayUrl,
      doorwayId: session.doorwayId,
      identifier: session.identifier,
      displayName: session.displayName,
      profileImageHash: session.profileImageHash,
      isActive: session.isActive,
      createdAt: session.createdAt,
      updatedAt: session.updatedAt,
      lastSyncedAt: session.lastSyncedAt,
      bootstrapUrl: session.bootstrapUrl,
    };
  }

  /**
   * Logout - delete local session and redirect to doorway picker.
   */
  async logout(): Promise<void> {
    const storageUrl = this.getStorageUrl();

    try {
      await fetch(`${storageUrl}/session`, { method: 'DELETE' });
    } catch (err) {
      console.warn('[TauriAuth] Failed to delete session:', err);
    }

    this.currentSession.set(null);
    this.status.set('needs_login');
    this.authService.logout();
    this.router.navigate(['/identity']);
  }

  /**
   * Navigate to doorway picker for login.
   */
  navigateToLogin(): void {
    this.router.navigate(['/identity']);
  }
}
