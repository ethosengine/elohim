-- Re-admission backoff state for a retired acquisition pin.
-- Source of truth: local (agent-scoped device pin, Category B — same as the row).
--
-- A retired pin was re-admitted on a flat 6 h cooldown and spent its whole retry
-- budget again each time, forever (alpha matthew 2026-10-13: 1103 retired pins,
-- 1063 for `e2e-<uuid>` heads no peer holds, re-probed four times a day). The
-- hold now doubles per consecutive retirement — 6 h, 12 h, then 24 h for good
-- — which needs the count to survive a restart: an in-memory count would reset
-- the ladder on every roll.
--
--   consecutive_retirements  retirements in a row with no fetch and no peer
--                            inventory naming the head between them. 0 = never
--                            retired, or the ladder was reset.
--   retired_at               when the pin was last retired — the ONLY clock a
--                            hold is computed from. Written by retirement and
--                            by nothing else (`updated_at` is stamped by
--                            unrelated bookkeeping, e.g. set_commitment_cid).
--   held_demand_count        read-misses the demand auto-pin ADMITTED against the
--                            pin while it sat in `retired` or `removed`. On a
--                            retired pin that makes it due on the next
--                            reconcile tick; on a removed pin it is only
--                            counted. Cleared on re-admission.
--   read_since_retired       1 when a read-miss the auto-pin did NOT admit
--                            (throttled, window full, ceiling) named the pin
--                            while it was retired. Caps its hold at the first
--                            rung (6 h from retired_at). Cleared on re-admission.
--   readmit_after            when a retired pin is due back: the end of its own
--                            hold, pulled in by a read (to retired_at for an
--                            admitted read, retired_at + 6 h otherwise). The
--                            reconcile tick selects due rows by it, through
--                            idx_acquisition_pins_status_readmit_after, so a
--                            pass at rest reads nothing.
--   last_outcome             'exhausted' | 'fetched' | NULL (no round concluded).
--   round                    bumped every time the pin is (re-)activated. The
--                            in-memory retry state is keyed on it, and a
--                            retirement names the round it measured, so a stale
--                            exhaustion can never retire a re-activated pin.
ALTER TABLE acquisition_pins ADD COLUMN consecutive_retirements INTEGER NOT NULL DEFAULT 0;
ALTER TABLE acquisition_pins ADD COLUMN held_demand_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE acquisition_pins ADD COLUMN last_outcome TEXT;
ALTER TABLE acquisition_pins ADD COLUMN retired_at TEXT;
ALTER TABLE acquisition_pins ADD COLUMN round INTEGER NOT NULL DEFAULT 0;
ALTER TABLE acquisition_pins ADD COLUMN read_since_retired INTEGER NOT NULL DEFAULT 0;
ALTER TABLE acquisition_pins ADD COLUMN readmit_after TEXT;
CREATE INDEX idx_acquisition_pins_status_readmit_after
    ON acquisition_pins(status, readmit_after);

-- A row already `retired` has been retired at least once: start it on the first
-- rung, with the retirement time it has been serving (`updated_at`, which the
-- retirement write stamped), so its next failed re-probe steps to 12 h instead
-- of repeating 6 h.
UPDATE acquisition_pins
   SET consecutive_retirements = 1, last_outcome = 'exhausted', retired_at = updated_at,
       readmit_after = strftime('%Y-%m-%dT%H:%M:%SZ', updated_at, '+6 hours')
 WHERE status = 'retired';
