/**
 * Stewardship Service - Graduated Capability Management
 *
 * Client SDK for managing stewardship relationships via ImagoDei zome.
 * This is about identity and self-knowledge - everyone has limits.
 *
 * Key operations:
 * - Get my computed policy (merged from all layers)
 * - Check content/feature access before loading
 * - Manage stewardship grants (create, delegate, revoke)
 * - File and track appeals
 * - View activity logs (if enabled)
 */

import { Injectable, signal, computed, inject } from '@angular/core';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import type {
  StewardshipGrant,
  CreateGrantInput,
  DelegateGrantInput,
  DevicePolicy,
  UpsertPolicyInput,
  ComputedPolicy,
  PolicyDecision,
  TimeAccessDecision,
  StewardshipAppeal,
  FileAppealInput,
  ActivityLog,
  PolicyEvent,
  PolicyChainLink,
} from '../models/stewardship.model';

// =============================================================================
// Types for Zome Calls
// =============================================================================

interface ZomeGrantOutput {
  actionHash: string;
  grant: RawStewardshipGrant;
}

interface ZomePolicyOutput {
  actionHash: string;
  policy: RawDevicePolicy;
}

interface ZomeAppealOutput {
  actionHash: string;
  appeal: RawStewardshipAppeal;
}

interface ZomeActivityLogOutput {
  actionHash: string;
  log: RawActivityLog;
}

// Raw types from zome (snake_case)
interface RawStewardshipGrant {
  id: string;
  stewardId: string;
  subjectId: string;
  tier: string;
  authorityBasis: string;
  evidenceHash: string | null;
  verifiedBy: string;
  contentFiltering: boolean;
  timeLimits: boolean;
  featureRestrictions: boolean;
  activityMonitoring: boolean;
  policyDelegation: boolean;
  delegatable: boolean;
  delegatedFrom: string | null;
  delegationDepth: number;
  grantedAt: string;
  expiresAt: string;
  reviewAt: string;
  status: string;
  appealId: string | null;
  createdAt: string;
  updatedAt: string;
}

interface RawDevicePolicy {
  id: string;
  subjectId: string;
  deviceId: string | null;
  authorId: string;
  authorTier: string;
  inheritsFrom: string | null;
  blockedCategoriesJson: string;
  blockedHashesJson: string;
  ageRatingMax: string | null;
  reachLevelMax: number | null;
  sessionMaxMinutes: number | null;
  dailyMaxMinutes: number | null;
  timeWindowsJson: string;
  cooldownMinutes: number | null;
  disabledFeaturesJson: string;
  disabledRoutesJson: string;
  requireApprovalJson: string;
  logSessions: boolean;
  logCategories: boolean;
  logPolicyEvents: boolean;
  retentionDays: number;
  subjectCanView: boolean;
  effectiveFrom: string;
  effectiveUntil: string | null;
  version: number;
  createdAt: string;
  updatedAt: string;
}

interface RawComputedPolicy {
  subjectId: string;
  computedAt: string;
  blockedCategories: string[];
  blockedHashes: string[];
  ageRatingMax: string | null;
  reachLevelMax: number | null;
  sessionMaxMinutes: number | null;
  dailyMaxMinutes: number | null;
  timeWindowsJson: string;
  cooldownMinutes: number | null;
  disabledFeatures: string[];
  disabledRoutes: string[];
  requireApproval: string[];
  logSessions: boolean;
  logCategories: boolean;
  logPolicyEvents: boolean;
  retentionDays: number;
  subjectCanView: boolean;
}

interface RawStewardshipAppeal {
  id: string;
  appellantId: string;
  grantId: string;
  policyId: string | null;
  appealType: string;
  groundsJson: string;
  evidenceJson: string;
  advocateId: string | null;
  advocateNotes: string | null;
  arbitrationLayer: string;
  assignedTo: string | null;
  status: string;
  statusChangedAt: string | null;
  decisionJson: string | null;
  decisionMadeBy: string | null;
  decisionMadeAt: string | null;
  filedAt: string;
  expiresAt: string;
  createdAt: string;
  updatedAt: string;
}

interface RawActivityLog {
  id: string;
  subjectId: string;
  deviceId: string | null;
  sessionId: string;
  sessionStartedAt: string;
  sessionDurationMinutes: number;
  categoriesAccessedJson: string;
  policyEventsJson: string;
  loggedAt: string;
  retentionExpiresAt: string;
}

