---
id: "backlog-deprecation-relationship-inference-source-retire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire RelationshipInferenceSource (hand-written enum drift vs generated InferenceSource)"
slug: "deprecation-relationship-inference-source-retire"
written: "2026-06-08"
author: "deprecation-triage"
status: "backlog"
priority: "medium"
deprecation_status: blocked
severity: low
fingerprints: ["546fb10112e3"]
relatedNodeIds: []
tags: [deprecation, typescript, angular, lamad, InferenceSource, enum-drift, content-graph]
cites:
  - app/lamad/src/app/models/content-node.model.ts
  - elohim/sdk/schemas/v1/enums/inference-source.schema.json
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
---

## What is deprecated

```
706: * @deprecated Hand-written drift vs the canonical generated `InferenceSource`
```

`RelationshipInferenceSource` in `app/lamad/src/app/models/content-node.model.ts:713`
is an intentional `@deprecated` marker — a hand-written legacy vocabulary that drifts
from the canonical generated `InferenceSource` enum. The marker is architectural
signalling placed during the content-graph work, not a surprise regression.

The two vocabularies disagree both in values AND in tier semantics:

| Legacy `RelationshipInferenceSource` (6 values) | Canonical `InferenceSource` (5 values) |
|---|---|
| `author` (explicitly defined by author) | `explicit` |
| `structural` (path/chapter structure) | `path` |
| — | `tag` (no legacy equivalent) |
| `semantic` (content similarity) | `semantic` |
| `usage` (user navigation patterns) | — (no canonical equivalent) |
| `citation` (extracted from links) | — (no canonical equivalent; closest is `explicit`) |
| `system` (inverse/system-generated) | `system` |

## Usage inventory

All usages are confined to a single file, `app/lamad/src/app/models/content-node.model.ts`:

- `:663` — `ContentRelationshipDetail.inferenceSource: RelationshipInferenceSource` (read model)
- `:704` — the `@deprecated` docblock
- `:713` — the `type RelationshipInferenceSource = …` definition
- `:755` — `ContentRelationshipDetailWire → ContentRelationshipDetail` adapter cast
  (`wire.inferenceSource as RelationshipInferenceSource`)
- `:775` — `RelationshipQuery.inferenceSource?` (query filter)
- `:789` — `CreateRelationshipInput.inferenceSource?` (write input)

No spec files reference the type. Blast radius: one model file, four typed surfaces.

## Migration path

Canonical truth chain (source of truth, not the hand-written type):
1. **DHT integrity** — `INFERENCE_SOURCES` constant in the `content_store_integrity`
   zome (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs`)
   notarizes the core 4: `explicit | path | tag | semantic`. Relationship integrity
   validation enforces these on-chain.
2. **Schema** — `elohim/sdk/schemas/v1/enums/inference-source.schema.json` agrees with
   the DHT constant and adds the storage-tier `system` value (projection-derived,
   never notarized).
3. **Generated TS** — `InferenceSource` flows to `*/generated/schema-enums` via
   `pnpm run schema:codegen:ts`. The deprecated type's docblock already directs new
   code to it.

Retirement = swap the four typed surfaces from `RelationshipInferenceSource` to the
generated `InferenceSource`, then delete the legacy type. The blocker is not the swap
itself but the **value reconciliation**: the wire adapter at `:755` currently casts
`wire.inferenceSource` (a backend-emitted string) straight to the legacy type. The
backend (`elohim-storage` `relationship_service.rs` / `graph_engine.rs`) must be
confirmed to emit canonical values (`explicit | path | …`), and the two legacy-only
values (`usage`, `citation`) must be mapped or proven dead before the cast can target
`InferenceSource` without silently widening the type past the DHT-enforced enum.

## Current decision

**Blocked.** This is a cross-vocabulary reconciliation across the DHT-integrity boundary,
not a bounded rename — it is the same class as the broader "Reach enum drift" (schema
enum ≠ Rust enum ≠ app vocabulary) reconciliation. A safe retirement must:
(1) confirm the `elohim-storage` relationship views emit only canonical
`InferenceSource` values; (2) decide the fate of the legacy-only `usage` and `citation`
values (map to `explicit`/`semantic`, or prove no producer); (3) swap the four typed
surfaces and re-run lamad type/codegen gates. That work spans the Rust storage layer
and the wire contract, exceeding the background-agent bounded-fix posture (it needs a
content-graph reconciliation slice, tracked as the model's "B-slice follow-up").

The `@deprecated` annotation is intentional and the `@typescript-eslint/no-deprecated`
warning is expected suppressible noise until the reconciliation slice runs. The sentinel
suppresses further dispatch on this fingerprint (ledger status: blocked).

## Verification

N/A — not yet fixed. Will be verified when the content-graph reconciliation slice swaps
the four surfaces to `InferenceSource`, deletes the legacy type, and the lamad
`pnpm test` + `schema:codegen:ts` freshness gates stay green.
