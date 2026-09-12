---
id: "backlog-ci-genesis-hosted-provision-conductor-admin-socket-abort"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A dropped admin websocket on ONE conductor refuses a hosted human's registration with 503 PROVISIONING_FAILED while the rest of the pool sits unasked — BrokenPipe and ConnectionAborted are transport faults, and the provisioner surfaced them as verdicts"
slug: "ci-genesis-hosted-provision-conductor-admin-socket-abort"
written: "2026-09-12"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [885aee5f78f7, 46c20a2dfea8]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, elohim-genesis, doorway, hosted-registration, provisioning, conductor, admin-websocket, transport-fault, shed-is-not-a-verdict, museum-trap-12, alpha]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1576/
  - https://jenkins.ethosengine.com/job/elohim-orchestrator/job/dev/1852/
  - doorway/doorway-service/src/conductor/provisioner.rs
  - doorway/doorway-service/src/conductor/registry.rs
  - genesis/a2o/features/deployment/doorway-portal-login-neighbourhood.feature
  - genesis/a2o/features/deployment/staging-validation.feature
  - genesis/data/timeline/backlog/ci-deploy-reads-storage-backpressure-as-failure.md
  - genesis/data/timeline/backlog/ci-genesis-conductor-adminws-unreachable.md
  - genesis/data/timeline/backlog/ci-genesis-agent-bindings-conductor-fanin.md
  - genesis/data/timeline/backlog/conductor-websocket-flap-breaks-deploy-write-path.md
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# One conductor drops its socket; the whole pool answers 503

## The failure

`elohim-genesis/dev` **#1576** (UNSTABLE), browser-scenario suite:
`92 scenarios (5 failed, 4 pending, 81 skipped, 2 passed)`. Three of the five failures
are this concern. Ledger `885aee5f78f7` and `46c20a2dfea8`, each seen 1, build 1576..1576.

```
AssertionError [ERR_ASSERTION]: could not register a hosted human on "alpha": 503
{"error":"Agent provisioning failed: Failed to install app on conductor-3: Admin error
 (install_app): External API wire error: InternalError(\"Conductor returned an error while
 using a ConductorApi: Other({\\\"error\\\":\\\"ConnectionAborted\\\"})\")",
 "code":"PROVISIONING_FAILED"}
```
```
AssertionError [ERR_ASSERTION]: could not register a hosted human on "alpha": 503
{"error":"Agent provisioning failed: Failed to generate agent key on conductor-3: Admin
 error (generate_agent_pub_key): External API wire error:
 InternalError(\"Other: {\\\"error\\\":\\\"BrokenPipe\\\"}\")",
 "code":"PROVISIONING_FAILED"}
```

Scenarios: `doorway-portal-login-neighbourhood.feature:27` (portal renders and the doorway
signs the human in), `:35` (a wrong password is refused), and
`staging-validation.feature:28` (git hash validation), the last reaching the same
`install_app`/`ConnectionAborted` through `doorway-session-client.ts:173`.

**The diagnostic tell.** A regex sweep of the whole 5,181-line console for
`conductor-[0-9]` returns **exactly three matches, all `conductor-3`**, all from the two
error strings above. Every other member of the pool is absent from the log because none
was ever asked.

Occurrence evidence, #1570–#1576: this signature appears in **#1576 only**. #1570–#1573
and #1575 are UNSTABLE for other reasons and carry no `PROVISIONING_FAILED`, no 503
assertion, no `EprRouter`/`warm_stream`/`caught_up` error shape (only the two routine
post-seed `EprRouter` success lines). #1574 is ABORTED.

Preconditions were green before the failing steps: `✅ storage caught up (healthy +
caughtUp + projection.writer)`, `✅ EprRouter populated without a restart — /health 200 +
conductor-connected`, `✅ Target site responding: https://alpha.elohim.host`.

#1576 carries no SCM changeset of its own (downstream of `elohim-orchestrator/dev`
#1852). #1852's 382 changed paths contain **zero** matches for
`doorway|conductor|deployments\.json|provision|agent` — its eight commits are
memory/governance/docs/pre-push work. Not a regression from the changeset.

## Verdict

**Real, in-tree — a caller-side classification defect, triggered by (not caused by) a
substrate transport fault.**

`ConnectionAborted` and `BrokenPipe` are what a socket says when it dies, not what a
conductor says when it refuses. In both cases the conductor never answered, so **nothing
is established about the work** — least of all that it cannot succeed on a different
member of the pool. This is the same class the substrate trust contract already names in
elohim-storage's `conductor_admission` ("a shed establishes NOTHING about the work — the
conductor never saw it — so a caller must route it exactly like backpressure, never like
a failure verdict"), and the same class the seed-blob forward learned on 2026-09-02.

It is **not** `ci-genesis-conductor-adminws-unreachable` (that concern is admin-WS TCP
refused from CI at the *probe*, fleet-wide, and is closed `verified`) and it is **not**
`conductor-websocket-flap-breaks-deploy-write-path` (that is the doorway↔conductor **app**
interface flapping and 503ing the deploy write path on ALL hosts). Here one pod's **admin**
socket died mid-call during hosted provisioning, and the doorway was otherwise healthy.

