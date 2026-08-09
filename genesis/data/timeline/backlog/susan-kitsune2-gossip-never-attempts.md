---
id: "backlog-susan-kitsune2-gossip-never-attempts"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "susan (shem) emits zero kitsune2 gossip attempts — a different failure mode from the fleet's relay-seam failures"
slug: "susan-kitsune2-gossip-never-attempts"
written: "2026-08-08"
author: "integrator-session"
status: "backlog"
priority: "high"
tags: [alpha-cluster, kitsune2, gossip, susan, shem, dht, relay-seam]

relatedNodeIds:
  - "backlog-edge-deploy-ready-gate-liveness-only"
---

# susan never attempts kitsune2 gossip (unique in the fleet)

Operator cluster pass, 2026-08-08, identical 30-min windows on image
`1.0.0-dev-ecc840eb`: shem siblings attempt gossip constantly and fail 100%
on the relay seam (`Failed to initiate gossip: "Connection attempted before
home relay URL is known"` — adam 57/57, eve 531/531, gertrude 538/538
failed), while **susan attempts zero times**. Never-attempts is a distinct
failure mode from attempt-and-fail — upstream of the relay seam. Her
elohim-storage layer is healthy (DualGossipPublisher inventory snapshots
once/min, sequence advancing). Lead (unverified): susan logs
`iroh_gossip::net: received command for unknown topic`. From conductor logs
susan carries 2 DNA hashes, matching all four shem peers (weak proxy).

Facts that bound the search: susan is a deliberate, documented active peer
(2026-07-02 cast directive; Seattle leg of the tri-region backup chain) —
not deploy drift. She is the ONLY `device-recycled-laptop` archetype on
shem (gertrude/eve are home-nucs), so archetype-derived render values are
the first susan-vs-siblings diff to check.

## Open legs

1. Repo-side: full effective-config diff susan vs eve/gertrude through the
   template render path; gossip-topic derivation behind the unknown-topic
   lead; git history of her rows (2026-07-16 archetype realignment).
   (Investigation dispatched 2026-08-08.)
2. Cluster-side (operator): whether susan's conductor installed/activated
   the cell — needs conductor admin API, blocked on shem kubelet serving
   cert (valid for 192.168.86.36, node joins as 10.99.0.2; `microk8s
   refresh-certs` on the shem host unblocks exec).
3. Note for measurement hygiene: a peer emitting zero DHT gossip is
   unlikely to contaminate F2 quiesce/AIMD baselines — but any fleet-wide
   convergence claim ("fleet quiesced") that counts susan is overstating.

## Repo-side investigation results (2026-08-08/09)

Rendered-config diff susan vs eve/gertrude: **byte-equivalent on every
gossip-relevant axis** (bootstrap/signal/relay URLs, arc factor, k2Gossip
tuning, ELOHIM_TRANSPORT_BACKEND=dual, P2P bootstrap mesh, memory). The only
field with real divergence history is **edgenodeCpuLimit**.

Ranked hypotheses:
1. **CPU starvation blocked peer discovery (leading).** Archetype canonical
   for recycled-laptop is 1000m (half the NUC 2000m); the 2026-07-16
   realignment left susan at 1000m while eve/gertrude went to 2000m; live
   100% CFS throttle documented 2026-08-03 (acab4bf82 — "cannot serve
   ContentHeadRecord bytes inside the 5s responder budget"), fixed to 2000m
   via resourceOverride, unpushed-by-design, riding the next wave; ancestor
   of ecc840ebd so edge #1326 SHOULD have applied it — but see the
   susan-lags precedent (susan-overnight-image-heal-miss backlog: ~13h of
   stale pod spec across two generations). Mechanism: kitsune2_gossip
   initiate.rs:44-51,143-146 — empty local-agents or empty peer store
   silently SKIPS initiation (debug-level only, 1-5s retry) — exactly the
   never-attempts signature, vs the dial-path relay error which requires a
   populated peer store (why siblings attempt-and-fail instead).
2. Susan-specific node/pull staleness (documented precedent, above).
3. Fleet relay-registration race — explains the SIBLINGS' failures, ruled
   out for susan (she never reaches the dial step).
4. Cell not installed — weakened: ensure_happ_installed failure crashes the
   whole process, and her storage layer is healthily steady.

Red herring closed: `iroh_gossip received command for unknown topic` is
elohim-storage's own dataplane (BLAKE3 TopicId, p2p_iroh/gossip.rs:53-56);
the conductor fork has no iroh-gossip dependency at all.

**Discriminator (operator, needs kubectl):** susan's LIVE pod
resources.limits.cpu — 1000m ⇒ deploy never reconciled her (fix already in
repo; needs a deploy that actually restarts the pod); 2000m ⇒ CPU ruled
out, pivot to admin-API list_apps + peer-store dump.

**Natural experiment in flight:** the T5-flip batch (f64a44fb7, pushed
2026-08-08 ~23:45Z) changes the STS env, forcing a fleet restart that
re-renders susan onto the current 2000m spec. Post-deploy Loki check:
susan kitsune2 attempt count nonzero = hypothesis 1 confirmed-and-cured.
