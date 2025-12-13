/**
 * Zome Client
 *
 * Low-level typed wrapper around zome calls.
 * Similar to Spring's RestTemplate - handles the raw communication.
 * Service classes use this for higher-level operations.
 */

import { HolochainConnection } from '../connection.js';
import {
  ZOME_NAME,
  type ContentOutput,
  type CreateContentInput,
  type BulkCreateContentInput,
  type BulkCreateContentOutput,
  type QueryByIdInput,
  type QueryByTypeInput,
  type ContentStats,
  type CreatePathInput,
  type AddPathStepInput,
  type PathWithSteps,
  type PathIndex,
  type PathStepOutput,
  type UpdatePathInput,
  type UpdateStepInput,
  // Chapter types
  type CreateChapterInput,
  type ChapterOutput,
  type ChapterWithSteps,
  type PathWithChaptersAndSteps,
  type UpdateChapterInput,
  // Progress tracking types
  type StartPathProgressInput,
  type CompleteStepInput,
  type ProgressSummary,
  type GrantAttestationInput,
  type CheckAttestationAccessInput,
  type AttestationAccessResult,
  type CreateRelationshipInput,
  type RelationshipOutput,
  type GetRelationshipsInput,
  type QueryRelatedContentInput,
  type ContentGraph,
  type CreateHumanInput,
  type HumanOutput,
  type QueryHumansByAffinityInput,
  type RecordCompletionInput,
  // Human Presence (Secure Login)
  type RegisterHumanInput,
  type HumanSessionOutput,
  type UpdateHumanInput,
  type LinkExternalIdentityInput,
  // Imago Dei: Expanded identity models
  type CreateAgentInput,
  type AgentOutput,
  type QueryAgentsInput,
  // Agent Presence (Elohim Service Layer)
  type RegisterElohimInput,
  type AgentSessionOutput,
  type UpdateAgentStateInput,
  type CreateAgentProgressInput,
  type AgentProgressOutput,
  type UpdateAgentProgressInput,
  type UpsertMasteryInput,
  type ContentMasteryOutput,
  type QueryMasteryInput,
  type InitializeMasteryInput,
  type RecordEngagementInput,
  type RecordAssessmentInput,
  type CheckPrivilegeInput,
  type PrivilegeCheckResult,
  type MasterySnapshot,
  type PathMasteryOverview,
  type MasteryStats,
  type AssessmentHistory,
  type CheckAttestationEligibilityInput,
  type AttestationEligibilityResult,
  type GrantAttestationWithMasteryInput,
  type StepAccessResult,
  // Practice Pool & Mastery Challenge
  type PracticePoolOutput,
  type MasteryChallengeOutput,
  type CreatePoolInput,
  type StartChallengeInput,
  type SubmitChallengeInput,
  type ChallengeResult,
  type CooldownCheckResult,
  type PoolRecommendations,
  type CreateAttestationInput,
  type AttestationOutput,
  type QueryAttestationsInput,
  // Shefa: Economic models
  type CreateEconomicEventInput,
  type EconomicEventOutput,
  type CreateEconomicResourceInput,
  type EconomicResourceOutput,
  type CreateContributorPresenceInput,
  type ContributorPresenceOutput,
  type QueryPresencesInput,
  type BeginStewardshipInput,
  type InitiateClaimInput,
  type CreateProcessInput,
  type ProcessOutput,
  type CreateIntentInput,
  type IntentOutput,
  type CreateCommitmentInput,
  type CommitmentOutput,
  type CreateAppreciationInput,
  type AppreciationOutput,
  type CreateClaimInput,
  type ClaimOutput,
  type CreateSettlementInput,
  type SettlementOutput,
  // Lamad: Learning Economy (uses Shefa hREA primitives)
  type EarnLamadPointsInput,
  type EarnLamadPointsResult,
  type LearnerPointBalanceOutput,
  type LamadPointEventOutput,
  type LamadContributorDashboard,
  type LamadContributorRecognitionOutput,
  type LamadContributorImpactOutput,
  // Lamad: Steward Economy
  type CreateStewardCredentialInput,
  type StewardCredentialOutput,
  type CreatePremiumGateInput,
  type PremiumGateOutput,
  type GrantAccessInput,
  type AccessGrantOutput,
  type StewardRevenueSummary,
} from '../types.js';
import type { ActionHash } from '@holochain/client';

/**
 * Typed zome client for content_store zome
 *
 * Each method corresponds to a zome function with proper typing.
 * Use BatchExecutor for bulk operations to handle WASM memory limits.
 */
export class ZomeClient {
  private connection: HolochainConnection;
  private zomeName: string;

  constructor(connection: HolochainConnection, zomeName: string = ZOME_NAME) {
    this.connection = connection;
    this.zomeName = zomeName;
  }

  // ==========================================================================
  // Content Operations
  // ==========================================================================

  async createContent(input: CreateContentInput): Promise<ContentOutput> {
    return this.connection.callZome<ContentOutput>(
      this.zomeName,
      'create_content',
      input
    );
  }

  async bulkCreateContent(
    input: BulkCreateContentInput
  ): Promise<BulkCreateContentOutput> {
    return this.connection.callZome<BulkCreateContentOutput>(
      this.zomeName,
      'bulk_create_content',
      input
    );
  }

