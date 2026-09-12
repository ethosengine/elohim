---
id: "backlog-self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "alpha's SSR degenerate burst is REAL and deploy-coupled: for ~15min after a bundle-head swap, ONE fetch of the 29 in every cold `/` render exceeds the 1200ms soft budget, so every visitor in that window gets a degenerate HTTP 200 — and the stalling URL is unknowable from outside the process"
slug: "self-heal-alpha-ssr-post-bundle-swap-cold-fetch-stall"
written: "2026-09-12"
author: "runtime-triage"
status: "backlog"
priority: "medium"
self_heal_status: blocked
severity: medium
fingerprints: [da8bb3bdd7e1]
nodes: [alpha, doorway-alpha, elohim-matthew-alpha]
relatedNodeIds: []
tags: [self-heal, render-degenerate, ssr, soft-budget, bundle-head-swap, cold-window, elohim-render, doorway, observability-gap, true-positive, matthew, alpha]
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
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - .claude/scripts/_lib/runtime_harvest.py
  - genesis/data/timeline/backlog/self-heal-render-degenerate-cumulative-counter-false-positive.md
  - genesis/data/timeline/backlog/self-heal-doorway-alpha-storage-breaker-matthew-rekey.md
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
