---
title: The host-compatibility foundation — lanes from story to code for the contract, the key, variant sets, placement, rakia and the feedback rails
id: host-compatibility-foundation-plan
status: Draft
class: architecture
date: 2026-11-10
steward: human:matthew
serves: [sprint-lands-each-closure-once, measure-runs-on-a-peer, workspace-device-arrival, dataplane-convergence]
refines: host-compatibility-foundation-design
cites:
  - "host-compatibility-foundation-design | the design this plan lanes: contract, key, variant sets, immune loop and feedback rails | sha256:31df4116d3d82d04 | path: genesis/docs/superpowers/specs/2026-11-10-host-compatibility-foundation-design.md"
  - "artifact-store-sprint-2026-11-07 | the in-flight sprint whose owed receipts lane K re-runs and whose hub points here | sha256:4b76fac9f23efae3 | path: genesis/docs/superpowers/sprints/2026-11-07-artifact-store/plan.md"
  - genesis/data/timeline/backlog/2026-08-29-compute-envelope-virtual-peer-contract.md
  - genesis/orchestrator/.epr-meta/sprint-lands-each-closure-once.habit.md
  - genesis/agentic/.epr-meta/measure-runs-on-a-peer.habit.md
  - .epr-meta/workspace-device-arrival.habit.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - "algedonic-feedback-signal | the design lane F proposes the five feedback rails to as an amendment | sha256:cf902beac8dd4aec | path: genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md"
  - genesis/a2o/.epr-meta
  - .claude/skills/valueflow-authoring/SKILL.md
---

# The host-compatibility foundation — sprint plan

The design is `host-compatibility-foundation-design` (the spec). This plan is only its lanes: each
`- [ ]` item below is a gap item that `epr flow project` mints, that one seat claims, and that closes
with a report and its verify evidence. A seat reads the spec for the why; this plan names the
story, the spec section, the code and the receipt for each item, so the chain from story to code
can be followed in one place.

Gap ids are positional. New items are appended at the end of the document; items are ticked, not
reordered or removed.

## Rulings in force (2026-11-10)

- **H1–H7 ruled as recommended** by the steward: K1 is key honesty only; K1b is withdrawn in favour
  of variant selection; a device may run a variant built under link inputs it lacks, subject to
  `admit` and reach; the declared target is the default for shared native builds; the contract
  lives in a small pure crate under `crates/`; aarch64 is the second-architecture proof; a coarse
  host class may be published in an attestation, limited to proven bundle names.
- **H8, H9 agreed**: portable vs native is drawn by must-agree vs may-differ; variant sets are open
  and membership is decided by a named conformance suite.
- **Direction ruled**: variety is the protocol's strength; damage stays cheap and repair happens at
  the edge (spec §7.4). H11 (the six rules and the seed as written) is **open**. H10 (rakia) is
  **open** and gated below.
- **Inherit, do not mint** (spec §9): every record is expressed through `elohim/epr` and
  `elohim/epr-rea` primitives unless the item says why not.
- **Rails, not one pipe**: feedback is judged against five rails (spec §9); several channels may
  remain.
- **No habit is set active.** The two-slot fence is held by `dataplane-convergence` and
  `push-delivers-within-budget`. Items serve habits; they do not occupy the fence.
- **Stories** use the full blind-reader loop of `genesis/a2o/.epr-meta`; no variance is granted.
  A story lands with, or before, the first code item it specifies.
- **Landing** follows the landing lane. This plan becomes claimable when it reaches the tree
  `epr flow project` runs on.

## Lanes and order

Lane A and items K1–K2 run first; every other lane reads them. Then K3–K7 and A2–A4 in parallel;
lanes V and P after K3; F3 with A2; F4 is independent. Lane S stories precede their lane's first
code item. Lane X is named and not scheduled.

