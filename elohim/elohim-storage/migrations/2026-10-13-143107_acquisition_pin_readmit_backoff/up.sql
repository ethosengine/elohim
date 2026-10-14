-- Re-admission backoff state for a retired acquisition pin.
-- Source of truth: local (agent-scoped device pin, Category B — same as the row).
--
-- A retired pin was re-admitted on a flat 6 h cooldown and spent its whole retry
-- budget again each time, forever (alpha matthew 2026-10-13: 1103 retired pins,
-- 1063 for `e2e-<uuid>` heads no peer holds, re-probed four times a day). The
-- hold now doubles per consecutive retirement, which needs the count to survive
-- a restart — an in-memory count would reset the ladder on every roll.
--
--   consecutive_retirements  retirements in a row with no fetch and no peer
--                            inventory naming the head between them. 0 = never
--                            retired, or the ladder was reset.
--   retired_at               when the pin was last retired — the ONLY clock a
--                            hold is computed from. Written by retirement and
--                            by nothing else (`updated_at` is stamped by
--                            unrelated bookkeeping, e.g. set_commitment_cid).
--   held_demand_count        read-misses recorded against the pin while it sat
--                            in `retired` or `removed`. On a retired pin demand
--                            caps the hold at its first rung; on a removed pin
--                            it is only counted. Cleared on re-admission.
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

-- A row already `retired` has been retired at least once: start it on the first
-- rung, with the retirement time it has been serving (`updated_at`, which the
-- retirement write stamped), so its next failed re-probe steps to 12 h instead
-- of repeating 6 h.
UPDATE acquisition_pins
   SET consecutive_retirements = 1, last_outcome = 'exhausted', retired_at = updated_at
 WHERE status = 'retired';
