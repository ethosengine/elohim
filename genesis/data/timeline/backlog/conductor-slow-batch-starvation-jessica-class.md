---
id: "backlog-conductor-slow-batch-starvation-jessica-class"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Conductor slow-batch starvation (jessica-class): circuit breaker sheds the content heal leg before per-item classification, starving every adopt/contest arm"
slug: "conductor-slow-batch-starvation-jessica-class"
written: "2026-08-15"
author: "claude (shift 2026-08-15T00-54-verify-spin-discharge-live, iteration 4b investigation)"
status: "open"
ci_status: "open"
priority: "medium"
tags: [dataplane, conductor, circuit-breaker, heal-pacing, starvation, saga-06-heads-converge, jessica]
cites:
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - genesis/data/timeline/backlog/susan-conductor-ws-dead-heal-pacing-blind-to-instant-errors.md
  - genesis/data/timeline/backlog/spin-divergent-undeclared-rows-block-a-convergence.md
---

# Conductor slow-batch starvation (jessica-class)

**Symptom (live, 2026-08-15 01:25–04:30Z, post-edge-#1350 restart):** jessica's
known_divergent{content} stayed flat at 13 for 3+ hours while matthew (13→2) and
james (14→1) drained the same SPIN-class rows. Not an algebra failure — the SPIN
discharge arm (9132e6d28) is proven reachable on jessica's vantage (one
Refreshed classification at 02:47:49Z).

**Mechanism (ci-investigator, Loki + code, quoted evidence in the shift
journal):** jessica's conductor times out the batched head-resolve calls (15s
per-attempt, fanout=2, batch=8) → `OPENED the unresponsive-conductor circuit`
fired ~28× in the window → the content leg is shed BEFORE per-item Answers
exist → all 5 adopt-candidate branches in `heal_content`
(projection_reconcile.rs:3696–4008) are unreachable (each requires an Answer)
→ `adopt_deferred_heads` receives zero candidates and early-returns before its
metric (adopt_sweep_total=0 across 16 reconcile sweeps). Comparable CALL-level
batch-failure counts on matthew (110 vs 99) but matthew still lands ~10× the
successful classifications (129 vs 13) — the pods are not on the same effective
clock.

**Open questions:**
- WHY jessica's conductor is slower per-batch (CPU/mem/DB contention vs
  post-restart catch-up load). Pyroscope now ingests the fleet (profiler eyes
  open 2026-08-14) — a CPU profile comparison jessica-vs-matthew during a sweep
  window is the natural first probe.
- Whether the circuit's shed-whole-leg behavior should degrade to
  shed-remaining-batch (partial progress per sweep instead of none) — relates
  to the heal-pacing-blind-to-instant-errors thread on susan.
- Whether starved pods self-recover when catch-up completes (watch jessica
  after the fleet quiesces) — if yes, this is a transient-churn class; if no,
  a standing per-pod outage class invisible to the liveness table (an arm can
  be modeled live and scheduled never — the 2026-08-03 lesson, new variant:
  modeled live and STARVED always).

**Probe that would make this class visible without Loki archaeology:** a
counter for circuit-open events per stream
(e.g. `elohim_projection_reconcile_circuit_opened_total{stream}`) + a gauge for
consecutive-sheds; today the circuit is WARN-log-only.

Claimable by any agent; read the shift journal
(.claude/shifts/2026-08-15T00-54-verify-spin-discharge-live.journal.md,
iteration 4b) for the full quoted evidence before starting.

## RECLASSIFIED 2026-08-15 ~11:40Z — same root cause as the fleet-wide contest failure volume