| Lane | Seat | Write-set |
|---|---|---|
| A | ark | `crates/<contract crate>/**`, `elohim/ark/**`, the preflight script |
| K | artifact store | `elohim/eprfs/eprfs-core/src/derive.rs`, `elohim/eprfs/epr-cli/src/artifact.rs`, `elohim/eprfs/epr-cli/src/peer.rs`, `elohim/eprfs/eprfs-peer/src/{fetch,daemon}.rs` |
| V | artifact store + ark | `elohim/eprfs/**` bin store and fetch, `elohim/ark/core/src/manifest.rs` |
| P | compute | `elohim/placement/**`, `elohim/ark/cli/src/task.rs`, `genesis/agentic/compute/measure.sh`, rakia schemas (after H10) |
| S | the seat of each lane | `genesis/a2o/features/{compute,devflow}/**` |
| F | feedback (algedonic design steward) + ark + a2o | the algedonic spec, `elohim/ark/core/src/{witness,exit}.rs`, `genesis/a2o/scripts/**` |

## Lane S — the stories

Chain: these stories are the specification each later lane is accepted against.

- [ ] **S1 — refine the four harvest stations (backlog 2026-10-22, spec rung H5) into `compute/measure-on-a-peer.feature`, `compute/peer-executed-stage.feature` and `compute/ci-runner-stewarded.feature`; the on-target station carries `@requires:owned-substrate`** (serves measure-runs-on-a-peer)

Verify: each scenario carries `@concern:measure-runs-on-a-peer`; one fresh blind reader returns READY with no correctness finding; round counts recorded.

- [ ] **S2 — variant selection and typed refusal in `devflow/device-declared-target.feature` and `devflow/sibling-fetches-the-build.feature`: a v2 device refuses a v3 variant before launch naming the unmet axis, and takes the v2 variant of the same pin** (serves sprint-lands-each-closure-once)

Verify: as S1, with `@concern:sprint-lands-each-closure-once` and `@concern:dataplane-convergence` where the scenario already carries it.

- [ ] **S3 — a `@wip` story for a device the network has not seen: it runs the portable tier from a seed, builds its own variant, and a later device of its class resolves that variant directly (spec §7.5)** (serves workspace-device-arrival)

Verify: as S1, with `@concern:workspace-device-arrival`.

## Lane A — the contract (spec §3, §4; rungs H0–H1)

Chain: S2 → spec §3.3 refusals → the contract crate → vector tests.

- [ ] **A1 — a small pure crate under `crates/` holding the axis vocabulary, bundles, `Requires`, `Provides` and `admit`, returning `elohim_epr::verdict::Verdict`; registered in its `seam-registry.yaml`; vectors include requirements extracted from binaries built on the other device** (serves sprint-lands-each-closure-once)

Verify: the crate's vector tests pass (v2 on v3 admitted, v3 on v2 refused, glibc floor both ways, unknown axis refused); `just gate rust-style` clean for the crate; the report states whether an unknown axis is a `Refuse` or a `Refer` and why.

- [ ] **A2 — ark observes `Provides` at start beside its `Fingerprint` (`elohim/ark/supervisor/src/driver.rs`, `native.rs`), not serialized off the device** (serves workspace-device-arrival)

Verify: both devices print their observed `Provides`; they differ in the ISA bundle; a test asserts it is not part of any serialized record.

- [ ] **A3 — `authenticate_candidate` (`elohim/ark/core/src/candidate.rs`) admits against the observed host instead of comparing to the installed runtime's triple; `WrongPlatform` carries the unmet axis** (serves workspace-device-arrival)

Verify: the candidate contract tests cover an admitted and a refused candidate on synthetic `Provides`.

- [ ] **A4 — `just mesh preflight` prints one admitted-or-refused line per mesh binary** (serves dataplane-convergence)

Verify: preflight output on both devices; a deliberately foreign binary is refused with its axis.

## Lane K — key honesty and family (spec §6.3, §7; rungs H2–H3; reviews F1–F6, G1, B1–B2)

Chain: S2 → spec §6.3 and §7 → `derive.rs`, `artifact.rs`, `fetch.rs` → the F2r receipt.

