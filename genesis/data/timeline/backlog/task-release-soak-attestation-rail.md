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
claimedBy: "claude-opus-t5"
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

## Implementation notes (2026-09-01)

Landed in the working tree, uncommitted — the integrator commits path-limited.
`status` stays `open`: the authoring half is **proven live**, the cross-peer
threshold half is **blocked on two measured substrate defects** that live in
files this atom must not edit (below).

### Chosen kind: `attestation:device-health` (infrastructure)

Read from `generated_attestation_kinds.rs` (24 kinds, verified on disk). The
constraint is real and was proven live, not assumed: on the household mesh,
`issue_attestation` with `attestation_kind: "attestation:release-soak"` is
refused with `unknown_attestation_subtype` — a new kind IS a DNA-hash move.

Why device-health is the least semantically-violent fit:

- **The payload axis fits exactly.** Its declared metadata
  (`device-health-metadata.schema.json`) is a health metric summarised over an
  observation window — `device_id`, `health_metric` ∈ {uptime, latency,
  availability, throughput, error-rate}, `period_start`, `period_end`,
  `sample_count`, `summary_value`. That *is* a soak window. We author
  `health_metric: "availability"` — "did it stay up".
- **The sentence it produces is true.** "Device D reports availability over
  window [start,end]" — with the subject anchor naming the release D was
  running. `device_id` names the device; the anchor names the release. Nothing
  has to be pretended.
- **Its live reader cannot be poisoned.** The only consumer of device-health
  (`infrastructure` zome, `lib.rs:821`) reads
  `get_attestations_for_subject(doorway_id, …)` — keyed on a DEVICE cid.
  Release attestations anchor on a RELEASE cid; the streams never intersect.
- **It is already authored through this exact rail** — the infrastructure DNA
  mints device-health via `content_store::issue_attestation` (`lib.rs:773`), so
  we compose the rail rather than opening a second one.

**Rejected: `attestation:content-quality`.** Its subject axis is right (a
release manifest is a `Content` entry), but it is a *reach-grant* attestation
with a closed `quality_dimension` enum, and it HAS a live consumer that renders
verification badges to learners
(`app/lamad/src/app/services/data-loader.service.ts:1029`,
`listBySubject(contentId, 'attestation:content-quality')`). Riding it would
surface release soak evidence as content-verification badges. Same semantic
strain, plus a user-visible blast radius.

**Residual strain, recorded not hidden:** the infrastructure manifest declares
`subject_kinds: ["device"]` while we anchor on a release CID. Nothing enforces
`subject_kinds` today (validator floors 2/3/4 are `TODO(C.3)`), and first-class
`attestation:soak` / `attestation:build-provenance` kinds belong in the §11.1
constitutional DNA batch.

### Where the discriminator + context live

Inside `metadata_json`, under **`proof_evidence`** — not under
`evidence_json.summary_metric`:

1. It is semantically the right home (probe results ARE the evidence).
2. Integrity **floor 8** validates `proof_evidence.class` + its required
   material, so the shape passes a LIVE floor. Proven: `class: "audit"` with no
   `merkle_root` is refused with `floor8_failed`. We author `class: "witness"` —
   the peer witnessed its own soak; witness needs no extra material.
3. It keeps `evidence_json.summary_metric` conformant to device-health's own
   `additionalProperties: false` schema, so a genuine device-health reader that
   ever meets one of these rows sees a well-formed device-health summary.

### What landed

- `elohim/elohim-storage/src/services/release_attestation.rs` — both normative
  fns plus the structs T3/T4 compose against. C1 is enforced **by type**:
  `AdoptionDiscipline` has no public constructor; the only way to make one is
  `ChannelAdoptionDiscipline::for_release(builder_agent)`, so the builder
  exclusion cannot be forgotten at a call site.
- One `mod` line in `services/mod.rs`.
- `genesis/a2o/scripts/release-attestation-probe.ts` — the wire-level mirror of
  the module, so it runs against a mesh whose storage binary predates it.
- `SoakContext::from_runtime(config, …)` reads the same sources boot
  registration does (`device_archetype` / `region` / `household_id` /
  `node_role`) plus the runtime passport's own `BuildInfo` (never the pin tag).

### Verification evidence

- `just gate elohim-storage` (migration-hygiene + fmt-check + `cargo clippy --
  -D warnings` + `cargo test`) → **EXIT=0**. Lib suite
  `test result: ok. 3107 passed; 0 failed; 2 ignored`; 164 integration binaries
  + doc-tests, 0 failed suites.
  23 unit tests cover every pure decision surface (wire shape, schema
  conformance, floor-8 pre-check, C1 exclusion, laundered-copy rejection,
  id-collision rejection, revocation, one-voice-per-agent, degraded-count
  flagging).