interface RawPolicyDecision {
  Allow?: null;
  Block?: { reason: string };
}

// =============================================================================
// Conversion Helpers
// =============================================================================

function toGrant(raw: RawStewardshipGrant): StewardshipGrant {
  return {
    id: raw.id,
    stewardId: raw.stewardId,
    subjectId: raw.subjectId,
    tier: raw.tier as StewardshipGrant['tier'],
    authorityBasis: raw.authorityBasis as StewardshipGrant['authorityBasis'],
    evidenceHash: raw.evidenceHash ?? undefined,
    verifiedBy: raw.verifiedBy,
    contentFiltering: raw.contentFiltering,
    timeLimits: raw.timeLimits,
    featureRestrictions: raw.featureRestrictions,
    activityMonitoring: raw.activityMonitoring,
    policyDelegation: raw.policyDelegation,
    delegatable: raw.delegatable,
    delegatedFrom: raw.delegatedFrom ?? undefined,
    delegationDepth: raw.delegationDepth,
    grantedAt: raw.grantedAt,
    expiresAt: raw.expiresAt,
    reviewAt: raw.reviewAt,
    status: raw.status as StewardshipGrant['status'],
    appealId: raw.appealId ?? undefined,
    createdAt: raw.createdAt,
    updatedAt: raw.updatedAt,
  };
}

function toPolicy(raw: RawDevicePolicy): DevicePolicy {
  const blockedCategories = JSON.parse(raw.blockedCategoriesJson ?? '[]');
  const blockedHashes = JSON.parse(raw.blockedHashesJson ?? '[]');
  const ageRatingMax = (raw.ageRatingMax ?? undefined) as DevicePolicy['ageRatingMax'];
  const reachLevelMax = raw.reachLevelMax ?? undefined;
  const sessionMaxMinutes = raw.sessionMaxMinutes ?? undefined;
  const dailyMaxMinutes = raw.dailyMaxMinutes ?? undefined;
  const timeWindows = JSON.parse(raw.timeWindowsJson ?? '[]');
  const cooldownMinutes = raw.cooldownMinutes ?? undefined;
  const disabledFeatures = JSON.parse(raw.disabledFeaturesJson ?? '[]');
  const disabledRoutes = JSON.parse(raw.disabledRoutesJson ?? '[]');
  const requireApproval = JSON.parse(raw.requireApprovalJson ?? '[]');

  return {
    id: raw.id,
    subjectId: raw.subjectId,
    deviceId: raw.deviceId ?? undefined,
    authorId: raw.authorId,
    authorTier: raw.authorTier as DevicePolicy['authorTier'],
    inheritsFrom: raw.inheritsFrom ?? undefined,
    // Nested rules for convenience
    contentRules: {
      blockedCategories,
      blockedHashes,
      ageRatingMax,
      reachLevelMax,
    },
    timeRules: {
      sessionMaxMinutes,
      dailyMaxMinutes,
      timeWindows,
      cooldownMinutes,
    },
    featureRules: {
      disabledFeatures,
      disabledRoutes,
      requireApproval,
    },
    // Flat rules for backwards compatibility
    blockedCategories,
    blockedHashes,
    ageRatingMax,
    reachLevelMax,
    sessionMaxMinutes,
    dailyMaxMinutes,
    timeWindows,
    cooldownMinutes,
    disabledFeatures,
    disabledRoutes,
    requireApproval,
    logSessions: raw.logSessions,
    logCategories: raw.logCategories,
    logPolicyEvents: raw.logPolicyEvents,
    retentionDays: raw.retentionDays,
    subjectCanView: raw.subjectCanView,
    effectiveFrom: raw.effectiveFrom,
    effectiveUntil: raw.effectiveUntil ?? undefined,
    version: raw.version,
    createdAt: raw.createdAt,
    updatedAt: raw.updatedAt,
  };
}

function toComputedPolicy(raw: RawComputedPolicy): ComputedPolicy {
  return {
    subjectId: raw.subjectId,
    computedAt: raw.computedAt,
    blockedCategories: raw.blockedCategories,
    blockedHashes: raw.blockedHashes,
    ageRatingMax: raw.ageRatingMax ?? undefined,
    reachLevelMax: raw.reachLevelMax ?? undefined,
    sessionMaxMinutes: raw.sessionMaxMinutes ?? undefined,
    dailyMaxMinutes: raw.dailyMaxMinutes ?? undefined,
    timeWindowsJson: raw.timeWindowsJson,
    cooldownMinutes: raw.cooldownMinutes ?? undefined,
    disabledFeatures: raw.disabledFeatures,
    disabledRoutes: raw.disabledRoutes,
    requireApproval: raw.requireApproval,
    logSessions: raw.logSessions,
    logCategories: raw.logCategories,
    logPolicyEvents: raw.logPolicyEvents,
    retentionDays: raw.retentionDays,
    subjectCanView: raw.subjectCanView,
  };
}

