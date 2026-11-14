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
- **H12, H13 and the scope of K10 are ruled** (2026-11-12, under the steward's delegation;
  `sprints/2026-11-11-host-compatibility/rulings-2026-11-12.md`): reach on the object decides who
  may draw it, with `serves` kept as a narrowing list; builds from different native compilers
  share a family and not a key (H13); K10 is built as the own-build rule first. Nothing in lanes A, K,
  V or P waits on a steward ruling as of 2026-11-14. Only P3 does (H10).
- **Inherit, do not mint** (spec §9): every record is expressed through `elohim/epr` and
  `elohim/epr-rea` primitives unless the item says why not.
- **In-kind work is still accounted** (steward, 2026-11-12): work the steward's own devices supply
  to each other is supplied in kind, and it still records the promise and what ran. Every run that
  follows an admission sits under a commitment (the work, its expected budget, and the `Requires`
  or variant set it needs), and its fulfilment's evidence names the variant CID, the providing
  device and its steward, the admission verdict and the measured cost. In kind means the price is
  zero or self-settled, not that the record is skipped. The rails are the ones
  `elohim/placement/core` already has (`CommitmentDraft`, `FulfilmentDraft`, `settle`); no
  compute-only ledger. Reason: compute accounting is the first market these primitives must be
  mastered on, so that hosting can later be negotiated across the network's boundary and the same
  shape reused by the other REA-native markets. The observed `Provides` stays on the device as
  A2 requires (its test asserts it is in no serialized record); only the verdict is recorded.
  Amends A3, P1 and P2 below.
- **A sibling's gate verdict may stand in for a local run** (steward, 2026-11-12), when the
  evidence supports it: deterministic floor, elohim ceiling. The floor is what a program can
  check: the same input closure, a valid attestation signed by a device on the steward's roster,
  a green result, clean inputs. Above the floor an elohim is called to validate the call (for
  example a gate whose result may depend on the host, or a record with declared gaps), and it is
  the party that repairs when an accepted verdict proves wrong, by the reconciliation walk of
  lane R. The ceiling is meant to be called rarely, and for questions of diversity, resilience,
  governance, integrity and witness. The contract, the extraction and the recorded inputs exist
  so that a job and the peer under it can know before a run whether it will fit, refuse early,
  and send that refusal back as feedback. Each time an elohim is called for something a program
  could have checked, that is a finding, and its repair is a new floor check. An accepted sibling
  verdict is accounted like any other supplied work (ruling above).
  Corrected 2026-11-14: an earlier wording here said `GATE_EVIDENCE=reuse` accepts only this
  workspace's own attestation and that the mechanism was still to be designed. The floor has been
  in the gate runner since 2026-10-27 (habit `push-gate-pays-once`, its DELTA of that date;
  `genesis/orchestrator/gate-evidence.mjs` `reusableHit`, `gate-runner.mjs` at the
  `siblingVerdicts` call). After a local miss the runner asks `epr peer verdicts` for
  `gate-verdict@1` rows from roster-admitted devices and reuses a green one for the exact
  closure, behind the project allow-list, clean inputs, the selector check, a red from any device
  as a veto, and a refusal when the push changes the gate's own governance paths. What is still
  owed is in lane G at the end of this plan: no push on either device has yet printed
  `independence: sibling`, and the closure key does not see the C compiler or the host's
  instruction set.
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

- [x] **S1 — refine the four harvest stations (backlog 2026-10-22, spec rung H5) into `compute/measure-on-a-peer.feature`, `compute/peer-executed-stage.feature` and `compute/ci-runner-stewarded.feature`; the on-target station carries `@requires:owned-substrate`** (serves measure-runs-on-a-peer)

Verify: each scenario carries `@concern:measure-runs-on-a-peer`; one fresh blind reader returns READY with no correctness finding; round counts recorded.

- [x] **S2 — variant selection and typed refusal in `devflow/device-declared-target.feature` and `devflow/sibling-fetches-the-build.feature`: a v2 device refuses a v3 variant before launch naming the unmet axis, and takes the v2 variant of the same pin** (serves sprint-lands-each-closure-once)

Verify: as S1, with `@concern:sprint-lands-each-closure-once` and `@concern:dataplane-convergence` where the scenario already carries it.

- [x] **S3 — a `@wip` story for a device the network has not seen: it runs the portable tier from a seed, builds its own variant, and a later device of its class resolves that variant directly (spec §7.5)** (serves workspace-device-arrival)