  async getContent(actionHash: ActionHash): Promise<ContentOutput | null> {
    return this.connection.callZome<ContentOutput | null>(
      this.zomeName,
      'get_content',
      actionHash
    );
  }

  async getContentById(id: string): Promise<ContentOutput | null> {
    return this.connection.callZome<ContentOutput | null>(
      this.zomeName,
      'get_content_by_id',
      { id } as QueryByIdInput
    );
  }

  async getContentByType(
    contentType: string,
    limit?: number
  ): Promise<ContentOutput[]> {
    return this.connection.callZome<ContentOutput[]>(
      this.zomeName,
      'get_content_by_type',
      { content_type: contentType, limit } as QueryByTypeInput
    );
  }

  async getContentByTag(tag: string): Promise<ContentOutput[]> {
    return this.connection.callZome<ContentOutput[]>(
      this.zomeName,
      'get_content_by_tag',
      tag
    );
  }

  async getMyContent(): Promise<ContentOutput[]> {
    return this.connection.callZome<ContentOutput[]>(
      this.zomeName,
      'get_my_content',
      null
    );
  }

  async getContentStats(): Promise<ContentStats> {
    return this.connection.callZome<ContentStats>(
      this.zomeName,
      'get_content_stats',
      null
    );
  }

  // ==========================================================================
  // Learning Path Operations
  // ==========================================================================

  async createPath(input: CreatePathInput): Promise<ActionHash> {
    return this.connection.callZome<ActionHash>(
      this.zomeName,
      'create_path',
      input
    );
  }

  async addPathStep(input: AddPathStepInput): Promise<ActionHash> {
    return this.connection.callZome<ActionHash>(
      this.zomeName,
      'add_path_step',
      input
    );
  }

  async getPathWithSteps(pathId: string): Promise<PathWithSteps | null> {
    return this.connection.callZome<PathWithSteps | null>(
      this.zomeName,
      'get_path_with_steps',
      pathId
    );
  }

  async getAllPaths(): Promise<PathIndex> {
    return this.connection.callZome<PathIndex>(
      this.zomeName,
      'get_all_paths',
      null
    );
  }

  async deletePath(pathId: string): Promise<boolean> {
    return this.connection.callZome<boolean>(
      this.zomeName,
      'delete_path',
      pathId
    );
  }

  async updatePath(input: UpdatePathInput): Promise<PathWithSteps> {
    return this.connection.callZome<PathWithSteps>(
      this.zomeName,
      'update_path',
      input
    );
  }

  async updateStep(input: UpdateStepInput): Promise<PathStepOutput> {
    return this.connection.callZome<PathStepOutput>(
      this.zomeName,
      'update_step',
      input
    );
  }

  async getStepById(stepId: string): Promise<PathStepOutput | null> {
    return this.connection.callZome<PathStepOutput | null>(
      this.zomeName,
      'get_step_by_id',
      stepId
    );
  }

  // ==========================================================================
  // Chapter Operations
  // ==========================================================================

  async createChapter(input: CreateChapterInput): Promise<ChapterOutput> {
    return this.connection.callZome<ChapterOutput>(
      this.zomeName,
      'create_chapter',
      input
    );
  }

  async getChapterById(chapterId: string): Promise<ChapterOutput | null> {
    return this.connection.callZome<ChapterOutput | null>(
      this.zomeName,
      'get_chapter_by_id',
      chapterId
    );
  }

  async getChaptersForPath(pathId: string): Promise<ChapterWithSteps[]> {
    return this.connection.callZome<ChapterWithSteps[]>(
      this.zomeName,
      'get_chapters_for_path',
      pathId
    );
  }

  async getPathFull(pathId: string): Promise<PathWithChaptersAndSteps | null> {
    return this.connection.callZome<PathWithChaptersAndSteps | null>(
      this.zomeName,
      'get_path_full',
      pathId
    );
  }

  async updateChapter(input: UpdateChapterInput): Promise<ChapterOutput> {
    return this.connection.callZome<ChapterOutput>(
      this.zomeName,
      'update_chapter',
      input
    );
  }

  // ==========================================================================
  // Progress Tracking Operations
  // ==========================================================================

  async startPathProgress(input: StartPathProgressInput): Promise<AgentProgressOutput> {
    return this.connection.callZome<AgentProgressOutput>(
      this.zomeName,
      'start_path_progress',
      input
    );
  }

  async completeStep(input: CompleteStepInput): Promise<AgentProgressOutput> {
    return this.connection.callZome<AgentProgressOutput>(
      this.zomeName,
      'complete_step',
      input
    );
  }

  async completePath(pathId: string): Promise<AgentProgressOutput> {
    return this.connection.callZome<AgentProgressOutput>(
      this.zomeName,
      'complete_path',
      pathId
    );
  }

  async getMyPathProgress(pathId: string): Promise<AgentProgressOutput | null> {
    return this.connection.callZome<AgentProgressOutput | null>(
      this.zomeName,
      'get_my_path_progress',
      pathId
    );
  }

