-- Source of truth: this peer's own evaluation choice + the DHT subgraph it
-- replays (Category C — operational). Accountable-correction contract §7.
--
-- A GENERATION = one (evaluator, PINNED policy bytes by CID) projection.
-- Per-act application is tracked per generation, so a second evaluator is never
-- suppressed by the first, and a policy change produces a NEW generation rather
-- than silently re-weighting history.
--
-- Policy bytes are pinned HERE, not read from the live registry at replay time:
-- `ManifestDebitWeightPolicy::from_registry` reads whatever the registry holds
-- now, which makes a replay non-deterministic the moment the manifest moves.
CREATE TABLE standing_generations (
    generation_id INTEGER PRIMARY KEY AUTOINCREMENT,
    evaluator_pubkey BLOB NOT NULL,
    policy_manifest_cid TEXT NOT NULL,
    -- The pinned policy itself (canonical JSON). A generation replays against
    -- these bytes forever, whatever the registry later says.
    policy_bytes TEXT NOT NULL,
    -- building | published | rebuilding
    --   building   = a fresh generation being replayed; not yet served.
    --   published  = the generation `standing_view` currently reflects.
    --   rebuilding = a published generation being replayed again. READERS SEE
    --                THIS: "rebuilding" is a visible state, not a silent gap.
    status TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    published_at TEXT
);

CREATE INDEX idx_standing_generations_evaluator
    ON standing_generations(evaluator_pubkey, status);

-- Per-generation aggregate. `standing_view` keeps its shape — ONE aggregate per
-- (evaluator, subject) — and is the PUBLISHED projection; this table is where a
-- generation accumulates while it is still `building`, so a rebuild is "built to
-- completion, then atomically published" rather than a live table being emptied
-- underneath its readers.
--
-- A subject with no ACCEPTED correction has NO ROW here. That is the
-- Unknown-vs-Neutral distinction §7 requires: readers distinguish an absent
-- aggregate (Unknown) from a zero one (Neutral), and reach eligibility consumes
-- that difference. An unaccepted allegation must leave the subject Unknown.
CREATE TABLE standing_generation_aggregate (
    generation_id INTEGER NOT NULL,
    evaluator_pubkey BLOB NOT NULL,
    subject_pubkey BLOB NOT NULL,
    debit_weight_sum INTEGER NOT NULL DEFAULT 0,
    -- Maximum INCLUDED action timestamp (micros), never the last enumerated.
    last_signal_at_micros BIGINT,
    PRIMARY KEY (generation_id, evaluator_pubkey, subject_pubkey)
);
