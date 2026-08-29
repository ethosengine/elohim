---
id: "backlog-security-federation-record-signature-serialized-never-verified"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "record_signature is constructed or serialized in five places in doorway-service and verified in none — so a federation row's signed/unsigned distinction is provenance a caller cannot check, and any origin answering the endpoint can mint the signed-looking form"
slug: "security-federation-record-signature-serialized-never-verified"
written: "2026-08-29"
author: "opus (grounding pass on the Keep client control surface; the client-side half was found in code this session shipped, and fixed there)"
status: "backlog"
priority: "high"
area: "doorway/doorway-service"
domain: "protocol"
jobs: [elohim-edge]
relatedNodeIds:
  - "concern:auth-discovery"
cites:
  - genesis/data/timeline/backlog/security-unsigned-gossip-peer-becomes-jwt-trust-anchor.md
tags: [security, federation, provenance, signature, doorway, trust-label]
---

# A distinction the wire carries and nothing checks

`GET /api/v1/federation/doorways` serves rows of two provenances. Rows read from the
infrastructure DHT carry `record_signature`, `signing_key` and `record_serial`; the gossip
merge path sets all three to `None` (`doorway/doorway-service/src/routes/federation.rs:112-131`).
That difference is real and worth reading.

What is not real is any check on it. `record_signature` appears five times in the service —
the struct field at `routes/federation.rs:41`, two constructions at `:92` and `:130`, one
more at `:391`, and a test fixture at `projection/epr_router.rs:971` that fills it with
`vec![1; 64]` — and there is no verification site anywhere. Not a key lookup, not a
signature check, not a serial ordering. The bytes are carried and never examined.

So the three fields are a *shape*, not a proof. Any origin that answers
`/api/v1/federation/doorways` can produce the signed-looking form by filling three fields
with arbitrary bytes. The `vec![1; 64]` fixture is the demonstration: it would satisfy every
consumer that treats the quartet's presence as evidence.

## Why this is filed now

A client shipped this session read those three fields and labelled the result
`dht-notarized` — a word asserting a notarization check that never runs. That label is
corrected in place (`app/elohim-library/projects/elohim-service/src/keep/peer-register.ts`,
now `dht-sourced`, with the reasoning and a test pinning that arbitrary bytes earn the label).
The client half is closed. This row is the server half, which is where the fix belongs.

It matters beyond a name. The design that produced the label also proposed gating content
candidates on it — an admit-list keyed on a value the admitting party cannot verify. That is
the gossip-adoption the substrate trust contract's I1 forbids, and it is an easy thing to
reach for precisely because the field is *named* like a proof. The rule was deleted before it
shipped; the next author will find the same three fields and the same temptation.

The distinction between SELECTION and ATTRIBUTION is already adjudicated in this repo, in
`genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md`: selection is safe on
an unverified binding, attribution is not. The same line applies here. Ordering a fallback by
provenance is fine. Granting authority by it is not.

## The fix

Serve the distinction as something a client can read without inferring it. Add a
`provenance` field to `DoorwaySummary` — set to the read path that produced the row, at the
point where that path is known — with `#[derive(TS)]` so the type crosses the boundary
generated rather than hand-copied. Then either verify `record_signature` at ingest or stop
serializing it, because a signature field that is never checked is worse than no field: it
invites exactly the reading it cannot support.

Note the snake_case fields crossing the wire here (`record_signature`, `signing_key`,
`record_serial`, `identity_root`) violate the repo's own boundary rule — snake_case never
leaves the Rust boundary. The `provenance` addition is the natural occasion to fix that too.

## Verified

- `grep -rn "record_signature" doorway/doorway-service/src --include=*.rs` — five hits,
  listed above; the same grep filtered for `verify|validate|check` returns zero.
- `routes/federation.rs:112-131` — the merge path setting all three to `None`.
- `projection/epr_router.rs:971` — `record_signature: vec![1; 64]`.
