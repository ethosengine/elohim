---
id: "backlog-self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "alpha's SSR renders stall at the 1200ms fetch soft budget in bursts: first seen as a ~15min window after a bundle-head swap, then (2026-10-02) sporadically for hours on a days-old head — the stalled fetches are named, storage never saw a slow request, so the time is lost before the request reaches storage"
slug: "self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall"
written: "2026-09-12"
updated: "2026-10-19"
author: "runtime-triage"
status: "wip"
priority: "medium"
self_heal_status: blocked
severity: medium
fingerprints: [da8bb3bdd7e1]
nodes: [alpha, doorway-alpha, elohim-matthew-alpha]
relatedNodeIds: []
tags: [self-heal, render-degenerate, ssr, pool-free-client, name-lookup, soft-budget, bundle-head-swap, cold-window, elohim-render, doorway, observability-gap, true-positive, recurrence, matthew, alpha, performance, perf-latency, trustful-self, friction-wait, plane-projection, fused-planes, unit-call, phase-transition, lane-interactive]
cites:
  - https://doorway-alpha.elohim.host/admin/render-stats
  - https://doorway-alpha.elohim.host/admin/self-healing
  - https://doorway-alpha.elohim.host/db/content/elohim-host-landing/head
  - https://elohim.host/admin/render-stats
  - https://elohim.host/db/content/elohim-host-landing/head
  - elohim/elohim-render/src/traced_fetcher.rs
  - elohim/elohim-render/src/stats.rs
  - elohim/elohim-render/src/types.rs
  - doorway/doorway-service/src/server/http.rs
  - doorway/doorway-service/src/render/registry.rs
  - doorway/doorway-service/src/render/warm_shell.rs
  - doorway/doorway-service/src/render/mod.rs
  - doorway/doorway-service/src/ssr.rs
  - .claude/data/runtime-cursor.json
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - .claude/scripts/_lib/runtime_harvest.py
  - .claude/scripts/_lib/__tests__/runtime_harvest_test.py
  - genesis/data/timeline/backlog/self-heal-render-degenerate-cumulative-counter-false-positive.md
  - genesis/data/timeline/backlog/self-heal-doorway-alpha-storage-breaker-matthew-rekey.md
  - elohim/elohim-storage/src/metrics.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/services/live_earned.rs
  - doorway/doorway-service/src/routes/storage_proxy.rs
---

# The third firing is the first TRUE positive — and it is a deploy-coupled cold window, not saturation

## What is exhausted

The per-fetch soft budget, on every cold render, for a bounded window after a
bundle-head swap. Ledger line (fp `da8bb3bdd7e1`, node **alpha**, filed poll 82,
`2026-09-12T12:12:45+00:00`):

```
render degenerate 11/31 of NEW renders = 0.35 over 2 of the last 7 polls (SSR stalled/timedOut saturation)
```

Re-fetch at triage, `GET https://doorway-alpha.elohim.host/admin/render-stats` (12:22:38Z, HTTP 200):

```json
{"total": 40, "rendered": 29, "renderedEmpty": 0, "stalled": 11, "timedOut": 0,
 "errored": 0, "avgWallMs": 430, "maxWallMs": 2359, "degenerateRate": 0.275}
```

And `GET /admin/self-healing` the same minute — every other self-healing arm is idle:

```json
"admission": {"maxInflight": 256, "available": 256, "shedTotal": 0},
"upstreams": [{"endpoint": "http://elohim-matthew-alpha...:8090",
               "circuit": "closed", "errorStreak": 0, "recentFailures": 0}],
"projector": {"lagSeconds": null, "caughtUp": true, "divergentAnchor": 8},
"conductor": {"connected": true, "connectedWorkers": 4, "totalWorkers": 4}
```

**This one is not a sensing artefact.** The two prior firings of this predicate were
(see the cited false-positive record); this one clears every floor that record added —
11 new degenerate renders against `DEGEN_MIN_EVENTS = 3`, a 31-render denominator, and
a `moving` base of 2 real poll transitions. It was then reproduced live, by hand, on a
single request with no concurrency.

## The reproduction (2026-09-12 ~12:23Z, cache-busted cold renders)

`GET /?probe=<rand>` forces `x-render-cache: MISS`, so each line is one fresh render:

| node | terminal | fetches | `x-ssr-wall-ms` |
|---|---|---|---|
| doorway-alpha | **stalled** | 29 | 1256 |
| doorway-alpha | **stalled** | 29 | 1248 |
| doorway-alpha | **stalled** | 29 | 1242 |
| doorway-alpha | **stalled** | 29 | 1232 |
| elohim.host | rendered | 28 | 159 |
| elohim.host | rendered | 29 | 124 |
| elohim.host | rendered | 28 | 441 |
| elohim.host | rendered | 28 | 556 |

Four for four, deterministic, and **clamped**: 1232–1256 ms is `DEFAULT_SOFT_BUDGET_MS`
(1200, `elohim/elohim-render/src/traced_fetcher.rs:27`) plus ~40 ms of everything else.
The arithmetic names the shape precisely — of 29 fetches, **28 settle fast and exactly
one never settles**, is cut at the budget (`traced_fetcher.rs:134-147`), and its 1200 ms
is the whole render's wall time. Every one of those renders was served to a real
requester as **HTTP 200 with a degenerate body**.

### What the reproduction RULES OUT

- **Not the peer's read path.** In the same minutes, through the same doorway, every
  storage route probed answered fast — and faster than the B-side:

  | route | doorway-alpha | elohim.host |
  |---|---|---|
  | `/db/content/elohim-host-landing` | 21 ms | 58 ms |
  | `/db/content/elohim-host-landing/head` | 20 ms | 30 ms |
  | `/db/humans` | 20 ms | 57 ms |
  | `/db/content` | 67 ms | 254 ms |
  | `/api/v1/resilience/summary` | 21 ms | 30 ms |
  | `/db/p2p/conductor-diagnostics` | 22 ms | **503 after 12.06 s** |

  A node whose every probed read answers in ~20 ms is not a slow node. One specific
  fetch hangs; the rest of the surface is healthy.
- **Not CPU/concurrency contention.** Single sequential requests, no burst. The SSR
  limiter is `ssr_semaphore_permits(None, true)` = `DEFAULT_SSR_RENDER_PERMITS = 2`
  (`doorway/doorway-service/src/render/mod.rs:43-52,85`; `/admin/capability` returns
  `null` on both doorways, so no capability-derived sizing), and a semaphore overflow
  sheds to the fallback with `x-ssr-skipped: overflow` — a *different* terminal that
  never reaches `RenderTraceStats` as `stalled` (`server/http.rs:4991-5025`).
- **Not the breaker or admission.** `circuit: closed`, `errorStreak: 0`,
  `shedTotal: 0`, `errored: 0` across the whole window. The render path never saw the
  upstream fail — and a 503 would land in `errored`, fast, not in `stalled`, slow.

### What it points AT — the bundle-head swap

The two doorways were serving **different app bundles** at triage time:

| | doorway-alpha (matthew) | elohim.host (adam) |
|---|---|---|
| `main-*.js` in the served HTML | `main-G4BXXWHN.js` | `main-UK6P25XM.js` |
| rendered HTML bytes | 79 855 | 63 998 |
| declared `elohim-host-landing` head | `uhCkkbF6csqsteIHtviZO_c3J1bmnQouOxKOQayepxWTx6nyBzr5x` | `uhCkkdgZGt1PmgwX-pw76FbedaWM9wiNsDhXeVBLKX2eJt0WAITOA` |
| declared `blobHash` | `sha256-22f885cda980b6c2…` | `sha256-3bf2280cbd0f64f6…` |
| head `updatedAt` | **2026-09-12 12:10:48** | 2026-09-11T16:26:41Z |