  async getMyAllProgress(): Promise<AgentProgressOutput[]> {
    return this.connection.callZome<AgentProgressOutput[]>(
      this.zomeName,
      'get_my_all_progress',
      null
    );
  }

  async getProgressByStatus(status: string): Promise<AgentProgressOutput[]> {
    return this.connection.callZome<AgentProgressOutput[]>(
      this.zomeName,
      'get_progress_by_status',
      status
    );
  }

  async getMyProgressSummaries(): Promise<ProgressSummary[]> {
    return this.connection.callZome<ProgressSummary[]>(
      this.zomeName,
      'get_my_progress_summaries',
      null
    );
  }

  // ==========================================================================
  // Attestation Operations
  // ==========================================================================

  async grantAttestation(input: GrantAttestationInput): Promise<AgentProgressOutput> {
    return this.connection.callZome<AgentProgressOutput>(
      this.zomeName,
      'grant_attestation',
      input
    );
  }

  async checkAttestationAccess(input: CheckAttestationAccessInput): Promise<AttestationAccessResult> {
    return this.connection.callZome<AttestationAccessResult>(
      this.zomeName,
      'check_attestation_access',
      input
    );
  }

  // ==========================================================================
  // Relationship Operations
  // ==========================================================================

  async createRelationship(
    input: CreateRelationshipInput
  ): Promise<RelationshipOutput> {
    return this.connection.callZome<RelationshipOutput>(
      this.zomeName,
      'create_relationship',
      input
    );
  }

  async getRelationships(
    input: GetRelationshipsInput
  ): Promise<RelationshipOutput[]> {
    return this.connection.callZome<RelationshipOutput[]>(
      this.zomeName,
      'get_relationships',
      input
    );
  }

  async queryRelatedContent(
    input: QueryRelatedContentInput
  ): Promise<ContentOutput[]> {
    return this.connection.callZome<ContentOutput[]>(
      this.zomeName,
      'query_related_content',
      input
    );
  }

  async getContentGraph(input: QueryRelatedContentInput): Promise<ContentGraph> {
    return this.connection.callZome<ContentGraph>(
      this.zomeName,
      'get_content_graph',
      input
    );
  }

  // ==========================================================================
  // Human Operations
  // ==========================================================================

  async createHuman(input: CreateHumanInput): Promise<HumanOutput> {
    return this.connection.callZome<HumanOutput>(
      this.zomeName,
      'create_human',
      input
    );
  }

  async getHumanById(id: string): Promise<HumanOutput | null> {
    return this.connection.callZome<HumanOutput | null>(
      this.zomeName,
      'get_human_by_id',
      id
    );
  }

  async queryHumansByAffinity(
    input: QueryHumansByAffinityInput
  ): Promise<HumanOutput[]> {
    return this.connection.callZome<HumanOutput[]>(
      this.zomeName,
      'query_humans_by_affinity',
      input
    );
  }

  async recordContentCompletion(input: RecordCompletionInput): Promise<boolean> {
    return this.connection.callZome<boolean>(
      this.zomeName,
      'record_content_completion',
      input
    );
  }

  // ==========================================================================
  // Human Presence Operations (Secure Login)
  // ==========================================================================

  async registerHuman(input: RegisterHumanInput): Promise<HumanSessionOutput> {
    return this.connection.callZome<HumanSessionOutput>(
      this.zomeName,
      'register_human',
      input
    );
  }

  async getCurrentHuman(): Promise<HumanSessionOutput | null> {
    return this.connection.callZome<HumanSessionOutput | null>(
      this.zomeName,
      'get_current_human',
      null
    );
  }

  async updateHumanProfile(input: UpdateHumanInput): Promise<HumanOutput> {
    return this.connection.callZome<HumanOutput>(
      this.zomeName,
      'update_human_profile',
      input
    );
  }

  async linkExternalIdentity(
    input: LinkExternalIdentityInput
  ): Promise<boolean> {
    return this.connection.callZome<boolean>(
      this.zomeName,
      'link_external_identity',
      input
    );
  }

  async getHumanByExternalIdentity(
    input: LinkExternalIdentityInput
  ): Promise<HumanOutput | null> {
    return this.connection.callZome<HumanOutput | null>(
      this.zomeName,
      'get_human_by_external_identity',
      input
    );
  }

  // ==========================================================================
  // Agent Operations (Expanded Identity Model)
  // ==========================================================================

  async createAgent(input: CreateAgentInput): Promise<AgentOutput> {
    return this.connection.callZome<AgentOutput>(
      this.zomeName,
      'create_agent',
      input
    );
  }

  async getAgentById(id: string): Promise<AgentOutput | null> {
    return this.connection.callZome<AgentOutput | null>(
      this.zomeName,
      'get_agent_by_id',
      id
    );
  }

  async queryAgents(input: QueryAgentsInput): Promise<AgentOutput[]> {
    return this.connection.callZome<AgentOutput[]>(
      this.zomeName,
      'query_agents',
      input
    );
  }

  // ==========================================================================
  // Agent Presence Operations (Elohim Service Layer)
  // ==========================================================================

