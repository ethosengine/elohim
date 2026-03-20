//! AUTO-GENERATED from protocol JSON schemas.
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json

/// Core completioncriteria — All completion criteria are protocol-level — they gate path progression and mastery attestation.
pub const CORE_COMPLETION_CRITERIA: &[&str] = &[
    "all-required",
    "pass-assessment",
    "view-content",
];

/// All completioncriteria — includes storage-only and extensible.
pub const ALL_COMPLETION_CRITERIA: &[&str] = &[
    "all-required",
    "pass-assessment",
    "view-content",
];

/// Core contentformat — DNA-notarized format categories. Broad enough to encompass all rendering approaches.
pub const CORE_CONTENT_FORMATS: &[&str] = &[
    "markdown",
    "html",
    "video",
    "audio",
    "interactive",
    "external",
];

/// All contentformat — includes storage-only and extensible.
pub const ALL_CONTENT_FORMATS: &[&str] = &[
    "markdown",
    "html",
    "video",
    "audio",
    "interactive",
    "external",
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
];

/// Core engagementtype — All engagement types are protocol-level — they drive recognition flows and couple knowledge+value+governance.
pub const CORE_ENGAGEMENT_TYPES: &[&str] = &[
    "view",
    "quiz",
    "practice",
    "discuss",
    "create",
    "peer",
    "teach",
    "apply",
];

/// All engagementtype — includes storage-only and extensible.
pub const ALL_ENGAGEMENT_TYPES: &[&str] = &[
    "view",
    "quiz",
    "practice",
    "discuss",
    "create",
    "peer",
    "teach",
    "apply",
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

/// Core pathvisibility — DNA-notarized visibility. Gates content distribution.
pub const CORE_PATH_VISIBILITIES: &[&str] = &[
    "private",
    "unlisted",
    "community",
    "public",
];

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
pub const CORE_STEP_TYPES: &[&str] = &[
    "content",
    "path",
    "external",
    "checkpoint",
    "reflection",
];

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
