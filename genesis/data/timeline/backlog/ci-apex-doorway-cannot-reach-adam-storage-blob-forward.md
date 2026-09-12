---
id: "backlog-ci-apex-doorway-cannot-reach-adam-storage-blob-forward"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The apex doorway cannot reach adam's alpha storage at all, so every SPA blob forward reds the App deploy — an honest forwarded_to_storage:false whose CAUSE the doorway declined to print, costing a triage dispatch to learn it was a connection failure and not a shed"
slug: "ci-apex-doorway-cannot-reach-adam-storage-blob-forward"
written: "2026-09-12"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: blocked
fingerprints: [5ee767181c07, 82831e35dcd7]
jobs: [elohim]
relatedNodeIds: []
tags: [ci, elohim-app, deploy, upload-spa-blob, apex, elohim-host, adam-alpha, storage-unreachable, transport-fault, error-chain-truncated, operator-owned, fingerprint-collision]
cites:
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1703/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1704/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1705/
  - scripts/ci/stage-spa-blob.sh
  - doorway/doorway-service/src/routes/seed.rs
  - genesis/manifests/cluster-state.yaml
  - genesis/data/timeline/backlog/ci-deploy-reads-storage-backpressure-as-failure.md
  - genesis/data/timeline/backlog/blob-put-large-body-shard-verify-drop.md
  - genesis/data/timeline/backlog/conductor-websocket-flap-breaks-deploy-write-path.md
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# The apex doorway cannot reach adam storage — an honest red with an unprintable cause

## The failure

`elohim/dev` **#1704** (FAILURE) and **#1705** (FAILURE), both in **Upload SPA Blob**.
Ledger: `5ee767181c07` (lamad-spa) and `82831e35dcd7` (elohim-host-landing), each
seen 1, builds 1704..1705.

```
{"success":true,"hash":"sha256-f37701d3fb85fdbf671bbd22f40d57f1d460267bab006be907707ac3b6d5edc8",
 "already_cached":false,"forwarded_to_storage":false,"size":11980941,
 "error":"could not reach storage to forward the blob: error sending request for url
  (http://elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090/blob/sha256-f37701d3fb85…)"}
  ✗ [elohim-host-landing] doorway cached the blob but storage forwarding FAILED
    (forwarded_to_storage:false) — refusing to call this staged
  ⚠ [elohim-host-landing] attempt 1/3 against https://elohim.host failed — retrying in 5s
… ERROR: [elohim-host-landing] stage failed after 3 attempt(s) against https://elohim.host
  — host left STALE
```

Occurrence evidence, from a backward walk of #1697–#1705:

| build | result | this signature |
|---|---|---|
| #1697 | SUCCESS | — (last full green) |
| #1698–#1701 | FAILURE | no — a different class (`SSR declaration failed … reason=different-serverBlobHash`, projected-head divergence) |
| #1702 | ABORTED | n/a — reads as 0 failures (museum trap #1) |
| #1703 | UNSTABLE | **no** — every forward on that build is `forwarded_to_storage:true` |
| **#1704** | FAILURE | **first occurrence** |
| #1705 | FAILURE | recurred, identical text, identical target, different content hash |

Scope: the failing host is **`https://elohim.host` (apex)**, not alpha. The SAME build's
earlier call for the same slug against `https://alpha.elohim.host` succeeded
(`forwarded_to_storage:true`). Both slugs fail identically against apex in both builds.

#1705 carries **no new file changes** — it was dispatched by the same empty re-run commit
`c932c26` as #1704. The re-run reproduced the failure exactly, which retires the
"transient fleet turbulence" reading that commit's own message offered.

## Verdict

**Infra — and the red is HONEST.** `forwarded_to_storage:false` is correct here: the apex
doorway's write-through cache holds the bytes and adam's alpha storage, the authoritative
store, never received them. The stage's refusal to stamp `staged` is the 2026-08-22
lesson working as designed (`blob-put-large-body-shard-verify-drop` — a declared head
against bytes nobody can materialize).

Three sibling concerns were checked and ruled out:

- **Not the shed/backpressure class** (`ci-deploy-reads-storage-backpressure-as-failure`,
  which carries the SAME fingerprint `5ee767181c07` — see the collision note below).
  That concern is a 503/429 shed with a `Retry-After`; the doorway now re-offers three
  times on that shape. This is `error sending request for url` — a **connection-level**
  failure with no HTTP status at all, so no shed classifier could ever see it and the
  re-offer ladder is not reached.
- **Not the shard-verify class.** 11,980,941 bytes is inline (`MAX_INLINE_SIZE` 16 MB),
  not sharded, and the failure is pre-transfer, not a mid-transfer drop.
