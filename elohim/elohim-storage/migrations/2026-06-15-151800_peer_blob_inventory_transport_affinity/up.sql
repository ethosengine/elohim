-- Per-object blob transport affinity (iroh-toggle sprint, Wave 2).
--
-- Source of truth: local (operational, Category C); NULL = Auto = default policy; reconstruction = default policy on loss
--
-- Lets a single blob declare which transport carries it. Kebab values:
-- 'auto' (== NULL), 'prefer-iroh', 'prefer-libp2p', 'iroh-only',
-- 'libp2p-only'. Consumed by http_blob_router::choose_backend to override
-- the negotiated per-request transport selection for this object only.
--
-- Purely additive: the column is NULLABLE with default NULL, which maps to
-- TransportAffinity::Auto — exactly today's behavior. The libp2p code path
-- never reads this column, so libp2p mode is unaffected.

ALTER TABLE peer_blob_inventory ADD COLUMN transport_affinity TEXT NULL;
