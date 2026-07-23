-- One-time canonicalization of pre-reconciliation reach values (spec §7.5 data-aware migration).
-- Order matters: 'public' (old top rung) must move to 'commons' BEFORE any legacy value maps INTO 'public'.
UPDATE content SET reach = 'commons'  WHERE reach = 'public';
UPDATE content SET reach = 'public'   WHERE reach IN ('district', 'federated');
UPDATE content SET reach = 'self'     WHERE reach = 'personal';
UPDATE content SET reach = 'trusted'  WHERE reach IN ('household', 'local');
UPDATE content SET reach = 'familiar' WHERE reach = 'neighborhood';
UPDATE content SET reach = 'community' WHERE reach = 'collective';
UPDATE content SET reach = 'intimate' WHERE reach = 'invited';
