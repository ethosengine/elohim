-- Source of truth: local (operational, Category C). Which content rows THIS
-- peer authored, written by the local write paths that author them: the HTTP
-- create and bulk-create paths (the seeder and app registration ride these),
-- local producers (observation reports, spool witnesses), and a successful
-- own-conductor commit made through `ContentService::update_via_conductor`.
--
-- A peer may mint a DHT root for a content row only when it holds local
-- authoring authority for that row (runbook invariants I1/I2: transport
-- evidence is never promoted into authority). Rows that arrive from peers --
-- the acquisition fetch, resolve-on-read, sync -- are never written here, so
-- the heal-class re-author sweeps (`reanchor_backfill`, the ghost witness)
-- hold them and leave them to adoption of the author's root. Before this a
-- receiver's backfill authored the root of a slug another peer had just
-- registered, three seconds ahead of the author (household, 2026-10-17).
--
-- Not a DHT entry and never gossiped. A lost row only holds the backfill for
-- that content until the author's root is adopted or a local write marks it
-- again; it never authorizes anything.
CREATE TABLE content_local_authorship (
    h_app_id TEXT NOT NULL,
    content_id TEXT NOT NULL,
    -- local-create | local-bulk | own-commit | pre-migration
    authored_via TEXT NOT NULL,
    marked_at TEXT NOT NULL,
    PRIMARY KEY (h_app_id, content_id)
);

-- Rows already on disk keep today's behaviour: their origin was never
-- recorded, so they are grandfathered as local. Only rows that arrive after
-- this migration are held by the receiver guard.
INSERT OR IGNORE INTO content_local_authorship (h_app_id, content_id, authored_via, marked_at)
SELECT h_app_id, id, 'pre-migration', datetime('now') FROM content;
