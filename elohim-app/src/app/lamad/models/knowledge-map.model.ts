/**
 * Knowledge Map Models - Polymorphic containers for learnable territory.
 *
 * Four Relational Dimensions:
 * 1. Domain Maps - Relationship with knowledge (what do I know?)
 *    Inspired by Khan Academy's "World of Math"
 *
 * 2. Self Maps - Relationship with self (who am I?) - "γνῶθι σεαυτόν"
 *    Inspired by Delphic maxim, Imago Dei framework, Socratic examination
 *
 * 3. Person Maps - Relationship with others (who do I know?)
 *    Inspired by Gottman's Love Maps research
 *
 * 4. Collective Maps - Relationship with communities (what do we know?)
 *    Inspired by organizational knowledge management
 *
 * The key insight: learning is fundamentally about building relationship -
 * with ideas, with self, with people, with communities.
 * The same navigation/affinity mechanics apply to all four.
 *
 * Theological grounding: "Love your neighbor as yourself" (Mark 12:31)
 * implies three loves: God, neighbor, and self. Self-knowledge is prerequisite.
 *
 * Holochain mapping:
 * - Entry type: "knowledge_map"
 * - Links to subject (content graph, agent, or organization)
 * - Private maps on source chain, shared maps on DHT
 */

/**
 * KnowledgeMapType - The four flavors of knowledge territory.
 *
 * Three relational dimensions + one self-reflective:
 * - domain: Relationship with ideas/knowledge (what do I know?)
 * - person: Relationship with others (who do I know?)
 * - self: Relationship with self (who am I?) - "know thyself"
 * - collective: Relationship with communities (what do we know?)
 */
export type KnowledgeMapType = 'domain' | 'person' | 'self' | 'collective';

/**
 * MapSubject - What is being mapped (the territory).
 */
export interface MapSubject {
  /** Type of subject being mapped */
  type: 'content-graph' | 'agent' | 'organization';

  /** Identifier of the subject */
  subjectId: string;

  /** Human-readable name of the subject */
  subjectName: string;
}

/**
 * KnowledgeMap - Base interface for all map types.
 *
 * A knowledge map is a personalized view of a learnable territory.
 * Unlike paths (which are curator-defined journeys), maps represent
 * the learner's own understanding and relationship with a subject.
 */
export interface KnowledgeMap {
  /** Unique identifier */
  id: string;

  /** Type discriminator for polymorphism */
  mapType: KnowledgeMapType;

  /** The subject being mapped */
  subject: MapSubject;

  /** Who created/owns this map */
  ownerId: string;

  /** Display title for this map */
  title: string;

  /** Description of what this map represents */
  description?: string;

  /**
   * Visibility controls who can see this map:
   * - 'private': Only the owner
   * - 'mutual': Owner and subject (for person maps)
   * - 'shared': Specific agents granted access
   * - 'public': Anyone can view
   */
  visibility: 'private' | 'mutual' | 'shared' | 'public';

  /** Agents granted access (when visibility is 'shared') */
  sharedWith?: string[];

  /** Knowledge nodes in this map */
  nodes: KnowledgeNode[];

  /** Paths through this map's territory */
  pathIds: string[];

  /** Overall affinity/familiarity score (0.0 - 1.0) */
  overallAffinity: number;

  /** Timestamps */
  createdAt: string;
  updatedAt: string;

  /** Optional metadata */
  metadata?: Record<string, unknown>;
}

/**
 * KnowledgeNode - A single piece of knowledge in a map.
 *
 * Unlike ContentNode (which is shared territory), KnowledgeNode
 * represents personal/relational knowledge that may be private.
 */
export interface KnowledgeNode {
  /** Unique identifier within the map */
  id: string;

  /** Category this knowledge belongs to */
  category: string;

  /** The knowledge content */
  title: string;
  content: string;

  /** Source of this knowledge */
  source?: KnowledgeSource;

  /** Affinity/confidence in this knowledge (0.0 - 1.0) */
  affinity: number;

  /** When was this last verified/updated? */
  lastVerified?: string;

  /** Related nodes within the same map */
  relatedNodeIds: string[];

  /** Tags for organization */
  tags: string[];

  /** Custom metadata */
  metadata?: Record<string, unknown>;
}

/**
 * KnowledgeSource - Where knowledge came from.
 */
export interface KnowledgeSource {
  type: 'direct-observation' | 'conversation' | 'shared-content' | 'inference' | 'external';
  sourceId?: string;
  timestamp: string;
  confidence: number;
}

// ============================================================================
// Domain Knowledge Map (Khan Academy / Elohim Protocol style)
// ============================================================================

