---
id: "backlog-codegen-prettier-pin-pre-emption"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Pin Prettier in schema:codegen:ts before attestation/observation TS regen"
slug: "codegen-prettier-pin-pre-emption"
written: "2026-05-14"
author: "cartographer"
status: "proposed"
priority: "low"
relatedNodeIds:
  - "memory:feedback_codegen_prettier_oscillation"
  - "memory:feedback_schema_first_ioc"
tags: [codegen, prettier, preventive, low-cost]
shift_objective: |
  Historian Wave 1 of memory ceremony Run #2 forecast: the codegen-Prettier oscillation
  will re-fire when attestation and observation TS interfaces regen (Reach/ContentFormat
  already drift idempotency-wise across 18 files; cosmetic-only today, but the same
  formatter behavior may produce real diffs when the underlying union types change).
  The proper fix (pin Prettier in codegen) was never landed; EPR Phase 3.5 T21 skipped
  the codegen freshness gate because of it.
  Outcome: pin Prettier version inside `elohim/sdk/schemas/scripts/codegen-ts.mjs`
  invocation OR add a deterministic-format post-step that's version-locked. Done when
  (a) running `pnpm run schema:codegen:ts` twice produces zero git diff for Reach +
  ContentFormat + the new attestation/observation views; (b) memory entry
  `feedback_codegen_prettier_oscillation` updated with the resolution.
---

# Codegen Prettier pin (pre-emption)

## Why this matters

Cheap fix today, sharp pain tomorrow. Attestation + observation TS regen will trigger
the oscillation; the carrier memory entry will then have to flip from "non-blocking"
to "blocking" mid-sprint. Pre-empting is single-shift work. Per historian Wave 1, the
specific re-fire risk is attestation Stage E's `0f826208f` storage-client-ts regen
and observation layer's `fb404e626`/`32b8d7d79` view additions.

## What's ready

- Carrier memory entry enumerates the exact symptom (Reach + ContentFormat across 18 files)
- `feedback_schema_first_ioc` gives the discipline framing (codegen scripts are part of
  the schema contract)

## Convergence

- Historian Wave 1: precedent 2 (forecast re-fire)
- Cartographer Wave 3: forward-lean only this agent sees → backed by Q-axis

## Definition of done

1. Codegen run is idempotent on Reach + ContentFormat
2. Memory entry updated with resolution
