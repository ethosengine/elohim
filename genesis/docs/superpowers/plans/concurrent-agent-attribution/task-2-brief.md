---
id: concurrent-agent-attribution-task-2-brief
title: Coordinate concurrent actor and flow authors without losing evidence
status: active
class: process-meta
gap: plans__2026-09-05-concurrent-agent-attribution-plan#2
actor: agent:implementer@gpt-6
cites:
  - "concurrent-agent-attribution-design | The existing-primitive, concurrency and evidence-preservation contracts this implementation must satisfy. | sha256:e1ba98b4f3ca4b47 | path: genesis/docs/superpowers/specs/2026-09-05-concurrent-agent-attribution-design.md"
  - "concurrent-agent-attribution-plan | Station 2 transaction, recovery and independent-process acceptance requirements. | sha256:af37cc0afeb713a8 | path: genesis/docs/superpowers/plans/2026-09-05-concurrent-agent-attribution-plan.md"
---

Implement station 2 only. Read the governing specification and plan. Preserve station 1's
uncommitted attribution changes. No new domain entities, atom encodings, authority or identity
semantics; filesystem coordination is an implementation detail of the existing sidecar seam.

Files in scope:

- `elohim/epr-rea/src/actor.rs`
- `elohim/epr-rea/src/store.rs`
- `elohim/epr-rea/src/lib.rs`
- `elohim/epr-rea/tests/fabric.rs`
- `elohim/epr-rea/seam-registry.yaml`
- `elohim/eprfs/epr-cli/src/actor.rs`
- `elohim/eprfs/epr-cli/src/flow/claim.rs`
- `elohim/eprfs/epr-cli/src/flow/fulfill.rs`
- `elohim/eprfs/epr-cli/src/flow/note.rs`
- `elohim/eprfs/epr-cli/tests/actor_claim.rs`
- `elohim/eprfs/epr-cli/tests/flow_edges.rs`
- `elohim/eprfs/epr-cli/seam-registry.yaml`

Small implementation-private sidecar helpers and focused test modules in these same crates are
allowed where they reduce duplicated mechanics; no new model, registry, daemon or dependency.
Honor the sidecar feature boundary and unchanged filesystem-free consumers. Parent will inspect
and repair the uncovered epr-rea manifest gate separately; do not edit manifests or justfile.

Make first open non-truncating; coordinate whole-record append/read and relevant read-check-append
transactions. Duplicate same-intent claims must not both win; exact retries remain idempotent.
Do not deadlock by locking an independently reopened handle inside a transaction. Specify which
durability guarantee is actually implemented. Lock/I/O failure cannot look like acceptance.
Interrupted writes must never be silently deleted or turned into valid evidence: preserve bytes,
surface the condition and prove the selected recovery/refusal behavior. Do not introduce an
unreviewed automatic truncation or repair command.

Tests must exercise separate OS processes, simultaneous first-open, same-role distinct workers,
competing claims, retry idempotence, coherent readers and interrupted writes. Include deterministic
red-state evidence where feasible; do not claim race reproduction from source inspection alone.
Measure a representative log's read/check cost before considering optimization. Preserve old
record CIDs and station 1 claim pins.

Run epr flow context for scoped files and derive gates from manifests. Claim/release the real
cargo berth before Cargo; no substitute private berth directory. Report exact command EXIT lines.
No staging, commits, install or push. Review is of the scoped worktree diff from the base; all
existing changes outside this slice belong to other work. Write task-2-report.md here with the
skill's required frontmatter, evidence, remaining limits and commits: []. Execute the skill's
single terminal flow verb. If a gate cannot run, preserve progress and diagnose safe alternatives.
