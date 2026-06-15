---
id: "backlog-seeder-validate-deployments-stale-validator-plus-human-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "validate:deployments fails 16 errors (committed, ungated): 13 are a STALE VALIDATOR (requires `manifest` for pattern=consolidated, but the documented convention is consolidated+template sed-render — only adam uses manifest); 3 are a REAL DATA GAP (caleb/daniel/emma deployment records reference humanIds absent from genesis/data/humans/humans.json). Found by the seeder shakeout; both fixes out of that shift's scope."
slug: "seeder-validate-deployments-stale-validator-plus-human-gap"
written: "2026-06-15"
author: "agentic-developer (shift doorway-crashloop-stabilize-then-seeder-shakeout, Phase 2)"
status: "backlog"
priority: "medium"
tags: [seeder, validate-deployments, stale-validator, deployments-json, humans-json, data-coherence, consolidated-pattern, sed-template, caleb-daniel-emma, tri-region-backup-chain, ungated-validator, out-of-shift-scope]
cites:
  - genesis/seeder/src/validate-deployments.ts
  - genesis/orchestrator/data/deployments.json
  - genesis/data/humans/humans.json
---

# `validate:deployments` — 16 errors: 13 stale-validator false-positives + 3 real human-gap

Surfaced by the seeder shakeout (Phase 2 of shift
`2026-06-15T03-46-doorway-crashloop-stabilize-then-seeder-shakeout`). The rest of the
seeder is healthy: seed JSON `schema:validate` 3431 valid / 0 errors, `schema:test` pass,
`schema:check-dna` pass, `holochain-seeder` unit tests 312 passed / 9 skipped / 0 failed.
`pnpm --filter holochain-seeder validate:all` fails **only** on `validate:deployments`
(exit 1, 16 errors). The files are clean/committed (not ambient WIP), so this is a
standing, reproducible failure — and it is **ungated** (CI/dev is not red on it, so
`validate:deployments` is not wired as a blocking gate).

## Error class 1 (13 errors) — STALE VALIDATOR; the data is correct

`validate-deployments.ts:129-135` requires a `manifest` field when `pattern === 'consolidated'`
(and `template` only for `pattern === 'legacy'`). But `deployments.json`'s own `$comment`
documents the *evolved* convention:

> "All humans now run the consolidated single-container pattern (elohim-node) … **Adam uses
> an explicit manifest file (the historical reference impl); everyone else sed-renders the
> template.**"

So every record except adam is correctly `pattern: consolidated` + `template:
…_edgenode-consolidated.template.yaml` (sed-rendered by the edge pipeline) — and the
pipeline works (alpha deploys from it). The validator's `legacy→template / consolidated→manifest`
mapping is stale relative to the convention the data + pipeline actually use.

**Bounded fix (out of the shift's scope.paths — genesis/seeder/src/ not in scope):** in the
`pattern === 'consolidated'` branch, accept EITHER `manifest` OR `template` (whichever is
present, verify the file exists); require one-of, not `manifest` specifically. ~5 lines.
Then wire `validate:deployments` into a pre-push/CI gate so it stays honest (it currently
catches nothing because it's both wrong AND ungated).

## Error class 2 (3 errors) — REAL data gap; protocol-law content

`caleb-spouse`, `daniel-brother`, `emma-spouse` have full deployment records (introduced
2026-05-22 for the tri-region reciprocal-backup chain: Dowell-SA ↔ Susan-Seattle ↔
Daniel-Tulsa, with caleb/emma as the second peer in the Susan/Daniel households) but their
`humanId`s are absent from the canonical `genesis/data/humans/humans.json`. Their household
siblings susan/eve/nancy (same 2026-05-22 batch) ARE in humans.json — so the addition was
partial. Net: `deployments.json` references 3 humans that don't exist canonically; the
deploy/seed/test paths that gate on deployments.json (a2o CLAUDE.md: it's the source of
truth for "is this human exercise-able") would try to act on non-existent humans.

**Fix (out of scope — genesis/data/humans/ not in scope.paths; humans.json is protocol-law
→ story-first):** EITHER add the 3 canonical human entries to humans.json (the bios already
exist in the deployment-record `$comment`s; story-first justification per humans.schema.json
being protocol-law), OR remove the 3 premature deployment records if the additions were
over-eager. This is an intent decision for the operator / a content-authoring pass, not a
mechanical fix.

## Why not fixed in-shift

Both fixes fall outside the shift's `objective.scope.paths`
(`genesis/seeder/src/validate-deployments.ts`, `genesis/data/humans/humans.json`), and the
humans.json change is protocol-law content (story-first). Captured here per the no-dump
discipline with the exact bounded fixes; a future shift or the operator can apply them.
