-- Content mastery tracking using Bloom's taxonomy
-- Includes freshness score for spaced repetition decay
CREATE TABLE content_mastery (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    human_id TEXT NOT NULL,
    content_id TEXT NOT NULL,

    -- Bloom's taxonomy level
    mastery_level TEXT NOT NULL DEFAULT 'not_started',
    mastery_level_index INTEGER NOT NULL DEFAULT 0,

    -- Freshness for spaced repetition
    freshness_score REAL NOT NULL DEFAULT 1.0,
    needs_refresh INTEGER NOT NULL DEFAULT 0,

    -- Engagement tracking
    engagement_count INTEGER NOT NULL DEFAULT 0,
    last_engagement_type TEXT,
    last_engagement_at TEXT,
    level_achieved_at TEXT,

    -- Evidence
    content_version_at_mastery TEXT,
    assessment_evidence_json TEXT,
    privileges_json TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Unique constraint: one mastery record per human per content per app
CREATE UNIQUE INDEX idx_mastery_unique ON content_mastery(app_id, human_id, content_id);

-- Indexes for mastery queries
CREATE INDEX idx_mastery_app_id ON content_mastery(app_id);
CREATE INDEX idx_mastery_human ON content_mastery(app_id, human_id);
CREATE INDEX idx_mastery_content ON content_mastery(content_id);
CREATE INDEX idx_mastery_level ON content_mastery(mastery_level);
CREATE INDEX idx_mastery_needs_refresh ON content_mastery(needs_refresh) WHERE needs_refresh = 1;
CREATE INDEX idx_mastery_freshness ON content_mastery(freshness_score);
