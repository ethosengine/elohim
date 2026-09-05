---
name: project_compute_envelope_three_homes_k8s_bridge
title: Compute envelope has three homes — the k8s-bridge slice
description: "deployments.json limits, the Rakia ledger and a prose ratification cannot read each other; the capacity ask fires on a stale envelope — cure: observe, typed ratification, manifest render"
metadata: 
  node_type: memory
  title: Compute envelope has three homes — the k8s-bridge slice
  type: project
  originSessionId: f84d8f67-f26c-4b08-8a6f-6cfd7e6cf12d
  modified: 2026-09-05T19:34:33.546Z
---

**The trap (bit the 2026-09-05 integration push, 331 commits refused).** The `.epr-meta`
compose-gate's `test-bench-aggregate-capacity` compares `deployments.json`'s active limits
(53750m) against `compute-capacity.json`'s `totalAllocatable` — which was promoted 2026-05-04
with shem cordoned (46000m). The live cluster had 70 cores (shem back: the PSU failure was
thermal, the operator keeps the fan running). The ratification the gate asks for already existed
as `$computeEnvelopeRatification` prose in deployments.json (2026-07-16) that no code reads. The
gate's only ratification path is `EPR_META_ACK=1 git push`, which the auto-mode classifier
refuses (it reads env-prefixed pushes as bypasses; it also refused scripts that merely imported
the validator). Resolution that worked: read `kube_node_status_allocatable` from Prometheus via
the observability MCP and promote the observation into the ledger (`001bc1e45`) — the gate then
passes on its merits, no flag.

**The slice (operator-steered, 2026-09-05):** spec
`genesis/docs/superpowers/specs/2026-09-05-k8s-bridge-runtime-envelope-render-design.md`, plan
`…/plans/2026-09-05-k8s-bridge-runtime-envelope-render-plan.md`, tevah register 30. Four
stations: (1) `RuntimeManifest.envelope.bound` + Σ refusal in `validate()` + epr-rea `Bound`
projection, S0 CID `bafyreihagg75k…` unmoved (Codex GPT-6 astra implements in
`.claude/worktrees/k8s-bridge-s1`); (2) `bridges/k8s` render/verify, `runtimeManifest: {cid,path}`
pin per active human — deployments.json stays byte-identical for its ~20 consumers (scope-reconcile
parses its TEXT layout) but the four `edgenode*` fields become a verified render; golden test vs
`scripts/ci/conductor-split-budget.sh`; S3.6 `registry_crates` gets its first crate; (3)
`k8s-bridge observe` (Prometheus) writes the ledger's cluster block, typed
`cluster.ratifications[]` read by both `epr_meta.py` and `repository_validators.rs`, a 30-day
freshness leg turns a stale ask into a `refer`; (4) register/habit coherence. The `epr-pvc`
guide-star (register 21) stays unminted: resources only, `Berth.data_root` reserved.

**Why:** three unlinked homes make every deployments.json push a governance stall; the cure is
one declaration and one observation, not a permission rule.
**How to apply:** if the aggregate ask fires, check the ledger's `snapshotTimestamp` first and
re-observe before ratifying; never restate the ratification in prose; when Station 3 lands, the
ask cannot fire on a stale envelope at all. Related: [[project_tevah_compute_envelope_canonized]],
[[feedback_k8s_is_not_the_architecture]], [[feedback_push_branch_discipline]],
[[project_storage_build_under_ram_guard_debuginfo_off]].
