---
id: "backlog-ci-doorway-dockerfile-fixture-context"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway image check stage misses elohim/sdk/fixtures — cargo test --lib --bins can't read include_str! vectors"
slug: "ci-doorway-dockerfile-fixture-context"
written: "2026-06-06"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [ef52f70f4178]
jobs: [elohim-edge]
relatedNodeIds: []
tags: [ci, elohim-edge, doorway, dockerfile, build-context, museum-trap-3, host-green-not-ci-green]
cites:
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1043/
  - doorway/doorway-service/Dockerfile
  - doorway/doorway-service/src/projection/epr_router.rs
  - doorway/doorway-service/src/server/http.rs
  - elohim/sdk/fixtures/route-claims.vectors.json
  - elohim/sdk/fixtures/spa-route-discrimination.vectors.json
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# Doorway image `check` stage under-covers the build context (missing test fixtures)

## The failure

`elohim-edge` build #1043, stage **Quality Gate: Doorway** (Docker `check`
target, Dockerfile line 84 `RUN RUSTFLAGS="" cargo test --lib --bins`):

```
error: couldn't read `/app/../../elohim/sdk/fixtures/route-claims.vectors.json`: No such file or directory (os error 2)
   --> src/projection/epr_router.rs:408:19
error: couldn't read `/app/../../elohim/sdk/fixtures/spa-route-discrimination.vectors.json`: No such file or directory (os error 2)
   --> src/server/http.rs:1396:19
error: could not compile `doorway` (lib test) due to 2 previous errors
ERROR: process "/bin/sh -c RUSTFLAGS=\"\" cargo test --lib --bins" did not complete successfully: exit code: 101
```

Occurrence evidence: seen 1, first_build 1043, last_build 1043 (job elohim-edge).
The stage is wrapped `catchError(... non-blocking ...)`, so the failure surfaces
as build **UNSTABLE**, not FAILURE — but the doorway image is never built from the
`check` target on a failed test, so this is a real broken gate, not noise. (The
same build also went UNSTABLE on an unrelated `P2P Simulation Test` stage —
`docker-compose: command not found`, exit 127 — which is a *separate* concern not
captured as its own fingerprint and not addressed here.)

## Verdict

**real — Dockerfile build-context completeness gap** (museum trap #3:
"Dockerfile / build-manifest completeness — a new path-dep breaks the Docker build
context but passes host pre-push"). See
`genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`
row 3 and the host-green ≠ CI-green cluster.

## Root cause

Two `#[cfg(test)]` modules in doorway-service consume shared test-vector fixtures
at compile time via `include_str!`:

- `src/projection/epr_router.rs:408` → `concat!(env!("CARGO_MANIFEST_DIR"), "/../../elohim/sdk/fixtures/route-claims.vectors.json")`
- `src/server/http.rs:1396` → `concat!(env!("CARGO_MANIFEST_DIR"), "/../../elohim/sdk/fixtures/spa-route-discrimination.vectors.json")`

`CARGO_MANIFEST_DIR` is `/app` inside the image, so the path resolves to
`/elohim/sdk/fixtures/...`. The Dockerfile `check` stage copies only
`doorway/doorway-service/src` and `templates` — never `elohim/sdk/fixtures` — so
the test build cannot read the vectors and aborts with exit code 101.

Because the `include_str!` calls are **test-only**, `cargo build --release` (the
`deps` and `builder` stages, which produce the actual binary) compiles fine; the
gap surfaces **only** at `cargo test --lib --bins`. And on the host (pre-push /
local), `elohim/sdk/fixtures/` exists at the real relative path, so the gate is
green there — the textbook host-green ≠ CI-green disguise. The fixtures landed via
the §4/§8.4 two-layer-drift-guard work (`385a7485a` doorway slice-3 + `74499bcd6`
SDK vectors); the Dockerfile COPY set was never extended to match.

## Current decision

**Bounded fix landed (local-verified), awaiting CI disappearance confirmation.**
Added `COPY elohim/sdk/fixtures /elohim/sdk/fixtures` to the Dockerfile `check`
stage (before the `cargo test` line). Destination is absolute `/elohim/sdk/fixtures`
to match the `include_str!` normalization of `/app/../../elohim/sdk/fixtures`.

Latent sibling (NOT this concern, NOT fixed here): `elohim-storage` also
`include_str!`s the same two fixtures (`src/db/rea_commitments.rs:1812`,
`src/http.rs:11054`, via the crate-local `/../sdk/fixtures/...` path). Its image
(`elohim/elohim-storage/Dockerfile`, elohim-holochain/DNA job) may carry the same
gap; that job is green at the current cursor, so it's noted for the operator, not
opened as a finding.

## Fix trail

- `doorway/doorway-service/Dockerfile` — added `COPY elohim/sdk/fixtures /elohim/sdk/fixtures`
  to the `check` stage with an explanatory comment.
- Local verification (host, `RUSTFLAGS=""`): both fixture-consuming tests pass —
  `projection::epr_router::tests::reserved_prefixes_fixture_agrees_with_is_service_path`
  and `server::http::shakeout_tests::shakeout_is_spa_route_agrees_with_shared_vectors`
  (`test result: ok. 2 passed`). This confirms the fixtures parse and the code is
  correct; the CI failure was purely the missing build-context COPY. Full Docker
  `check` build not run locally (no Docker-in-dev); the COPY path arithmetic is
  verified against the `include_str!` normalization above.
- Commit-only (integrator pushes; a `[build:edge]`-tagged integrator push will
  rebuild the doorway image and confirm by green streak).
