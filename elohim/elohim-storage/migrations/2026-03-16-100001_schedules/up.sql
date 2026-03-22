CREATE TABLE IF NOT EXISTS schedules (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    scheduled_at TEXT,
    expires_at TEXT,
    rrule TEXT,
    last_occurred_at TEXT,
    next_occurrence_at TEXT,
    occurrence_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_schedules_entity ON schedules (entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_schedules_next ON schedules (next_occurrence_at);
CREATE INDEX IF NOT EXISTS idx_schedules_scheduled ON schedules (scheduled_at);
