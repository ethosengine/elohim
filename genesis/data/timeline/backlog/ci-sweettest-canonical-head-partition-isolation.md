---
id: "backlog-ci-sweettest-canonical-head-partition-isolation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Sweettest earned-vs-staging canonical-head partition test failed in CI — non-isolated conductors defeat the pre-exchange partition"
slug: "ci-sweettest-canonical-head-partition-isolation"
written: "2026-07-11"
author: "ci-failure-triage"
status: "wip"
priority: "medium"
ci_status: "in-progress"
fingerprints: [c1f402a74fd1, b3696104006e, 56ec1c027ba6]
jobs: [elohim-holochain]
relatedNodeIds: []
tags: [ci, elohim-holochain, dna, sweettest, notary, canonical-head, kitsune2, partition-test]
cites:
  - https://jenkins.ethosengine.com/job/elohim-holochain/job/dev/1354/
  - elohim/holochain/tests/sweettest/src/tests/lamad.rs
  - elohim/holochain/tests/sweettest/src/common/conductors.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
---

# Sweettest earned-vs-staging canonical-head partition test — conductor isolation

## The failure

DNA pipeline `elohim-holochain/dev` **#1354** (commit `0c1a698ff`, 2026-07-10
22:31 UTC) failed the sweettest stage on one test:

```
TRY 1 FAIL  elohim_sweettest::lamad earned_beats_newer_staging_at_resolve
  Guest("Content with id 'tier-x' already exists. Use update_content to modify existing entries.")   [content_store/src/lib.rs:2361]
TRY 2 FAIL  elohim_sweettest::lamad earned_beats_newer_staging_at_resolve
  Guest("declare_canonical_content_head: earned head is protected for id 'tier-x' — the staging scaffold cannot override an earned canonical")   [content_store/src/lib.rs:3294]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 7 filtered out
```

Three ledger fingerprints, **one concern**:
- `c1f402a74fd1` — `test earned_beats_newer_staging_at_resolve ... FAILED` (the test)
- `b3696104006e` — `test result: FAILED. 0 passed; 1 failed; ...` (its nextest aggregate)
- `56ec1c027ba6` — `DNA BUILD FAILED` (the pipeline post-action banner, echoed because the
  sweettest stage failed — not an independent build-toolchain fault)

Occurrence evidence: `seen: 1`, `first_build == last_build == 1354`. Single build.

## Verdict — real (test-authoring bug), NOT a flake, NOT infra

The two nextest attempts fail with *different* zome guards (duplicate-id at create,
then earned-head protection at declare), which reads flaky but is deterministic: the
test's intended **pre-exchange partition never existed**. Both peers were already
gossiping before the author phase, so which guard fires depends only on which peer's
create/declare the other had already absorbed.

## Root cause

`earned_beats_newer_staging_at_resolve` exercises tier-aware canonical-head
resolution: agent a1 declares an EARNED head for `tier-x`, then (strictly later,
while "partitioned") a2 declares a NEWER STAGING head; after healing, both peers must
resolve the EARNED head despite the staging link being newer. The test built its two
conductors with `SweetConductorConfig::standard()`, which leaves `mem_bootstrap: true,
disable_bootstrap: false`. Kitsune2's in-memory bootstrap store is a **process-global
`HashMap<(test_id, space_id), _>`** keyed by the *thread id at conductor-construction
time* × the DNA-hash space. Under `#[tokio::test(flavor = "multi_thread")]` the two
same-space conductors frequently land on one worker thread and share that store, so
the second conductor's startup bootstrap poll discovers the first's agent info and
begins gossiping **before** the test authored anything. The "partitioned" creates then
collide on the shared `tier-x` id (TRY1) or the staging declare is refused because the
earned head has already propagated (TRY2).

This is the sweettest cross-agent-consistency trap (memory anchor
`feedback_sweettest_cross_agent_consistency`): `standard()` conductors do not give a
genuine partition. Not a museum-tier CI trap (first occurrence; the durable mechanism
already lives in the `two_agent_conductors_isolated` helper docstring in
`conductors.rs` and the test docstring in `lamad.rs`).

## Current decision — triaged; fix already landed, awaiting disappearance-confirmation

Fresh dev-integration regression, self-cured **~2h after the failing build** by the
same author. `0c1a698ff` (the substrate-cure sprint merge that introduced the notary
head-election test) landed at 22:29 UTC and built red as #1354; the cure
`2036473d7` landed 2026-07-11 00:40 UTC and is an ancestor of current HEAD. No further
code action required from triage. Sweep confirms by job green-streak (≥3, no
recurrence); ledger stamped `decompose_on_confirm: true` because the lesson is already
durably captured in the two code docstrings + the memory anchor, so the backlog carries
no incremental museum-worthy lesson.

## Fix trail

- **Regressor:** `0c1a698ff` merge(dev) — introduced `earned_beats_newer_staging_at_resolve`
  built on `standard()` conductors.
- **Cure:** `2036473d7` fix(sweettest): genuine conductor isolation for the
  earned-vs-staging partition test — switches the test to
  `two_agent_conductors_isolated()` (adds `single_agent_conductor_isolated()` +
  `two_agent_conductors_isolated()` in `conductors.rs`, setting
  `disable_bootstrap = true`; gossip/publish stay enabled so the post-`exchange_peer_info`
  `await_consistency` still converges).
- **CI evidence of the cure:** `elohim-holochain/dev` **#1355** (first build after the
  fix) = SUCCESS. Green-streak confirmation (≥3) is the harvester's to finalize.
</content>
</invoke>