/**
 * DomainKnowledgeMap - Knowledge map over a content graph.
 *
 * This is what we've been building: a learner's relationship with
 * a structured body of knowledge like "The Elohim Protocol" or
 * "World of Math".
 */
export interface DomainKnowledgeMap extends KnowledgeMap {
  mapType: 'domain';

  subject: {
    type: 'content-graph';
    subjectId: string;  // ID of the root content node or graph
    subjectName: string;
  };

  /** The content graph being mapped */
  contentGraphId: string;

  /** Mastery levels per content node */
  masteryLevels: Map<string, MasteryLevel>;

  /** Learning goals within this domain */
  goals?: DomainGoal[];
}

// MasteryLevel is imported from agent.model.ts to avoid duplication
// Re-export for convenience within this file
import type { MasteryLevel } from './agent.model';
export type { MasteryLevel };

export interface DomainGoal {
  id: string;
  title: string;
  targetNodes: string[];
  targetMastery: MasteryLevel;
  deadline?: string;
  completed: boolean;
}

// ============================================================================
// Self Knowledge Map ("Know Thyself" / γνῶθι σεαυτόν)
// ============================================================================

/**
 * SelfKnowledgeMap - The most intimate map: knowledge of oneself.
 *
 * Inspired by:
 * - Delphic maxim "γνῶθι σεαυτόν" (know thyself)
 * - Imago Dei framework (discovering the divine image within)
 * - Socratic self-examination
 * - Modern psychological self-awareness frameworks
 *
 * This is fundamentally different from person maps:
 * - Subject is the mapper themselves (reflexive)
 * - Always private unless explicitly shared
 * - Integrates with HumanProfile (imagodei-* dimensions)
 * - Enables personal growth tracking and self-discovery
 *
 * Theological grounding: To love others as yourself (Mark 12:31),
 * you must first know yourself. Self-knowledge is prerequisite to love.
 */
export interface SelfKnowledgeMap extends KnowledgeMap {
  mapType: 'self';

  subject: {
    type: 'agent';
    subjectId: string;  // Same as ownerId - maps self
    subjectName: string;
  };

  /** Imago Dei dimensions - core identity facets */
  imagoDeiDimensions: ImagoDeiDimension[];

  /** Personal values hierarchy (what matters most) */
  valuesHierarchy: PersonalValue[];

  /** Life chapters (narrative structure of one's journey) */
  lifeChapters: LifeChapter[];

  /** Discovered gifts/strengths (imagodei-gifts) */
  discoveredGifts: DiscoveredGift[];

  /** Shadow work (areas of growth, blind spots) */
  shadowAreas: ShadowArea[];

  /** Vocational clarity (calling, purpose) */
  vocation?: VocationalClarity;

  /** Integration with domain learning (what subjects reveal about self) */
  domainReflections: DomainReflection[];

  /** Psychometric assessment history (validated self-knowledge) */
  assessmentHistory: AssessmentResult[];

  /** Pattern alerts detected by Elohim from assessment data */
  patternAlerts?: PatternAlert[];

  /** Research contribution consent */
  researchConsent?: ResearchConsent;
}

/**
 * ImagoDeiDimension - Facets of the divine image being uncovered.
 *
 * Maps to the imagodei-* architecture:
 * - Core: Stable identity, the "I am"
 * - Experience: How life has shaped me
 * - Gifts: What I've been given/developed
 * - Synthesis: How I make meaning
 */
export interface ImagoDeiDimension {
  id: string;
  dimension: 'core' | 'experience' | 'gifts' | 'synthesis';
  title: string;
  description: string;
  insights: SelfInsight[];
  affinity: number;  // How well do I know this dimension?
  lastExplored: string;
}

export interface SelfInsight {
  id: string;
  content: string;
  source: InsightSource;
  timestamp: string;
  significance: 'revelation' | 'confirmation' | 'question';
}

export type InsightSource =
  | 'reflection'           // Direct self-examination
  | 'learning-path'        // Emerged from domain learning
  | 'relationship'         // Revealed in relationship with others
  | 'trial'                // Discovered through difficulty
  | 'celebration'          // Revealed in joy/success
  | 'feedback'             // Others' perspective
  | 'spiritual'            // Prayer, meditation, worship
  | 'assessment';          // Psychometrically validated instrument

/**
 * PersonalValue - Core values in priority order.
 */
export interface PersonalValue {
  id: string;
  name: string;
  description: string;
  rank: number;  // Priority order (1 = highest)
  evidencedBy: string[];  // Life examples where this value appeared
  inTensionWith?: string[];  // Other values that create tension
}

