//! AUTO-GENERATED from protocol JSON schemas.
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json

/// Core completioncriteria — All completion criteria are protocol-level — they gate path progression and mastery attestation.
pub const CORE_COMPLETION_CRITERIA: &[&str] = &["all-required", "pass-assessment", "view-content"];

/// All completioncriteria — includes storage-only and extensible.
pub const ALL_COMPLETION_CRITERIA: &[&str] = &["all-required", "pass-assessment", "view-content"];

/// Core contentformat — DNA-notarized format categories. Broad enough to encompass all rendering approaches.
pub const CORE_CONTENT_FORMATS: &[&str] = &[
    "markdown",
    "html",
    "video",
    "audio",
    "interactive",
    "external",
    "epr-composite",
];

/// All contentformat — includes storage-only and extensible.
pub const ALL_CONTENT_FORMATS: &[&str] = &[
    "markdown",
    "html",
    "video",
    "audio",
    "interactive",
    "external",
    "epr-composite",
    "plaintext",
    "text",
    "plain",
    "gherkin",
    "perseus",
    "perseus-json",
    "perseus-quiz-json",
    "video-embed",
    "audio-file",
    "html5-app",
    "spa-bundle",
    "human-json",
    "organization-json",
    "json",
    "sophia",
    "sophia-quiz-json",
];

/// Core contenttype — Three-leg coupled: knowledge + value + governance. DNA-notarized, immutable once signed.
pub const CORE_CONTENT_TYPES: &[&str] = &[
    "epic",
    "concept",
    "lesson",
    "scenario",
    "assessment",
    "reflection",
    "discussion",
    "exercise",
    "article",
    "path",
];

/// All contenttype — includes storage-only and extensible.
pub const ALL_CONTENT_TYPES: &[&str] = &[
    "epic",
    "concept",
    "lesson",
    "scenario",
    "assessment",
    "reflection",
    "discussion",
    "exercise",
    "article",
    "path",
    "human",
    "role",
    "collective",
    "example",
    "reference",
    "feature",
    "practice",
    "contributor",
    "video",
    "audio",
    "book",
    "book-chapter",
    "documentary",
    "bible-verse",
    "activity",
    "narrative",
    "course-module",
    "module",
    "quiz",
    "podcast",
    "simulation",
    "node-context",
    "stewardship-context",
    "work-story",
    "work-project",
    "issue-report",
    "application",
    "gate-process-declaration",
    "universal-band-declaration",
    "gate-rules-declaration",
    "aggregation-spec",
    "escalation-target-spec",
];

/// Core devicearchetype — DNA-notarized device archetypes for AgentPeerBinding entries in imagodei integrity zome (Task A.2). 'node' = dedicated elohim-node instances (household blades, server processes). 'desktop' = Tauri-wrapped desktop clients. 'mobile' = mobile devices. 'steward' = collective infrastructure steward processes. All four values are required at genesis; no extensible tier defined.
pub const CORE_DEVICE_ARCHETYPES: &[&str] = &["node", "desktop", "mobile", "steward"];

/// All devicearchetype — includes storage-only and extensible.
pub const ALL_DEVICE_ARCHETYPES: &[&str] = &["node", "desktop", "mobile", "steward"];

/// Core elohimskill — Gate-shaped skills with active registered gate interfaces: content-safety-gate, discernment-gate-v1-mechanical, reach-gate-v1. These 8 skills are wired to protocol gate invocation paths. Phase 4 Section 6.3 gate primitives. DNA-notarized for dispatch validation.
pub const CORE_ELOHIM_SKILLS: &[&str] = &[
    "content-safety-review",
    "discernment-evaluation",
    "reach-negotiation",
    "attestation-recommendation",
    "spiral-detection",
    "care-connection",
    "graduated-intervention",
    "constitutional-verification",
];

/// All elohimskill — includes storage-only and extensible.
pub const ALL_ELOHIM_SKILLS: &[&str] = &[
    "content-safety-review",
    "discernment-evaluation",
    "reach-negotiation",
    "attestation-recommendation",
    "spiral-detection",
    "care-connection",
    "graduated-intervention",
    "constitutional-verification",
    "accuracy-verification",
    "knowledge-map-synthesis",
    "affinity-analysis",
    "path-recommendation",
    "cross-layer-validation",
    "existential-boundary-enforcement",
    "governance-ratification",
    "path-analysis",
    "learning-objective-validation",
    "prerequisite-verification",
    "mastery-assessment-design",
    "family-value-alignment",
    "personal-agent-support",
    "feedback-profile-negotiation",
    "feedback-profile-enforcement",
    "feedback-profile-upgrade",
    "feedback-profile-downgrade",
    "place-attestation",
    "place-naming-governance",
    "geographic-reach-assignment",
    "bioregional-enforcement",
];

/// Core elohimspecialty — Protocol-known specialties that affect gate dispatch routing. An elohim declaring child-safety, medical, or crisis receives specific gate invocations the protocol itself orchestrates. DNA-notarized for validation at imagodei layer.
pub const CORE_ELOHIM_SPECIALTIES: &[&str] = &[
    "child-safety",
    "family-dynamics",
    "content-safety",
    "discernment",
    "reach-evaluation",
    "medical",
    "legal",
    "crisis",
];

