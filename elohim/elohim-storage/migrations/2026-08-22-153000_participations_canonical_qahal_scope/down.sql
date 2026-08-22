-- Irreversible by design, and harmless: the pre-migration state was rows split
-- across two `h_app_id` scopes with no rule for which row belonged to which —
-- that split is the defect, not a state worth restoring. The projection is
-- re-derivable from the DHT (the participations reconcile arm), so a rollback
-- needs no data motion.
SELECT 1;