  async getCurrentAgent(): Promise<AgentSessionOutput | null> {
    return this.connection.callZome<AgentSessionOutput | null>(
      this.zomeName,
      'get_current_agent',
      null
    );
  }

  async registerElohim(input: RegisterElohimInput): Promise<AgentSessionOutput> {
    return this.connection.callZome<AgentSessionOutput>(
      this.zomeName,
      'register_elohim',
      input
    );
  }

  async updateAgentState(input: UpdateAgentStateInput): Promise<AgentOutput> {
    return this.connection.callZome<AgentOutput>(
      this.zomeName,
      'update_agent_state',
      input
    );
  }

  async getElohimByScope(scope: string): Promise<AgentOutput[]> {
    return this.connection.callZome<AgentOutput[]>(
      this.zomeName,
      'get_elohim_by_scope',
      scope
    );
  }

  // ==========================================================================
  // AgentProgress Operations
  // ==========================================================================

  async getOrCreateAgentProgress(
    input: CreateAgentProgressInput
  ): Promise<AgentProgressOutput> {
    return this.connection.callZome<AgentProgressOutput>(
      this.zomeName,
      'get_or_create_agent_progress',
      input
    );
  }

  async updateAgentProgress(
    input: UpdateAgentProgressInput
  ): Promise<AgentProgressOutput> {
    return this.connection.callZome<AgentProgressOutput>(
      this.zomeName,
      'update_agent_progress',
      input
    );
  }

  // ==========================================================================
  // ContentMastery Operations
  // ==========================================================================

  async upsertMastery(input: UpsertMasteryInput): Promise<ContentMasteryOutput> {
    return this.connection.callZome<ContentMasteryOutput>(
      this.zomeName,
      'upsert_mastery',
      input
    );
  }

  async getMastery(input: QueryMasteryInput): Promise<ContentMasteryOutput[]> {
    return this.connection.callZome<ContentMasteryOutput[]>(
      this.zomeName,
      'get_mastery',
      input
    );
  }

  /** Initialize mastery tracking for a content node */
  async initializeMastery(input: InitializeMasteryInput): Promise<ContentMasteryOutput> {
    return this.connection.callZome<ContentMasteryOutput>(
      this.zomeName,
      'initialize_mastery',
      input
    );
  }

  /** Get my mastery for a specific content node */
  async getMyMastery(contentId: string): Promise<ContentMasteryOutput | null> {
    return this.connection.callZome<ContentMasteryOutput | null>(
      this.zomeName,
      'get_my_mastery',
      contentId
    );
  }

  /** Get all my mastery records */
  async getMyAllMastery(): Promise<ContentMasteryOutput[]> {
    return this.connection.callZome<ContentMasteryOutput[]>(
      this.zomeName,
      'get_my_all_mastery',
      null
    );
  }

  /** Batch query mastery for multiple content IDs (Khan Academy missions view) */
  async getMasteryBatch(contentIds: string[]): Promise<MasterySnapshot[]> {
    return this.connection.callZome<MasterySnapshot[]>(
      this.zomeName,
      'get_mastery_batch',
      contentIds
    );
  }

  /** Get full path mastery overview (Khan Academy missions view) */
  async getPathMasteryOverview(pathId: string): Promise<PathMasteryOverview> {
    return this.connection.callZome<PathMasteryOverview>(
      this.zomeName,
      'get_path_mastery_overview',
      pathId
    );
  }

  /** Record engagement with content (view, practice, etc.) */
  async recordEngagement(input: RecordEngagementInput): Promise<ContentMasteryOutput> {
    return this.connection.callZome<ContentMasteryOutput>(
      this.zomeName,
      'record_engagement',
      input
    );
  }

  /** Record assessment result (quiz/test) with potential level up */
  async recordAssessment(input: RecordAssessmentInput): Promise<ContentMasteryOutput> {
    return this.connection.callZome<ContentMasteryOutput>(
      this.zomeName,
      'record_assessment',
      input
    );
  }

  /** Check if agent has privilege for a content node */
  async checkPrivilege(input: CheckPrivilegeInput): Promise<PrivilegeCheckResult> {
    return this.connection.callZome<PrivilegeCheckResult>(
      this.zomeName,
      'check_privilege',
      input
    );
  }

  /** Get my mastery statistics dashboard */
  async getMyMasteryStats(): Promise<MasteryStats> {
    return this.connection.callZome<MasteryStats>(
      this.zomeName,
      'get_my_mastery_stats',
      null
    );
  }

  /** Get all mastery records at a specific level */
  async getMasteryByLevel(level: string): Promise<ContentMasteryOutput[]> {
    return this.connection.callZome<ContentMasteryOutput[]>(
      this.zomeName,
      'get_mastery_by_level',
      level
    );
  }

  // ==========================================================================
  // Assessment History Operations
  // ==========================================================================

  /** Get assessment history for a content node */
  async getAssessmentHistory(contentId: string): Promise<AssessmentHistory> {
    return this.connection.callZome<AssessmentHistory>(
      this.zomeName,
      'get_assessment_history',
      contentId
    );
  }

  // ==========================================================================
  // Attestation Gating Operations
  // ==========================================================================