`styles-7XLYMW2X.css` and `polyfills-X2TQPNDQ.js` are identical on both; only `main-*`
differs. Both doorways run the same `SSR_BUNDLE_SLUG: elohim-host-landing` +
`SSR_BUNDLE_SLUGS: elohim-host-landing,lamad-spa`
(`genesis/orchestrator/manifests/doorway/alpha.yaml:249-261`, `alpha-b.yaml:295-304`),
so the divergence is not configuration — each doorway materializes the head **its own
storage peer declares** (`SSR_STORAGE_URL`), and matthew declared a new one at
**12:10:48Z, roughly two minutes before the poll that filed this finding**.

### Then it cleared, on its own, with no bundle change (~12:26Z)

| node | terminal | fetches | `x-ssr-wall-ms` |
|---|---|---|---|
| doorway-alpha | rendered | 29 | 59 |
| doorway-alpha | rendered | 29 | 63 |
| doorway-alpha | rendered | 29 | 36 |
| doorway-alpha | rendered | 29 | 53 |

Same bundle, same route, same 29 fetches — **36–63 ms**, now the fastest of the pair.
The condition is a **cold window**, roughly 15 minutes wide, opened by the head swap
and closed by something warming underneath it. That the SAME bundle renders clean is
what rules out "the new app bundle introduced a hanging fetch" as a sufficient
explanation.

## Root-cause inventory (scope pass)

1. **`elohim/elohim-render/src/traced_fetcher.rs:27,134-147`** — `DEFAULT_SOFT_BUDGET_MS
   = 1_200`; a fetch outstanding past it is recorded `FetchOutcome::Stalled` and returned
   as an error so the render falls back fast. Working exactly as designed. It is the
   *messenger* here, not the defect, and the standing instruction from the two prior
   records holds: **do not widen it** (`DOORWAY_SSR_FETCH_SOFT_BUDGET_MS`,
   `render/registry.rs:357-361`) — that trades a fast degenerate answer for a slow one.
2. **`doorway/doorway-service/src/ssr.rs:96-200` (`ResolverFetcher`)** — every bundle
   fetch is rewritten onto `SSR_STORAGE_URL` and issued on the shared pooled storage
   client. So the stalling fetch IS a storage read, on the same client and the same
   upstream whose other routes answer in 20 ms.
3. **THE BLINDNESS — the stalling URL is recorded and then thrown away.** `RenderTrace`
   carries a `FetchEvent { url, method, offset_ms, outcome }` per fetch
   (`traced_fetcher.rs:148-156`, `elohim-render/src/types.rs`), and **nothing exposes
   it**:
   - the wire headers carry a *count* only — `x-ssr-fetches`, `x-ssr-terminal`,
     `x-ssr-wall-ms`, `x-ssr-trace-id` (`server/http.rs:7426-7437`);
   - the Loki line carries the same three scalars — `path`, `terminal`, `fetches`,
     `wall_ms`, `observation_id` (`server/http.rs:5192-5200`);
   - `/admin/render-stats` is a lifetime counter roll-up (`elohim-render/src/stats.rs`).

   The consequence, measured: this triage could establish *that* exactly one of 29
   fetches hangs — by arithmetic on `x-ssr-wall-ms` against the soft budget — and could
   not establish *which*, from any surface, including Loki. Three firings of this
   predicate have now cost three triage passes, and the one field that would have ended
   any of them in a minute is computed in-process on every render and discarded.
4. **Not the poller.** `_render_degenerate` (`.claude/scripts/_lib/runtime_harvest.py`)
   reported a real, live, reproducible condition with an honest base. See the handoff
   section added to the cited false-positive record: that record's "zero true positives"
   tally is superseded by this entry.

## Fix path

Two moves, both bounded, in order of value:

1. **Name the fetch (observability, ~10 lines).** On a degenerate terminal, emit the
   offending fetch's URL: a `x-ssr-stalled-url` response header and/or the URL on the
   existing `doorway::ssr::trace` warn arm — the data is already in `trace.fetches`, so
   this is a selection, not a new instrument. Filter to the `Stalled`/`Errored` events
   so a healthy render's 29 URLs never reach the log. This turns every future firing of
   this fingerprint into a one-line diagnosis.
2. **Warm after a swap (behaviour).** `doorway/doorway-service/src/render/warm_shell.rs`
   already exists for exactly this shape of problem (serving `/` through an upstream's
   catch-up window) but covers the bundle-carrying *shell*, not the render's *data*
   fetches. A single warm render issued after a bundle-head hot-swap — the isolate is
   already being rebuilt at that point (`render/registry.rs`, `/admin/ssr-bundle/refresh`)
   — would pay the cold fetch once, internally, instead of charging it to the first N
   real visitors as degenerate 200s.

Deliberately NOT proposed: widening the soft budget; changing `_render_degenerate`
again; raising `DEFAULT_SSR_RENDER_PERMITS`. None of those addresses a single hanging
fetch, and the first two hide it.

## Current decision

**BLOCKED — canonicalized, condition self-cleared, fix path needs a gate this agent
cannot run.**

Both fix moves are Rust in `doorway-service` / `elohim-render`. Landing either requires
`just gate doorway` (RUSTFLAGS="" per root CLAUDE.md) and then an operator fleet roll
before it observes anything, and this triage pass was dispatched under an explicit
no-cargo, no-commit, no-kubectl constraint. Handing over a Rust diff that no gate has
seen would be worse than handing over this record.

What the poller cites on re-encounter: **the alpha `render-degenerate` fingerprint is a
deploy-coupled cold window, not saturation.** If it re-fires, read the
`elohim-host-landing` head's `updatedAt` on that node FIRST — if it is within ~15 minutes
of the finding, this is the same concern and it will clear itself. If the head is hours
old and renders still stall, that is a NEW condition and this record does not cover it.

## Verification

- 2026-09-12 12:22:38Z — `/admin/render-stats` + `/admin/self-healing` re-fetched on
  both nodes (HTTP 200, quoted above); the stored 8-sample cursor window read from disk.
- 2026-09-12 ~12:23Z — four cache-busted cold renders on alpha, 4/4 `stalled` at
  1232–1256 ms; four on elohim.host, 4/4 `rendered` at 124–556 ms.
- 2026-09-12 ~12:26Z — four more cache-busted cold renders on alpha, 4/4 `rendered` at
  36–63 ms. **Condition cleared without intervention.**
- Closure left to the poller: the delta predicate goes silent once the burst samples age
  out of the `WINDOW`-8 ring buffer, and the line is deleted at `CLOSE_STREAK`. The
  ledger line is marked `blocked` (the fix path is blocked, not the closure) and is
  **not** manually deleted — unlike the two false positives, this condition was REAL, so
  disappearance is the honest closure evidence and a recurrence must re-file as NEW.

### Instrument disclosure

The eight cache-busted renders above are counted in alpha's own lifetime counters: they
moved `total` 40 → 48 and `stalled` 11 → 15. **Four of the 15 lifetime stalls on that
node are this triage's probes.** A later reader comparing `/admin/render-stats` against
the 12:12:45Z ledger line must subtract them. Recorded because the measurement is not
free here — reading this node's render health perturbs it.

## Observation, not a claim

The two peers' `updatedAt` for the same content id are serialized differently —
`"2026-09-12 12:10:48"` (space-separated, no zone) on matthew versus
`"2026-09-11T16:26:41Z"` (RFC 3339) on adam. Same route, same view type. That is either
two write paths for the same projection row or two serializers, and it is the kind of
asymmetry that makes cross-peer head comparison brittle. Named here because it was seen;
not chased, and not claimed to relate to the stall.

(2026-09-28: both peers now serialize `updatedAt` as RFC 3339, `2026-09-28T03:59:32Z`.
This asymmetry no longer reproduces.)