- Live on the 3-peer household mesh (matthew/jessica/james), 2026-09-01
  18:52–18:54Z, release `release-soak-probe-1788288668227`:
  - floor 1 refuses a new kind (`unknown_attestation_subtype`) ✅
  - floor 8 refuses `audit` without `merkle_root` ✅
  - all three attestations committed; `serde_json::Value` crosses the WASM
    boundary intact and the full context round-tripped byte-perfect through
    `metadata_json` ✅
  - `get_content_by_id("attest-{kind}-{issuer}")` returns `author_id` +
    `metadata_json` with the discriminator + context ✅
  - **cross-peer count NOT achieved** — see blockers.

### Blockers for the integrator (both outside this atom's write-set)

Story-graph nodes, in mintable shape:

1. **chain** release-promotion-evidence / **between** "N peers author soak
   attestations" → "a peer counts N qualifying" / **missing node** *"a peer's
   attestation is visible, per-issuer, on another peer"* / **state** BROKEN.
   `content_store/src/attestation.rs:89` stamps
   `Content.id = format!("attest-{kind}-{issuer}")`. That id is not unique per
   attestation and IS the `attestations` projection's PRIMARY KEY, so one issuer
   can hold at most ONE row per kind across ALL subjects, forever — a second
   release's soak silently REPLACES the first. **Probe:** author two
   device-health attestations from one agent on two different subjects; the
   projection keeps one row. **Fix:** include the subject in the id
   (coordinator-only change → DNA-hash-neutral, hot-swappable via
   `sync_coordinators`).
2. **chain** attestation-provenance / **between** "peer A authors an
   attestation" → "peer B reads it" / **missing node** *"the issuer survives
   replication"* / **state** BROKEN. Root cause located exactly:
   `services/reanchor_backfill.rs:51` — `is_canonical_content_type` returns
   TRUE for `attestation:` / `governance-action:` prefixes, so
   `p2p/projection_reconcile.rs` (~2251-2295, 2559-2667) feeds peer-discovered
   attestation rows into `reanchor_backfill::run_once` →
   `ContentService::update_via_conductor` → `conductor_writes::call_create_content`,
   which mints a NEW DHT entry authored by the **local** agent. The re-author
   loop's own doc says it "re-authors either kind identically" — correct for
   ordinary content, provenance laundering for an attestation, where the author
   IS the claim. Measured: on jessica, all three probe attestations projected
   with `issuer_cid = jessica`, each at a distinct local ActionHash, while the
   row `id` still named the real issuer. This affects *every* attestation
   authored through `issue_attestation`, not just release ones — worth a
   red-team look beyond this atom. **Probe:** the probe's
   `SQL PROJECTION … ← LAUNDERED` contrast lines. **Likely fix:** exclude the
   attestation/governance-action prefixes from re-authoring (they are
   author-bound by construction; a re-author is not a heal), and let the
   AttestationToSubject link walk be the only cross-peer read path.
3. **Also observed, lower stakes:** `services/attestation_projector.rs:198`
   reads `metadata["evidence"]`, but the coordinator writes
   `metadata["evidence_json"]` — so the `evidence_json` column is `{}` on every
   attestation authored through this rail. Not on this module's path (it reads
   the conductor), but it is why the projection cannot serve subtype metadata.
4. **Link gossip:** ~5 min after authoring, each peer's
   `get_attestations_for_subject(release_cid)` returned only its OWN
   attestation (1/3). The re-check at ~50 min could not run — the devspace
   container restarted and took the mesh with it (`/tmp/elohim-local-mesh/pids`
   gone). **The live probe run is the integrator's step**: bring the mesh up and
   run `cd genesis/a2o && pnpm exec tsx scripts/release-attestation-probe.ts`.
   Exit 0 = full DoD; exit 3 = rail proven, count blocked with a named
   diagnosis printed.

### Deviations from the atom's letter

- Both pub fns are `async` — authoring and reading both cross the conductor,
  and `HcClient::call_zome` is async. Arity and names are unchanged.
- The reader reads **through the conductor**, not the local `attestations`
  projection, because of blockers 1–2: the projected `issuer_cid` cannot answer
  C1. It cross-checks the link walk's authenticated issuer against the entry's
  `author_id`, which makes it fail-closed against both defects (they can only
  DEFLATE a count). `QualifyingEvidence` reports `provenance_mismatched` /
  `unresolved` and `is_degraded()` so an under-count is never read as a real
  evidence deficit.
- Per-decision metrics (C8) are NOT wired — `metrics.rs` is outside the
  write-set. Integrator station.