  /** Check if current agent is eligible for an attestation based on mastery requirements */
  async checkAttestationEligibility(
    input: CheckAttestationEligibilityInput
  ): Promise<AttestationEligibilityResult> {
    return this.connection.callZome<AttestationEligibilityResult>(
      this.zomeName,
      'check_attestation_eligibility',
      input
    );
  }

  /** Grant attestation only if mastery requirements are met */
  async grantAttestationWithMasteryCheck(
    input: GrantAttestationWithMasteryInput
  ): Promise<AgentProgressOutput> {
    return this.connection.callZome<AgentProgressOutput>(
      this.zomeName,
      'grant_attestation_with_mastery_check',
      input
    );
  }

  /** Check step access based on required attestation and mastery */
  async checkStepAccess(stepId: string): Promise<StepAccessResult> {
    return this.connection.callZome<StepAccessResult>(
      this.zomeName,
      'check_step_access',
      stepId
    );
  }

  /** Batch check multiple steps for access (efficient for path overview) */
  async checkPathStepAccess(pathId: string): Promise<StepAccessResult[]> {
    return this.connection.callZome<StepAccessResult[]>(
      this.zomeName,
      'check_path_step_access',
      pathId
    );
  }

  // ==========================================================================
  // Practice Pool Operations (Khan Academy-style organic learning)
  // ==========================================================================

  /** Get or create practice pool for current agent */
  async getOrCreatePracticePool(input: CreatePoolInput): Promise<PracticePoolOutput> {
    return this.connection.callZome<PracticePoolOutput>(
      this.zomeName,
      'get_or_create_practice_pool',
      input
    );
  }

  /** Refresh practice pool with content from paths and knowledge graph */
  async refreshPracticePool(): Promise<PracticePoolOutput> {
    return this.connection.callZome<PracticePoolOutput>(
      this.zomeName,
      'refresh_practice_pool',
      null
    );
  }

  /** Add a path to the practice pool */
  async addPathToPool(pathId: string): Promise<PracticePoolOutput> {
    return this.connection.callZome<PracticePoolOutput>(
      this.zomeName,
      'add_path_to_pool',
      pathId
    );
  }

  /** Get pool recommendations for what to practice */
  async getPoolRecommendations(): Promise<PoolRecommendations> {
    return this.connection.callZome<PoolRecommendations>(
      this.zomeName,
      'get_pool_recommendations',
      null
    );
  }

  /** Check if agent can take a mastery challenge (cooldown) */
  async checkChallengeCooldown(): Promise<CooldownCheckResult> {
    return this.connection.callZome<CooldownCheckResult>(
      this.zomeName,
      'check_challenge_cooldown',
      null
    );
  }

  // ==========================================================================
  // Mastery Challenge Operations
  // ==========================================================================

  /** Start a mastery challenge with mixed content from pool */
  async startMasteryChallenge(input: StartChallengeInput): Promise<MasteryChallengeOutput> {
    return this.connection.callZome<MasteryChallengeOutput>(
      this.zomeName,
      'start_mastery_challenge',
      input
    );
  }

  /** Submit challenge responses and apply level changes (up or down) */
  async submitMasteryChallenge(input: SubmitChallengeInput): Promise<ChallengeResult> {
    return this.connection.callZome<ChallengeResult>(
      this.zomeName,
      'submit_mastery_challenge',
      input
    );
  }

  /** Get challenge history for current agent */
  async getChallengeHistory(): Promise<MasteryChallengeOutput[]> {
    return this.connection.callZome<MasteryChallengeOutput[]>(
      this.zomeName,
      'get_challenge_history',
      null
    );
  }

  // ==========================================================================
  // Attestation Operations
  // ==========================================================================

  async createAttestation(
    input: CreateAttestationInput
  ): Promise<AttestationOutput> {
    return this.connection.callZome<AttestationOutput>(
      this.zomeName,
      'create_attestation',
      input
    );
  }

  async getAttestations(
    input: QueryAttestationsInput
  ): Promise<AttestationOutput[]> {
    return this.connection.callZome<AttestationOutput[]>(
      this.zomeName,
      'get_attestations',
      input
    );
  }

  // ==========================================================================
  // Shefa: Economic Event Operations
  // ==========================================================================

  async createEconomicEvent(
    input: CreateEconomicEventInput
  ): Promise<EconomicEventOutput> {
    return this.connection.callZome<EconomicEventOutput>(
      this.zomeName,
      'create_economic_event',
      input
    );
  }

  async getEconomicEventById(id: string): Promise<EconomicEventOutput | null> {
    return this.connection.callZome<EconomicEventOutput | null>(
      this.zomeName,
      'get_economic_event_by_id',
      id
    );
  }

  async getEventsByProvider(providerId: string): Promise<EconomicEventOutput[]> {
    return this.connection.callZome<EconomicEventOutput[]>(
      this.zomeName,
      'get_events_by_provider',
      providerId
    );
  }

  async getEventsByReceiver(receiverId: string): Promise<EconomicEventOutput[]> {
    return this.connection.callZome<EconomicEventOutput[]>(
      this.zomeName,
      'get_events_by_receiver',
      receiverId
    );
  }