---

# 2026-09-28: `da8bb3bdd7e1` fired again after an edge deploy (second true positive)

## What is exhausted

Ledger line (fp `da8bb3bdd7e1`, node **alpha**, filed poll 204, `2026-09-28T04:05:16+00:00`):

```
render degenerate 11/33 of NEW renders = 0.33 over 4 of the last 8 polls (SSR stalled/timedOut saturation)
```

The base clears every floor the false-positive record added: 11 new degenerate renders
(floor 3), a 33-render delta, and 4 poll transitions in which `total` moved. The stored
window (`.claude/data/runtime-cursor.json`, poll 204) begins at `total: 1` on alpha and
`total: 0` on alpha-b. **Both doorways had just restarted**, which fits the
`a62972d24` `[build:conductor] [build:edge]` roll. Across the same window:

| node | `total` | `stalled` | `maxWallMs` |
|---|---|---|---|
| alpha (matthew) | 1 → 34 | 0 → 11 | 1272 |
| alpha-b (adam) | 0 → 27 | 0 → 0 | 2015 |

Re-fetch at triage, `GET https://doorway-alpha.elohim.host/admin/render-stats` (04:06:21Z, HTTP 200):

```json
{"total": 34, "rendered": 23, "renderedEmpty": 0, "stalled": 11, "timedOut": 0,
 "errored": 0, "avgWallMs": 457, "maxWallMs": 1272, "degenerateRate": 0.3235}
```

`/admin/self-healing` from the same minute shows every other arm idle:
`admission.shedTotal: 0`, upstream `circuit: closed, errorStreak: 0`,
`conductor.connectedWorkers: 4/4`, `warmup.completed: true`. `projector.caughtUp: false`
with `divergentAnchor: 60` and all seven conductor peers `Degraded`: the fleet was still
catching up after the roll.

`maxWallMs: 1272` is the same clamp as on 2026-09-12: the 1200 ms soft budget plus the
rest of the render. `errored: 0` again. One fetch never settles, and the render is cut at
the budget.

## The re-encounter rule said to read the head first

- `GET /db/content/elohim-host-landing/head` on doorway-alpha returned `updatedAt: 2026-09-28T03:59:32Z`.
  The finding was filed **5m44s later**.
- Served bundle: alpha `main-LDLJH3DK.js`, which is new (it was `main-G4BXXWHN.js` on
  2026-09-12). Adam is still on `main-UK6P25XM.js`, unchanged since 2026-09-12.
- Only alpha took a new app bundle, and only alpha stalled. The same thing happened on
  2026-09-12.

**Same concern, same shape.** The fingerprint maps onto this record. It does not need a
new one.

## Already cleared by triage time

Three cache-busted cold renders of `/` on doorway-alpha at 04:06:43Z (`x-render-cache: MISS`):
`rendered`, 29/28/29 fetches, **83 / 57 / 40 ms**. A fourth at 04:08:38Z rendered in
117 ms. On elohim.host at 04:08:38Z the render took 396 ms. The window had closed about
7 minutes after the head update. On 2026-09-12 it took about 15 minutes.

Instrument disclosure: those probes add 4 renders to alpha's lifetime counters and 1 to
alpha-b's. None of them stalled.

## Decision this time: fix move 1 (name the fetch) is landed

On 2026-09-12 both fix moves were blocked because that pass could not run cargo. This
pass could. Move 1 was the prerequisite, because move 2 (warm after a swap) cannot be
designed until we know which fetch is cold.

- `doorway/doorway-service/src/render/mod.rs`: new `degenerate_fetch_summary(&RenderTrace)`.
  It selects only the `Stalled`/`Errored` `FetchEvent`s as `METHOD url (outcome Nms)`,
  names at most 5 and appends `+N more`, and returns `None` on a healthy render. Three
  unit tests cover this, one of them the 28-arrive/1-stalls cold-window shape.
- `doorway/doorway-service/src/server/http.rs`, at the SSR trace site: when the summary
  is `Some`, a `warn!` on target `doorway::ssr::trace` carries `degenerate_fetches` next
  to `path`, `terminal`, `wall_ms` and `observation_id`.

Deliberately log-only. A response header was NOT added: the fetch URL can be the rewritten
`SSR_STORAGE_URL` target (cluster-internal DNS), and that should not be served to the
public.

Still not proposed: widening the soft budget, touching `_render_degenerate`, or raising
`DEFAULT_SSR_RENDER_PERMITS`. The reasons above are unchanged.

## Current decision (supersedes the 2026-09-12 BLOCKED)

**IN PROGRESS. The instrument is committed and the cure waits on its first reading.**

- Next step (not an operator action): the next edge deploy that carries this commit opens
  a cold window of its own. When it does, query Loki on alpha for target
  `doorway::ssr::trace` lines containing `degenerate_fetches` to get the stalling URL.
- Then decide move 2 using that URL. It could be a warm render after the swap, or a fix to
  whatever storage read is cold for that route.
- What the poller cites on re-encounter: this record. If the head's `updatedAt` is within
  ~15 min of the finding, this is the same cold window and it clears on its own. Read the
  `degenerate_fetches` warn before triaging again.

## Verification

- 2026-09-28 04:06:21Z: `/admin/render-stats`, `/admin/self-healing` and the landing head
  re-fetched on both nodes, all HTTP 200, quoted above. The cursor window was read from disk.
- 2026-09-28 04:06:43Z and 04:08:38Z: four cold renders on alpha, 4/4 `rendered` at
  40–117 ms. **The condition had cleared without intervention.**
- `just gate doorway` (fmt-check, clippy `-D warnings`, test, `RUSTFLAGS=""`): EXIT=0. The lib suite passed 1720, 0 failed, 2 ignored, including
  the 3 new `render::degenerate_fetch_summary_tests`. The bins suites were green too.
- Closure: left to the poller, by disappearance. The ledger line is `triaged` and is NOT
  manually deleted. The condition was real, so it closes by disappearance and a recurrence
  files as NEW.

## Observation, not a claim (cross-peer head vs blob)

Both peers report the **same** `headActionHash` (`uhCkkEBj4KjHttF86rslxs2fzwFvgJU8jQZJHsVLEbsYrw_lGBP0K`)
and the same `updatedAt` (`2026-09-28T03:59:32Z`) for `elohim-host-landing`, but different
`blobHash`es. Matthew has `sha256-9a0bae8d…`. Adam has `sha256-3bf2280c…`, the blob adam
already declared on 2026-09-12. So one declared head resolves to two blobs, and adam's
doorway serves a 16-day-old app bundle. That is a head/blob coherence question on the
content-sync plane, not an SSR render question. It is named here because it was seen,
not investigated, and not claimed to cause the stall (adam, the peer with the stale blob,
is the one that does NOT stall).

---

# 2026-10-02: `da8bb3bdd7e1` fired a third time, and this time the head is days old

## What is exhausted

Ledger line (fp `da8bb3bdd7e1`, node **alpha**, filed poll 227, `2026-10-02T17:45:07+00:00`):

```
render degenerate 6/11 of NEW renders = 0.55 over 3 of the last 8 polls (SSR stalled/timedOut saturation)
```

The stored window (`.claude/data/runtime-cursor.json`, poll 227) on alpha runs `total` 1 → 8 → 11 → 12
and `stalled` 0 → 3 → 5 → 6. alpha-b over the same window: `total` 1 → 12, `stalled` 0.

Re-fetch at triage, `GET https://doorway-alpha.elohim.host/admin/render-stats` (17:45:59Z, HTTP 200):

```json
{"total": 12, "rendered": 6, "renderedEmpty": 0, "stalled": 6, "timedOut": 0,
 "errored": 0, "avgWallMs": 723, "maxWallMs": 1287, "degenerateRate": 0.5}
```