Verify: as S1, with `@concern:workspace-device-arrival`.

## Lane A — the contract (spec §3, §4; rungs H0–H1)

Chain: S2 → spec §3.3 refusals → the contract crate → vector tests.

- [x] **A1 — a small pure crate under `crates/` holding the axis vocabulary, bundles, `Requires`, `Provides` and `admit`, returning `elohim_epr::verdict::Verdict`; registered in its `seam-registry.yaml`; vectors include requirements extracted from binaries built on the other device** (serves sprint-lands-each-closure-once)

Verify: the crate's vector tests pass (v2 on v3 admitted, v3 on v2 refused, glibc floor both ways, unknown axis refused); `just gate rust-style` clean for the crate; the report states whether an unknown axis is a `Refuse` or a `Refer` and why.

- [ ] **A2 — ark observes `Provides` at start beside its `Fingerprint` (`elohim/ark/supervisor/src/driver.rs`, `native.rs`), not serialized off the device** (serves workspace-device-arrival)

Verify: both devices print their observed `Provides`; they differ in the ISA bundle; a test asserts it is not part of any serialized record.

State (2026-11-14, ethosengine): built on `hostcompat/lane-a-admission` (81605d283, 179ef207b).
`ark host provides` prints on ethosengine (v3 set plus `sha_ni`, `adx`, glibc 2.39); the
privacy tests pass in `just gate elohim-ark`. Open: shem's print, for the two-device check.

- [x] **A3 — `authenticate_candidate` (`elohim/ark/core/src/candidate.rs`) admits against the observed host instead of comparing to the installed runtime's triple; `WrongPlatform` carries the unmet axis** (serves workspace-device-arrival)

Verify: the candidate contract tests cover an admitted and a refused candidate on synthetic `Provides`. The admission returns a verdict a caller can record (variant CID, permit or the unmet clause) without exposing `Provides`, so P2 can put it in a fulfilment's evidence (ruling "In-kind work is still accounted").

State (2026-11-14, review of d12a28be..240693e3): the verify above is met by contract tests. No
running path enforces admission yet: outside tests nothing calls `admit_artifact`, builds a
`CandidateObservation` or calls `authenticate_candidate`. Two bindings the spec states are
conventions in the code and not checks, and item A5 (end of this plan) closes them before the
first caller (P1, P2, V4 or R1) is wired. (1) `CompatibilityEffect` derives `Deserialize` and has
a public constructor in ark-core, and `authenticate_candidate` checks only that the effect names
the same executable and that its decision is a permit, so a hand-built effect with no checks
authenticates. (2) `OwnProduce::from_local_lookup` takes two bare CIDs and looks nothing up, so
"this device recorded the Produce" (ruling condition 2) is the caller's word.

- [ ] **A4 — `just mesh preflight` prints one admitted-or-refused line per mesh binary** (serves dataplane-convergence)

Verify: preflight output on both devices; a deliberately foreign binary is refused with its axis.

