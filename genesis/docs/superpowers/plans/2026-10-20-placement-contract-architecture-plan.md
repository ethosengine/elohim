---
title: The placement contract's architecture — lanes for the interfaces, the schemas, eprfs slice 1 and the stories
id: placement-contract-architecture-plan
status: Active
class: architecture
date: 2026-10-20
steward: human:matthew
serves: [materialize-what-is-read]
cites:
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:e1436ef4504566d1 | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
  - elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md
  - elohim/eprfs/eprfs-core/.epr-meta
  - elohim/eprfs/eprfs-storage/.epr-meta
  - elohim/eprfs/eprfs-local/.epr-meta
  - genesis/a2o/.epr-meta
  - .claude/skills/valueflow-authoring/SKILL.md
---

# The placement contract's architecture

The design is `what-every-app-inherits-placement-contract-design` (the spec). This plan is only its
lanes: each `- [ ]` item below is a gap item that `epr flow project` mints, that one seat claims, and
that closes with a report and its verify evidence. Nothing here restates the spec; a seat reads it.

Operator rulings in force (2026-10-20):

- **Architecture before landing.** Clean architecture, declarative artifacts, interfaces and seam
  boundaries, composed so the whole can be implemented without guesswork. No fetch, branch, commit or
  push until the operator asks to land.
- **Order of layers**: eprfs foundations, then commons compute (ark and berth enabling brit, rakia and
  lvi), brit as the first consumer, the runtime last.
- **Deterministic floor, elohim ceiling** for placement (spec §7).
- **The identity root is held by the network**, never by one device (spec ruling 6).
- **Stories**: all of spec §16, each under the full blind-reader loop of `genesis/a2o/.epr-meta` (no
  variance granted).

Write-set (session covenant): specs, plans, architecture docs, habit atoms, `elohim/eprfs/**`,
`elohim/sdk/schemas/v1/**`, `genesis/a2o/features/**`, and the habit projection. Every other path a lane
finds it needs is named in its report as a follow-on, never written.

## Lanes and order

Lane 0 runs first. Lane I runs next, because every other lane builds against the interfaces it fixes.
Lanes E, S and T then run in parallel on disjoint write-sets: E owns `elohim/eprfs/**`, S owns
`elohim/sdk/schemas/v1/**`, T owns `genesis/a2o/features/**`. Lane H closes the plan.

## Lane 0 — Settle the spec

- [ ] **P0 — apply the verified claim corrections, the coherence findings and the design-lens findings to the spec** (serves materialize-what-is-read)

Verify: every correction upheld by its skeptics is applied; `epr flow cites verify` passes on the spec.

## Lane B — The baseline

- [ ] **B0 — the before: bytes and wall-clock of a clone and a worktree setup of this monorepo today, and the actual read set of a typical agent session, recorded so every later slice states its gain against it** (serves materialize-what-is-read)

Verify: the numbers and the commands that produced them are in the item's report; nothing is landed.

## Lane I — The composed interfaces

- [ ] **I1 — interfaces document: the crate graph and dependency direction, every trait and type signature with its contract, the schema shapes, the TypeScript SDK surface, the extension points, and one worked example per seam** (serves materialize-what-is-read)
- [ ] **I2 — adversarial seam check: each interface tested against the existing code that must implement or adapt it (EprfsStorage implementors, PlacementStrategy, ark-core berth and intent, the app manifest, the service worker)** (serves materialize-what-is-read)

Verify: a judge panel's synthesis, then every seam survives its check or is revised; the document's
cites verify.

## Lane E — eprfs slice 1

- [ ] **E1 — View and the view-scoped walk over any EprfsStorage, bounded by TreeLimits on the visited spine, beside load_verified_tree** (serves materialize-what-is-read)
- [ ] **E2 — the three queries over a verified walk: read one path, list one directory, diff two tree roots under a view** (serves materialize-what-is-read)
- [ ] **E3 — tests: the counting-storage spine proof on a 10,000-entry tree under a three-path view, limit refusals, external boundaries, absent paths named; `just gate eprfs` green** (serves materialize-what-is-read)
- [ ] **E5 — the acceptance probe from the start: a tree root of this monorepo, sealed by brit, walked under an agent session's real read set, reporting nodes and bytes visited against the B0 baseline** (serves materialize-what-is-read)
- [ ] **E4 — eprfs seam governance: the eprfs-core .epr-meta teaches the view-walk seam; seam-registry rows for the new decision points; the habit moves from unwired to red with its check** (serves materialize-what-is-read)

