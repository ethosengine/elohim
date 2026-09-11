---
id: nachalah-grant-record-brief
title: Authenticate the exact notarized grant record at the acting node
status: Active
class: protocol-canonical
gap: nachalah-supervised-activation#1b-a
actor: agent:implementer@gpt-6
habits: [runtime-upgrade-propagation]
cites:
  - "nachalah-supervised-activation-sprint-handoff | Nachalah next sprint | sha256:ce355b7759e4ed56 | path: genesis/docs/superpowers/plans/2026-09-05-nachalah-supervised-activation-sprint-handoff.md"
---

Base: `7f95ac47fda5ee3f6f5176b747145a76aa9e19f1`. Preserve the shared worktree.
No staging, commits, pushes, household process control, or integrity-zome edits
in this seat. The signed candidate increment is committed and reviewed.

Implement exact signed commitment record authentication, a prerequisite within
station #1b. This output proves immutable grant content and author only. It must
not confer activation permission or claim fresh non-revocation. Parent #1b and
the sprint remain open. Earned-arc actuation stays disabled.

## P2P gate and scope

The existing grant is Notarized (A), `mishpat_integrity::EntryTypes::Commitment`
in the mishpat DNA (`dna/mishpat/dna.yaml`). `mishpat::create_commitment` returns
both ActionHash and EntryHash, with the existing `CommitmentCommitted` projection.
Exact ActionHash binds the authoring act; EntryHash binds content. Distinct authors
can create identical immutable entry bytes. Existing `get_commitment(EntryHash)`
cannot distinguish those acts and omits signed action evidence.

Readback and authentication add zero entries at seed or one year, no recurring
head sweep/election cost and no extra SQLite/Automerge projection. Reuse the
existing conductor connection; no HTTP route or new transport. Coordinator-only
changes are DNA-hash-neutral. Signature/author/identity checks are constitutional
floors at every network stage. An in-memory authenticated result is operational
(C), reconstructable by resolving and verifying the notarized record.

Add an exact-ActionHash coordinator read that returns the existing signed Record
shape, with correct Commitment app entry type checks. Preserve legacy readback
for existing callers; correct its misleading one-record-per-entry comment.
Use the matching Holochain hashing and serialization primitives; SignedAction
signs serialized ACTION bytes, not ActionHash or a JSON re-encoding. Add the
storage conductor adapter and acting-node verifier. Do not add Holochain runtime
dependencies to ark's pure core.

The verifier accepts independently pinned expected ActionHash, EntryHash and
Holochain author key; none comes from the record under verification. Recompute
the action and entry identities, verify the actual signature, require Create of
the expected Commitment entry definition, reject Update/Delete/non-Commitment
records, and return private-field read-only authenticated content. Enforce bounds
on input work before serialization/hash/crypto. The Agent EPR CID that signs a
candidate is NOT a Holochain agent key: no cross-namespace equality or inferred
issuer binding. Explicit independent provisioning remains required until the
full authority verifier binds issuer, recipient, scope, validity and freshness.

Own a new coordinator module under
`elohim/holochain/dna/mishpat/zomes/mishpat/src/` plus its module export in
`lib.rs`; storage `services/conductor_writes.rs`, a new service verification
module and `services/mod.rs`; meaningful independent contract tests and owning
seam-registry rows. Read each tree's local governance and epr flow context before
edits. Reuse existing traits/types and crypto. Add no dependency unless an
existing direct dependency cannot expose the needed pure primitive; report it.

## Required proof

Use two independent signing keys and real Holochain-shaped signed fixtures.
Refuse a different author creating identical entry bytes; tampering with action,
entry bytes or signature; mismatched independently expected action/entry/author;
wrong app entry type; Update/Delete; missing entry; malformed/oversized input.
Valid exact record succeeds without any process effect. Tests must not use the
verifier's own helper to derive expected outputs or a mock boolean signature.
Register C0–C14 answers from canon. Mark full C12 authority and activation
unbound, with the remaining #1b-b station as the gap.

Run owning gates returned by epr flow context and focused execution tests (a
compile-only cargo check is insufficient). Claim cargo through
`genesis/agentic/bin/berth` and inspect host activity before using shared capacity.
Use pool slots for native cargo and the DNA workspace's normal WASM placement.
Escalate tools as needed for host process observation and pool writes. Do not
stop another session's processes. Report command, direct EXIT status and limits.

Write `task-nachalah-grant-record-report.md`; fulfil only #1b-a if verified.
If a required gate is unavailable or red, use HOLD with exact evidence, not a
completion claim. Root will obtain independent review before accepting the work.
