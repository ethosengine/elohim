---
id: "backlog-ci-projected-head-convergence-race"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-app deploy projected-head probe gates on the served serverBlobHash at a single instant — it reads the doorway BEFORE its 300s reconcile tick swaps the head, so any build that changes the SSR bundle is guaranteed a MISMATCH on whichever host/slug loses the convergence race"
slug: "ci-projected-head-convergence-race"
written: "2026-07-23"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [15133508b92b, 5fed1dca2f69, 7569d2b6e0c6, c7b21e0d88e4]
jobs: [elohim/dev]
relatedNodeIds: []
tags: [ci, elohim-app, deploy, projected-head, served-vs-declared, ssr-bundle-head, doorway-reconcile-tick, eventual-consistency, convergence-race, probe-reads-converging-value, T4-2, T4-1, retry-ladder, in-flight-fix]
cites:
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1628/
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1629/
  - scripts/ci/verify-projected-head.sh
  - doorway/doorway-service/src/render/registry.rs
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
---

# elohim-app deploy `projected-head` probe — served-vs-declared MISMATCH is a convergence-window race, not a broken deploy

## The failure

Four fingerprints, one concern — the post-deploy `projected-head` verify stage
(`scripts/ci/verify-projected-head.sh`, Track-4 T4-2) reports a served-vs-declared
head MISMATCH on the SSR (`server`) bundle for a host/slug pair:

| fp | build | host / slug | verdict in log |
|----|-------|-------------|----------------|
| `15133508b92b` | 1628 | alpha.elohim.host / lamad-spa | MISMATCH |
| `5fed1dca2f69` | 1628 | elohim.host / elohim-host-landing | MISMATCH |
| `7569d2b6e0c6` | 1629 | alpha.elohim.host / elohim-host-landing | MISMATCH |
| `c7b21e0d88e4` | 1629 | elohim.host / lamad-spa | MISMATCH |

Quoted signature (build 1628):

```
ERROR: projected head MISMATCH on elohim.host elohim-host-landing: declared=sha256-d379…a59d served=sha256-47c2…e250
ERROR: projected head MISMATCH on alpha.elohim.host lamad-spa:     declared=sha256-d4ce…7fed served=sha256-fa68…f30f
```

Occurrence evidence: each fingerprint `seen: 1`; two first appeared at build
1628, two at 1629 (each fp `first_build == last_build`). Both builds finished
`UNSTABLE`.

**Which host/slug fails rotates per build** — in 1628 the two *passing* pairs were
`alpha/elohim-host-landing` and `elohim.host/lamad-spa`; in 1629 those same two
pairs FAILED while the previously-failing pair passed. Four distinct fingerprints
across two builds precisely because the loser of the race is drawn fresh each run.

## Verdict — real probe defect (deterministic race), not infra flake, not a broken deployment

The smoking gun is in the served hashes across the two builds:

- Build 1629 `alpha.elohim.host elohim-host-landing`: `declared=df9e…0494  served=d379…a59d`
- Build 1628 `…/elohim-host-landing` had `declared=d379…a59d` — i.e. **1629 is served exactly 1628's declared head.**
- Same for lamad-spa: 1629 serves `d4ce…7fed`, which is 1628's declared lamad-spa head.

So the doorway is faithfully serving the *previous* build's head because its
reconcile loop has not ticked since this build's head-PATCH. The bytes uploaded,
the head was authored — the deployment is healthy and converges within one
reconcile interval. The probe simply reads the converging value at the wrong
instant and gates on it as if it were synchronous.

