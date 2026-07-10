# Elohim Agent Domain

This domain declares the canonical agentic capability vocabulary for Elohim SDK
manifests and V1 elohim package/projection fixtures for skills and subagents.

## Source Of Truth

The canonical vocabulary source is the EPR/app-manifest vocabulary in this
directory:

- `manifest.json`
- `manifest/content-types/*.json`
- `schemas/*-metadata.schema.json`

Generic EPR `Manifest` atoms can carry these app-manifest artifact shapes.
Generic EPR `Agent` atoms can later point at or embody the executable agent
contracts declared here. This slice does not add new core EPR kinds.

The canonical local V1 package fixtures live under
`.epr-meta/elohim/packages/*` until EPR storage seeding/projection exists.
Claude and Codex runtime files are projections of those elohim packages.

## Projection Boundary

Runtime surfaces are future projections, not canonical sources:

- `.claude/*`
- `CLAUDE.md`
- `AGENTS.md`
- `.agents/*`
- `.codex/*`
- hook files
- plugin manifests and plugin folders

Those files may be generated from native package/projection artifacts, and they
may be scanned to report projection drift, but they do not own the vocabulary.

## V1 Non-Goals

- No changes to `elohim/epr/src/kind.rs`.
- No changes to `epr-kind.schema.json`.
- No changes to `manifest-epr.schema.json`.
- No `eprfs` materialization or Claude/Codex semantics.
- No runtime agent services.

The package/projection V1 is package-first locally:

- `pnpm run elohim-agent:packages:test` verifies package schemas and projection drift.
- `pnpm run elohim-agent:packages:write` refreshes imported Claude packages and all projection fixtures.
- `pnpm run elohim-agent:packages:import` imports configured legacy Claude sources.
- `pnpm run elohim-agent:packages:project` regenerates package-derived projection fixtures.
- `pnpm run elohim-agent:packages:runtime` writes `.claude/*` and `.codex/*` runtime projections.

Native packages with `metadata.sourceRuntime: "elohim-agent"` do not need a
matching Claude source file. eprfs materialization of package trees is deferred;
this slice uses eprfs only for `.epr-meta/manifest.md` resolution.

## Directionality contract

- **Import (Claude→package) is certified byte-lossless.** The standing
  fidelity gate — `verifySourceFidelity` in `scripts/package-projections.mjs`,
  run by `elohim-agent:packages:verify` — asserts
  `project(import(source)) === source`, byte-for-byte, for every Claude-sourced
  skill and agent. This is the floor everything below stands on.
- **`.claude/*` and `.codex/*` are the authored source today.** For
  Claude-sourced packages (`metadata.sourceRuntime: "claude"`), the runtime
  files under `.claude/` are what a human or Claude edits directly. Packages
  under `.epr-meta/elohim/packages/` are a certified mirror of that source
  AND the governance home — each package carries an `EprRef` governance
  backref (`metadata.governance.eprRef`) that the Claude/Codex source files
  do not.
- **The package-master flip is deferred.** Flipping direction so runtime
  files become generated-and-clobbered (packages the sole source, `.claude`/
  `.codex` regenerated on every change) requires both the fidelity gate above
  staying green AND governance sign-off before it ships. Until that flip,
  edit `.claude`/`.codex` for Claude-sourced packages; edit the package JSON
  for native ones.
- **Native packages are package-first.** A package with
  `metadata.sourceRuntime: "elohim-agent"` has no Claude source file — its
  package JSON `instructions.body` IS the source of truth, and `.claude`/
  `.codex` projections are generated from it. Its governance block
  (`metadata.governance`) must be set by hand in the package JSON: the
  `governanceFor()` helper in `scripts/package-projections.mjs` only threads
  through the Claude-import path (`skillPackageFromClaude`/
  `agentPackageFromClaude`) and never runs for native packages. Future
  native-package authors: write the `governance.eprRef`/`policy`/`gates`/
  `ledger` block yourself — nothing generates it for you.
- **The governance `EprRef` is the offline floor anchor.** It proves local
  fidelity and existence, gated by `epr-meta-resolver` +
  `elohim-agent:packages:verify`. Its ceiling — deep trust and REA
  value-flow reconciliation across peers — resolves through the p2p
  substrate, not this local tooling.