/**
 * LifeChapter - Narrative structure of personal history.
 *
 * Life as story, not just events. Supports meaning-making.
 */
export interface LifeChapter {
  id: string;
  title: string;  // "The Wilderness Years", "Finding My Voice"
  timespan: {
    start: string;
    end?: string;  // Undefined = current chapter
  };
  theme: string;
  keyEvents: LifeEvent[];
  lessonsLearned: string[];
  characterDevelopment: string;  // How did I grow?
}

export interface LifeEvent {
  id: string;
  title: string;
  date: string;
  type: 'formative' | 'transformative' | 'milestone' | 'ordinary';
  description: string;
  impact: string;  // How this shaped me
}

/**
 * DiscoveredGift - Strengths/gifts uncovered through self-examination.
 */
export interface DiscoveredGift {
  id: string;
  name: string;
  category: GiftCategory;
  description: string;
  evidencedBy: string[];  // Where this gift has manifested
  developmentLevel: 'latent' | 'emerging' | 'practiced' | 'mastered';
  calledTo?: string;  // How this gift might be used
}

export type GiftCategory =
  | 'intellectual'     // Analytical, creative thinking
  | 'relational'       // Connection, empathy, leadership
  | 'practical'        // Skills, craftsmanship
  | 'artistic'         // Creative expression
  | 'spiritual'        // Faith, wisdom, discernment
  | 'physical';        // Body, health, athleticism

/**
 * ShadowArea - Areas for growth, blind spots, struggles.
 *
 * Not for shame, but for honest self-awareness and growth.
 * "The cave you fear to enter holds the treasure you seek." - Joseph Campbell
 */
export interface ShadowArea {
  id: string;
  area: string;
  awareness: 'blind-spot' | 'acknowledged' | 'working-on' | 'integrated';
  triggers?: string[];
  growthPath?: string;
  supportNeeded?: string;
}

/**
 * VocationalClarity - Understanding of one's calling/purpose.
 */
export interface VocationalClarity {
  /** Core sense of purpose */
  missionStatement?: string;

  /** What problems am I called to solve? */
  problemsToSolve: string[];

  /** Who am I called to serve? */
  peopleToServe: string[];

  /** How do my gifts align with needs? */
  giftAlignment: Array<{
    giftId: string;
    need: string;
    fit: number;  // 0.0 - 1.0
  }>;

  /** Current clarity level */
  clarityLevel: 'searching' | 'glimpsing' | 'discerning' | 'walking';
}

/**
 * DomainReflection - How learning reveals the self.
 *
 * When studying a domain, we learn not just about the subject
 * but about ourselves. What topics energize us? What do we avoid?
 */
export interface DomainReflection {
  domainMapId: string;
  domainTitle: string;
  selfDiscoveries: string[];
  energizers: string[];  // Topics that light me up
  resistances: string[];  // Topics I avoid (why?)
  connectionToVocation?: string;
}

// ============================================================================
// Psychometric Assessments (Validated Self-Knowledge)
// ============================================================================

/**
 * Psychometric assessments serve three purposes:
 *
 * 1. SELF-KNOWLEDGE: Scientifically validated insights about oneself
 *    - Personality (Big Five, MBTI-adjacent)
 *    - Attachment styles (relationship patterns)
 *    - Emotional intelligence
 *    - Values inventories
 *    - Strengths assessments
 *
 * 2. CONTRIBUTION: Anonymized data for research (with consent)
 *    - Aggregated patterns support scientific understanding
 *    - Contributors earn recognition for participation
 *    - Data sovereignty: human owns their data, chooses to contribute
 *
 * 3. ELOHIM GUIDANCE: Pattern detection enables personalized growth paths
 *    - Crisis detection (anxiety patterns, relationship distress)
 *    - Blind spot identification
 *    - Learning path recommendations based on profile
 *    - "You might benefit from the attachment repair path"
 *
 * Privacy model:
 * - Raw results stored on human's private source chain (never on DHT)
 * - Anonymized contributions use differential privacy
 * - Elohim accesses patterns, not raw data
 * - Human can revoke research consent at any time
 */

/**
 * AssessmentInstrument - A validated psychometric tool.
 *
 * These are NOT casual quizzes. They are scientifically validated
 * instruments with known reliability and validity.
 *
 * ATTRIBUTION MODEL:
 * Assessments are first-class attributable content, just like learning paths.
 * Contributors who develop or validate instruments earn recognition when:
 * - Someone completes their assessment
 * - Their instrument is cited in research
 * - Their validation work enables new insights
 *
 * An assessment can function as a specialized learning path:
 * - Pre-assessment content (preparation, context)
 * - The instrument itself (questions/exercises)
 * - Post-assessment content (interpretation, growth resources)
 * - Attestations granted upon completion
 */
