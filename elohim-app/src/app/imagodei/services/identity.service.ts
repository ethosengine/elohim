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
import { type PasswordCredentials, type AuthResult } from '../models/auth.model';
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
  isStewardMode,
  isNetworkMode,
} from '../models/identity.model';

import { AuthService } from './auth.service';
import { PasswordAuthProvider } from './providers/password-auth.provider';
import { SessionHumanService } from './session-human.service';
import { SovereigntyService } from './sovereignty.service';

// Re-export utility functions for consumers
export { isNetworkMode, isStewardMode, getInitials } from '../models/identity.model';

// =============================================================================
// Wire Format Types (internal - snake_case matches conductor response)
// =============================================================================

/** Human entry as returned from conductor */
interface HumanEntry {
  id: string;
  displayName: string;
  bio: string | null;
  affinities: string[];
  profileReach: string;
  location: string | null;
  createdAt: string;
  updatedAt: string;
}

/** Attestation as returned from conductor */
interface AttestationEntry {
  actionHash: Uint8Array;
  attestation: {
    id: string;
    attestationType: string;
    attesterId: string;
    recipientId: string;
    evidenceJson: string;
    issuedAt: string;
  };
}

/** Session result from get_current_human / register_human */
interface HumanSessionResult {
  agentPubkey: string;
  actionHash: Uint8Array;
  human: HumanEntry;
  sessionStartedAt: string;
  attestations: AttestationEntry[];
}

/** Result from update_human_profile */
interface HumanUpdateResult {
  actionHash: Uint8Array;
  human: HumanEntry;
}

/** Payload for registering a human */
interface RegisterHumanPayload {
  id: string;
  displayName: string;
  bio?: string;
  affinities: string[];
  profileReach: string;
  location?: string;
}

/** Payload for updating human profile */
interface UpdateHumanPayload {
  displayName?: string;
  bio?: string;
  affinities?: string[];
  profileReach?: string;
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
    displayName: entry.displayName,
    bio: entry.bio,
    affinities: entry.affinities ?? [],
    profileReach: entry.profileReach as ProfileReach,
    location: entry.location,
    createdAt: entry.createdAt,
    updatedAt: entry.updatedAt,
  };
}

/**
 * Generate a unique human ID.
 * Uses crypto.randomUUID() for secure random UUIDs.
 */
function generateHumanId(): string {
  return crypto.randomUUID();
}

/**
 * Map domain RegisterHumanRequest to wire format.
 */
function toRegisterPayload(request: RegisterHumanRequest): RegisterHumanPayload {
  return {
    id: generateHumanId(),
    displayName: request.displayName,
    bio: request.bio,
    affinities: request.affinities,
    profileReach: request.profileReach,
    location: request.location,
  };
}

/**
 * Map domain UpdateProfileRequest to wire format.
 */
function toUpdatePayload(request: UpdateProfileRequest): UpdateHumanPayload {
  return {
    displayName: request.displayName,
    bio: request.bio,
    affinities: request.affinities,
    profileReach: request.profileReach,
    location: request.location,
  };
}

// =============================================================================
// DID Generation
// =============================================================================

/**
 * Gateway domain for session DIDs.
 * In production, this would come from environment configuration.
 */
const GATEWAY_DOMAIN = 'gateway.elohim.host';

/**
 * Hosted domain for hosted identity DIDs.
 */
const HOSTED_DOMAIN = 'hosted.elohim.host';

/**
 * Generate a W3C DID based on identity mode.
 *
 * DID patterns by mode (mapped to Elohim trust tiers):
 * - anonymous: null (no DID for travelers)
 * - session: did:web:gateway.elohim.host:session:{sessionId} (ephemeral visitor)
 * - hosted: did:web:hosted.elohim.host:humans:{humanId} (custodial, medium trust)
 * - self-sovereign (steward): did:key:{multibase-pubkey} (cryptographic, highest trust)
 * - migrating: keeps previous DID during migration
 *
 * Note: The mode value 'self-sovereign' maps to "steward" in Elohim terminology.
 * Stewards own their keys and operate as first-class network participants.
 *
 * @param mode - Current identity mode
 * @param humanId - Human ID (for session/hosted)
 * @param agentPubKey - Agent public key base64 (for steward/self-sovereign)
 * @param sessionId - Session ID (for session mode fallback)
 */