`/admin/self-healing` the same minute: `admission.shedTotal: 0`, upstream `circuit: closed`,
`errorStreak: 0`, `warmup.completed: true`, `conductor.connectedWorkers: 4/4`,
`projector.caughtUp: false` with `divergentAnchor: 27`, two of seven conductor peers `Degraded`.

## The re-encounter rule, applied: this is NOT the cold window

The rule above says to read the head first. `GET /db/content/elohim-host-landing/head` returned
`updatedAt: 2026-09-30T09:39:10Z` and `blobHash: sha256-9a0bae8d…`, the blob alpha already served on
2026-09-28. The head is **more than two days old**. `/health` reports `uptime: 27755` s, so the
doorway process started about 10:04Z (image `5ffe985`, built 2026-10-02T09:32:53Z). The seven stalls
are spread over the following **7.7 hours**. Neither a head swap nor a restart is within 15 minutes of
most of them.

So the "deploy-coupled ~15 min cold window" was one occasion of this condition, not its definition.
The record keeps the fingerprint (same node, same terminal, same clamp at the soft budget) and the
frame widens: **renders on alpha stall in sporadic bursts with no deploy nearby.**

## The instrument from 2026-09-28 read out

The `degenerate_fetches` warn landed with image `5ffe985`. Loki, `{namespace="elohim-alpha"} |= "degenerate_fetches"`,
2026-10-02T09:00Z → 17:49Z: **7 lines, all from pod `elohim-doorway-alpha-bd68cb6fc-h2d2t`**, all
`path: "/"`, `terminal: "stalled"`. The B-side doorway pods have none.

| time (UTC) | `wall_ms` | fetches named, each `stalled 1200ms` |
|---|---|---|
| 11:45:44 | 1287 | resilience/policymakers, epr-head/developers, resilience/developers, epr-head/communities, resilience/communities, +1 more |
| 13:05:58 | 1257 | resilience/developers, epr-head/communities, resilience/communities, epr-head/manifesto |
| 13:06:08 | 1227 | epr-head/developers, resilience/developers, epr-head/communities, resilience/communities, epr-head/manifesto |
| 15:56:52 | 1239 | resilience/policymakers, epr-head/developers, resilience/developers, epr-head/communities, resilience/communities, +1 more |
| 15:57:38 | 1268 | resilience/communities, epr-head/manifesto |
| 17:34:44 | 1271 | resilience/developers, epr-head/communities, resilience/communities, epr-head/manifesto |
| 17:46:18 | 1229 | epr-head/developers, resilience/developers, epr-head/communities, resilience/communities, epr-head/manifesto |

(Full paths are `GET /epr-head/concept-path-forward-<x>`, `GET /api/v1/resilience/concept-path-forward-<x>`
and `GET /epr-head/manifesto`. The logged host is `localhost:8090`: the trace records the URL the
bundle asked for, and `ResolverFetcher` rewrites it onto `SSR_STORAGE_URL` before sending,
`doorway/doorway-service/src/ssr.rs`.) The 17:46:18 line is one of this triage's own probes.

Two corrections to what this record said before:

- **It is not one fetch.** 2 to 6 fetches stall together. The 2026-09-12 arithmetic ("28 settle, exactly
  one never settles") could not tell one from several, because concurrent fetches cut at the same
  budget cost the same 1200 ms.
- **It is always the tail.** The named fetches are a suffix of the same ordered sequence
  (policymakers → developers → communities → manifesto). How far up the sequence the stall reaches
  varies from 2 to 6. Everything issued before that point arrives.

## Storage is ruled out, by its own histogram

The same routes, fetched through the doorway's pooled proxy client at triage, 6 times each:
`/epr-head/concept-path-forward-communities`, `/api/v1/resilience/concept-path-forward-communities`,
`/api/v1/resilience/concept-path-forward-developers`, `/epr-head/manifesto`,
`/epr-head/concept-path-forward-policymakers`. All 30 answered HTTP 200 in **14–25 ms**.

Storage records one `elohim_http_request_duration_ms` observation per request at its single dispatch
point, labelled by `route_class` (`elohim/elohim-storage/src/metrics.rs`). `/api/v1/resilience/*` is
class `api`. Prometheus, pod `elohim-matthew-alpha-0`, 10:00Z → 17:50Z, 30 s scrape:

- `api` / `2xx`: 1665 observations, **all in the ≤500 ms bucket**. `api` / `4xx`: 37, all ≤500 ms.
- `api` / `5xx` (where a request dropped by the client before a status is recorded would land):
  the count above 1000 ms last moved at 10:15Z and never after.
- No step in `_count − _bucket{le="1000.0"}` for `api` or `content` within ±2 min of the six stalls
  from 11:45 to 17:34.
- `elohim_http_requests_in_flight` max over 8 h: 3.

Fifteen `api`-class fetches are named as cut at 1200 ms by the doorway today. Storage never held an
`api` request for even 500 ms. **The stalled requests were not slow inside storage.** The 1200 ms is
spent before storage's dispatch wrapper starts its clock, or after it stops: in the name lookup, in the
TCP connect, in transit, or on the render thread.

(Storage logs no per-request lines, so Loki cannot show whether a stalled request arrived late or
never. The histogram is the only storage-side witness.)

## Root-cause inventory (this pass)

1. **`doorway/doorway-service/src/server/http.rs`, `init_ssr_render_client`** — the SSR render client is
   pool-free on purpose (`pool_max_idle_per_host(0)`, the containment for the 2026-08-21 parked-driver
   incident documented on that function). The consequence: every render fetch opens a fresh TCP
   connection and does a fresh name lookup of `elohim-matthew-alpha.elohim-alpha.svc.cluster.local`.
   A `/` render does about 29 of each. The proxy client that answered the same routes in 17 ms keeps
   pooled connections and does neither. That is the one structural difference between the path that
   stalls and the path that does not.
2. **`doorway/doorway-service/src/routes/storage_proxy.rs:48`** — `STORAGE_PROXY_CONNECT_TIMEOUT_SECS = 3`
   is the render client's connect bound. It is above the 1200 ms soft budget, so a slow connect is cut
   by the soft budget first and reads as `stalled`, never as `errored`. That matches `errored: 0` on
   every firing of this fingerprint.
3. **`elohim/elohim-render/src/traced_fetcher.rs`** — unchanged and still only the messenger.
4. **Not established:** which of lookup, connect or the render thread loses the time. Candidates on the
   record, none proven: a slow `getaddrinfo` (this doorway has a recorded history of DNS flaps, see the
   doorway ops incidents memory); a dropped SYN (a 1 s retransmit lands near the budget, and earlier
   records show alpha-b renders that completed near 1 s: `maxWallMs` 991 today, with no stall); the render thread's
   `current_thread` runtime not polling the tail fetches. The doorway pod runs on node `intel-nuc` and
   the storage pod on `ethosengine`, so every one of these connections crosses nodes.

## Fix path

1. **Time the name lookup (landed this pass).** `TimedDnsResolver` in `doorway/doorway-service/src/ssr.rs`
   is now the SSR render client's resolver. It resolves exactly as before (`tokio::net::lookup_host`,
   the same `getaddrinfo` on the blocking pool) and emits a `warn!` on target `doorway::ssr::dns` with
   `host` and `elapsed_ms` when a lookup takes ≥ 250 ms or fails. On the next stall, one Loki query
   settles the lookup question: a `doorway::ssr::dns` line beside the `degenerate_fetches` line means
   the lookup is the cause; a stall with no such line rules it out.
2. **Then the cure, chosen by that reading.**
   - Lookup is slow: cache the resolved address on the render client for a short TTL, so a render does
     one lookup and not 29, and a slow refresh serves the previous answer.
   - Lookup is clean: the next split is connect vs render thread. That needs a connect-phase timing on
     the render client, or the 2026-09-12 move 2 (a warm render) re-examined against the tail-suffix shape.
