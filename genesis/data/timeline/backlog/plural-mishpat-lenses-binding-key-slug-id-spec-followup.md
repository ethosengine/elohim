---
id: "backlog-plural-mishpat-lenses-binding-key-slug-id-spec-followup"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Plural-Mishpat-Lenses spec §5/§13: amend the Lens↔EPR binding key from dag-cbor CID to EPR slug-id (and note Wave-2 is hash-neutral)"
slug: "plural-mishpat-lenses-binding-key-slug-id-spec-followup"
written: "2026-06-27"
author: "plural-mishpat-lenses Wave-1 plan — interface-fit grounding (A3/A6)"
status: "backlog"
priority: "low"
domain: "D7"
jobs: [elohim]
relatedNodeIds:
  - "spec:plural-mishpat-lenses-over-epr-design"
  - "plan:plural-mishpat-lenses-wave1-plan"
---

## The follow-up

The spec `2026-06-27-plural-mishpat-lenses-over-epr-design.md` (§5 entity table, §13 open
questions) pins the **Lens↔EPR forward-index binding** on the **dag-cbor `EprHead` CID**
(`bafyrei…`). The Wave-1 plan's interface-fit grounding (A3) found that the *live* scope/bounds
machinery keys on the EPR **slug-id string** (`epr:lamad-spa`) everywhere — `bounds.epr_scope`,
`in_scope_of`, `find_active_delegates_compute`. A forward index on the dag-cbor entry_hash lines
up with **no existing scope row**.

**Correction (already adopted in the Wave-1 plan A3):** bind on the **slug-id**, reusing the SQL
scope projection. This also **collapses the spec's "lone DNA-move risk"** (§13): per the DNA
CLAUDE.md ("a link that exists only to serve a query belongs in the SQL projection"), the
forward-index LinkType is only forced if the index itself must be *notarized* — Wave-2 does not
require that, so **Waves 1 AND 2 are both hash-neutral**.

## The action

Amend the spec §5 (Lens↔EPR binding row: address = EPR slug-id scope key, not dag-cbor CID) and
§13 (drop "lone DNA-move risk" to "only if a notarized index is later required"). Low priority —
the plan already encodes the correct behavior; this keeps the canonical spec coherent with the
plan it spawned. Route through the cite tooling (managed surface).