State (2026-11-14): part 1 done (238528a6a): `ark host admit --requires <file>` prints one
admitted or REFUSED line naming the axis and required value; a declared `avx512f` requirement is
refused on ethosengine. Part 2, the preflight lines, is blocked: no epr verb returns a recorded
output's requirement document, and reading the flow log and payload layout from the mesh script
would duplicate the store's format outside eprfs. Missing node: `chain A4 / between mesh-bin
derivation record → ark host admit / missing node: epr artifact requires --output <cid>
(env:requires CID, document bytes, self) / not built`.

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

- [ ] **K6 — a declared target with a pinned sysroot (itself a recorded derivation with a key and a requirement); F2r closes with one family in both directions (each device records its own key under it; H13), including the v3-on-v2 refusal under emulation** (serves sprint-lands-each-closure-once)

Verify: the F2r receipt names the sysroot CID; each device records its own key under one family; each fetches the other's variant by family and admits or refuses it; the emulated v2 run refuses the v3 variant before launch. Keys are equal only when the compilers are too (H13, ruled 2026-11-12, `sprints/2026-11-11-host-compatibility/rulings-2026-11-12.md`).

- [ ] **K7 — re-run the artifact-store sprint's owed receipts (F1r sibling reuse, the F2r fetch half) on the new key, reuse by family between devices with different compilers (H13)** (serves sprint-lands-each-closure-once)

Verify: both receipts in `sprints/2026-11-07-artifact-store/evidence/`, linked from its `RESULT.md`.

Items K8–K12 were added 2026-11-11 from spec §7.9 (an artifact is born self-describing) and §7.8
(reach on the object). Each one is a precondition for a second device to close a gap from the
record alone. State at addition: K1, K2, K3 part 1, K4, K5 and K6 slice 1 are on dev (6e8c1e2a1).

- [ ] **K8 — `put` extracts `Requires` from the bytes itself (arch, OS, loader, needed libraries, libc symbol floor, kernel ABI note, ISA level) with `host_contract::Requires` and writes `env:requires`; a caller-supplied `--requires` may only make a clause stricter; an artifact whose requirements cannot be extracted refuses** (serves sprint-lands-each-closure-once)

Axis ids and version lines come from the `host_contract` constants the A1 owner is adding (arch, os, isa.x86, kernel.linux, abi.libc, abi.loader, lib; lines glibc, musl, linux), never from literal strings, so two spellings cannot silently fail to compare. K8 starts after those constants land. Constraints from the A1 owner (shem, 2026-11-11): `put` builds `Requires` through `Requires::try_from(Vec<Clause>)` or `insert`, so a conflict between a declared and an extracted clause surfaces as `MergeError` and never silently weakens; extracted clauses carry `source: extracted`; the building host's `Provides` is never recorded (it is not serializable by design).

Code is decoded within function boundaries: symbol-table function sizes and `.eh_frame` FDE ranges, plus the ABI stub sections (`.init`, `.fini`, `.plt` and kin) whole. Only a non-stub range whose linear decode hits an invalid or truncated instruction falls back to bounded control-flow traversal from its start, allowing inline tables such as V8’s while recording unreached bytes in `partialFunctions` and section coverage; a reached invalid instruction still refuses. A partial scan records `isa.x86.partial-decode` in `env:gaps` (`pending, not enforced`); its ISA clause is then a lower bound. The exhaustive flow policy includes both XBEGIN abort-target encodings and transactional fallthrough, and stops all UD exception instructions. Executable bytes outside every boundary are counted in the extraction evidence, not decoded. The extraction evidence (what each axis read, example symbols per feature, coverage) is its own document, recorded as `env:requires-evidence`. Hint-space instructions that execute as NOP without the feature (ENDBR) are listed as not required.

Verify: recording the epr built on ethosengine yields the x86-64-v3 level with `source: extracted` and its sqlite3 evidence (`walIndexTryHdr` avx2, `sqlite3LogEst` lzcnt), plus `abi.libc ≥ glibc 2.39`. Measured 2026-11-11 (K8 evidence, `k8-extraction-ethosengine.json`): the clause also carries seven extras reached only from runtime-dispatched code (ring's `adx`, `aes`, `pclmulqdq`, `vpclmulqdq`, `vaes`, `sha_ni`; compiler_builtins' `fma4`), counted until K10 proves the dispatch. The same record admitted against a synthetic v2 `Provides` refuses naming the missing features; an unreadable artifact refuses.

- [x] **K9 — the key's input lines are stored as a document by the input CID, readable by the person's own devices (reach `self`) like the method record, so another of their devices can diff its own lines against the record's** (serves sprint-lands-each-closure-once)

Reach: `self`, not wider. The lines include the compiler's predefined-macro dump and the link-input paths, which reveal the building host's default ISA and sysroot layout: a host fact, which spec §4 keeps local. Between one person's devices that disclosure is within the roster's trust (H12). Beyond them, a declared-target build (K6) keys the floor by its CID in place of host lines, so what crosses at wider reach is the floor's identity, not the host's.

Verify: on two devices of one person, the same descriptor's input documents are fetched and diffed, and the diff names exactly the differing lines (on ethosengine and shem: `link:Scrt1.o`, `link:crt1.o` and the macro dump); the stored document's CID equals the key's input CID; a requester that is not one of the person's devices is refused the document.

Receipt 2026-11-12 (`k9-two-device-receipt.md`): both directions paired the 8b273f826 records and printed the same 11 differing lines. The macro dump agrees; the whole glibc and gcc link floor differs at identical paths (RHEL 10.0 glibc -145 against AlmaLinux 10.2 glibc -128 x86_64_v2, same gcc version), which is the floor K6 keys by CID. Reaching it took dev's crossing policy on both daemons and record-exact serving and pairing (6060f8347).

- [ ] **K10 — dispatch evidence (spec §3.2): the ISA extraction counts only code reached without a runtime feature check, on evidence bound to the bytes (compiler multi-versioning metadata or a build-time attestation naming guarded functions by content), not a symbol-name pattern; unproven dispatch counts as a requirement** (serves sprint-lands-each-closure-once)

Recording: a clause narrowed by dispatch evidence carries `Clause.narrowing` (`Narrowing { removed, evidence }`, host_contract, provenance only: admit ignores it), so a reader sees why the requirement shrank (A1 owner's review note, 2026-11-11). When narrowing removes every above-floor feature, the clause is omitted, not kept empty (an empty clause is refused as unknown-axis by a host that does not observe the axis), and the `Narrowing` record travels on the artifact's Produce instead. Constraint from the A1 owner: dispatch evidence never adds an `isa` clause; it removes the above-floor features it covers from the extracted clause, and when nothing above the floor remains the clause is absent (spec §3.1 rule 4: an absent clause requires nothing).

Scope (ruled 2026-11-12, `sprints/2026-11-11-host-compatibility/rulings-2026-11-12.md`): the sequence C, then B, then A. Step 1, now: a narrow own-build rule. A device may run a build whose recorded requirement it does not meet only when every unmet clause is `isa.x86` with source `extracted`, the `Produce` was recorded by that same device, and the verdict is recorded as an own-build permit naming each unmet clause. It never applies to a sibling, and an illegal-instruction death of that artifact on that device withdraws it. Spec §3.2 and A3's verify are amended to state the rule before any code relies on it. Step 2 (baseline-compile attestation, the `tzcnt`/`bmi1` class) starts when K11's build receipt exists. Step 3 (the K10 note's minimum certificate slice) starts on a measured trigger: a second same-level device needs a dispatched build, the own-build rule has stood 30 days for one artifact with no certificate work begun, or the variant count becomes a held cost. K8 keeps counting the seven extras meanwhile.

Verify: ark's blake3 `*_avx2` paths do not raise its extracted ISA clause when their guard is evidenced; with the evidence withheld, the same bytes extract v3; epr's sqlite3 v3 code stays a requirement. For epr, exactly v3 is an outcome of complete coverage, not a preset target: narrowing removes a feature only when every occurrence of it is certified, and the remaining instructions then decide the level. Withheld evidence keeps all seven K8 extras and refuses against the synthetic v3 `Provides`.

Design input (read-only pass, 2026-11-11, `sprints/2026-11-07-artifact-store/evidence/k10-dispatch-evidence-design.md`): a function hash proves identity, not guarded reachability. The evidence is a certificate produced after the final link, bound to the whole artifact's CID, naming each guarded region, its detector, every incoming edge and the pointer or object invariants. `put` checks it and rescans every occurrence, and another device re-checks it without running the code. The scope is wider than ring and compiler_builtins: `sha_ni` also comes from two `sha2` versions. The three real mechanisms (ring's capability tokens and key variants, compiler_builtins' cached function pointer, blake3's public platform enum) each defeat a branch-dominance check, so the first slice covers one ELF and link configuration and these named contracts only. Unsupported or unresolved paths keep the requirement.

Second-device receipt (shem, 2026-11-12, `k8-extraction-shem-built.json`): an epr built natively on shem extracts the identical Requires CID, but for other causes. Its sqlite is v2 C, and the v3 level comes only from dispatched code (chacha20's avx2 backend, ring, sha2, compiler_builtins). shem's own Sandy Bridge CPU (no avx2, bmi, fma, adx or sha_ni) runs this epr, yet the recorded clause refuses it: the motivating case for K10. A second narrowing class comes from the same receipt. 1,277 of 1,281 `bmi1` sites are `tzcnt`, which LLVM emits in baseline code as `rep bsf`, and which runs as BSF without BMI1. Unlike ENDBR it is not a pure NOP: the two differ on a zero input, and code compiled with bmi1 may rely on the zero result. So K8 keeps counting it. K10 removes it only on evidence that the containing code was compiled at baseline (a build attestation naming the code's target features); without that, a v2-targeted build would be refused on v2 hosts for bmi1.

- [ ] **K11 — `epr artifact describe|put` generalised beyond the mesh's cargo binaries: a fresh lvi workspace with only the installed `epr` can record a cargo project of its own, and a recipe kind that cannot yet satisfy spec §7.9 refuses with the missing items named, rather than recording a thinner artifact** (serves workspace-device-arrival)

The steward's uniform rule (2026-11-12) classifies every §7.9 item as present, a declared gap, or a missing fact that refuses every recipe together before any write; no mesh recognition or identity exemption exists. Four declared gaps are allowed when applicable: `reach` (K12, pending H12), `build-inputs.build-scripts` (build-script/proc-macro inputs remain unenumerated), `build-inputs.dependency-sources` (registry checksums pin dependencies but source trees remain outside the captured view), and `build-inputs.compiler-inputs-unattested` (every Cargo recipe: edited dep-info cannot be detected without a build receipt). K6's pinned build closure must close build-scripts and dependency-sources; compiler-inputs-unattested closes only when put runs the build or an ark task records a build receipt binding compiler inputs, closure key and output bytes. Dep-info naming another target or older than the binary remains a missing fact that refuses, naming both paths. Describe/put prints each applicable gap as pending, not enforced while Produce records one sorted `env:gaps` slot. Committed locks and source commits, dependency pins, captured path/compiler inputs, resolved link inputs, attribution, supported artifact/target/narrowing, and writable destinations remain required for ours and foreign projects alike; recording orders destination preflight, content-addressed documents (reported harmless orphans on failure), Brit attestation, then Produce LAST. Neither the flow log nor Brit notes are ever rolled back. A failed Produce append reports `attested, not produced: re-run put with the same arguments to complete`; retries reuse a verified matching attestation from this signer and append Produce once. The shared recorder adds `reach` for every recipe, including sysroot put, whose banner also names it.

Verify: a fresh workspace records a small cargo binary end to end; a non-cargo recipe refuses naming each missing §7.9 item.

- [ ] **K12 — `env:reach` on every `Produce`, derived as the narrowest input reach (`epr::Reach`), and the peer gate serving when the requester's relationship meets it, with `berth.artifacts.serves` as a narrowing list (spec §7.8). H12 ruled 2026-11-12 (option B, `sprints/2026-11-11-host-compatibility/rulings-2026-11-12.md`); K12 may start** (serves dataplane-convergence)

Seat (2026-11-14): the highest-capability seat the runtime offers designs and reviews this item (on the Claude runtime today, a subagent started with model `fable`); implementation may be handed down as packets. It is the one item in this plan where a mistake widens who can draw bytes.

Verify: with an unchanged `pool-policy.json`, an admitted device of the person draws a `self` floor (an unattributed floor is `self`) and a `self` mesh build without a new grant row; a `private` artifact, or one with an input the derivation cannot classify, refuses; a record with no `env:reach` slot is served only by a `serves` row naming its method, as today; a `serves` row whose audience excludes the requester denies even when reach allows; nothing above `self` is served under H12.

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

- [ ] **V6 — contribution to a binary family as an attributed, valued event (spec §7 framing): an adapted variant, a packed floor or a replication of a family's leaves is recorded as a `Commitment` naming the collective (an ordinary, network-type `Collective`; nothing binary-specific), reusing the commons-pool-as-collective-party shape (no new entry type), with REA `EconomicEvent`s when others draw on it. BLOCKED on the commons-pool slice (held, no habit) and on elohim-storage's per-object custody and collection health (`arch-dataplane-refactor-backlog` row 23)** (serves dataplane-convergence)

Verify: an adapted variant appended to a family on one device is credited to its contributor; a second device's fetch of it records a draw against the collective; the family's health reads the collection-health fold from elohim-storage, not an artifact-store count.

## Lane P — placement, measure and rakia (spec §7.3, §10; rung H5; H10)

Chain: S1 → spec §10 → placement `admit`, `ark task run`, `measure.sh` → the measure-on-a-peer scenarios.

- [ ] **P1 — a compatibility stage in `elohim/placement/core/src/eligibility.rs` between the gate stage and the capacity stage, with its own `IneligibleReason`; the provider evaluates the task's variant set against its own `Provides`, kept beside `DeviceFacts` and not serialized** (serves measure-runs-on-a-peer)

Verify: placement tests refuse an incompatible variant with the new reason and admit the compatible one; consent and gate ordering unchanged. The task's `Requires` or variant set is read from its commitment (`CommitmentDraft`), not from a side channel, so what was promised includes what the work needs to run.

- [ ] **P2 — the same admission inside `ark task run` (`elohim/ark/cli/src/task.rs`, the carried-here step) and beside the executor resolution in `genesis/agentic/compute/measure.sh`** (serves measure-runs-on-a-peer)

Verify: `MEASURE_DRY_RUN=1 just measure <feature> --on jessica` prints the admitted variant or the refusal before launch. A completed run's fulfilment (`FulfilmentDraft`) carries the admitted variant CID, the providing device and its steward, the admission verdict and the measured cost beside the commitment's expected budget; a run on the steward's own device records the same, settled in kind. A death after admission is joined to that fulfilment by F3's witness. The commitment and fulfilment are written as `elohim/epr-rea` flow events into the store `epr flow` reads, on the plane `RecordPlane` chooses, so `epr flow report` shows them beside the artifact store's `Produce` events; as of 2026-11-12 the placement drafts have no caller outside `elohim/placement/core` and reach no store.

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

- [ ] **X3 — the one-click devspace as a consumer of these floor checks (steward's vision, 2026-11-12; a later plan, with lvi)** (serves workspace-device-arrival)

A person follows an EPR link, from a repository explorer or from a running client app (as one would inspect an element), into a devspace focused on the source of one component, governed by that directory's `.epr-meta`. Its dependencies arrive as recorded artifacts from the peer mesh, not as checkouts. Before anything starts, the space says what the component is and what this device can do with it: whether it can build and run here and how (`admit` over the recorded `Requires`), and which attestations this device could produce. A device that cannot carry it, such as a thin client, is offered a workspace started on another steward's device with the capacity (the person's own or a friend's) and carried on over the web, under a commitment accounted by the ruling above. An agent in that context shows how a proposed change flows into the build and its queue, and which queues the device could serve while idle. The aim is that development, artifacts and integration stay legible to the people who explore and join, at a thousand agents across a hundred machines.

What it reuses from this plan, so that none is built twice: `Requires`, `Provides` and `admit` (lane A); the method and input records and family lookup (lane K); variant sets and conformance (lane V); placement eligibility and the commitment and fulfilment (lane P); typed refusals as feedback (lane F). What it needs that this plan does not hold: the devspace seed and arrival (`elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md`), a source tree composed from artifacts, and the link and explorer surfaces.

Home (2026-11-12): `elohim/lvi/docs/specs/2026-11-12-view-source-anywhere-one-click-devspace.md`, listed as horizon item H6 in `elohim/lvi/docs/plans/2026-07-20-lvi-devspace-roadmap.md`.

Verify: this item has a named home in an lvi spec or a backlog entry before this plan closes, and that home cites the lanes above.

## Lane R — the reconciliation walk (spec §7.6; appended 2026-11-10)

Appended after lane X so earlier gap ids stay stable. It runs after F3 and V1, beside V and P.

Chain: S4 → spec §7.6 (path, floor, ceiling) → the walk recipe, the berth and bin store, the
update offer → one walk taken end to end by an agent.

- [ ] **S4 — a story for the update that cannot run here: a device is offered a release one of whose components needs instructions its CPU lacks; it is not offered that variant, stays on what it runs, is told why in plain words, and is shown its path (a compatible variant, a local build, or waiting for one)** (serves workspace-device-arrival)

Verify: `@concern:workspace-device-arrival`; one fresh blind reader returns READY with no correctness finding.

- [ ] **V5 — probation before adoption: a new variant becomes current only after a health window; a death in the window restores the previous variant automatically, records the witness, and withdraws the variant's claim for that host class** (serves dataplane-convergence)

Verify: under emulation, a v3 variant adopted on a v2 host dies with an illegal instruction, the previous variant is running again with no person acting, and the witness names the variant and the ISA clause.

- [ ] **R1 — the update offer filters by admission: an update is presented only when one of its variants is admitted on this device; otherwise the person is told a newer version exists that this device cannot run yet, and why** (serves workspace-device-arrival)

Verify: the S4 scenario passes on the v2 device; the same release is offered normally on the v3 device.

- [ ] **R2 — the walk as a recipe: propose a `host-reconciliation` process specification for `.claude/epr-meta/recipes.yaml` with one stage per step of spec §7.6 (exception, restore, pain, finding, variant, proof, reach) and the floor checks as edge validators; the registry's steward rules before it lands** (serves dataplane-convergence)

Verify: the proposed recipe is in the item's report with each validator named against an existing check or a gap item in this plan; the steward's ruling is recorded as a note on this plan.

- [ ] **R3 — dogfood one walk: an agent takes a real failure (the v3-on-v2 case from K6 or V5) through every step of the recipe, recording each step as a `Process` under it, and stops where a step's gap is still open, naming it** (serves dataplane-convergence)

Verify: `epr flow ledger` on the walk's first record shows the chain of steps taken; the report lists the step where it stopped and the gap item that blocks it.

## Lane C — channels (spec §7.7; appended 2026-11-10)

Appended after lane R so earlier gap ids stay stable. It runs after V1. Open discovery and
advertisement are held back by the steward's ruling (spec §7.7, "Held back by design"): this lane
covers channels a device follows and what the steward's own devices can read, and C4's commons
listing is named only, not built.

Chain: S5 → spec §7.7 → storage's release channel, ark's `ArtifactRef::Channel` and `follows`, the
steward-space head move → a repair reused by a second device.

- [ ] **S5 — a story for reuse through a channel: one device's elohim repairs a variant and publishes it on a repair channel at the steward's reach; a second device of the same class follows that channel and adopts the repair after probation; a third device of another class follows it and is told nothing there fits it** (serves dataplane-convergence)

Verify: `@concern:dataplane-convergence`; one fresh blind reader returns READY with no correctness finding.

- [ ] **S6 — the next scenarios in the unseen-device arc, extending S3's `devflow/unseen-device-builds-its-variant.feature` (or a new file that cites it, so the story stays in one place), for the binary as a collective (spec §7 framing): a person whose device class nobody has seen runs the family's portable variant on day one, contributes an adaptation for their hardware, and later sees that adaptation become what other devices of their class rely on, with the contribution credited to them** (serves dataplane-convergence)

Verify: `@concern:dataplane-convergence`; its author runs the a2o blind-reader loop (one fresh reader returns READY with no correctness finding; S3's run does not carry over); the story speaks of a generic collective and names no artifact-store-specific mechanism the framing table does not map to an existing EPR concept.

- [ ] **C1 — a channel head names a variant-set CID: resolve ark's `ArtifactRef::Channel` through the device's declared `follows`, reusing storage's release-channel id rule (`elohim/elohim-storage/src/release_channel.rs`) and the steward-space compare-and-swap head move** (serves dataplane-convergence)

Verify: one channel head resolves to different admitted variants on the two devices; a stale head move is refused and the second writer re-reads and appends.

- [ ] **C2 — creating a channel records its steward, reach (private by default), family and suite; a repair channel is opened by the R3 walk for its adapted variant** (serves dataplane-convergence)

Verify: the R3 walk's report names the repair channel and its record; the channel is not readable outside its reach.

- [ ] **C3 — selection over the union of a device's followed channels for one family, preferring the release channel unless the person or the elohim chose the repair; switching follows passes probation** (serves workspace-device-arrival)

Verify: the S5 scenario's second device adopts the repair after its health window; the third device reports that nothing fits.

- [ ] **C4 — channel discovery inside the steward's devices (channel id, family, suite; not followers), and the commons-reach listing named as a missing node with the commons rung of spec §7.2** (serves dataplane-convergence)

Verify: an elohim on one device lists the channels carrying a family from the steward space; the missing node for commons reach is recorded in the spec or the backlog.

## Lane G and late items (appended 2026-11-14)

Appended after lane C so earlier gap ids stay stable. Lane G is the gate-verdict half of the
ruling "A sibling's gate verdict may stand in for a local run": the floor is built, and these
items prove it and name what it does not see. A5 and K13 belong to lanes A and K and sit here
only because items are appended, not inserted. Order: K13, then A4 part 2 and A5 together; G1
needs no code and can run on the next landing.

- [ ] **K13 — `epr artifact requires --output <cid>`: a read-only verb that returns a recorded output's requirement (the `env:requires` CID and the document bytes) and whether this device recorded the `Produce`, checked against this device's key and not taken from a flag; it is the missing node A4 part 2 names** (serves dataplane-convergence)

Rules (steering verdict, 2026-11-14): it reads this device's own store and never the network.
Several `Produce` events for one output are merged through `Requires::try_from`, so a conflict
is a `MergeError` and never a silent choice; a `Produce` with no `env:requires` slot is ignored
when another carries one (shem's v2 floor has one of each, journal "The floor"). An output with
no slot at all answers `unmeasured` with exit 0 and a distinct line, so the preflight can print
it without treating it as a refusal. The `self` answer is the only source A5 accepts for an
own-build claim.

Verify: on both devices the verb returns the mesh storage binary's requirement and `self: true`; for a sibling's fetched output it returns `self: false`; an output with no slot prints `unmeasured`.

- [ ] **A5 — the admission effect and the own-build claim are bound by construction before any running path consumes them: `authenticate_candidate` re-checks that the verdict's subject is the executable, that its checks cover `arch` and `os`, and that an own-build waiver is present exactly when an own Produce is named; `CompatibilityEffect` is no longer deserializable (a record carries its recordable projection, and the effect is rebuilt by admission); `OwnProduce` is constructed only from K13's verified answer; `Provides` loses its derived `Debug`** (serves workspace-device-arrival)

ark-core stays free of the contract crate (240693e34): the re-checks read only ark-core's own
verdict type. The observed host is the supervisor's own process environment (its libc, its
mount namespace, the core the probe ran on); a child launched under another root is outside
what A2 observes, and that limit is stated in spec §4 with this item, not solved by it.

Verify: tests refuse a hand-built effect with empty checks, an effect whose subject differs from its executable, and a waiver with no own Produce; a sibling's Produce cannot produce an own-build permit; `{:?}` of a `Provides` does not compile or prints no observed value; `just gate elohim-ark` and `just gate host-contract` green.

- [ ] **G1 — a live two-device receipt of sibling gate reuse (the artifact-store sprint's owed F1r, K7): one device gates a clean commit, the other pushes the same commit with `GATE_EVIDENCE=reuse` and its log prints `reused … independence: sibling` for at least one cargo project** (serves sprint-lands-each-closure-once)

No code is owed. Both peer daemons must run an epr and a `pool-policy.json` that cross
`gate-verdict@1`, and the commit must not touch the governance paths
(`SIBLING_REUSE_GOVERNANCE_PATHS`), which always run locally. A miss is as useful as a hit when
its `why` line is kept: it names the next floor check.

Verify: the push log line and the `run-kind=sibling-reused` cycle row, linked from the habit `push-gate-pays-once` as a DELTA; the seconds the landing lane was held, beside the previous landing's.

- [ ] **G2 — the gate verdict names what the closure key does not see: a `gate-verdict@1` row carries the recording device's coarse host class (proven bundle names only, H7), and the reuse line prints it beside the reusing device's own** (serves sprint-lands-each-closure-once)

The key holds the source closure and `rustc -vV`. It does not hold the C compiler, the link
floor or the instruction set, and the two devices differ in all three (K9 receipt). Verdict
(2026-11-14): this does not gate G1 or daily use between the steward's own devices. A green
that a host difference would have turned red is caught by CI on dev, and the device that
accepted it repairs. G2 makes the difference visible on the line; a project is taken off sibling
reuse (a per-project `sibling: false` in `gate_reuse`) only after a measured disagreement,
never in advance.

Verify: a reuse line showing both classes; a test that a row with no class still reuses and prints `class unmeasured`.

- [ ] **I1 — the record shapes lanes V and P read, as interfaces only: the `VariantSet` document (V1), the recordable admission verdict (A3, A5), and the commitment and fulfilment as `elohim/epr-rea` flow events on the plane `RecordPlane` chooses (P2); types, seam-registry rows and vector tests, no behaviour** (serves sprint-lands-each-closure-once)

Seat: the highest-capability seat the runtime offers (on the Claude runtime today, a subagent
started with model `fable`), as one seat holding all three shapes. V1 to V4, P1, P2, F3, R1 and
C1 read them across ark-core, the eprfs fetch path, placement-core and epr-rea, so a shape
chosen inside one lane is paid for by the others. The coordinating session (any model) starts
this seat when A5 and K13 have landed and K3's fetch by family is on dev, gives it this plan,
the spec and the rulings file, and afterwards hands V and P out as packets against the landed
types. Each shape passes the `p2p-design-gate` skill before it is proposed. ark-core and
placement-core do not gain the contract crate (240693e34).

Verify: the three types land with vector tests and seam-registry rows; V1, P1 and P2's briefs cite them by path; no behaviour change in `just gate elohim-ark`, `eprfs` or `elohim-compute`.

