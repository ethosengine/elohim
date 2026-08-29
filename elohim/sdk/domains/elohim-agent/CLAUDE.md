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
Claude, Codex, and supported Antigravity runtime files are projections of
those elohim packages.

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
- No new core EPR kinds — the flip adds *authority*, not vocabulary.
- No runtime agent *services*. The behavioral-judge / conductor ceiling
  (peer audit of a running machine, standing hits, compute-cost discipline)
  resolves through the p2p substrate, not this local tooling.

The package/projection surface is package-first locally:

- `pnpm run elohim-agent:packages:test` verifies package schemas and projection drift.
- `pnpm run elohim-agent:packages:verify` runs the fidelity gate (`project(import(source)) === source`) plus the governance-backref check.
- `pnpm run elohim-agent:packages:write` refreshes imported Claude packages and all projection fixtures.
- `pnpm run elohim-agent:packages:import` imports configured legacy Claude sources.
- `pnpm run elohim-agent:packages:project` regenerates package-derived projection fixtures.
- `pnpm run elohim-agent:packages:runtime` writes every declared runtime
  projection. Skills currently target `.claude/skills`, `.codex/skills`, and
  Antigravity's documented `.agents/skills` workspace layout.

Native packages with `metadata.sourceRuntime: "elohim-agent"` do not need a
matching Claude source file. A skill package also owns the bytes of every
relative reference, script, or resource in its `assets` collection; an
entrypoint without those assets is not a complete projection. eprfs
materializes package trees and records their composition: the `eprfs-agent`
binary (built from `elohim/eprfs`) walks a package tree into content-addressed
blobs, materializes entrypoints and skill assets, and emits the CID-attributed
derivation graph (`eprfs-agent compose-graph`). CID is
single-sourced there — `eprfs_core::BlobCid::compute` — never recompute one in
JS/Python (parity hazard). This tooling also uses eprfs for `.epr-meta/manifest.md`
resolution. (Publishing `eprfs-agent` to Nexus and baking it into the dev image
is a CI follow-up; until then, build it locally from `elohim/eprfs`.)

## Directionality contract

- **Import (Claude→package) is certified byte-lossless.** The standing
  fidelity gate — `verifySourceFidelity` in `scripts/package-projections.mjs`,
  run by `elohim-agent:packages:verify` — asserts
  `project(import(source)) === source`, byte-for-byte, for every Claude-sourced
  skill and agent. This is the floor everything below stands on.
- **For an un-planted Claude-sourced package, `.claude/*` is the authored
  source.** When such a package
  (`metadata.sourceRuntime: "claude"` with no `metadata.master`) has not
  been planted, the runtime files under `.claude/` are what a human or
  Claude edits directly. The package under `.epr-meta/elohim/packages/` is
  a certified mirror of that source AND the governance home — it carries an
  `EprRef` governance backref (`metadata.governance.eprRef`) that the
  runtime source file does not. `.codex/*` and `.agents/*` are generated
  mirrors. Planting (next bullet) flips this.
- **The package-master flip is available, per package (2026-07-10).** The
  `plant-eprfs-*` skill family (`plant-eprfs-skill`, `-agent`, `-hook`,
  `-agentdoc`, `-command`) plants a runtime-authored artifact: the package keeps
  `metadata.sourceRuntime` (origin preserved) and gains
  `metadata.master: "package"` (authority rooted in the package). Once
  planted, the package body (`instructions.body` or verbatim `source.body`) is the source of truth and the
  declared `.claude`/`.codex`/`.agents` files are generated-and-clobbered projections —
  editing them directly is drift. The fidelity gate above is what makes
  planting lossless: because `project(import(source)) === source`
  byte-for-byte, planting changes *authority*, not *content*. The projector
  keys on the marker, so both disciplines coexist in one tree: **edit the
  package JSON for a planted (`master: "package"`) or native package; edit
  `.claude` for an un-planted Claude-sourced package.** Adoption is
  one-at-a-time and operator-gated — plant one artifact, prove it green
  (`elohim-agent:packages:verify` + `eprfs-agent compose-graph`), then the
  next.
- **Native packages are package-first.** A package with
  `metadata.sourceRuntime: "elohim-agent"` has no Claude source file — its
  package JSON `instructions.body` IS the source of truth, and every declared
  runtime projection is generated from it. Its governance block
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

## Antigravity support boundary

Runtime support follows a documented ABI, not a directory-name resemblance.
Antigravity skills are supported because the installed 1.1.x customization
guide defines `.agents/skills/<name>/SKILL.md`; skill entrypoints and packaged
assets therefore project there with Antigravity-localized provenance.

AgentPackage, CommandPackage, HookPackage, and McpProfilePackage do not yet
declare Antigravity. The installed guide does not establish compatible
repository-backed agent or command files, hooks use a distinct payload and
decision contract, and project MCP configuration needs a verified mounting
strategy. Add each runtime target only with its documented ABI, adapter, and
round-trip verification; do not manufacture parity by copying a Claude or
Codex file into `.agents`.