  async getEventsByAction(action: string): Promise<EconomicEventOutput[]> {
    return this.connection.callZome<EconomicEventOutput[]>(
      this.zomeName,
      'get_events_by_action',
      action
    );
  }

  async getEventsByLamadType(lamadEventType: string): Promise<EconomicEventOutput[]> {
    return this.connection.callZome<EconomicEventOutput[]>(
      this.zomeName,
      'get_events_by_lamad_type',
      lamadEventType
    );
  }

  // ==========================================================================
  // Shefa: Economic Resource Operations
  // ==========================================================================

  async createEconomicResource(
    input: CreateEconomicResourceInput
  ): Promise<EconomicResourceOutput> {
    return this.connection.callZome<EconomicResourceOutput>(
      this.zomeName,
      'create_economic_resource',
      input
    );
  }

  async getEconomicResourceById(id: string): Promise<EconomicResourceOutput | null> {
    return this.connection.callZome<EconomicResourceOutput | null>(
      this.zomeName,
      'get_economic_resource_by_id',
      id
    );
  }

  async getResourcesByOwner(ownerId: string): Promise<EconomicResourceOutput[]> {
    return this.connection.callZome<EconomicResourceOutput[]>(
      this.zomeName,
      'get_resources_by_owner',
      ownerId
    );
  }

  async getResourcesBySpec(specId: string): Promise<EconomicResourceOutput[]> {
    return this.connection.callZome<EconomicResourceOutput[]>(
      this.zomeName,
      'get_resources_by_spec',
      specId
    );
  }

  // ==========================================================================
  // Shefa: Contributor Presence Operations
  // ==========================================================================

  async createContributorPresence(
    input: CreateContributorPresenceInput
  ): Promise<ContributorPresenceOutput> {
    return this.connection.callZome<ContributorPresenceOutput>(
      this.zomeName,
      'create_contributor_presence',
      input
    );
  }

  async getContributorPresenceById(id: string): Promise<ContributorPresenceOutput | null> {
    return this.connection.callZome<ContributorPresenceOutput | null>(
      this.zomeName,
      'get_contributor_presence_by_id',
      id
    );
  }

  async queryContributorPresences(
    input: QueryPresencesInput
  ): Promise<ContributorPresenceOutput[]> {
    return this.connection.callZome<ContributorPresenceOutput[]>(
      this.zomeName,
      'query_contributor_presences',
      input
    );
  }

  async getPresencesBySteward(stewardId: string): Promise<ContributorPresenceOutput[]> {
    return this.connection.callZome<ContributorPresenceOutput[]>(
      this.zomeName,
      'get_presences_by_steward',
      stewardId
    );
  }

  async getPresencesByState(presenceState: string): Promise<ContributorPresenceOutput[]> {
    return this.connection.callZome<ContributorPresenceOutput[]>(
      this.zomeName,
      'get_presences_by_state',
      presenceState
    );
  }

  // Stewardship Lifecycle Operations

  async beginStewardship(
    input: BeginStewardshipInput
  ): Promise<ContributorPresenceOutput> {
    return this.connection.callZome<ContributorPresenceOutput>(
      this.zomeName,
      'begin_stewardship',
      input
    );
  }

  async initiateClaim(input: InitiateClaimInput): Promise<ContributorPresenceOutput> {
    return this.connection.callZome<ContributorPresenceOutput>(
      this.zomeName,
      'initiate_claim',
      input
    );
  }

  async verifyClaim(presenceId: string): Promise<ContributorPresenceOutput> {
    return this.connection.callZome<ContributorPresenceOutput>(
      this.zomeName,
      'verify_claim',
      presenceId
    );
  }

  // ==========================================================================
  // Shefa: Process Operations
  // ==========================================================================

  async createProcess(input: CreateProcessInput): Promise<ProcessOutput> {
    return this.connection.callZome<ProcessOutput>(
      this.zomeName,
      'create_process',
      input
    );
  }

  async getProcessById(id: string): Promise<ProcessOutput | null> {
    return this.connection.callZome<ProcessOutput | null>(
      this.zomeName,
      'get_process_by_id',
      id
    );
  }

  async getProcessesByPath(pathId: string): Promise<ProcessOutput[]> {
    return this.connection.callZome<ProcessOutput[]>(
      this.zomeName,
      'get_processes_by_path',
      pathId
    );
  }

  async getProcessesByPerformer(performerId: string): Promise<ProcessOutput[]> {
    return this.connection.callZome<ProcessOutput[]>(
      this.zomeName,
      'get_processes_by_performer',
      performerId
    );
  }

  // ==========================================================================
  // Shefa: Intent Operations
  // ==========================================================================

  async createIntent(input: CreateIntentInput): Promise<IntentOutput> {
    return this.connection.callZome<IntentOutput>(
      this.zomeName,
      'create_intent',
      input
    );
  }

  async getIntentById(id: string): Promise<IntentOutput | null> {
    return this.connection.callZome<IntentOutput | null>(
      this.zomeName,
      'get_intent_by_id',
      id
    );
  }

  async getIntentsByProvider(providerId: string): Promise<IntentOutput[]> {
    return this.connection.callZome<IntentOutput[]>(
      this.zomeName,
      'get_intents_by_provider',
      providerId
    );
  }

