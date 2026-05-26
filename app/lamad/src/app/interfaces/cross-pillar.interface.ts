/**
 * Cross-pillar P+inversion interfaces for lamad.
 *
 * Services that stay in elohim-app (due to non-lamad deps) expose narrow
 * injection tokens here. Lamad consumers inject via these tokens; the concrete
 * classes are registered in lamad's app.config.ts via { useExisting: ... }.
 *
 * Tokens in this file:
 *   LAMAD_HOLOCHAIN_CLIENT  — HolochainClientService (narrow: callZome, isConnected)
 *   LAMAD_AFFINITY_TRACKING — AffinityTrackingService (narrow: affinity ops)
 *   LAMAD_EPR_RESOLVER      — EprResolverService (narrow: resolve, resolveUrl, etc.)
 *   LAMAD_GOVERNANCE_SIGNAL — GovernanceSignalService (narrow: recordLearningSignal)
 *   LAMAD_ELOHIM_PRESENCE   — ElohimPresenceService (narrow: onDiscoveryCompleted)
 *   LAMAD_HUMAN_CONSENT     — HumanConsentService (narrow: getConsentWith)
 *   LAMAD_GOVERNANCE        — GovernanceService (narrow: state + challenge + discussion)
 *   LAMAD_PROFILE           — ProfileService (narrow: getCurrentFocus, resume, paths, timeline)
 *   LAMAD_ELOHIM_AGENT      — ElohimAgentService (narrow: invoke)
 *   LAMAD_CONTEXT_ASSEMBLY  — ContextAssemblyService (narrow: assembleAndNegotiate)
 *
 * Slice 2.1c: Cross-pillar import cleanup.
 */

import { InjectionToken } from '@angular/core';
import { Observable } from 'rxjs';
import type { EprRef } from '@elohim/service';

// ============================================================================
// Shared narrow types (re-shaped to avoid cross-pillar model imports)
// ============================================================================

/** Narrow shape for a Holochain zome call result. */
export interface LamadZomeCallResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

/** Narrow shape for a Holochain zome call input. */
export interface LamadZomeCallInput {
  zomeName: string;
  fnName: string;
  payload: unknown;
  roleName?: string;
}

/** Resolved EPR — transport-specific URL + in-app route. */
export interface LamadResolvedEpr {
  ref: EprRef;
  url: string;
  route: string[] | null;
}

/** Resolved EPR content — includes content metadata. */
export interface LamadResolvedContent {
  ref: EprRef;
  content: unknown;
  blobUrl: string | null;
  route: string[];
}

/** Context-aware EPR resolution result. */
export interface LamadContextResolvedRoute {
  route: string[];
  resolution: 'in-path' | 'cross-path' | 'standalone';
  stepIndex?: number;
  crossPath?: { pathId: string; stepIndex: number };
}

/** Lightweight step reference for context-aware EPR resolution. */
export interface LamadStepRef {
  resourceId: string;
  order: number;
}

/** Cross-path lookup result. */
export interface LamadCrossPathMatch {
  pathId: string;
  stepIndex: number;
}

/** Narrow EprHead for popover resolution. */
export interface LamadEprHead {
  id: string;
  title: string;
  description?: string;
  contentType?: string;
}

/** Aggregated governance signals for a content node. */
export interface LamadAggregatedSignals {
  contentId: string;
  reactionCounts: Record<string, number>;
  feedbackStats: unknown;
  learningSignalCount: number;
  completionCount: number;
  sentimentScore: number;
  effectivenessScore: number;
  consensusState: {
    level: 'strong' | 'moderate' | 'contested' | 'unknown';
    confidence: number;
    dominantPosition: string | null;
    polarization: number;
    [key: string]: unknown;
  };
  lastUpdated: string;
}

/** Learning signal input for governance recording. */
export interface LamadLearningSignalInput {
  contentId: string;
  signalType:
    | 'content_viewed'
    | 'progress_update'
    | 'quiz_attempt'
    | 'mastery_achieved'
    | 'interactive_completion';
  payload: Record<string, unknown>;
}

/** Narrow presence moment for assessment completion display. */
export interface LamadPresenceMoment {
  id: string;
  type: string;
  capability: string;
  message: string;
  reasoning: { primaryPrinciple: string; [key: string]: unknown };
  cost: unknown;
  dismissible: boolean;
  createdAt: Date;
}

