-- Category C (operational). The feedback discovery sweep's RETIREMENT evidence:
-- a subscription member that completed COLD_AFTER_CLEAN_SWEEPS clean sweeps in
-- generation_id. Written when the member goes Cold, deleted when anything
-- re-arms it, and read once at boot so a restart does not re-sweep a settled
-- set. Keyed to the generation: a new generation has no rows, so every member
-- is swept again before it can publish.
CREATE TABLE feedback_subscription_settled (
    member_kind TEXT NOT NULL,
    member_key TEXT NOT NULL,
    generation_id INTEGER NOT NULL,
    settled_at TEXT NOT NULL,
    PRIMARY KEY (member_kind, member_key)
);
