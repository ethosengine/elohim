-- Economic events for hREA/ValueFlows integration
-- Tracks value flows: content engagement, recognition transfer, stewardship
CREATE TABLE economic_events (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'shefa',
    action TEXT NOT NULL,
    provider TEXT NOT NULL,
    receiver TEXT NOT NULL,

    -- Resource specification
    resource_conforms_to TEXT,
    resource_inventoried_as TEXT,
    resource_classified_as_json TEXT,

    -- Quantities
    resource_quantity_value REAL,
    resource_quantity_unit TEXT,
    effort_quantity_value REAL,
    effort_quantity_unit TEXT,

    -- Timing
    has_point_in_time TEXT NOT NULL,
    has_duration TEXT,

    -- Process context
    input_of TEXT,
    output_of TEXT,

    -- Lamad-specific tracking
    lamad_event_type TEXT,
    content_id TEXT,
    contributor_presence_id TEXT,
    path_id TEXT,

    -- Event chain
    triggered_by TEXT,
    state TEXT NOT NULL DEFAULT 'recorded',

    -- Metadata
    note TEXT,
    metadata_json TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for event queries
CREATE INDEX idx_event_app_id ON economic_events(app_id);
CREATE INDEX idx_event_provider ON economic_events(app_id, provider);
CREATE INDEX idx_event_receiver ON economic_events(app_id, receiver);
CREATE INDEX idx_event_action ON economic_events(action);
CREATE INDEX idx_event_lamad_type ON economic_events(lamad_event_type);
CREATE INDEX idx_event_content ON economic_events(content_id);
CREATE INDEX idx_event_presence ON economic_events(contributor_presence_id);
CREATE INDEX idx_event_path ON economic_events(path_id);
CREATE INDEX idx_event_time ON economic_events(has_point_in_time);
CREATE INDEX idx_event_state ON economic_events(state);
