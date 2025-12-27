/**
 * Barrel export for Lamad models
 *
 * Lamad is the Content/Learning pillar of the Elohim Protocol.
 *
 * Architecture:
 * - Territory: ContentNode (the knowledge graph)
 * - Journey: LearningPath, PathStep (curated sequences)
 * - Traveler: Agent, AgentProgress (human state)
 * - Maps: KnowledgeMap (polymorphic: domain, self, person, collective)
 *
 * For models from other pillars, import directly:
 * - @app/elohim/models/* - Protocol-core (agents, attestations, trust)
 * - @app/imagodei/models/* - Identity (profile, session)
 * - @app/qahal/models/* - Community (consent, governance, affinity)
 * - @app/shefa/models/* - Economy (REA, economic events)
 */

// ============================================================================
// LAMAD-SPECIFIC MODELS (Content/Learning)
// ============================================================================

// Territory models (Content Graph)
export type {
  ContentNode,
  ContentMetadata,
  ContentType,
  ContentFormat,
  ContentRelationship,
  ContentGraph,
  ContentGraphMetadata,
  ContentGraphMetadata as GraphMetadata,
  ContentReach,
  ContentFlag
} from './content-node.model';
export { ContentRelationshipType } from './content-node.model';

// Content Attestation models (Trust credentials granted TO content)
export * from './content-attestation.model';

// Journey models (Learning Path)
export * from './learning-path.model';

// Knowledge Map models (Polymorphic: Domain, Person, Collective)
// Export selectively to avoid RelationshipType collision
export type {
  KnowledgeMapType,
  MapSubject,
  KnowledgeMap,
  KnowledgeNode,
  KnowledgeSource,
  DomainKnowledgeMap,
  DomainGoal,
  SelfKnowledgeMap,
  ImagoDeiDimension,
  SelfInsight,
  InsightSource,
  PersonalValue,
  LifeChapter,
  LifeEvent,
  DiscoveredGift,
  GiftCategory,
  ShadowArea,
  VocationalClarity,
  DomainReflection,
  AssessmentInstrument,
  InstrumentContributor,
  ContributorRole,
  InstrumentDerivation,
  InstrumentRecognitionModel,
  AssessmentPathStructure,
  AssessmentStep,
  InstrumentSection,
  AssessmentQuestion,
  QuestionType,
  QuestionOption,
  AssessmentAttestation,
  AssessmentAttestationType,
  AssessmentAttestationRequirement,
  AssessmentPrerequisite,
  AssessmentDomain,
  InstrumentValidation,
  InstrumentLicense,
  AssessmentResult,
  AssessmentInterpretation,
  SubscaleResult,
  LongitudinalChange,
  SelfKnowledgeLink,
  ResponseQuality,
  PatternAlert,
  PatternType,
  PatternEvidence,
  SuggestedAction,
  PatternResponse,
  ResearchConsent,
  ResearchConsentScope,
  ContributionRecognition,
  CrisisProtocol,
  CrisisResource,
  PersonKnowledgeMap,
  SubjectConsent,
  ConsentScope,
  PersonKnowledgeCategory,
  PersonKnowledgeCategoryType,
  RelationshipMetrics,
  CollectiveKnowledgeMap,
  CollectiveMember,
  CollectiveGovernance,
  CollectiveDomain,
  KnowledgeMapIndexEntry,
  KnowledgeMapIndex,
  KnowledgeMapUpdate,
  MapMergeRequest
} from './knowledge-map.model';
// Export RelationshipType with alias to avoid collision
export type { RelationshipType as PersonRelationshipType } from './knowledge-map.model';

// Path Extension models (Learner customization)
export * from './path-extension.model';

// Exploration models (Graph traversal and discovery)
export * from './exploration.model';

// Search models (enhanced search with scoring and facets)
export * from './search.model';

// Content Access models (visitor/member/attested tiers)
export * from './content-access.model';

// Content Mastery models (Bloom's Taxonomy progression)
export * from './content-mastery.model';

// Practice Pool models (Khan Academy-style organic learning)
export * from './practice.model';

// Learning Points models (Shefa economic integration)
export * from './learning-points.model';

// Mastery Visualization (colors, icons, labels for UI)
export * from './mastery-visualization';

// Content Lifecycle models (refresh policies, deprecation, archival)
export * from './content-lifecycle.model';

// Feedback Profile models (virality as privilege, emotional reaction constraints)
export * from './feedback-profile.model';

// Path Negotiation models (collaborative path creation)
export * from './path-negotiation.model';

// Human Node models (humans in the graph - stays in lamad for graph integration)
// Export selectively to avoid collisions
export type {
  HumanNode,
  HumanReach,
  HumanRelationship
} from './human-node.model';
export type { RelationshipType as HumanRelationshipType } from './human-node.model';
export {
  RELATIONSHIP_LAYER_MAP,
  RELATIONSHIP_DEFAULT_INTIMACY,
  getRelationshipLayer,
  getDefaultIntimacy,
  isFamilyRelationship,
  isWorkplaceRelationship,
  isHighTrustRelationship
} from './human-node.model';

// ============================================================================
// LAMAD STEWARD ECONOMY MODELS
// ============================================================================

// Steward Economy models - LAMAD-NATIVE
export * from './steward-economy.model';
