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
  ContentFilterRules,
  TimeLimitRules,
  FeatureRestrictionRules,
} from '../models/stewardship.model';

// =============================================================================
// Types for Zome Calls
// =============================================================================

interface ZomeGrantOutput {
  action_hash: string;
  grant: RawStewardshipGrant;
}

interface ZomePolicyOutput {
  action_hash: string;
  policy: RawDevicePolicy;
}

interface ZomeAppealOutput {
  action_hash: string;
  appeal: RawStewardshipAppeal;
}

interface ZomeActivityLogOutput {
  action_hash: string;
  log: RawActivityLog;
}

// Raw types from zome (snake_case)
interface RawStewardshipGrant {
  id: string;
  steward_id: string;
  subject_id: string;
  tier: string;
  authority_basis: string;
  evidence_hash: string | null;
  verified_by: string;
  content_filtering: boolean;
  time_limits: boolean;
  feature_restrictions: boolean;
  activity_monitoring: boolean;
  policy_delegation: boolean;
  delegatable: boolean;
  delegated_from: string | null;
  delegation_depth: number;
  granted_at: string;
  expires_at: string;
  review_at: string;
  status: string;
  appeal_id: string | null;
  created_at: string;
  updated_at: string;
}

interface RawDevicePolicy {
  id: string;
  subject_id: string;
  device_id: string | null;
  author_id: string;
  author_tier: string;
  inherits_from: string | null;
  blocked_categories_json: string;
  blocked_hashes_json: string;
  age_rating_max: string | null;
  reach_level_max: number | null;
  session_max_minutes: number | null;
  daily_max_minutes: number | null;
  time_windows_json: string;
  cooldown_minutes: number | null;
  disabled_features_json: string;
  disabled_routes_json: string;
  require_approval_json: string;
  log_sessions: boolean;
  log_categories: boolean;
  log_policy_events: boolean;
  retention_days: number;
  subject_can_view: boolean;
  effective_from: string;
  effective_until: string | null;
  version: number;
  created_at: string;
  updated_at: string;
}

interface RawComputedPolicy {
  subject_id: string;
  computed_at: string;
  blocked_categories: string[];
  blocked_hashes: string[];
  age_rating_max: string | null;
  reach_level_max: number | null;
  session_max_minutes: number | null;
  daily_max_minutes: number | null;
  time_windows_json: string;
  cooldown_minutes: number | null;
  disabled_features: string[];
  disabled_routes: string[];
  require_approval: string[];
  log_sessions: boolean;
  log_categories: boolean;
  log_policy_events: boolean;
  retention_days: number;
  subject_can_view: boolean;
}

interface RawStewardshipAppeal {
  id: string;
  appellant_id: string;
  grant_id: string;
  policy_id: string | null;
  appeal_type: string;
  grounds_json: string;
  evidence_json: string;
  advocate_id: string | null;
  advocate_notes: string | null;
  arbitration_layer: string;
  assigned_to: string | null;
  status: string;
  status_changed_at: string | null;
  decision_json: string | null;
  decision_made_by: string | null;
  decision_made_at: string | null;
  filed_at: string;
  expires_at: string;
  created_at: string;
  updated_at: string;
}

