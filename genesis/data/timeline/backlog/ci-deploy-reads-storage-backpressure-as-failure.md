---
id: "backlog-ci-deploy-reads-storage-backpressure-as-failure"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The App deploy reads storage BACKPRESSURE as a failure verdict at two seams — a conductor-admission shed reached the canonical-head declare as a 502, and a shed read-back reached the blob stage as forwarded_to_storage:false — so a saturated-for-seconds alpha left one host STALE and scenario 2 red"
slug: "ci-deploy-reads-storage-backpressure-as-failure"
written: "2026-09-02"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [f8003a16a985, 5ee767181c07]
jobs: [elohim]
relatedNodeIds: []
tags: [ci, elohim-app, deploy, upload-spa-blob, canonical-head, conductor-admission, shed, backpressure-not-verdict, honest-shed, alpha, doorway-b, admission-floor, scenario-2]
cites:
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1683/
  - scripts/ci/stage-spa-blob.sh
  - Jenkinsfile
  - elohim/elohim-storage/src/conductor_admission.rs
  - elohim/elohim-storage/src/services/response.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-storage/src/api/source_chain.rs
  - doorway/doorway-service/src/routes/seed.rs
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - genesis/data/timeline/backlog/blob-put-large-body-shard-verify-drop.md
  - genesis/data/timeline/backlog/alpha-spa-blob-patch-503.md
  - genesis/data/timeline/backlog/ci-alpha-cluster-degraded-substrate.md
---

# One lesson, two seams: a shed is not a verdict

## The failure

`elohim/dev` **#1683** (UNSTABLE, 2026-09-02T06:48Z, 57.7 min, upstream
`elohim-orchestrator/dev` #1784). Both fingerprints were emitted inside the
**Upload SPA Blob** stage (log lines 5894–8055); no other stage failed. Both are
first-seen: neither signature appears in #1682, #1681, #1680, #1678 or #1675
(four of which reached the same stage with full-length logs).

**Facet A — `f8003a16a985`** (seen 1, builds 1683..1683). One non-retried declare
against alpha, immediately after that blob's own upload had SUCCEEDED:

```
✓ [elohim-host-landing] blob uploaded (via /admin/seed/blob)
✓ verified elohim-host-landing serverBlobHash = sha256-57dd4d3205…
⚠ canonical-head declare FAILED — POST https://alpha.elohim.host/db/content/elohim-host-landing/canonical-head
returned HTTP 502: {"error":"Request timeout: conductor admission: shed: no conductor permit for
content_store within 5000ms (class=interactive, capacity=5, in_flight=5) — nothing was dispatched"}
— coordinator may be pre-cure; scenario 2 will stay red
```

