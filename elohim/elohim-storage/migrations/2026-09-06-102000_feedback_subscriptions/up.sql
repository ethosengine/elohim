-- Source of truth: local operator/steward intent plus discovery (Category C —
-- the set is rebuildable from what this peer stewards, and re-discoverable).
-- Accountable-correction contract §3.
--
-- Two member kinds, because acceptance vouches link from the CORRECTION action,
-- not from the content action (`TargetToFeedbackSignal` with the correction as
-- base). A peer that only subscribed to content targets would discover the
-- correction and never its acceptance.
CREATE TABLE feedback_subscriptions (
    -- content-target | correction-action
    member_kind TEXT NOT NULL,
    -- Base action hash the link enumeration runs from.
    member_key TEXT NOT NULL,
    origin_dna_hash TEXT NOT NULL,
    -- Where this member came from: steward | discovered | notified
    source TEXT NOT NULL DEFAULT 'steward',
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_visited_at TEXT,
    visit_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (member_kind, member_key)
);

-- The persisted ROTATION position (§3). Not a high-water cursor over DHT
-- history — link enumeration is unordered and a late link must still be picked
-- up. This is a fairness cursor over the SUBSCRIPTION SET: it advances on
-- failure as well as success, so N members under a per-tick budget of B are
-- each visited within ceil(N/B) ticks. The reference watcher's
-- `take(MAX_CHANNELS_PER_SWEEP)` starves the ninth member; that shape is not
-- copied.
CREATE TABLE feedback_rotation_cursor (
    id INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    -- The last member visited, in the (member_kind, member_key) total order.
    -- The next tick resumes at the smallest key strictly greater than this,
    -- wrapping to the start. Storing the KEY rather than an offset keeps the
    -- rotation fair when members are added or removed between ticks.
    cursor_kind TEXT,
    cursor_key TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
INSERT INTO feedback_rotation_cursor (id, cursor_kind, cursor_key, updated_at)
    VALUES (1, NULL, NULL, datetime('now'));