3. Unchanged refusals: do not widen the soft budget, do not touch `_render_degenerate`, do not raise
   `DEFAULT_SSR_RENDER_PERMITS`, and do not re-pool the render client (that reopens the 2026-08-21 wedge).

## Current decision (supersedes 2026-09-28)

**IN PROGRESS. Storage is exonerated; the second instrument is committed and the cure waits on its
first reading.** `just gate doorway` (fmt-check, clippy `-D warnings`, test, `RUSTFLAGS=""`): EXIT=0, lib suite 1723 passed, 0 failed, 2 ignored, including the 3 new `ssr::tests` for the resolver.

What the poller cites on re-encounter: this record. The condition is intermittent (1 stall in 14
cold probes at triage) and is no longer tied to a deploy, so **expect this fingerprint to close by
disappearance and re-file**. On the next firing, before anything else, run in Loki over the firing
window: `{namespace="elohim-alpha"} |= "doorway::ssr::dns"` and `|= "degenerate_fetches"`, and compare
timestamps. That reading is the whole triage. The instrument only reads after an edge deploy carries
this commit; a firing on image `5ffe985` or older has no `doorway::ssr::dns` lines by construction.

## Verification

- 2026-10-02 17:45:59Z: `/admin/render-stats`, `/admin/self-healing`, `/health`, `/version` and the
  landing head re-fetched on alpha (HTTP 200, quoted above). The cursor window was read from disk.
- 17:46:17Z → 17:47:05Z: 14 cache-busted cold renders of `/` on alpha. 13 `rendered` at 35–67 ms,
  **1 `stalled` at 1229 ms**. Two on elohim.host: `rendered`, 142 and 155 ms. The condition is live
  and intermittent, not cleared.
- 17:51:55Z: `/admin/render-stats` reads `total: 26, stalled: 7`.
- Loki and Prometheus readings as quoted above.
- Closure: left to the poller. The ledger line is `triaged` and is not manually deleted.

### Instrument disclosure

This triage added 14 renders and 1 stall to alpha's lifetime counters (`total` 12 → 26, `stalled`
6 → 7) and 2 renders to alpha-b's. It also made 6 `/head` reads on alpha and 4 on alpha-b; each is a
2 s conductor ask (below) and each lands in storage's `content` histogram above 1000 ms. The `content`
step of +6 on matthew and +4 on adam at 17:47Z in Prometheus is this triage, not the stall.

## Observation, not a claim (`/head` costs 2.0 s on both peers, every time)

`GET /db/content/elohim-host-landing/head` took **2.02 s** on doorway-alpha (6 of 6, also for
`lamad-spa`) and **2.02–2.08 s** on elohim.host (3 of 3), and returned `stagingCandidateState: "unavailable"`.
Storage logs the reason each time: `head read: local election ask timed out — reporting unavailable`,
`outcome: "deadline"`, `elapsed_ms: 2001`. That is `LIVE_ELECTION_BUDGET` (2 s,
`elohim/elohim-storage/src/services/live_earned.rs:59`) expiring on the conductor's
`resolve_canonical_election` call in `ask_local_election` (`elohim/elohim-storage/src/http.rs`). The
local conductor does not answer that ask within 2 s on either peer. In the same hours storage logged
`record_peer_status zome call failed … init() callback … blocking this zome call for longer than 30 seconds`
(11:45:45Z) and the doorway logged `Zome call timed out after 10000ms (infrastructure/infrastructure/get_all_doorways)`
against adam's and james's conductors.

This is a conductor-responsiveness question, not an SSR one, and it is not claimed to cause the
stall: the `/` render does not fetch `/head`, and the stalled routes are plain SQLite reads that
storage answered fast. It is named because every consumer of `/head` (the doorway's bundle-heads
reconciler, `doorway/doorway-service/src/render/bundle_heads.rs`, treats `unavailable` as "keep the
last candidate") is currently paying 2 s for an answer of "unavailable". It has no backlog home yet.

---

# 2026-10-09: `da8bb3bdd7e1` fired a fourth time. The lookup instrument read out empty, and it could not have read otherwise

## What is exhausted

Ledger line (fp `da8bb3bdd7e1`, node **alpha**, filed poll 198, `2026-10-09T15:47:00+00:00`):

```
render degenerate 5/16 of NEW renders = 0.31 over 5 of the last 8 polls (SSR stalled/timedOut saturation)
```

Re-fetch at triage, `GET https://doorway-alpha.elohim.host/admin/render-stats` (15:48:07Z, HTTP 200):

```json
{"total": 19, "rendered": 13, "renderedEmpty": 0, "stalled": 6, "timedOut": 0,
 "errored": 0, "avgWallMs": 499, "maxWallMs": 1274, "degenerateRate": 0.3158}
```

`/admin/self-healing` the same minute: `admission.shedTotal: 0`, upstream `circuit: closed`,
`errorStreak: 0`, `projector.caughtUp: true` with `divergentAnchor: 33`, `warmup.completed: true`,
`conductor.connectedWorkers: 4/4`, two of seven conductor peers `Degraded`. `elohim.host` (alpha-b)
at the same time: `total: 14, stalled: 0, maxWallMs: 1004`.

The re-encounter rule, applied: the `elohim-host-landing` head reads `updatedAt: 2026-10-06T21:30:37Z`,
three days old. `/health` reports `uptime: 17061` s, so this doorway process (image `b4a3911`, built
2026-10-09T10:21:07Z) started about 11:04Z. Same shape as 2026-10-02: sporadic stalls on a days-old
head with no deploy nearby. **Same concern; this record still covers it.**

## The stalls, from the 2026-09-28 instrument

Loki, `{namespace="elohim-alpha"} |= "degenerate_fetches"`, 2026-10-09T10:30Z → 15:49Z: **6 lines**, all
from pod `elohim-doorway-alpha-74fd78cbd-bbvc7`, all `path: "/"`, `terminal: "stalled"`, each preceded
by a trace line reading `fetches: 29`. The three B-side doorway pods have none.

| time (UTC) | `wall_ms` | fetches named, each `stalled 1200ms` |
|---|---|---|
| 11:32:19 | 1265 | epr-head/developers, resilience/developers, epr-head/communities, resilience/communities, epr-head/manifesto |
| 11:38:05 | 1228 | resilience/know-thyself-path, epr-head/policymakers, resilience/policymakers, epr-head/developers, resilience/developers, +3 more |
| 12:28:44 | 1271 | resilience/developers, epr-head/communities, resilience/communities, epr-head/manifesto |
| 12:48:11 | 1274 | epr-head/communities, resilience/communities, epr-head/manifesto |
| 14:46:24 | 1234 | resilience/policymakers, epr-head/developers, resilience/developers, epr-head/communities, resilience/communities, +1 more |
| 15:16:20 | 1251 | resilience/developers, epr-head/communities, resilience/communities, epr-head/manifesto |

The tail-suffix shape from 2026-10-02 holds in all six: the stalled fetches are a suffix of the same
ordered sequence, 3 to 8 deep this time (11:38:05 reaches one step further up, to `know-thyself-path`).

Storage is exonerated again by its own histogram. Prometheus, pod `elohim-matthew-alpha-0`,
`elohim_http_request_duration_ms`, 10:30Z → 15:49Z in 5-minute steps: route class `api` has **zero**
requests slower than 1000 ms in the whole range; `content` has one burst of about 90 in the
11:05–11:10Z step (just after the process start, none near any stall) and zero elsewhere.
`elohim_http_requests_in_flight` max: 2.

## The 2026-10-02 instrument read out: zero lines, and that reading decides nothing

`{namespace="elohim-alpha"} |= "doorway::ssr::dns"`, same range: **zero lines**, namespace-wide. The
spelling is not the reason (`"target":"doorway::ssr::trace"` lines from the same pod match the same way),
and the image carries the resolver (`7fe05122f` is an ancestor of `b4a3911`).

The 2026-10-02 section said a stall with no `doorway::ssr::dns` line beside it "rules the lookup out".
**That was wrong, and this pass does not draw that conclusion.** Reading the code path end to end:

- `elohim/elohim-render/src/traced_fetcher.rs` cuts a slow fetch with
  `tokio::time::timeout(soft, self.inner.fetch(request))`. On expiry the inner future is **dropped**.
- That future is `ResolverFetcher::fetch` → `reqwest` `send()` → connect → `TimedDnsResolver::resolve`
  (`doorway/doorway-service/src/ssr.rs`). The resolver logged only **after** `lookup_host(...).await`
  returned. A lookup still pending at 1200 ms goes away with the fetch and never reaches either log arm.

So the resolver could name a lookup of 250–1199 ms, and was silent by construction for any lookup of
1200 ms or more: exactly the lookups that would produce a `stalled` fetch. Zero lines across six stalls
is therefore consistent with two different worlds, and the instrument as shipped cannot tell them apart:

1. every lookup answered in under 250 ms and the time is lost in the connect or on the render thread; or
2. the lookups behind the stalled fetches hung past the cut (a resolver timeout is seconds, not
   hundreds of milliseconds, so a hung lookup would skip the 250–1199 ms band entirely).

What the zero does establish: no lookup on this client completed in the 250–1199 ms band in 5.3 hours.

The abandoned-lookup reasoning is from reading the code, not from a reproduction: no test in this pass
drives a real fetch through `reqwest` and cuts it mid-lookup.

Seen in the same pod, named and not claimed as cause: between 13:33Z and 15:06Z the doorway logged
`failed to lookup address information: Name or service not known` on targets
`doorway::worker::conductor` and `doorway::projection::subscriber` (at least 100 lines; the query was
capped). That is a failed lookup of a conductor host name on a different client. It shows name
resolution in this pod is not uniformly healthy; it says nothing about how long the render client's
lookups of the storage service name take.

## Root-cause inventory (this pass)

1. **`doorway/doorway-service/src/ssr.rs`, `TimedDnsResolver::resolve`**: the instrument blindness
   above. Repaired this pass (below).
2. **`.claude/scripts/_lib/runtime_harvest.py`, `_render_degenerate`**: the finding's "5/16" is
   arithmetic across a pod restart. The stored window (`.claude/data/runtime-cursor.json`, poll 198) on
   alpha runs `total` 3, 3, 4, 5, 23, 25, **5**, 19 and degenerate 1, 1, 1, 1, 2, 3, **2**, 6: the counters
   reset between the sixth and seventh sample. The predicate subtracted first from last (19 − 3 = 16,
   6 − 1 = 5), and its reset guard only looked at those two endpoints, so a restart whose new counters
   had already grown past the first sample was invisible. Summed per transition the window holds
   22 + 5 + 14 = 41 renders and 2 + 2 + 4 = 8 degenerate, **0.20, under the 0.25 threshold**. This
   firing's number is a sensing error; the stalls underneath it are real (6 of 19 in the current
   process, each one in Loki). Repaired this pass (below). It is a correctness repair to the delta and
   does not loosen the predicate: a restarted process's renders are still counted as new.