This is not random infra jitter (retrigger would not reliably clear it — a build
that changed the SSR bundle is *guaranteed* to observe the pre-swap head on any
host whose reconcile tick hasn't fired yet), so it is a REAL defect in the probe's
timing model, not a flake to wait out.

## Root cause

1. **Probe (`scripts/ci/verify-projected-head.sh`, ~line 118 / MISMATCH exit at ~line 157).**
   The retry ladder (4×20s) fires **only on unreachability** — the header comment
   is explicit: "Once ANY health surface answers 200, we stop retrying: whether
   servedBundleHeads is present or absent is a fact about deployment state, not
   something a retry changes." That premise is wrong for the MISMATCH case: a
   *present-but-stale* served head is precisely a value a retry changes, because
   the doorway converges it on its own tick. On MISMATCH the script `exit 1`s
   immediately with no convergence-window retry.

2. **Doorway reconcile cadence (T4-1).** `servedBundleHeads` is swapped only on the
   doorway's ~300s reconcile tick (bundle-head reconcile, commit `0c785b5ef`), not
   synchronously on the head-PATCH the App pipeline issues. The probe runs seconds
   after the PATCH — inside that window the served head is still last build's.

3. **Complementary silent-pass gap (`doorway/doorway-service/src/render/registry.rs`, ~line 382).**
   A slug whose boot-time `serverBlobHash` resolve fails takes the `return
   Self::empty()` path → it gets **no `BundleHead` entry**, is excluded from every
   reconcile pass, and never appears in `servedBundleHeads`. The probe reads that
   *absence* as a skip/pass (`⚠ attestation not deployed … skipping`, `exit 0`) —
   a false green. This is the inverse failure mode of the four fingerprints above
   (it produces no failing fingerprint, by nature), but it is the same concern: the
   served-vs-declared probe does not yet faithfully gate SSR head propagation.

## Museum relation

Nearest relative in the anti-patterns museum
(`…/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`) is the
**post-dispatch observational-stage fail-regime boundary**: stages *after* the
world is already what it is must not hard-FAIL on transient state. The
`projected-head` probe is post-deploy and reads an eventually-consistent value; a
MISMATCH inside the convergence window is exactly the transient-state hard-fail
that boundary warns against.

This is arguably a **new recurring-trap candidate** — *"a post-deploy probe that
samples an eventually-consistent value at a single instant and gates on it as if
synchronous."* It has occurred once (2 builds, same session), so it does NOT yet
graduate into the museum. If it recurs after the in-flight fix, the lesson should
graduate INTO the museum record (extend it; do not fork a second lessons doc).
Left un-stamped for `decompose_on_confirm` deliberately so the harvester reports
this entry for graduate-then-decompose rather than silently deleting it.

## Current decision

Bounded fix in flight **this session** on `shift/reach-vocab-slice2` (owned by the
concurrent session; this triage does NOT implement a competing fix):

- A **convergence-window retry ladder on MISMATCH** in `verify-projected-head.sh`
  — retry the served-head read (not just the reachability probe) across the
  doorway's reconcile interval before declaring MISMATCH.
- The `registry.rs:382` **BundleHead-on-resolve-failure** fix so a slug that fails
  its boot `serverBlobHash` resolve is not silently omitted from `servedBundleHeads`
  (closes the false-green absence-reads-as-pass gap).

Ledger stamped `triaged` with `triaged_at_build` = each entry's `last_build`
(1628 / 1629): the fix targets everything ≤ those builds. The harvester's
disappearance guard is the safety net — if a build > `triaged_at_build` still
carries the fingerprint (fix didn't land or didn't take), it reopens automatically;
a green streak ≥3 confirms the fix and routes this entry to graduate-then-decompose.

## Fix trail

- In flight (not yet committed at triage time; working tree clean at both fix sites,
  script still lacks retry-on-mismatch): retry-on-MISMATCH ladder in
  `scripts/ci/verify-projected-head.sh`; BundleHead-on-resolve-failure in
  `doorway/doorway-service/src/render/registry.rs`.
- Related prior commits: `5fec4f025` (T4-2 probe), `0c785b5ef` / `f476ea040`
  (T4-1 bundle-head reconcile).
- Local verification will be the concurrent session's; CI confirmation is the
  green streak on `elohim/dev` builds > 1629.
