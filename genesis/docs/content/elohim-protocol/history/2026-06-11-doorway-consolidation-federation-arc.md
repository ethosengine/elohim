---
title: "History: The doorway consolidation + federation design arc (Dec 2025 – Apr 2026)"
id: doorway-consolidation-federation-arc
type: history-gotcha
status: noted
tier: history
created: 2026-06-11
topic: [doorway, federation, consolidation, did, jwt, blob-fanout, design-arc]
# Provenance breadcrumb: the two retiring island docs this record distills (dates are git first-commit; FEDERATION.md last revised 2026-04-30).
derived_from:
  - doorway/doorway-service/ARCHITECTURE.md  # retired to git 2026-06-11 (doorway island recompose; authored 2025-12-19)
  - doorway/doorway-service/FEDERATION.md    # retired to git 2026-06-11 (doorway island recompose; authored 2025-12-25)
canonical:
  - doorway/CLAUDE.md
  - genesis/docs/content/elohim-protocol/history/2026-06-02-doorway-dispatch-registry-fallback-and-vocabulary.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md
cites:
  - doorway/CLAUDE.md
  - doorway/doorway-service/src/lib.rs
  - doorway/doorway-service/src/config.rs
  - doorway/doorway-service/src/signal/media.rs
  - doorway/doorway-service/src/projection/subscriber.rs
  - doorway/doorway-service/src/cache/rules.rs
  - doorway/doorway-service/src/cache/resolution.rs
  - doorway/doorway-service/src/routes/api.rs
  - doorway/doorway-service/src/auth/jwt.rs
  - doorway/doorway-service/src/routes/federation.rs
  - doorway/doorway-service/src/routes/identity.rs
  - doorway/doorway-service/src/services/federation.rs
  - doorway/doorway-service/src/services/did_resolver.rs
  - doorway/doorway-service/src/main.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs
  - elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs
  - storage-dual-plane-design-arc | sibling recompose record (same day) holding the cache-core extraction, WriteBuffer removal, and P2P-bootstrap-role verdicts — cited, never restated | sha256:2315c84345a2ef3c | path: genesis/docs/content/elohim-protocol/history/2026-06-11-storage-dual-plane-design-arc.md
memory_anchors:
  - project_doorway_single_target_no_fanout
  - project_inventory_exchange_not_byte_replication
---

# History: The doorway consolidation + federation design arc (Dec 2025 – Apr 2026)

> **Hot-context pointer (the one sentence to remember):**
> The three-service consolidation paid in full and most federation plumbing shipped
> file-for-file — but FEDERATION.md's custodian-selection content routing was
> **constitutionally inverted** by doorway/CLAUDE.md's No Blob Fan-Out rule: a
> federation design can ship every surrounding mechanism while the trust-boundary
> canon reverses its core flow.

## The consolidation bet paid

ARCHITECTURE.md proposed absorbing three separate Holo-infrastructure services
(CloudFlare bootstrap worker, SBD signal server, gateway) into one Rust binary.
That is the live shape: `src/lib.rs:20,37` (`bootstrap`/`signal` modules),
`src/config.rs:90,94` (`BOOTSTRAP_ENABLED`/`SIGNAL_ENABLED`, both default `true`),
`src/signal/media.rs` — one domain, one deploy (`doorway/CLAUDE.md` "Three
Consolidated Services"). The type-agnostic projection principle (doorway never
parses DNA signal payloads) survived into the live subscriber: opaque
`doc_type`-keyed signals in `src/projection/subscriber.rs:51`.

## What shipped as designed (rare wins to name)

- **`__doorway_cache_rules` discovery is LIVE end-to-end**: the DNA implements it
  (`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:1213`), doorway
  consumes it (`src/cache/rules.rs:3,63` — defaults when a DNA declines). Contrast
  with the sibling record's finding that `__doorway_routes` of the SAME introspection
  family is a stub (routes come from steward self-registration): the cache-rule half
  of DNA introspection landed while the route half didn't — see
  `2026-06-11-storage-dual-plane-design-arc.md`, not re-derived here.
- **`/api/v1/cache/{type}/{id}`** still live as drawn (`src/routes/api.rs:8,184`).
- **DoorwayResolver tiered resolution** live via elohim-cache-core
  (`src/cache/resolution.rs:35,89` — `elohim_cache_core::ContentResolver`). The
  cache-core extraction itself is canonized in the sibling record.

ARCHITECTURE.md Phase 5's `DoorwayWriteBuffer` was later removed from doorway
("WriteBuffer out of doorway … landed verbatim") and FEDERATION.md §P2P Bootstrap
Role (signal-server libp2p peer exchange) was never built — both already canonized
in the sibling record `2026-06-11-storage-dual-plane-design-arc.md`; cited, not restated.

## The philosophy inversion (the strongest lesson)

FEDERATION.md §Content Routing designed doorway-side custodian selection: "Doorway A
probes Doorway B, C, D for health … selects best custodian … fetches from selected
custodian." That is exactly the peer-aware blob fan-out that `doorway/CLAUDE.md`
§"No Blob Fan-Out — Doorway is Single-Target Dispatch" now constitutionally FORBIDS:
doorway forwards each request to a single storage target; byte mobility is the
substrate's job (the self-healing P2P dataplane design, retired to git from
`genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md`).
A federation design can be inverted by the trust-boundary canon while its
surrounding mechanisms ship.

## Federation mechanisms that DID ship (file:line verified 2026-06-11)

- JWT `doorway_id`/`doorway_url` claims: `src/auth/jwt.rs:92-95`.
- `GET /.well-known/doorway-keys` JWKS for cross-doorway token validation:
  `src/routes/federation.rs:143-147` (`handle_doorway_keys`).
- `GET /.well-known/did.json` DID document with `elohim:capabilities`/`elohim:region`:
  `src/routes/identity.rs:46-50,229`.
- Doorway self-registration as `DoorwayRegistration` DHT entry:
  `src/services/federation.rs:125` (`register_doorway_in_dht`, zome call
  `register_doorway` at :153) → entry at
  `elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs:81-103`.
- `FEDERATION_PEERS` discovery: `src/config.rs:207`, `src/main.rs:970-978`
  (`spawn_peer_discovery_task`, defined `src/services/federation.rs:766`).
- `storage_dids` on `BlobManifest`: DNA-side only
  (`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:704`);
  zero consumers in doorway-service — the DID-routed federated blob fetch flow was
  superseded by single-target dispatch + substrate replication.

## Stale code-comment found (repaired in this recompose)

`src/routes/identity.rs:4` and `src/services/did_resolver.rs:6` cited
"holochain/doorway/DID-FEDERATION.md", which never existed at that path — the
content was FEDERATION.md §Doorway Discovery via DIDs (now this record's git
history). Repaired 2026-06-11 as part of this recompose: both module comments now
point at `doorway/CLAUDE.md` §Federation, where the live federation mechanisms are
documented.

OPEN QUESTION: the hub-edge design spec
(`genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md`) was retired
to git (deleted in 53190a234) while its 13 decomposed gap-items remain OPEN in
`.claude/memory-kit/gap-items/specs__2026-05-08-doorway-hub-edge-design.json` —
where does live hub-edge guidance now anchor?
