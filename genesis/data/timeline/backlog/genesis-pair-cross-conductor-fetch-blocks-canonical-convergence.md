---
id: "backlog-genesis-pair-cross-conductor-fetch-blocks-canonical-convergence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis-pair cross-conductor DHT fetch is down — elohim.host-side conductors cannot retrieve matthew-authored actions, the SOLE remaining blocker for notary-authority scenario 2"
slug: "genesis-pair-cross-conductor-fetch-blocks-canonical-convergence"
written: "2026-07-11"
author: "shift notary-scenario2-green"
status: "open"
priority: "high"
area: "substrate/kitsune2-connectivity"
domain: "operator"
jobs: [elohim-edge]
relatedNodeIds:
  - "memory:project_alpha_topology_bootstrap_pair"
  - "memory:feedback_reach_head_replication_distinct_planes"
cites:
  - genesis/data/timeline/backlog/view-federation-request-flakiness-mesh-wide.md
  - peer-discovery-fractal-federation | Peer Discovery as Fractal Federation | sha256:42ae0e67f9e9d4bc | path: genesis/docs/superpowers/specs/2026-07-09-peer-discovery-fractal-federation-design.md
  - genesis/a2o/features/dataplane/notary-authority.feature
tags: [substrate, kitsune2, tx5, dht-fetch, genesis-pair, notary-authority, f-t19, signal-bus, canonical-head]
---

# Genesis-pair cross-conductor DHT fetch is down

## What works (proven live, 2026-07-11 overnight shift)

The entire notary head-election chain above the substrate is DONE:
tier-aware cross-root selector hot-swapped onto BOTH genesis conductors
(functional proof: scenario 3 green — the new guard refuses unauthorized
moves; elohim.host's conductor answers the new fn's own retrievability
refusal from lib.rs:3207), declaration act wired (deploy designates via
`POST /db/content/{id}/canonical-head`, propagated to EVERY doorway each
app deploy — `DECLARE_ONLY` leg in `scripts/ci/stage-spa-blob.sh`), and
the declaring side converges within the same deploy.

## The blocker

`elohim.host`-side conductors cannot RETRIEVE matthew-authored actions:
every propagation attempt returns

    Guest("declare_canonical_head: target action ActionHash(...) is not retrievable")

(zome-side network `get` timeout). Watched divergent 48+ minutes across
multiple freshly-authored heads. This is the F-T19 class the spine's
notary-authority evidence already names — now sharpened: the signal bus
is VERIFIED live on both doorways (`/health dhtBacking.signalShared:
true`), bridges the SBD relays, and the gap persists. So the failure sits
BELOW the relay layer. Candidate causes, in investigation order:

1. tx5/kitsune2 WebRTC session re-establishment: conductors may hold
   long-backoff state or never re-attempt cross-relay handshakes even
   though the bus now bridges frames (conductor restarts in edge #1168,
   #1171 did not heal it).
2. The bus's cross-pod delivery path is UNVERIFIED at runtime — the
   `#[ignore]` mongo cross-pod test (`frame_published_on_a_is_drained_by_b_not_a`)
   has never run against a real Mongo (needs `MONGODB_TEST_URI`). A
   one-shot manual run against alpha's mongo would confirm or refute in
   minutes.
3. Node-level networking on the elohim.host side (the original F-T19
   framing: outbound timeouts to ~11 peers).

## The standing diagnostic (free, every deploy)

Every app deploy now emits the live probe in the `authorHeadOnce` /
propagation console output: `✓ canonical head propagated to <doorway>`
means the fetch path healed; the `⚠ not retrievable` warning means it is
still down. No manual reproduction needed.

## Why this is the last mile

The moment cross-conductor fetch works, convergence is AUTOMATIC on the
next app deploy (no code change): the propagation declare succeeds on the
second peer, its row eager-stamps, and notary-authority scenario 2 flips
green. Everything above the substrate is waiting on this single link.
