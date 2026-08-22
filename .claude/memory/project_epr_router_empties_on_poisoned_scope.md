---
index: false
name: project_epr_router_empties_on_poisoned_scope
title: EprRouter empties on one poisoned scope row
description: "One poisoned scope row empties EprRouter (Welcome at /, 404 /lamad): fail-closed collect + stale-binary array-wrap; resolvers degrade per-row (f38be2635)."
metadata: 
  node_type: memory
  type: project
  originSessionId: c1085239-d5b2-4b3c-874d-1a74fcacf87f
---

Post-rebuild-all routing "regression" (elohim-genesis #1104/#1105, 2026-06-07):
the doorway served the operator dashboard (title "Welcome") at `/` and 404'd all
`/lamad` deep links. Root cause was NOT the recent EPR Slice-2/3 dispatch commits
(the obvious suspects) — it was data + a fail-closed resolver.

**The poison class:** a `project-epr` `rea_commitments` row had
`in_scope_of = ["doorway:X|epr:Y"]` (JSON-array-wrapped) instead of the canonical
bare pipe-string `doorway:X|epr:Y`. The DHT `Commitment` entry always stores
`in_scope_of_json` as a JSON array (zome `content_store`); the storage replay path
must unwrap it via `first_or_none` (fixed in 43951281f, 2026-05-26). A STALE
binary running during the `IfNotPresent`/`Always` stale-image window
([[feedback_ci_pull_policy_always_freshness]]) replayed the array verbatim into
the SQLite column. The seeder write-path was already correct (bare string,
66f16ab5e) — so this is a runtime-replay poison, not a seed bug.

**The amplifier (the real bug):** `find_active_projections`
(`elohim/elohim-storage/src/db/rea_commitments.rs`) used
`.map(commitment_to_projection_view).collect::<Result<Vec,_>>()` — fail-closed.
The `in_scope_of LIKE '%doorway:..|%'` filter still MATCHED the wrapped string, so
the one bad row entered the set and its `parse_projection_scope` error failed the
WHOLE `collect()`. The doorway's `fetch_projections_from_storage` →
`EprRouter.replace_all` then saw nothing → empty router (sitemap.xml empty,
generation 0) → `/` falls to `/threshold`, every pillar mount 404s.

**Fix (f38be2635):** (1) `parse_projection_scope` heals leading-`[` legacy shapes
on read; (2) `find_active_projections` degrades per-row (`filter_map` +
`tracing::warn!`). General lesson: **Category-A projection resolvers that feed a
routing/index table MUST degrade per-row, never fail-closed** — one malformed row
must cost one projection, not the whole surface. This is the second EprRouter
wipe; incident #1 was the NULL-scope era (same function's LIKE-excludes-NULL note).

**Recovery without a code deploy:** reseeding `project-epr` commitments writes the
bare string back via `upsert_with_anchor` (existing-row branch), which even an
un-patched binary can parse. The patched binary heals on read with no reseed.

**Diagnostic shortcut:** `curl …/db/rea_commitments?action=project-epr&doorwayId=<id>`
returning `Scope missing 'doorway:' prefix: [...]` + an empty `/sitemap.xml` is the
signature. Don't chase the dispatch commits.

Related: [[project_local_stack_dht_anchor_gap]],
[[project_resilience_snapshot_humans_junction]], [[feedback_ci_pull_policy_always_freshness]].
