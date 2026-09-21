---
id: "backlog-mesh-start-does-not-read-household-environment"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "`just mesh start` never sources the household's own environment.sh — an operator's MESH_HAPP_PATH pin silently drops on a plain-shell recast"
slug: "mesh-start-does-not-read-household-environment"
written: "2026-09-21"
author: "serving-edge household-acceptance campaign, 2026-09-20/21 review"
status: "open"
priority: "medium"
jobs: [elohim-app, elohim-genesis]
cluster: "mesh-prologue-cast-and-env-gaps"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [mesh, harness, environment, household-dowell, coordinator-pin, design-question]
---

**The fact.** `genesis/local-dev/household-dowell/environment.sh` is an operator-curated file whose
own header says to source it "before `just mesh start`, `wait`, `prologue`, or tests" — it pins
`STORAGE_BIN`, `DOORWAY_BIN`, `MESH_HAPP_PATH` (the good, post-cure `.happ` bundle), `TMPDIR`, `PATH`,
`HOLOCHAIN_BIN`, `HC_BIN_DIR` (seven exports total). No `just` recipe or script — `hc-mesh.sh`, the
`justfile`, `hc-mesh-prologue.sh` — sources it automatically; a grep for the file's own path across
`.sh`/justfile sources in the tree finds zero hits. On 2026-09-20 a `MESH_RESET` recast ran from a
plain shell that deliberately wanted a HEAD-built storage binary (so leaving `STORAGE_BIN` unsourced
was correct), but `MESH_HAPP_PATH` rode along in the same file and dropped with it: the recast fell
back to the default workdir `elohim.happ`, frozen at 2026-09-14 02:51, ~2.5 hours before the
coordinator cure (`8ebce05a3`) it needed.

**Evidence.** `genesis/local-dev/household-dowell/environment.sh` (full contents, seven exports, read
2026-09-21). `hc-mesh.sh:331`: `HAPP_PATH="${MESH_HAPP_PATH:-$HAPP_WORKDIR/elohim.happ}"` — the only
place `MESH_HAPP_PATH` is consumed, as an env var, never as a file to read. Commit `420a47648`'s own
message states it plainly: "The household's own environment.sh pins artifacts/elohim.happ, which has
the cure; the recast that caused this simply never sourced it." Root mechanism and byte-level proof:
`FINDING-election-ordering-503.md` §3.

**Why it matters.** `420a47648` (landed the same night) adds a freshness preflight that refuses or
repacks a stale `.happ` relative to its own tracked `.dna`/wasm sources — real protection against
silent staleness — but it does not make the launcher read `environment.sh`, so an explicit operator
pin (a deliberately *good*, non-default bundle chosen for reasons a freshness check can't infer) can
still be dropped by whoever starts the household without remembering to source the file. The
freshness preflight and the unread pin are two different gaps; this item is the one still open.

**Smallest next step — a decision, not a patch.** Two honest resolutions: (1) `just mesh start` /
`wait` / `prologue` read the household's own `environment.sh` (when `MESH_DIR` resolves to a household
that has one) before falling back to defaults, so a household's declared build inputs are
self-enforcing rather than memory-dependent; or (2) the file should not exist as a "remember to source
me" convention at all — fold its pins into a per-household config `hc-mesh.sh` already reads by path
keyed off `MESH_DIR`, removing the human-memory step entirely. Either answer closes the hole; leaving
the convention exactly as documented ("source me first") does not.

**Links.** Evidence: `genesis/a2o/reports/recovery/serving-edge-20260920/FINDING-election-ordering-503.md`
§3. Commit: `420a47648` (freshness preflight — adjacent, does not close this). Habit:
`elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`. Sibling:
`genesis/data/timeline/backlog/election-ordering-503-retryable-but-permanent.md` (the symptom this
cause produced).
