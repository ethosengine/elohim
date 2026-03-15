CREATE TABLE IF NOT EXISTS proposal_options (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    position INTEGER NOT NULL,
    source TEXT,
    source_justification TEXT,
    created_at TEXT NOT NULL
);

ALTER TABLE proposals ADD COLUMN voting_mechanism TEXT NOT NULL DEFAULT 'consent';
ALTER TABLE proposals ADD COLUMN score_min INTEGER;
ALTER TABLE proposals ADD COLUMN score_max INTEGER;
ALTER TABLE proposals ADD COLUMN dots_per_voter INTEGER;
ALTER TABLE proposals ADD COLUMN quorum_percentage REAL;
ALTER TABLE proposals ADD COLUMN passage_threshold REAL;
