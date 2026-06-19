---
id: conductor-authority-arc-memory-scaling
status: design
created: 2026-06-13
class: substrate
artifact_kind: spec
written: 2026-06-13
cites:
  - tiered-quilt-stewardship-design | Tiered Quilt Stewardship | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - genesis/plans/2026-04-13-device-archetypes-design.md
---

# Conductor Authority-Arc Memory Scaling

> ## ⚠ EVIDENCE CORRECTED — 2026-06-19 (the corpus-scaling rationale stands on first principles, not this OOM)
> This spec cites alpha james OOM-flapping (3Gi/~9min; 8Gi only moved the ceiling) as live evidence for the
> corpus-authority-arc memory concern, and proposes a `target_arc_factor:0` ablation. That OOM was NOT
> corpus-arc — it was a native glibc-malloc arena leak, arc-INDEPENDENT (arc=0 leaked the same shape), now
> CURED by glibc→jemalloc (conductors flat ~2.1–2.9GB). So the alpha evidence here is void and the ablation is
> moot. The "per-node RAM ∝ corpus at full arc" scaling argument remains valid on first principles as a
> future-scale concern — but it was never demonstrated by the alpha OOM.
> Truth: .claude/data/conductor-leak-jemalloc-cure-verdict-2026-06-19.md · conductor-leak-rca-native-heap-reframe-2026-06-18.md


**The per-node memory problem is not a code leak — it is the authority-arc replication topology, and the fix is the sharding the design already specifies.**

## The through-line (write this down)

Today every alpha node runs the embedded Holochain/kitsune2 conductor at **`network.target_arc_factor = 1`** (full authority arc) — the value is set *nowhere* in any deployed config, so it defaults to full. At full arc **every node holds a DHT working set proportional to the whole corpus**: region/op-hash sets, arc-set diffs, the fetch pool, and per-round gossip op batches, all resident. So **per-node RAM ∝ total corpus**, and it grows as the corpus grows — on *every* node, forever. That is the "every node mirrors everything" model, and it is exactly what does not scale.

> **As corpus/network grows: engage arc-shrink (the sharding the tiered-quilt design already specifies) so per-node RAM stays bounded instead of tracking total corpus.**

With `target_arc_factor < 1`, a node holds a **bounded shard** of the DHT, not the whole thing. As peer count rises, each node's arc can shrink (it covers `~1/N` of the keyspace), so coverage is *shared* across peers while each one's resident working set stays flat. That is the same RS-sharded "no node holds everything" stewardship the tiered-quilt design specifies for the data plane — applied to the conductor's DHT working set. It is also the precondition for the protocol's stated floor that **a laptop (or a chromebook) is a full participant**: a lean device can hold a shard; it cannot hold a growing whole-corpus arc.

The corollary for operations: **stop treating per-node OOM as a RAM-sizing problem.** Adding gigabytes per node tracks the corpus upward and re-OOMs at the next ceiling (we are already several bumps deep — see below). The durable lever is the arc, not the limit.

## Evidence (2026-06-13)

Live Prometheus `container_memory_working_set_bytes` on the `elohim-node` container (image `1.0.0-dev-ee46afb1`), over ~80 min:

| node | load | trajectory | reading |
|---|---|---|---|
| matthew (proxy/read target) | heavy | 2.2 → 2.6 →⤴ 3.2 → 3.5 →⤴ 4.0 → **4.15GB**, still rising | climbing |
| jessica | heavy | 1.4 →⤴ 2.9 → 3.5 →⤴ 3.9 → **4.18GB**, still rising | climbing |
| james (largest corpus, 3654 docs) | heavy | 1.2 →⤴ 3.0 → 3.3 → **OOM**, ~9-min cycle for hours at 3Gi; bumped to 8Gi, already back to 3.3GB | climbing → re-OOM |
| adam (quiet bootstrap) | light | 1.5 → 2.0 → 2.1 → **2.2GB** | plateau |

The signature: **load-correlated, with periodic retained step-jumps** (~0.5–1.7GB allocated in one ~3–5 min interval and held). The *quiet* node plateaus ~2.2GB; *loaded* nodes don't plateau. james (largest corpus) OOMs worst — exactly what a full-arc, corpus-proportional gossip/op working set predicts.

A 4-agent code hunt across `elohim-storage` confirmed **no bulk-byte holder in our own Rust can produce a GB step-jump** — every payload path is disk-backed, dead-wired, or byte-trickle. The only structure matching the full signature is the conductor's DHT working set. (The conductor shares one cgroup with the storage process per the consolidated edgenode topology, so container metrics measure both.)

