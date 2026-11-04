---
id: "backlog-native-dev-rea-spend-as-events"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "REA spend as events: Work and deliver-service verbs, Use quantities for gate cost, deliverables as projections"
slug: "native-dev-rea-spend-as-events"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, rea, spend, projections, shefa, f5]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md).

## Item (spec §6, join F5)

1. **Verbs.** Verified: `ReaVerb` (`elohim/epr/src/witness.rs:315`) has Use, Consume, Produce, Cite, Affirm, Dismiss; no `work` or `deliver-service`. Add them (`Work` for carrying under `delegates-compute`).
2. **Gate cost.** Today it rides as `--env` slots on a `Cite` note; move to `Use` quantities (cpu-s, wall-s, bytes) against the producing `Produce` event.
3. **Deliverables are projections.** Changelogs, commit trailers, steward-of-record lines, habit DELTAs and valueflow contributions become unsigned recomputable projections over fulfilled commitments (seed: contribution projection under `epr flow memory`). Each hand-written record gets an owner and retire-when and is removed once its projection reproduces it.
4. Retire `.claude/data/*.jsonl` and berth ledgers as ad-hoc meters into events.

**Receipt.** A changelog reproduced from fulfilled commitments.

**retire-when.** No hand-written CHANGELOG or meter ledger remains alongside its projection.

## 2026-11-04 (adversarial review): split of scope between F1r/F2 and F5 (spec §6, §11 joins)

Minimal `Use`/`Produce` emission with quantities (cpu-s, wall-s, bytes) moves **into joins F1r and F2**, since the prize receipt (a sibling row with its `Use` event) and the fetch-vs-rebuild receipt need them. **F5 keeps** the `Work` verb (and `deliver-service`), the retirement of ad-hoc meters, and deliverables-as-projections. Items 2 and 4 above therefore start in F1r/F2 and finish in F5.
