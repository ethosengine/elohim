import { Injectable } from '@angular/core';

// M-AGGR-1 migration: session activity counting and upgrade prompts are now
// substrate projections served by:
//   GET /api/v1/identity/{agentId}/session          → SessionHumanView
//   GET /api/v1/identity/{agentId}/upgrade-prompts  → UpgradePromptView
//
// The former 1100-line service is now ~200 lines of session-identity only.
// Consumers that called record*() methods should emit intents via M-REA-1's
// EventService.emitEvent() instead.
//
// @coverage: 94.9% (2026-02-24) — coverage scope now covers session-identity
// lifecycle only; projection-derived stats covered by schema contract tests.

import { BehaviorSubject, Observable } from 'rxjs';

import {
  ContentAccessMetadata,
  AccessCheckResult,
  AccessLevel,
  AccessAction,
} from '@app/lamad/models/content-access.model';
import {
  SessionHuman,
  SessionPathProgress,
  SessionMigration,
  SessionState,
  UpgradeIntent,
} from '../models/session-human.model';

/**
 * SessionHumanService — session identity for anonymous visitors.
 *
 * Scope (post M-AGGR-1):
 * - Session lifecycle: sessionId, accessLevel, isAnonymous
 * - Content access control (visitor vs gated vs protected)
 * - Upgrade intent tracking (which upgrade stage the human is navigating)
 * - Profile metadata (displayName, bio, avatar, locale, interests)
 * - Migration helpers: prepareMigration(), markAsMigrated(), clearAfterMigration()
 *
 * NOT this service's scope:
 * - Activity counting (nodesViewed, pathsStarted, etc.) → SessionHumanView substrate projection
 * - Upgrade prompt determination → UpgradePromptView substrate projection
 * - REA event creation → EventService.emitEvent() (M-REA-1)
 * - localStorage activity-history arrays → removed
 *
 * Storage keys:
 * - lamad-session: SessionHuman identity object (not stats)
 * - lamad-session-{sessionId}-affinity: Affinity data (other services)
 * - lamad-session-{sessionId}-progress-{pathId}: Path progress (other services)
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

  private initializeSession(): void {
    const existing = this.loadSession();
    if (existing) {
      existing.lastActiveAt = new Date().toISOString();
      this.saveSession(existing);
      this.sessionSubject.next(existing);
    } else {
      const session = this.createNewSession();
      this.saveSession(session);
      this.sessionSubject.next(session);
    }
  }

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
      isAnonymous: true,
      accessLevel: 'visitor',
      sessionState: 'active',
      linkedAgentPubKey: undefined,
      linkedHumanId: undefined,
      linkedAt: undefined,
      upgradeIntent: undefined,
    };
  }

  private generateSessionId(): string {
    const timestamp = Date.now().toString(36);
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const random = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 8);
    return `session-${timestamp}-${random}`;
  }

  getSession(): SessionHuman | null {
    return this.sessionSubject.value;
  }

  getSessionId(): string {
    return this.sessionSubject.value?.sessionId ?? '';
  }

  hasSession(): boolean {
    return this.sessionSubject.value !== null;
  }

  setDisplayName(name: string): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.displayName = name.trim() || 'Traveler';
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  setAvatarUrl(url: string): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.avatarUrl = url.trim() || undefined;
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  setBio(bio: string): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.bio = bio.trim() || undefined;
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  setLocale(locale: string): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.locale = locale.trim() || undefined;
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  setInterests(interests: string[]): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.interests = interests.map(i => i.trim()).filter(i => i.length > 0);
      if (session.interests.length === 0) {
        session.interests = undefined;
      }
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
      this.sessionSubject.next({ ...session });
    }
  }

  getStorageKeyPrefix(): string {
    const session = this.sessionSubject.value;
    return session ? `lamad-session-${session.sessionId}` : 'lamad-session-anonymous';
  }

  touch(): void {
    const session = this.sessionSubject.value;
    if (session) {
      session.lastActiveAt = new Date().toISOString();
      this.saveSession(session);
    }
  }

  // =========================================================================
  // Path Progress (Session-scoped, kept for offline use during upgrade)
  // =========================================================================

  getPathProgress(pathId: string): SessionPathProgress | null {
    const session = this.sessionSubject.value;
    if (!session) return null;
    const key = `lamad-session-${session.sessionId}-progress-${pathId}`;
    try {
      const stored = localStorage.getItem(key);
      if (stored) return JSON.parse(stored) as SessionPathProgress;
    } catch {
      // ignore
    }
    return null;
  }

  savePathProgress(progress: SessionPathProgress): void {
    const session = this.sessionSubject.value;
    if (!session) return;
    const key = `lamad-session-${session.sessionId}-progress-${progress.pathId}`;
    try {
      localStorage.setItem(key, JSON.stringify(progress));
    } catch {
      // quota exceeded — substrate projections are the durable record
    }
    this.touch();
  }

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
          if (data) progress.push(JSON.parse(data) as SessionPathProgress);
        } catch {
          // skip malformed entries
        }
      }
    }
    return progress;
  }

  // =========================================================================
  // Affinity (Session-scoped)
  // =========================================================================

  getAffinityStorageKey(): string {
    const session = this.sessionSubject.value;
    return session
      ? `lamad-session-${session.sessionId}-affinity`
      : 'lamad-session-anonymous-affinity';
  }

  // =========================================================================
  // Upgrade-prompt dismissal — browser-local UI state
  //
  // The active prompt list is served from the substrate via
  // GET /api/v1/identity/{agentId}/upgrade-prompts (M-AGGR-1). This pair
  // records per-browser dismissal so the UI can suppress re-display until
  // the next session — UI affordance only, NOT substrate-policy.
  // =========================================================================

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

  getDismissedPromptIds(): string[] {
    try {
      const stored = localStorage.getItem('lamad-dismissed-prompts');
      return stored ? (JSON.parse(stored) as string[]) : [];
    } catch {
      return [];
    }
  }

  // =========================================================================
  // Hybrid State Management (Session + Holochain)
  // =========================================================================

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

  isLinkedToHolochain(): boolean {
    const session = this.sessionSubject.value;
    return session?.sessionState === 'linked' && !!session.linkedAgentPubKey;
  }

  getLinkedAgentPubKey(): string | null {
    return this.sessionSubject.value?.linkedAgentPubKey ?? null;
  }

  getLinkedHumanId(): string | null {
    return this.sessionSubject.value?.linkedHumanId ?? null;
  }

  // =========================================================================
  // Upgrade Intent Tracking
  // =========================================================================

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

  updateUpgradeProgress(currentStep: string, completedStep?: string): void {
    const session = this.sessionSubject.value;
    if (!session?.upgradeIntent) return;
    session.upgradeIntent.currentStep = currentStep;
    if (completedStep) session.upgradeIntent.completedSteps.push(completedStep);
    this.saveSession(session);
    this.sessionSubject.next({ ...session });
  }

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

  cancelUpgrade(): void {
    const session = this.sessionSubject.value;
    if (!session) return;
    session.upgradeIntent = undefined;
    session.sessionState = 'active';
    session.accessLevel = 'visitor';
    this.saveSession(session);
    this.sessionSubject.next({ ...session });
  }

  getUpgradeIntent(): UpgradeIntent | null {
    return this.sessionSubject.value?.upgradeIntent ?? null;
  }

  isUpgrading(): boolean {
    const session = this.sessionSubject.value;
    return session?.sessionState === 'upgrading' && !session.upgradeIntent?.paused;
  }

  // =========================================================================
  // Migration to Holochain
  // =========================================================================

  prepareMigration(): SessionMigration | null {
    const session = this.sessionSubject.value;
    if (!session) return null;
    const affinityKey = this.getAffinityStorageKey();
    let affinity: Record<string, number> = {};
    try {
      const stored = localStorage.getItem(affinityKey);
      if (stored) {
        const parsed = JSON.parse(stored) as Record<string, unknown>;
        affinity = (parsed['affinity'] as Record<string, number>) ?? {};
      }
    } catch {
      // ignore
    }
    return {
      sessionId: session.sessionId,
      migratedAt: new Date().toISOString(),
      affinity,
      pathProgress: this.getAllPathProgress(),
      activities: [],
      status: 'pending',
    };
  }

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

  clearAfterMigration(): void {
    const session = this.sessionSubject.value;
    if (!session) return;
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

  getSessionState(): SessionState | null {
    return this.sessionSubject.value?.sessionState ?? null;
  }

  resetSession(): void {
    this.clearAfterMigration();
    const newSession = this.createNewSession();
    this.saveSession(newSession);
    this.sessionSubject.next(newSession);
  }

  // =========================================================================
  // Content Access Control
  // =========================================================================

  getAccessLevel(): AccessLevel {
    return 'visitor';
  }

  checkContentAccess(accessMetadata?: ContentAccessMetadata): AccessCheckResult {
    if (!accessMetadata || accessMetadata.accessLevel === 'open') {
      return { canAccess: true };
    }
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
      actions.push(this.createInstallAction('Join the Elohim network'));
      if (requirements?.requiredPaths?.length) {
        actions.push({
          type: 'complete-path',
          label: 'Complete Training',
          description: 'Complete the prerequisite training path',
          pathId: requirements.requiredPaths[0],
        });
      }
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
    return { canAccess: false, reason: this.NOT_AUTHENTICATED };
  }

  private createInstallAction(description: string): AccessAction {
    return {
      type: 'install-holochain',
      label: 'Join Network',
      description,
      installUrl: '/install',
    };
  }

  canAccessContent(accessMetadata?: ContentAccessMetadata): boolean {
    return this.checkContentAccess(accessMetadata).canAccess;
  }

  onGatedContentAccess(_contentId: string, _contentTitle?: string): void {
    // Previously triggered a local upgrade prompt.
    // M-AGGR-1: upgrade prompts are now served by the substrate:
    //   GET /api/v1/identity/{agentId}/upgrade-prompts → UpgradePromptView
    // Angular components should subscribe to that view directly.
  }

  // =========================================================================
  // Storage Helpers
  // =========================================================================

  private loadSession(): SessionHuman | null {
    try {
      const stored = localStorage.getItem(this.STORAGE_KEY);
      if (stored) return JSON.parse(stored) as SessionHuman;
    } catch (err) {
      if (err instanceof Error) {
        console.warn('[SessionHumanService] Failed to parse session:', err.message);
      }
    }
    return null;
  }

  private saveSession(session: SessionHuman): void {
    try {
      localStorage.setItem(this.STORAGE_KEY, JSON.stringify(session));
    } catch {
      // quota exceeded — session-identity is small; this is unlikely
    }
  }
}
