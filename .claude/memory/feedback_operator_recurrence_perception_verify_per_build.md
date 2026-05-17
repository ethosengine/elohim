---
name: operator-recurrence-perception-verify-per-build
description: "when operator says \"we keep hitting this,\" verify per-build via getBuild + first_failing_stage before assuming same root cause; recurrence-shape (F/U/F/F/U cluster) ≠ same-root-cause"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: d5ebc70b-b1ff-43c0-9172-9d14847a28ec
---

When the operator describes a CI/pipeline state as "we keep hitting this issue," do not assume the same root cause across the recent failure cluster. Pull `first_failing_stage` and a one-line failure signature for each of the last N builds and confirm the failure SHAPE matches before scoping the fix.

**Why:** Surfaced 2026-05-17 by the `rca-orchestrator-963-graph-failure` shift. Operator pointed at build 963 and said "we keep hitting this." Builds 959–963 were F/U/F/F/U/F (five non-clean in a row). Naïve framing: "we keep hitting a graph failure." Investigator pass:

| Build | First-failing-downstream | Class |
|---|---|---|
| 959 (U) | elohim-genesis #1010 | downstream FAILURE → orchestrator UNSTABLE |
| 960 (F) | elohim-edge #968 | hard FAILURE |
| 961 (F) | elohim-edge #969 | retry on same Cargo.lock change |
| 962 (U) | E2E Cucumber parse | feature-file grammar errors |
| 963 (F) | elohim-epr Trigger Downstream | `No item named elohim-epr/dev found` — first occurrence |

Five distinct failure classes. The recurrence was the macro-shape of a string of unrelated failures, not one bug biting repeatedly. The shift would have framed the fix wrong (and probably missed the actual class 5 entirely) if it had taken the "graph failure keeps recurring" framing at face value.

**How to apply:** Iteration 1 of any RCA shift should include a "recurrence pattern" step: for each of the last 5 builds on the branch, dispatch ci-observer or ci-investigator to retrieve first_failing_stage + a one-line failure signature. If the classes diverge, the shift is fixing class N (the one the operator pointed at); classes 1..N-1 become follow-up Objective candidates surfaced in the sprint result. Don't fold them into the active Objective unless they share a root cause.

This is the inverse of [[cascade-halt-masks-failures]] — there, fixing one root cause unmasks more. Here, the cluster has multiple roots and the danger is collapsing them.

See also: [[cascade-halt-masks-failures]], [[cascade-hidden-test-surface]].
