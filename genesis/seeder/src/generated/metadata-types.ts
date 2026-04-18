// AUTO-GENERATED from lamad manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen

export interface ConceptMetadata {
  /** Brief summary of the concept */
  summary?: string;
  /** Original file path from import pipeline */
  sourcePath?: string;
  /** IDs of related content nodes */
  relatedNodeIds?: string[];
  /** Estimated time to consume in minutes */
  estimatedMinutes?: number;
  /** Blob path or resolved URL for concept thumbnail */
  thumbnailUrl?: string;
  /** Bloom's taxonomy level (remember, understand, apply, analyze, evaluate, create) */
  bloomsLevel?: string;
  /** Reference to source document */
  sourceDoc?: string;
  /** Inline relationship declarations from import */
  relationships?: { type?: string; targetId?: string }[];
  /** Decentralized identifier for the concept */
  did?: string;
  /** Open Graph metadata for social sharing */
  openGraphMetadata?: Record<string, unknown>;
  /** JSON-LD or schema.org structured data */
  linkedData?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface AssessmentMetadata {
  /** Reference to the psychometric instrument definition (content ID) */
  instrument?: string;
  /** Assessment mode determining scoring and feedback behavior */
  mode?: 'mastery' | 'discovery' | 'reflection';
  /** Scoring configuration — instrument-specific rules for translating responses to scores */
  scoringRules?: Record<string, unknown>;
  /** Subscale definitions for multi-dimensional assessments */
  subscales?: { id?: string; name?: string; weight?: number }[];
  [key: string]: unknown;
}

export interface ExperienceStoryMetadata {
  /** CID or slug reference to the subject ContentNode (human or collective). */
  subjectRef: string;
  /** CID or slug reference to the role ContentNode. */
  roleRef: string;
  /** CID or slug reference to the feature ContentNode (gherkin feature canonical entry). */
  featureRef: string;
  /** Optional human-readable EPR alias, e.g. epr:experience-story/matthew-manager/as-entrepreneur/learning-journey. */
  aliasEpr?: string;
}

export interface ExperienceMomentMetadata {
  /** CID or slug reference to the subject ContentNode (human or collective). */
  subjectRef: string;
  /** CID or slug reference to the role ContentNode. */
  roleRef: string;
  /** CID or slug reference to the feature ContentNode (gherkin feature canonical entry). */
  featureRef: string;
  /** The scenario name (Scenario: or Scenario Outline: line from the feature file). */
  scenarioName: string;
  /** Relative path from genesis/a2o/features to the feature file, e.g. 'lamad/learning-paths/path-progression.feature'. */
  scenarioUri: string;
  /** Line number where the scenario starts in the feature file. */
  scenarioLine?: number;
  /** Cucumber tags from the scenario (e.g., @regression, @smoke). */
  scenarioTags?: string[];
  /** Scenario execution status. */
  status: 'passed' | 'failed' | 'pending' | 'skipped';
  /** Total execution time in milliseconds. */
  durationMs: number;
  /** Git commit SHA (abbreviated 7 chars or full 40 chars). */
  commit: string;
  /** Unique identifier for this test run (e.g., CI build ID or local run timestamp). */
  runId: string;
  /** Format: {pod}:{deviceArchetype}:{archetypeRevisionHash} — identifies the compute environment. */
  computeFingerprint: string;
  /** Present when status=failed; classifies the error (e.g., 'AssertionError/timeout', 'NetworkError/503'). */
  errorClass?: string;
  /** References to test artifacts (CIDs or paths). */
  sidecarArtifacts?: { cucumber?: string; observation?: string; screenshot?: string; trace?: string; console?: string };
  /** CID or slug reference to the parent experience-story ContentNode. */
  relatedExperienceStory?: string;
}

export interface PathMetadata {
  /** journey | guided | self-paced | assessment */
  pathType?: string;
  /** beginner | intermediate | advanced */
  difficulty?: string;
  /** Human-readable duration (e.g. '6-8 hours') */
  estimatedDuration?: string;
  version?: string;
  purpose?: string;
  /** Blob path or resolved URL for path thumbnail */
  thumbnailUrl?: string;
  thumbnailAlt?: string;
  contributors?: string[];
  prerequisitePaths?: string[];
  attestationsGranted?: string[];
  [key: string]: unknown;
}

export interface ProcessDag {
  /** Step id where execution begins. */
  entrypoint: string;
  /** Map of step-id to step definition. Keys are stable references used by edges. */
  steps: Record<string, unknown>;
  /** Map of terminal-id to terminal node. Terminals emit a decision and optional side effects. */
  terminals: Record<string, unknown>;
}

export interface StepNode {
  /** One of the seven v1 protocol-governed step types (spec §2.1). */
  type: 'context-assemble' | 'wisdom-invoke' | 'mechanical-ruleset' | 'aggregate-attestations' | 'skill-invoke' | 'synthesize' | 'escalate-to-review';
  /** Step-type-specific parameters. Shape depends on type. CID references may point to gate-rules-declaration, aggregation-spec, or escalation-target-spec ContentNodes. */
  params: Record<string, unknown>;
  /** Unconditional next step or terminal id. */
  next?: string;
  /** Conditional edges evaluated against GateContext. First matching edge wins. */
  edges?: ConditionalEdge[];
}

export interface ConditionalEdge {
  /** Simple comparison expression over GateContext keys (e.g., 'ruleDecision == null'). Intentionally non-Turing-complete for static reachability analysis. */
  when: string;
  /** Target step id or terminal id. */
  target: string;
}

export interface TerminalNode {
  /** Decision shape emitted at this terminal (status + verdict/grounds as applicable). */
  decision: Record<string, unknown>;
  /** Side effects the caller must execute. The gate does NOT execute them itself. */
  sideEffects?: SideEffectSpec[];
}

export interface SideEffectSpec {
  /** Side effect kind. */
  type: 'MintAttestation' | 'EmitEconomicEvent' | 'OpenStewardReview' | 'UpdateReachAggregation';
  /** For MintAttestation: attestation shape name (e.g., StoryPointLink). */
  shape?: string;
  /** GateContext keys whose values populate the side effect's parameters. */
  paramsFromKeys?: string[];
}

export interface GateProcessDeclarationMetadata {
  /** Kebab-case gate name including version suffix (e.g., discernment-gate-v1-mechanical). */
  name: string;
  /** Semantic version of this gate process declaration. */
  version: string;
  /** Which RelationalImpactEvent variant this gate handles (kebab-case). */
  eventType: 'content-publish' | 'attestation-write' | 'economic-event-emit' | 'peer-message' | 'sync-to-peers' | 'advice-sought' | 'capability-invoke' | 'private-to-public-crossing';
  /** CID or $ref to a JSON schema describing the gate's input GateContext shape. */
  inputSchemaRef?: string;
  /** CID or $ref to a JSON schema describing the gate's output decision + side-effect shape. */
  outputSchemaRef?: string;
  /** CID of a superseded gate-process-declaration, or null. */
  supersedes?: unknown;
  /** The step graph this gate executes. */
  dag: ProcessDag;
}

export interface UniversalBandDeclarationMetadata {
  /** Semantic version of this universal-band declaration. */
  version: string;
  /** ISO 8601 timestamp when protocol governance ratified this declaration. */
  activatedAt: string;
  /** CID of a superseded universal-band-declaration, or null for the first. */
  supersedes?: unknown;
  /** The universal-band step graph. Runs for every invocation regardless of app context. */
  dag: unknown;
  /** Universal band is always governed at protocol reach. */
  governanceReach?: string;
  /** CIDs of protocol-reach ratification attestations that authorized activation. */
  ratificationAttestationCids?: string[];
}

export interface Rule {
  /** Stable rule identifier (e.g., rule-1-first-pass-green). */
  id: string;
  /** Declarative match expression over input keys. Non-Turing-complete; inspectable by elohim before execution. */
  when: string;
  /** Outcome returned by this rule on match. Null means silence (baseline absence). */
  outcome: unknown;
  /** Human-readable justification. Carried into ConstitutionalReasoning when the rule fires. */
  rationale?: string;
}

export interface GateRulesDeclarationMetadata {
  /** Kebab-case rule-set name (e.g., seven-valence-v1). */
  ruleSetName: string;
  /** Semantic version of this rule set. */
  version: string;
  /** Human-readable summary of what this rule set classifies. */
  description?: string;
  /** Which RelationalImpactEvent variant this rule set reasons over. */
  appliesToEventType?: string;
  /** GateContext keys the rules consume (e.g., ['moment', 'priorAttestations']). */
  inputKeysExpected?: string[];
  /** JSON schema $ref or shape descriptor for the rule-set's output (e.g., StoryPointTag shape). */
  outputShape?: string;
  /** Ordered rule list. First match wins. Last rule typically matches steady-state (no-mint). */
  rules: Rule[];
}

export interface Reduction {
}

export interface AggregationSpecMetadata {
  /** Kebab-case aggregation spec name (e.g., reach-level-v1). */
  specName: string;
  /** Semantic version of this aggregation spec. */
  version: string;
  /** What this aggregation computes. */
  description?: string;
  /** Which attestation kinds (or content-addressed attestation shape names) feed this aggregation. */
  attestationKinds: string[];
  /** What the aggregation's subject represents. */
  subjectScope?: 'content-node' | 'agent' | 'gate-decision' | 'app-manifest';
  /** ISO 8601 duration limiting the aggregation window (e.g., P180D). Null = all time. */
  timeWindow?: unknown;
  /** How query results reduce to a scalar. */
  reduction: Reduction;
  /** Shape of the reduced output (e.g., ReachLevel enum, numeric threshold, weighted score). */
  outputShape?: string;
}

export interface EscalationTier {
  severity: 'low' | 'medium' | 'high' | 'existential';
  target: Target;
  /** Optional reference to a notification template CID. */
  notificationShape?: string;
  /** If the target doesn't respond within this window, auto-escalate to the next tier. */
  timeoutSeconds?: number;
}

export interface Target {
}

export interface EscalationTargetSpecMetadata {
  /** Kebab-case escalation-target spec name. */
  specName: string;
  /** Semantic version of this escalation spec. */
  version: string;
  /** What class of escalations this spec handles. */
  description?: string;
  /** Ordered tiers keyed by severity. Higher severity routes to higher reach. */
  tiers: EscalationTier[];
}