Overnight id-level attribution (shift iteration 7-8) unified this with matthew's
contest failures: the throttle is the conductor admission ceiling
(`conductor_admission.rs` — `content_store`, `class=interactive`,
`capacity = max(2*cpus,8) - CONDUCTOR_RESERVE(3) = 5` on the ~4-CPU household
pods, 5s interactive shed), hit identically on matthew/jessica/james with the
verbatim error string. Jessica's "slower conductor" is the same ceiling seen
from the batch head-resolve side. Levers, ranked: (1) DEMAND — head-plane L1–L3
batching (arch-dataplane-refactor-backlog / DATAPLANE-SDK-PATH critical-path #2)
collapses per-id round-trips; tonight's evidence gives it hard numbers
(106 shed/timeout failure lines in ~7h on one pod). (2) SUPPLY —
`ELOHIM_CONDUCTOR_PERMITS` env bump (OPERATOR ceiling decision; cautioned:
11/27 declare errors were conductor-side websocket timeouts, so the conductor
itself saturates — over-admission trades shed-at-gate for timeout-in-flight).
(3) Class audit — contest declares ride `class=interactive` (5s hold burns
permit time); whether they belong in `Background` (1s defer, cheap retry) is a
bounded code question.

## Ghost-trio remediation prep (2026-08-15; commands are not yet run)

This section is an operator runbook, not evidence that any live repair ran. The
shift classified two exact FCT ids and one member of the
`scenario-public-observer-parent-*` family as real-seed `no_record` candidates.
There is a source-record gap for the third id: iteration 8 says only
"one scenario-public-observer-parent doc" (journal lines 129–134), and the
morning menu repeats that family-only wording (line 156). No exact suffix occurs
anywhere in that journal. **Do not choose one of the 39 matching scenario seed
files by guess.** Recover the investigator's omitted id first, then replace the
held row and command below.

### Declare-cycle membership: none of the trio is automatic

`stage-spa-blob.sh` is capable of declaring any one caller-supplied slug:

> `Usage: stage-spa-blob.sh <dist-dir> <slug> <doorway-epr-url> [kind]`
> (`scripts/ci/stage-spa-blob.sh:10`)

In `DECLARE_ONLY=1`, it resolves that slug's `headActionHash` from the source
doorway (`:44–55`), asks for the matching serialized record (`:86–98`), and
POSTs the declaration to the target doorway's
`/db/content/${SLUG}/canonical-head` (`:132–136`). That makes the script a
usable *manual* channel for arbitrary ids; it does not define the automatic
membership set.

The caller defines that set. The app pipeline's complete `bundles` literal is:

> `slug: "elohim-host-landing"` (browser and server) and
> `slug: "lamad-spa"` (browser and server)
> (`Jenkinsfile:511–515`)

Phase 2 iterates only that literal (`Jenkinsfile:527–535`), and
`authorHeadOnce` passes only `bundle.slug` into `DECLARE_ONLY`
(`Jenkinsfile:342–352`). Therefore neither FCT id nor any
`scenario-public-observer-parent-*` id belongs to the normal declare-cycle.
An ordinary app deploy will not re-declare them.

### Seed provenance and prepared repair per id

#### `fct-bible-galatians-6-4-5`

- **Seed JSON:**
  `genesis/data/lamad/content/fct-bible-galatians-6-4-5.json`.
- **Authored source:** the JSON says
  `"sourcePath": "fct/Module 13 - Helping People Thrive - A Christian Definition .md"`
  (line 8), which resolves under `genesis/docs/content/`. The authored verse is
  present at that markdown file's line 141. This is the documented
  `genesis/docs/content/` → `elohim-import` → `genesis/data/lamad/content/`
  transformation pipeline, followed by the deterministic genesis seeder.
- **DECLARE_ONLY coverage:** **no**; its id is absent from the four-entry
  `Jenkinsfile:511–515` bundle set.
- **Prepared targeted re-seed/re-author command:** run the helper in the next
  subsection with this exact invocation:

  ```bash
  reseed_one 'fct-bible-galatians-6-4-5'
  ```

#### `fct-bible-psalm-101-5`

- **Seed JSON:**
  `genesis/data/lamad/content/fct-bible-psalm-101-5.json`.
- **Authored source:** the JSON says
  `"sourcePath": "fct/Module 10 - Creating Shared Understanding - Recognize Distortions.md"`
  (line 8), which resolves under `genesis/docs/content/`. The authored verse is
  present at that markdown file's line 98. It follows the same
  source → `elohim-import` → seed JSON → genesis seeder pipeline.
- **DECLARE_ONLY coverage:** **no**; its id is absent from the four-entry
  `Jenkinsfile:511–515` bundle set.
- **Prepared targeted re-seed/re-author command:** run:

  ```bash
  reseed_one 'fct-bible-psalm-101-5'
  ```

#### `scenario-public-observer-parent-*` — exact id HELD

- **Seed JSON/source:** unresolved until the omitted suffix is recovered. The
  39 candidate JSON files are under `genesis/data/lamad/content/`; each embeds
  one of four source paths under
  `genesis/docs/content/elohim-protocol/public_observer/parent/scenarios/`:
  `community.feature`, `district.feature`, `educational.feature`, or
  `municipality.feature`. Family membership is not enough to select one.
- **DECLARE_ONLY coverage:** **no for every candidate**; no member of this
  family appears in `Jenkinsfile:511–515`.
- **Prepared command after evidence recovery:** replace `<EXACT-ID-FROM-INVESTIGATOR>`
  only with the omitted, peer-confirmed id, then run:

  ```bash
  reseed_one '<EXACT-ID-FROM-INVESTIGATOR>'
  ```

### Targeted re-seed/re-author helper (integrator, alpha builder context)

Run this from the repository root in a shell that can resolve the alpha
cluster's internal Matthew storage service. It stages exactly one seed JSON in
a temporary `DATA_DIR`; it does not widen the operation to the whole corpus.

```bash
reseed_one() {
  content_id="$1"
  source_json="/projects/elohim/genesis/data/lamad/content/${content_id}.json"
  seed_dir="$(mktemp -d /tmp/elohim-ghost-reseed.XXXXXX)"
  test -f "$source_json"
  mkdir -p "$seed_dir/content"
  cp "$source_json" "$seed_dir/content/"
  cd /projects/elohim/genesis/seeder
  STORAGE_URL='http://elohim-matthew-alpha.elohim-alpha.svc.cluster.local:8090' \
    DATA_DIR="$seed_dir" \
    SKIP_BLOB_UPLOAD=true \
    pnpm exec tsx src/seed-sqlite.ts --content-only
}
```

Why this helper, rather than `pnpm seed:dev -- --ids=… --force`: the live CI
content path is `seed-genesis-peer.sh` → `seed-sqlite.ts`, and
`/db/content/bulk` is explicitly skip-on-exists. `seed-sqlite.ts` follows the
bulk call with a reach PATCH for every transformed row
(`genesis/seeder/src/seed-sqlite.ts:982–1004`). The conductor-backed PATCH has
the dead-incarnation repair: when the old anchor has no DHT entry, storage logs
`stale dht_anchor_hash ... healing via create_content re-publish` and calls
`create_content` from the existing SQL row
(`elohim/elohim-storage/src/services/content_service.rs:399–417`). That creates
a fresh servable record; the plain bulk insert cannot.

Treat a seeder summary alone as insufficient because the reach PATCH is
best-effort. For each invocation require the named stale-anchor-heal log (or a
successful live update if the record had already recovered), then verify the
fresh head record before using the manual declaration fan-out below.

### Manual re-declare fan-out after a source has a servable head record

This is not a substitute for re-authoring a true `no_record` ghost. Use it only
after `${SOURCE}/db/content/${ID}/head-record` returns the record matching
`${SOURCE}/db/content/${ID}/head`, and require the script log to say
`carrying head record`; its documented hash-only fallback leaves permanent
absence possible (`scripts/ci/stage-spa-blob.sh:103–109`). Run once per target
doorway:

```bash
ID='<EXACT-ID>'
SOURCE='https://alpha.elohim.host'
TARGET='https://elohim.host'
DECLARE_ONLY=1 \
SOURCE_DOORWAY_URL="$SOURCE" \
STORAGE_API_KEY_ADMIN="$STORAGE_API_KEY_ADMIN" \
DECLARE_MAX_ATTEMPTS=24 \
bash scripts/ci/stage-spa-blob.sh - "$ID" "$TARGET" browser
```

Reverse `SOURCE`/`TARGET` only when the other doorway is the one proven to hold
the matching servable record. Do not run both directions speculatively.
