/**
 * Identity Service - Unified identity management for Elohim app.
 *
 * Philosophy:
 * - Provides a single source of truth for identity state
 * - Abstracts the difference between session and Holochain identity
 * - Supports graceful migration from session to Holochain
 *
 * This service wraps:
 * - SessionHumanService for localStorage-based sessions
 * - HolochainClientService for Holochain zome calls
 * - SovereigntyService for sovereignty stage tracking
 *
 * It does NOT replace these services - they remain available for direct use.
 */

import { Injectable, inject, signal, computed, effect, untracked } from '@angular/core';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import { SessionHumanService } from './session-human.service';
import { SovereigntyService } from './sovereignty.service';
import { AuthService } from './auth.service';
import { PasswordAuthProvider } from './providers/password-auth.provider';
import {
  type IdentityState,
  type IdentityMode,
  type HumanProfile,
  type RegisterHumanRequest,
  type UpdateProfileRequest,
  type ProfileReach,
  type KeyLocation,
  type KeyBackupStatus,
  type HostingCostSummary,
  type NodeOperatorHostingIncome,
  INITIAL_IDENTITY_STATE,
  getInitials,
} from '../models/identity.model';
import { type PasswordCredentials, type AuthResult } from '../models/auth.model';

// =============================================================================
// Wire Format Types (internal - snake_case matches conductor response)
// =============================================================================

/** Human entry as returned from conductor */
interface HumanEntry {
  id: string;
  display_name: string;
  bio: string | null;
  affinities: string[];
  profile_reach: string;
  location: string | null;
  created_at: string;
  updated_at: string;
}

/** Attestation as returned from conductor */
interface AttestationEntry {
  action_hash: Uint8Array;
  attestation: {
    id: string;
    attestation_type: string;
    attester_id: string;
    recipient_id: string;
    evidence_json: string;
    issued_at: string;
  };
}

/** Session result from get_current_human / register_human */
interface HumanSessionResult {
  agent_pubkey: string;
  action_hash: Uint8Array;
  human: HumanEntry;
  session_started_at: string;
  attestations: AttestationEntry[];
}

/** Result from update_human_profile */
interface HumanUpdateResult {
  action_hash: Uint8Array;
  human: HumanEntry;
}

/** Payload for registering a human */
interface RegisterHumanPayload {
  display_name: string;
  bio?: string;
  affinities: string[];
  profile_reach: string;
  location?: string;
  email_hash?: string;
  passkey_credential_id?: string;
  external_identifiers_json: string;
}

/** Payload for updating human profile */
interface UpdateHumanPayload {
  display_name?: string;
  bio?: string;
  affinities?: string[];
  profile_reach?: string;
  location?: string;
}

// =============================================================================
// Type Mappers
// =============================================================================

/**
 * Map wire format Human to domain HumanProfile.
 */
function mapToProfile(entry: HumanEntry): HumanProfile {
  return {
    id: entry.id,
    displayName: entry.display_name,
    bio: entry.bio,
    affinities: entry.affinities ?? [],
    profileReach: entry.profile_reach as ProfileReach,
    location: entry.location,
    createdAt: entry.created_at,
    updatedAt: entry.updated_at,
  };
}

/**
 * Map domain RegisterHumanRequest to wire format.
 */
function toRegisterPayload(request: RegisterHumanRequest): RegisterHumanPayload {
  return {
    display_name: request.displayName,
    bio: request.bio,
    affinities: request.affinities,
    profile_reach: request.profileReach,
    location: request.location,
    external_identifiers_json: '{}',
  };
}

/**
 * Map domain UpdateProfileRequest to wire format.
 */
function toUpdatePayload(request: UpdateProfileRequest): UpdateHumanPayload {
  return {
    display_name: request.displayName,
    bio: request.bio,
    affinities: request.affinities,
    profile_reach: request.profileReach,
    location: request.location,
  };
}