  async getIntentsByReceiver(receiverId: string): Promise<IntentOutput[]> {
    return this.connection.callZome<IntentOutput[]>(
      this.zomeName,
      'get_intents_by_receiver',
      receiverId
    );
  }

  // ==========================================================================
  // Shefa: Commitment Operations
  // ==========================================================================

  async createCommitment(input: CreateCommitmentInput): Promise<CommitmentOutput> {
    return this.connection.callZome<CommitmentOutput>(
      this.zomeName,
      'create_commitment',
      input
    );
  }

  async getCommitmentById(id: string): Promise<CommitmentOutput | null> {
    return this.connection.callZome<CommitmentOutput | null>(
      this.zomeName,
      'get_commitment_by_id',
      id
    );
  }

  async getCommitmentsByProvider(providerId: string): Promise<CommitmentOutput[]> {
    return this.connection.callZome<CommitmentOutput[]>(
      this.zomeName,
      'get_commitments_by_provider',
      providerId
    );
  }

  async getCommitmentsByReceiver(receiverId: string): Promise<CommitmentOutput[]> {
    return this.connection.callZome<CommitmentOutput[]>(
      this.zomeName,
      'get_commitments_by_receiver',
      receiverId
    );
  }

  async getCommitmentsByState(state: string): Promise<CommitmentOutput[]> {
    return this.connection.callZome<CommitmentOutput[]>(
      this.zomeName,
      'get_commitments_by_state',
      state
    );
  }

  // ==========================================================================
  // Shefa: Appreciation Operations
  // ==========================================================================

  async createAppreciation(
    input: CreateAppreciationInput
  ): Promise<AppreciationOutput> {
    return this.connection.callZome<AppreciationOutput>(
      this.zomeName,
      'create_appreciation',
      input
    );
  }

  async getAppreciationById(id: string): Promise<AppreciationOutput | null> {
    return this.connection.callZome<AppreciationOutput | null>(
      this.zomeName,
      'get_appreciation_by_id',
      id
    );
  }

  async getAppreciationsByGiver(giverId: string): Promise<AppreciationOutput[]> {
    return this.connection.callZome<AppreciationOutput[]>(
      this.zomeName,
      'get_appreciations_by_giver',
      giverId
    );
  }

  async getAppreciationsByReceiver(receiverId: string): Promise<AppreciationOutput[]> {
    return this.connection.callZome<AppreciationOutput[]>(
      this.zomeName,
      'get_appreciations_by_receiver',
      receiverId
    );
  }

  // ==========================================================================
  // Lamad: Learning Economy (uses Shefa hREA primitives)
  // ==========================================================================
  // Learning-specific economic operations that compose Shefa's hREA substrate.
  // Points → EconomicEvents, Balances → EconomicResources,
  // Recognition → Appreciation flowing to ContributorPresence.

  /**
   * Earn learning points for an activity.
   * Creates a LamadPointEvent (learning-specific EconomicEvent)
   * and triggers recognition flow to contributors via Shefa ContributorPresence.
   */
  async earnLamadPoints(input: EarnLamadPointsInput): Promise<EarnLamadPointsResult> {
    return this.connection.callZome<EarnLamadPointsResult>(
      this.zomeName,
      'earn_points',
      input
    );
  }

  /** Get my current learning point balance */
  async getMyLamadPointBalance(): Promise<LearnerPointBalanceOutput | null> {
    return this.connection.callZome<LearnerPointBalanceOutput | null>(
      this.zomeName,
      'get_my_point_balance',
      null
    );
  }

  /** Get my learning point event history */
  async getMyLamadPointHistory(limit?: number): Promise<LamadPointEventOutput[]> {
    return this.connection.callZome<LamadPointEventOutput[]>(
      this.zomeName,
      'get_my_point_history',
      limit ?? 50
    );
  }

  /** Get learning point balance for any agent */
  async getLamadPointBalance(agentId: string): Promise<LearnerPointBalanceOutput | null> {
    return this.connection.callZome<LearnerPointBalanceOutput | null>(
      this.zomeName,
      'get_point_balance',
      agentId
    );
  }

  /** Get learning point events for specific content */
  async getLamadPointsByContent(contentId: string): Promise<LamadPointEventOutput[]> {
    return this.connection.callZome<LamadPointEventOutput[]>(
      this.zomeName,
      'get_points_by_content',
      contentId
    );
  }

  // --------------------------------------------------------------------------
  // Lamad: Learning Economy Aggregations
  // --------------------------------------------------------------------------
  // Domain-specific aggregation views over Shefa hREA flows.
  // Other domains would create their own aggregations (SellerDashboard, etc.)

  /**
   * Get the learning contributor dashboard.
   * Aggregates hREA value flows to show impact of a contributor's content.
   */
  async getLamadContributorDashboard(contributorId: string): Promise<LamadContributorDashboard> {
    return this.connection.callZome<LamadContributorDashboard>(
      this.zomeName,
      'get_contributor_dashboard',
      contributorId
    );
  }

  /** Get my learning contributor dashboard (for current agent) */
  async getMyLamadContributorDashboard(): Promise<LamadContributorDashboard> {
    return this.connection.callZome<LamadContributorDashboard>(
      this.zomeName,
      'get_my_contributor_dashboard',
      null
    );
  }