- [ ] **K1 — the derivation input holds source and cargo configuration only; host tool identity moves into the method, derived from the build (effective linker from `rustc -vV`, link inputs the link read), retiring the fixed keyed-tool list in `artifact.rs`** (serves sprint-lands-each-closure-once)

Verify: the two devices' keys for the same source and declared target are equal; a changed link input changes the key; a changed unused system tool does not.

- [ ] **K2 — the declared target under the reserved `target.` prefix of the method's non-path inputs; target-cpu lifted out of `RUSTFLAGS`; each recorded build-environment variable classified as target or method; `family_cid` as a named rule in `eprfs-core/src/derive.rs`** (serves sprint-lands-each-closure-once)

Verify: existing method CIDs do not move for descriptors without `target.*`; v2 and v3 descriptors of one recipe share a family CID; unit tests in `derive.rs`.

- [ ] **K3 — `Produce` events gain additive `env:family` and `env:requires` slots; on an own-key miss, fetch reads by family, groups by key, admits each variant, chooses, re-derives the chosen variant's key itself and resolves holders for that key only** (serves sprint-lands-each-closure-once)

Verify: a fetch on the v2 device of a family that holds a v2 and a v3 variant selects v2 and fetches it; the daemon's key re-check passes on the re-derived key.

- [ ] **K4 — a second output under one key narrows the holders (to the set's named CID, or by trust policy) and is recorded as divergence evidence, instead of refusing the fetch** (serves dataplane-convergence)

Verify: a test with two differing outputs under one key fetches from the agreeing holder and records one divergence observation.

- [ ] **K5 — holder lookup by output CID over the same `Produce` fold, and a fewer-than-N-holders warning read from it** (serves dataplane-convergence)

Verify: a lookup by `env:output` returns the same holders as the key lookup for a single-variant family.

- [ ] **K6 — a declared target with a pinned sysroot (itself a recorded derivation with a key and a requirement); F2r closes with equal keys in both directions, including the v3-on-v2 refusal under emulation** (serves sprint-lands-each-closure-once)

Verify: the F2r receipt names the sysroot CID and shows one identity hit each way; the emulated v2 run refuses the v3 variant before launch.

- [ ] **K7 — re-run the artifact-store sprint's owed receipts (F1r sibling reuse, the F2r fetch half) on the new key** (serves sprint-lands-each-closure-once)

Verify: both receipts in `sprints/2026-11-07-artifact-store/evidence/`, linked from its `RESULT.md`.

## Lane V — variant sets and conformance (spec §7, §7.1; rungs H4, H9; reviews G2, G3, B5)

Chain: S2 → spec §7 and §7.1 → the variant-set document, the daemon, the bin store → berth records.

- [ ] **V1 — a content-addressed `VariantSet` document; `ArtifactRef` and `RuntimeManifest` may pin one; the berth records the variant its device resolved** (serves dataplane-convergence)

Verify: one manifest CID resolves to different variants on the two devices; both berth records shown.

- [ ] **V2 — a conformance suite as a pinned `ProcessSpec`; a run's result recorded as an `Affirm` or `Dismiss` flow event on the variant CID naming the suite CID and the proven bundle names** (serves sprint-lands-each-closure-once)

Verify: two variants judged by one suite with expectations minted by the other device's variant; a deliberately wrong variant is refused (rung H9).

- [ ] **V3 — a second admission path in the fetch daemon for adapted variants: set CID and member CID with no key, with typed refusals `not-in-set`, `set-not-accepted`, `attestation-insufficient`** (serves dataplane-convergence)

Verify: one admitted and three refused fetches, each refusal naming its reason; output and chunk CID checks unchanged.

- [ ] **V4 — landing a set member as a device tool is judged by membership in an accepted set, through an explicit recorded transition shaped like the bin store's lineage rebase** (serves sprint-lands-each-closure-once)

Verify: an adapted member lands with its recorded transition; a non-member is refused as before.

## Lane P — placement, measure and rakia (spec §7.3, §10; rung H5; H10)

Chain: S1 → spec §10 → placement `admit`, `ark task run`, `measure.sh` → the measure-on-a-peer scenarios.

- [ ] **P1 — a compatibility stage in `elohim/placement/core/src/eligibility.rs` between the gate stage and the capacity stage, with its own `IneligibleReason`; the provider evaluates the task's variant set against its own `Provides`, kept beside `DeviceFacts` and not serialized** (serves measure-runs-on-a-peer)

Verify: placement tests refuse an incompatible variant with the new reason and admit the compatible one; consent and gate ordering unchanged.

- [ ] **P2 — the same admission inside `ark task run` (`elohim/ark/cli/src/task.rs`, the carried-here step) and beside the executor resolution in `genesis/agentic/compute/measure.sh`** (serves measure-runs-on-a-peer)

Verify: `MEASURE_DRY_RUN=1 just measure <feature> --on jessica` prints the admitted variant or the refusal before launch.

- [ ] **P3 — ruling H10 put to the rakia steward: the declared target as an input of a rakia step, and building and proving as separate planned steps, with execution staying in `ark task run`; no rakia schema change before the ruling** (serves sprint-lands-each-closure-once)

Verify: the ruling is recorded as an `epr flow note --kind ruling` on this plan; spec §12 H10 updated to match.

## Lane F — feedback rails (spec §9; define, audit, convert one)

Chain: spec §9 rails → the algedonic design → ark's witness and the a2o reporter → one query by subject and kind.

- [ ] **F1 — the five rails (attributed, addressed subject, declared kind and reach, one identity derivation, pain folded from a declared bound) proposed to the algedonic design's steward as an amendment to `2026-08-10-algedonic-feedback-signal-design.md`** (serves dataplane-convergence)

Verify: the amendment is in the algedonic spec with its steward's ruling noted, or the refusal and its reason are recorded on this plan.

- [ ] **F2 — an audit table in this plan's lane F report: each feedback implementation (CI, runtime and deprecation ledgers, gate tap, ark witness, brit attestations, `epr flow` notes, storage observations, a2o sprint reports, brit command use) against the five rails** (serves dataplane-convergence)

Verify: every implementation has a row with file references; each failed rail names the smallest change that would meet it.

- [ ] **F3 — ark's death witness names the variant CID and the contradicted `Requires` clause (additive fields); `ExitClass` gains illegal-instruction and loader-failure classes** (serves workspace-device-arrival)

Verify: an illegal-instruction death under emulation produces a witness naming the variant and the ISA clause; old witnesses still decode.

- [ ] **F4 — convert one: the a2o sprint-report reporter also records each finding as a flow observation on the household attestation's system-under-test CID, following the Jenkins bridge and brit parity-observation pattern; the 12-hex fingerprint stays as an index key** (serves dataplane-convergence)

Verify: one household run's findings are read back with `epr flow` by subject CID and kind, alongside a ci finding projected by the Jenkins bridge.

- [ ] **F5 — the misfits recorded as open decisions on the rails amendment, not forced: witness identity for a node, service or ark; a verb for failure distinct from dismissal; close-by-delete in the runtime ledger vs append-only records** (serves dataplane-convergence)

Verify: each misfit has an owner and a stated question in the algedonic spec or this plan.

## Lane X — named, not scheduled

- [ ] **X1 — name the seed's members (`epr`, `brit`, `ark`, the cargo toolchain for a sibling-less bootstrap) and measure their size on both devices** (serves workspace-device-arrival)

Verify: a table of members, sizes and what each carries, in the item's report.

- [ ] **X2 — open frontier, each a later plan: second architecture (rung H6), the `Wasm` driver (H7), one accelerator axis (H8), the immune loop on two devices (H11), a finding reach narrower than the steward's crossing (review B4), and commons-rung holders for wider reach (review G4)** (serves dataplane-convergence)

Verify: each frontier item has a named home (backlog entry or spec section) before this plan closes.
