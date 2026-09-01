---
id: "backlog-task-release-soak-attestation-rail"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: soak-attestation rail — author context-bearing soak/build attestations riding an existing generated attestation kind, and the threshold reader that turns them into promotion evidence"
slug: "task-release-soak-attestation-rail"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "spec:runtime-artifacts-elected-content"
  - "backlog-task-release-adoption-controller-observe"
  - "backlog-task-release-apply-vehicles"
tags: [upgrade-propagation, rung5, attestation, soak, evidence, elohim-storage, delegable]
---

**Claimable by any implementation agent. Standalone — parallel with T3/T4; it
owns its own file and exposes two public fns the siblings call. The spec's
evidence leg (§5 BuildAttestation/SoakAttestation): what moves a release from
staging to earned is attested soak, never assertion.**

## Why

Promotion must carry evidence with CONTEXT — hardware archetype, region,
probe results — because two peers' different experiences of the same release
are information (rakia stage-2-canopy), and context is what lets a regional
channel elect what fits while commons holds the envelope. The verification
workflow (2026-09-01) proved the trap this task must not fall into: the
generated `ATTESTATION_KINDS` list is compiled INTO the integrity zome
(`content_store_integrity/src/attestation_validator.rs` floor 1 +
`generated_attestation_kinds.rs`) — a NEW kind is a DNA-hash move. MVP rides
an existing kind.

## P2P design-gate decision

Carried by spec §5: Notarized (A) riding an EXISTING generated attestation
kind with a `metadata_json` discriminator (`kind: "release-soak"` /
`"release-build"`); agent-scoped composite identity (agent × releaseCid ×
kind); DNA-hash-NEUTRAL by construction — this task MUST pick the existing
kind from `generated_attestation_kinds.rs` (read the list; choose the least
semantically-violent fit and record the choice + rationale in this atom) and
MUST pass the validator's metadata_json floor checks. C1: the threshold
reader EXCLUDES the release's own builder agent from the qualifying count.

## Scope

1. `elohim/elohim-storage/src/services/release_attestation.rs` (its own
   file — disjoint from T3/T4's module):
   - `pub fn author_soak_attestation(ctx, release_cid, soak: SoakContext,
     outcome: SoakOutcome) -> Result<AttestationRef, TypedRefusal>` — authors
     through the conductor (the attestation authoring rail the consolidated
     kinds already use), `metadata_json` carrying the discriminator +
     context: `{kind, releaseCid, channelId, deviceArchetype, region,
     probeResults, buildInfo, outcome}`.
   - `pub fn count_qualifying_attestations(ctx, release_cid, discipline:
     &AdoptionDiscipline) -> QualifyingEvidence` — resolves attestations for
     the release cid, filters by discipline (count now; the struct carries
     archetype/region so diversity thresholds are additive later — spec
     §11.2), excludes the builder (C1), and reports
     `{qualifying, total, byArchetype}`.
2. SoakContext population from the runtime passport + boot registration
   (device archetype / capability level — `boot_registration.rs`).
3. Probe `genesis/a2o/scripts/release-attestation-probe.ts`: author two
   attestations from two mesh peers for a fixture release cid, read back the
   qualifying count from a third, prove builder-exclusion with a negative
   control.

## Interface contract (consumed by T3 verify, T4 post-apply)

- The two pub fns above are normative; T3 calls the reader in its threshold
  arm, T4 calls the author post-apply. Both compose against the structs, not
  the storage tables.

## Disjointness contract

- MAY create `release_attestation.rs`, the probe script, tests, edit this
  atom.
- MUST NOT edit `services/release_adoption/` (T3/T4's module — they add the
  call sites), the integrity zome, `generated_attestation_kinds.rs` or its
  generator (the first-class kinds are the spec §11.1 batch), or sibling
  scripts.

## DoD + verification

- Probe exits 0 on the mesh: attestations authored from two peers land,
  validator floors pass, third peer counts 2 qualifying; builder-authored
  attestation provably excluded.
- The chosen existing kind + rationale recorded in this atom; `cargo test`
  green for the module.
