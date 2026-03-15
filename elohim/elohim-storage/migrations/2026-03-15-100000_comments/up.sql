CREATE TABLE comments (
    id TEXT PRIMARY KEY NOT NULL,
    app_id TEXT NOT NULL DEFAULT 'lamad',
    content_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    body TEXT NOT NULL,
    reach TEXT NOT NULL DEFAULT 'close',
    governance_state TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_comments_content_id ON comments (content_id);
CREATE INDEX idx_comments_human_id ON comments (human_id);
CREATE INDEX idx_comments_app_id ON comments (app_id);