export interface AssessmentInstrument {
  id: string;
  name: string;
  shortName: string;  // e.g., "BFI-2", "ECR-R", "VIA"
  description: string;

  /** What this assessment measures */
  domain: AssessmentDomain;

  /** Psychometric properties */
  validation: InstrumentValidation;

  /** How long it typically takes */
  estimatedMinutes: number;

  /** How often it should be retaken for longitudinal data */
  recommendedInterval?: string;  // ISO 8601 duration, e.g., "P6M" (6 months)

  /** Content warnings (some assessments touch difficult topics) */
  contentWarnings?: string[];

  /** Which Imago Dei dimensions this informs */
  informsDimensions: Array<'core' | 'experience' | 'gifts' | 'synthesis'>;

  /** License/usage restrictions */
  license: InstrumentLicense;

  // =========================================================================
  // ATTRIBUTION & RECOGNITION (Assessments as Attributable Content)
  // =========================================================================

  /** Contributors who developed this instrument */
  contributors: InstrumentContributor[];

  /** Original source (if adapting existing instrument) */
  derivedFrom?: InstrumentDerivation;

  /** Content reach - same model as ContentNode */
  reach: 'private' | 'invited' | 'local' | 'community' | 'federated' | 'commons';

  /** Recognition flows when instrument is used */
  recognitionModel: InstrumentRecognitionModel;

  // =========================================================================
  // ASSESSMENT AS LEARNING PATH
  // =========================================================================

  /**
   * Assessments can be structured as mini learning paths:
   * 1. Pre-assessment: Context, instructions, emotional preparation
   * 2. Instrument: The actual questions/exercises
   * 3. Post-assessment: Results interpretation, growth resources
   *
   * This enables attestations for completing assessments.
   */
  pathStructure?: AssessmentPathStructure;

  /** Attestations granted upon completion */
  attestationsGranted?: AssessmentAttestation[];

  /** Prerequisites (other assessments or paths) */
  prerequisites?: AssessmentPrerequisite[];
}

/**
 * InstrumentContributor - Someone who contributed to this assessment.
 *
 * Uses ContributorPresence model - can be claimed or unclaimed.
 */
export interface InstrumentContributor {
  /** Contributor presence ID (may be unclaimed external researcher) */
  contributorPresenceId: string;

  /** Role in development */
  role: ContributorRole;

  /** Specific contribution description */
  contribution?: string;

  /** Recognition share (0.0 - 1.0, must sum to 1.0 across contributors) */
  recognitionShare: number;
}

export type ContributorRole =
  | 'original-author'      // Created the instrument
  | 'validator'            // Conducted validation studies
  | 'adapter'              // Adapted for new context/population
  | 'translator'           // Translated to new language
  | 'normer'               // Developed norms for population
  | 'digitizer'            // Converted to digital format
  | 'curator'              // Curated into Lamad with interpretation
  | 'elohim-synthesizer';  // AI-assisted development

/**
 * InstrumentDerivation - When an assessment builds on prior work.
 */
export interface InstrumentDerivation {
  /** Original instrument this derives from */
  originalInstrumentId?: string;

  /** External reference if not in system */
  externalReference?: string;

  /** How it was derived */
  derivationType: 'adaptation' | 'translation' | 'short-form' | 'extension' | 'synthesis';

  /** What changed from original */
  modifications: string[];

  /** License compatibility confirmed */
  licenseCompatible: boolean;
}

/**
 * InstrumentRecognitionModel - How recognition flows when instrument is used.
 */
export interface InstrumentRecognitionModel {
  /** Recognition event type when someone completes assessment */
  completionEventType: 'assessment-complete';

  /** Base recognition amount per completion */
  baseRecognition: number;

  /** Multiplier for high-quality responses */
  qualityMultiplier?: number;

  /** Recognition for research contribution (if consented) */
  researchContributionBonus?: number;

  /** Recognition flows to these contributor presences */
  recognitionRecipients: Array<{
    contributorPresenceId: string;
    share: number;  // 0.0 - 1.0
  }>;

  /** Citation recognition when used in research */
  citationRecognition?: number;
}

/**
 * AssessmentPathStructure - Assessment as a mini learning path.
 */
export interface AssessmentPathStructure {
  /** Pre-assessment content (preparation, context) */
  preAssessmentSteps?: AssessmentStep[];

  /** The instrument itself (questions organized into sections) */
  instrumentSections: InstrumentSection[];

  /** Post-assessment content (interpretation, resources) */
  postAssessmentSteps?: AssessmentStep[];