/** Narrow presence response from onDiscoveryCompleted. */
export interface LamadPresenceResponse {
  status: 'fulfilled' | 'rejected';
  requestId: string;
  constitutionalReasoning: { primaryPrinciple: string; [key: string]: unknown };
  cost: unknown;
  respondedAt: Date;
}

/** HumanConsent narrow shape (only what path-negotiation uses). */
export interface LamadHumanConsent {
  humanId: string;
  intimacyLevel: string;
  consentState: string;
  grantedAt: Date;
  expiresAt?: Date;
}

/** Governance state narrow shape. */
export interface LamadGovernanceState {
  entityType: string;
  entityId: string;
  status: string;
  [key: string]: unknown;
}

/** Challenge record narrow shape. */
export interface LamadChallengeRecord {
  id: string;
  entityType: string;
  entityId: string;
  [key: string]: unknown;
}

/** Discussion record narrow shape. */
export interface LamadDiscussionRecord {
  id: string;
  entityType: string;
  entityId: string;
  [key: string]: unknown;
}

/** Resume point narrow shape. */
export interface LamadResumePoint {
  contentId: string;
  pathId?: string;
  stepIndex?: number;
  progressPercent?: number;
}

/** Timeline event narrow shape. */
export interface LamadTimelineEvent {
  type: string;
  contentId?: string;
  title?: string;
  timestamp: string;
  [key: string]: unknown;
}

/** Paths overview narrow shape. */
export interface LamadPathsOverview {
  inProgress: unknown[];
  completed: unknown[];
  suggested: unknown[];
}

/** Current focus item narrow shape. */
export interface LamadCurrentFocus {
  pathId: string;
  pathTitle: string;
  currentStepIndex: number;
  totalSteps: number;
  progressPercent: number;
  lastActiveAt: string;
  [key: string]: unknown;
}

/** Elohim request narrow shape (what knowledge-map.service calls). */
export interface LamadElohimRequest {
  requestId: string;
  targetElohimId: string;
  capability: string;
  params: Record<string, unknown>;
  requesterId: string;
  priority: 'low' | 'normal' | 'high';
  requestedAt: string;
}

/** Elohim response narrow shape. */
export interface LamadElohimResponse {
  status: 'fulfilled' | 'rejected';
  requestId: string;
  data?: unknown;
  constitutionalReasoning: { primaryPrinciple: string; [key: string]: unknown };
  respondedAt: Date;
  cost: unknown;
  [key: string]: unknown;
}

/** Affinity change event narrow shape. */
export interface LamadAffinityChangeEvent {
  nodeId: string;
  oldValue: number;
  newValue: number;
  timestamp: string;
}

/** Category affinity stats narrow shape (matches @app/qahal structurally). */
export interface LamadCategoryAffinityStats {
  category: string;
  nodeCount: number;
  averageAffinity: number;
  engagedCount: number;
}

/** Affinity stats narrow shape. */
export interface LamadAffinityStats {
  byCategoryId?: Map<string, LamadCategoryAffinityStats>;
  byCategory?: Map<string, LamadCategoryAffinityStats>;
  totalEngaged?: number;
  averageAffinity?: number;
  [key: string]: unknown;
}

/** Context assembly result narrow shape. */
export interface LamadContextAssemblyResult {
  status: 'fulfilled' | 'rejected';
  [key: string]: unknown;
}

// ============================================================================
// ILamadHolochainClient — narrow Holochain client contract
// ============================================================================

export interface ILamadHolochainClient {
  isConnected: import('@angular/core').Signal<boolean>;
  callZome<T>(input: LamadZomeCallInput): Promise<LamadZomeCallResult<T>>;
  connect(config?: Record<string, unknown>): Promise<void>;
  waitForConnection(timeoutMs?: number): Promise<boolean>;
}

export const LAMAD_HOLOCHAIN_CLIENT = new InjectionToken<ILamadHolochainClient>(
  'LamadHolochainClient'
);

// ============================================================================
// ILamadAffinityTracking — narrow affinity tracking contract
// ============================================================================

export interface ILamadAffinityTracking {
  affinity$: Observable<Record<string, number>>;
  changes$: Observable<LamadAffinityChangeEvent | null>;
  getAffinity(nodeId: string): number;
  setAffinity(nodeId: string, value: number): void;
  incrementAffinity(nodeId: string, delta: number): void;
  trackView(nodeId: string): void;
  getStats(contentNodes: { id: string; [key: string]: unknown }[]): LamadAffinityStats;
}