  /** Get learning recognition events received by a contributor */
  async getLamadRecognitionByContributor(contributorId: string): Promise<LamadContributorRecognitionOutput[]> {
    return this.connection.callZome<LamadContributorRecognitionOutput[]>(
      this.zomeName,
      'get_recognition_by_contributor',
      contributorId
    );
  }

  /** Get contributor impact summary for learning */
  async getLamadContributorImpact(contributorId: string): Promise<LamadContributorImpactOutput | null> {
    return this.connection.callZome<LamadContributorImpactOutput | null>(
      this.zomeName,
      'get_contributor_impact',
      contributorId
    );
  }

  // ==========================================================================
  // Shefa: Claim & Settlement Operations
  // ==========================================================================

  async createClaim(input: CreateClaimInput): Promise<ClaimOutput> {
    return this.connection.callZome<ClaimOutput>(
      this.zomeName,
      'create_claim',
      input
    );
  }

  async getClaimById(id: string): Promise<ClaimOutput | null> {
    return this.connection.callZome<ClaimOutput | null>(
      this.zomeName,
      'get_claim_by_id',
      id
    );
  }

  async getClaimsByProvider(providerId: string): Promise<ClaimOutput[]> {
    return this.connection.callZome<ClaimOutput[]>(
      this.zomeName,
      'get_claims_by_provider',
      providerId
    );
  }

  async getClaimsByReceiver(receiverId: string): Promise<ClaimOutput[]> {
    return this.connection.callZome<ClaimOutput[]>(
      this.zomeName,
      'get_claims_by_receiver',
      receiverId
    );
  }

  async createSettlement(input: CreateSettlementInput): Promise<SettlementOutput> {
    return this.connection.callZome<SettlementOutput>(
      this.zomeName,
      'create_settlement',
      input
    );
  }

  async getSettlementById(id: string): Promise<SettlementOutput | null> {
    return this.connection.callZome<SettlementOutput | null>(
      this.zomeName,
      'get_settlement_by_id',
      id
    );
  }

  async getSettlementsByClaim(claimId: string): Promise<SettlementOutput[]> {
    return this.connection.callZome<SettlementOutput[]>(
      this.zomeName,
      'get_settlements_by_claim',
      claimId
    );
  }

  // ==========================================================================
  // Lamad: Steward Economy Operations
  // ==========================================================================

  /** Create a steward credential */
  async createStewardCredential(input: CreateStewardCredentialInput): Promise<StewardCredentialOutput> {
    return this.connection.callZome<StewardCredentialOutput>(
      this.zomeName,
      'create_steward_credential',
      input
    );
  }

  /** Get steward credential by ID */
  async getStewardCredential(credentialId: string): Promise<StewardCredentialOutput | null> {
    return this.connection.callZome<StewardCredentialOutput | null>(
      this.zomeName,
      'get_steward_credential',
      credentialId
    );
  }

  /** Get credentials for a human/steward presence */
  async getCredentialsForHuman(humanPresenceId: string): Promise<StewardCredentialOutput[]> {
    return this.connection.callZome<StewardCredentialOutput[]>(
      this.zomeName,
      'get_credentials_for_human',
      humanPresenceId
    );
  }

  /** Create a premium gate for content */
  async createPremiumGate(input: CreatePremiumGateInput): Promise<PremiumGateOutput> {
    return this.connection.callZome<PremiumGateOutput>(
      this.zomeName,
      'create_premium_gate',
      input
    );
  }

  /** Get premium gate by ID */
  async getPremiumGate(gateId: string): Promise<PremiumGateOutput | null> {
    return this.connection.callZome<PremiumGateOutput | null>(
      this.zomeName,
      'get_premium_gate',
      gateId
    );
  }

  /** Get gates for a resource */
  async getGatesForResource(resourceId: string): Promise<PremiumGateOutput[]> {
    return this.connection.callZome<PremiumGateOutput[]>(
      this.zomeName,
      'get_gates_for_resource',
      resourceId
    );
  }

  /** Grant access through a gate */
  async grantAccess(input: GrantAccessInput): Promise<AccessGrantOutput> {
    return this.connection.callZome<AccessGrantOutput>(
      this.zomeName,
      'grant_access',
      input
    );
  }

  /** Check if current agent has access to a gate */
  async checkAccess(gateId: string): Promise<AccessGrantOutput | null> {
    return this.connection.callZome<AccessGrantOutput | null>(
      this.zomeName,
      'check_access',
      gateId
    );
  }

  /** Get my access grants */
  async getMyAccessGrants(): Promise<AccessGrantOutput[]> {
    return this.connection.callZome<AccessGrantOutput[]>(
      this.zomeName,
      'get_my_access_grants',
      null
    );
  }

  /** Get steward revenue summary */
  async getStewardRevenueSummary(stewardPresenceId: string): Promise<StewardRevenueSummary> {
    return this.connection.callZome<StewardRevenueSummary>(
      this.zomeName,
      'get_steward_revenue_summary',
      stewardPresenceId
    );
  }
}
