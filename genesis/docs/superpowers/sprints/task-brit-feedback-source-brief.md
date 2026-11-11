---
id: task-brit-feedback-source-20261111
gap: plans__2026-11-11-brit-feedback-operation#2
actor: agent:orchestrator@gpt-6-astra
status: In Progress
---

# Retained feedback is a native reconciliation source

Implement station 2 of `genesis/docs/superpowers/plans/2026-11-11-brit-feedback-operation.md` under Matthew's stewardship. Reuse `acceptance::EvidenceResources` to resolve verified occurrence CIDs through the existing FeedbackStore. Missing, corrupt and reclaimed evidence must remain distinguishable; document sources retain their existing checks. Immutable captured views must not implicitly read private device files. Original Intent, commitment, actor bindings and scope CIDs remain unchanged; availability never grants acceptance authority.

Owned implementation paths: `elohim/eprfs/epr-cli/src/flow/acceptance.rs`, `elohim/eprfs/epr-cli/src/flow/reconciliation.rs`, a narrowly scoped source resolver module if needed, and focused reconciliation tests. Coordinate any core feedback-store changes with the integrator. Read existing feedback CLI policy resolution; do not duplicate it. No edit to feedback command dispatch, qualification, launcher or a2o files.

Base: `6d994a931`. Existing feedback implementation is uncommitted in this shared tree; preserve all unrelated work. Run context for owned paths and nearest governance before editing. Do not commit or promote artifacts. Cargo runs need the declared pool slot and berth; coordinate builds with the integrator. Tests must cover original scope/actor continuity, absent/corrupt/reclaimed records, document regressions, view isolation, and lack of automatic acceptance. Report exact changed paths and verification evidence. Root owns final gate, publication and habit projection.
