---
id: "backlog-gate-merge-gradient-reading-2026-10-25"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Next sprint: integration is composition, not a re-gate — what eight landing pushes between two workspaces told us"
slug: "gate-merge-gradient-reading-2026-10-25"
written: "2026-10-25"
author: "claude-opus-5-5 (shift land-gate-evidence-reuse), with the operator"
status: "backlog"
priority: "high"
area: "scm / ci-cd / gates / cooperative devspaces"
tags: [scm, ci, gates, trust-gradient, brit, integrate, devspace, algedonic, performance]
cites:
  - genesis/orchestrator/.epr-meta/push-gate-pays-once.habit.md
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
  - genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md
---

# Integration is composition, not a re-gate

## What happened (2026-10-25)

Landing `feat/gate-evidence-reuse` took eight gated pushes over about nine hours. The branch touched
shared surfaces (the root `justfile`, `pool-policy.json`, a `seam-contracts` comment), so every push
selected 25 local gates. Push 7 cleared all 25 in 4407 s and was still rejected at the ref lock: a
docs-only letter series landed every few minutes while a 73-minute pre-push held an advisory
`dev-integration` lease that the other pusher never saw. Push 8 then went red on a flake in dev's own
renew-loop code (`prepush_releases_on_gate_failure` saw each lease released twice).

The branch landed with the pre-push bypassed, on push 7's verdict. The operator ruled it deliberately:
"by pushing and breaking CI/CD we're being intentional about the pain here."

## What the break is FOR

The bypass is not a shortcut around a working process. It refuses to keep paying a cost the
architecture should not charge at all.

Every gated retry made the same point in a new costume. Verification is serialized in front of one
mutable ref, on one device, re-deriving verdicts that already existed, while a second workspace could
not see any of it. Absorbing that again on a ninth push would hide the signal.

Landing by bypass puts the pain where the system can sense it: a dev push that no local gate
witnessed, which CI then judges on its own. It makes the case for the missing node below: verdicts
carried by closure, integration as composition, and cooperation over a shared live channel. That is
the architecture cooperative devspaces need in order to land work without racing each other. If CI
goes red on this push, that red is information about what the local re-gate was or was not
protecting, and it is read as such, not hidden.

## What was measured

All of this came from the `gate-cycle-seconds@1` rows, preserved at
`genesis/local-dev/gate-reuse/gate-cycle-2026-10-25.jsonl`.

**Closure-keyed reuse collapses the big gates:**

| Gate | Seconds when it ran | Seconds when reused |
|---|---|---|
| elohim-storage | 5757 / 1938 | 56 |
| steward-node | 1222 / 221 | 48 |
| doorway | 686 / 125 | 25 |

**The push total fell** from 6128 s (no reuse) to 4407 s (three reuses). What remains is mostly the
gates keyed on the whole tree: eprfs at about 1700 s and memory-ceremony at about 1200 s. Both re-pay
on every rebase, even a docs-only one.

**A cold worktree build cache costs 4 to 15 times more on every small gate:**

| Gate | Cold | Warm |
|---|---|---|
| cargo-bridge | 193 s | about 40 s |
| elohim-sdk | 181 s | about 25 s |

**All four local failures of the day were environment, not code:** an exported toolchain leaked into a
WASM gate; the pinned toolchain lacked the wasm32 target; a fresh worktree had no built
storage-client dist; and a test that dev's own session-envelope commit broke.

**CI's verdict on `b2130d38b`, read job by job (lane 0, 2026-10-25 21:45Z).** Orchestrator #2150
carries it; #2149 is the baseline. eprfs (#67 vs #66) failed in both, holochain passed in both, and
edge and genesis were UNSTABLE in both. Only `elohim/dev` #1777 differs: UNSTABLE on its
`release.adopt adam` leg. That is a peer-adoption timeout ("delivered, not yet proven"), red since
#1757, which predates the landing, and the app wasn't dispatched in #2149 at all. So the difference
is environment and dispatch, not code. `131130a4b` (lanes A and D) has no orchestrator build yet;
#2151 is running at `09f68a04e`.

