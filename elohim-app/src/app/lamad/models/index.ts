/**
 * Barrel export for Lamad models
 *
 * Lamad is the Content/Learning pillar of the Elohim Protocol.
 * This barrel re-exports types from other pillars for backward compatibility.
 *
 * Pillar Structure:
 * - elohim/: Protocol-core (shared primitives, agents, attestations)
 * - imagodei/: Identity (profile, session, human nodes)
 * - lamad/: Content (nodes, paths, exploration, mastery)
 * - qahal/: Community (consent, governance, affinity, places)
 * - shefa/: Economy (REA, economic events, contributor presence)
 *
 * Six-layer architecture:
 * - Territory: ContentNode (the knowledge graph)
 * - Journey: LearningPath, PathStep (curated sequences)
 * - Traveler: Agent, AgentProgress (human state)
 * - Maps: KnowledgeMap (polymorphic: domain, self, person, collective)
 * - Economic: REA value flows and contributor recognition
 * - Governance: Constitutional moderation, deliberation, feedback profiles
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
// RE-EXPORTS FROM ELOHIM (Protocol-Core)
// TODO: Remove these re-exports. Import directly from @app/elohim/models/*
// ============================================================================

/** @deprecated Import from '@app/elohim/models/protocol-core.model' */
export * from '@app/elohim/models/protocol-core.model';

/** @deprecated Import from '@app/elohim/models/rea-bridge.model' */
export * from '@app/elohim/models/rea-bridge.model';

/** @deprecated Import from '@app/elohim/models/agent.model' */
export * from '@app/elohim/models/agent.model';

/** @deprecated Import from '@app/elohim/models/elohim-agent.model' */
export * from '@app/elohim/models/elohim-agent.model';

/** @deprecated Import from '@app/elohim/models/trust-badge.model' */
export type {
  TrustBadge,
  BadgeDisplay,
  BadgeType,
  BadgeColor,
  TrustLevel,
  BadgeWarning,
  TrustIndicator,
  IndicatorSource,
  TrustIndicatorSet,
  BadgeAction,
  CompactTrustBadge
} from '@app/elohim/models/trust-badge.model';
/** @deprecated Import from '@app/elohim/models/trust-badge.model' */
export {
  badgeToIndicator,
  warningToIndicator,
  ATTESTATION_PRIORITY,
  ATTESTATION_BADGE_CONFIG,
  REACH_BADGE_CONFIG,
  WARNING_CONFIG,
  calculateTrustLevel,
  generateTrustSummary,
  generateAriaLabel,
  toCompactBadge
} from '@app/elohim/models/trust-badge.model';

/** @deprecated Import from '@app/elohim/models/json-ld.model' */
export * from '@app/elohim/models/json-ld.model';
/** @deprecated Import from '@app/elohim/models/open-graph.model' */
export * from '@app/elohim/models/open-graph.model';
/** @deprecated Import from '@app/elohim/models/verifiable-credential.model' */
export * from '@app/elohim/models/verifiable-credential.model';
/** @deprecated Import from '@app/elohim/models/source-chain.model' */
export * from '@app/elohim/models/source-chain.model';

// ============================================================================
// RE-EXPORTS FROM IMAGODEI (Identity)
// TODO: Remove these re-exports. Import directly from @app/imagodei/models/*
// ============================================================================

/** @deprecated Import from '@app/imagodei/models/session-human.model' */
export * from '@app/imagodei/models/session-human.model';

/** @deprecated Import from '@app/imagodei/models/profile.model' */
export * from '@app/imagodei/models/profile.model';

/** @deprecated Import from '@app/imagodei/models/attestations.model' */
export type {
  Attestation as ImagodeiAttestation,
  AttestationJourney,
  Endorsement,
  AttestationRequirement as ImagodeiAttestationRequirement,
  AttestationAccessRequirement,
  UserAttestations,
  AttestationProgress
} from '@app/imagodei/models/attestations.model';

// ============================================================================
// RE-EXPORTS FROM ELOHIM (Economy - canonical location)
// TODO: Remove these re-exports. Import directly from @app/elohim/models/*
// ============================================================================

/** @deprecated Import from '@app/elohim/models/economic-event.model' */
export * from '@app/elohim/models/economic-event.model';

/** @deprecated Import from '@app/elohim/models/contributor-presence.model' */
export * from '@app/elohim/models/contributor-presence.model';

// Steward Economy models - LAMAD-NATIVE (stay here)
export * from './steward-economy.model';

// ============================================================================
// RE-EXPORTS FROM QAHAL (Community)
// TODO: Remove these re-exports. Import directly from @app/qahal/models/*
// ============================================================================

/** @deprecated Import from '@app/qahal/models/human-affinity.model' */
export * from '@app/qahal/models/human-affinity.model';

/** @deprecated Import from '@app/elohim/models/human-consent.model' */
export type {
  HumanConsent,
  ConsentRequest
} from '@app/elohim/models/human-consent.model';

/** @deprecated Import from '@app/qahal/models/governance-feedback.model' */
export * from '@app/qahal/models/governance-feedback.model';

/** @deprecated Import from '@app/qahal/models/governance-deliberation.model' */
export * from '@app/qahal/models/governance-deliberation.model';

/** @deprecated Import from '@app/qahal/models/place.model' */
export type {
  Place,
  PlaceType,
  PlaceTypeCategory,
  PlaceNameType,
  PlaceNameDisputeStatus,
  PlaceName,
  BoundaryType,
  GeoJSONGeometry,
  PlaceGeography,
  EcologicalRelationshipType,
  EcologicalRelationship,
  CulturalContext,
  GeographicReach,
  GeographicDeterminationMethod,
  GeographicDetermination,
  EcologicalLimitType,
  EcologicalLimit,
  BioregionalAuthority,
  PlaceCapability,
  PlaceAwareElohim,
  PlaceHierarchy,
  PlaceHierarchyNode,
  PlaceServiceInterface
} from '@app/qahal/models/place.model';
/** @deprecated Import from '@app/qahal/models/place.model' */
export { PLACE_TYPE_CATEGORIES } from '@app/qahal/models/place.model';
