---
name: feedback-ci-evidence-underdetermines-cluster-diagnosis
description: "When CI surfaces UNSTABLE + downstream failure + empty probes against a fresh deploy, the failure shape on the cluster is the ground truth — ci-investigator output is necessary but not sufficient. Ask for kubectl-side evidence before attributing causality."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: f5ed8831-8faa-47bc-a508-fa91142db0de
---

When CI surfaces UNSTABLE + downstream failure + empty probes against a fresh deploy, the failure shape on the cluster is the ground truth — ci-investigator output is necessary but not sufficient. Ask the operator for `kubectl get pods -A | grep <namespace>`, `kubectl describe pod <stuck-pod>`, and `kubectl logs <stuck-pod>` before attributing causality.

**Why:** During Task 10 of substrate-rea-replication-fix (2026-05-27), I triaged orchestrator #1069's UNSTABLE result entirely from Jenkins logs + alpha HTTPS probes and produced three confidently wrong attributions:
- "9 peer rollouts timed out" was actually "9 pods Running, readiness probes failing because embedded conductor `install_app` was timing out under CPU contention on `shem`."
- "timothy-tutor peer offline" was actually "no workload named `elohim-timothy-tutor-alpha` exists anywhere — the seeder polls a name that doesn't match the deployed StatefulSet (`elohim-timothy-alpha`)."
- "substrate-rea code not implicated" was actually "every healthy peer is spamming `InvalidHashFormat` from `elohim/elohim-storage/src/p2p/inventory_gossip.rs:132–134` because the verifier expects bare hex but the canonical wire format is `sha256-<hex>`."

The operator's cluster-side `kubectl describe` + `kubectl logs` pass corrected all three in one round. The pattern: CI sees pods-not-Ready, log lines about hash format, and empty API responses — the same surface symptoms map to a half-dozen root causes, and ci-investigator can only see the symptoms.

**How to apply:** Before writing a "halted, here's the cause" report when CI is UNSTABLE + probes failed:
1. Note that you have CI-visible evidence only.
2. List the candidate root causes consistent with that evidence (don't collapse to one).
3. Ask the operator for the specific kubectl signals that would disambiguate. Pattern: `kubectl get pods -n <ns>` + `kubectl describe pod <stuck>` + `kubectl logs <stuck> --previous` + a name-grep across all namespaces if the failure mentions a workload that should exist.
4. If the operator confirms "kubectl is operator-only" (see [[feedback_no_kubectl_from_dev_env]]), say so explicitly — frame the report as "CI evidence, kubectl needed to disambiguate" rather than "the cause is X."

Related: [[feedback_verify_cluster_state_before_runbook]] (the inverse — verify cluster matches manifests before writing instructions); [[feedback_no_kubectl_from_dev_env]] (kubectl boundary I cannot cross).