export const LAMAD_AFFINITY_TRACKING = new InjectionToken<ILamadAffinityTracking>(
  'LamadAffinityTracking'
);

// ============================================================================
// ILamadEprResolver — narrow EPR resolver contract
// ============================================================================

export interface ILamadEprResolver {
  resolveUrl(input: string, blobHash?: string): LamadResolvedEpr;
  resolve(input: string): Observable<LamadResolvedContent | null>;
  resolveBlobUrl(hash: string): string;
  resolveInContext(
    eprUri: string,
    currentPathId: string | null,
    currentSteps: LamadStepRef[],
    crossPathMatches?: LamadCrossPathMatch[]
  ): LamadContextResolvedRoute;
  resolveEprHead(input: string): Observable<LamadEprHead | null>;
}

export const LAMAD_EPR_RESOLVER = new InjectionToken<ILamadEprResolver>('LamadEprResolver');

// ============================================================================
// ILamadGovernanceSignal — narrow governance signal contract
// ============================================================================

export interface ILamadGovernanceSignal {
  recordLearningSignal(signal: LamadLearningSignalInput): Observable<boolean>;
  getContentSignals(contentId: string): Observable<LamadAggregatedSignals>;
}

export const LAMAD_GOVERNANCE_SIGNAL = new InjectionToken<ILamadGovernanceSignal>(
  'LamadGovernanceSignal'
);

// ============================================================================
// ILamadElohimPresence — narrow elohim presence contract
// ============================================================================

export interface ILamadElohimPresence {
  onDiscoveryCompleted(
    nodeId: string,
    title: string,
    learnerId: string
  ): Observable<LamadPresenceResponse>;
}

export const LAMAD_ELOHIM_PRESENCE = new InjectionToken<ILamadElohimPresence>(
  'LamadElohimPresence'
);

// ============================================================================
// ILamadHumanConsent — narrow human consent contract
// ============================================================================

export interface ILamadHumanConsent {
  getConsentWith(humanId: string): Observable<LamadHumanConsent | null>;
}

export const LAMAD_HUMAN_CONSENT = new InjectionToken<ILamadHumanConsent>('LamadHumanConsent');

// ============================================================================
// ILamadGovernance — narrow governance contract
// ============================================================================

export interface ILamadGovernance {
  getGovernanceState(
    entityType: string,
    entityId: string
  ): Observable<LamadGovernanceState | null>;
  getChallengesForEntity(
    entityType: string,
    entityId: string
  ): Observable<LamadChallengeRecord[]>;
  getDiscussionsForEntity(
    entityType: string,
    entityId: string
  ): Observable<LamadDiscussionRecord[]>;
}

export const LAMAD_GOVERNANCE = new InjectionToken<ILamadGovernance>('LamadGovernance');

// ============================================================================
// ILamadProfile — narrow profile contract
// ============================================================================

export interface ILamadProfile {
  getCurrentFocus(): Observable<LamadCurrentFocus[]>;
  getResumePoint(): Observable<LamadResumePoint | null>;
  getPathsOverview(): Observable<LamadPathsOverview>;
  getTimeline(limit: number): Observable<LamadTimelineEvent[]>;
}

export const LAMAD_PROFILE = new InjectionToken<ILamadProfile>('LamadProfile');

// ============================================================================
// ILamadElohimAgent — narrow elohim agent contract
// ============================================================================

export interface ILamadElohimAgent {
  invoke(request: LamadElohimRequest): Observable<LamadElohimResponse>;
}

export const LAMAD_ELOHIM_AGENT = new InjectionToken<ILamadElohimAgent>('LamadElohimAgent');

// ============================================================================
// ILamadContextAssembly — narrow context assembly contract
// ============================================================================

export interface ILamadContextAssembly {
  assembleAndNegotiate(payload: unknown): Observable<LamadContextAssemblyResult>;
}

export const LAMAD_CONTEXT_ASSEMBLY = new InjectionToken<ILamadContextAssembly>(
  'LamadContextAssembly'
);

// ============================================================================
// ILamadLensRegistry — narrow lens registry contract (scope-sequence registration)
// ============================================================================

export interface ILamadLensProvider {
  type: string;
  [key: string]: unknown;
}

export interface ILamadLensRegistry {
  register(provider: ILamadLensProvider): void;
}

export const LAMAD_LENS_REGISTRY = new InjectionToken<ILamadLensRegistry>('LamadLensRegistry');
