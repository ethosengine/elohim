-- Source of truth: Holochain DHT (imagodei `Membership` entries); this table is
-- a read-optimized projection. Classification: A.
--
-- Converge `collective_participations` onto its CANONICAL `h_app_id` scope.
--
-- The column's own DDL default has always been 'qahal'
-- (2026-01-08-000000_initial), but two writers disagreed with it and with each
-- other: account-import wrote 'qahal' while the `MembershipProjected` signal
-- projector and the legacy `POST /db/collectives/{id}/participants` route wrote
-- 'lamad' (the operating content scope). Readers split the same way, so rows
-- written by one surface were invisible to the other — the empty
-- `{"items": [], "count": 0}` the household-formation E2E measured on alpha
-- 2026-08-20. `db::context::PARTICIPATIONS_HAPP_ID` now owns the scope and the
-- db-layer functions take no AppContext at all, so no new straggler can appear;
-- this migration moves the ones already on disk.
--
-- Safe under `UNIQUE(h_app_id, collective_id, human_id)`: a straggler whose
-- (collective_id, human_id) pair ALREADY exists under the canonical scope would
-- collide on the UPDATE, so it is merged and dropped first. Nothing is
-- destroyed that the canonical row does not already carry — and the row is
-- re-derivable from the DHT by the participations reconcile arm regardless.

-- (1) Carry DHT provenance from a straggler onto the canonical row it collides
--     with, when the canonical row does not already have it. `dht_anchor_hash`
--     is the reconcile identity; losing it would drop the row out of the
--     cross-peer inventory.
UPDATE collective_participations AS q
   SET dht_anchor_hash = (
         SELECT s.dht_anchor_hash FROM collective_participations s
          WHERE s.h_app_id <> 'qahal'
            AND s.collective_id = q.collective_id
            AND s.human_id = q.human_id
            AND s.dht_anchor_hash IS NOT NULL
          LIMIT 1),
       member_cid = COALESCE(q.member_cid, (
         SELECT s.member_cid FROM collective_participations s
          WHERE s.h_app_id <> 'qahal'
            AND s.collective_id = q.collective_id
            AND s.human_id = q.human_id
            AND s.dht_anchor_hash IS NOT NULL
          LIMIT 1))
 WHERE q.h_app_id = 'qahal'
   AND q.dht_anchor_hash IS NULL
   AND EXISTS (
         SELECT 1 FROM collective_participations s
          WHERE s.h_app_id <> 'qahal'
            AND s.collective_id = q.collective_id
            AND s.human_id = q.human_id
            AND s.dht_anchor_hash IS NOT NULL);

-- (2) Drop stragglers whose canonical twin now exists (their truth is merged).
DELETE FROM collective_participations
 WHERE h_app_id <> 'qahal'
   AND EXISTS (
         SELECT 1 FROM collective_participations q
          WHERE q.h_app_id = 'qahal'
            AND q.collective_id = collective_participations.collective_id
            AND q.human_id = collective_participations.human_id);

-- (3) Move the rest onto the canonical scope.
UPDATE collective_participations SET h_app_id = 'qahal' WHERE h_app_id <> 'qahal';
