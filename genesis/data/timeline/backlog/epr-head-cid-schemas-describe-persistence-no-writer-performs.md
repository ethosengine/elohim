---
id: "backlog-epr-head-cid-schemas-describe-persistence-no-writer-performs"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Three schemas describe a persisted \"EprHead CID\" that no writer mints — harmless today, a migration the moment someone wires a real one"
slug: "epr-head-cid-schemas-describe-persistence-no-writer-performs"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 review (design-pass follow-up F6)"
status: "open"
priority: "low"
jobs: [elohim]
cluster: "arch-dataplane-refactor-backlog"
tags: [epr-head, content-addressing, schema-drift, mishpat, valueflows]
---

**The fact.** Three shipped view/commitment schemas describe a field as holding an "EprHead CID," but no
production writer ever mints a real dag-cbor head CID into any of them:

| Schema field | What actually gets written |
|---|---|
| `elohim/sdk/schemas/v1/economic-events/republish-epr.schema.json:35` (`target`), `:39` (`supersedes`) — *"CID of the new/previous EprHead"* | Persisted as a signed DHT EconomicEvent, but the validator (`elohim-storage/src/services/republish_epr_validator.rs:65-72`) treats `target_epr_id` as the canonical EPR **identity** (a slug), not a dag-cbor head cid |
| `elohim/sdk/schemas/v1/commitments/acknowledges-reach-change.schema.json:26-29` (`target_epr_cid`) — *"CID of the new EprHead"* | Persisted twice (a Mishpat DHT Commitment, projected into `mishpat_commitments.recipient`, `mishpat_projection.rs:1176-1200`); every fixture uses the placeholder literal `"bafy-new-epr-head-cid"` (`:1570`, `:1593`) — no production writer mints a real one |
| `elohim/sdk/schemas/v1/feedback-signals/reach-escalation-pending.schema.json:17` (`target`) — *"CID of the new EprHead"* | No producer wiring found at all |

The correct rule is already stated once, in a sibling schema: `elohim/sdk/schemas/v1/commitments/author-lens.schema.json:18`
— `governs_epr` is "the EPR SLUG-ID this lens governs... **NOT** the dag-cbor EprHead CID... binding on the CID
would line up with no existing scope row."

**Evidence.** `genesis/a2o/reports/recovery/serving-edge-20260920/epr-head-envelope-design.md` §3 (~line 289-306),
follow-up F6 (§7, ~line 680-686): "three schemas claim a persistence nobody performs... cheap now, a migration
after a writer exists." Verified directly: `republish-epr.schema.json:35-39`,
`acknowledges-reach-change.schema.json:26-29`, `reach-escalation-pending.schema.json:13-17`, and
`author-lens.schema.json:14-18` all match the design doc's quotes exactly.

**Why it matters.** Prose claiming a persistence nobody performs is dormant risk, not a live bug — every current
consumer of these three fields actually treats them as slug/identity strings (or, for `reach-escalation-pending`,
nothing consumes them yet). But the schemas are the contract a future writer will read: someone implementing a
real head-CID writer against `republish-epr` or `acknowledges-reach-change` today would be implementing exactly
the wrong thing, because the schema text says CID while every actual reader expects a slug.

**Smallest next step.** Propagate `author-lens.schema.json:18`'s sentence (slug-id, not dag-cbor CID) to the three
schemas above — a documentation-only fix, no code or data migration, because nothing currently produces or reads
a real CID in any of the three fields. Scoped separately from the EPR-head envelope slice because it touches
Mishpat and REA/ValueFlows schemas outside that slice's blast radius (design doc, C10 row, ~line 627).

**Links.** Design doc: `genesis/a2o/reports/recovery/serving-edge-20260920/epr-head-envelope-design.md` (§3, F6,
§7; C10 row). Sibling finding from the same reading: `epr-head-envelope-differs-per-peer.md`.
