---
name: project_household_gossipsub_silent_on_ethosengine
title: "Household custody gossip silent on ethosengine (resolved R1)"
description: "2026-11-06: custody announcements never reached iroh and gossipsub logged InsufficientPeers; cured by SignedCustodyAnnouncement via fan-out (settled 2026-11-07 both households); open: uploader self-copy only when PeerSelection picks itself, striped shard_index > 6 refused by node-registry."
metadata:
  node_type: memory
  type: project
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-06T13:30:38.882Z
---

On the recast household (`MESH_RESET=1`, transport dual, three peers on localhost) at the
re-genesis 1 integrated tree (dev 6a31ed270), deliverability (7/7), blob-replication (2/2),
epr-content-addressing (2/2) and app-blob-heal-on-read (3/3) passed, but
`chaos-peer-churn.feature` "Cascading peer loss" refused at its baseline step: custody of the
content was real on all three peers (bytes + commitments, commitments even healed across peers
by projection-reconcile), yet matthew's `shard_locations` held only its own `self-held` row.

Cause observed: libp2p gossipsub never formed a mesh — every publish on every topic logged
`GossipPublish: gossipsub publish failed … InsufficientPeers` on all three peers from start to
end (75 min), `elohim_inventory_pages_published_total{outcome="sent"} = 0`. Remote custody reaches
`shard_locations` only through the custody-announcement gossip (`elohim/storage/custody/2`; the
iroh copy is unsigned and carries no custody claim under C1), and no storage handler projects the
DNA's ShardAssignment/ShardLocation signals into that table. Request-response planes (peer
discovery, reconcile) worked throughout.

Unknown: whether the crossing's inventory/custody topic rework (signed envelopes, topic v2,
author binding) or this device's libp2p environment silenced gossipsub — the household on
ethosengine had never run before; earlier receipts came from shem. Decide with shem's run on dev
or a pre-crossing comparison.

Separate definite defect from the same run: storage registers striped custody units with a
global index (`elohim-storage/src/http.rs` ~4076, `custody_units().enumerate()`), node-registry
integrity refuses `shard_index > MAX_SHARD_INDEX = 6` → 63 "Background shard-assignment
registration failed" on one striped content. Needs a `stripe_index` on ShardAssignment or a
lifted cap (integrity change either way).

Related: [[project_regenesis1_landing_traps_receipt_and_epr_closure]],
[[project_content_sync_plane]].

**Resolution (2026-11-06, R1):** the producer never reached iroh (`announce_custody` published straight to libp2p gossipsub, not the fan-out) and the receiver refused iroh-sourced announcements (no authenticated author). Fix = `SignedCustodyAnnouncement{sig: SignatureEnvelope, body, epoch, author_peer}` under domain `elohim:custody-announce:v2:` (transcript v1), published through the fan-out, admitted by signature → author_peer derived from the signing key → `peer_transport_manifest` binding → C1; holders claim their own pushed shards (`HeldShardSink` → `ClaimHeldShard`). Dual-transport nodes sign with the iroh key because the household's manifest rows carry only `iroh_node_id`. Residual (pre-existing, plane-wide): bindings are self-asserted, so custody rows are self-reported possession until agent↔transport bindings are cross-signed; the inventory envelope still signs with the libp2p key (same unbound-author gap on iroh-only households).

**Settled (2026-11-07, both households on dev 89d4a22ff):** R1 holds — refused_* = 0 on every peer on ethosengine and shem; the one-peer "received 0" seen on ethosengine was iroh-gossip neighbour topology (shem saw 15/38/18 on the same peer), not a defect. The chaos-peer-churn `:112` red that remains on shem (2 of 3 copies) is a different, pre-existing defect: the uploader records its own verified copy only when PeerSelection picks itself (`services/peer_selection.rs:319`, `p2p/mod.rs` ~2420–2660), so whether the fold sees 3/3 or 2/3 depends on fresh agent-key order per recast. Ruling: record + announce the verified local full copy regardless of selection (fix in storage, fixture unchanged).
