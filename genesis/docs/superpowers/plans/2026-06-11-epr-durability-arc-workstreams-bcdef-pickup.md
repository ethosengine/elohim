# EPR Durability Arc — Workstreams B–F pickup (fresh-session dispatch prompt)

Continuation of `2026-06-10-epr-durability-replication-arc-plan.md` after the
2026-06-11 overnight shift (sprint result:
`genesis/docs/shifts/2026-06-11-epr-durability-sprint-result.md`). Workstream A
is storage-side complete; this prompt picks up B–F. Paste-able mission for a
fresh session follows.

---

## Mission

Finish the EPR durability arc: prove healing, peer-loss failover, truthful
aggregates, projection durability, and doorway federation failover on the
alpha cluster — CI-gated, story-first, stability-gated done (3 consecutive
green genesis runs per gate).

## FIRST ACTION — read the wave-6 verdict before anything else

At shift close (21:20Z 06-11), orchestrator #1224 was deploying the
attestation-lockout DNA fix (edge #1063 → genesis #1127). Check:

1. `bash .claude/shifts/2026-06-11T12-15-epr-durability-cluster-validation.measure.sh`
   — **0** means custody-convergence flipped: Workstream A's gate is a
   done-candidate; one more fresh-trigger genesis run completes its stability
   gate. **1** means the attestation chain still needs tracing: did the happ
   reinstall fire (ALLOW_DNA_REINSTALL=true on alpha; check edge #1063's
   installer + conductor wasm hash in rejections — old hash was
   `uhCokSPspAA…`), did doorways re-register ("re-registered under a NEW
   operator agent" WARN in conductor logs / "Doorway registered in DHT" on
   doorway pods)?
2. Loki: have the 5-minute `"Only the doorway operator can record
   attestations"` rejections stopped on matthew/adam conductors?
3. Genesis #1127 artifacts: `propagation.custody-convergence`,
   `resilience.*` (peer-statuses still dark until the infrastructure-role
   connect is fixed — separate item below).

## WAVE-6 VERDICT (landed 22:27Z, after this prompt was first written)

Genesis #1127 completed (4th consecutive); **measure stayed 1**. The
attestation rejections CONTINUE at 22:26Z with the SAME OLD wasm hash
(`uhCokSPspAA…`) even though edge #1063 deployed and matthew's pod
restarted (new UID 32d4509c). **The happ reinstall is NOT firing.** The
FIRST ACTION's trace therefore narrows to: (a) how edge resolves
HAPP_TAG — did #1063 (an empty-commit retrigger) consume the happ bundle
from DNA #1325 or a stale tag?; (b) the happ-installer container's own
runtime log (different container label than elohim-node — find it via
label values) — did it run, and did its stale-check + ALLOW_DNA_REINSTALL
logic decide skip?; (c) if the installer logic only reinstalls on
role-structure change even with the flag, that's the bug (gospel says the
flag forces it — verify). The DNA fix itself is correct and built; only
its delivery is stuck.

POSITIVE side-finding (21:58Z, matthew): `update_via_conductor: stale
dht_anchor_hash — healing via create_content re-publish` fired across
dozens of content ids — the conductor path is actively healing the
bulk-seed anchor gap (ci-seeder-stamp-conductor-anchor-circularity.md);
expect reach re-notarization and anchor coverage to improve run-over-run.

## State of the world (verified 2026-06-11, do not re-derive)