// =============================================================================
// Identity Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class IdentityService {
  // Dependencies
  private readonly holochainClient = inject(HolochainClientService);
  private readonly sessionHumanService = inject(SessionHumanService);
  private readonly sovereigntyService = inject(SovereigntyService);
  private readonly authService = inject(AuthService);
  private readonly passwordProvider = inject(PasswordAuthProvider);

  // ==========================================================================
  // State
  // ==========================================================================

  /** Core identity state signal */
  private readonly identitySignal = signal<IdentityState>(INITIAL_IDENTITY_STATE);

  /** Guard to prevent re-entry during async operations */
  private isCheckingIdentity = false;

  /** Track if we've already tried checking Holochain identity (avoid retrying on error) */
  private hasCheckedHolochainIdentity = false;

  // ==========================================================================
  // Public Signals (read-only)
  // ==========================================================================

  /** Complete identity state */
  readonly identity = this.identitySignal.asReadonly();

  /** Current identity mode */
  readonly mode = computed(() => this.identitySignal().mode);

  /** Whether user is authenticated (session or Holochain) */
  readonly isAuthenticated = computed(() => this.identitySignal().isAuthenticated);

  /** Human ID */
  readonly humanId = computed(() => this.identitySignal().humanId);

  /** Display name for UI */
  readonly displayName = computed(() => this.identitySignal().displayName);

  /** Holochain agent public key */
  readonly agentPubKey = computed(() => this.identitySignal().agentPubKey);

  /** Full profile (may be null if not loaded) */
  readonly profile = computed(() => this.identitySignal().profile);

  /** Attestations earned */
  readonly attestations = computed(() => this.identitySignal().attestations);

  /** Whether identity is loading */
  readonly isLoading = computed(() => this.identitySignal().isLoading);

  /** Error message if any */
  readonly error = computed(() => this.identitySignal().error);

  // ==========================================================================
  // Derived Signals
  // ==========================================================================

  /** Whether user can access gated content (requires network authentication) */
  readonly canAccessGatedContent = computed(() => {
    const mode = this.identitySignal().mode;
    const isNetworkMode = mode === 'hosted' || mode === 'self-sovereign';
    return isNetworkMode && this.identitySignal().isAuthenticated;
  });

  /** Whether user has a session (can be upgraded) */
  readonly hasSession = computed(() =>
    this.sessionHumanService.hasSession()
  );

  /** Whether Holochain is connected */
  readonly isHolochainConnected = computed(() =>
    this.holochainClient.isConnected()
  );

  /** Whether user can upgrade from session to Holochain */
  readonly canUpgrade = computed(() =>
    this.hasSession() && this.isHolochainConnected() && this.mode() === 'session'
  );

  // ==========================================================================
  // Constructor
  // ==========================================================================

  constructor() {
    // Register password auth provider
    if (!this.authService.hasProvider('password')) {
      this.authService.registerProvider(this.passwordProvider);
    }

    // Watch for Holochain connection changes
    // Use untracked() to read identity mode without creating a dependency
    // This prevents the effect from re-running when identity state changes
    effect(() => {
      const isConnected = this.holochainClient.isConnected();

      // Read mode without tracking - only react to isConnected changes
      const currentMode = untracked(() => this.identitySignal().mode);

      if (isConnected && currentMode === 'session') {
        // Holochain just connected - check if we have an identity there
        this.checkHolochainIdentity();
      } else if (!isConnected && (currentMode === 'hosted' || currentMode === 'self-sovereign')) {
        // Holochain disconnected - fall back to session
        this.fallbackToSession();
      }
    });

    // Watch for auth state changes (login/logout)
    // Use untracked() to read identity mode without creating a dependency
    effect(() => {
      const auth = this.authService.auth();

      if (auth.isAuthenticated && auth.humanId && auth.agentPubKey) {
        // Read mode without tracking - only react to auth changes
        const currentMode = untracked(() => this.identitySignal().mode);
        if (currentMode === 'session' || currentMode === 'anonymous') {
          // Connect to Holochain as this authenticated user
          this.connectAsAuthenticatedUser(auth.humanId, auth.agentPubKey);
        }
      }
    });

    // Initialize identity state
    this.initializeIdentity();
  }

  // ==========================================================================
  // Initialization
  // ==========================================================================

  /**
   * Initialize identity state based on current connections.
   */
  private async initializeIdentity(): Promise<void> {
    // Start with session identity if available
    const session = this.sessionHumanService.getSession();

    if (session) {
      // Determine mode based on session state
      const mode: IdentityMode = session.isAnonymous ? 'session' : 'session';

      this.updateState({
        mode,
        isAuthenticated: true,
        humanId: session.sessionId,
        displayName: session.displayName,
        agentPubKey: session.linkedAgentPubKey ?? null,
        sovereigntyStage: 'visitor',

        // Session-specific state
        keyLocation: 'none',
        canExportKeys: false,
        keyBackup: null,
        isLocalConductor: false,
        conductorUrl: null,
        linkedSessionId: null,
        hasPendingMigration: session.sessionState === 'upgrading',
        hostingCost: null,
        nodeOperatorIncome: null,
      });
    }

    // If Holochain is connected, check for identity there
    if (this.holochainClient.isConnected()) {
      await this.checkHolochainIdentity();
    }
  }

  /**
   * Check if we have an identity in Holochain.
   * This is optional - visitors can browse without a Holochain identity.
   */
  private async checkHolochainIdentity(): Promise<void> {
    // Prevent re-entry while already checking
    if (this.isCheckingIdentity) {
      return;
    }

    // Don't retry if we've already checked (successful or not)
    if (this.hasCheckedHolochainIdentity) {
      return;
    }

    this.isCheckingIdentity = true;
    this.hasCheckedHolochainIdentity = true;
    this.updateState({ isLoading: true, error: null });

    try {
      // Call imagodei DNA to get current human profile
      // Note: get_my_human returns HumanOutput { action_hash, human } not full HumanSessionResult
      const result = await this.holochainClient.callZome<HumanSessionResult | null>({
        zomeName: 'imagodei',
        fnName: 'get_my_human',
        payload: null,
        roleName: 'imagodei',  // Use imagodei DNA for identity
      });

      if (result.success && result.data) {
        const sessionResult = result.data;

        // Determine conductor type and key location
        const conductorInfo = this.detectConductorType();
        const identityMode = conductorInfo.isLocal ? 'self-sovereign' : 'hosted';
        const keyLocation = conductorInfo.isLocal ? 'device' : 'custodial';
        const sovereigntyStage = conductorInfo.isLocal ? 'app-user' : 'hosted';

        // Check if session exists alongside Holochain
        const session = this.sessionHumanService.getSession();
        const linkedSessionId = session?.sessionId ?? null;

        this.updateState({
          mode: identityMode,
          isAuthenticated: true,
          humanId: sessionResult.human.id,
          displayName: sessionResult.human.display_name,
          agentPubKey: sessionResult.agent_pubkey,
          profile: mapToProfile(sessionResult.human),
          attestations: sessionResult.attestations.map(a => a.attestation.attestation_type),
          sovereigntyStage,

          // Key management
          keyLocation,
          canExportKeys: keyLocation === 'custodial', // Can export from hosted to device
          keyBackup: null, // TODO: Fetch from conductor if available

          // Conductor info
          isLocalConductor: conductorInfo.isLocal,
          conductorUrl: conductorInfo.url,

          // Session link
          linkedSessionId,
          hasPendingMigration: session?.sessionState === 'upgrading',

          // Hosting costs - TODO: Fetch from conductor
          hostingCost: sovereigntyStage === 'hosted' ? this.getDefaultHostingCost() : null,
          nodeOperatorIncome: null, // TODO: Fetch if node-operator

          isLoading: false,
        });

        // If session exists, link it to this Holochain identity
        if (session && session.sessionState !== 'linked' && session.sessionState !== 'migrated') {
          this.sessionHumanService.linkToHolochainIdentity(
            sessionResult.agent_pubkey,
            sessionResult.human.id
          );
        }
      } else {
        // No Holochain identity - keep session mode
        this.updateState({ isLoading: false });
      }
    } catch (err) {
      // This is expected for visitors - the zome function may not exist or user may not be registered
      // Don't treat this as an error - just stay in session mode
      const errorMessage = err instanceof Error ? err.message : String(err);
      const isExpectedError = errorMessage.includes("doesn't exist") ||
                              errorMessage.includes('not found') ||
                              errorMessage.includes('No human found');

      if (isExpectedError) {
        console.log('[IdentityService] No Holochain identity found, staying in session mode');
      } else {
        console.warn('[IdentityService] Unexpected error checking Holochain identity:', err);
      }

      // Clear loading state, don't set error for expected cases
      this.updateState({
        isLoading: false,
        error: isExpectedError ? null : errorMessage,
      });
    } finally {
      this.isCheckingIdentity = false;
    }
  }

  /**
   * Detect whether connected to local or remote conductor.
   * TODO: Implement proper detection via conductor metadata
   */
  private detectConductorType(): { isLocal: boolean; url: string | null } {
    const displayInfo = this.holochainClient.getDisplayInfo();

    // Use appUrl to determine if local or remote
    const url = displayInfo.appUrl ?? null;
    const isLocal = url ? (
      url.includes('localhost') ||
      url.includes('127.0.0.1') ||
      url.includes('[::1]')
    ) : false;

    return { isLocal, url };
  }

  /**
   * Get default hosting cost for new hosted users.
   */
  private getDefaultHostingCost(): HostingCostSummary {
    return {
      coverageSource: 'commons',
      monthlyCostDisplay: 'Free (Commons)',
      storageUsedDisplay: '0 MB',
      migrationRecommended: false,
    };
  }

  /**
   * Fall back to session identity when Holochain disconnects.
   */
  private fallbackToSession(): void {
    const session = this.sessionHumanService.getSession();

    if (session) {
      // Determine mode based on session state
      const mode: IdentityMode = session.sessionState === 'linked' ? 'session' : 'session';
      const isAuthenticated = session.sessionState !== 'migrated';

      this.updateState({
        mode,
        isAuthenticated,
        humanId: session.sessionId,
        displayName: session.displayName,
        agentPubKey: session.linkedAgentPubKey ?? null,
        profile: null,
        attestations: [],
        sovereigntyStage: 'visitor',

        // Key management - no keys in session mode
        keyLocation: 'none',
        canExportKeys: false,
        keyBackup: null,

        // Conductor - disconnected
        isLocalConductor: false,
        conductorUrl: null,

        // Session link - preserve if was linked
        linkedSessionId: session.linkedHumanId ? session.sessionId : null,
        hasPendingMigration: session.sessionState === 'upgrading',

        // No hosting costs in visitor mode
        hostingCost: null,
        nodeOperatorIncome: null,
      });
    } else {
      this.updateState(INITIAL_IDENTITY_STATE);
    }
  }

  // ==========================================================================
  // Registration
  // ==========================================================================

  /**
   * Register a new human identity in Holochain.
   */
  async registerHuman(request: RegisterHumanRequest): Promise<HumanProfile> {
    if (!this.holochainClient.isConnected()) {
      throw new Error('Holochain not connected');
    }

    this.updateState({ isLoading: true, error: null });

    try {
      const payload = toRegisterPayload(request);
      // Call imagodei DNA to create human profile
      const result = await this.holochainClient.callZome<HumanSessionResult>({
        zomeName: 'imagodei',
        fnName: 'create_human',
        payload,
        roleName: 'imagodei',  // Use imagodei DNA for identity
      });

      if (!result.success || !result.data) {
        throw new Error(result.error ?? 'Registration failed');
      }

      const sessionResult = result.data;
      const profile = mapToProfile(sessionResult.human);

      // Determine conductor type
      const conductorInfo = this.detectConductorType();
      const identityMode = conductorInfo.isLocal ? 'self-sovereign' : 'hosted';
      const keyLocation = conductorInfo.isLocal ? 'device' : 'custodial';
      const sovereigntyStage = conductorInfo.isLocal ? 'app-user' : 'hosted';

      // Get session for linking
      const session = this.sessionHumanService.getSession();

      this.updateState({
        mode: identityMode,
        isAuthenticated: true,
        humanId: sessionResult.human.id,
        displayName: sessionResult.human.display_name,
        agentPubKey: sessionResult.agent_pubkey,
        profile,
        attestations: sessionResult.attestations.map(a => a.attestation.attestation_type),
        sovereigntyStage,

        // Key management
        keyLocation,
        canExportKeys: keyLocation === 'custodial',
        keyBackup: null,

        // Conductor info
        isLocalConductor: conductorInfo.isLocal,
        conductorUrl: conductorInfo.url,

        // Session link
        linkedSessionId: session?.sessionId ?? null,
        hasPendingMigration: false,

        // Hosting costs
        hostingCost: sovereigntyStage === 'hosted' ? this.getDefaultHostingCost() : null,
        nodeOperatorIncome: null,

        isLoading: false,
      });

      // Mark session as migrated if it exists
      if (session) {
        this.sessionHumanService.markAsMigrated(
          sessionResult.agent_pubkey,
          sessionResult.human.id
        );
      }

      return profile;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Registration failed';
      this.updateState({ isLoading: false, error: errorMessage });
      throw err;
    }
  }

  // ==========================================================================
  // Profile Management
  // ==========================================================================

  /**
   * Get current human profile from Holochain.
   */
  async getCurrentHuman(): Promise<HumanProfile | null> {
    if (!this.holochainClient.isConnected()) {
      return null;
    }

    try {
      // Call imagodei DNA to get current human profile
      const result = await this.holochainClient.callZome<HumanSessionResult | null>({
        zomeName: 'imagodei',
        fnName: 'get_my_human',
        payload: null,
        roleName: 'imagodei',  // Use imagodei DNA for identity
      });

      if (result.success && result.data) {
        const profile = mapToProfile(result.data.human);
        this.updateState({ profile });
        return profile;
      }

      return null;
    } catch (err) {
      console.error('[IdentityService] Failed to get current human:', err);
      return null;
    }
  }

  /**
   * Update the current human's profile.
   */
  async updateProfile(request: UpdateProfileRequest): Promise<HumanProfile> {
    if (!this.holochainClient.isConnected()) {
      throw new Error('Holochain not connected');
    }

    const mode = this.mode();
    const isNetworkMode = mode === 'hosted' || mode === 'self-sovereign';
    if (!isNetworkMode) {
      throw new Error('Cannot update profile in session mode');
    }

    this.updateState({ isLoading: true, error: null });

    try {
      const payload = toUpdatePayload(request);
      // Call imagodei DNA to update human profile
      const result = await this.holochainClient.callZome<HumanUpdateResult>({
        zomeName: 'imagodei',
        fnName: 'update_human',
        payload,
        roleName: 'imagodei',  // Use imagodei DNA for identity
      });

      if (!result.success || !result.data) {
        throw new Error(result.error ?? 'Update failed');
      }

      const profile = mapToProfile(result.data.human);

      this.updateState({
        displayName: profile.displayName,
        profile,
        isLoading: false,
      });

      return profile;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Update failed';
      this.updateState({ isLoading: false, error: errorMessage });
      throw err;
    }
  }

  // ==========================================================================
  // Authentication (Login/Logout)
  // ==========================================================================

  /**
   * Login with email/username and password.
   *
   * @param identifier - Email or username
   * @param password - Password
   * @returns Authentication result
   */
  async loginWithPassword(identifier: string, password: string): Promise<AuthResult> {
    const credentials: PasswordCredentials = {
      type: 'password',
      identifier,
      password,
    };

    return this.authService.login('password', credentials);
  }

  /**
   * Logout and return to visitor session.
   */
  async logout(): Promise<void> {
    // Logout from auth service
    await this.authService.logout();

    // Fall back to session identity
    this.fallbackToSession();

    console.log('[IdentityService] Logged out, reverted to session');
  }

  /**
   * Connect to Holochain as an authenticated user (after login).
   * This is called by the auth state effect when login succeeds.
   */
  private async connectAsAuthenticatedUser(humanId: string, agentPubKey: string): Promise<void> {
    // For hosted mode, the edge node holds the keys
    // We need to verify the identity and load the profile

    if (!this.holochainClient.isConnected()) {
      console.log('[IdentityService] Waiting for Holochain connection to verify auth');
      return;
    }

    try {
      // Verify the identity by fetching the current human from imagodei DNA
      const result = await this.holochainClient.callZome<HumanSessionResult | null>({
        zomeName: 'imagodei',
        fnName: 'get_my_human',
        payload: null,
        roleName: 'imagodei',  // Use imagodei DNA for identity
      });

      if (result.success && result.data) {
        const sessionResult = result.data;

        // Verify the humanId matches
        if (sessionResult.human.id !== humanId) {
          console.warn('[IdentityService] HumanId mismatch after login');
          // Continue anyway - the auth token is still valid
        }

        // Update identity state
        const conductorInfo = this.detectConductorType();
        const identityMode = conductorInfo.isLocal ? 'self-sovereign' : 'hosted';
        const keyLocation = conductorInfo.isLocal ? 'device' : 'custodial';
        const sovereigntyStage = conductorInfo.isLocal ? 'app-user' : 'hosted';

        this.updateState({
          mode: identityMode,
          isAuthenticated: true,
          humanId: sessionResult.human.id,
          displayName: sessionResult.human.display_name,
          agentPubKey: sessionResult.agent_pubkey,
          profile: mapToProfile(sessionResult.human),
          attestations: sessionResult.attestations.map(a => a.attestation.attestation_type),
          sovereigntyStage,
          keyLocation,
          canExportKeys: keyLocation === 'custodial',
          isLocalConductor: conductorInfo.isLocal,
          conductorUrl: conductorInfo.url,
          hostingCost: sovereigntyStage === 'hosted' ? this.getDefaultHostingCost() : null,
          isLoading: false,
          error: null,
        });

        console.log('[IdentityService] Connected as authenticated user:', sessionResult.human.display_name);
      }
    } catch (err) {
      console.error('[IdentityService] Failed to verify authenticated user:', err);
    }
  }

  // ==========================================================================
  // Display Helpers
  // ==========================================================================

  /**
   * Get display information for UI components.
   */
  getDisplayInfo(): {
    name: string;
    initials: string;
    avatarUrl: string | null;
    mode: IdentityMode;
  } {
    const identity = this.identitySignal();

    return {
      name: identity.displayName,
      initials: getInitials(identity.displayName),
      avatarUrl: identity.profile?.avatarUrl ?? null,
      mode: identity.mode,
    };
  }

  /**
   * Clear any error state.
   */
  clearError(): void {
    this.updateState({ error: null });
  }

  // ==========================================================================
  // State Management
  // ==========================================================================

  /**
   * Update identity state (partial update).
   */
  private updateState(partial: Partial<IdentityState>): void {
    this.identitySignal.update(current => ({
      ...current,
      ...partial,
    }));
  }
}
