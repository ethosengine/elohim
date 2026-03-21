-- Source of truth: DHT (Attestation entry in imagodei DNA)
-- Classification: A (Notarized) — maps to Attestation with type=credential
CREATE TABLE steward_credentials (
    id TEXT PRIMARY KEY NOT NULL,
    presence_id TEXT NOT NULL,
    content_id TEXT NOT NULL,
    affinity_coefficient REAL NOT NULL DEFAULT 0.0,
    credential_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_steward_credentials_presence ON steward_credentials(presence_id);
CREATE INDEX idx_steward_credentials_content ON steward_credentials(content_id);

-- Source of truth: DHT (Link on Content entry in lamad DNA)
-- Classification: A (Notarized)
CREATE TABLE premium_gates (
    id TEXT PRIMARY KEY NOT NULL,
    steward_credential_id TEXT NOT NULL REFERENCES steward_credentials(id),
    steward_presence_id TEXT NOT NULL,
    gated_resource_type TEXT NOT NULL,
    gated_resource_ids TEXT NOT NULL,
    gate_title TEXT NOT NULL,
    gate_description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_premium_gates_credential ON premium_gates(steward_credential_id);
CREATE INDEX idx_premium_gates_presence ON premium_gates(steward_presence_id);

-- Source of truth: DHT (Attestation entry in imagodei DNA)
-- Classification: A (Notarized)
CREATE TABLE access_grants (
    id TEXT PRIMARY KEY NOT NULL,
    gate_id TEXT NOT NULL REFERENCES premium_gates(id),
    grantee_presence_id TEXT NOT NULL,
    contributor_presence_id TEXT,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT,
    status TEXT NOT NULL DEFAULT 'active'
);

CREATE INDEX idx_access_grants_gate ON access_grants(gate_id);
CREATE INDEX idx_access_grants_grantee ON access_grants(grantee_presence_id);
