CREATE TABLE IF NOT EXISTS votes (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    position TEXT NOT NULL,
    reason TEXT,
    anonymous INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(proposal_id, human_id)
);

ALTER TABLE proposals ADD COLUMN voting_anonymous INTEGER NOT NULL DEFAULT 0;
