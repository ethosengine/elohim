---
id: "backlog-security-unsigned-gossip-peer-becomes-jwt-trust-anchor"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A doorway learned only from UNSIGNED HTTP gossip is fetched for JWKS on a timer, and insert_positive overwrites an existing kid last-writer-wins — so a gossiped URL can replace a legitimate doorway's token-verification key"
slug: "security-unsigned-gossip-peer-becomes-jwt-trust-anchor"
written: "2026-08-29"
author: "opus (red-team lens of the client control-surface design pass; every line re-verified on disk)"
status: "backlog"
priority: "critical"
area: "doorway/doorway-service"
domain: "protocol"
jobs: [elohim-edge]
relatedNodeIds:
  - "concern:auth-discovery"
cites:
  - genesis/data/timeline/backlog/security-doorway-oauth-redirect-uri-interception.md
  - genesis/data/timeline/backlog/security-client-doorway-origin-synthesis-credential-exfil.md
tags: [security, auth, jwt, federation, jwks, trust-anchor, gossip, critical]
---

# The peer cache is two things at once, and only one of them is signed

`GET /api/v1/federation/doorways` deliberately serves rows of two provenances: rows sourced
from the infrastructure DHT carry `record_signature` / `signing_key` / `record_serial`, and rows
merged from HTTP gossip carry `None` for all three (`services/federation.rs`, the merge path).
That distinction is honest and useful — the client design consuming it labels the two
`dht-notarized` and `unsigned-gossip`.

**The JWT trust set does not observe the distinction.** Verified on disk 2026-08-29:

1. `fetch_single_peer` deserializes a peer's answer into
   `PeerEntry { id, url, region, capabilities, status }` (`federation.rs:1038-1045`). There is
   **no signature field in the struct at all** — nothing to verify even in principle — and every
   parsed row is mapped into a `PeerDoorway` and returned (`:1047-1058`).

2. `spawn_peer_jwks_refresh_task` reads `get_cached_peers(&peer_cache)` — *the same cache* —
   and hands the whole list to `refresh_peer_jwks_cache` on a timer (`:840-861`). Its own doc
   comment states the design intent plainly: it "piggybacks that machinery's OUTPUT (the
   known-peer list)".

3. `refresh_peer_jwks_cache` loops every peer, fetches its JWKS, and inserts every key it
   serves (`:789-803`).

4. `PeerJwksCache::insert_positive` is an unconditional `self.0.insert(kid, …)`
   (`:684-692`) — **no comparison against an existing entry for that `kid`.** Last writer wins.

## The chain

A peer already in the cache gossips `{id, url}` for a host the operator has never approved. The
timer fetches that host's JWKS. It serves a `kid` that collides with a legitimate doorway's, and
`insert_positive` silently replaces the real public key with the attacker's. Every subsequent
foreign-`kid` verification for that `kid` now succeeds against attacker-signed tokens.

Two facts bound the blast radius today, and neither should be relied on:

- The foreign-kid verify path refuses any non-EdDSA algorithm (`auth/jwt.rs:569-574`), and
  doorways currently mint **HS256** by default (`JwtSignAlg::Hs256`, `jwt.rs:330`, `:343`;
  `DOORWAY_JWT_SIGN_ALG` appears in no deployment manifest). So the cross-doorway path is
  narrow *because it is barely used* — the day EdDSA is turned on, this widens without any
  change here.
- `should_attempt_fetch` guards a fetch STORM (`:703-709`), not trust. It has no opinion about
  whether the answering host was ever authorized.

## The fix

1. **A peer enters the JWKS trust set only from a signature-verified `DoorwaySummary`.** Filter
   `get_cached_peers` output by provenance before `refresh_peer_jwks_cache` sees it — the
   distinction already exists in the data; it is simply not read here.
2. **`insert_positive` must refuse to overwrite a live positive entry with a DIFFERENT pubkey
   for the same `kid`**, and log the collision loudly. Re-inserting the same key is a refresh;
   changing it is either a rotation (which needs its own signed path) or an attack. Today they
   are indistinguishable and both silent.
3. Consider whether `fetch_single_peer` should carry a signature field at all, so an unsigned
   row is a *typed* second class rather than an absence.

## Provenance

Surfaced by the peer-plurality grounding leg of the client control-surface design pass, whose
output is otherwise about a TypeScript client. It is filed here rather than absorbed into that
design because it is a server-side trust-set defect that no client change can mitigate: the
client cannot see which peers the doorway has already decided to trust as token issuers.