3. Items 1–3 of the 2026-10-02 inventory stand: the pool-free render client
   (`doorway/doorway-service/src/server/http.rs`, `init_ssr_render_client`), the 3 s connect bound above
   the 1200 ms soft budget (`doorway/doorway-service/src/routes/storage_proxy.rs`), and the traced
   fetcher as messenger.
4. **Still not established:** which of lookup, connect or render thread loses the time.

## Fix path

1. **Name the abandoned lookup (landed this pass).** `LookupInFlight` in
   `doorway/doorway-service/src/ssr.rs` is a guard created when a lookup starts and stood down when it
   answers. If it goes out of scope while still armed, it emits a `warn!` on target `doorway::ssr::dns`,
   message `SSR render fetch: name lookup abandoned before it answered`, with `host` and `elapsed_ms`.
   Resolution is unchanged. The reading now has three outcomes instead of one ambiguous silence:
   - `abandoned` lines at about 1200 ms beside a `degenerate_fetches` line: the lookup is where the time goes.
   - `slow name lookup` lines: same conclusion, milder.
   - a stall with neither: every lookup answered in under 250 ms, and the lookup is ruled out for real.
2. **Count the window per transition (landed this pass).** `_render_degenerate` sums deltas across
   consecutive samples and treats a transition where a counter went down as a restart whose since-boot
   counters are that transition's delta.
3. **Then the cure, chosen by reading 1.** Unchanged from 2026-10-02: lookup slow → cache the resolved
   address on the render client for a short TTL (one lookup per render, not 29); lookup clean → time the
   connect, or re-examine the warm render against the tail-suffix shape.
4. Unchanged refusals: do not widen the soft budget, do not raise `DEFAULT_SSR_RENDER_PERMITS`, do not
   re-pool the render client. The address cache in 3 was deliberately NOT landed with the instrument:
   shipped together, a fall in stalls could not be attributed, and the lookup has not been shown slow.

## Current decision (supersedes 2026-10-02)

**IN PROGRESS. The lookup question is still open. The instrument that was meant to close it was blind in
the deciding case and is repaired. The cure waits on its first honest reading.**

What the poller cites on re-encounter: this record. The condition is live and intermittent on alpha
(6 stalls in 19 renders over 4.7 h). Expect the fingerprint to close by disappearance and re-file. On
the next firing, check `/version` on doorway-alpha first:

- image at or after the commit that carries `LookupInFlight`: run
  `{namespace="elohim-alpha"} |= "doorway::ssr::dns"` beside `|= "degenerate_fetches"` over the firing
  window and compare timestamps. That reading is the whole triage and selects the cure in fix path 3.
- image `b4a3911` or older: there is nothing new to read. A zero from `doorway::ssr::dns` on those
  images means nothing. Do not re-triage; cite this section.

Both changes observe only after they are carried: the resolver by an edge deploy, the predicate by the
commit reaching the checkout the poller runs from. Neither has been observed live.

## Verification

- 2026-10-09 15:48:07Z: `/admin/render-stats`, `/admin/self-healing`, `/health`, `/version` and the
  landing head re-fetched on alpha, and `/admin/render-stats` on elohim.host (all HTTP 200, quoted
  above). The cursor window was read from disk. **The condition is live, not cleared.**
- Loki and Prometheus readings as quoted above (range 10:30Z → 15:49Z).
- No cache-busted probe renders were issued this pass, so the triage added nothing to alpha's render
  counters. It made one `/head` read on alpha (a 2 s conductor ask, see the 2026-10-02 observation).
- `just gate doorway` (fmt-check, clippy `-D warnings`, test, `RUSTFLAGS=""`): EXIT=0. The lib suite
  passed 1763, 0 failed, 2 ignored, including the 2 new `ssr::tests` for the guard
  (`a_lookup_dropped_before_it_answers_is_named_as_abandoned`,
  `a_lookup_that_answered_is_not_named_as_abandoned`). Those two test the guard itself; they do not
  prove that `reqwest` lets go of the lookup future when a fetch is cut.
- `python3 .claude/scripts/_lib/__tests__/runtime_harvest_test.py`: 71 assertions passed, including the
  measured alpha window (silent) and a restart followed by a real storm (fires, `4/8`).
- Closure: left to the poller, by disappearance. The ledger line is `triaged` and is not manually
  removed: the stalls are real and unresolved, so a recurrence must be free to file.


