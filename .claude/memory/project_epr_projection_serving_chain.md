---
id: project-epr-projection-serving-chain
name: project_epr_projection_serving_chain
description: "The full chain that makes / and /lamad serve via projected-EPR bundles across doorways, and the 7 failure layers found delivering it (2026-05-30) — extract_app_context routing shadow, in_scope_of NULL backfill, spa-bundle filter, stageSpaBlobs single-doorway, EprRouter self-heal."
metadata:
  node_type: memory
  type: project
cites:
  - .claude/shifts/2026-05-30T03-15-landing-pages-both-doorways.sprint-result.md
---

How `/` (landing) and `/lamad` (Lamad LMS) actually load on a doorway, and the failure
modes hit delivering it across BOTH doorways (apex elohim.host=adam, alpha=matthew) on
2026-05-30. See sprint-result `.claude/shifts/2026-05-30T03-15-landing-pages-both-doorways.sprint-result.md`.

**The serving chain (every link must hold):**
1. Angular bundles built + uploaded as content rows with a blob: slugs `elohim-host-landing`
   (html5-app) + `lamad-spa` (spa-bundle). Uploaded by the APP pipeline's `stageSpaBlobs`
   (root Jenkinsfile) → PUT /admin/seed/blob + PATCH /db/content/{slug}.
2. project-epr REA commitments seeded (genesis `seed-projections.ts` → POST /api/v1/commitments)
   with `in_scope_of = doorway:{doorwayId}|epr:{eprId}`, scoped per doorway.
3. doorway EprRouter populated from `GET {STORAGE_URL}/db/rea_commitments?action=project-epr&doorwayId={id}`
   → expects a BARE `Vec<EprProjectionView>` JSON array. Populated at boot (one-shot) +
   a 30s self-heal refresh loop + SSE projection.registered.
4. request `/lamad` → EprRouter longest-prefix match → `dispatch_to_projected_epr` → proxy to
   `{STORAGE_URL}/apps/{epr_id}/{sub_path}` → storage slug_index resolves the blob.

**The 7 failure layers (each masked the next — cascade-unmasking):**
- **storage spa-bundle filter:** `/apps/{slug}` resolver (`load_slug_index` + `lookup_slug_blob_hash`
  in elohim-storage http.rs) hardcoded `content_format=="html5-app"` → excluded `spa-bundle`
  rows ("App not found"). Also indexed by inner `content.slug` not row `id` (lamad-spa's inner
  slug is "lamad" ≠ id). Fix: serve html5-app + spa-bundle; index by id AND inner slug.
- **extract_app_context routing shadow (THE big one):** `/db/{single-segment}` paths NOT in
  `legacy_prefixes` (http.rs ~2800) fall into the new-route branch (`/db/{h_app_id}`) and
  default `resource_path` to "stats" → served by `handle_db_stats` → empty `DbStats
  {contentCount,uniqueTags}`. So `/db/rea_commitments` returned DbStats, the real handler was
  DEAD CODE, the doorway's `Vec<EprProjectionView>` decode failed ("error decoding response
  body"), router stayed empty. Fix: add rea_commitments + the mishpat projection routes
  (gate-decisions, gate-decision-challenges, challenge-outcomes, elohim-reputation) to legacy_prefixes.
- **in_scope_of = NULL:** rows authored before the camelCase-deser fix (CreateReaCommitmentInput
  `#[serde(rename_all="camelCase")]`) reached the deployed binary persisted NULL scope.
  `find_active_projections` filters `in_scope_of LIKE '%doorway:..|%'` and `NULL LIKE _` is
  NEVER true → `[]`. A reseed could NOT self-heal: content-addressed ids → `upsert_with_anchor`'s
  existing-row branch updated ONLY `dht_anchor_hash`. Fix: `upsert_with_anchor` backfills
  in_scope_of/note/metadata on existing rows WHEN input.in_scope_of.is_some() (guarded so the
  state-only update_state path never clobbers) → genesis reseed repairs the NULL rows.
- **stageSpaBlobs single-doorway:** seeded ONE branch-resolved doorway (dev→alpha only); adam
  (apex) got neither bundle bytes nor blobHash, and blobs do NOT auto-replicate P2P. Fix: loop
  BOTH alpha+apex backends. (Matches the Jenkinsfile's own "projected by doorway-A + doorway-B"
  comment; impl had diverged.)
- **EprRouter never self-healed:** boot-fetch is one-shot; an empty router stays empty until a
  doorway restart. Fix: 30s periodic refresh loop (DOORWAY_EPR_REFRESH_SECS) calling
  fetch_projections_from_storage + replace_all on success.
- **apex DOORWAY_ID/seed drift:** apex doorway `DOORWAY_ID=apex-elohim-host` (deliberate, per
  manifests/doorway/alpha-b.yaml) but seed scoped apex rows to bare `elohim-host`. Fix: seed
  apex-elohim-host (the seed was the drift).
- **(prereq) SQLite busy_timeout crashloop** — see [[project_alpha_edge_deploy_debugging_landmarks]].

**GOTCHAS (durable):**
- storage `/health` does NOT acquire a diesel connection → a pod can be Ready while its DB
  layer is jammed. Probe `/db/rea_commitments?action=project-epr` directly.
- `{contentCount,uniqueTags}` is `DbStats` (db/mod.rs) = the `/db/stats` shape; seeing it from a
  non-stats `/db/{x}` route means extract_app_context shadowed `{x}` to "stats".
- `/db/rea_commitments` returns a BARE array by contract (doorway decodes `Vec<EprProjectionView>`);
  the `{items,count}` wrapper used by other /db list routes would break the decode.

Validated end-to-end on ALPHA (alpha/lamad=200 "Lamad — Learning Platform", alpha/=200 landing);
apex follows the identical path once the genesis reseed propagates the repaired rows to adam.
See also [[project_alpha_edge_deploy_debugging_landmarks]] (CI/Loki/orchestrator landmarks:
sccache NUL clippy probe, two-commit orchestrator collision, empty-[build:all] no-op).
