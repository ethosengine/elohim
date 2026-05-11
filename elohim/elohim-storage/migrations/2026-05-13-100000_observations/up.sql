-- Observation/Event Layer — Stage 3 of the observation-event-layer-design spec.
-- See genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md
--
-- All tables here are Category C (operational projections). The authoritative
-- form of every observation is the observer's per-observer iroh-blob log on
-- Track 2 substrate; these SQL rows are rebuildable by replaying that log.

-- One row per observation. Reconstructable by replaying the observer's iroh-blob log.
-- Source of truth: iroh-blob log (per-observer, content-addressed). Classification: C.
CREATE TABLE observations (
    observer_cid              TEXT    NOT NULL,
    log_cid                   TEXT    NOT NULL,
    log_offset                BIGINT  NOT NULL,
    observed_at               BIGINT  NOT NULL,
    seq                       BIGINT  NOT NULL,
    observation_kind          TEXT    NOT NULL,
    subject_cid               TEXT,
    subject_kind              TEXT,
    payload_json              TEXT    NOT NULL,
    observer_household_cid    TEXT,
    observer_collective_cid   TEXT,
    observer_region           TEXT,
    observer_archetype        TEXT,
    observer_compute_class    TEXT,
    signature_b64             TEXT    NOT NULL,
    PRIMARY KEY (observer_cid, log_cid, log_offset)
);

CREATE INDEX observations_by_subject_kind
    ON observations (subject_cid, observation_kind, observed_at);

CREATE INDEX observations_by_kind_time
    ON observations (observation_kind, observed_at);

CREATE INDEX observations_by_observer_seq
    ON observations (observer_cid, seq);

-- Per-observer log roster — what is the latest log root for each observer we follow.
-- Source of truth: observer's iroh-blob log. Classification: C.
CREATE TABLE observation_logs (
    observer_cid       TEXT     PRIMARY KEY,
    latest_log_cid     TEXT     NOT NULL,
    latest_offset      BIGINT   NOT NULL,
    retention_class    TEXT     NOT NULL,
    last_attested_at   BIGINT
);

-- Per-(observer, viewer) cursor — how far this viewer has projected each observer's log.
-- Source of truth: SQLite (operational). Classification: C.
-- Mirrors the existing projector_cursor / peer_inventory_cursor pattern.
CREATE TABLE observation_cursors (
    observer_cid             TEXT    NOT NULL,
    viewer_peer_id           TEXT    NOT NULL,
    last_projected_offset    BIGINT  NOT NULL,
    last_seen_at             BIGINT  NOT NULL,
    PRIMARY KEY (observer_cid, viewer_peer_id)
);

-- Audit log of verify-path queries (point-in-time diversity re-checks per spec §6.3).
-- Source of truth: SQLite (operational). Classification: C.
CREATE TABLE audit_observations (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    requested_at      BIGINT  NOT NULL,
    requester_cid     TEXT    NOT NULL,
    subject_cid       TEXT    NOT NULL,
    observation_kind  TEXT    NOT NULL,
    result_json       TEXT    NOT NULL
);
