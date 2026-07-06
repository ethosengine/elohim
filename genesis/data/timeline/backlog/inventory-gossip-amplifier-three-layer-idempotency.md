---
id: "backlog-inventory-gossip-amplifier-three-layer-idempotency"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Inventory-gossip ~500× amplifier — three-layer idempotency (receive done, publish + gossip-id open)"
slug: "inventory-gossip-amplifier-three-layer-idempotency"
written: "2026-07-06"
author: "adam-p2p-instability-sprint"
status: "open"
priority: "high"
ci_status: blocked
severity: high
discovered: 2026-07-06
discovered_by: shift/adam-p2p-instability-sprint
domain: dataplane / p2p-protocol
pipelines: [elohim-genesis]
requires_env: alpha-cluster-6peer  # final root needs live gossipsub mesh introspection
needs: brainstorm (publish-side suppression + gossip message-id) — receive-side idempotency LANDED
cites:
  - genesis-pipeline-substrate-gated-adam-arc-saturation  # the storm this amplifies into
---

## What this is

The P0 fix (receive-side snapshot idempotency, `apply_snapshot` →
`SnapshotApplyOutcome::{Applied,Deduplicated}`, migration
`2026-07-06-120000_peer_inventory_cursor_content_hash`) makes adam DRAIN the storm
idempotently — it removes the CPU peg and the `/health` starvation regardless of what
amplifies. **This doc is the remaining question the P0 fix does NOT answer: why is the
inventory gossip plane carrying ~500× the design traffic in the first place?** After P0
this is a bandwidth/efficiency concern, not a stability one — but a WAN-native plane that
floods 500× is still wrong.

## Live evidence (adam inbound "Inventory snapshot applied", 2026-07-06 ~13:51Z)

- **Rate:** ~53 applies/sec (design ≈ 0.1/sec across ~7 peers = **~500×**).
- **Universal relay fan-in:** `from` (propagation_source) ≠ `peer_id` (author) on *every*
  line; the same author arrives via 5+ distinct `from` hops.
- **Historical sequence replay:** author `…QAa…` arrives with app `sequence=1303` at
  13:51:48 then `sequence=1086` at 13:51:57 — an *older* app-snapshot delivered *later*.
- **A few large-inventory peers dominate cost:** `…QAa…`/`…FhAP…` carry `count=263`/`count`
  large with runaway app-sequences (1042–1303) while household peers sit at `count=3–4`,
  `sequence≈3–58`. Each expensive re-apply is a 263-blob delete+reinsert + 263-hash score
  loop — this is why P0's skip-the-re-score is the leverage.

## Ruled out (do NOT re-chase)

| Candidate | Verdict |
|---|---|
| gap-recovery snapshot-REQUEST feedback loop (delta gap → request → snapshot flood) | **Ruled out** — `P2PCommand::SnapshotRequest` is a **Stage-1 no-op placeholder** (`p2p/mod.rs:3453`, logs "relying on next periodic snapshot"). Requests do nothing; they cannot amplify. |
| gossipsub re-delivering the SAME published message N× | **Unlikely as the primary** — default `message_id_fn` = `hash(source, gossip-seqno)` with the default ~1-min duplicate cache suppresses exact re-delivery of a given published message. The observed traffic is DISTINCT app-sequences, not one message re-delivered. |
| tx5 / MTU / arc-factor | already tabled in [[genesis-pipeline-substrate-gated-adam-arc-saturation]]. |

## Leading hypothesis (needs live mesh introspection to confirm)

**Publish-side re-flood by the large-inventory seed/bootstrap pair.** The two seed peers
(adam ↔ matthew per the alpha bootstrap-pair topology) have big inventories and runaway
app-sequences (1000s vs ~50 for household peers). If a peer's `broadcast_inventory_snapshot`
fires far faster than the 60s cadence — a mis-driven timer, or a re-publish-on-receive path —
it originates a new gossip message (new gossip-seqno → new message_id → not deduped) on every
tick, and the mesh relays each via ~6 hops. Two such peers mutually amplify. **Confirm with:**
per-peer publish rate (publisher-side log/metric), gossipsub mesh degree + IWANT counts, and
whether app-`sequence` on a seed peer climbs at >>1/60s. None of this is readable from the dev
container without a publisher-side probe — this is the `/brainstorm` + live-introspection leg.

## The architecture: three-layer idempotency for the inventory plane

The unifying thesis (P2P/seed layers must be idempotent/convergent under WAN adversity)
resolves the whole plane in THREE complementary layers. Receive-side is the authoritative,
restart-safe floor and is DONE; the other two cut upstream waste.

1. **Receive-side idempotency — LANDED (P0).** Content-fingerprint dedup on the cursor;
   identical snapshots no-op (no set churn, no re-score). Authoritative and restart-safe:
   works even if every publisher and the mesh misbehave. `db/peer_blob_inventory.rs`.

2. **Publish-side idempotency — RECOMMENDED next.** `broadcast_inventory_snapshot` should NOT
   re-emit an unchanged blob set: track the last-published content fingerprint and, when it is
   unchanged, either skip the publish or emit an empty delta / heartbeat instead of a full
   snapshot. On alpha (fairly static content) this collapses the origin rate from "every 60s
   per peer" to "only on change." Highest upstream leverage. **Caveat:** lives in the
   broadcast path in `p2p/mod.rs` — coordinate with the live-ops lane (same file as the
   receive-side change) and verify against a drained alpha.

3. **Gossip-layer idempotency — RECOMMENDED.** Set a custom `message_id_fn` for the inventory
   topic keyed on `(topic, author_peer_id, content_fingerprint)` and a `duplicate_cache_time`
   ≥ the broadcast interval, in `p2p/behaviour.rs` (currently a bare builder: only
   `heartbeat_interval(10s)` + `Strict`). This suppresses redundant re-snapshots of unchanged
   inventory at the gossip layer, BEFORE decode/app — the gossip-layer twin of layer 1. Keying
   on the AUTHOR (not content alone) preserves per-peer attribution: two peers legitimately
   hosting the same set stay distinct. `message_id_fn` is node-local (no wire/protocol version
   bump) → safe to deploy incrementally.

## Verification gate

Layers 2–3 need a drained alpha for a trustworthy before/after (the storm confounds any
measurement). Sequence: land P0 → confirm the apply-rate collapses on Loki → then brainstorm
+ implement layers 2–3 with live mesh introspection.