# 2026-10-13: `da8bb3bdd7e1` fired a fourth time; both instruments read out, and the name lookup is not where the time goes

## What is exhausted

Ledger line (fp `da8bb3bdd7e1`, node **alpha**, filed poll 219, `2026-10-13T23:21:18+00:00`):

```
render degenerate 18/50 of NEW renders = 0.36 over 6 of the last 8 polls (SSR stalled/timedOut saturation)
```

Re-fetch at triage, `GET https://doorway-alpha.elohim.host/admin/render-stats` (23:26:48Z, HTTP 200):

```json
{"total": 52, "rendered": 34, "renderedEmpty": 0, "stalled": 18, "timedOut": 0,
 "errored": 0, "avgWallMs": 516, "maxWallMs": 1270, "degenerateRate": 0.346}
```

`GET https://elohim.host/admin/render-stats` the same second: `total: 50, stalled: 0, maxWallMs: 1127`.
Both doorways run the same image (`/version`: commit `85ad63f`, built 2026-10-12T11:23:08Z) and
started together (`/health` uptime 127327 s on alpha, 127272 s on alpha-b: about 2026-10-12T12:04Z).
`/admin/self-healing` on alpha: `admission.shedTotal: 0`, upstream `circuit: closed`, `errorStreak: 0`,
`warmup.completed: true`, `conductor.connectedWorkers: 4/4`, `projector.caughtUp: true`.

The stored window (`.claude/data/runtime-cursor.json`, poll 219) runs `total` 2 -> 52 and `stalled`
0 -> 18 on alpha, and `total` 2 -> 50 with `stalled` 0 on alpha-b. The poller is hook-driven, so the
8 samples are not evenly spaced: the oldest sample (`total: 2`) is from just after the 2026-10-12
start. **The 18 stalls are the whole life of this process, about 35 hours, not a recent burst.**
The base is honest (18 events, floor 3; 6 moving polls). The predicate is right again; leave it alone.

## Every stall, from the 2026-09-28 instrument

Loki, `{namespace="elohim-alpha"} |= "degenerate_fetches"`, 2026-10-12T12:00Z -> 2026-10-13T23:30Z:
**18 lines, not truncated, all from pod `elohim-doorway-alpha-66cd864646-hxs7s`**, all `path: "/"`,
`terminal: "stalled"`, `wall_ms` 1226-1270. The count equals the counter. Abbreviations: R x =
`GET /api/v1/resilience/concept-path-forward-x`, E x = `GET /epr-head/concept-path-forward-x`,
E manifesto = `GET /epr-head/manifesto`; every fetch reads `(stalled 1200ms)`.

| # | time (UTC) | `wall_ms` | fetches named |
|---|---|---|---|
| 1 | 10-12 13:22:52 | 1270 | R policymakers |
| 2 | 10-12 13:34:53 | 1257 | R developers, E communities, R communities, E manifesto |
| 3 | 10-12 18:52:18 | 1239 | R developers, E communities, R communities, E manifesto |
| 4 | 10-13 01:50:24 | 1261 | R developers, E communities, R communities, E manifesto |
| 5 | 10-13 05:06:07 | 1265 | R policymakers, E developers, R developers, E communities, R communities, +1 more |
| 6 | 10-13 05:45:33 | 1261 | R communities, E manifesto |
| 7 | 10-13 06:08:37 | 1258 | R developers, E communities, R communities, E manifesto |
| 8 | 10-13 07:40:32 | 1247 | R communities |
| 9 | 10-13 09:47:40 | 1240 | E developers, R developers, E communities, R communities, E manifesto |
| 10 | 10-13 12:08:57 | 1226 | R policymakers, E developers, R developers, E communities, R communities, +1 more |
| 11 | 10-13 12:42:20 | 1243 | R policymakers, E developers, E communities, R communities, E manifesto |
| 12 | 10-13 12:57:52 | 1251 | R developers, E communities, R communities, E manifesto |
| 13 | 10-13 14:49:59 | 1248 | R developers, E communities, R communities, E manifesto |
| 14 | 10-13 16:44:35 | 1238 | R developers, E communities, R communities, E manifesto |
| 15 | 10-13 20:01:07 | 1263 | R policymakers, R developers, E communities, R communities, E manifesto |
| 16 | 10-13 20:14:31 | 1227 | R policymakers, E developers, R developers, E communities, R communities, +1 more |
| 17 | 10-13 21:22:58 | 1269 | R policymakers, E developers, R developers, E communities, R communities, +1 more |
| 18 | 10-13 22:17:52 | 1256 | R communities, E manifesto |

Same six fetches as on 2026-10-02, and the same tail-of-the-sequence shape. Three refinements:

- **The tail is not always a clean suffix.** Rows 1 and 8 name one fetch that is not the last of the
  sequence (the fetches issued after it arrived). Rows 11 and 15 skip one in the middle. So it is
  "some of the last six", weighted to the end, not "everything after a point".
- **About one cold render in three, steadily.** 18 of 52 over 35 h, in ones and twos, hours apart.
  No burst, no trend.
- **The 19:15Z pin removal is neither the cause nor a cure.** The operator removed 1,063 retired
  `epr:e2e-*` pins from matthew at about 19:15Z on 10-13 (the doorway logged the `DELETE /api/v1/pins/<n>`
  calls from 19:14:10Z). Rows 1-14 predate it by up to 30 hours; rows 15-18 follow it. No deploy rolled
  the fleet on 10-13 either. The condition started with the process on 10-12 and continued through.

## The 2026-10-02 instrument (`doorway::ssr::dns`) read out for the first time

This is the first firing on an image that carries `TimedDnsResolver` (`7fe05122f` is an ancestor of
the deployed `85ad63f`). Loki, `{namespace="elohim-alpha"} |= "ssr::dns"`, same range: **one line**.

```json
{"timestamp":"2026-10-12T22:49:41.360593Z","level":"WARN","fields":{"message":"SSR render fetch: slow name lookup",
 "host":"elohim-matthew-alpha.elohim-alpha.svc.cluster.local","elapsed_ms":12552954,"addresses":1},"target":"doorway::ssr::dns"}
```

- **No stall has a `doorway::ssr::dns` line within 5 s of it.** The nearest stalls to this line are
  3 h 57 min before and 3 h 1 min after. By the rule this record set on 2026-10-02, the lookup is
  ruled out. That rule needs the qualification in the next section before it is believed.
- **`elapsed_ms: 12552954` is 3 h 29 min.** The lookup succeeded (`addresses: 1`). No `getaddrinfo`
  takes that long. The clock started at about 19:20:28Z on 10-12 and stopped at 22:49:41Z; in between
  the future was simply not polled. That is the render thread: `elohim/elohim-render/src/angular.rs`
  runs each render on a `current_thread` runtime that goes back to a blocking channel recv between
  renders, so anything still pending when a render returns sits unpolled until the next render lands on
  that thread. The line is the residue of one fetch that was still in flight when its render returned.
  It measures how long a render thread sat idle (this node renders about 1.5 times an hour), not a slow lookup.

## Storage is ruled out again, by the same histogram

Prometheus, `elohim_http_request_duration_ms`, pod `elohim-matthew-alpha-0`, hourly from 10-12 12:00Z
to 10-13 23:00Z: requests slower than 1 s (`_count` minus `_bucket{le="1000.0"}`) for
`route_class="api"`: **0 in every hour**. Eleven or more `api`-class fetches a day are cut at 1200 ms by
the doorway and storage never held one for a second. For `content`: 93 in the 12:00-13:00Z hour on 10-12
(the start) and 0.04 in the 20:00-21:00Z hour on 10-13, 0 otherwise. `elohim_http_requests_in_flight`
max over two days: 29 on one instance of the pod, which is the fetch count of one `/` render arriving at once.

## Root-cause inventory (this pass)