  /** Estimated total duration including pre/post content */
  totalEstimatedMinutes: number;

  /** Can sections be completed across multiple sessions? */
  allowPartialCompletion: boolean;

  /** Save progress between sessions? */
  saveProgress: boolean;
}

export interface AssessmentStep {
  /** Step order */
  order: number;

  /** Content node ID for this step */
  contentId: string;

  /** Step narrative (why this matters) */
  narrative: string;

  /** Is this step required? */
  required: boolean;
}

export interface InstrumentSection {
  /** Section identifier */
  id: string;

  /** Section title */
  title: string;

  /** Section instructions */
  instructions?: string;

  /** Questions in this section */
  questions: AssessmentQuestion[];

  /** Time limit for section (optional) */
  timeLimitMinutes?: number;

  /** Subscale(s) this section contributes to */
  contributesToSubscales: string[];
}

export interface AssessmentQuestion {
  /** Question identifier */
  id: string;

  /** Question text */
  text: string;

  /** Question type */
  type: QuestionType;

  /** Response options (for multiple choice, likert) */
  options?: QuestionOption[];

  /** Scale anchors (for likert) */
  scaleAnchors?: { low: string; high: string };

  /** Reverse scored? */
  reverseScored: boolean;

  /** Which subscale(s) this contributes to */
  subscales: string[];

  /** Content warning for sensitive questions */
  contentWarning?: string;

  /** Allow skip? (some instruments require all responses) */
  allowSkip: boolean;
}

export type QuestionType =
  | 'likert-5'           // 5-point Likert scale
  | 'likert-7'           // 7-point Likert scale
  | 'multiple-choice'    // Single selection
  | 'multiple-select'    // Multiple selections allowed
  | 'ranking'            // Rank items in order
  | 'slider'             // Continuous scale
  | 'open-text'          // Free text response
  | 'forced-choice';     // Choose between two options

export interface QuestionOption {
  value: number | string;
  label: string;
  description?: string;
}

/**
 * AssessmentAttestation - Attestation granted upon assessment completion.
 *
 * Unlike path attestations which certify knowledge/skill,
 * assessment attestations certify self-knowledge in a domain.
 */
export interface AssessmentAttestation {
  /** Attestation type ID */
  attestationTypeId: string;

  /** What this attests to */
  attestsTo: AssessmentAttestationType;

  /** Human-readable name */
  name: string;

  /** Description of what this means */
  description: string;

  /** Requirements for granting */
  requirements: AssessmentAttestationRequirement[];

  /** Can be displayed publicly? */
  publiclyDisplayable: boolean;

  /** Enables access to content/paths? */
  enablesAccess?: string[];  // Content or path IDs
}

export type AssessmentAttestationType =
  | 'self-knowledge'       // "I know my attachment style"
  | 'domain-exploration'   // "I've explored my values"
  | 'pattern-awareness'    // "I'm aware of my anxiety patterns"
  | 'growth-commitment'    // "I've committed to growth in this area"
  | 'longitudinal'         // "I've tracked this over time"
  | 'research-contributor' // "I've contributed to research"
  | 'instrument-certified'; // "I'm certified to administer this instrument"

export interface AssessmentAttestationRequirement {
  type: 'completion' | 'score-threshold' | 'quality-threshold' | 'repeat-count' | 'time-span';
  value: number | string;
  description: string;
}

/**
 * AssessmentPrerequisite - What's needed before taking this assessment.
 */
export interface AssessmentPrerequisite {
  type: 'assessment' | 'path' | 'attestation' | 'consent';

  /** ID of required item */
  requiredId?: string;

  /** Reason for prerequisite */
  reason: string;

  /** Is this a hard requirement or recommendation? */
  required: boolean;
}

export type AssessmentDomain =
  | 'personality'           // Big Five, temperament
  | 'attachment'            // Attachment style in relationships
  | 'emotional-intelligence' // EQ, emotion regulation
  | 'values'                // Personal values hierarchy
  | 'strengths'             // Character strengths (VIA)
  | 'relationship'          // Relationship satisfaction, patterns
  | 'wellbeing'             // Mental health screening
  | 'spiritual'             // Faith development, spiritual gifts
  | 'vocational'            // Career interests, work values
  | 'cognitive'             // Learning styles, cognitive patterns
  | 'trauma'                // ACE, trauma screening (sensitive)
  | 'family-systems';       // Family of origin patterns

export interface InstrumentValidation {
  /** Published validation study reference */
  validationStudy?: string;

  /** Internal consistency (Cronbach's alpha) */
  reliability?: number;

  /** Test-retest reliability */
  testRetest?: number;

