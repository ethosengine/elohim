---
id: "backlog-codegen-verify-blind-to-stale-canonical"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "schema:codegen:ts --verify cannot detect a stale canonical — it compares consumers to disk, never disk to the schema"
slug: "codegen-verify-blind-to-stale-canonical"
written: "2026-08-11"
author: "claude (integrator, 2026-08-11 ghost-decay wave)"
status: "proposed"
priority: "high"
area: "elohim/sdk/schemas"
recurrence: 1
domain: "code"
cites:
  - elohim/sdk/schemas/scripts/codegen-ts.mjs
tags: [codegen, schema, pre-push-gate, false-green, verification, code-domain]
---

# `--verify` reports "up to date" for content codegen would never emit

## The defect

`codegen-ts.mjs --verify` skips generation entirely — Part 1 is wrapped in
`if (!VERIFY)`. It then populates `interfaceContents` by **reading the existing
canonical files off disk** (`OUTPUT_DIR`, i.e. `elohim/sdk/schemas/generated-ts/`)
and asserts only that each consumer copy equals what it just read.

So the verifier answers "do the consumers match the canonical on this disk?" It
never asks "does the canonical match the schema?" A stale canonical is therefore
self-certifying: it propagates to every consumer tree and reports `TypeScript
codegen is up to date.` indefinitely.

Compounding it: `elohim/sdk/schemas/generated-ts/` is **gitignored**. The
canonical is a local, per-machine artifact, so "the disk" the verifier trusts
differs between agents and CI, and nothing in review can see it.

## How it surfaced (2026-08-11)

The pre-push gate produced two verdicts on the same bytes:

- `schema-codegen: PASSED (4s)` — "TypeScript codegen is up to date."
- `elohim-library: FAILED` — 28 `prettier/prettier` errors across 18 generated
  files.

The disagreement was real, not a tooling skew: root and library Prettier are
both **3.8.1** with byte-identical configs (`printWidth: 100`), and *both* want
`export type Reach = …` (114 chars, 8 members) wrapped vertically. The committed
files carried the single-line form that neither wants.

`collapseUnionAliases` was innocent — its `singleLine.length <= printWidth`
guard correctly refuses to collapse at 114. A plain regen emitted the vertical
form immediately (117 files changed, `ed5972718`), after which BOTH gates pass.

This is worth separating from [feedback_codegen_prettier_oscillation]: that
memory describes a *cosmetic* near-boundary flip with no fixed point. This was
not that. The files were simply **stale**, and the freshness gate is structurally
incapable of saying so.

## Why it matters

A gate that cannot fail on the condition it exists to detect is worse than no
gate — it converts "unverified" into "verified," which is the exact false-green
shape the CI museum keeps re-recording. The next drift hides identically, and
only an unrelated lint gate in a different project will catch it, by accident.

## Repair direction

Make `--verify` generate into a temp dir and diff **schema → canonical** as well
as canonical → consumers. Two assertions, not one:

1. freshly-generated content == canonical on disk (currently absent)
2. canonical == each consumer copy (currently the only check)

Consider also tracking `generated-ts/` rather than gitignoring it, so the
canonical is reviewable and identical across agents and CI; if it stays ignored,
step 1 becomes the only thing standing between a stale local artifact and a
green gate.

Worth a regression test: mutate a canonical file, assert `--verify` exits
non-zero. Today it exits 0.
