---
id: "backlog-delivery-peer-row-local-port-and-untyped-capabilities"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "GET /api/v1/peers/delivery cannot be acted on: httpPort is a hardcoded 8090 for every peer, and the capability facts are flattened into an untyped string array with ready_content dropped"
slug: "delivery-peer-row-local-port-and-untyped-capabilities"
written: "2026-08-21"
author: "a2o-drain"
status: "backlog"
priority: "medium"
severity: medium
domain: D-delivery
source: "a2o wip-drain, features/delivery/delivery-diagnostics.feature — measured on the Act I household mesh 2026-08-21"
relatedNodeIds:
  - genesis/a2o/features/delivery/delivery-diagnostics.feature
  - genesis/a2o/features/delivery/peer-mesh.feature
tags: [delivery, p2p, peer-capabilities, wire-contract, elohim-storage]
cites:
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/http.rs
  - genesis/a2o/steps/drain/delivery-capabilities.steps.ts
---

`GET /api/v1/peers/delivery` is the only HTTP surface that answers "which peer should I
fetch this from?" Three defects together make its answer unusable, so a client that
consults it learns nothing it can act on and falls back to whole-blob download from the
doorway every time — which is precisely the storage-fanout the projection cache exists to
prevent.

## 1. `httpPort` is a constant wearing a per-peer field's clothes

`extract_http_port` (`elohim/elohim-storage/src/p2p/mod.rs`) takes a multiaddr and ignores
it:

```rust
fn extract_http_port(_addr: &str) -> u16 {
    // Default HTTP port for elohim-storage — peers don't advertise
    // HTTP port in multiaddr (that's for libp2p). The convention is 8090.
    8090
}
```

Measured on the household mesh, whose three peers serve HTTP on 8090 / 8091 / 8092, every
row came back `"httpPort": 8090`. A client that dials a `network: "lan"` peer at its
advertised port therefore dials **its own node** for two peers out of three — a silent
self-fetch that looks like a working LAN read while touching no peer at all. On a fleet
where every node happens to use 8090 the bug is invisible, which is why it has survived:
it is only wrong where it matters, on a mesh with more than one node per host.

The comment is honest about the cause — a libp2p multiaddr does not carry an HTTP port —
but the cure is not a convention baked into a function that pretends to derive something.
Either the capacity announcement carries the peer's real HTTP port (it is the peer's own
fact to state), or the field is removed and the client is told to discover it, rather than
handed a number that is right by luck.

## 2. The capability facts are flattened into an untyped string array

`DeliveryPeer.capabilities: Vec<String>` carries what the libp2p side models properly.
`EprResponse::DeliveryInfo` has typed `serves_extracted` / `serves_compressed` /
`cache_tier` / `warm`; the HTTP row has `["serves_compressed", "cache_tier:extraction"]` —
same facts, re-encoded as strings a consumer must parse and string-match. Absence is
ambiguous by construction: a peer that cannot serve extracted files and a peer that never
said either produce the same row.

On the household mesh every row read `["serves_compressed"]` alone, so nothing on the HTTP
surface can say whether ANY peer serves files individually.

## 3. `ready_content` is not projected onto HTTP at all

The list of content a peer holds warm exists on the libp2p side and has no HTTP
representation. It is the field that turns a peer list into a routing decision — without
it a client can rank peers by proximity but never by "who already has this."

## Why this is a defect and not test debt

The a2o scenarios "Operator can query a peer's delivery capabilities" and "Operator can see
all peers and their delivery capabilities" (`features/delivery/delivery-diagnostics.feature`)
were drained out of `@wip` onto this surface and now assert against its real shape. Their
failure messages name each missing field rather than being softened to match, so this row's
resolution is what turns them green — they are the regression seatbelt, not the complaint.

Related and worth deciding together: `EprRequest::QueryDelivery` has no HTTP trigger at
all, so the typed answer that already exists cannot be asked for from outside the process.
`features/delivery/peer-mesh.feature`'s "QueryDelivery protocol returns delivery info"
stays `@wip` for exactly that reason. Projecting the typed `DeliveryInfo` shape onto the
delivery-peer row would close both.

**2026-08-21 — part 1 closed.** `extract_http_port` deleted; the peer's real port now
travels via a `http_port=<port>` suffix on the libp2p Identify `agent_version` (self-declared
by each node from `Config::http_port`, decoded by `parse_http_port_from_agent_version`,
defaulting to `DEFAULT_HTTP_PORT` (8090) for un-upgraded peers) and is applied to the
`DeliveryPeer` row from both the mDNS-discovery insert and the Identify-received handler
(whichever fires second wins, so ordering can't leave a stale port). Parts 2 and 3
(untyped `capabilities` string array, missing `ready_content`) are untouched — still open.
