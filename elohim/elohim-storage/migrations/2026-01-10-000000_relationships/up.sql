-- Content relationships (edges in the knowledge graph)
-- Rich metadata for graph exploration: confidence, provenance, bidirectionality
CREATE TABLE relationships (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    inference_source TEXT NOT NULL DEFAULT 'explicit',
    is_bidirectional INTEGER NOT NULL DEFAULT 0,
    inverse_relationship_id TEXT,
    provenance_chain_json TEXT,
    governance_layer TEXT,
    reach TEXT NOT NULL DEFAULT 'commons',
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Unique constraint: only one relationship of each type between source and target per app
CREATE UNIQUE INDEX idx_relationships_unique ON relationships(app_id, source_id, target_id, relationship_type);

-- Indexes for graph traversal
CREATE INDEX idx_relationships_app_id ON relationships(app_id);
CREATE INDEX idx_relationships_source ON relationships(source_id);
CREATE INDEX idx_relationships_target ON relationships(target_id);
CREATE INDEX idx_relationships_type ON relationships(relationship_type);
CREATE INDEX idx_relationships_source_type ON relationships(source_id, relationship_type);
CREATE INDEX idx_relationships_inverse ON relationships(inverse_relationship_id);
CREATE INDEX idx_relationships_inference ON relationships(inference_source);