  /** Convergent/discriminant validity */
  validityNotes?: string;

  /** Whether normed on diverse populations */
  normingNotes?: string;

  /** Known limitations */
  limitations?: string[];
}

export type InstrumentLicense =
  | 'public-domain'         // Free to use
  | 'creative-commons'      // CC license
  | 'research-only'         // Academic use only
  | 'licensed'              // Requires license fee
  | 'elohim-developed';     // Developed within Elohim Protocol

/**
 * AssessmentResult - A completed assessment.
 */
export interface AssessmentResult {
  id: string;
  instrumentId: string;
  completedAt: string;

  /** Raw scores (private, on source chain) */
  rawScores: Record<string, number>;

  /** Interpreted results (human-readable) */
  interpretation: AssessmentInterpretation;

  /** How this connects to self-knowledge */
  selfKnowledgeLinks: SelfKnowledgeLink[];

  /** Whether this result is contributing to research */
  contributingToResearch: boolean;

  /** Confidence level (did they rush through?) */
  responseQuality: ResponseQuality;
}

export interface AssessmentInterpretation {
  /** Overall summary in plain language */
  summary: string;

  /** Subscale results */
  subscales: SubscaleResult[];

  /** Strengths highlighted */
  highlightedStrengths: string[];

  /** Growth areas identified */
  growthAreas: string[];

  /** Recommended learning paths based on results */
  recommendedPaths?: string[];

  /** Comparison to previous results (if longitudinal) */
  longitudinalComparison?: LongitudinalChange[];
}

export interface SubscaleResult {
  name: string;
  score: number;
  percentile?: number;  // If normed
  interpretation: string;
  relatedGifts?: string[];
  relatedShadows?: string[];
}

export interface LongitudinalChange {
  subscale: string;
  previousScore: number;
  currentScore: number;
  changeDirection: 'increased' | 'decreased' | 'stable';
  significance: 'significant' | 'marginal' | 'none';
  interpretation: string;
}

export interface SelfKnowledgeLink {
  /** Which part of the self-map this informs */
  targetType: 'dimension' | 'value' | 'gift' | 'shadow' | 'vocation';
  targetId?: string;

  /** How this assessment informs that area */
  insight: string;

  /** Confidence in this link */
  confidence: number;  // 0.0 - 1.0
}

export interface ResponseQuality {
  /** Time taken (was it rushed?) */
  completionTime: number;

  /** Consistency check (if instrument has one) */
  consistencyScore?: number;

  /** Social desirability bias indicator */
  socialDesirabilityFlag?: boolean;

  /** Overall quality assessment */
  quality: 'high' | 'acceptable' | 'questionable';

  /** Reason if questionable */
  qualityNotes?: string;
}

/**
 * PatternAlert - Elohim-detected patterns requiring attention.
 *
 * When assessment data reveals concerning patterns (anxiety spike,
 * relationship distress, etc.), Elohim can gently alert the human
 * and suggest growth paths.
 *
 * This is NOT diagnosis. It's pattern recognition that says
 * "You might benefit from exploring this area."
 */
export interface PatternAlert {
  id: string;
  detectedAt: string;

  /** What pattern was detected */
  patternType: PatternType;

  /** Severity/urgency */
  urgency: 'gentle-nudge' | 'worth-attention' | 'important' | 'urgent';

  /** Human-readable description */
  description: string;

  /** Evidence from assessments */
  evidence: PatternEvidence[];

  /** Suggested response */
  suggestedActions: SuggestedAction[];

  /** Has the human acknowledged this? */
  acknowledged: boolean;
  acknowledgedAt?: string;

  /** Human's chosen response */
  response?: PatternResponse;
}

export type PatternType =
  | 'anxiety-elevation'          // Anxiety indicators increasing
  | 'depression-indicators'      // Depression screening flags
  | 'relationship-distress'      // Relationship satisfaction declining
  | 'attachment-activation'      // Attachment patterns being triggered
  | 'burnout-risk'               // Work-life balance concerns
  | 'value-conflict'             // Living out of alignment with values
  | 'growth-opportunity'         // Positive pattern - ready for next level
  | 'blind-spot-revealed'        // Assessment revealed unknown pattern
  | 'longitudinal-shift';        // Significant change over time

export interface PatternEvidence {
  assessmentResultId: string;
  subscale: string;
  observation: string;
  weight: number;  // How much this contributes to pattern detection
}

export interface SuggestedAction {
  type: 'learning-path' | 'assessment' | 'reflection' | 'professional-help' | 'community';
  title: string;
  description: string;
  resourceId?: string;  // Path ID, assessment ID, etc.
  urgency: 'when-ready' | 'soon' | 'promptly';
}

