DROP INDEX IF EXISTS idx_collective_participations_member_cid;
DROP INDEX IF EXISTS idx_collectives_cid;

ALTER TABLE collective_participations DROP COLUMN dht_anchor_hash;
ALTER TABLE collective_participations DROP COLUMN member_kind;
ALTER TABLE collective_participations DROP COLUMN member_cid;

ALTER TABLE collectives DROP COLUMN slug;
ALTER TABLE collectives DROP COLUMN collective_cid;