interface RawActivityLog {
  id: string;
  subject_id: string;
  device_id: string | null;
  session_id: string;
  session_started_at: string;
  session_duration_minutes: number;
  categories_accessed_json: string;
  policy_events_json: string;
  logged_at: string;
  retention_expires_at: string;
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
    stewardId: raw.steward_id,
    subjectId: raw.subject_id,
    tier: raw.tier as StewardshipGrant['tier'],
    authorityBasis: raw.authority_basis as StewardshipGrant['authorityBasis'],
    evidenceHash: raw.evidence_hash ?? undefined,
    verifiedBy: raw.verified_by,
    contentFiltering: raw.content_filtering,
    timeLimits: raw.time_limits,
    featureRestrictions: raw.feature_restrictions,
    activityMonitoring: raw.activity_monitoring,
    policyDelegation: raw.policy_delegation,
    delegatable: raw.delegatable,
    delegatedFrom: raw.delegated_from ?? undefined,
    delegationDepth: raw.delegation_depth,
    grantedAt: raw.granted_at,
    expiresAt: raw.expires_at,
    reviewAt: raw.review_at,
    status: raw.status as StewardshipGrant['status'],
    appealId: raw.appeal_id ?? undefined,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

function toPolicy(raw: RawDevicePolicy): DevicePolicy {
  const blockedCategories = JSON.parse(raw.blocked_categories_json || '[]');
  const blockedHashes = JSON.parse(raw.blocked_hashes_json || '[]');
  const ageRatingMax = (raw.age_rating_max ?? undefined) as DevicePolicy['ageRatingMax'];
  const reachLevelMax = raw.reach_level_max ?? undefined;
  const sessionMaxMinutes = raw.session_max_minutes ?? undefined;
  const dailyMaxMinutes = raw.daily_max_minutes ?? undefined;
  const timeWindows = JSON.parse(raw.time_windows_json || '[]');
  const cooldownMinutes = raw.cooldown_minutes ?? undefined;
  const disabledFeatures = JSON.parse(raw.disabled_features_json || '[]');
  const disabledRoutes = JSON.parse(raw.disabled_routes_json || '[]');
  const requireApproval = JSON.parse(raw.require_approval_json || '[]');

  return {
    id: raw.id,
    subjectId: raw.subject_id,
    deviceId: raw.device_id ?? undefined,
    authorId: raw.author_id,
    authorTier: raw.author_tier as DevicePolicy['authorTier'],
    inheritsFrom: raw.inherits_from ?? undefined,
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
    logSessions: raw.log_sessions,
    logCategories: raw.log_categories,
    logPolicyEvents: raw.log_policy_events,
    retentionDays: raw.retention_days,
    subjectCanView: raw.subject_can_view,
    effectiveFrom: raw.effective_from,
    effectiveUntil: raw.effective_until ?? undefined,
    version: raw.version,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

function toComputedPolicy(raw: RawComputedPolicy): ComputedPolicy {
  return {
    subjectId: raw.subject_id,
    computedAt: raw.computed_at,
    blockedCategories: raw.blocked_categories,
    blockedHashes: raw.blocked_hashes,
    ageRatingMax: raw.age_rating_max ?? undefined,
    reachLevelMax: raw.reach_level_max ?? undefined,
    sessionMaxMinutes: raw.session_max_minutes ?? undefined,
    dailyMaxMinutes: raw.daily_max_minutes ?? undefined,
    timeWindowsJson: raw.time_windows_json,
    cooldownMinutes: raw.cooldown_minutes ?? undefined,
    disabledFeatures: raw.disabled_features,
    disabledRoutes: raw.disabled_routes,
    requireApproval: raw.require_approval,
    logSessions: raw.log_sessions,
    logCategories: raw.log_categories,
    logPolicyEvents: raw.log_policy_events,
    retentionDays: raw.retention_days,
    subjectCanView: raw.subject_can_view,
  };
}

function toAppeal(raw: RawStewardshipAppeal): StewardshipAppeal {
  return {
    id: raw.id,
    appellantId: raw.appellant_id,
    grantId: raw.grant_id,
    policyId: raw.policy_id ?? undefined,
    appealType: raw.appeal_type as StewardshipAppeal['appealType'],
    grounds: JSON.parse(raw.grounds_json || '[]'),
    evidenceJson: raw.evidence_json,
    advocateId: raw.advocate_id ?? undefined,
    advocateNotes: raw.advocate_notes ?? undefined,
    arbitrationLayer: raw.arbitration_layer,
    assignedTo: raw.assigned_to ?? undefined,
    status: raw.status as StewardshipAppeal['status'],
    statusChangedAt: raw.status_changed_at ?? undefined,
    decision: raw.decision_json ? JSON.parse(raw.decision_json) : undefined,
    filedAt: raw.filed_at,
    expiresAt: raw.expires_at,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

function toActivityLog(raw: RawActivityLog): ActivityLog {
  return {
    id: raw.id,
    subjectId: raw.subject_id,
    deviceId: raw.device_id ?? undefined,
    sessionId: raw.session_id,
    sessionStartedAt: raw.session_started_at,
    sessionDurationMinutes: raw.session_duration_minutes,
    categoriesAccessed: JSON.parse(raw.categories_accessed_json || '[]'),
    policyEvents: JSON.parse(raw.policy_events_json || '[]'),
    loggedAt: raw.logged_at,
    retentionExpiresAt: raw.retention_expires_at,
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
    reachLevel?: number,
  ): Promise<PolicyDecision> {
    const result = await this.holochain.callZome<RawPolicyDecision>({
      zomeName: 'imagodei',
      fnName: 'check_content_access',
      payload: {
        content_hash: contentHash,
        categories,
        age_rating: ageRating ?? null,
        reach_level: reachLevel ?? null,
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
    let policy = this.policySignal();
    if (!policy) {
      policy = await this.getMyPolicy();
    }

    if (!policy) {
      return true; // Fail open
    }

    return !policy.disabledFeatures.includes(feature);
  }

  /**
   * Check if a route is accessible based on current policy.
   */
  async checkRouteAccess(route: string): Promise<boolean> {
    let policy = this.policySignal();
    if (!policy) {
      policy = await this.getMyPolicy();
    }

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
    let policy = this.policySignal();
    if (!policy) {
      policy = await this.getMyPolicy();
    }

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
        subject_id: input.subjectId,
        authority_basis: input.authorityBasis,
        evidence_hash: input.evidenceHash ?? null,
        verified_by: input.verifiedBy,
        content_filtering: input.contentFiltering,
        time_limits: input.timeLimits,
        feature_restrictions: input.featureRestrictions,
        activity_monitoring: input.activityMonitoring,
        policy_delegation: input.policyDelegation,
        delegatable: input.delegatable,
        expires_in_days: input.expiresInDays,
        review_in_days: input.reviewInDays,
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
        parent_grant_id: input.parentGrantId,
        new_steward_id: input.newStewardId,
        content_filtering: input.contentFiltering ?? null,
        time_limits: input.timeLimits ?? null,
        feature_restrictions: input.featureRestrictions ?? null,
        activity_monitoring: input.activityMonitoring ?? null,
        policy_delegation: input.policyDelegation ?? null,
        expires_in_days: input.expiresInDays,
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
      this.mySubjectsSignal.update(subjects =>
        subjects.filter(g => g.id !== grantId)
      );
      this.myStewardsSignal.update(stewards =>
        stewards.filter(g => g.id !== grantId)
      );
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
        subject_id: input.subjectId ?? null,
        device_id: input.deviceId ?? null,
        // Content rules
        blocked_categories: input.contentRules.blockedCategories,
        blocked_hashes: input.contentRules.blockedHashes,
        age_rating_max: input.contentRules.ageRatingMax ?? null,
        reach_level_max: input.contentRules.reachLevelMax ?? null,
        // Time rules
        session_max_minutes: input.timeRules.sessionMaxMinutes ?? null,
        daily_max_minutes: input.timeRules.dailyMaxMinutes ?? null,
        time_windows_json: JSON.stringify(input.timeRules.timeWindows),
        cooldown_minutes: input.timeRules.cooldownMinutes ?? null,
        // Feature rules
        disabled_features: input.featureRules.disabledFeatures,
        disabled_routes: input.featureRules.disabledRoutes,
        require_approval: input.featureRules.requireApproval,
        // Monitoring rules (with defaults)
        log_sessions: input.monitoringRules?.logSessions ?? false,
        log_categories: input.monitoringRules?.logCategories ?? false,
        log_policy_events: input.monitoringRules?.logPolicyEvents ?? true,
        retention_days: input.monitoringRules?.retentionDays ?? 30,
        subject_can_view: input.monitoringRules?.subjectCanView ?? true,
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
        grant_id: input.grantId,
        policy_id: input.policyId ?? null,
        appeal_type: input.appealType,
        grounds: input.grounds,
        evidence_json: input.evidenceJson,
        advocate_id: input.advocateId ?? null,
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
    policyEvents: PolicyEvent[],
  ): Promise<ActivityLog | null> {
    const result = await this.holochain.callZome<ZomeActivityLogOutput>({
      zomeName: 'imagodei',
      fnName: 'log_activity',
      payload: {
        session_id: sessionId,
        session_duration_minutes: sessionDurationMinutes,
        categories_accessed: categoriesAccessed,
        policy_events_json: JSON.stringify(policyEvents),
      },
      roleName: 'imagodei',
    });

    if (result.success && result.data) {
      return toActivityLog(result.data.log);
    }

    return null;
  }

  /**
   * Get my activity logs (if subject_can_view is true).
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
      await Promise.all([
        this.getMyPolicy(),
        this.getMySubjects(),
        this.getMyStewards(),
      ]);
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
