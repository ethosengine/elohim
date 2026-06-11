# Sprint Result — EPR Durability Cluster Validation (overnight 2026-06-11)

**Objective:** drive the genesis substrate gates (mesh.adjacency ×2,
propagation.custody-convergence) to 0 with formation past founder binding.
**Outcome: measure 3 → 1, floor held across THREE complete genesis runs**
(the first complete runs after a week of aborts). Target 0 not reached at
close — the lone failing gate (custody-convergence) is the
operator-diagnosed doorway-attestation lockout, whose DNA fix was BUILT
(#1325) and DEPLOYING at close (wave-6: edge #1063 → genesis #1127 carries
the verdict; watcher output in the session task dir). Edge #1062 had raced
the fire-and-forget DNA build and shipped the pre-fix happ — the
dispatch-ordering wart is journaled (ebbe201f7). **Done-to-the-ceiling**,
with the deploy verdict handed to morning review.

## Proven on the live cluster tonight

- Persistent bootstrap peering: 14/14 pods `connected:13`, zero redial
  failures — the weeks-old matthew↔jessica partition is gone (CI asserts
  mesh.adjacency BOTH directions green, #1123 + #1124).
- Byte plane: jessica served the build-unique probe blob 0s/1-attempt;
  filesystem replicas persisted (5→7).
- Discovery: every pod names its responders; matthew's 28-34 commitments
  discovered fleet-wide (was 0 since Phase 0).
- Reconcile controller: FIRST successful connect in alpha history
  (app-id mismatch fixed — it had literally never run).
- Identity seeding: founder created on his own conductor, 0 conflicts.
- Seed Database: seconds, not hours (stamp circuit breaker killed the
  hour-sink that aborted #1121/#1122).

## Ceiling menu (operator decisions; full diagnoses in the backlog docs)

1. ~~Conductor PVC schema~~ **RETRACTED by operator on-host evidence**
   (~20:00Z): conductor DBs healthy, gossip publishing 1035/1035, zero
   DhtOp errors. DO NOT touch PVCs. See the retraction in
   `dna-conductor-dht-gossip-gap.md` (superseded).
2. **THE real root (operator-diagnosed): doorway-operator identity
   mismatch** — the attestation guard checks the registration record's
   frozen operator_agent vs the doorway's CURRENT cell key (re-keyed by
   reinstalls); register/update both reject, total lockout. Fix landed
   this shift (re-registration + latest-wins). 
   `dna-health-attestation-ci-authz.md` (now central).
3. **infrastructure role** connect failures + read-pool saturation on
   ethosengine conductors (Util 125-162%) — tuning, not blocking.
4. elohim-epr Jenkins job not provisioned (orchestrator warns every run).
5. Dataplane SDK: 3 design questions → /brainstorm.
   `arch-dataplane-sdk-proposal.md`.

## Anti-patterns observed (museum candidates)

- **One-shot-init disease** — 3 instances cured in one night: bootstrap
  dials (retry only at connected==0), HcClient 5-then-die, and the
  reconcile-controller app-id mismatch masked beneath the retry noise.
- **Baseline-advance-despite-failure** under-build hole (3 occurrences;
  `[build:app,edge]` workaround;
  `ci-orchestrator-baseline-advance-despite-failure.md`).
- **Audit rows graded docs over code** — 4 stale audit rows caught at
  implementation; the code review then caught the implementation's own
  blind spot (N1 production no-op with a green test). Two verify layers,
  both earned their keep.
- K8s pod-exec websocket transient (canonicalized + ledger-triaged).

## Visual validation

Not measured — visual gate OFF for this shift (substrate work; no
in-scope `@elohim-visually-validated` surface).

## Follow-up objective candidates

B peer-recovery drill (after the PVC decision) · D quilt-shaped
aggregates (p2p-design-gate + reach-enum reconciliation first; junction
now fillable with the controller alive) · E emitter-side pull state
field · dataplane refactor backlog items 1-9
(`arch-dataplane-refactor-backlog.md`).

Journal (full iteration log):
`.claude/shifts/2026-06-11T12-15-epr-durability-cluster-validation.journal.md`
(gitignored; backup at /tmp/sprint-journal-backup.md)