function toAppeal(raw: RawStewardshipAppeal): StewardshipAppeal {
  return {
    id: raw.id,
    appellantId: raw.appellantId,
    grantId: raw.grantId,
    policyId: raw.policyId ?? undefined,
    appealType: raw.appealType as StewardshipAppeal['appealType'],
    grounds: JSON.parse(raw.groundsJson ?? '[]'),
    evidenceJson: raw.evidenceJson,
    advocateId: raw.advocateId ?? undefined,
    advocateNotes: raw.advocateNotes ?? undefined,
    arbitrationLayer: raw.arbitrationLayer,
    assignedTo: raw.assignedTo ?? undefined,
    status: raw.status as StewardshipAppeal['status'],
    statusChangedAt: raw.statusChangedAt ?? undefined,
    decision: raw.decisionJson ? JSON.parse(raw.decisionJson) : undefined,
    filedAt: raw.filedAt,
    expiresAt: raw.expiresAt,
    createdAt: raw.createdAt,
    updatedAt: raw.updatedAt,
  };
}

function toActivityLog(raw: RawActivityLog): ActivityLog {
  return {
    id: raw.id,
    subjectId: raw.subjectId,
    deviceId: raw.deviceId ?? undefined,
    sessionId: raw.sessionId,
    sessionStartedAt: raw.sessionStartedAt,
    sessionDurationMinutes: raw.sessionDurationMinutes,
    categoriesAccessed: JSON.parse(raw.categoriesAccessedJson ?? '[]'),
    policyEvents: JSON.parse(raw.policyEventsJson ?? '[]'),
    loggedAt: raw.loggedAt,
    retentionExpiresAt: raw.retentionExpiresAt,
  };
}

function toPolicyDecision(raw: RawPolicyDecision): PolicyDecision {
  if (raw.Block) {
    return { type: 'block', reason: raw.Block.reason };
  }
  return { type: 'allow' };
}