conductor-3's own pod state — restarts, OOM, readiness — is **not observable from the
Jenkins artifact** and is not claimed here. It would need a Loki/Prometheus read scoped to
the `#1576` window (build start ≈ 2026-09-11T13:23Z, ~43.6 min). The in-tree defect does
not depend on that answer.

## Root cause

`doorway/doorway-service/src/conductor/provisioner.rs`, `provision_agent`: a single
straight-line attempt against a single conductor, with every failure `map_err`'d into a
string and returned.

1. `find_existing_app` runs **once**, at the top.
2. `registry.find_least_loaded()` picks **one** conductor.
3. `generate_agent_pub_key` / `install_app` / `enable_app` each `return Err(...)` on the
   first failure, with no classification.
4. `auth_routes.rs` renders that string as `503 PROVISIONING_FAILED` to the human.

So a transport fault on the least-loaded member is indistinguishable, at every layer
above, from "this pool cannot host you". The pool had other members; the budget to ask
them did not exist.

This is **museum trap #12 one seam out**. Trap #12 is first-reachable-wins degenerating
to *index 0 always wins* when every candidate matches the predicate. Here the predicate is
capacity, the selection is correct, and the missing piece is the other half of the same
lesson: **a pool selector needs an exclusion set, or a single sick member absorbs every
attempt.** The trap's stated diagnostic tell — *every failing row names the same target* —
fires verbatim on this log.

## Current decision

**Fixed in-tree, locally verified; awaiting disappearance on a green streak.** Ledger
`885aee5f78f7` and `46c20a2dfea8` → `status: triaged`, `triaged_at_build: 1576`.

No `decompose_on_confirm` stamp. This is the site at which the recurring lesson earned a
museum row — *an error that establishes nothing about the work must never be surfaced as
a verdict, and the cure has to be applied per answering site, not once* — graduated as
trap #19. Read the museum before deleting this entry.

The fix cannot be confirmed by triggering a build (Jenkins MCP is anonymous); confirmation
rides the integrator's next genesis dispatch. Expect the next transport fault to print the
doorway-side warning `conductor admin socket dropped mid-provision — re-offering to
another conductor` and the registration to succeed on a sibling.

## Fix trail

`doorway/doorway-service/src/conductor/provisioner.rs` — a dropped admin socket is a
re-offer, not a refusal:

- `provision_agent` is now a bounded loop over `PROVISION_TRANSPORT_ATTEMPTS` (3). Each
  attempt re-runs `find_existing_app` **first** — an `install_app` whose socket died may
  have landed before it died, and a blind retry would otherwise install a second app for
  the same human.
- `provision_on(&conductor, user)` holds the previous single-attempt body verbatim
  (capacity check, connect, generate key, install, enable, cell-genesis poll, register).
  No step changed; only who calls it and how many times.
- `is_transport_fault(&str)` classifies `BrokenPipe`, `ConnectionAborted`,
  `ConnectionReset`, `ConnectionRefused`, `connection closed`/`Connection closed`,
  `websocket closed`, `Failed to connect to admin`. Matched on the rendered string
  because that is the only shape `holochain_client`'s admin errors reach this layer in
  (the discriminant is already flattened into text upstream). Asymmetric by design: a
  false positive costs one extra attempt on a different conductor; a false negative costs
  a human their registration.
- A transport fault **excludes** that conductor and re-offers; anything else (at capacity,
  cell genesis timed out, enable failed, registry write failed) returns immediately and
  unchanged — those are verdicts.

`doorway/doorway-service/src/conductor/registry.rs`:

- `find_least_loaded_excluding(&[String])` added; `find_least_loaded()` delegates to it
  with an empty slice, so the existing callers are byte-identical in behaviour.

Tests (4 new):

- The two #1576 error strings, verbatim from the build log, must classify as transport
  faults.
- Four verdict strings (at capacity, cell-genesis timeout, enable failure, registry write
  failure) must NOT — a verdict must never be retried into a second side effect.
- A two-member pool: the re-offer must reach a DIFFERENT conductor, and excluding every
  member must yield `None` rather than wrap around.

Local verification: `cargo test --lib` for doorway-service — **1278 passed, 0 failed**
(the pre-existing `test_provision_no_conductors` still passes: an empty pool still returns
`No conductors available for provisioning`); `cargo clippy --lib --tests -- -D warnings`
EXIT=0; `cargo fmt --check` EXIT=0.

## Done when

- [x] A transport fault on one conductor's admin socket is re-offered to another
- [x] A conductor verdict is still returned immediately and unchanged
- [x] The idempotency search re-runs per attempt so a re-offer cannot double-install
- [x] Lesson graduated into the anti-patterns museum (trap #19)
- [ ] `885aee5f78f7` / `46c20a2dfea8` absent from an `elohim-genesis/dev` green streak ≥3
- [ ] Not addressed here: conductor-3's pod-level cause (restarts/OOM/readiness). Needs a
      Loki/Prometheus read, is operator-owned, and the re-offer makes a single sick member
      survivable rather than curing it. If MULTIPLE conductors start dropping, this entry
      is the wrong home — that is a fleet condition.
