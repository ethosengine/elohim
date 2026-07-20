---
title: "History/Finding: adam slow-link melt — gossip storm saturates the conductor write guard (composition defect, not placement)"
id: adam-slow-link-write-guard-saturation
type: incident-analysis
status: Resolved (structural fix landed 2026-07-20; conductor-fork patch deferred)
tier: history
created: 2026-07-20
topic: [dataplane, kitsune2, gossip, holochain-sqlite, ptxnguard, doorway-breaker, shem, wireguard, inventory-sync]
# Observability-side analysis of the 2026-07-20 elohim.host catching-up 503 instability.
# The k8s/observability leg is complete; the code-read leg (which write path holds the
# guard; retry/backoff policy; cap-2000 convergence) is handed to the dev session.
canonical:
  - ../architecture/2026-07-12-substrate-trust-contract-runbook.md   # the trust contract these probes serve
memory_anchors:
  - feedback_k8s_is_not_the_architecture
  - project_alpha_substrate_probe_rails
  - project_inventory_exchange_not_byte_replication
---

# adam slow-link melt — gossip storm saturates the conductor write guard

> **Hot-context pointer (the one sentence to remember):**
> A slow network link must never surface as a held local database write guard. On adam it does:
> the gossip/sync layer transmits link quality straight into `holochain_sqlite` write-lock
> saturation (~50% duty cycle), starving content reads and opening the doorway breaker.
> Restarts and rescheduling are non-fixes — the loop re-enters on boot, and moving adam off
> shem would launder away the very signal the heterogeneous-network fixture exists to produce.

## Symptom (user-visible)

`elohim.host` alternates between `503 {"status":"catching-up"}` (`x-ssr-skipped:
shell-breaker-open`, fast ~40ms sheds) and occasional slow 200s (2.4–7.4s SSR — half-open
breaker probes that squeak through). `alpha.elohim.host` serves 200 in 20–100ms throughout.
**Both doorways run the same commit (`b0fac6f`)** — the difference is entirely which storage
node each doorway reads: elohim.host → adam, alpha → matthew.

## Topology context (by design, not a bug)

- adam-alpha-0 runs on **shem** — remote-WAN WireGuard node (10.99.0.2, MTU-1420 ceiling).
- matthew-alpha-0 runs on **ethosengine** — local LAN.
- This heterogeneity is the **fixture doing its job**: k8s here models a hostile-network peer
  condition the P2P stack must absorb. Fixing this by rescheduling adam onto the LAN would
  delete the evidence, not the defect ([[feedback_k8s_is_not_the_architecture]]).

## Evidence chain (all measured 2026-07-20, ~13:00–15:20 UTC)

| Signal (adam vs matthew) | adam (shem/WAN) | matthew (LAN) |
|---|---|---|
| Doorway upstream circuit | half-open, errorStreak 12 | closed, errorStreak 0 |
| Conductor CPU | multi-core pin, **flat for 6h+** (windows measured at 4.3–6.0 and 2.24–3.2 cores; zero downward slope) | 0.28–0.58 cores |
| Memory / restarts | 2.25 GB, 0 restarts | 3.23 GB, 0 restarts — **not** a memory/OOM event |
| `PTxnGuard was held` (holochain_sqlite write guard) | **~45–66/min, median 732ms, max 1560ms ≈ 50% duty cycle** | ~3/min baseline |
| Inventory/projection log events | ~6–12/min | comparable |
| Holochain workflow events (app_validation, publish_dht_ops) | ~8/min, **healthy** (1/1 validated, 0 awaiting deps, 192/192 published) | healthy |
| Gossip pathology | full ~4258-entry inventory re-exchange every few seconds; `Outbound sync request failed … Timeout`; `Outbound shard request failed … Timeout`; kitsune2 `Initiated round timed out`; `ProjectionInventory: content inventory truncated at cap cap:2000 served:2000 … true count exceeds cap` | same primitives at far lower degree (rounds mostly complete) |
| kitsune2 `Unsolicited RingSectorDetailsDiff` PeerBehaviorError | 5 / 15min — incidental noise, not a driver | — |

## Hypotheses tested

| Hypothesis | Verdict | Evidence |
|---|---|---|
| Post-deploy transient; adam will settle | **Refuted** | No deploy for hours; CPU flat with zero downward slope across 6h |
| Memory pressure / OOM (the historical adam class) | **Refuted** | adam RAM *below* matthew's; 0 restarts |
| k8s placement bug (adam stranded on shem) | **Reframed** | Placement is the deliberate fixture; the defect is that the stack transmits the condition instead of absorbing it |
| Doorway restart heals it | **Refuted** | Morning fleet rollout restarted the doorway only → breaker counter reset, re-opened within minutes; adam-0 storage never restarted and stayed pinned |
| Inventory projection holds the write guard | **Refuted by timestamp correlation** | Projection fires ~6–12/min; guard is taken ~45/min at ~730ms — an order of magnitude too infrequent to be the holder |
| Gossip-storm-driven high-frequency `holochain_sqlite` write path saturates the guard | **Lead — consistent with all evidence; code-read pending** | Guard holder is not surfaced in INFO logs; conductor workflows are healthy and too infrequent; the only high-rate work on adam is the gossip reconcile |

## Confirmed mechanism (the causal chain)

1. On the slow link, outbound sync/shard/gossip rounds time out and retry at high rate;
   inventory exchange is **full-snapshot (~4258 entries), not delta**, and advertisement is
   **capped at 2000 below the true count** — lead hypothesis for why reconcile never converges
   (the advertised set cannot represent the real set → perpetual re-gossip). Unproven: matthew
   escaping the cap could not be cleanly confirmed (sampling artifact).
