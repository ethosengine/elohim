import { Injectable } from '@angular/core';

// @coverage: 94.9% (2026-02-05)

import { BehaviorSubject, Observable } from 'rxjs';

import {
  ContentAccessMetadata,
  AccessCheckResult,
  AccessLevel,
  AccessAction,
} from '../../lamad/models/content-access.model';
import {
  SessionHuman,
  SessionStats,
  SessionActivity,
  SessionPathProgress,
  SessionMigration,
  HolochainUpgradePrompt,
  UpgradeTrigger,
  SessionState,
  UpgradeIntent,
} from '../models/session-human.model';
// Content access models from lamad pillar

/**
 * SessionHumanService - Manages temporary session identity for MVP.
 *
 * Philosophy:
 * - Zero-friction entry: humans explore immediately
 * - Progress persists in localStorage during session
 * - Meaningful moments prompt Holochain "upgrade"
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
 * - lamad-session-{sessionId}-activities: Activity history
 */
@Injectable({ providedIn: 'root' })
export class SessionHumanService {
  private readonly STORAGE_KEY = 'lamad-session';
  private readonly ACTIVITY_LIMIT = 1000; // Max activities to store

  // Upgrade trigger constants
  private readonly PROGRESS_AT_RISK = 'progress-at-risk';
  private readonly SESSION_AT_RISK = 'progress-at-risk';
  private readonly NOT_AUTHENTICATED = 'not-authenticated';

  private readonly sessionSubject = new BehaviorSubject<SessionHuman | null>(null);
  private readonly upgradePromptsSubject = new BehaviorSubject<HolochainUpgradePrompt[]>([]);

  public readonly session$: Observable<SessionHuman | null> = this.sessionSubject.asObservable();
  public readonly upgradePrompts$: Observable<HolochainUpgradePrompt[]> =
    this.upgradePromptsSubject.asObservable();

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

