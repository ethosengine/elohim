---
name: elohim-package-authoring
description: Author and maintain Elohim-native skills and agents from .epr-meta/elohim/packages, treating Claude and Codex files as generated projections. Use when creating or updating Elohim-native skills or agents, or when checking projection drift between .epr-meta/elohim/packages and Claude/Codex runtime surfaces.
metadata:
  runtime: codex
  sourceRuntime: elohim-agent
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/elohim-package-authoring.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/elohim-package-authoring"
---

# Elohim Package Authoring

Use `.epr-meta/elohim/packages` as the canonical authoring surface for Elohim-native skills and agents.

## Rules

- Edit package JSON first.
- Treat `.claude/*` and `.codex/*` as runtime projections.
- Run the package projection check after changing packages.
- Do not require a Claude source file for packages whose `metadata.sourceRuntime` is `elohim-agent`.

## Workflow

1. Author or edit the package under `.epr-meta/elohim/packages`.
2. Regenerate projection fixtures with the package projection CLI.
3. Generate runtime surfaces only when the repo intentionally wants local Claude/Codex files refreshed.
4. Verify projection drift before committing.

## Directionality contract

- **Import (Claude→package) is certified byte-lossless.** The standing fidelity gate (`verifySourceFidelity` in `package-projections.mjs`, run by `elohim-agent:packages:verify`) asserts `project(import(source)) === source`, byte-for-byte, for every Claude-sourced skill and agent. Everything below stands on that floor.
- **For an un-planted Claude-sourced package, `.claude/*` and `.codex/*` are the authored source.** When such a package (`metadata.sourceRuntime: "claude"` with no `metadata.master`) has not been planted, the runtime files under `.claude/` are what a human or Claude edits directly. The package under `.epr-meta/elohim/packages/` is a certified mirror of that source AND the governance home — it carries an `EprRef` governance backref (`metadata.governance.eprRef`) that the Claude/Codex source files do not. Planting (next bullet) flips this.
- **The package-master flip is available, per package (2026-07-10).** The `plant-eprfs-*` skill family (`plant-eprfs-skill`, `-agent`, `-hook`, `-agentdoc`) plants a runtime-authored artifact: the package keeps `metadata.sourceRuntime` (origin preserved) and gains `metadata.master: "package"` (authority rooted in the package). Once planted, the package `instructions.body` is the source of truth and `.claude`/`.codex` are generated-and-clobbered projections — editing them directly is drift. The fidelity gate makes planting lossless: because `project(import(source)) === source` byte-for-byte, planting changes *authority*, not *content*. Both disciplines coexist in one tree: edit the package JSON for a planted (`master: "package"`) or native package; edit `.claude`/`.codex` for an un-planted Claude-sourced package. Adoption is one-at-a-time and operator-gated — plant one, prove it green, then the next.
- **Native packages are package-first.** A package with `metadata.sourceRuntime: "elohim-agent"` (this skill included) has no Claude source file — its package JSON `instructions.body` IS the source of truth, and `.claude`/`.codex` projections are generated from it. Its governance block (`metadata.governance`) must be set by hand in the package JSON: the `governanceFor()` helper in `package-projections.mjs` only threads through the Claude-import path (`skillPackageFromClaude`/`agentPackageFromClaude`) and never runs for native packages. Future native-package authors: write the `governance.eprRef`/`policy`/`gates`/`ledger` block yourself — nothing generates it for you.
- **The governance `EprRef` is the offline floor anchor.** It proves local fidelity and existence, gated by `epr-meta-resolver` + `elohim-agent:packages:verify`. Its ceiling — deep trust and REA value-flow reconciliation across peers — resolves through the p2p substrate, not this local tooling.
