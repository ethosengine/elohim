---
name: project_local_pair_failover_validation_rail
title: Local pair-failover validation rail
description: How to validate doorway-failover / saga ch04 locally on `just mesh` (two doorways + 3 peers + mongod) before any [build:edge] — the seeding order, the shed drill, and the traps that cost time on 2026-08-21.
metadata:
  type: project
---

`just mesh start` = doorway A :8888 (matthew-primary, id `alpha-elohim-host`) + doorway B :8889 (jessica-primary,
id `apex-elohim-host`, the elohim.host stand-in) + matthew/jessica/james storage (:8090-8092). a2o resolves
`alpha-A`→`E2E_DOORWAY_ALPHA`, `elohim.host`→`E2E_DOORWAY_B`, so `@concern:doorway-failover` runs unchanged:
`cd genesis/a2o && E2E_DOORWAY_ALPHA=http://localhost:8888 E2E_DOORWAY_B=http://localhost:8889 npx cucumber-js --tags '@concern:doorway-failover or @concern:saga-04-doorway-serves'`.

Provisioning order that actually yields `/` 200 on BOTH doorways (2026-08-21):
1. `STORAGE_BIN=<pool dev/debug slot> just mesh start` — no release storage binary exists in the pool; the
   mesh starts doorways BEFORE storage, so the EPR router/warm-stream boots empty → **restart the two
   doorways after storage is up** (`fuser -k 8888/tcp 8889/tcp; just mesh start` is idempotent per component).
2. Seed the landing row via BOTH doorways (jessica has no row otherwise — `--ids` seeding never reaches the
   conductor, no anchor, no gossip): `cd genesis/seeder && DOORWAY_URL=http://localhost:8888 STORAGE_URL=http://localhost:8090 HOLOCHAIN_ADMIN_URL=ws://localhost:4444 npx tsx src/seed.ts --ids=elohim-host-landing` (then 8889/8091/4454). Post-flight "paths expected 8" failure is scope noise.
3. `--ids` seeding SKIPS the bindings/projections phases → `npx tsx src/seed-operator-bindings.ts` then
   `npx tsx src/seed-projections.ts` (defaults cover both `alpha-elohim-host` and `apex-elohim-host`). Router
   refresh is 30 s; until then `/` sheds in <1 ms with "Projection cache: empty".
4. Stage EVERY pillar bundle (landing AND lamad-spa — `/lamad` 404s otherwise) exactly as CI does, PER HOST: upload+PATCH through A, then `DECLARE_ONLY=1 SOURCE_DOORWAY_URL=http://localhost:8888 … stage-spa-blob.sh <dist> <slug> http://localhost:8889` to carry the head record to B. After a full 3,400-row seed, gossip-only convergence of a freshly PATCHed head sat un-adopted >8 min (jessica's sweep was busy adopting thousands of other heads — the head-plane cost, live); declare-only converged it in <10 s. Note: each zip run mints a NEW blob hash (timestamps), so never re-upload to B — propagate A's head. Original landing step: `DO_PATCH=1 STORAGE_API_KEY_ADMIN= bash scripts/ci/stage-spa-blob.sh app/elohim-app/dist/elohim-app/browser elohim-host-landing http://localhost:8888` (18 MB; replicated to jessica within ~1 min; james serves it reassembled from 18 shards).

**Why:** first local attempt lost ~20 min to (a) `pkill -f 'elohim-storage.*--http-port'` killing the calling
shell (pattern matched its own cmdline — use `fuser -k <port>/tcp`); the SAME trap from the script's side: `just mesh stop`
kills by `pgrep -f 'elohim-storag[e]'` etc., so a shell whose command line contains `elohim-storage`/`doorway`/`holochain` (e.g.
`STORAGE_BIN=…/elohim-storage just mesh stop`, or a `pgrep -af 'elohim-storage …'` in the same command) gets SIGTERM'd (exit 144) — run
`just mesh stop` ALONE in its own command, then start/prologue in another, (b) the boot-order empty router, (c) the
projections phase silently absent under `--ids`.

**How to apply:** shed drill = stock a knowledge read, `fuser -k 8090/tcp`, expect 200 amber (not 503) on
`/db/content/elohim-host-landing` and 503 + `x-elohim-freshness-required: green` on `/head-record`, restore,
green returns within one breaker cooldown (~30 s) in the same process — pin the doorway pid, other sessions
restart doorways on the shared mesh. Baseline before any doorway change, then rebuild the doorway debug slot and restart only the
doorways. Herd check: `herd-hammer` 40-concurrent straight at storage :8090 after a storage restart — the fix
(9be1f84a7) shows as exactly ONE "extracted + cached" line and zero `os error 39`. Since cb0e182b4 (2026-08-21) the mesh runs a loopback mongod (needs `mongod` on PATH or ~/bin — in the
udi-plus image from che-devworkspaces 9409696), so the warm-shell archive wiring (18a65fd0d) IS desk-provable:
look for `archive_backed=true` + `hydrated=N>0` in both doorway boot logs; without the binary the doorways run
archive-less and `mesh status` says so.
Related: [[project_local_stack_dht_anchor_gap]], [[project_freshness_graded_by_declared_stakes]].
