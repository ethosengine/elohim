import { Injectable } from '@angular/core';

// @coverage: 94.9% (2026-02-24)

import { BehaviorSubject, Observable } from 'rxjs';

import {
  ContentAccessMetadata,
  AccessCheckResult,
  AccessLevel,
  AccessAction,
} from './content-access.model';
import {
  SessionHuman,
  SessionPathProgress,
  SessionMigration,
  HolochainUpgradePrompt,
  SessionState,
  UpgradeIntent,
} from './session-human.model';

/**
 * SessionHumanService - Manages temporary session identity for MVP.
 *
 * Philosophy:
 * - Zero-friction entry: humans explore immediately
 * - Progress persists in localStorage during session
 * - Meaningful moments prompt Holochain "upgrade" (prompts now driven by
 *   UpgradePromptView projection via /api/v1/identity/{agentId}/upgrade-prompts)
 * - Migration preserves all session progress
 *
 * Holochain migration:
 * - This service becomes a thin wrapper around HolochainService
 * - Session data migrates to agent's private source chain
 * - sessionId maps to AgentPubKey
 *
 * Storage keys:
 * - lamad-session: SessionHuman object
 * - lamad-session-{sessionId}-affinity: Affinity data
 * - lamad-session-{sessionId}-progress-{pathId}: Path progress
 *
 * Migrated from @app/imagodei/services/session-human.service to
 * @elohim/identity as part of Slice 2.3 cross-pillar import cleanup.
 *
 * M-AGGR-1: activity tracking (record* methods) and upgrade-prompt
 * localStorage management have moved to the Rust substrate. Callers
 * should emit EconomicEvents via the lamad-event service and read
 * UpgradePromptView from /api/v1/identity/{agentId}/upgrade-prompts.
 */
@Injectable({ providedIn: 'root' })
export class SessionHumanService {
  private readonly STORAGE_KEY = 'lamad-session';
  private readonly NOT_AUTHENTICATED = 'not-authenticated';

  private readonly sessionSubject = new BehaviorSubject<SessionHuman | null>(null);

  public readonly session$: Observable<SessionHuman | null> = this.sessionSubject.asObservable();

  constructor() {
    this.initializeSession();
  }

  // =========================================================================
  // Session Lifecycle
  // =========================================================================

  /**
   * Initialize or restore session.
   * Creates new session if none exists.
   */
  private initializeSession(): void {
    const existing = this.loadSession();

    if (existing) {
      // Restore existing session
      existing.lastActiveAt = new Date().toISOString();
      existing.stats.sessionCount++;
      this.saveSession(existing);
      this.sessionSubject.next(existing);
    } else {
      // Create new session
      const session = this.createNewSession();
      this.saveSession(session);
      this.sessionSubject.next(session);
    }
  }

  /**
   * Create a new session with generated ID.
   */
  private createNewSession(): SessionHuman {
    const sessionId = this.generateSessionId();
    const now = new Date().toISOString();

    return {
      sessionId,
      displayName: 'Traveler',
      createdAt: now,
      lastActiveAt: now,
      stats: {
        nodesViewed: 0,
        nodesWithAffinity: 0,
        pathsStarted: 0,
        pathsCompleted: 0,
        stepsCompleted: 0,
        totalSessionTime: 0,
        averageSessionLength: 0,
        sessionCount: 1,
      },

      // Session state - active visitor by default
      isAnonymous: true,
      accessLevel: 'visitor',
      sessionState: 'active',

      // No Holochain link initially
      linkedAgentPubKey: undefined,
      linkedHumanId: undefined,
      linkedAt: undefined,

      // No upgrade in progress
      upgradeIntent: undefined,
    };
  }

