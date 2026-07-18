---
id: "backlog-keyrotation-mint-path-witness-backed"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "KeyRotation mint path — no coordinator fn mints a valid witness-backed KeyRotation (blocks identity-lineage end-to-end recovery)"
slug: "keyrotation-mint-path-witness-backed"
written: "2026-07-18"
author: "identity-head arc (Wave B review, operator: defer-the-mint)"
status: "open"
priority: "high"
area: "imagodei/identity-recovery"
domain: "D2"
jobs: [elohim-edge]
relatedNodeIds:
  - "memory:project_rea_compute_commitment_primitive"
cites:
  - genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
tags: [identity, key-rotation, recovery, humanity-witness, key-stewardship, cryptographic-quorum, m4-migration]
---

# KeyRotation mint path — the deferred last mile of identity-lineage recovery

## Why this exists
The identity-head arc (Waves A–C) ships the lineage primitive: chain-root over the
KeyRotation DAG, the `binds-identity` controller declaration, the `rotate_identity_key`
authorization gate, and did:elohim resolution of real controllers + lineage. All
DNA-hash-neutral, reviewed, banked. **But no coordinator fn mints a valid `KeyRotation`
entry**, so the *success* path (an authorized/recovery-quorum rotation actually appending a
node + advancing the head) is proven only by pure-logic unit tests + the authorization gate
— never an end-to-end conductor mint.

Root cause (confirmed, Wave B review): post-M4 cross-DNA migration, `submit_intimate_witness`
synthesizes a `HumanityWitness` for signal emission but **writes none to imagodei's DHT**
(`imagodei/zomes/imagodei/src/lib.rs:3913`); `recovery_m3::m3_happy_path_intimate_quorum` is
a bodyless `TODO` stub. `validate_key_rotation`'s `IntimateQuorum` branch resolves
`witness_hashes` via `get()` against entries no coordinator fn creates.

## Consequence
The grandma-recovery a2o (identity-head plan Wave D) lands as `@wip` partial — authorization
gate + wiring + read-side chain-walk proven — with the true end-to-end mint deferred here.
Operator decision 2026-07-18: defer the mint, ship the primitive.

## The narrower path (Wave B review finding)
The **`CryptographicQuorum` recovery variant does NOT depend on `HumanityWitness`** — an M-of-N
*cryptographic* controller quorum could mint a real `KeyRotation` without rebuilding the
witness/M4 attestation path. That is the likely-minimal unblock: a `rotate_identity_key`
success path gated on `CryptographicQuorum` controller signatures, aligned with the
recovery-quorum controllers `binds-identity` already declares. The full witness-backed
(`IntimateQuorum`) mint remains the larger, M4-dependent piece.

## When picked up
Flip the identity-head plan Wave D from `@wip` to green; route through `p2p-design-gate`
(this mints a notarized entry) and the DNA-hash-neutrality gate (coordinator-only). Prefer the
CryptographicQuorum path first as the minimal end-to-end proof.

## ⚠ HARD PRECONDITION — this mint path MUST NOT land without plan task B1b
(Whole-arc review, 2026-07-18.) Today every identity is a degenerate single-node chain, so the
storage `identity_root_cid(k)` (trim-only, returns `k`) and the imagodei `identity_chain_root(k)`
(walks the DAG) are *accidentally equal* — the ONLY reason the Wave-A re-pointings (REA
provider/receiver, `claimed_agent_id`) and the identity-head `chain_root` coincide. The instant a
real KeyRotation can be minted, `identity_chain_root(new_key)` walks back to genesis `G` while
storage `identity_root_cid(new_key)` still returns `new_key` → every re-pointing diverges from the
head's `chain_root` and **silently breaks** (a claim stored under `G` won't resolve for a lookup
routed through `new_key`; an REA provider written as `new_key` won't match a head with
`chain_root=G`). So landing this mint path REQUIRES the plan's **B1b** (upgrade storage
`identity_root_cid` to walk the DAG AND route the read filters through the resolved root) in the
SAME change — otherwise the arc's data integrity silently regresses. See
`genesis/docs/superpowers/plans/2026-07-17-identity-head-key-lineage-plan.md` Wave B, task B1b.
