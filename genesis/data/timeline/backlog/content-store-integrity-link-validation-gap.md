---
id: "backlog-content-store-integrity-link-validation-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "content_store_integrity accepts every create-link unvalidated — IdToContent poison links are mintable by any compatible coordinator (hash-moving fix)"
slug: "content-store-integrity-link-validation-gap"
written: "2026-08-08"
author: "cartographer"
status: "backlog"
priority: "high"

relatedNodeIds:
  - "genesis/docs/superpowers/plans/2026-08-08-head-plane-trust-gradient-program-plan.md"

tags: [dna, integrity-zome, validation, security, content-store, hash-moving]
---

# content_store integrity link-validation gap

Surfaced by the operator's batch-extern contract review (2026-08-08, during the
head-plane trust-gradient program T1/T3 window):

- `content_store_integrity` accepts every create-link operation without
  validating `IdToContent` base, target, or tag
  (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:4253`).
- Consequence 1: a compatible custom coordinator can link an ID to a
  non-Content action. The election machinery succeeds, then
  `build_content_head_output` deterministically errors on the record shape
  (`content_store/src/lib.rs:3461`) — persistent per-id poison, not transient
  backpressure.
- Consequence 2 (worse): linking an ID to VALID Content belonging to a
  DIFFERENT id did not error at all — the resolver labeled the answer with the
  requested id, a silent wrong-content answer. The coordinator-side
  `content.id == requested_id` check closes the silent path NOW (landed with
  the T1 contract revision, coordinator-only); the link-level validation gap
  remains.

## Shape of the fix (why it is parked, not done now)

Integrity-zome validation of `IdToContent` links (base derivation, target type,
tag shape) is a **hash-moving DNA-lineage change**: it changes integrity code,
mints a new DNA hash, and therefore rides the full
`ALLOW_DNA_REINSTALL`/migration calculus — the alpha genesis pair must move
together or the fleet partitions into different DHTs (CLAUDE.md "DNA changes
don't redeploy by default"). It must land as its own planned lineage step, not
inside a coordinator-only sprint.

Until then the trust posture is: coordinator-side re-derivation (the
`content.id == requested_id` refusal, C5 evidence-not-authority) is the
enforced boundary; the DHT link layer is witness-only for this link class.

shift_objective: design + land IdToContent link validation in
content_store_integrity (base/target/tag), sequenced as a DNA-lineage change
with the genesis-pair migration plan; extend sweettest with poison-link
scenarios proving both consequences above are refused at validation time.
