-- EPR Phase 2B — B.3: projector cursor table
-- Category C operational cursor; rebuildable from epr_atoms replay; no DHT anchor by design.
-- Tracks "how far has the projector advanced" per (pillar, kind) pair.
-- PRIMARY KEY is (pillar, kind) — operational C-class, not a notarized entity.

CREATE TABLE projector_cursor (
    pillar         TEXT NOT NULL,
    kind           TEXT NOT NULL,
    last_epr_cid   TEXT,         -- CID of last projected EPR atom (content-address, not DHT anchor)
    last_issued_at TEXT,         -- ISO-8601 timestamp of last projected atom
    updated_at     TEXT NOT NULL, -- ISO-8601 timestamp of last cursor advance
    PRIMARY KEY (pillar, kind)
);
