-- EPR storage layer — Phase 2a
-- See genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md §8
--
-- Note: SQLite backend. Timestamps stored as TEXT (ISO-8601). Binary data as BLOB.
-- Source of truth: EPR atoms (self-notarized via content-derived CID + Ed25519).
-- These tables are a local-queryable projection of the P2P-native EPR substrate.

-- Primary atom table
CREATE TABLE epr_atoms (
    cid               TEXT PRIMARY KEY,
    kind              TEXT NOT NULL,
    schema_ref        TEXT NOT NULL,
    schema_key        TEXT NOT NULL,
    reach             TEXT NOT NULL,
    issued_at         TEXT NOT NULL,              -- ISO-8601 timestamp
    signer_cid        TEXT NOT NULL,
    supersedes        TEXT,                       -- FK to epr_atoms.cid; nullable; enforced app-side
    canonical_bytes   BLOB NOT NULL,
    payload_bytes     BLOB NOT NULL,
    proof_bytes       BLOB NOT NULL,              -- Ed25519 signature
    proof_algorithm   TEXT NOT NULL               -- "ed25519" for now
);

CREATE INDEX epr_atoms_kind_schema_ref_idx ON epr_atoms (kind, schema_ref);
CREATE INDEX epr_atoms_reach_idx ON epr_atoms (reach);
CREATE INDEX epr_atoms_signer_cid_idx ON epr_atoms (signer_cid);
CREATE INDEX epr_atoms_supersedes_idx ON epr_atoms (supersedes) WHERE supersedes IS NOT NULL;

-- Coupling legs (normalized FK rows, NOT a JSON column, per Integrator Compatibility Contract §4)
CREATE TABLE epr_coupling (
    epr_cid           TEXT NOT NULL REFERENCES epr_atoms(cid) ON DELETE CASCADE,
    leg               TEXT NOT NULL CHECK (leg IN ('knowledge', 'value', 'governance')),
    target_cid        TEXT NOT NULL,
    PRIMARY KEY (epr_cid, leg)
);

CREATE INDEX epr_coupling_target_cid_idx ON epr_coupling (target_cid);

-- Claims (outcome assertions) — the EPR asserts these Claim-EPRs as its outcomes
CREATE TABLE epr_claims (
    epr_cid           TEXT NOT NULL REFERENCES epr_atoms(cid) ON DELETE CASCADE,
    claim_cid         TEXT NOT NULL,
    PRIMARY KEY (epr_cid, claim_cid)
);

CREATE INDEX epr_claims_claim_cid_idx ON epr_claims (claim_cid);

-- Supersedence index (predecessor → successor, attested at revision time)
CREATE TABLE epr_supersedence (
    predecessor       TEXT NOT NULL,
    successor         TEXT NOT NULL,
    attested_by       TEXT NOT NULL,
    attested_at       TEXT NOT NULL,              -- ISO-8601 timestamp
    PRIMARY KEY (predecessor, successor)
);

CREATE INDEX epr_supersedence_predecessor_idx ON epr_supersedence (predecessor);
CREATE INDEX epr_supersedence_successor_idx ON epr_supersedence (successor);