      // Check if this is a return visit (session older than 24h)
      const lastActive = new Date(existing.lastActiveAt).getTime();
      const dayAgo = Date.now() - 24 * 60 * 60 * 1000;
      if (lastActive < dayAgo) {
        this.triggerUpgradePrompt('return-visit');
      }
    } else {
      // Create new session
      const session = this.createNewSession();
      this.saveSession(session);
      this.sessionSubject.next(session);
    }

    // Load upgrade prompts
    this.loadUpgradePrompts();
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
  // Activity Tracking
  // =========================================================================

  /**
   * Record a content view.
   */
  recordContentView(nodeId: string): void {
    this.recordActivity({
      timestamp: new Date().toISOString(),
      type: 'view',
      resourceId: nodeId,
      resourceType: 'content',
    });
    this.incrementStat('nodesViewed');
  }

  /**
   * Record an affinity change.
   */
  recordAffinityChange(nodeId: string, value: number): void {
    this.recordActivity({
      timestamp: new Date().toISOString(),
      type: 'affinity',
      resourceId: nodeId,
      resourceType: 'content',
      metadata: { value },
    });

    // Trigger upgrade prompt on first affinity
    const session = this.sessionSubject.value;
    if (session?.stats.nodesWithAffinity === 0 && value > 0) {
      this.triggerUpgradePrompt('first-affinity');
    }

    if (value > 0) {
      this.incrementStat('nodesWithAffinity');
    }
  }

  /**
   * Record path started.
   */
  recordPathStarted(pathId: string): void {
    this.recordActivity({
      timestamp: new Date().toISOString(),
      type: 'path-start',
      resourceId: pathId,
      resourceType: 'path',
    });
    this.incrementStat('pathsStarted');

    // Trigger upgrade prompt on first path
    const session = this.sessionSubject.value;
    if (session?.stats.pathsStarted === 1) {
      this.triggerUpgradePrompt('path-started');
    }
  }

  /**
   * Record step completed.
   */
  recordStepCompleted(pathId: string, stepIndex: number): void {
    this.recordActivity({
      timestamp: new Date().toISOString(),
      type: 'step-complete',
      resourceId: pathId,
      resourceType: 'step',
      metadata: { stepIndex },
    });
    this.incrementStat('stepsCompleted');
  }

  /**
   * Record path completed.
   */
  recordPathCompleted(pathId: string): void {
    this.recordActivity({
      timestamp: new Date().toISOString(),
      type: 'path-complete',
      resourceId: pathId,
      resourceType: 'path',
    });
    this.incrementStat('pathsCompleted');
    this.triggerUpgradePrompt('path-completed');
  }

  /**
   * Record exploration activity.
   */
  recordExploration(nodeId: string): void {
    this.recordActivity({
      timestamp: new Date().toISOString(),
      type: 'explore',
      resourceId: nodeId,
      resourceType: 'content',
    });
  }

  /**
   * Record notes saved.
   */
  recordNotesSaved(_pathId: string, _stepIndex: number): void {
    this.triggerUpgradePrompt('notes-saved');
  }

  /**
   * Record generic activity.
   */
  private recordActivity(activity: SessionActivity): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    const key = `lamad-session-${session.sessionId}-activities`;
    let activities: SessionActivity[] = [];

    try {
      const stored = localStorage.getItem(key);
      if (stored) {
        activities = JSON.parse(stored);
      }
    } catch {
      activities = [];
    }

    activities.push(activity);

    // Trim to limit
    if (activities.length > this.ACTIVITY_LIMIT) {
      activities = activities.slice(-this.ACTIVITY_LIMIT);
    }

    try {
      localStorage.setItem(key, JSON.stringify(activities));
    } catch (err) {
      // localStorage quota exceeded - handle gracefully by prompting user to upgrade
      // This is intentional: we catch quota errors and trigger upgrade flow rather than losing data
      if (err instanceof Error && err.message.includes('QuotaExceededError')) {
        this.triggerUpgradePrompt(this.PROGRESS_AT_RISK);
      }
    }

    // Update last active
    this.touch();
  }

  /**
   * Increment a stat counter.
   */
  private incrementStat(stat: keyof SessionStats): void {
    const session = this.sessionSubject.value;
    if (session && typeof session.stats[stat] === 'number') {
      session.stats[stat]++;
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  /**
   * Get activity history.
   */
  getActivityHistory(): SessionActivity[] {
    const session = this.sessionSubject.value;
    if (!session) return [];

    const key = `lamad-session-${session.sessionId}-activities`;
    try {
      const stored = localStorage.getItem(key);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch {
      // Ignore
    }
    return [];
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
        return JSON.parse(stored);
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
    } catch (err) {
      // localStorage quota exceeded - handle gracefully by prompting user to upgrade
      // This is intentional: we catch quota errors and trigger upgrade flow rather than losing data
      if (err instanceof Error && err.message.includes('QuotaExceededError')) {
        this.triggerUpgradePrompt(this.PROGRESS_AT_RISK);
      }
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
            progress.push(JSON.parse(data));
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
  // Upgrade Prompts
  // =========================================================================

  /**
   * Trigger an upgrade prompt.
   */
  triggerUpgradePrompt(trigger: UpgradeTrigger): void {
    const prompts = this.upgradePromptsSubject.value;

    // Don't show if already dismissed
    const existing = prompts.find(p => p.trigger === trigger);
    if (existing?.dismissed) return;

    const prompt = this.createUpgradePrompt(trigger);
    if (prompt) {
      // Remove existing prompt for this trigger
      const filtered = prompts.filter(p => p.trigger !== trigger);
      filtered.push(prompt);
      this.upgradePromptsSubject.next(filtered);
      this.saveUpgradePrompts(filtered);
    }
  }

  /**
   * Dismiss an upgrade prompt.
   */
  dismissUpgradePrompt(promptId: string): void {
    const prompts = this.upgradePromptsSubject.value;
    const prompt = prompts.find(p => p.id === promptId);
    if (prompt) {
      prompt.dismissed = true;
      prompt.dismissedAt = new Date().toISOString();
      this.upgradePromptsSubject.next([...prompts]);
      this.saveUpgradePrompts(prompts);
    }
  }

  /**
   * Get active (non-dismissed) upgrade prompts.
   */
  getActiveUpgradePrompts(): HolochainUpgradePrompt[] {
    return this.upgradePromptsSubject.value.filter(p => !p.dismissed);
  }

  /**
   * Create upgrade prompt content.
   */
  private createUpgradePrompt(trigger: UpgradeTrigger): HolochainUpgradePrompt | null {
    const id = `prompt-${trigger}-${Date.now()}`;

    switch (trigger) {
      case 'first-affinity':
        return {
          id,
          trigger,
          title: 'Save Your Progress',
          message:
            "You're building a personal knowledge map! Install the Elohim app to save it permanently.",
          benefits: [
            'Your progress syncs across devices',
            'Join a network of learners',
            'Never lose your journey',
          ],
          dismissed: false,
        };

      case 'path-started':
        return {
          id,
          trigger,
          title: "You've Started a Journey",
          message:
            'Your learning path is stored in your browser. Install Elohim to make it permanent.',
          benefits: [
            'Resume from any device',
            'Get updates to your paths',
            'Connect with fellow travelers',
          ],
          dismissed: false,
        };

      case 'path-completed':
        return {
          id,
          trigger,
          title: 'Congratulations! 🎉',
          message: 'You completed a learning path! Install Elohim to earn verifiable credentials.',
          benefits: [
            'Earn attestations for your achievement',
            'Share your credentials',
            'Discover advanced paths',
          ],
          dismissed: false,
        };

      case 'notes-saved':
        return {
          id,
          trigger,
          title: 'Your Notes Are Valuable',
          message: 'Personal notes enrich your learning. Install Elohim to keep them safe.',
          benefits: ['Notes stored securely', 'Searchable across all content', 'Export anytime'],
          dismissed: false,
        };

      case 'return-visit':
        return {
          id,
          trigger,
          title: 'Welcome Back!',
          message: 'Good to see you again. Install Elohim to never worry about losing progress.',
          benefits: ['Automatic progress backup', 'Sync between devices', 'Join the community'],
          dismissed: false,
        };

      case this.PROGRESS_AT_RISK:
        return {
          id,
          trigger,
          title: 'Storage Running Low',
          message:
            'Your browser storage is filling up. Install Elohim to safely store your progress.',
          benefits: ['Unlimited progress storage', 'Automatic backups', 'Secure and private'],
          dismissed: false,
        };

      case 'network-feature':
        return {
          id,
          trigger,
          title: 'Network Feature',
          message: 'This feature requires joining the Elohim network.',
          benefits: [
            'Connect with other learners',
            'Share and receive content',
            'Participate in governance',
          ],
          dismissed: false,
        };

      default:
        return null;
    }
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
        const parsed = JSON.parse(stored);
        affinity = parsed.affinity ?? {};
      }
    } catch {
      // Ignore
    }

    return {
      sessionId: session.sessionId,
      migratedAt: new Date().toISOString(),
      affinity,
      pathProgress: this.getAllPathProgress(),
      activities: this.getActivityHistory(),
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
        return JSON.parse(stored);
      }
    } catch (err) {
      // Session parse failure is non-critical - falls back to null for visitor mode
      // This can happen if localStorage is corrupted or quota exceeded
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
    } catch (err) {
      // localStorage write failure is non-critical
      // This can happen if localStorage is disabled or quota exceeded
      // User can continue with temporary session until upgrade
      if (err instanceof Error && err.message.includes('QuotaExceededError')) {
        this.triggerUpgradePrompt(this.SESSION_AT_RISK);
      }
    }
  }

  /**
   * Load upgrade prompts from localStorage.
   */
  private loadUpgradePrompts(): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    const key = `lamad-session-${session.sessionId}-prompts`;
    try {
      const stored = localStorage.getItem(key);
      if (stored) {
        this.upgradePromptsSubject.next(JSON.parse(stored));
      }
    } catch {
      // Ignore
    }
  }

  /**
   * Save upgrade prompts to localStorage.
   */
  private saveUpgradePrompts(prompts: HolochainUpgradePrompt[]): void {
    const session = this.sessionSubject.value;
    if (!session) return;

    const key = `lamad-session-${session.sessionId}-prompts`;
    try {
      localStorage.setItem(key, JSON.stringify(prompts));
    } catch {
      // Ignore
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
    this.upgradePromptsSubject.next([]);
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

  /**
   * Trigger upgrade prompt when human tries to access gated content.
   */
  onGatedContentAccess(_contentId: string, _contentTitle?: string): void {
    this.triggerUpgradePrompt('network-feature');
  }
}
