---
id: nachalah-candidate-contract-brief
title: Authenticate a conductor candidate without granting activation
status: Active
class: protocol-canonical
gap: nachalah-supervised-activation#1a
actor: agent:implementer@gpt-6
habits: [runtime-upgrade-propagation]
cites:
  - "nachalah-supervised-activation-sprint-handoff | Nachalah next sprint | sha256:ce355b7759e4ed56 | path: genesis/docs/superpowers/plans/2026-09-05-nachalah-supervised-activation-sprint-handoff.md"
---

Base: `7fb0e9d0c`. The ark prerequisite is committed and independently reviewed;
`just gate elohim-ark` passed format, clippy and 120 tests on this tree.
Mesh declarations and package projections from the handoff remain uncommitted.
Preserve them and all unrelated work. No staging, commits, or pushes in this seat.

Implement the pure, non-actuating candidate contract substation of station 1.
This is useful input validation, explicitly not completion of station 1 or a
permission to stop a process. Return a type named for authenticated candidate
content, never an activation grant or authorized release. Do not add an apply
vehicle, supervisor control path, or schema claim that conductor adoption exists.

## P2P gate, before types

The existing release is Notarized (A): `content_store_integrity::EntryTypes::Content`,
created/updated by `content_store::{create_content,update_content}` with existing
`ContentCommitted` projection. The current release identity is a Holochain
ActionHash despite the storage field's `release_cid` name; artifact and manifest
identities are CIDs. No integrity change or HTTP route is required here.

The signed request is a local operational projection (C) of the elected release,
delegation and installed observations. Its identity is the existing EPR envelope
CID; reconstruct it from authenticated inputs, never infer adoption from it.
It has no DHT head, SQLite table, Automerge document or transport in this substation.
This substation does not introduce a recovery journal or classify irreplaceable
fallback material as disposable. Network stage never weakens signature, identity,
compatibility or rollback checks. Existing artifact fetch selects transport.

The future authority is the existing Notarized (A) `mishpat_integrity::Commitment`
with `action=delegates-compute`, created by `mishpat::create_commitment` (entry and
action hashes returned), projected by `CommitmentCommitted`. The future material
outcome reuses `content_store::issue_attestation` / Content-backed device-health
attestations. Household budget: 52 releases and 156 three-peer outcome attestations
per year at weekly cadence, plus bounded grants, no per-poll head. This does not
price a fleet above 500 heads or claim measured quiesce impact.

**Missing node #1b:** between verified release and activation, the acting node
must verify notarized grant action/author, issuer, recipient, exact scope, validity
and revocation. `path_evidence` documents author-mintable rosters; its quorum is
not authority. Current commitment fetching omits signed author evidence, and
generic bounds validation does not compare performer to recipient. A locally
pinned signature alone therefore cannot satisfy C12. Keep activation unavailable.

## Contract and bounded files

Own `elohim/ark/core/src/candidate.rs`, its export and purity-source list in
`elohim/ark/core/src/lib.rs`, core contract tests, and relevant rows in
`elohim/ark/core/seam-registry.yaml`. Reuse `elohim_epr::Envelope::canonical_bytes`,
`cid::compute_cid`, and `proof::verify`; add no dependency or custom identity codec.
Read `epr flow context` for scoped files and the owning governance first.

Use a signed EPR envelope with a typed payload binding channel, release ActionHash,
grant reference, target berth identity/incarnation and child, expected predecessor
runtime manifest/executable, candidate manifest/executable, platform, expiry and
protocol capabilities. Bind to independently supplied expected installed state;
never trust a caller-provided key or `verified` flag. Issuer pin is explicit
Agent-EPR-CID plus Ed25519 public key, provisioned separately from incoming content;
never compare that CID to a `uhCAk` agent key. Domain/schema discriminator and
canonical envelope CID must match. Reject malformed identity, wrong issuer/key,
payload tampering, expired/future validity, wrong target, stale predecessor,
wrong platform, missing capabilities and incompatible database format.

For this first contract require identical durable database format across incumbent
and candidate, with both runtimes explicitly reading and writing it. Candidate
version is informational; version-string equality cannot prove compatibility.
Do not accept a candidate that writes a format its predecessor cannot read.
Artifact digest comparison is against independently supplied observed bytes;
this pure function does not hash filesystem paths or claim they are immutable.

Check C0–C14 against actual applicable canon. Register new predicates and refusal
reasons with meaningful independent tests. Mark C12 unbound and supervisor
integration unwired; do not green a future call site. Tampering tests must sign a
valid fixture then change it; pin/key tests use a second key; compatibility tests
alter independent installed facts, not mirror the implementation.

Use owning `just gate elohim-ark`, after claiming cargo through
`genesis/agentic/bin/berth`. Shared host Cargo PID 2724375 was observed in
`.claude/worktrees/integ-push`; coordinate capacity before any build. Sandbox `ps`
cannot establish host process absence. The pool is outside sandbox writable roots;
request tool escalation as needed. Never stop another session's process.

Report exact command/EXIT evidence and limitations in
`task-nachalah-candidate-contract-report.md`. Fulfil only #1a if complete; leave
parent station 1, delegation #1b and the full sprint open. Retain earned-arc
actuation disabled.