### This is the Nth bump — the breadcrumb was already in the manifest

`genesis/orchestrator/data/deployments.json` records the bump history per-human (each tied to a `deviceArchetype`):
- **jessica**: 1536Mi → 3Gi (2026-05-05) → 4Gi (2026-06-04). The 2026-06-04 `$comment` says verbatim: *"If 4Gi OOMs too, stop bumping and **profile elohim-node for a leak** (suspect list: inventory gossip, conductor pool churn)."*
- **james**: chromebook-edu floor (384Mi/768Mi) → 1536Mi (2026-05-22) → 3Gi (2026-06-03) → 8Gi (live, 2026-06-13).

This note **is** that profile. The answer is not the guessed suspects (inventory gossip / pool churn) but the broader fact: **full authority arc makes the conductor's resident working set track the corpus.** james is on the deliberately-lean `device-chromebook-edu` archetype — full-arc-on-a-growing-corpus is fundamentally mismatched with that archetype, and no RAM bump reconciles them. Arc-shrink does.

## The lever and its trade

`network.target_arc_factor` (kitsune2, in conductor-config.yaml; finer control via `network.advanced.coreGossip`/`coreFetch`):
- `1.0` = full authority arc (current default everywhere). Max replication: every node serves/holds its full arc. Max memory.
- `< 1.0` = bounded arc. Memory drops with the arc; coverage is shared across peers.
- `0` = leecher (holds/serves no authority). Minimum memory; contributes nothing to replication.

**This is a resilience↔memory trade and an operator/design decision — never a silent flip.** At the current 14-peer alpha with a small corpus, full arc is defensible (max redundancy on a tiny mesh). The scaling action is to **shrink arcs as the network and corpus grow**, so total coverage stays high (sum of arcs across many peers) while each node's resident set stays bounded. The open design work is the *policy*: how arc-factor is chosen per device archetype and adjusted as peer count grows (a chromebook-edu node should carry a much smaller arc than an operations node; both should shrink as N rises). `disable_gossip`/`disable_publish` are NOT options — they are `#[cfg(feature="test-utils")]`, absent from the production config schema.

## Open question: leak vs. bounded-large (must confirm before committing the lever)

Container `working_set_bytes` cannot attribute RSS between the conductor child and the storage parent, and "climbs forever (leak)" looks identical to "settles above the limit but the ceiling kills it first (bounded-large)." Three cheap, operator-side discriminators (no Pyroscope datasource is wired):
1. **Per-process RSS split** — in one `elohim-node` container, `ps -o rss,comm` for the `holochain` child vs the `elohim-storage` parent during the climb. Conductor dominates ⇒ this note's diagnosis holds; storage parent climbs ⇒ the hunt reopens.
2. **Raise one loaded node 3Gi → 6Gi and watch** — plateau below 6Gi = bounded-large (sizing/arc trade); climb past 6Gi = genuine leak.
3. **`target_arc_factor: 0` on one loaded node** — if the climb collapses, the conductor DHT working set is confirmed as the driver.

## Not the cause (fixed anyway, as hygiene)

Three *real* but minor unbounded structures in `elohim-storage`, all far too small/slow to be the GB OOM (each tagged "won't fix the OOM"): `services/provide_reconcile.rs` latch (insert-only `HashMap`, now pruned per pass), `p2p/mod.rs` `pending_epr_resolves` (the only `pending_*` map missing its `OutboundFailure` cleanup, now mirrored), `reconcile/controller.rs` `observed_kinds` (append-per-signal `Vec`, now gated to `#[cfg(test)]`). Latent/dead (cannot leak today; fix before wiring): `observation/log.rs` ObservationLog, `signals.rs` received_chunks, `conductor_agent_info_gossip` last_seen.

## Next steps

1. **Confirm** with the per-process RSS split (#1 above) — settles dependency-vs-our-code in one read.
2. **Stop the RAM-bump treadmill.** The live 8Gi james bump (and the mongo-pin) are stopgaps that revert on the next CI deploy; if mirrored into `deployments.json` they MUST carry a `$comment` marking them stopgaps with restoration tied to *this* work (per the file's established discipline), not silent permanent floors on lean archetypes.
3. **Design the arc-factor policy** — per-archetype arc targets + a shrink-as-N-grows rule, aligned with tiered-quilt sharding. This is the durable fix and the thing that keeps "a laptop is a full participant" true at scale.

Cross-refs: tiered-quilt-stewardship-design (the data-plane sharding this extends to the conductor working set); device-archetypes-design (the per-device floors full-arc violates); native memory `project_per_node_memory_is_conductor_authority_arc`; `project_hub_optional_floor`.