- **Not the conductor-websocket flap** (`conductor-websocket-flap-breaks-deploy-write-path`).
  That class 503s the PATCH/declare leg on ALL hosts with the doorway↔conductor app
  interface flapping. Here the alpha leg of the same build is fully green and only the
  apex→adam-alpha storage hop fails.

The changeset is not the cause: #1704's three commits touch `habits.yaml`, a
`.habit.md`, a plan doc, and one `sdk/domains/shefa/CLAUDE.md` cite migration. Nothing
in `scripts/ci/`, the seed route, or storage blob routes.

## Root cause

**Substrate, operator-owned: the apex doorway has no working network path to
`elohim-adam-alpha.elohim-alpha.svc.cluster.local:8090`.** The repo is not the cure
surface; per gospel, the live cluster is the operator's and `genesis/manifests/` is ours.

**But the finding could not be routed from CI, and that half IS ours.** The doorway
printed the wrapper and swallowed the cause. `reqwest::Error`'s `Display` renders
`error sending request for url (…)` and deliberately stops there — the discriminating
fact (DNS failure vs connection-refused vs timeout vs TLS) lives in
`std::error::Error::source()`, which nothing walked. The 2026-09-02 cure in this exact
file made every refusal name the **leg** that refused, precisely so a
`forwarded_to_storage:false` would stop sending a human into the doorway's pod logs. It
worked — the `error:` field is present and is how this triage identified the class in
one read. It stopped one layer short of the **cause**, which is the field that decides
*who owns the finding*: a shed is ours, an unreachable service is the operator's.

## Current decision

**Blocked on the operator; the diagnostic half is fixed and landed.**

The unblocking move is an operator one: restore (or re-route) the apex doorway's path to
adam's alpha storage — the manifest surface is `genesis/manifests/`, and
`genesis/manifests/cluster-state.yaml` is the durable declaration if this turns out to be
a capability loss rather than a transient. Never `kubectl`.

Ledger `5ee767181c07` and `82831e35dcd7` → `status: blocked`. Deliberately **not**
`triaged`: the landed fix improves the next occurrence's legibility, it does not make the
fingerprint disappear, and stamping `triaged_at_build` would set the sweep up to read a
still-broken path as a fix that didn't take.

**Fingerprint collision, recorded on purpose.** `5ee767181c07` is the generic line
`✗ [lamad-spa] doorway cached the blob but storage forwarding FAILED …` and is ALSO
carried by `ci-deploy-reads-storage-backpressure-as-failure` (a shed, 2026-09-02) and
emitted by `blob-put-large-body-shard-verify-drop`'s mitigation (a shard drop,
2026-08-22). Three root causes, one string. **The discriminator is now mechanical: read
the `error:` field of the JSON line immediately above it** — `storage shed the …` is the
backpressure concern, `could not reach storage …` is this one, a read-back failure is the
2026-08-16 class. That field exists because of the 2026-09-02 fix; before it, telling
these three apart required the doorway's logs. This is the over-coarse polarity of
`ci-harvest-fingerprint-granularity-banner-collision`, resolved by a payload the harvester
does not read but a triage agent does.

## Fix trail

`doorway/doorway-service/src/routes/seed.rs` — a transport refusal names its CAUSE, not
only its wrapper:

- New `transport_reason(prefix, err)` walks `std::error::Error::source()` and joins the
  chain with `: `. Deduplicates consecutive identical links (hyper/reqwest repeat the
  wrapper text often enough that an unfiltered walk reads as stutter) and stops at
  `TRANSPORT_CAUSE_CHAIN_MAX = 4` links, so a pathological nesting cannot inflate a JSON
  body CI reads in full.
- Applied at both transport sites in `forward_once`: the PUT send failure (the one
  #1704/#1705 hit) and the read-back send failure.
- Tests: a three-link chain must surface both the wrapper the deploy leg already
  recognises AND the root cause (`dns error: …`); a stuttering chain renders once; a
  ten-link chain stops at the declared bound.

Local verification: `cargo test --lib` for doorway-service — **1278 passed, 0 failed**;
`cargo clippy --lib --tests -- -D warnings` EXIT=0; `cargo fmt --check` EXIT=0. (All run
with `RUSTFLAGS=""` and an explicit `CARGO_TARGET_DIR` pool slot.)

## Done when

- [x] The next occurrence names its own cause in the CI log (DNS vs refused vs timeout)
- [x] Concern separated from the two siblings that share its fingerprint, with a
      mechanical discriminator recorded
- [ ] Apex doorway reaches adam-alpha storage (operator move; repo manifests are the
      cleanup surface)
- [ ] `5ee767181c07` / `82831e35dcd7` absent from an `elohim/dev` green streak ≥3