## The gradient reading

The paths are read with `/gradient-reading`; tag definitions are in `CONVENTIONS.md`.

1. **Re-verifying our own unchanged green**
   - Tags: `trustful-self`, `friction-verify`, per push × per gate.
   - Compressible, and compressed now by closure-keyed evidence (habit `push-gate-pays-once`).
2. **Gates keyed on the whole tree re-pay on docs-only rebases**
   - Tags: `fused-planes`.
   - eprfs's code gate is welded to corpus-integrity checks: its cites tests walk `genesis/docs/**`
     and its live-manifest tests walk the repository.
   - The cure is separation, not trust: corpus conformance becomes its own gate, keyed on the corpus.
3. **The ref race**
   - Tags: `plane-head` fused with verification. The lock contention itself is `friction-mechanical`.
   - Its cause is structural: head advancement waits behind a client-side verification window.
   - The lease was a real declaration on a trust-blind floor.
4. **Two sessions built the same reuse machinery** (gate receipts vs brit evidence)
   - Tags: `trustful-declared`, `friction-blind`.
   - Neither saw the other's in-flight intent until it landed. The result was three conflict merges and
     a new flake at the seam.
5. **CI re-runs every pipeline on what the author already verified**
   - Tags: `friction-verify` paid twice.
   - CI's value is independence (`custody-diverse`), not repetition. Meanwhile 16 of the last 18 dev
     reds were dataplane or environment.
6. **Environment failures recorded as code failures**
   - Tags: `friction-mechanical`, owed unconditionally.
   - The device should declare its toolchain, targets and built artifacts at arrival, and each cycle
     record should carry a failure class.

## The missing node

```
chain: closure attested green → dev head advanced
between A (closure attested green by an admitted signer) → C (dev head advanced)
missing node B: integrate composes attested closures and advances the head by one ref
  compare-and-swap, without re-verification; probe: an integrator landing of an attested branch over
  a docs-only delta completes in under 60 s
current state: absent; heads advance only through a client-side pre-push that re-gates everything
```

A data point from the device-runtime session (2026-10-25) that supports this reading. Pre-push
cannot land a device's epr head from a worktree. At hook time the pushed SHA is on neither
`refs/heads/dev` (a linked worktree's `dev` is the main checkout's) nor `origin/dev` (the push has not
completed), and the bin store correctly refuses to land an unpublished commit. The landing happens
after publication: the next session arrival's `follow` adopts the gate's ingested candidate for the
published HEAD with no rebuild (`Store::adopt`; first observed for 16af7710f, landed 13:23:59Z). Work
that must follow publication belongs with the integrator, not the pre-push.

## Next sprint, in order

1. **F4, native `brit integrate`**
   - Source: `/projects/.claude-config/plans/brit-is-greenfield-we-elegant-journal.md`.
   - Shape: one ref compare-and-swap; evidence from F1; gate misses run in a restored draft.
   - Tonight is its evidence: push 7's verdicts plus a docs delta should have landed in seconds.
2. **Split the corpus-conformance tests out of eprfs and memory-ceremony** into a gate keyed on the
   corpus, so those code gates can key on named reads and survive docs rebases.
3. **Carry integration leases, in-flight closures and intents on the governed peer channel**
   (sibling: `native-peer-dev-channel-and-collective-memory-seed`), so a second workspace sees "X is
   being gated" and "Y is building reuse" before it builds or pushes.
4. **Admit signers beyond self** once brit-build-ref verifies signatures: roster devices, then the
   integrator, then CI. CI then admits authored verdicts and spends itself on unattested and
   environment-bound work, plus diverse spot-witnessing.
5. **Small fixes to the measurement substrate**
   - gate-cost.mjs orders rows by ledger append order, not `occurredAt`, which is the commit author
     clock and survives rebases.
   - Cycle rows carry a failure class: environment or code.
   - On a reuse miss, the runner names the closure component that moved.
   - dev's renew-loop double-release flake is fixed.