2. That reconcile churn drives a high-frequency `holochain_sqlite` write path that holds the
   conductor's write guard ~45×/min at ~730ms each — **~50% duty cycle**.
3. Content reads starve behind the guard → adam serves content in 2–7s while its lightweight
   health ping still passes ("reachable but degraded" — exactly the trust-contract shape).
4. Doorway SSR shell-breaker sits at that latency threshold → opens → `elohim.host` 503
   catching-up; half-open probes occasionally pass a slow 200 → the observed flap. The
   catching-up page itself behaved exactly as designed.

**Protocol-level significance:** this is a composition defect. Resilience *over* the stack
means a peer on a hostile network is merely slow to converge while staying locally healthy
(async fetch/validate; take the write lock only to commit; bounded backoff; deltas; projections
read off snapshots). Here, link slowness becomes local resource serialization — the P2P layer
transmits the network condition through into the database instead of absorbing it.

## Handoff — code-read checklist (dev session)

1. **Who holds the guard**: find the `holochain_sqlite` write path that runs ~45/min with
   ~700ms holds under a kitsune2 gossip storm (op integration / gossip-driven re-fetch +
   re-integration — are idempotent re-writes still taking the write transaction?).
2. **Retry/backoff policy** in `elohim_storage` inventory/shard/sync on outbound `Timeout` —
   bounded with backoff, or tight-loop retry?
3. **Cap-2000 convergence**: `ProjectionInventory` caps advertisement at 2000 while adam holds
   4258 — can the reconcile diff ever close? If not, this is the perpetual-motion source.
4. **Full-inventory vs delta** exchange in the content-sync plane.

## Resolution (2026-07-20, same day)

The code read confirmed the mechanism and the fix landed as two waves (independent review + adversarial
verification between implement and integrate):

1. **Structural (elohim-storage):** `run_replication_cycle` was full-snapshot-pulling every peer every
   60s with no backoff, no per-peer in-flight guard, and no pagination (`has_more` existed on the wire
   but nothing ever requested page 2); on a slow link each pull timed out at the fixed 30s and re-fired
   next tick — a self-sustaining thrash loop. Fix: per-peer in-flight guard + exponential backoff
   (60s→15min cap, reset on success — `p2p/replication_schedule.rs`), ContentList pagination at
   1000/page, ProjectionInventory honest wire `total` (was served-count — the consumer couldn't even
   see truncation), an optional wire-compatible `inventory_offset` rotating the reconcile window so the
   cold tail (rows beyond `PROJECTION_INVENTORY_CAP=2000`, ordered `updated_at DESC`) is no longer
   permanently invisible, and a `MAX_INVENTORY_WINDOW_TOTAL` clamp so a lying peer can't pin the window
   (sibling of `MAX_SYNC_LIST_OFFSET`).
2. **Conductor config:** `network.advanced.k2Gossip: roundTimeoutMs 60000, maxConcurrentAcceptedRounds 4`
   on all three live surfaces (edgenode config, consolidated template, adam's hand-crafted manifest) —
   slow-link rounds complete instead of thrash-retrying into historical-catch-up slice-hash writes.
3. **Rejected during review:** `db_sync_strategy: Fast` — source-verified to mean `PRAGMA synchronous=OFF`
   (corruption-on-crash), not the assumed NORMAL; the default `Resilient` already IS NORMAL. Dropped.

**Deferred follow-on (the likely guard holder itself):** kitsune2's `HolochainOpStore::store_slice_hash`
opens an unconditional write txn per historical slice-hash update with no change-check
(`op_store.rs` ~354-391, driven by `TimePartition::inform_ops_stored` catch-up). Patching it means
re-pointing the edge image build from the prebuilt holo-host binary to the fork-source build
(`Dockerfile.zombie-fix` pattern, proven by the tx5 zombie-leak precedent) — a separate lift, and the
config pacing attacks the same duty cycle without code risk. Revisit if the probes below stay hot
after this fix deploys.

## Post-deploy measurements (2026-07-20 ~20:30 UTC, ~75min after fleet restart on the fix)

| Probe | Pre-fix | Post-deploy | Verdict |
|---|---|---|---|
| elohim.host serving | 503 catching-up storm, breaker flap | 200 + `x-ssr-rendered: 1` at 31–48ms; circuit closed; errorStreak 0 | healed |
| kitsune2 round timeouts (adam, 15min) | 43 | **3** | k2Gossip config active + effective |
| Outbound sync/shard failures (15min) | storm | 34 (~2/min, backing off) | storage backoff effective |
| `PTxnGuard was held` (15min) | 984 | **2,147** | see below |
| adam CPU | ~5 cores flat | ~5 cores flat | unchanged |

The guard-rate *increase* alongside the 14× round-timeout drop has one coherent reading: rounds now
COMPLETE and deliver ops, so kitsune2's historical catch-up (`store_slice_hash` write-per-slice-update)
is finally doing productive work against the divergence backlog it previously could never finish.
**A/B discriminator, decided by the trend:** (A) guard rate decays over hours = finite backlog drain —
no conductor patch needed; (B) guard rate still ~2k/15min a day later = the `store_slice_hash`
unconditional-write amplification is steady-state cost on a full-arc node → execute the deferred
conductor-fork patch. Either way the serving layer stays insulated (breaker + doorway hot cache carry
elohim.host at full speed; only the first cold fetch shows a multi-second tail while the storm runs).

## Verification probes (after any fix lands)

- adam-0 CPU falls to matthew's band (~0.3–0.6 cores), not just post-restart dip.
- `PTxnGuard was held` rate on adam → baseline (~single digits/15min).
- `elohim.host/status.json` → `upstreams[0].circuit: "closed"`, `errorStreak: 0`.
- `curl -A Mozilla https://elohim.host/` → repeated 200 + `x-ssr-rendered: 1` at <200ms.