function generateDID(
  mode: IdentityMode,
  humanId: string | null,
  agentPubKey: string | null,
  sessionId: string | null
): string | null {
  switch (mode) {
    case 'anonymous':
      return null;

    case 'session':
      // Session-based DID using session ID (ephemeral visitor)
      if (humanId) {
        return `did:web:${GATEWAY_DOMAIN}:session:${humanId}`;
      }
      if (sessionId) {
        return `did:web:${GATEWAY_DOMAIN}:session:${sessionId}`;
      }
      return null;

    case 'hosted':
      // Hosted identity DID using human ID (custodial keys)
      if (humanId) {
        return `did:web:${HOSTED_DOMAIN}:humans:${humanId}`;
      }
      return null;

    case 'steward':
    case 'self-sovereign': // @deprecated - use 'steward'
      // Steward DID using agent public key (cryptographic, self-custodied keys)
      // did:key uses multibase encoding - we use z prefix for base58btc
      if (agentPubKey) {
        // Convert base64 agentPubKey to multibase format
        // For simplicity, we use the raw pubkey with z prefix
        // A full implementation would use proper multicodec encoding
        return `did:key:z${agentPubKey.replace(/[+/=]/g, '')}`;
      }
      return null;

    case 'migrating':
      // During migration, DID should be preserved from previous state
      // The caller should handle this by not updating DID during migration
      return null;

    default:
      return null;
  }
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

  /** W3C Decentralized Identifier for this identity */
  readonly did = computed(() => this.identitySignal().did);

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
    return isNetworkMode(mode) && this.identitySignal().isAuthenticated;
  });

  /** Whether user has a session (can be upgraded) */
  readonly hasSession = computed(() => this.sessionHumanService.hasSession());

  /** Whether Holochain is connected */
  readonly isHolochainConnected = computed(() => this.holochainClient.isConnected());

  /** Whether user can upgrade from session to Holochain */
  readonly canUpgrade = computed(
    () => this.hasSession() && this.isHolochainConnected() && this.mode() === 'session'
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
      } else if (!isConnected && isNetworkMode(currentMode)) {
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

      // Generate DID for session identity
      const did = generateDID(
        mode,
        session.sessionId,
        session.linkedAgentPubKey ?? null,
        session.sessionId
      );

      this.updateState({
        mode,
        isAuthenticated: true,
        humanId: session.sessionId,
        displayName: session.displayName,
        agentPubKey: session.linkedAgentPubKey ?? null,
        did,
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

    // Check for restored auth session that needs identity fetch
    // This handles the case where AuthService restored a token from localStorage
    // but doesn't have humanId/agentPubKey (they weren't persisted)
    await this.fetchRestoredSessionIdentity();
  }

  /**
   * Fetch identity from server if we have a restored auth session without humanId/agentPubKey.
   * This completes the session restoration by fetching the missing identity data.
   */
  private async fetchRestoredSessionIdentity(): Promise<void> {
    const auth = this.authService.auth();

    // Only fetch if we have a token but missing identity fields
    if (!auth.isAuthenticated || !auth.token || (auth.humanId && auth.agentPubKey)) {
      return;
    }

    console.log('[IdentityService] Fetching identity for restored session...');

    // Use the password provider to fetch current user from /auth/me
    const provider = this.authService.getProvider('password') as PasswordAuthProvider | undefined;
    if (!provider?.getCurrentUser) {
      console.warn('[IdentityService] No provider available to fetch identity');
      return;
    }

    try {
      const identity = await provider.getCurrentUser(auth.token);
      if (identity) {
        console.log('[IdentityService] Fetched identity:', identity.humanId);
        // Now connect as the authenticated user
        await this.connectAsAuthenticatedUser(identity.humanId, identity.agentPubKey);
      } else {
        console.warn('[IdentityService] Failed to fetch identity from server');
      }
    } catch (err) {
      console.error('[IdentityService] Error fetching identity:', err);
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
        roleName: 'imagodei', // Use imagodei DNA for identity
      });

      if (result.success && result.data) {
        const sessionResult = result.data;

        // Determine conductor type and key location
        const conductorInfo = this.detectConductorType();
        const identityMode = conductorInfo.isLocal ? 'steward' : 'hosted';
        const keyLocation = conductorInfo.isLocal ? 'device' : 'custodial';
        const sovereigntyStage = conductorInfo.isLocal ? 'app-user' : 'hosted';

        // Check if session exists alongside Holochain
        const session = this.sessionHumanService.getSession();
        const linkedSessionId = session?.sessionId ?? null;

        // Generate DID for this identity
        const did = generateDID(
          identityMode,
          sessionResult.human.id,
          sessionResult.agentPubkey,
          linkedSessionId
        );

        this.updateState({
          mode: identityMode,
          isAuthenticated: true,
          humanId: sessionResult.human.id,
          displayName: sessionResult.human.displayName,
          agentPubKey: sessionResult.agentPubkey,
          did,
          profile: mapToProfile(sessionResult.human),
          attestations: sessionResult.attestations.map(a => a.attestation.attestationType),
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
            sessionResult.agentPubkey,
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
      const isExpectedError =
        errorMessage.includes("doesn't exist") ||
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
    const isLocal = url
      ? url.includes('localhost') || url.includes('127.0.0.1') || url.includes('[::1]')
      : false;

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

      // Generate DID for session identity
      const did = generateDID(
        mode,
        session.sessionId,
        session.linkedAgentPubKey ?? null,
        session.sessionId
      );

      this.updateState({
        mode,
        isAuthenticated,
        humanId: session.sessionId,
        displayName: session.displayName,
        agentPubKey: session.linkedAgentPubKey ?? null,
        did,
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
   * Register a new human identity via doorway (hosted mode).
   *
   * Doorway handles atomically:
   * 1. Creating Holochain identity via imagodei zome
   * 2. Storing auth credentials in MongoDB
   * 3. Returning JWT + profile
   *
   * For native/steward mode (local conductor), use registerHumanNative().
   */
  async registerHuman(request: RegisterHumanRequest): Promise<HumanProfile> {
    // Validate email is provided
    if (!request.email) {
      throw new Error('Email is required for registration');
    }

    if (!request.password) {
      throw new Error('Password is required for registration');
    }

    this.updateState({ isLoading: true, error: null });

    try {
      // Register via doorway (handles identity creation + credentials atomically)
      const authResult = await this.authService.register('password', {
        identifier: request.email,
        identifierType: 'email',
        password: request.password,
        displayName: request.displayName,
        bio: request.bio,
        affinities: request.affinities,
        profileReach: request.profileReach,
        location: request.location,
      });

      if (!authResult.success) {
        throw new Error(authResult.error);
      }

      // Auth service effect will update identity state
      // Build profile from auth result
      const profile: HumanProfile = {
        id: authResult.humanId,
        displayName: request.displayName,
        bio: request.bio ?? null,
        affinities: request.affinities,
        profileReach: request.profileReach,
        location: request.location ?? null,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };

      // Update state with registered profile
      const session = this.sessionHumanService.getSession();
      const did = generateDID(
        'hosted',
        authResult.humanId,
        authResult.agentPubKey,
        session?.sessionId ?? null
      );

      this.updateState({
        mode: 'hosted',
        isAuthenticated: true,
        humanId: authResult.humanId,
        displayName: request.displayName,
        agentPubKey: authResult.agentPubKey,
        did,
        profile,
        attestations: [],
        sovereigntyStage: 'hosted',
        keyLocation: 'custodial',
        canExportKeys: true,
        keyBackup: null,
        isLocalConductor: false,
        conductorUrl: null,
        linkedSessionId: session?.sessionId ?? null,
        hasPendingMigration: false,
        hostingCost: this.getDefaultHostingCost(),
        nodeOperatorIncome: null,
        isLoading: false,
      });

      // Mark session as migrated if it exists
      if (session) {
        this.sessionHumanService.markAsMigrated(authResult.agentPubKey, authResult.humanId);
      }

      console.log('[IdentityService] Registered via doorway:', authResult.humanId);
      return profile;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Registration failed';
      this.updateState({ isLoading: false, error: errorMessage });
      throw err;
    }
  }

  /**
   * Register a new human identity via local conductor (steward mode).
   *
   * Used when running with a local conductor (native app or dev mode).
   * Calls the imagodei zome directly.
   */
  async registerHumanNative(request: RegisterHumanRequest): Promise<HumanProfile> {
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
        roleName: 'imagodei', // Use imagodei DNA for identity
      });

      if (!result.success || !result.data) {
        throw new Error(result.error ?? 'Registration failed');
      }

      const sessionResult = result.data;
      const profile = mapToProfile(sessionResult.human);

      // Get session for linking
      const session = this.sessionHumanService.getSession();

      // Generate DID for steward identity
      const did = generateDID(
        'steward',
        sessionResult.human.id,
        sessionResult.agentPubkey,
        session?.sessionId ?? null
      );

      this.updateState({
        mode: 'steward',
        isAuthenticated: true,
        humanId: sessionResult.human.id,
        displayName: sessionResult.human.displayName,
        agentPubKey: sessionResult.agentPubkey,
        did,
        profile,
        attestations: sessionResult.attestations.map(a => a.attestation.attestationType),
        sovereigntyStage: 'app-user',
        keyLocation: 'device',
        canExportKeys: false,
        keyBackup: null,
        isLocalConductor: true,
        conductorUrl: this.detectConductorType().url,
        linkedSessionId: session?.sessionId ?? null,
        hasPendingMigration: false,
        hostingCost: null,
        nodeOperatorIncome: null,
        isLoading: false,
      });

      // Mark session as migrated if it exists
      if (session) {
        this.sessionHumanService.markAsMigrated(sessionResult.agentPubkey, sessionResult.human.id);
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
        roleName: 'imagodei', // Use imagodei DNA for identity
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
    if (!isNetworkMode(mode)) {
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
        roleName: 'imagodei', // Use imagodei DNA for identity
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
   * Wait for identity to be fully authenticated (hosted or steward mode).
   *
   * Use this after login to ensure the identity state is fully established
   * before navigating away from the login page.
   *
   * @param timeoutMs - Maximum time to wait in milliseconds (default: 3000)
   * @returns True if authenticated state was reached, false if timed out
   */
  async waitForAuthenticatedState(timeoutMs = 3000): Promise<boolean> {
    // If already in authenticated mode, return immediately
    const currentMode = this.mode();
    if (currentMode === 'hosted' || currentMode === 'steward') {
      return true;
    }

    // Wait for mode to change or timeout
    return new Promise(resolve => {
      const startTime = Date.now();
      const checkInterval = setInterval(() => {
        const mode = this.mode();
        if (mode === 'hosted' || mode === 'steward') {
          clearInterval(checkInterval);
          resolve(true);
        } else if (Date.now() - startTime > timeoutMs) {
          clearInterval(checkInterval);
          console.log('[IdentityService] Timeout waiting for authenticated state');
          resolve(false);
        }
      }, 100);
    });
  }

  /**
   * Connect to Holochain as an authenticated user (after login).
   * This is called by the auth state effect when login succeeds.
   */
  private async connectAsAuthenticatedUser(humanId: string, agentPubKey: string): Promise<void> {
    // For hosted mode, the edge node holds the keys
    // We need to verify the identity and load the profile

    if (!this.holochainClient.isConnected()) {
      console.log('[IdentityService] Waiting for Holochain connection...');

      // Wait up to 10 seconds for connection
      const connected = await this.waitForHolochainConnection(10000);
      if (!connected) {
        console.warn(
          '[IdentityService] Timeout waiting for Holochain - setting hosted mode without profile'
        );
        // Still update state to show logged-in UI, just without full profile
        this.setMinimalAuthenticatedState(humanId, agentPubKey);
        return;
      }
    }

    try {
      // Verify the identity by fetching the current human from imagodei DNA
      const result = await this.holochainClient.callZome<HumanSessionResult | null>({
        zomeName: 'imagodei',
        fnName: 'get_my_human',
        payload: null,
        roleName: 'imagodei', // Use imagodei DNA for identity
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
        const identityMode = conductorInfo.isLocal ? 'steward' : 'hosted';
        const keyLocation = conductorInfo.isLocal ? 'device' : 'custodial';
        const sovereigntyStage = conductorInfo.isLocal ? 'app-user' : 'hosted';

        // Generate DID for authenticated identity
        const did = generateDID(
          identityMode,
          sessionResult.human.id,
          sessionResult.agentPubkey,
          null
        );

        this.updateState({
          mode: identityMode,
          isAuthenticated: true,
          humanId: sessionResult.human.id,
          displayName: sessionResult.human.displayName,
          agentPubKey: sessionResult.agentPubkey,
          did,
          profile: mapToProfile(sessionResult.human),
          attestations: sessionResult.attestations.map(a => a.attestation.attestationType),
          sovereigntyStage,
          keyLocation,
          canExportKeys: keyLocation === 'custodial',
          isLocalConductor: conductorInfo.isLocal,
          conductorUrl: conductorInfo.url,
          hostingCost: sovereigntyStage === 'hosted' ? this.getDefaultHostingCost() : null,
          isLoading: false,
          error: null,
        });

        console.log(
          '[IdentityService] Connected as authenticated user:',
          sessionResult.human.displayName
        );
      }
    } catch (err) {
      console.error('[IdentityService] Failed to verify authenticated user:', err);
    }
  }

  /**
   * Wait for Holochain connection with timeout.
   *
   * @param timeoutMs - Maximum time to wait in milliseconds
   * @returns True if connected, false if timeout
   */
  private async waitForHolochainConnection(timeoutMs: number): Promise<boolean> {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      if (this.holochainClient.isConnected()) {
        return true;
      }
      await new Promise(resolve => setTimeout(resolve, 500));
    }
    return false;
  }

  /**
   * Set minimal authenticated state when Holochain is unavailable.
   * This allows the UI to show logged-in state even without full profile.
   *
   * @param humanId - The authenticated human ID
   * @param agentPubKey - The agent public key
   */
  private setMinimalAuthenticatedState(humanId: string, agentPubKey: string): void {
    // Generate DID for hosted identity
    const did = generateDID('hosted', humanId, agentPubKey, null);

    this.updateState({
      mode: 'hosted',
      isAuthenticated: true,
      humanId,
      agentPubKey,
      did,
      displayName: humanId, // Use humanId as fallback display name
      sovereigntyStage: 'hosted',
      keyLocation: 'custodial',
      canExportKeys: true,
      isLocalConductor: false,
      conductorUrl: null,
      hostingCost: this.getDefaultHostingCost(),
      isLoading: false,
      error: null,
    });

    console.log('[IdentityService] Set minimal authenticated state for:', humanId);
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