Read the body against the status: it says **`nothing was dispatched`** while the
status says **502 Bad Gateway** — "the upstream answered badly". Only one of
those is true. The caller believed the status. (The ledger line reads `within #`
— `ci-harvest`'s duration normalizer, not a message bug.)

The sibling declare on the same host (`lamad-spa`, line 7915) failed the same
way on a **503 `{"status":"catching-up","retryAfter":30}`** — the leg had no
retry for *any* status, so it surrendered on a host that had literally just told
it when to come back.

**Facet B — `5ee767181c07`** (seen 1, builds 1683..1683). Seven occurrences
across three staging invocations of the **3,081,219-byte** `lamad-spa` browser
bundle:

| invocation | host | outcome |
|---|---|---|
| 1 | alpha.elohim.host | fail, fail, **succeed on 3/3** |
| 2 | elohim.host | fail, fail, **succeed on 3/3** |
| 3 (same hash re-staged) | alpha.elohim.host | fail ×3 → `host left STALE` → `authorHeadOnce` failover |

```
{"success":true,"hash":"sha256-9f7dfbefae…","already_cached":false,"forwarded_to_storage":false,"size":3081219}
  ✗ [lamad-spa] doorway cached the blob but storage forwarding FAILED (forwarded_to_storage:false) — refusing to call this staged
  ⚠ [lamad-spa] attempt 1/3 against https://alpha.elohim.host failed — retrying in 5s
… ERROR: [lamad-spa] stage failed after 3 attempt(s) against https://alpha.elohim.host — host left STALE
```

## Verdict

**real — one in-tree classification defect at two seams, triggered by (not
caused by) alpha saturation.** Three candidate homes were checked and ruled out:

- **Not the shard-verify class** (`blob-put-large-body-shard-verify-drop`, the
  atom whose mitigation emits facet B's line). That concern is a 69 MB bundle
  crossing `MAX_INLINE_SIZE` (16 MB) into the sharded regime, failing
  *deterministically* with an empty reply. This bundle is **3 MB** — inline, not
  sharded — and the **identical hash succeeded on a later attempt on both
  doorways**. Fails-then-succeeds on the same bytes is a load signature, not a
  durability signature.
- **Not the doorway-B admin-key 403** (`ci-app-apex-seed-403-doorway-b-admin-key`).
  #1683's log contains **no 403 at all** (the one literal "403" is a file size in
  an `ls -la`). Facet B hit BOTH doorways, and elohim.host recovered on retry.
- **Not the degraded-substrate concern** (`ci-alpha-cluster-degraded-substrate`).
  `cluster-state.yaml` has carried `alpha-cluster-6peer: available: true` since
  2026-08-20 (7/7 peers up, zero restarts). The peer plane is healthy; this is
  the T4/storage plane, and the defect is ours.

Nor is it the changeset: #1683's six commits touch only `epr-home` Angular
components and a2o step files — nothing in `scripts/ci/`, conductor admission,
or `content_store`.

The saturation itself is unremarkable and should be expected permanently.
`capacity=5` is the **admission floor**: `PERMIT_FLOOR` (8, mirroring the
conductor's own `calculate_default_db_max_readers` floor) minus
`CONDUCTOR_RESERVE` (3). An alpha storage pod sized at the floor saturates on
five concurrent interactive zome calls, so a deploy landing while a projection
sweep runs meets `in_flight == capacity` routinely. The gate was doing its job.
The bug is what the deploy did about it.

## Root cause

`conductor_admission` states the contract in its own source, and the repo has
already cured it **three times**:

> A shed establishes NOTHING about the work — the conductor never saw it — so a
> caller must route it exactly like backpressure (return to pending, retry with
> backoff), never like a failure verdict.

`services::response::error_response` classifies shed first, by marker, before
its `Timeout → 504` arm. `HttpServer::escaped_error_response` does the same
(cured 2026-08-18, after 4,538 sheds reached the wire as bare 500s).
`api::source_chain::zome_error` carries a regression test that the marker
survives the handler's wrap.

**Facet A — the three content head routes reach none of them.** `POST
/db/content/{id}/head` (declare arm), `POST /db/content/{id}/canonical-head` and
`GET /db/content/{id}/head-record` each build their conductor-error response
inline and `return Ok(...)`, so the error never *escapes* to a classifying seam.
All three collapsed every conductor error — shed included — to `BAD_GATEWAY`.
The classification was cured at every seam errors fall *through*, and at none of
the routes that answer for themselves; those three routes are precisely the
deploy's write path.

The caller-side half is sharper than "it didn't retry". `stage-spa-blob.sh`'s
**DECLARE_ONLY** fan-out ladder already knows this exact lesson, in its own
comment:

> `HTTP 503/429` — the peer's admission layer sheds writes while its reconcile
> drain holds the write pool. Backpressure clears in seconds-to-minutes, NOT
> structural: app #1612 aborted here at attempt 3/8 and elohim.host kept the
> superseded head for the whole day.

It keys that arm on **status 503|429 only**. Storage answered this shed with
**502**, so the one ladder in the repo written against this class fell through to
its "structural" arm and stopped. The CI knowledge and the storage classifier
disagreed about the wire shape of backpressure, and the disagreement was
invisible because each side was internally correct.

**Facet B — the same laundering, one layer out.** Doorway's `forward_to_storage`
returned a bare `bool`, folding four distinct conditions into `false`: PUT
refused, PUT unreachable, read-back non-2xx, read-back unreachable. The read-back
is deliberately strict — "forwarded means storage SERVES the blob, not that a PUT
returned 200", added after the 2026-08-16 vanishing-blob incident — and that
strictness is right. But a **shed on the read-back** says nothing about
durability: the PUT already succeeded, and storage merely declined to answer a
read. Folding it into `forwarded_to_storage:false` tells the deploy its bytes are
gone. The fails-fails-succeeds pattern on identical bytes, on both doorways,
during a window where facet A independently proves the same fleet was shedding,
is that false negative.

Compounding it: the response carried `error: None` on the forward-false path, so
`forwarded_to_storage:false` was the *entire* diagnosis available to CI. Which
leg refused was knowable only from doorway logs — the reason this fingerprint
needed a triage dispatch at all.

## Current decision

**Fixed in-tree at both seams and both callers; awaiting disappearance on a
green streak.** Ledger `f8003a16a985` and `5ee767181c07` → `status: triaged`,
`triaged_at_build: 1683`.

No `decompose_on_confirm` stamp on either. The lesson — *a route (or a function)
that answers its own errors opts out of every classification the shared seams
provide, and the cure has to be applied per answering site, not once* — is a
museum candidate if it appears at a fourth site. Read this entry before deleting
it.

The sentinel cannot trigger builds (anonymous MCP); confirmation rides the
integrator's next App push. Expect the first post-fix build to print
`⚠ canonical-head declare BACKPRESSURE (HTTP 502)` rather than `FAILED` — the
fleet's storage pods still carry the pre-fix 502 until the edge image rolls,
which is exactly why both CI ladders now match the shed marker in the body and
not only the status code.

## Fix trail

Storage — classify the shed at the routes that answer for themselves:

- `elohim/elohim-storage/src/services/response.rs` — new
  `conductor_write_error(&StorageError)`: shed → `admission_shed_backpressure()`
  (503 + `Retry-After` + `X-Available-Permits`, the existing one-home wire shape
  for both pools); everything else keeps its 502. Placed here, not in `http.rs`,
  because `http.rs` is over its LoC ceiling (architecture finding
  `a711583b7334`) and this module is already the declared home for the shed's
  wire shape.
- `elohim/elohim-storage/src/http.rs` — all three head routes now return
  `response::conductor_write_error(&e)`. The `/head` declare arm classifies shed
  *before* its `not the author` / `head delegation` / `not in the version chain`
  string arms; those substrings cannot appear in a shed, but the ordering is the
  contract, not an optimization.
- `elohim/elohim-storage/src/http.rs` `admission_egress_tests` — a `content_store`
  shed must reach the wire as 503 + `Retry-After`; a real conductor failure must
  still reach it as 502.

Doorway — separate shed from absence, and name the leg that refused:

- `doorway/doorway-service/src/routes/seed.rs` — `forward_to_storage` returns
  `Result<(), String>` instead of `bool`; `backpressure_wait` classifies 503/429
  (honouring `Retry-After`, clamped 1–5 s so a 30 s catching-up advert cannot
  park a deploy hop); `forward_once` re-offers up to
  `FORWARD_BACKPRESSURE_ATTEMPTS` (3) on backpressure at BOTH the PUT and the
  read-back, and returns a named reason otherwise. The 2026-08-16 read-back
  strictness is preserved untouched — a 404/500 read-back still reports failure.
- Same file — `BlobUploadResponse.error` is now populated whenever
  `forwarded_to_storage:false`, so the next occurrence names its own cause in the
  CI log instead of costing a triage dispatch.
- Tests: a refusal must name the leg; 503/429 classify as backpressure while
  404/500 do not; an advertised `Retry-After: 30` clamps to the ceiling.

CI — stop reading backpressure as structural:

- `scripts/ci/stage-spa-blob.sh`, advisory declare leg — bounded re-offer ladder
  (`CANONICAL_HEAD_ATTEMPTS`, default 4, linear 5 s backoff) on 503/429 **or**
  the shed marker in the body. Every arm still returns success; the leg remains
  advisory and cannot fail a deploy.
- `scripts/ci/stage-spa-blob.sh`, DECLARE_ONLY ladder — third retryable class
  added: the shed marker in the body, at the 30 s cadence its 503|429 sibling
  uses. This is the arm that would have saved #1683's fan-out.

Local verification: `bash -n` on the staging script; `cargo fmt`;
`cargo test --lib seed::` for doorway-service; the elohim-storage tests were run
in a detached worktree at HEAD + this diff, because `dev`'s working tree
currently carries unrelated in-flight edits under
`services/release_adoption/watch.rs` that do not compile (two `E0308`s against
`verify_envelope`) — that breakage is NOT from this change and is owned
elsewhere.

## Adjacent, deliberately not folded in

- `alpha-spa-blob-patch-503` (2026-06-27) — the PATCH leg 503ing on this same
  script. Same family, different leg, and that leg already retries. If it reds
  again post-fix, merge it here.
- `blob-put-large-body-shard-verify-drop` — still open and still right for the
  >64 MB sharded regime. Facet B is explicitly NOT that class; the two share only
  the log line, because that atom's mitigation is what emits it.
- The `capacity=5` floor. Raising it is not obviously correct:
  `conductor_admission`'s module doc requires `d(λ)/d(capacity) > 0` before
  anyone raises permits, and records a measurement where 17→34 left throughput
  flat. Sizing alpha's storage pods so `conductor_db_max_readers` clears the
  floor is an operator move, and it is a *mitigation* — the caller must survive a
  full gate at any capacity.
