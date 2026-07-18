---
title: Alpha Test-Bench Compute Envelope — observed constraint to governed commitment
id: alpha-test-bench-compute-envelope
tier: architecture
status: Ratification-pending operational governance contract
created: 2026-07-16
pillar coupling: rakia (capacity observation), shefa (bounded commitment), elohim (reach governance)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-04-compute-commitment-substrate-floor-design.md
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
informs:
  - genesis/data/rakia/compute-capacity.json
  - genesis/data/devices/archetype-resource-budgets.json
  - genesis/orchestrator/data/deployments.json
---

# Alpha Test-Bench Compute Envelope

The alpha Kubernetes cluster is a development test bench, not the protocol architecture. It is,
however, the physical constraint against which this repository can honestly promise deployments
today. This EPR models the relationship without turning Kubernetes into protocol truth.

## Three projections, one signal path

1. `genesis/data/rakia/compute-capacity.json` is the operator-promoted observation of the cluster's
   allocatable CPU and memory. It is a Category-C operational projection derived from a read-only
   snapshot, not a self-ratifying measurement.
2. `genesis/data/devices/archetype-resource-budgets.json` models the canonical per-instance budget
   for each hardware archetype. A justified `resourceOverride` preserves local variance.
3. `genesis/orchestrator/data/deployments.json` composes the active portfolio. A resource edit is
   checked first against its archetype, then the portfolio is checked against the promoted cluster
   envelope.

The aggregate check replaces the capacity ledger's previously observed human allocation with the
prospective active portfolio. It retains the ledger's non-human commitments, then compares projected
requests and limits with `cluster.totalAllocatable`. The same calculation runs when either the
deployment portfolio or the capacity ledger changes.

## Agency and reach

Arithmetic is the substrate floor: deterministic evidence that can stand without an Elohim. It does
not decide whether an overcommit is wise. Both policies therefore use `ask`. Authors remain free to
experiment locally; when the change reaches others, the steward must either reconcile the budgets,
promote a newer observed capacity ledger, or explicitly acknowledge the exception. That
acknowledgement is a governance action, not a claim that the hard constraint disappeared.

The future protocol form remains the existing bounded compute `Commitment` plus linked
`FeedbackSignal`; this repository gate introduces no new DHT entry type, HTTP route, or database
authority.
