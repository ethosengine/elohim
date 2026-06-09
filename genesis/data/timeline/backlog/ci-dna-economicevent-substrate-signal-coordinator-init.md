---
id: "backlog-ci-dna-economicevent-substrate-signal-coordinator-init"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "content_store coordinator's EconomicEvent initializer missed the substrate_signal field — DNA build E0063"
slug: "ci-dna-economicevent-substrate-signal-coordinator-init"
written: "2026-06-09"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [ffbbd932b6a9, 56ec1c027ba6]
jobs: [elohim-holochain]
relatedNodeIds: []
tags: [ci, elohim-holochain, dna-build, rea, economic-event, e0063, host-green-not-ci-green, museum-trap-3, cross-workspace]
cites:
  - https://jenkins.ethosengine.com/job/elohim-holochain/job/dev/1317/
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# content_store coordinator's `EconomicEvent` initializer missed `substrate_signal` — DNA build E0063

## The failure

`elohim-holochain/dev` build #1317 (FAILURE), DNA WASM build of the elohim
(lamad) DNA workspace:

```
error[E0063]: missing field `substrate_signal` in initializer of `content_store_integrity::EconomicEvent`
   --> zomes/content_store/src/lib.rs:12170:17
    |
12170 |     let event = EconomicEvent {
    |                 ^^^^^^^^^^^^^ missing `substrate_signal`
error: could not compile `content_store` (lib) due to 1 previous error; 17 warnings emitted
═══════════════════════════════════════════════════════════
DNA BUILD FAILED
═══════════════════════════════════════════════════════════
```

Occurrence evidence: seen 1, first_build 1317, last_build 1317 (job
elohim-holochain). Two fingerprints, **one concern**: `ffbbd932b6a9` is the
E0063 compile error itself; `56ec1c027ba6` ("DNA BUILD FAILED") is the
pipeline's failure banner echoing the same single compile error — the log
reads "due to **1** previous error", i.e. there is exactly one defect, not
two.

## Verdict

**real — cross-workspace field-addition drift; host-green ≠ CI-green**
(museum trap #3 cluster: "a new path-dep / new field added in one workspace
breaks another workspace's build but passes host pre-push", and the #71-73
host-green ≠ CI-green reading). See
`genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`.
No NEW recurring trap — known museum pattern, cited not re-derived.

## Root cause

The `substrate_signal: Option<String>` field was added to
`content_store_integrity::EconomicEvent` by the substrate-signal slice
(b7d0e501b). That slice's wasm-check covered the mishpat workspace, but the
**elohim (lamad) DNA coordinator's** create path —
`create_rea_economic_event` in `content_store/src/lib.rs`, the sole
struct-literal `EconomicEvent { ... }` initializer in that zome (line 461 is
`WireEconomicEvent`, a different struct) — was never updated. `just check` /
the pre-push native gate on the host did not exercise this WASM workspace, so
the gap surfaced only in the DNA CI build. Same shape as the doorway-fixture
concern: a check that passes on the host but fails in CI because the CI
environment compiles a workspace the host gate skipped.

## Current decision

**Bounded fix landed (local-verified), awaiting CI disappearance
confirmation.** Supplied `substrate_signal: None` in the coordinator's
`EconomicEvent` initializer. `None` = unspecified, old-chain compatible,
validator-accepted (the integrity validator explicitly
`accepts_absent_substrate_signal`). Origination on this create path is
**deliberately deferred** — server-derive-from-`lamad_event_type` is the open
column-vs-`metadata_json` decision tracked by the cluster-3 substrate_signal
migration plan; this fix only un-breaks the build, it does not decide
origination.

`decompose_on_confirm: true` — once the elohim-holochain green streak confirms
disappearance, this concern carries no museum-worthy lesson beyond the already
-recorded host-green ≠ CI-green / cross-workspace trap, so it decomposes
cleanly.

## Fix trail

- Commit `000e144f7` — `fix(dna): supply substrate_signal in the content_store
  coordinator's EconomicEvent initializer`. One file, +4 lines (3 comment + 1
  field) at `content_store/src/lib.rs` (now line ~12201).
- Local verification: `RUSTFLAGS='--cfg getrandom_backend="custom"'
  CARGO_TARGET_DIR=/tmp/cs-check-target cargo check --target
  wasm32-unknown-unknown -p content_store` → `Finished dev profile` (exit 0,
  warnings only; the E0063 is gone). /tmp target dir per the volume-fingerprint
  container quirk; WASM workspace keeps the custom getrandom flag and plain
  cargo.
- Commit-only (integrator pushes; a `[build:dna]`/`[build:edge]`-tagged
  integrator push rebuilds the DNA and confirms by green streak ≥3).
