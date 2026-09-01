---
id: "backlog-http-reach-enforcement-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "HTTP path serves intimate-reach content to any authenticated caller (reach gate is P2P-resolve-only)"
slug: "http-reach-enforcement-gap"
written: "2026-06-04"
author: "deliver-shakeout"
status: "refined"
priority: "high"
themes: [reach-enforcement, intimate-reach, doorway, privacy, epr-service]
relatedNodeIds:
  - "genesis/a2o/features/lms/intimate-reach-household.feature"
  - "genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md"
tags: [security, reach, household-testable]
---

# HTTP path serves intimate-reach content to any authenticated caller

Found 2026-06-04 while wiring step defs for `intimate-reach-household.feature` against a live
local stack. Two coupled gaps, both empirically proven:

1. **Reach gate is P2P-resolve-only.** `check_reach_authorization`'s intimate branch
   (`elohim/elohim-storage/src/epr_service.rs:413-439` — mutual + dual-consented relationship)
   is reached only via `handle_resolve` (`epr_service.rs:86`), which is invoked solely from the
   libp2p transport (`p2p/mod.rs:2656`) and iroh backend (`p2p_iroh/epr_backend.rs:62`) — never
   from an HTTP route. The HTTP paths apply a coarser gate: `GET /db/content/{id}`
   (`http.rs:3743-3756`) refuses anonymous but returns **200 to ANY authenticated caller**;
   `GET /epr-head/{id}` (`http.rs:6807`) is provenance-only (even anonymous gets 200).
   Proven against the provenanced `love-map-adam-eve` row: authed James AND Jessica both 200.
   Doorway's documented reach table (doorway/CLAUDE.md "Reach Enforcement": private requires
   beneficiary match) does not match actual HTTP behavior for content reads.

2. **Provenance is written only by the libp2p drain loop.** `mark_published`
   (`db/content_diesel.rs:883`) is called exclusively from `p2p/mod.rs:3204`; no HTTP write
   path sets `dht_anchor_hash`/`p2p_published_at`. On a P2P-disabled stack, freshly created
   content is permanently invisible to external reads (the provenance gate at
   `content_diesel.rs:155-160`).

**Acceptance spec already exists:** the two `@wip` read/refuse scenarios in
`genesis/a2o/features/lms/intimate-reach-household.feature` (step defs real and binding in
`genesis/a2o/steps/lamad/intimate-reach.steps.ts`). When HTTP-path reach enforcement lands,
run them and flip `@wip` — no new test authoring needed.

**Shape of the fix (for the design pass, not prescribed):** the HTTP content-read handlers
should route through the same `check_reach_authorization` the P2P path uses (agent_cid is
already resolved on the doorway-proxied path via X-Agent-Cid). Note dev-mode caveat: all
personas share `agentPubKey uhCAk-dev-mode-agent-key`; identity for the gate derives from
humanId (`human-<identifier>`, `auth_routes.rs:1512`).