- **Mesh PROVEN**: 14/14 pods `connected:13`; `mesh.adjacency` both
  directions green in CI (#1123/#1124/#1126); persistent-peering redial
  live. **Byte plane PROVEN**: probe blob replicates same-build (0s/1
  attempt), filesystem replicas persist. **Discovery PROVEN**: per-peer
  inventory lines name responders; matthew's commitments discovered
  fleet-wide.
- **Reconcile controller CONNECTED** (first time ever — app-id fix
  `app_id=args.app_id, role="imagodei"`); junction stamps
  (`humans.household_id`, dual-vocabulary keying, family-gated) are live
  code awaiting real MembershipProjected signals.
- **Formation**: identities seed conflict-free on own conductors (3
  created); formation has a DHT-settle retry on `DepMissingFromDht`; full
  ceremony completion unverified — confirm in the latest genesis log.
- **Attestation lockout** (operator-diagnosed): zome now allows
  re-registration + latest-wins lookup; deploy verdict per FIRST ACTION.
- **Retracted**: the PVC/DhtOp "gossip gap" diagnosis — conductor DBs are
  HEALTHY; do not touch PVCs (see superseded
  `dna-conductor-dht-gossip-gap.md` for the museum lesson: source-pin
  subagent-quoted log lines).

## Workstreams

**B — healing & backup-restore drill.** Scenario exists
(`genesis/a2o/features/federation/peer-recovery.feature`, @wip). Implement
the CI drill: wipe jessica's blob store via a pipeline pod-op, prove
heal-on-read + custody sweep reconverge (filesystem_count returns, blob
GETs 200). Gate: genesis Jenkinsfile is CPS-capped — drill bash lives in
`genesis/scripts/ci/`, one thin stage call. Destructive pod-ops are
operator-ratified per the arc plan but verify the current stance before
wiring. Precondition: FIRST ACTION verdict healthy.

**C — sync & peer counts.** Transport floors proven; remaining: (1)
peer-loss failover scenario (`peer-loss-failover.feature` landed) — kill a
pod mid-suite, assert reads keep serving + returning peer re-syncs; (2)
tune `MESH_MIN_CONNECTED` floors per-peer from the real topology now
archived in `substrate-verify-mesh.json` `context.peers[].connectedPeerIds`;
(3) truthful peer counts need `resilience.peer-statuses` lighting — blocked
on the **infrastructure-role connect failure** (role fails 5/5 on
matthew/jessica: role name vs happ manifest — likely the same class as the
fixed imagodei app-id mismatch; check `connect_role("infrastructure")`
inputs vs the happ's actual role names; bounded fix candidate).

**D — free-storage / stewarded-commitment aggregates.** OPERATOR-CONFIRMED
precondition (task #5 metadata): aggregates must be quilt-shaped, not flat —
(1) run `p2p-design-gate` against the tiered-quilt-stewardship seed (MAP
D5): dimension by tier (archive vs cache/draw) AND reach class; (2)
reconcile the 3-way reach-enum drift FIRST (canonical = schema +
`elohim/epr/src/reach.rs`, pinned by the new bijection test; divergent =
`epr_kind.rs:88` + doorway `custodian.rs:55`; decisions in the coherence
audit's design-gate lane); (3) scope the `content:<reach>` provide-row gap
(test_util-only). Junction is fillable once formation completes
end-to-end.

**E — projection durability (kill `restart-doorway-epr.sh`).** Landed:
EprRouter per-row degradation + truthful pool-fallback classification +
duplicate-key WARN (the Welcome-at-/ class is guarded at both seams).
Remaining: (1) root-cause the crutch — SSE subscriber reliability vs
documented eventual-consistency (coherence audit design-gate #4; Loki
correlate doorway refresh outcomes vs storage `projection_reconcile`
caughtUp); (2) emitter-side pull state field (`rollup()` exposing
`idle|active|caughtUp` explicitly — the CI tri-state then passes idle
confidently instead of WARNing); (3) delete the crutch + a2o scenario.

**F — doorway federation failover.** DESIGN-GATED: the coherence audits
returned conflicting verdicts on whether failover-retarget violates the
no-fan-out rule (doorway/CLAUDE.md). Adjudicate first (options: replication
makes single-target sufficient / sequential re-target on 5xx from an
operator pool / failover lives in storage P2P). Then implement +
`peer-loss-failover.feature` assertions. Breadth legs @requires:shem.

## Standing rails (unchanged)

Commit-only unless a push lease exists in `.claude/data/push-lease.json`
(expired = ceiling; AskUserQuestion to renew). Orchestrator under-build
hole: after ANY failed wave, force `[build:app,edge]` style tags (comma
syntax) — baselines advance at plan time
(`ci-orchestrator-baseline-advance-despite-failure.md`). Fire-and-forget
DNA races edge deploys — push DNA changes in their own wave or retrigger
edge after the DNA build completes (tonight's ebbe201f7 lesson). Genesis
Jenkinsfile + root Jenkinsfile CPS-capped: bash in `scripts/ci/`. Container:
no nextest, pool-slot fingerprint ENOENT → /tmp targets, RUSTFLAGS="" for
doorway/native, ambient for storage/WASM, plain cargo in DNA workspaces.
Loki: storage container=`elohim-node`; adam burst chunks 502 — zero-results
during 502s are query failures. Never kubectl from dev. p2p-design-gate
before ANY new entity. Story-first: scenario + implementation land
together; story-harvest on finish.

## Open side-items (pick up opportunistically)

read-pool saturation on ethosengine conductors (Util 125-162% — spread
cells / raise read threads); `record_health_attestation` CI-caller decision
(`dna-health-attestation-ci-authz.md`); eyes-sprint CSS budgets +
fontawesome path (`app-css-budgets-...md`); mishpat integrity sweettests
(`dna-mishpat-integrity-defense-in-depth-coverage.md`); dataplane refactor
backlog items 1-9 (`arch-dataplane-refactor-backlog.md`); SDK proposal's 3
design questions → /brainstorm (`arch-dataplane-sdk-proposal.md`);
elohim-epr Jenkins job provisioning (operator).

## Done

Per gate: 3 consecutive green genesis builds. Arc-level: published content
survives multi-peer loss — wipe drill green, failover green, aggregates
truthful, crutch deleted, resiliency scenarios passing every genesis build.
