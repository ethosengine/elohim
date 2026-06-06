---
id: "backlog-ci-lamad-attention-flow-null-contentid"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Lamad bundle build fails — AttentionFlow eprHref(string) called with EconomicEventView.contentId (string | null)"
slug: "ci-lamad-attention-flow-null-contentid"
written: "2026-06-06"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [4993e8352110]
jobs: [elohim]
relatedNodeIds: []
tags: [ci, elohim, lamad, angular, strict-templates, ng5, type-boundary]
cites:
  - https://jenkins.ethosengine.com/job/elohim/job/dev/1505/
  - app/lamad/src/app/components/attention-flow/attention-flow.component.ts
  - app/lamad/src/app/components/attention-flow/attention-flow.component.html
  - elohim/sdk/storage-client-ts/src/generated/EconomicEventView.ts
---

# Lamad bundle build fails on a null-unsafe template binding (AttentionFlowComponent)

## The failure

`elohim` build #1505, stage labeled **Build Lamad Bundle** (the failure is in the
`ng build` run at `app/lamad`; the harvester labels by the next stage marker):

```
✘ [ERROR] NG5: Argument of type 'string | null' is not assignable to parameter of type 'string'.
  Type 'null' is not assignable to type 'string'. [plugin angular-compiler]
    src/app/components/attention-flow/attention-flow.component.html:21:35:
      21 │         [attr.href]="eprHref(event.contentId)"
  Error occurs in the template of component AttentionFlowComponent.
 ELIFECYCLE  Command failed with exit code 1.
```

The build failed at this stage; all downstream stages (Unit Test, SonarQube,
Upload SPA Blob, Build Image, Deploy …) were "skipped due to earlier failure(s)"
and the pipeline ended FAILURE. Occurrence evidence: seen 1, first_build 1505,
last_build 1505 (job elohim).

## Verdict

**real — Angular strict-template type error** at the Rust-to-TypeScript boundary.
Not a flake, not infra. Strict templates are enabled (per root CLAUDE.md Code
Style: "strict TypeScript + Angular strict templates").

## Root cause

`AttentionFlowComponent.eprHref` was declared `eprHref(contentId: string): string`,
but the template calls it with `event.contentId`, and on the generated
`EconomicEventView` type, `contentId` is `string | null`
(`elohim/sdk/storage-client-ts/src/generated/EconomicEventView.ts` — ts-rs
generated, not hand-editable). Some economic events legitimately carry no content
reference (the component's own `ngOnInit` already `.filter(Boolean)`s contentId
for the unique-content count, acknowledging nulls). Passing `string | null` to a
`string` parameter is the NG5 violation.

This is the snake-case-never-leaves-Rust boundary working as intended: the wire
shape is honestly `string | null`, and the consumer must handle null — the bug was
the consumer narrowing too eagerly.

## Current decision

**Already fixed on `dev` (commit `9235e963d`), landed AFTER the harvest captured
build #1505 — awaiting CI disappearance confirmation.** The fix widened `eprHref`
to accept `string | null` and return `null` for a null/empty id. A null
`[attr.href]` binding removes the attribute (correct degradation: a content-less
event renders an `<a>` with no href rather than a broken link), so the template
binding type-checks and the runtime behavior is correct. Mirrors the
`node?.id ?? ''` null-guard pattern the same build flagged as a softer NG8107
warning in `sophia-renderer.component.ts`.

Build #1505 was built from `a2066688` (orchestrator-1169), which carried the buggy
`eprHref(contentId: string): string`; the corrective commit landed on `dev`
between the harvest and this triage. No new code change was needed from triage —
the ledger stamp (`triaged_at_build: 1505`) lets the sweep confirm by green streak
once a build > 1505 runs.

## Fix trail

- Fix commit (pre-existing on `dev`): `9235e963d` — *"fix(lamad): null-tolerant
  eprHref — AOT template type-check (CI elohim#1505 Build Lamad Bundle)"*.
- `app/lamad/src/app/components/attention-flow/attention-flow.component.ts` —
  `eprHref(contentId: string): string` → `eprHref(contentId: string | null): string | null`
  with an early `if (!contentId) return null;` guard.
- Verification (against HEAD = `9235e963d`): `pnpm run build` in `app/lamad` →
  "Application bundle generation complete" / "Output location:
  .../app/lamad/dist/lamad" with no NG5/ERROR lines (the exact CI command for the
  failing stage). At the failing build's commit `a2066688` the same build aborts
  at ~28s with the NG5 error.
- No integrator action needed for this concern beyond the next `elohim` pipeline
  run, which confirms by green streak.