/// All elohimspecialty — includes storage-only and extensible.
pub const ALL_ELOHIM_SPECIALTIES: &[&str] = &[
    "child-safety",
    "family-dynamics",
    "content-safety",
    "discernment",
    "reach-evaluation",
    "medical",
    "legal",
    "crisis",
    "education",
    "code-review",
    "financial-advice",
    "creative-writing",
    "technical-documentation",
    "translation",
    "poetry",
    "philosophy",
    "governance",
    "conflict-resolution",
    "curriculum-design",
    "counseling",
    "theology",
    "science",
    "mathematics",
    "history",
    "language-learning",
    "mental-health",
    "disability-support",
    "elder-care",
    "research",
    "journalism",
];

/// Core elohimstrength — Protocol-named attestation-history dimensions. high-confidence-judgments: decisions made confidently and upheld. appeals-sustained: appeals that overturned prior decisions (surfaces calibration gaps). consensus-alignment: agreement with peer elohim on same inputs. novel-pattern-detection: surfaced new attestation classes. steady-baseline: consistent no-mint steady-state. These five dimensions directly inform peer-diversity selection and elohim trust scoring in Phase 11+.
pub const CORE_ELOHIM_STRENGTHS: &[&str] = &[
    "high-confidence-judgments",
    "appeals-sustained",
    "consensus-alignment",
    "novel-pattern-detection",
    "steady-baseline",
];

/// All elohimstrength — includes storage-only and extensible.
pub const ALL_ELOHIM_STRENGTHS: &[&str] = &[
    "high-confidence-judgments",
    "appeals-sustained",
    "consensus-alignment",
    "novel-pattern-detection",
    "steady-baseline",
    "consistent-constitutional-reasoning",
    "low-false-positive-rate",
    "fast-resolution",
    "cross-context-consistency",
    "escalation-accuracy",
];

/// Core engagementtype — All engagement types are protocol-level — they drive recognition flows and couple knowledge+value+governance.
pub const CORE_ENGAGEMENT_TYPES: &[&str] = &[
    "view", "quiz", "practice", "discuss", "create", "peer", "teach", "apply",
];

/// All engagementtype — includes storage-only and extensible.
pub const ALL_ENGAGEMENT_TYPES: &[&str] = &[
    "view", "quiz", "practice", "discuss", "create", "peer", "teach", "apply",
];

/// Core instrumentarchetype — All six archetypes are protocol primitives — they define the categories of questions a system must ask about itself.
pub const CORE_INSTRUMENT_ARCHETYPES: &[&str] = &[
    "retention-check",
    "outcome-correlation",
    "distribution-health",
    "cost-accumulation",
    "outcome-divergence",
    "community-report",
];

/// All instrumentarchetype — includes storage-only and extensible.
pub const ALL_INSTRUMENT_ARCHETYPES: &[&str] = &[
    "retention-check",
    "outcome-correlation",
    "distribution-health",
    "cost-accumulation",
    "outcome-divergence",
    "community-report",
];

/// Core masterylevel — Bloom's taxonomy levels. DNA-notarized. Level 4 (apply) is the attestation gate for governance participation.
pub const CORE_MASTERY_LEVELS: &[&str] = &[
    "not_started",
    "seen",
    "remember",
    "understand",
    "apply",
    "analyze",
    "evaluate",
    "create",
];

/// All masterylevel — includes storage-only and extensible.
pub const ALL_MASTERY_LEVELS: &[&str] = &[
    "not_started",
    "seen",
    "remember",
    "understand",
    "apply",
    "analyze",
    "evaluate",
    "create",
    "recognize",
    "recall",
    "synthesize",
];

/// Core observationpolarity — Binary polarity is a protocol invariant — every observation either supports or strains a claim.
pub const CORE_OBSERVATION_POLARITIES: &[&str] = &["positive", "negative"];

/// All observationpolarity — includes storage-only and extensible.
pub const ALL_OBSERVATION_POLARITIES: &[&str] = &["positive", "negative"];

/// Core pathvisibility — DNA-notarized visibility. Gates content distribution.
pub const CORE_PATH_VISIBILITIES: &[&str] = &["private", "unlisted", "community", "public"];

/// All pathvisibility — includes storage-only and extensible.
pub const ALL_PATH_VISIBILITIES: &[&str] = &[
    "private",
    "intimate",
    "unlisted",
    "community",
    "public",
    "draft",
];

/// Core reach — All reach levels are DNA-notarized. They gate content distribution and are enforced by doorway.
pub const CORE_REACH_LEVELS: &[&str] = &[
    "private",
    "self",
    "intimate",
    "trusted",
    "familiar",
    "community",
    "public",
    "commons",
];

/// All reach — includes storage-only and extensible.
pub const ALL_REACH_LEVELS: &[&str] = &[
    "private",
    "self",
    "intimate",
    "trusted",
    "familiar",
    "community",
    "public",
    "commons",
];

/// Core steptype — DNA-notarized step types. Structural path elements.
pub const CORE_STEP_TYPES: &[&str] = &["content", "path", "external", "checkpoint", "reflection"];

/// All steptype — includes storage-only and extensible.
pub const ALL_STEP_TYPES: &[&str] = &[
    "content",
    "read",
    "path",
    "external",
    "practice",
    "assess",
    "video",
    "interactive",
    "checkpoint",
    "reflection",
];

/// Core substratesignal — All substrate signals are protocol-level primitives. They are the dimensions through which value, governance, and knowledge couple to infrastructure.
pub const CORE_SUBSTRATE_SIGNALS: &[&str] = &[
    "attention",
    "compute",
    "storage",
    "bandwidth",
    "energy",
    "time",
    "resource",
];

/// All substratesignal — includes storage-only and extensible.
pub const ALL_SUBSTRATE_SIGNALS: &[&str] = &[
    "attention",
    "compute",
    "storage",
    "bandwidth",
    "energy",
    "time",
    "resource",
];