Verify: `just gate eprfs` with `EXIT=$?` echoed on its own line; a reviewer seat checks reuse of
`tree.rs` verification and the IoC shape against Lane I.

## Lane S — The declarative artifacts

- [ ] **S1 — closure and view as SDK objects (a tree root plus path prefixes)** (serves materialize-what-is-read)
- [ ] **S2 — the manifest needs block of spec §6, optional and additive in the app manifest** (serves materialize-what-is-read)
- [ ] **S3 — the placement intents: a placement need, a berth offer and a posture** (serves materialize-what-is-read)
- [ ] **S4 — the placement parameter set: the ceiling's declared, versioned input to the floor** (serves materialize-what-is-read)
- [ ] **S5 — schema tests pass and codegen is fresh; every existing manifest still validates** (serves materialize-what-is-read)

Verify: `pnpm run schema:test` and the existing manifest validations pass; a reviewer seat checks each
schema against `elohim/sdk/schemas/v1/views/CONVENTIONS.md` and the p2p design gate.

## Lane T — The stories

One item per story group of spec §16. Each story is born `@wip`, with no step definitions, and closes
only when a fresh blind reader returns no correctness or interpretability finding.

- [ ] **T1 — bytes-layer stories** (serves materialize-what-is-read)
- [ ] **T2 — compute and placement stories** (serves materialize-what-is-read)
- [ ] **T3 — contract stories** (serves materialize-what-is-read)
- [ ] **T4 — live channel, attention and connectivity stories** (serves materialize-what-is-read)
- [ ] **T5 — identity, agent standing and inference stories** (serves materialize-what-is-read)
- [ ] **T6 — edge stories: reach, publication and bridges** (serves materialize-what-is-read)
- [ ] **T7 — lifecycle and proof stories** (serves materialize-what-is-read)

Verify: per story, the rounds and per-class finding counts recorded in the item's report, as the a2o
rule requires; a new feature subdirectory carries its own `.epr-meta` or a reviewed inheritance.

## Lane H — Ratchet

- [ ] **H1 — one evidence delta per habit served, then re-project the habit register** (serves materialize-what-is-read)

## Lane 0b — The design lenses and the absolutes (appended)

- [ ] **P0b — apply the three design-lens findings under rulings R1–R6, and make every absolute in the spec cite its enforcer or point at its missing node** (serves materialize-what-is-read)

## Lane C — The interfaces as compiling code (appended; operator grant 2026-10-20: interface code and the first adapters)

Each item builds what the interfaces spec (`placement-contract-interfaces-design`) names for its crate:
types, traits, rails and contract tests, and the adapters named in its reuse map. Order: C1, then C2 and
C4 in parallel, then C3 and C5; C6 runs beside them.

- [ ] **C1 — seam-contracts: ruling 7's `Unanswered<R>`, `PendingStatus` and status cursor, and the probe `Reading<T>`, with contract tests** (serves materialize-what-is-read)
- [ ] **C2 — elohim-compute's runtime feature split, then the new pure crate elohim/placement/core with every floor and ceiling interface, the built-in ranking, the ranking contract and rails R-P1 to R-P8** (serves materialize-what-is-read)
- [ ] **C3 — ark-core task.rs: the task spec, berth offer and task kind, under the existing purity rail** (serves materialize-what-is-read)
- [ ] **C4 — eprfs: the slice-1 renames, the defaulted EprfsStorage methods, tier, reach and view-declaration types, rails R-E1 to R-E3, the eprfs-serve crate with the tier contract harness, and the pure eprfs-ts package with shared vectors** (serves materialize-what-is-read)
- [ ] **C5 — elohim-storage adapters: the PlacementStrategy shim onto placement-core's ranking, the blob-store tier and root reach gate, and rail R-P8** (serves materialize-what-is-read)
- [ ] **C6 — TypeScript: verifyAddress in @elohim/epr, the HttpTier and placement status in storage-client-ts, and the service worker's CacheTier with rail R-T2** (serves materialize-what-is-read)