export interface PatternResponse {
  action: 'pursuing' | 'noted' | 'dismissed' | 'sought-help';
  notes?: string;
  respondedAt: string;
}

/**
 * ResearchConsent - Human's consent for data contribution.
 *
 * Sovereignty principle: Humans own their data and choose
 * whether to contribute to collective knowledge.
 */
export interface ResearchConsent {
  /** Has consent been granted? */
  granted: boolean;

  /** When consent was given/updated */
  updatedAt: string;

  /** What level of contribution? */
  scope: ResearchConsentScope;

  /** Specific domains excluded */
  excludedDomains?: AssessmentDomain[];

  /** Can results be used for pattern detection? */
  allowPatternDetection: boolean;

  /** Can Elohim suggest paths based on results? */
  allowPathSuggestions: boolean;

  /** How long consent is valid */
  expiresAt?: string;

  /** Recognition earned for contributions */
  contributionRecognition?: ContributionRecognition;
}

export type ResearchConsentScope =
  | 'none'              // No contribution
  | 'aggregate-only'    // Only aggregate statistics
  | 'anonymized'        // Full anonymized individual data
  | 'identifiable';     // Willing to be contacted for studies

export interface ContributionRecognition {
  /** Number of assessments contributed */
  assessmentsContributed: number;

  /** Research impact score */
  impactScore: number;

  /** Studies that used this human's data */
  studiesContributed: string[];

  /** Recognition in REA terms */
  recognitionEvents: string[];  // EconomicEvent IDs
}

/**
 * CrisisProtocol - When patterns indicate serious concern.
 *
 * If assessment patterns suggest crisis (suicidal ideation,
 * severe depression, domestic violence risk), the system
 * must respond appropriately.
 *
 * NOT a replacement for professional help, but a safety net.
 */
export interface CrisisProtocol {
  /** Threshold scores that trigger protocol */
  triggerThresholds: Record<string, number>;

  /** Immediate resources shown */
  immediateResources: CrisisResource[];

  /** Elohim escalation (if human consents) */
  elohimEscalation?: {
    notifyLevel: 'individual-elohim' | 'family-elohim' | 'community-elohim';
    consentRequired: boolean;
  };

  /** Cooling-off period before allowing dismissal */
  minimumAcknowledgmentTime: number;  // seconds
}

export interface CrisisResource {
  name: string;
  description: string;
  contactInfo: string;
  type: 'hotline' | 'text-line' | 'website' | 'local-service';
  available: string;  // e.g., "24/7", "M-F 9am-5pm"
}

// ============================================================================
// Person Knowledge Map (Gottman Love Maps)
// ============================================================================

/**
 * PersonKnowledgeMap - Knowledge map about another person.
 *
 * Inspired by Gottman's Love Maps research: the mental space where
 * you store detailed knowledge about someone you care about.
 *
 * Key differences from domain maps:
 * - Subject is a person, not a content graph
 * - Knowledge is relational (requires consent for deep access)
 * - Categories are relationship-oriented
 * - Privacy is paramount
 */
export interface PersonKnowledgeMap extends KnowledgeMap {
  mapType: 'person';

  subject: {
    type: 'agent';
    subjectId: string;  // The person being mapped
    subjectName: string;
  };

  /** Relationship type between mapper and subject */
  relationshipType: RelationshipType;

  /** Consent from the subject to be mapped */
  subjectConsent?: SubjectConsent;

  /** Categories of knowledge (Gottman-inspired) */
  categories: PersonKnowledgeCategory[];

  /** Reciprocal map (if subject also maps the owner) */
  reciprocalMapId?: string;

  /** Relationship health metrics */
  relationshipMetrics?: RelationshipMetrics;
}

export type RelationshipType =
  | 'spouse'
  | 'partner'
  | 'parent'
  | 'child'
  | 'sibling'
  | 'friend'
  | 'mentor'
  | 'mentee'
  | 'colleague'
  | 'acquaintance'
  | 'other';

/**
 * SubjectConsent - Permission from the person being mapped.
 *
 * Critical for ethical knowledge mapping. Without consent,
 * maps are limited to publicly available information.
 */
export interface SubjectConsent {
  /** Has the subject granted permission? */
  granted: boolean;

  /** What scope of access is permitted? */
  scope: ConsentScope;

  /** When was consent granted? */
  grantedAt?: string;

  /** When does consent expire? (optional) */
  expiresAt?: string;

  /** Can the subject see what's in the map? */
  transparencyLevel: 'none' | 'categories-only' | 'full-read' | 'collaborative';
}

