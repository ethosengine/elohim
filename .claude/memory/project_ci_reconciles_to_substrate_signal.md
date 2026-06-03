---
id: project-ci-reconciles-to-substrate-signal
name: project_ci_reconciles_to_substrate_signal
description: CI/CD reconciles its scope to one substrate toggle (ELOHIM_SHEM_STATUS) as the hardware peer set flexes — tolerant but loud; re-run restabilizes
metadata: 
  node_type: memory
  type: project
  originSessionId: 7c732b67-b888-46d5-a52e-6372cedb7b53
---

The CI/CD pipeline is a reconciliation controller over the hardware substrate: when a peer node flaps on/off (e.g. shem, the multi-tenant remote pool hosting most personas), the deployment-wait, seeder, and test suite must self-restabilize to the available compute — not cascade-fail. Partial-cluster is steady state. Re-running the pipeline re-probes and reconverges; no config edit needed. The P2P substrate itself stays fully resilient and k8s-blind — this is purely the CI/planning layer learning to read the substrate.

**The single toggle = `ELOHIM_REMOTE_COMPUTE_STATUS`** (`available|unavailable|unknown`; renamed from the original `ELOHIM_SHEM_STATUS`). Landed 2026-06-02 (commit 032ceb3a8, genesis/Jenkinsfile):
- `Probe Substrate` stage runs `kubectl get node shem` → sets `env.ELOHIM_SHEM_STATUS`; loud reduced-scope banner naming auto-skipped personas; archives `substrate-status.json`; marks build **UNSTABLE not FAILED** when down (honors deployments.json's "Pending if shem is down is intentional fail-loud" + "pipeline tolerates 1..N genesis peers").
- Seed Database / Verify Seeding / Reset Storage reconcile via **seed-whoever-is-ready** (per-peer readiness probe `waitForStorageReady`; skip unreachable → UNSTABLE; fatal only on zero genesis peers seeded or a reachable-but-empty peer — a real defect, never a peer that's simply gone).
- E2E threads `ELOHIM_REMOTE_COMPUTE_STATUS` into cucumber.

**Two signal homes, kept coherent (2026-06-03):** the runtime signal `ELOHIM_REMOTE_COMPUTE_STATUS` and the durable home `genesis/manifests/cluster-state.yaml` must agree. CI/the operator set the env var explicitly (probe-derived) and that always WINS; when it is UNSET, `isRemoteComputeAvailable()` now DERIVES from cluster-state (shem available iff `available: true`), mirroring `scope-reconcile.py`'s `derive_remote_compute_status()`. This closed the last hole: a bare local a2o run used to fail-open to "available" and silently disagree with a `shem: false` declaration. Fail-open survives only when cluster-state itself is unreadable (e.g. a published consumer without the genesis tree). The format-candidate audit detector was scope-gated the same day so it only flags cites the migrator can actually convert (shared doc-root scope). See [[project_gap_granular_substrate_scope]], [[project_cite_seal_born_linked_enforcement]].

**Non-obvious gotcha this fixed:** the a2o gating discipline (`isHumanDeployed` / `nodeTypes:['remote']` auto-skip / `isRemoteComputeAvailable`, in genesis/a2o/src/framework/fixtures/humans.ts) existed and was unit-tested but was **INERT** — nothing ever SET the signal, so remote-only persona scenarios false-failed against dead conductors. The seeder also ignored it entirely (separate, contradictory topology). The fix wires one signal through both.

**Why:** AI/CI defaults to all-or-nothing "every declared peer must be up." The substrate flexes; the right behavior is tolerant-but-loud reconciliation to confirmed compute. **How to apply:** any new pipeline stage that iterates peers/humans must probe-and-skip (UNSTABLE), not hard-wait+exit 1; gate cross-node scenarios so they auto-skip when their pool is down rather than false-fail. Jenkinsfile stage bodies extract to helpers above `pipeline {}` (64KB CPS limit; `.claude/hooks/jenkinsfile-method-size.py`). Regression: `genesis/a2o/features/resilience/substrate-reconciliation.feature` (@resilience-p1) locks "household always reachable; only remote-only personas ever scale out."

Cluster-side faults stay operator-owned (Harbor 502, shem kubelet Ready=Unknown, james AddrInUse CrashLoop). See [[project_seed_whoever_is_ready]], [[project_shem_is_p2p_live_canvas]], [[project_principle_p1_reconciliation_controller]], [[feedback_shift_measure_jenkins]], [[feedback_no_kubectl_from_dev_env]].