  /**
   * Generate a unique session ID.
   * Format: session-{timestamp}-{random}
   */
  private generateSessionId(): string {
    const timestamp = Date.now().toString(36);
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const random = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 8);
    return `session-${timestamp}-${random}`;
  }

  /**
   * Get the current session (synchronous).
   */
  getSession(): SessionHuman | null {
    return this.sessionSubject.value;
  }

  /**
   * Get the current session ID.
   */
  getSessionId(): string {
    return this.sessionSubject.value?.sessionId ?? '';
  }

  /**
   * Check if human has an active session.
   */
  hasSession(): boolean {
    return this.sessionSubject.value !== null;
  }

  /**
   * Update the display name.
   */
  setDisplayName(name: string): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.displayName = name.trim() || 'Traveler';
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  /**
   * Update the avatar URL.
   */
  setAvatarUrl(url: string): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.avatarUrl = url.trim() || undefined;
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  /**
   * Update the bio/description.
   */
  setBio(bio: string): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.bio = bio.trim() || undefined;
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  /**
   * Update the locale preference.
   */
  setLocale(locale: string): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.locale = locale.trim() || undefined;
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  /**
   * Update interests/keywords.
   */
  setInterests(interests: string[]): void {
    const session = this.sessionSubject.value;
    if (session) {
      // Filter empty strings and trim each interest
      session.interests = interests.map(i => i.trim()).filter(i => i.length > 0);

      if (session.interests.length === 0) {
        session.interests = undefined;
      }

      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  /**
   * Get storage key prefix for namespacing session-scoped data.
   * Used by other services (e.g., ContentMasteryService) to namespace their storage.
   */
  getStorageKeyPrefix(): string {
    const session = this.sessionSubject.value;
    return session ? `lamad-session-${session.sessionId}` : 'lamad-session-anonymous';
  }

  /**
   * Update session activity timestamp.
   */
  touch(): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
    }
  }

  // =========================================================================
  // Path Progress (Session-scoped)
  // =========================================================================

  /**
   * Get progress for a path.
   */
  getPathProgress(pathId: string): SessionPathProgress | null {
    const session = this.sessionSubject.value;
    if (!session) return null;

    const key = `lamad-session-${session.sessionId}-progress-${pathId}`;
    try {
      const stored = localStorage.getItem(key);
      if (stored) {
        return JSON.parse(stored) as SessionPathProgress;
      }
    } catch {
      // Ignore
    }
    return null;
  }

  /**
   * Save progress for a path.
   */
  savePathProgress(progress: SessionPathProgress): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    const key = `lamad-session-${session.sessionId}-progress-${progress.pathId}`;
    try {
      localStorage.setItem(key, JSON.stringify(progress));
    } catch {
      // localStorage quota exceeded — caller should prompt upgrade via substrate view
    }

    this.touch();
  }

  /**
   * Get all path progress records.
   */
  getAllPathProgress(): SessionPathProgress[] {
    const session = this.sessionSubject.value;
    if (!session) return [];

    const prefix = `lamad-session-${session.sessionId}-progress-`;
    const progress: SessionPathProgress[] = [];

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith(prefix)) {
        try {
          const data = localStorage.getItem(key);
          if (data) {
            progress.push(JSON.parse(data) as SessionPathProgress);
          }
        } catch {
          // Skip malformed entries
        }
      }
    }

    return progress;
  }

  // =========================================================================
  // Affinity (Session-scoped)
  // =========================================================================

  /**
   * Get affinity storage key for current session.
   */
  getAffinityStorageKey(): string {
    const session = this.sessionSubject.value;
    return session
      ? `lamad-session-${session.sessionId}-affinity`
      : 'lamad-session-anonymous-affinity';
  }

  // =========================================================================
  // Upgrade Prompts (thin dismissal shim — content driven by substrate)
  // =========================================================================

  /**
   * Dismiss an upgrade prompt by its id.
   * The active prompt list is now served from the substrate via
   * GET /api/v1/identity/{agentId}/upgrade-prompts. This shim records
   * the dismissal in localStorage so the UI can suppress re-display
   * until the next session.
   */
  dismissUpgradePrompt(promptId: string): void {
    try {
      const key = 'lamad-dismissed-prompts';
      const stored = localStorage.getItem(key);
      const dismissed: string[] = stored ? (JSON.parse(stored) as string[]) : [];
      if (!dismissed.includes(promptId)) {
        dismissed.push(promptId);
        localStorage.setItem(key, JSON.stringify(dismissed));
      }
    } catch {
      // Ignore — dismissal is best-effort UI state
    }
  }

  /**
   * Return prompt IDs dismissed in this browser.
   * Consumers filter the substrate UpgradePromptView.activePrompts against this list.
   */
  getDismissedPromptIds(): string[] {
    try {
      const stored = localStorage.getItem('lamad-dismissed-prompts');
      return stored ? (JSON.parse(stored) as string[]) : [];
    } catch {
      return [];
    }
  }

  /**
   * Get active (non-dismissed) upgrade prompts from a substrate-provided list.
   * @deprecated Use the substrate UpgradePromptView directly via
   * GET /api/v1/identity/{agentId}/upgrade-prompts and filter against
   * getDismissedPromptIds(). This shim bridges callers until they migrate.
   */
  getActiveUpgradePrompts(): HolochainUpgradePrompt[] {
    // M-AGGR-1: prompt content is now substrate-driven. Return empty so existing
    // callers see no locally-managed prompts; they should migrate to the route.
    return [];
  }

  /**
   * Trigger an upgrade prompt.
   * @deprecated Activity signals now flow via EconomicEvents to the Rust
   * substrate. The substrate derives UpgradePromptView from those events.
   * This stub is retained for call-site compatibility; it is a no-op.
   */
  onGatedContentAccess(_contentId: string, _contentTitle?: string): void {
    // M-AGGR-1: substrate-driven. Callers should emit a lamad EconomicEvent
    // and read UpgradePromptView from the substrate route.
  }

  // =========================================================================
  // Hybrid State Management (Session + Holochain)
  // =========================================================================

  /**
   * Link this session to a Holochain identity.
   * Used when user creates Holochain identity but wants to keep session for offline use.
   */
  linkToHolochainIdentity(agentPubKey: string, humanId: string): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    session.linkedAgentPubKey = agentPubKey;
    session.linkedHumanId = humanId;
    session.linkedAt = new Date().toISOString();
    session.sessionState = 'linked';
    session.isAnonymous = false;
    session.accessLevel = 'linked';

    this.saveSession(session);
    this.sessionSubject.next({ ...session });
  }

  /**
   * Check if session is linked to a Holochain identity.
   */
  isLinkedToHolochain(): boolean {
    const session = this.sessionSubject.value;
    return session?.sessionState === 'linked' && !!session.linkedAgentPubKey;
  }

  /**
   * Get linked Holochain agent pubkey.
   */
  getLinkedAgentPubKey(): string | null {
    return this.sessionSubject.value?.linkedAgentPubKey ?? null;
  }

  /**
   * Get linked Human ID.
   */
  getLinkedHumanId(): string | null {
    return this.sessionSubject.value?.linkedHumanId ?? null;
  }

  // =========================================================================
  // Upgrade Intent Tracking
  // =========================================================================

  /**
   * Start an upgrade intent (user begins but hasn't completed upgrade).
   */
  startUpgradeIntent(targetStage: 'hosted' | 'app-steward' | 'node-steward'): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    session.upgradeIntent = {
      targetStage,
      startedAt: new Date().toISOString(),
      currentStep: 'initiated',
      completedSteps: [],
      paused: false,
    };
    session.sessionState = 'upgrading';
    session.accessLevel = 'pending';

    this.saveSession(session);
    this.sessionSubject.next({ ...session });
  }

  /**
   * Update upgrade progress.
   */
  updateUpgradeProgress(currentStep: string, completedStep?: string): void {
    const session = this.sessionSubject.value;
    if (!session?.upgradeIntent) return;

    session.upgradeIntent.currentStep = currentStep;
    if (completedStep) {
      session.upgradeIntent.completedSteps.push(completedStep);
    }

    this.saveSession(session);
    this.sessionSubject.next({ ...session });
  }

  /**
   * Pause upgrade (user abandons temporarily).
   */
  pauseUpgrade(reason?: string): void {
    const session = this.sessionSubject.value;
    if (!session?.upgradeIntent) return;

    session.upgradeIntent.paused = true;
    session.upgradeIntent.pauseReason = reason;
    session.sessionState = 'active';
    session.accessLevel = 'visitor';

    this.saveSession(session);
    this.sessionSubject.next({ ...session });
  }

  /**
   * Resume a paused upgrade.
   */
  resumeUpgrade(): void {
    const session = this.sessionSubject.value;
    if (!session?.upgradeIntent) return;

    session.upgradeIntent.paused = false;
    session.upgradeIntent.pauseReason = undefined;
    session.sessionState = 'upgrading';
    session.accessLevel = 'pending';

    this.saveSession(session);
    this.sessionSubject.next({ ...session });
  }

  /**
   * Cancel upgrade intent entirely.
   */
  cancelUpgrade(): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    session.upgradeIntent = undefined;
    session.sessionState = 'active';
    session.accessLevel = 'visitor';

    this.saveSession(session);
    this.sessionSubject.next({ ...session });
  }

  /**
   * Get current upgrade intent.
   */
  getUpgradeIntent(): UpgradeIntent | null {
    return this.sessionSubject.value?.upgradeIntent ?? null;
  }

  /**
   * Check if upgrade is in progress.
   */
  isUpgrading(): boolean {
    const session = this.sessionSubject.value;
    return session?.sessionState === 'upgrading' && !session.upgradeIntent?.paused;
  }

  // =========================================================================
  // Migration to Holochain
  // =========================================================================

  /**
   * Prepare migration package for Holochain.
   * Called when human installs Holochain app.
   *
   * M-AGGR-1: activities are no longer collected client-side; the substrate
   * derives session stats from EconomicEvents. activities is always [] here.
   */
  prepareMigration(): SessionMigration | null {
    const session = this.sessionSubject.value;
    if (!session) return null;

    // Gather all affinity data
    const affinityKey = this.getAffinityStorageKey();
    let affinity: Record<string, number> = {};
    try {
      const stored = localStorage.getItem(affinityKey);
      if (stored) {
        const parsed = JSON.parse(stored) as Record<string, unknown>;
        affinity = (parsed['affinity'] as Record<string, number>) ?? {};
      }
    } catch {
      // Ignore
    }

    return {
      sessionId: session.sessionId,
      migratedAt: new Date().toISOString(),
      affinity,
      pathProgress: this.getAllPathProgress(),
      // Activity history is now substrate-derived (EconomicEvents).
      activities: [],
      status: 'pending',
    };
  }

  /**
   * Mark session as migrated (keeps reference but data moves to Holochain).
   * Use this when you want to preserve the session for fallback/offline.
   */
  markAsMigrated(agentPubKey: string, humanId: string): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    session.sessionState = 'migrated';
    session.linkedAgentPubKey = agentPubKey;
    session.linkedHumanId = humanId;
    session.linkedAt = new Date().toISOString();
    session.isAnonymous = false;
    session.accessLevel = 'linked';
    session.upgradeIntent = undefined;

    this.saveSession(session);
    this.sessionSubject.next({ ...session });
  }

  /**
   * Clear session completely after migration.
   * Use this when user wants to fully delete session data.
   */
  clearAfterMigration(): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    // Clear all session-related storage
    const prefix = `lamad-session-${session.sessionId}`;
    const keysToRemove: string[] = [];

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith(prefix) || key === this.STORAGE_KEY) {
        keysToRemove.push(key);
      }
    }

    keysToRemove.forEach(key => localStorage.removeItem(key));

    this.sessionSubject.next(null);
  }

  /**
   * Get session state.
   */
  getSessionState(): SessionState | null {
    return this.sessionSubject.value?.sessionState ?? null;
  }

  // =========================================================================
  // Storage Helpers
  // =========================================================================

  /**
   * Load session from localStorage.
   * Returns null if session doesn't exist or cannot be parsed.
   */
  private loadSession(): SessionHuman | null {
    try {
      const stored = localStorage.getItem(this.STORAGE_KEY);
      if (stored) {
        return JSON.parse(stored) as SessionHuman;
      }
    } catch (err) {
      // Session parse failure is non-critical - falls back to null for visitor mode
      if (err instanceof Error) {
        console.warn(
          '[SessionHumanService] Failed to parse session from localStorage:',
          err.message
        );
      }
    }
    return null;
  }

  /**
   * Save session to localStorage.
   * Silently fails if localStorage is unavailable or quota exceeded.
   */
  private saveSession(session: SessionHuman): void {
    try {
      localStorage.setItem(this.STORAGE_KEY, JSON.stringify(session));
    } catch {
      // localStorage write failure is non-critical — session continues in memory
    }
  }

  /**
   * Reset everything (for testing).
   */
  resetSession(): void {
    this.clearAfterMigration();
    const newSession = this.createNewSession();
    this.saveSession(newSession);
    this.sessionSubject.next(newSession);
  }

  // =========================================================================
  // Content Access Control
  // =========================================================================

  /**
   * Get the current human's access level.
   * Session humans are always 'visitor'.
   */
  getAccessLevel(): AccessLevel {
    return 'visitor';
  }

  /**
   * Check if human can access content with given access metadata.
   */
  checkContentAccess(accessMetadata?: ContentAccessMetadata): AccessCheckResult {
    // No access metadata = open content
    if (!accessMetadata || accessMetadata.accessLevel === 'open') {
      return { canAccess: true };
    }

    // Session humans cannot access gated or protected content
    if (accessMetadata.accessLevel === 'gated') {
      return {
        canAccess: false,
        reason: this.NOT_AUTHENTICATED,
        actionRequired: this.createInstallAction(
          accessMetadata.restrictionReason ?? 'This content requires joining the Elohim network.'
        ),
      };
    }

    if (accessMetadata.accessLevel === 'protected') {
      const requirements = accessMetadata.requirements;
      const actions: AccessAction[] = [];

      // First requirement: Join network
      actions.push(this.createInstallAction('Join the Elohim network'));

      // Second: Complete prerequisite path
      if (requirements?.requiredPaths?.length) {
        actions.push({
          type: 'complete-path',
          label: 'Complete Training',
          description: `Complete the prerequisite training path`,
          pathId: requirements.requiredPaths[0],
        });
      }

      // Third: Earn attestation
      if (requirements?.requiredAttestations?.length) {
        actions.push({
          type: 'earn-attestation',
          label: 'Earn Attestation',
          description: `Earn the ${requirements.requiredAttestations[0]} attestation`,
          attestationId: requirements.requiredAttestations[0],
        });
      }

      return {
        canAccess: false,
        reason: requirements?.requiredPaths?.length ? 'missing-path' : this.NOT_AUTHENTICATED,
        actionRequired: actions[0],
        missingAttestations: requirements?.requiredAttestations,
        missingPaths: requirements?.requiredPaths,
        unlockPath: accessMetadata.unlockPath,
      };
    }

    // Default: deny
    return {
      canAccess: false,
      reason: this.NOT_AUTHENTICATED,
    };
  }

  /**
   * Create the "install Holochain" action.
   */
  private createInstallAction(description: string): AccessAction {
    return {
      type: 'install-holochain',
      label: 'Join Network',
      description,
      installUrl: '/install', // Future: actual Holochain install page
    };
  }

  /**
   * Check if content is accessible without showing details.
   * For use in listings to show/hide locked indicators.
   */
  canAccessContent(accessMetadata?: ContentAccessMetadata): boolean {
    return this.checkContentAccess(accessMetadata).canAccess;
  }
}