export type ConsentScope =
  | 'public-info'    // Only publicly shared information
  | 'shared-only'    // Only what subject explicitly shares with mapper
  | 'full-access';   // Deep knowledge mapping permitted

/**
 * PersonKnowledgeCategory - Gottman-inspired knowledge categories.
 */
export interface PersonKnowledgeCategory {
  id: string;
  type: PersonKnowledgeCategoryType;
  title: string;
  description?: string;
  nodes: KnowledgeNode[];
  affinity: number;
}

export type PersonKnowledgeCategoryType =
  | 'life-history'         // Past experiences, childhood, formative events
  | 'current-stressors'    // Present challenges, worries, pressures
  | 'dreams-aspirations'   // Future hopes, goals, ambitions
  | 'values-beliefs'       // Core principles, worldview, ethics
  | 'preferences-dislikes' // Daily preferences, pet peeves, favorites
  | 'friends-family'       // Social network, important relationships
  | 'work-career'          // Professional life, skills, ambitions
  | 'health-wellbeing'     // Physical/mental health, self-care
  | 'communication-style'  // How they express, receive love/feedback
  | 'conflict-patterns'    // How they handle disagreement
  | 'love-language'        // Primary ways of giving/receiving love
  | 'custom';              // User-defined categories

/**
 * RelationshipMetrics - Health indicators for the relationship.
 */
export interface RelationshipMetrics {
  /** Overall relationship health (0.0 - 1.0) */
  overallHealth: number;

  /** How complete is the knowledge map? */
  mapCompleteness: number;

  /** How recent is the knowledge? */
  knowledgeFreshness: number;

  /** Are both parties actively mapping each other? */
  reciprocity: number;

  /** Last meaningful interaction/update */
  lastInteraction: string;
}

// ============================================================================
// Collective Knowledge Map (Organizations, Teams)
// ============================================================================

/**
 * CollectiveKnowledgeMap - Shared knowledge within a group.
 *
 * Represents what "we" know as a team, organization, or community.
 * Combines individual contributions into collective intelligence.
 */
export interface CollectiveKnowledgeMap extends KnowledgeMap {
  mapType: 'collective';

  subject: {
    type: 'organization';
    subjectId: string;  // The collective being mapped
    subjectName: string;
  };

  /** Members who contribute to this map */
  members: CollectiveMember[];

  /** Governance model for the map */
  governance: CollectiveGovernance;

  /** Domains of collective knowledge */
  domains: CollectiveDomain[];

  /** Attestations granted by collective consensus */
  collectiveAttestations: string[];
}

export interface CollectiveMember {
  agentId: string;
  role: 'steward' | 'contributor' | 'viewer';
  joinedAt: string;
  contributionCount: number;
}

export interface CollectiveGovernance {
  /** How are changes approved? */
  approvalModel: 'steward-only' | 'majority-vote' | 'consensus' | 'open';

  /** Minimum contributors for changes */
  quorum?: number;

  /** Who can add new members? */
  membershipControl: 'steward-only' | 'member-invite' | 'open';
}

export interface CollectiveDomain {
  id: string;
  title: string;
  description: string;
  stewards: string[];  // Agent IDs responsible for this domain
  nodes: KnowledgeNode[];
  affinity: number;  // Collective mastery level
}

// ============================================================================
// Knowledge Map Index & Discovery
// ============================================================================

/**
 * KnowledgeMapIndex - Lightweight entry for map discovery.
 */
export interface KnowledgeMapIndexEntry {
  id: string;
  mapType: KnowledgeMapType;
  title: string;
  subjectName: string;
  ownerId: string;
  ownerName: string;
  visibility: string;
  overallAffinity: number;
  nodeCount: number;
  updatedAt: string;
}

/**
 * KnowledgeMapIndex - Response from map catalog endpoint.
 */
export interface KnowledgeMapIndex {
  lastUpdated: string;
  totalCount: number;
  maps: KnowledgeMapIndexEntry[];
}

// ============================================================================
// Map Operations
// ============================================================================

/**
 * KnowledgeMapUpdate - Mutation operation on a map.
 */
export interface KnowledgeMapUpdate {
  mapId: string;
  operation: 'add-node' | 'update-node' | 'remove-node' | 'update-affinity';
  nodeId?: string;
  data: Partial<KnowledgeNode>;
  source?: KnowledgeSource;
  timestamp: string;
}

/**
 * MapMergeRequest - Request to merge knowledge from another map.
 */
export interface MapMergeRequest {
  sourceMapId: string;
  targetMapId: string;
  nodeIds: string[];  // Specific nodes to merge
  conflictResolution: 'source-wins' | 'target-wins' | 'manual';
}