1. **Unchanged from 2026-10-02:** `init_ssr_render_client` (`doorway/doorway-service/src/server/http.rs`)
   is pool-free on purpose, so one `/` render opens about 29 fresh connections with 29 fresh lookups,
   all at once, on the render thread own runtime. The proxy client, pooled, on the main runtime, has never
   shown this.
2. **`doorway/doorway-service/src/ssr.rs`, `TimedDnsResolver::resolve` — the instrument has a blind
   spot (read from code, not measured).** It logs only when the lookup future *completes*.
   `TracingFetcher::fetch` (`elohim/elohim-render/src/traced_fetcher.rs`) wraps the whole inner fetch in
   `tokio::time::timeout(soft, ...)`, and at the cut the inner future is dropped, the pending lookup with
   it. A lookup that would have taken longer than the 1200 ms budget therefore logs nothing. So "no
   `doorway::ssr::dns` line beside the stall" excludes a lookup of 250-1200 ms that then completed; it
   does not, on its own, exclude a lookup that hung past the budget (for example a 5 s glibc resolver
   retry after a dropped UDP query; the image is `debian:bookworm-slim`, the manifest sets no
   `dnsConfig`, and the upstream name has four dots under the default `ndots:5`, so each lookup walks the
   search list first).
3. **What argues against the lookup anyway (inferred):** the stalled fetches are a subset of 29 lookups
   of the *same name* issued in the same few milliseconds on the same blocking pool; 23 or more of them
   resolve and complete in well under the budget every time. And the fetches that stall are always from
   the same six, late in the issue order. A resolver fault would not pick by position.
4. **What the 3.5 h line adds (measured for one fetch, inferred as a mechanism):** on this path, time
   spent *not being polled* is indistinguishable from time spent waiting on the network. Every phase of a
   render fetch (lookup wakeup, connect, request write, response read) advances only when the render
   thread polls it, and that thread also runs the V8 isolate. The third candidate from 2026-10-02 ("the
   render thread `current_thread` runtime not polling the tail fetches") is now the leading one: it is the
   only candidate that explains why position in the issue order matters, why storage sees nothing slow,
   and why the lookup instrument is quiet. **Not proven.** What it does not explain: why adam, on the same
   image and bundle path, shows 0 of 50.
5. **Unplaced observation:** in two of six +/-60 s windows around stalls, storage logged
   `Connection error ... connection closed before message completed` from `10.1.58.42` (13:34:43Z on
   10-12, ten seconds before row 2; 20:14:43Z on 10-13, twelve seconds after row 16; twelve seconds is
   `STORAGE_PROXY_REQUEST_TIMEOUT_SECS`). If `10.1.58.42` is the doorway-alpha pod, storage accepted a
   connection on which a complete request never arrived: the connect succeeded and the write did not
   happen. Whose address that is was not established (it needs a cluster read).

## Fix path

No cure is chosen, because the phase is still not measured. The next move is one more instrument, and it
must be one that reports at the cut instead of at completion:

1. **A drop guard on the lookup.** In `TimedDnsResolver::resolve`, hold a small guard across the
   `lookup_host` await that is disarmed on completion and, in `Drop`, emits
   `warn!(target: "doorway::ssr::dns", host, elapsed_ms, "SSR render fetch: name lookup abandoned before it answered")`.
   This closes the blind spot in item 2: after it, a stall with no `doorway::ssr::dns` line really does
   rule the lookup out.
2. **A phase guard on the fetch.** In `ResolverFetcher::fetch` (`doorway/doorway-service/src/ssr.rs`),
   a guard that records the phase reached (`sending` -> `headers` -> `body`) and how many times the future
   was polled, and on drop before completion emits one `warn!` on a new target `doorway::ssr::fetch` with
   `path`, `phase`, `elapsed_ms`, `polls` and `max_poll_gap_ms`. Reading: `phase=sending` with a large
   `max_poll_gap_ms` is the render thread not polling; `phase=sending` polled steadily with no dns line is
   connect or write; `phase=headers` or `body` is storage or transit after all.
3. **Then the cure, by that reading.** Render thread starving its own fetches: drive the render fetches
   from a runtime that is not the isolate thread (a dedicated fetch runtime that the render thread awaits
   through a channel keeps the pool-free containment and removes the starvation). Connect: a short-TTL
   address cache plus a bounded connect retry inside the budget. Neither is designed here.
4. Unchanged refusals: do not widen the soft budget, do not touch `_render_degenerate`, do not raise
   `DEFAULT_SSR_RENDER_PERMITS`, do not re-pool the render client.

## Current decision (supersedes 2026-10-02)

**BLOCKED. The lookup instrument has been read and is quiet; the next instrument is specified and not
built.** This pass ran under a no-cargo constraint (the storage build slot was held and the machine was
loaded), and both guards are Rust in `doorway-service` that no gate has seen. Owed, in order:

1. Implement items 1 and 2 above and run `just gate doorway` (`RUSTFLAGS=""`).
2. An edge deploy that carries them (operator-initiated; this agent does not roll the fleet).
3. One Loki read after the next few stalls: `{namespace="elohim-alpha"} |= "doorway::ssr::fetch"` beside
   `|= "degenerate_fetches"`.
4. Operator, optional and cheap: whether `10.1.58.42` is the doorway-alpha pod address (item 5).

What the poller cites on re-encounter: this record. **This fingerprint is a standing condition of the
alpha doorway, about one cold `/` render in three served as a degenerate HTTP 200, since at least
2026-10-02.** It is not tied to a deploy, a head swap, or the 2026-10-13 pin removal. It will not close
by disappearance while the process lives, because the delta window keeps collecting stalls. A re-file
needs no new triage until the phase guard is deployed; the reading in step 3 is the next triage.

## Verification

- 2026-10-13 23:26:48Z: `/admin/render-stats`, `/admin/self-healing`, `/health`, `/version` re-fetched on
  both doorways (HTTP 200, quoted above). The cursor window was read from disk. **The condition is live.**
- Loki and Prometheus readings as quoted, taken about 23:30Z through the observability datasource.
- No cache-busted render probes and no `/head` reads were made this pass (read-only status surfaces
  only), so this triage added nothing to either node counters. The landing head age was therefore not
  read; the process age (35 h) and the spread of the stalls carry the "not a cold window" reading.
- Nothing was built, so nothing is claimed fixed. The ledger line is `blocked` and is not deleted.

# 2026-10-19: both owed instruments land, with the 2026-10-09 harvester fix

The 2026-10-09 section above and the 2026-10-13 section were written on separate lines of work and
met at the 2026-10-18 integration pass. The 2026-10-13 fix path's two instruments are now on dev:

1. **The drop guard on the lookup** (fix path item 1) is `LookupInFlight` in
   `doorway/doorway-service/src/ssr.rs`, from 7e7b5d867 (the 2026-10-09 commit): a lookup cut before it
   answers warns "name lookup abandoned before it answered" with host and elapsed_ms.
2. **The phase guard on the fetch** (fix path item 2) is the `doorway::ssr::fetch` phase handle from
   c9f9f8ea0, cherry-picked from perf/call-floor-2: an abandoned fetch names the phase it reached and
   whether it was woken.

The 2026-10-09 change to `_render_degenerate` lands with them. It sums the delta per poll transition, so
a pod restart inside the window no longer subtracts one process's counters from another's. It leaves
DEGEN_RATE, DEGEN_POLLS and DEGEN_MIN_EVENTS alone. Refusal 4 above ("do not touch
`_render_degenerate`") guards against tuning the predicate to make the finding go away, and this change
does not tune it.

**Current decision:** still BLOCKED on fix path step 2, an edge deploy that carries both guards. The
dev landing reaches alpha through the next edge build. Then do step 3, the Loki read beside
`degenerate_fetches`.
