---
id: task-brit-feedback-qualify-20261111
gap: plans__2026-11-11-brit-feedback-operation#4
actor: agent:orchestrator@gpt-6-astra
status: In Progress
---

# Native qualification of the feedback pair

Implement the native qualification operation accepted in the session plan, serving station 4 of `genesis/docs/superpowers/plans/2026-11-11-brit-feedback-operation.md` under Matthew's stewardship. Add `epr feedback qualify --brit <executable> --root <policy-root> --json`, using the invoked EPR executable as recorder and an isolated temporary feedback store. Also support an explicit recorder-only qualification for first installation; it must never claim the pair qualified. Reuse the completed contract in `genesis/a2o/scripts/verify-brit-feedback.ts`: successful and refused command capture, reopening, no private argument capture, exact occurrence-to-Intent linkage, no implicit admission, and recorder failure preserving command exits. Produce the existing CheckWitness shape/outcomes rather than a new telemetry model. Native landing will consume this command; qualify must emit structured failure/unmeasured results and a nonzero exit on either.

Owned paths: `elohim/eprfs/epr-cli/src/feedback/` qualification/dispatch, focused qualification integration tests, and the existing a2o probe/test adapter. Preserve installed discovery behavior until root supplies pinless native resolution. Root owns device-runtime changes and final main help integration. Keep bounded child execution and cleanup; borrow contextual policy without touching the real store or session pins. Explicitly selected binaries are candidates, never installed delivery.

Base: `6d994a931`. Existing implementation is uncommitted in this shared tree; preserve unrelated work. Read context/governance first. Do not commit or promote. Coordinate native builds with root and use the cargo berth. Final report must list exact changed paths, gates and limitations. Root owns final gates, release and habit projection.
