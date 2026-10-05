-- The node's own person's sign-in account, and the sessions sign-in proves.
--
-- Source of truth: this node alone (operational, Path C). The credential is
-- private to this node: never on the DHT, a source chain, the identity
-- declaration, a sync or a projection. It is not reconstructable from
-- anywhere else; a lost secret is set again from the node's own machine
-- (`epr identity secret`), which is the deterministic floor.
--
-- One row at most: the node speaks for one person. `identifier` is the word
-- the person signs in with (a claim, shown, never used to decide or join);
-- `verifier` is an Argon2id PHC string, NULL until a secret is set.
CREATE TABLE node_account (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    identifier TEXT NOT NULL,
    display_name TEXT,
    human_id TEXT NOT NULL,
    agent_pub_key TEXT NOT NULL,
    verifier TEXT,
    secret_set_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Sessions proven by sign-in. Ephemeral: each lives thirty days at most and
-- ends at sign-out or when a new secret is set. The cookie carries a random
-- token; only its SHA-256 is kept, so this table names no live session.
-- `proven_by` says how the person proved themselves ('password' today);
-- `bound_key` is a public key the session is bound to, always NULL today,
-- kept so a later step can require a signature from it on each signing
-- request without changing this table or the routes.
CREATE TABLE node_signin_sessions (
    token_digest TEXT PRIMARY KEY NOT NULL,
    created_at_micros BIGINT NOT NULL,
    expires_at_micros BIGINT NOT NULL,
    source TEXT NOT NULL,
    proven_by TEXT NOT NULL DEFAULT 'password',
    bound_key TEXT
);
