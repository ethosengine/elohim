---
title: adam CPU storm — inventory-snapshot amplification under WAN loss (NOT arc-factor)
status: open
ci_status: blocked
severity: high
discovered: 2026-07-06
discovered_by: shift/overnight-genesis-pipeline-stabilize
corrected_by: live-ops investigation 2026-07-06 (kubectl + source trace on /home/matthew/elohim)
domain: dataplane / p2p-protocol
pipelines: [elohim-genesis]
requires_env: alpha-cluster-6peer
needs: fix (receive-side idempotency) + brainstorm (gossipsub amplifier)
---

## ⚠️ CORRECTION — the overnight conclusion in the first draft of this file was WRONG

The overnight shift concluded "full-arc gossip saturation → `target_arc_factor < 1`."
A deeper live investigation (kubectl on adam + source trace) **overturned that lever.**
Do NOT chase arc-factor or MTU. Recording both the correct root and the discarded
framings so the next shift doesn't re-walk them.

| Framing | Verdict |
|---|---|
| overnight: full-arc gossip saturation → arc-factor<1 / more compute | **Right genre, WRONG lever** — arc-factor only dampens; more cores just feed the spin |
| MTU / k8s-VXLAN-over-WireGuard double-encap black-hole | **Wrong** — overlay MTU is correctly clamped (pod eth0 = 1370); storage libp2p over the overlay connects fine |
| tx5 / WebRTC signal-preflight flap | **Real but minor** — ~14 events/33s, a secondary WAN-transport robustness issue, not the CPU sink |
| **inventory-snapshot amplification storm** | **ROOT CAUSE** — ~150 snapshot-applies/sec, non-idempotent, in `elohim_storage` inventory |

## Root cause (grounded)

Adam pegs ~2 cores because **~95% of his log output (~2847/3000 lines, ~150/sec) is
"Inventory snapshot applied"** — he re-applies full `BlobInventorySnapshot`s continuously
and never converges. The design cadence (archetype family-node-base → node class) is
**~1 snapshot / 60s per peer** → with ~7 alpha peers adam should see **~0.1/sec**. He sees
**~150/sec = ~1000× amplification.**

Mechanism:
- `apply_snapshot` (db/peer_blob_inventory) is **deliberately applied regardless of
  sequence** ("recovery path from sequence-manipulation attacks") and each apply re-runs
  `score_and_enqueue_snapshot` (commitment scoring = real CPU). Per-message "correct."
- Deltas ARE idempotent (seq<=max → replay-drop; gap → request snapshot). Snapshots bypass
  all of it.
- Under shem's WAN loss/reorder, gossipsub re-floods duplicate snapshots (IHAVE/IWANT
  gap-recovery re-delivery), and peers relay each other's inventory (`from` ≠ `peer_id`),
  multiplying the fan-in. Adam — full-mesh, full-arc sink — absorbs and re-applies every echo.
  Positive feedback, no receive-side idempotency, no backoff → never converges → /health
  starved → liveness SIGKILL every ~25m → cold reload of the full 512-sector arc → same storm.

**shem did not cause this — its WAN harshness EXPOSED it** (on-prem near-zero-loss, adam
just barely stays ahead; over WireGuard he can't drain it). This is the adversarial-peer
node doing exactly its job.

## The fix (code — elohim-storage, ships via edge pipeline; NOT a live patch)

Owned by the live-ops session (kubectl + /home/matthew/elohim checkout). Do not duplicate
from other checkouts.

1. **Surgical / high-leverage — receive-side idempotency.** At `p2p/mod.rs:~5702` +
   `db/peer_blob_inventory::apply_snapshot`: before apply + `score_and_enqueue_snapshot`,
   drop snapshots whose `(peer_id, sequence, content-hash)` was already applied. Preserves
   the deliberate "accept regardless of sequence for attack-recovery" for genuinely
   new/different snapshots; a byte-identical re-echo becomes a cheap no-op instead of a full
   re-score. Kills the CPU bleed regardless of what amplifies.
2. **Deeper — the ~1000× gossipsub amplifier.** Why is gossipsub re-delivering at that rate
   (message-id dedup window, mesh degree, re-publish-on-receipt)? Protocol-design question →
   `/brainstorm`, needs peer-side logs + mesh introspection.

## Live stopgap (optional; does NOT fix — just stops making it worse)

Liveness probe `httpGet /health` → `tcpSocket:8090` (readiness stays httpGet). Stops the
destructive ~25m SIGKILL→cold-full-arc-reload loop that guarantees adam never catches up,
while fix #1 ships. Do NOT bump CPU (feeds the storm).

## Still-true observations from the overnight shift (not overturned)

- genesis stability is substrate-gated on adam; while adam storms, `alpha.elohim.host/`
  serves 503 (root path can't compose conductor-served content) → genesis "Verify Target
  Health" 120s probe times out (exit 124) → FAILURE. Fixing the storm should clear this.
- The reanchor heal (elohim-storage dev-8a1f7a29) is live on adam and stopped reanchor
  thrash — necessary, orthogonal to this storm.
- Companion pipeline-robustness candidate stands: genesis "Verify Target Health" couples the
  seed-readiness gate to the app SPA host (TARGET_HOST) rather than storage/doorway — a 503
  on the SPA hard-fails seeding for an orthogonal reason.
