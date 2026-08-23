---
id: "backlog-token-discernment-mint-unauthenticated"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "SECURITY: POST /api/v1/token/discernment-mint accepts caller-supplied agent_id and amount with no authorization check"
slug: "token-discernment-mint-unauthenticated"
written: "2026-08-23"
author: "monetary-posture research pass (mint)"
status: "backlog"
priority: "high"
tags: [security, elohim-storage, token-plane, auth, research-mint]
cites:
  - elohim/elohim-storage/src/api/token.rs
  - elohim/elohim-storage/src/services/token_mint_service.rs
---
# Unauthenticated discernment-mint

`POST /api/v1/token/discernment-mint` parses a request body carrying a caller-supplied `agent_id` and
`amount` and calls `TokenMintService::discernment_mint` directly, with **no authorization check**
(`elohim/elohim-storage/src/api/token.rs:346-367`, `handle_discernment_mint`). Any caller who can
reach the route can mint to any agent id, in any amount.

## Status

- Found **independently by two of five red-team agents** during the 2026-08-07 issuance audit
  ([trap detectors](epr:comparative-political-economy-trap-detectors-2026-08-07) §10).
- **Re-verified still open 2026-08-23** on `fix/doorway-breaker-trial-theft-and-apps-extraction-herd`
  during the monetary-posture pass.

## Present impact, stated honestly

Currently **nil in effect**: the token plane is local-SQLite only, every write site hardcodes
`dht_anchor_hash: None` so no peer can verify or consume the rows, and the whole `/api/v1/token/*`
surface has **zero consumers repo-wide**. Nothing reads what this writes.

That is a reason it has survived, not a reason to leave it. It is a write endpoint that trusts its
caller's identity claim, in a plane whose disposition is an open decision. If any part of the token
plane is ever wired to a consumer before this is fixed, the defect ships with it.

## Fix shape

Either (a) authorize the route on the same operator-verb path the rest of the write surface uses, or
(b) retire the route with the rest of the crate's superseded surface — see the `elohim-token`
disposition item, since a route with no consumers may not need to exist at all.

**Standalone, not folded into a cluster**: operationally-atomic security defect per `CLUSTERS.md`.

Minted from [the succession evidence bridge](epr:succession-without-conquest-mutualist-lineage-2026-08-23) §2.6.1 and
[the monetary posture](epr:monetary-posture-internal-currencies-external-fiat-2026-08-23) §5.
