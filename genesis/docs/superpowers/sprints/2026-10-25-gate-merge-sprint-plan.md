---
title: "Sprint plan — two peers land on dev without racing or re-gating"
id: gate-merge-sprint-plan-20261025
date: 2026-10-25
status: active
author: "claude-opus-5-5 (Che workspace shem, device z6Mkm5de…, peer 4370a806…), with the operator (steward of record)"
habits: [devices-share-one-berth, push-gate-pays-once]
cites:
  - genesis/data/timeline/backlog/gate-merge-gradient-reading-2026-10-25.md
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
  - genesis/agentic/.epr-meta/devices-share-one-berth.habit.md
  - genesis/orchestrator/.epr-meta/push-gate-pays-once.habit.md
---

# Sprint plan — two peers land on dev without racing or re-gating

This sprint carries out backlog item `gate-merge-gradient-reading-2026-10-25`. The logjam it
answers: landing one branch took eight gated pushes. Push 7 cleared all 25 gates and still lost the
ref race, because docs commits from another workspace landed past an `dev-integration` lease that
workspace never saw.

Operator rulings, 2026-10-25 (covenant session 46e7b111, clauses `bafyreibk4rc2…` steward and
`bafyreid2jkbf…` purpose):

- Coordination between peers leads, then integration as composition.
- F4 lands first in `epr integrate`, which already advances the ref by compare-and-swap. Linking it
  into brit (the 10-14 plan's decision 1) follows and changes no visible behaviour.
- This session's write-set covers lanes A and D. Lanes B and C are planned here and run under a
  later grant.

## What the grounding found

These were four read-only Codex legs against `b2130d38b`, spot-checked by hand. File:line
references are against that commit.

- **The lease is already shared, but it isn't visible.** `dev-integration` is scoped to the steward
  (`genesis/agentic/pool-policy.json:116`). Berth writes it to the device's channel store, and every
  claim reads `epr peer leases --json` (`genesis/agentic/bin/berth:329,1148`). `epr peer serve`
  carries signed channel heads over iroh, and `epr flow fold --from <device> --root <cid>` receives
  them. The peer missed the lease for four reasons:
  - only one peer endpoint is declared, and it is served on demand;
  - a push with no resolvable session goes ahead `UNLEASED` (`.husky/pre-push.bash:118,257`);
  - a `berth say` intent written from a worktree can miss the store that is served;
  - the remote lease view drops task, closure and expiry
    (`elohim/eprfs/eprfs-peer/src/lease_view.rs:102,276`).
- **Live reading, 18:35 UTC.** Run on this device, `berth status` shows
  `dev-integration held by 2e8f68b9 … for 72m (verify, ttl 30m) — pre-push to dev`, so sessions on one
  device do see each other's lease. Under "other devices" it prints `none — no other device is
  admitted on this roster`. Two consequences:
  - The docs pushes that raced push 7 were made on this device. They could only have passed a held
    lease by going `UNLEASED` or bypassing the hook, so **A2 cures the race itself**.
  - The other workspace, ethosengine, can see nothing until its device is admitted on this roster.
    ethosengine holds the chain-root key (`z6MkpZRh…`, peer `95e41161…`), so its own PeerEndpoint
    row admits it (`epr actor device endpoint`). Only ethosengine can sign that row, so **A4 is
    ethosengine running that step**, with the operator's confirmation in that session.
- **The race recurred live, 19:00Z.** While `2e8f68b9` held `dev-integration` (renewed every
  10 min since 17:19Z), `f74cccbb9` and `8c97287ff` from session `01C86Tki…` (device-runtime)
  landed on dev. The ledger shows no claim by any other holder in that window. Those commits went
  in from an unadmitted device or unleased, so the waiting push now has to rebase. A2 closes the
  same-device path; A4 (roster admission) closes the cross-device path.
- **A third time, 20:11Z, with the lease taken on the other device.** While `2e8f68b9` held
  `dev-integration` for push 7, device-runtime's `09f68a04e` landed under a lease that only its own
  device could see. Its device is not admitted here, so to this roster the lease was invisible
  rather than missing. The waiting push restarted for the second time (push 8). A2 can't cure
  this; only A4 can.
- **A live push ran unleased for 37 min, 20:32–21:09Z.** Push 7 was killed (TaskStop) without a
  release. Push 8's claim came back `renewed`, and the hook treated a renew as a lease it hadn't
  taken, so it started no renewer. Fixed in `131130a4b` (BERTH_KEPT). Two berth questions stay
  open:
  - That `renewed` wrote no ledger row. `dseq` 157 (20:24:34 renew) runs straight into 158
    (21:09:41 claim), and no other berth directory exists. Every renew path in `berth` records a
    row under its lock, so the reply came from a path that renews nothing.
  - The 21:09:41 claim records `expired_after_s 2238`, so the lease expired at 20:32:23. That is
    within a minute of push 7's kill, not at its TTL: the 20:24:34 renew set 1800 s, which runs to
    20:54:34. The lease's liveness seems to follow the killed hook's process. A holder still alive in
    the same session then sees its own lease as dead.
- **F4 is closer than the plan assumed.** `epr integrate` already advances the ref atomically with
  `update-ref` and an expected-old value (`elohim/eprfs/epr-cli/src/integrate.rs:295`). What blocks it:
  - preconditions for a clean checkout and HEAD at the source (`:146`);
  - it runs the pinned gate instead of asking for evidence;
  - the evidence lookup (`gate-evidence.mjs:248`, `brit-build-ref build find`) is already at brit
    pin `76ba95d8a`, but it does not verify signatures.
  The plan cites `rakia-brit/src/baselines.rs` for `MustExistAndMatch`, and that citation is wrong
  (it uses `PreviousValue::Any`).
- **The corpus split is on F4's critical path.** When a read can't be resolved, the whole commit
  tree goes into the eprfs closure key (`gate-evidence.mjs:113,329`). So a docs-only delta misses
  eprfs, and integrate would refuse it. The "lands in under 60 s" probe needs C1.
- **The double-release flake.** The renew loop inherits the cleanup trap until it resets it
  (`.husky/pre-push.bash:174`). If the gate fails at once, TERM arrives before that reset, so the
  child runs the parent's cleanup and the parent then releases its own copy.
- **CI's reading.** Orchestrator #2144–#2148 were all FAILURE or UNSTABLE before `b2130d38b` was
  queued. A red on that push says nothing on its own; it has to be read job by job against #2149.

## Lanes

### 0. Read CI's verdict on `b2130d38b`

Compare each job against #2149. Record which jobs differ and whether each difference is
environment or code. Append the result to the backlog item's "What was measured" section.

### A. Peer coordination (habit `devices-share-one-berth`, red)

| Slice | Change | Probe |
|---|---|---|
| A1 | `hook_on_exit` releases only in the shell that owns the lease (`BASHPID` guard) | `prepush_releases_on_gate_failure`, plus a case where the renewer is terminated before its trap reset, asserting one release per lease |
| A2 | A dev/main push with no resolvable session refuses, naming the cure, instead of going `UNLEASED`; an intent written from a worktree goes to the served channel store | `prepush-berth.test.mjs` refusal case; `berth_test.py::worktree_intent_reaches_peer_before_first_claim` |
| A3 | The lease record carries closure fingerprint, task and expiry; `lease_view.rs` keeps them; `berth status` shows a remote holder's closure and expiry. Local holders get the same treatment: the line shows the last renew and the expiry, not only the time held. On 2026-10-25 a peer read `held … for 72m (ttl 30m)` as an expired lease; the ledger showed renews every 10 min | `peer_sees_closure_and_expiry_before_push` (berth + lease_view tests) |
| A4 | Both T8 halves run between the real workspaces. Shem's endpoint is row 3 and its daemon serves; ethosengine writes its own PeerEndpoint row (the 4th) and runs the holder half | `@concern:devices-share-one-berth and @requires:shem`, joined T8 receipt |

A3 changes `eprfs-peer` (Rust), which is outside this session's write-set. If the remote view
can't carry the fields without it, A3 stops at the berth half and names the Rust half for lane B's
grant. A4 needs the other workspace, so it is the operator's item to schedule.

### B. Integration as composition (planned; later grant)

1. B1, an evidence adapter that only inspects: it reuses `gateEvidence()` → `reusableHit()` and
   never runs a recipe. Probe: `inspection_never_runs_recipe`, and the newest-red, signer and
   selector refusals still hold.
2. B2, `epr integrate --dry-run` prints the compare-and-swap plan and each gate's evidence hit or
   miss. Probe: `dry_run_reports_cas_and_misses_without_mutation`.
3. B3, a ref compare-and-swap on cached evidence only: drop the checkout preconditions, refuse any
   miss, and keep git's checked-out-branch rule. Probe:
   `integrate_fast_forwards_target_ref_without_touching_index_head_or_worktree`, plus a
   target-race refusal.
4. Later, link epr-cli into brit-cli. It is a library call with no visible behaviour change.

### C. Corpus split (planned; later grant)

1. C1, a `corpus-conformance` gate (`genesis/corpus-conformance/build-manifest.json`) takes the
   corpus tests out of eprfs (about 15 tests across `epr-cli/tests/{live_manifest_integrity,
   flow_cites,flow_scope,flow_memory_affiliations,flow_memory_sponsorship}.rs`). It declares the
   corpus inventories and the evaluator code as its inputs. Tests that read both code and corpus
   (`flow_parity`, `flow_edge_parity`, `canon_lift_parity`) go to the corpus gate with the
   evaluator code declared. Probe: a docs-only mutation leaves the eprfs key unchanged; a corpus
   mutation moves only the corpus key.
   Live evidence, 2026-10-26: a docs-only push (6061e04e7, the restart-to-connected plan) carried
   one bare-path cite. Its pre-push selected no gate, because no project reads the sprint docs as
   a declared input. eprfs's `flow_cites live_corpus_migrate_reports_zero_pending` then failed every
   eprfs gate on dev, until `5ac2681ef` sealed the cite. C1 selects that test from the docs
   change itself. Until C1 lands, run `epr flow cites seal <doc>` before pushing any doc with
   `cites:`.
2. C2, memory-ceremony gets a real step and declared reads, and enrolls in reuse only after its
   attribution parity check is snapshotted or excluded.

### D. Measurement (this session)

| Slice | Change | Probe |
|---|---|---|
| D1 | `gate-cost.mjs` uses ledger append order, not `occurredAt` (author clock, `gate-cost.mjs:42`); the main-checkout and worktree ledgers have a declared precedence | fixtures with descending and repeated author dates, and two ledgers |
| D2 | Cycle rows carry `failure_class=environment\|code` from structured preflight and test outcomes | four classifier fixtures (leaked toolchain, missing wasm32 target, missing dist, assertion failure) |
| D3 | A reuse miss names the closure component that moved | change recipe, compiler, gitlink and reads one at a time; each is named and reuse stays refused |

D3 needs a brit verb that fetches the previous verdict for a project without filtering on the
exact key. Until brit has one, it prints "comparison unavailable".

### Next sprint: signers beyond self

Brit's `build find` verifies signatures; verdicts are signed with the device identity; roster
devices are admitted from contest-aware admission. All of this needs a brit landing plus a pin move,
and a pin move rolls the fleet, so it rides the next planned bump.

## Done means

- `devices-share-one-berth` moves toward green: A1–A3 have landed with their probes, and A4 is
  scheduled with the other workspace.
- Lane D's rows say why a cycle failed and why reuse missed.
- Each slice adds a DELTA line to its habit atom, and the register is re-projected.