// =============================================================================
// Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class StewardshipService {
  // ===========================================================================
  // Dependencies
  // ===========================================================================

  private readonly holochain = inject(HolochainClientService);

  // ===========================================================================
  // State
  // ===========================================================================

  /** Cached computed policy */
  private readonly policySignal = signal<ComputedPolicy | null>(null);

  /** My grants as steward */
  private readonly mySubjectsSignal = signal<StewardshipGrant[]>([]);

  /** Grants where I am being stewarded */
  private readonly myStewardsSignal = signal<StewardshipGrant[]>([]);

  /** Loading state */
  private readonly loadingSignal = signal(false);

  /** Error state */
  private readonly errorSignal = signal<string | null>(null);

  // ===========================================================================
  // Public Signals (read-only)
  // ===========================================================================

  /** My computed policy (merged from all layers) */
  readonly policy = this.policySignal.asReadonly();

  /** Subjects I steward */
  readonly mySubjects = this.mySubjectsSignal.asReadonly();

  /** Stewards managing my capabilities */
  readonly myStewards = this.myStewardsSignal.asReadonly();

  /** Whether data is loading */
  readonly isLoading = this.loadingSignal.asReadonly();

  /** Current error */
  readonly error = this.errorSignal.asReadonly();

  /** Whether I have any active stewards */
  readonly hasStewards = computed(() => this.myStewardsSignal().length > 0);

  /** Whether I have any subjects */
  readonly hasSubjects = computed(() => this.mySubjectsSignal().length > 0);

  // ===========================================================================
  // Policy Operations
  // ===========================================================================

  /**
   * Get my computed policy (merged from all layers).
   * Caches result for subsequent checks.
   */
  async getMyPolicy(): Promise<ComputedPolicy | null> {
    this.loadingSignal.set(true);
    this.errorSignal.set(null);

    try {
      const result = await this.holochain.callZome<RawComputedPolicy>({
        zomeName: 'imagodei',
        fnName: 'get_my_computed_policy',
        payload: null,
        roleName: 'imagodei',
      });

      if (result.success && result.data) {
        const policy = toComputedPolicy(result.data);
        this.policySignal.set(policy);
        return policy;
      }

      return null;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to get policy';
      this.errorSignal.set(message);
      return null;
    } finally {
      this.loadingSignal.set(false);
    }
  }

  /**
   * Check if content can be accessed based on current policy.
   */
  async checkContentAccess(
    contentHash: string,
    categories: string[],
    ageRating?: string,
    reachLevel?: number
  ): Promise<PolicyDecision> {
    const result = await this.holochain.callZome<RawPolicyDecision>({
      zomeName: 'imagodei',
      fnName: 'check_content_access',
      payload: {
        contentHash: contentHash,
        categories,
        ageRating: ageRating ?? null,
        reachLevel: reachLevel ?? null,
      },
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return toPolicyDecision(result.data);
    }

    // Default to allow if check fails (fail open for user experience)
    return { type: 'allow' };
  }

  /**
   * Check if a feature is accessible based on current policy.
   * Uses cached policy if available.
   */
  async checkFeatureAccess(feature: string): Promise<boolean> {
    // Ensure we have policy
    const policy = this.policySignal() ?? (await this.getMyPolicy());

    if (!policy) {
      return true; // Fail open
    }

    return !policy.disabledFeatures.includes(feature);
  }

  /**
   * Check if a route is accessible based on current policy.
   */
  async checkRouteAccess(route: string): Promise<boolean> {
    const policy = this.policySignal() ?? (await this.getMyPolicy());

    if (!policy) {
      return true; // Fail open
    }

    // Check if route matches any disabled pattern
    for (const pattern of policy.disabledRoutes) {
      if (this.matchRoutePattern(route, pattern)) {
        return false;
      }
    }

    return true;
  }

  /**
   * Check time-based access status.
   * Returns remaining time if applicable.
   */
  async checkTimeAccess(): Promise<TimeAccessDecision> {
    const policy = this.policySignal() ?? (await this.getMyPolicy());

    if (!policy) {
      return { status: 'allowed' };
    }

    // For now, just check if time limits are configured
    // Full implementation would track session/daily usage locally
    return {
      status: 'allowed',
      remainingSession: policy.sessionMaxMinutes,
      remainingDaily: policy.dailyMaxMinutes,
    };
  }

  // ===========================================================================
  // Grant Operations
  // ===========================================================================

  /**
   * Create a stewardship grant for another user.
   */
  async createGrant(input: CreateGrantInput): Promise<StewardshipGrant | null> {
    const result = await this.holochain.callZome<ZomeGrantOutput>({
      zomeName: 'imagodei',
      fnName: 'create_stewardship_grant',
      payload: {
        subjectId: input.subjectId,
        authorityBasis: input.authorityBasis,
        evidenceHash: input.evidenceHash ?? null,
        verifiedBy: input.verifiedBy,
        contentFiltering: input.contentFiltering,
        timeLimits: input.timeLimits,
        featureRestrictions: input.featureRestrictions,
        activityMonitoring: input.activityMonitoring,
        policyDelegation: input.policyDelegation,
        delegatable: input.delegatable,
        expiresInDays: input.expiresInDays,
        reviewInDays: input.reviewInDays,
      },
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      const grant = toGrant(result.data.grant);
      // Update local state
      this.mySubjectsSignal.update(subjects => [...subjects, grant]);
      return grant;
    }

    return null;
  }

  /**
   * Delegate an existing grant to another steward.
   */
  async delegateGrant(input: DelegateGrantInput): Promise<StewardshipGrant | null> {
    const result = await this.holochain.callZome<ZomeGrantOutput>({
      zomeName: 'imagodei',
      fnName: 'delegate_grant',
      payload: {
        parent_grantId: input.parentGrantId,
        new_stewardId: input.newStewardId,
        contentFiltering: input.contentFiltering ?? null,
        timeLimits: input.timeLimits ?? null,
        featureRestrictions: input.featureRestrictions ?? null,
        activityMonitoring: input.activityMonitoring ?? null,
        policyDelegation: input.policyDelegation ?? null,
        expiresInDays: input.expiresInDays,
      },
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return toGrant(result.data.grant);
    }

    return null;
  }

  /**
   * Revoke a stewardship grant.
   */
  async revokeGrant(grantId: string): Promise<boolean> {
    const result = await this.holochain.callZome<ZomeGrantOutput>({
      zomeName: 'imagodei',
      fnName: 'revoke_grant',
      payload: grantId,
      roleName: 'imagodei',
    });

    if (result.success) {
      // Update local state
      this.mySubjectsSignal.update(subjects => subjects.filter(g => g.id !== grantId));
      this.myStewardsSignal.update(stewards => stewards.filter(g => g.id !== grantId));
      return true;
    }

    return false;
  }

  /**
   * Get grants where I am steward (my subjects).
   */
  async getMySubjects(): Promise<StewardshipGrant[]> {
    const result = await this.holochain.callZome<ZomeGrantOutput[]>({
      zomeName: 'imagodei',
      fnName: 'get_my_subjects',
      payload: null,
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      const grants = result.data.map(o => toGrant(o.grant));
      this.mySubjectsSignal.set(grants);
      return grants;
    }

    return [];
  }

  /**
   * Get grants where I am being stewarded (my stewards).
   */
  async getMyStewards(): Promise<StewardshipGrant[]> {
    const result = await this.holochain.callZome<ZomeGrantOutput[]>({
      zomeName: 'imagodei',
      fnName: 'get_my_stewards',
      payload: null,
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      const grants = result.data.map(o => toGrant(o.grant));
      this.myStewardsSignal.set(grants);
      return grants;
    }

    return [];
  }

  // ===========================================================================
  // Policy Management
  // ===========================================================================

  /**
   * Create or update a device policy for a subject.
   * Accepts nested rules format from policy console.
   */
  async upsertPolicy(input: UpsertPolicyInput): Promise<DevicePolicy | null> {
    const result = await this.holochain.callZome<ZomePolicyOutput>({
      zomeName: 'imagodei',
      fnName: 'upsert_policy',
      payload: {
        subjectId: input.subjectId ?? null,
        deviceId: input.deviceId ?? null,
        // Content rules
        blockedCategories: input.contentRules.blockedCategories,
        blockedHashes: input.contentRules.blockedHashes,
        ageRatingMax: input.contentRules.ageRatingMax ?? null,
        reachLevelMax: input.contentRules.reachLevelMax ?? null,
        // Time rules
        sessionMaxMinutes: input.timeRules.sessionMaxMinutes ?? null,
        dailyMaxMinutes: input.timeRules.dailyMaxMinutes ?? null,
        timeWindowsJson: JSON.stringify(input.timeRules.timeWindows),
        cooldownMinutes: input.timeRules.cooldownMinutes ?? null,
        // Feature rules
        disabledFeatures: input.featureRules.disabledFeatures,
        disabledRoutes: input.featureRules.disabledRoutes,
        requireApproval: input.featureRules.requireApproval,
        // Monitoring rules (with defaults)
        logSessions: input.monitoringRules?.logSessions ?? false,
        logCategories: input.monitoringRules?.logCategories ?? false,
        logPolicyEvents: input.monitoringRules?.logPolicyEvents ?? true,
        retentionDays: input.monitoringRules?.retentionDays ?? 30,
        subjectCanView: input.monitoringRules?.subjectCanView ?? true,
      },
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return toPolicy(result.data.policy);
    }

    return null;
  }

  /**
   * Get policies for a subject.
   */
  async getPoliciesForSubject(subjectId: string): Promise<DevicePolicy[]> {
    const result = await this.holochain.callZome<ZomePolicyOutput[]>({
      zomeName: 'imagodei',
      fnName: 'get_policies_for_subject',
      payload: subjectId,
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return result.data.map(o => toPolicy(o.policy));
    }

    return [];
  }

  /**
   * Get my grant for a specific subject.
   */
  async getGrantForSubject(subjectId: string): Promise<StewardshipGrant | null> {
    // First ensure we have subjects loaded
    let subjects = this.mySubjectsSignal();
    if (subjects.length === 0) {
      subjects = await this.getMySubjects();
    }

    // Find grant for this subject
    const grant = subjects.find(g => g.subjectId === subjectId && g.status === 'active');
    return grant ?? null;
  }

  /**
   * Get policy for a specific subject.
   */
  async getSubjectPolicy(subjectId: string): Promise<DevicePolicy | null> {
    const policies = await this.getPoliciesForSubject(subjectId);
    // Return the most recent active policy
    return policies.length > 0 ? policies[0] : null;
  }

  /**
   * Get the parent policy (inherited from) for a subject.
   */
  async getParentPolicy(subjectId: string): Promise<ComputedPolicy | null> {
    const result = await this.holochain.callZome<RawComputedPolicy | null>({
      zomeName: 'imagodei',
      fnName: 'get_parent_policy',
      payload: subjectId,
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return toComputedPolicy(result.data);
    }

    return null;
  }

  /**
   * Get the policy inheritance chain for a subject.
   */
  async getPolicyChain(subjectId: string): Promise<PolicyChainLink[]> {
    const result = await this.holochain.callZome<PolicyChainLink[]>({
      zomeName: 'imagodei',
      fnName: 'get_policy_chain',
      payload: subjectId,
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return result.data;
    }

    return [];
  }

  /**
   * Get my own policy inheritance chain.
   */
  async getMyPolicyChain(): Promise<PolicyChainLink[]> {
    const result = await this.holochain.callZome<PolicyChainLink[]>({
      zomeName: 'imagodei',
      fnName: 'get_my_policy_chain',
      payload: null,
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return result.data;
    }

    return [];
  }

  // ===========================================================================
  // Appeal Operations
  // ===========================================================================

  /**
   * File an appeal against a grant or policy.
   */
  async fileAppeal(input: FileAppealInput): Promise<StewardshipAppeal | null> {
    const result = await this.holochain.callZome<ZomeAppealOutput>({
      zomeName: 'imagodei',
      fnName: 'file_appeal',
      payload: {
        grantId: input.grantId,
        policyId: input.policyId ?? null,
        appealType: input.appealType,
        grounds: input.grounds,
        evidenceJson: input.evidenceJson,
        advocateId: input.advocateId ?? null,
      },
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return toAppeal(result.data.appeal);
    }

    return null;
  }

  /**
   * Get my appeals (where I am appellant).
   */
  async getMyAppeals(): Promise<StewardshipAppeal[]> {
    const result = await this.holochain.callZome<ZomeAppealOutput[]>({
      zomeName: 'imagodei',
      fnName: 'get_my_appeals',
      payload: null,
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return result.data.map(o => toAppeal(o.appeal));
    }

    return [];
  }

  // ===========================================================================
  // Activity Logging
  // ===========================================================================

  /**
   * Log activity (only if monitoring is enabled).
   */
  async logActivity(
    sessionId: string,
    sessionDurationMinutes: number,
    categoriesAccessed: string[],
    policyEvents: PolicyEvent[]
  ): Promise<ActivityLog | null> {
    const result = await this.holochain.callZome<ZomeActivityLogOutput>({
      zomeName: 'imagodei',
      fnName: 'log_activity',
      payload: {
        sessionId: sessionId,
        sessionDurationMinutes: sessionDurationMinutes,
        categories_accessed: categoriesAccessed,
        policyEventsJson: JSON.stringify(policyEvents),
      },
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return toActivityLog(result.data.log);
    }

    return null;
  }

  /**
   * Get my activity logs (if subjectCanView is true).
   */
  async getMyActivityLogs(): Promise<ActivityLog[]> {
    const result = await this.holochain.callZome<ZomeActivityLogOutput[]>({
      zomeName: 'imagodei',
      fnName: 'get_my_activity_logs',
      payload: null,
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return result.data.map(o => toActivityLog(o.log));
    }

    return [];
  }

  // ===========================================================================
  // Initialization
  // ===========================================================================

  /**
   * Load all stewardship data for current user.
   */
  async initialize(): Promise<void> {
    this.loadingSignal.set(true);
    this.errorSignal.set(null);

    try {
      await Promise.all([this.getMyPolicy(), this.getMySubjects(), this.getMyStewards()]);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to initialize stewardship';
      this.errorSignal.set(message);
    } finally {
      this.loadingSignal.set(false);
    }
  }

  /**
   * Clear cached data.
   */
  clearCache(): void {
    this.policySignal.set(null);
    this.mySubjectsSignal.set([]);
    this.myStewardsSignal.set([]);
    this.errorSignal.set(null);
  }

  // ===========================================================================
  // Private Helpers
  // ===========================================================================

  /**
   * Match route against a pattern (supports * and ** wildcards).
   */
  private matchRoutePattern(route: string, pattern: string): boolean {
    // Convert pattern to regex
    const regexPattern = pattern
      .replace(/\*\*/g, '@@DOUBLE_STAR@@')
      .replace(/\*/g, '[^/]*')
      .replace(/@@DOUBLE_STAR@@/g, '.*');

    const regex = new RegExp(`^${regexPattern}$`);
    return regex.test(route);
  }
}
