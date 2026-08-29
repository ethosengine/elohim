---
id: "backlog-security-unsigned-gossip-peer-becomes-jwt-trust-anchor"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A doorway learned only from UNSIGNED HTTP gossip is fetched for JWKS on a timer, and insert_positive overwrites an existing kid last-writer-wins — so a gossiped URL can replace a legitimate doorway's token-verification key"
slug: "security-unsigned-gossip-peer-becomes-jwt-trust-anchor"
written: "2026-08-29"
author: "opus (red-team lens of the client control-surface design pass; every line re-verified on disk)"
status: "resolved"
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


---

## RESOLVED 2026-08-29

Both halves landed. `cargo test federation` green: 36 tests, three of them new.

**(a) The entry guard.** `PeerJwksCache::insert_positive` returns bool and refuses
to replace a live kid with a DIFFERENT pubkey; the same key for the same kid is a
legitimate refresh and still succeeds. Two tests pin both directions
(`insert_positive_refuses_conflicting_pubkey_for_same_kid`,
`insert_positive_accepts_identical_pubkey_as_refresh`) so a guard that simply
refused everything would not pass.

**(b) The source.** `spawn_peer_jwks_refresh_task` no longer reads `PeerCache` at
all. It takes the ZomeCaller + FederationConfig, asks the DHT doorway registry,
and fetches JWKS only for doorways carrying a non-empty `signing_key` — asserted
by `jwks_refresh_never_reaches_a_doorway_without_a_signing_key`.

**This row's own step 1 was NOT implemented, because it is unimplementable as
written.** "Filter the peer cache by provenance" cannot be done: `PeerDoorway`
carries no signature material, `PeerCache` is 100% gossip by construction, and
`record_signature` is verified nowhere in this service. A literal implementation
would either no-op or empty the trust set. Changing the SOURCE was the available
fix, not filtering the existing one. The `record_signature` half is filed
separately as `security-federation-record-signature-serialized-never-verified`.

**Where it landed:** commit `4230637f4`. That commit's message describes
dataplane inventory work and does not mention this fix — a concurrent session
committed across the shared worktree while these changes were staged. The code is
correct and tested; the trail is recorded here because the commit message will
not lead anyone to it.

**Carry forward:** half (b) is fail-closed. A doorway with no `zome_caller` ends
with an empty JWKS trust set and cross-doorway verification stops. That is nearly
free today — HS256 is the mint default and the foreign-kid path hard-refuses
non-EdDSA — but it becomes load-bearing the moment `DOORWAY_JWT_SIGN_ALG` is
flipped to eddsa. Re-check it BEFORE that flip, not after.
