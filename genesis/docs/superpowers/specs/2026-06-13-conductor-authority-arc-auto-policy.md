---
id: conductor-authority-arc-auto-policy
status: design
created: 2026-06-13
class: substrate
artifact_kind: spec
written: 2026-06-13
cites:
  - conductor-authority-arc-memory-scaling | 2026-06-13-conductor-authority-arc-memory-scaling | sha256:0efe97d01b797d5d | path: genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md
  - tiered-quilt-stewardship-design | Tiered Quilt Stewardship | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - genesis/plans/2026-04-13-device-archetypes-design.md
---

# Conductor Authority-Arc Auto Policy

> ## ⚠ LEAK-CLAIM CORRECTED — 2026-06-19 (the arc-policy design + verified {0,1} infeasibility STAND)
> The "if it is a genuine leak, arc-shrink shrinks the structure that leaks" claim (and the
> leak-vs-bounded-large discriminator framing) is RESOLVED and falsified for the alpha OOM: it was a native
> glibc-malloc arena leak, arc-INDEPENDENT (arc=0 nodes leaked the same shape), CURED by glibc→jemalloc.
> Arc-shrink does NOT touch it. The forward corpus-memory-scaling design and the spike-verified "fractional
> arc infeasible on kitsune2 0.3.2/0.4.1 without forking holochain_p2p" finding are unaffected and stand.
> Truth: genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md · genesis/docs/content/elohim-protocol/history/2026-06-18-conductor-leak-rca-native-heap-reframe.md


## 1. Problem & through-line

Every alpha node runs the embedded Holochain 0.6 / kitsune2 conductor at `network.target_arc_factor = 1` (full authority arc). The value is set *nowhere* in any deployed config — `elohim/holochain/edgenode/conductor-config.yaml`'s `network:` block (lines 24–35) declares only `bootstrap_url`, `signal_url`, `enable_mdns`, `enable_relaying`, `webrtc_config`, so kitsune2 defaults to full. At full arc **each node holds a DHT working set proportional to the whole corpus** (region/op-hash sets, arc-set diffs, the fetch pool, per-round gossip op batches — all resident), so **per-node RAM ∝ total corpus** and grows forever on every node. This is the "every node mirrors everything" model, and it does not scale. Live evidence: james (largest corpus, 3654 docs, the deliberately-lean `device-chromebook-edu` archetype) OOM-flapped against 3Gi every ~9 min for hours; an 8Gi bump only moved the ceiling (sealed scaling note §Evidence, lines 31–38). We are several RAM bumps deep on a treadmill that re-OOMs at the next ceiling.

The answer is **arc-shrink**: hold a *bounded shard* of the DHT, not the whole thing. As peer count `N` rises, each node's arc shrinks toward `~1/N` of the keyspace while coverage stays whole across the mesh. This is the tiered-quilt data-plane sharding applied to the conductor's DHT working set, and it is the precondition for the protocol's stated floor that a laptop/chromebook is a *full participant* (`project_hub_optional_floor`): a lean device holds a shard; it cannot hold a growing whole-corpus arc.

This spec **formalizes** what two parent docs assert and does not contradict them:
- **Sealed scaling note** `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md` (commit 79e9ef506) — the lever/trade (lines 50–57), the evidence table (31–38), the Nth-bump breadcrumb (42–48), and the leak-vs-bounded-large open question (59–64). This spec is "Next steps" item 3 of that note (line 74): *"Design the arc-factor policy — per-archetype arc targets + a shrink-as-N-grows rule, aligned with tiered-quilt sharding."*
- **Self-healing vision §12** `genesis/docs/superpowers/specs/2026-06-13-actuatable-self-healing-control-plane-design.md` lines 302–314 (arc-shrink subsection). This spec turns its three assertions into code-shaped contracts: Pillar 3's `derive()` (§3 here), Pillars 1+2's coverage invariant (§4), Pillar 4's REA actuation (§5).

**It bounds the corpus-memory CLASS regardless of how the leak-vs-bounded-large question resolves** (scaling note line 279). If the climb is DHT-sync convergence, arc-shrink lowers the plateau; if it is a genuine leak, arc-shrink shrinks the structure that leaks. The discriminator (`ps -o rss,comm` conductor-child vs storage-parent; or `target_arc_factor: 0` on one loaded node) is the operator's confirm, not a precondition for this design.

## 2. Feasibility verdict — the lever is STATIC, and the *fractional* lever is VERIFIED NOT-FEASIBLE (T3 only)

**Lead finding: `target_arc_factor` is read once at conductor start. There is no runtime resize API.** The field is read from the YAML config at process spawn (`elohim/elohim-storage/src/conductor/process_manager.rs:64-66`, `Command::new(binary).arg("--config-path")…`); the only RPC surface `process_manager` connects to is `AdminWebsocket` (`holochain_client-0.9.0-dev.22/src/admin_websocket.rs:193-557`), which has **no** arc-resize / set-target-arc / network-config-update method. Changing the arc requires editing config and **restarting the conductor**. So the policy is **derive-at-startup + re-derive-on-restart**, never continuous live adjust. This is the baseline and we design to it.

**But the verdict has two levels the spec must not flatten.** The config field `target_arc_factor` is a `u32` *participation switch*, not a fractional size dial. Its effective domain is `{0, 1}`: `0` = leecher (Empty arc, holds nothing), `1` = full participant. `holochain_p2p`'s `apply_arc_factor` clamps `factor > 1` back to `1` with an error log ("multi-factor sharding isn't yet implemented"), and `< 1` is impossible (unsigned). The doc on the field confirms intent: *"leave this as the default 1 … For leacher nodes that do not contribute to gossip, set to zero"* (`holochain_conductor_api-0.7.0-dev.21/src/config/conductor.rs:302-306`, default `1` at line 362). The *arc itself* is runtime-dynamic — gossip grows a node's **current** arc toward its **target** as data is collected — but the target kitsune2 chases is always `DhtArc::FULL`; there is **no peer-count-based shrink** of the target in the deployed line.

**Consequence — and the actuation tiers, spike-verified.** A fractional `a` (our policy output, §3) cannot be expressed at the `target_arc_factor` field. The hoped-for continuous lever was the `LocalAgent` target-arc hint (`set_tgt_storage_arc_hint`, `kitsune2_api/src/agent.rs:69-76`) via `network.advanced`. **The P0 spike (2026-06-13) ran and VERIFIED that lever does not exist on any line available to us:**

| Tier | Mechanism | Granularity | Status (spike-verified 2026-06-13) | Fixes james? |
|---|---|---|---|---|
| **T1** | `network.target_arc_factor` | `{0, 1}` only | **VERIFIED — the only working lever** | No — full or leecher; leecher breaks the floor |
| **T2** | `LocalAgent` target-arc **hint** via `network.advanced` | fractional `a ∈ (0,1)` | **VERIFIED NOT-FEASIBLE** — no config path, deployed *or* on the 0.4.1 upgrade | — |
| **T3** | fork holochain_p2p / custom `LocalAgent`, or upstream kitsune2 sharding | fractional, N-aware | the ONLY fractional path; substantial | Yes, with carrying cost |

**The verdict, with the decisive evidence (VERIFIED).** holochain_p2p 0.6.0 `src/local_agent.rs` `set_tgt_storage_arc_hint` HARD-CLAMPS any `target_arc_factor > 1` to `1`, logging *"Received target arc factor > 1, this is not yet allowed until sharding is implemented!"*; `apply_arc_factor` yields only FULL (factor=1) or Empty (factor=0). No kitsune2 ModConfig (`coreSpace`, `k2Gossip`, dht constants) at **0.3.2 OR 0.4.1** exposes a target-arc key — `to_k2_config()` merges `advanced` only as URL/timeout module blobs. **The pending 0.4.1 upgrade does NOT unlock it**: `CoreSpaceConfig` is identically arc-less and the same aspirational *"FULL or a true target value"* docstring persists. So fractional arc requires **T3**: either upstream kitsune2 finally ships the sharding module (that "not yet allowed until sharding is implemented" log is the literal upstream marker to watch) or a holochain_p2p fork overriding `get_tgt_storage_arc` — a fork/version-gate, not a config change.

**The trap this closes (do NOT "validate by boot").** `network.advanced` is free-form `serde_json::Value`; an invented arc key (e.g. `coreSpace.tgtStorageArc`) is **silently dropped, not rejected** — the conductor boots clean and still chases FULL. A "config parses + pod starts" check would falsely read as success. The only honest confirm that the field moves memory at all is **discriminator #3**: set `target_arc_factor: 0` on one loaded node, watch the working-set climb collapse — which also doubles as the leak-vs-bounded discriminator (scaling note line 64).

**Design rule from this verdict (revised).** Everything in §3 (`derive()`), §4 (the coverage floor), §6 (the gauge), and operator override ships **now** — derivation/read-model/config work — and `derive()`'s computed fractional aim is a *signal* (the gauge shows "this node wants `a=0.5`") even when actuation is binary. But **on the deployed substrate the actuatable lever is T1 `{0,1}` only**: a rich always-on node stays the full-arc anchor; a lean/large-corpus node (james) can only be made an *accountable leecher* (arc=0, §5) — relieving its RAM but removing its served coverage, so it is a coherent stopgap ONLY while the remaining mesh holds the coverage floor (§4). **The clean fractional fix for lean-but-participating devices is T3 and is not available today.** Until then james's memory stays a RAM-stopgap-or-leecher choice, and `SAFE_MIN_ARC > 0` (a minimum *contributing* shard, §4) is not expressible. This is the honest ceiling on what arc-shrink can do on the current line.

## 3. The Auto-derive function `derive(mem_ceiling, archetype, observed_N) -> arc_factor`

A pure, deterministic, side-effect-free, re-runnable function (matching the vision's `derive()` contract, §3b/§96–126). It generalizes the **one verified-wired resource-derived path** in the tree — `render_semaphore ← compute_budget` (`doorway/doorway-service/src/render/capability.rs`, call site `main.rs:557-600`) — onto the corpus/memory axis. But it is **structurally different** from that precedent and copying its shape is the wrong-deliverable trap.

The precedent (`min_cpu_budget`, `capability.rs:47-56`) takes a **min of three ceilings** — all push the same direction (down). Arc-factor's inputs pull in **opposite** directions: memory is a *ceiling* (pressure → shrink), coverage is a *floor* (shrink too far → keyspace gap). So the shape is a **CLAMP**, not a min.

### Inputs and how they are read

- **`mem_ceiling`** — the cgroup memory limit of the container the conductor shares with storage. **M1 is now LANDED** (`system_metrics::container_memory_limit_bytes()`, commit `17b24df30`): reads cgroup v2 `/sys/fs/cgroup/memory.max` → v1 `memory.limit_in_bytes`, handling `max` / the v1 sentinel / a 1 PiB sanity ceiling, unit-tested. This was the hard precondition — `total_memory_bytes()` still returns HOST RAM (a 512Mi pod sees host GBs), so `derive()` MUST take the cgroup value, never `total_memory_bytes()`. Note: conductor + storage share **one** cgroup in the consolidated edgenode (scaling note line 40), so `memory.max` is the joint ceiling — `derive()` subtracts `storage_headroom_bytes` for the storage parent.
- **`archetype`** — from `genesis/orchestrator/data/deployments.json`, the `deviceArchetype` field (header `$comment`: the source of truth for per-human conductor budgets). The arc policy keys off the **archetype FLOOR**, not the current bumped limit: chromebook-edu `384Mi/768Mi` (`$chromebookFloor`), recycled-laptop `768Mi/3Gi` (`$recycledLaptopFloor`), family-node-base `1Gi/4Gi` (matthew `$comment` line 37). The bumped limits (james 8Gi, jessica 4Gi) are the *problem being solved* — the arc policy exists to make the lean floors viable again and end the bump treadmill. Capability gradient + per-archetype tuning: `genesis/plans/2026-04-13-device-archetypes-design.md` (levels 0–5; `target_arc_factor` is the missing corpus-axis row in its "Operational Envelope Per Archetype" table, lines 216–230, whose stated principle — "derive the right behavior from what the device reports about itself," line 230 — this generalizes).
- **`observed_N`** — the **conductor / kitsune2 DHT peer count**, NOT the storage libp2p swarm. Read from the conductor health response's `network.peer_count` (`elohim/elohim-storage/src/hc_client.rs:60`, parsed at line 337). This is load-bearing: `target_arc_factor` is a kitsune2 concept and the coverage invariant is over the conductor's DHT mesh. The storage libp2p swarm count (`p2p/mod.rs:1943` `connected_peers()`) and the lagging operational projections (`api/peer_statuses.rs`, `api/network_posture.rs`) are the **wrong** N and would silently break the coverage math.

### The formula

The function is **`derive(mem_ceiling M, archetype, observed_N, corpus_working_set C, local_authored_share L) -> arc_factor`**. It stays pure/deterministic by taking `C` and `L` as **sampled inputs**, not ambient reads: `C` (corpus DHT working-set size) and `L` (the node's own always-resident authored share) are sampled from storage/conductor stats at the call site (boot or re-derive event) and passed in. `M` is the cgroup ceiling (minus storage-parent headroom, since the two share one cgroup). Let `N` = conductor peer count, `a` = the node's authority-arc fraction (the policy output, `a ∈ [0,1]`), `R` = required distinct holders per key (the adopted `R_arc`, see §4).

```
  a_cov(N)        = min(1, R / N)                 # coverage floor — the HARD lower bound
  a_mem_permit    = clip01( (M − L) / (C − L) )   # memory-permitted ceiling, clipped to [0,1].
                                                  #   if L ≥ M (local share alone exceeds the
                                                  #   ceiling, e.g. james on a 768Mi floor):
                                                  #   a_mem_permit = 0 — no foreign arc fits.
  a_floor         = max(a_cov(N), SAFE_MIN_ARC)   # never below coverage, never a silent leecher
  target          = clip01( archetype_base_arc(archetype) / arc_share(N) )   # archetype + N set the aim

  if a_mem_permit < a_floor:        # coverage cannot be met within the memory budget — the james case
      a = a_floor                   # COVERAGE WINS — bias to over-replication (§4); do NOT shrink past it
      elevate("cannot meet mem budget at required coverage at N=<N>; "
              "needs more peers or a RAM allowance, not a smaller arc")
  else:
      a = clamp(target, low = a_floor, high = a_mem_permit)   # memory pushes down; coverage stops the fall
```

**Why not a single `clamp`:** when memory demands a smaller arc than coverage requires (`a_mem_permit < a_floor` — precisely the lean-device/large-corpus/small-N case, i.e. james), a naive `clamp(target, low=a_floor, high=a_mem_permit)` has `low > high`, which is undefined (`f64::clamp` panics). The degenerate region *is* the motivating scenario. The explicit branch encodes §4's "if shrinking would drop any key below `R_floor`, do NOT shrink — bias toward over-replication": **coverage wins and the node elevates a finding** (the runtime form of the operator's "add peers / this device is mismatched" signal) rather than shrinking into a keyspace gap. This is the formula-level statement of "the cure must not cause the partition." `archetype_base_arc`, `arc_share`, and `clip01` (clamp to `[0,1]`) are named design placeholders; their exact constants are tuning work, bounded by the invariants here. **The ordering is fixed: operator-override > Auto-derived > safe-floor** (vision §3c line 122, mirroring the precedent's `override_cfg.max_concurrent.unwrap_or(derived_default)` at `capability.rs:193-196`). An explicit operator-set `network.target_arc_factor` (or hint) in conductor-config always wins; Auto fills the gap when unset (everywhere today); the safe floor is the last-resort clamp. As in the precedent, **operator override may breach the coverage floor downward** (a deliberate leecher) — but only because it is accountable as a revocable REA commitment (§5), never a silent edit. Honest degradation: if `mem_ceiling` is unreadable (M1 not yet shipped) or `observed_N` is unavailable, `derive()` returns the **safe static default = full arc (1.0)** — the current behavior — never a guessed shrink. A mem-derived shrink is UNSAFE until M1 lands.

### The memory-bound math (why this scales)

With `N` peers each covering a uniform arc of fraction `a`, expected distinct holders per key `E[holders] ≈ N·a`. Holding coverage `≥ R` for every key needs `N·a ≥ R ⟹ a ≥ R/N` — the coverage floor `a_cov`. A node's foreign resident set `∝ a·C`. Substituting the floor: `resident ∝ (R/N)·C`. If corpus grows with peers (`C ≈ c·N`), then `resident ∝ (R/N)·c·N = R·c` — **bounded, independent of N.** That is the scaling win. Full arc (`a=1`) gives `resident ∝ C` — the current OOM trajectory.

**One refinement the spec must carry (the james bound):** the arc bounds only *foreign* authority. A node always holds its own source chain `L`, so `RAM ≈ L + a·(C − L) ≈ (1 + R)·(C/N)`. Boundedness survives (`∝ C/N`), but `L` stays resident *regardless of arc*. **Arc-shrink helps light-authoring lean devices far more than it helps james specifically** — james's 3654 authored docs are the largest `L` in the mesh. Arc-shrink is necessary but not by itself sufficient for the heaviest author; for james the policy can at most halve the foreign term (see small-N below), and his own corpus stays put.

### Small-N behavior (the 14-peer alpha) — corrected

Do **not** parrot "R/N ≈ 1 at N=14 forces full arc." Plug in the adopted target `R = 7`:

```
  a_cov(14, R=7) = 7/14 = 0.5      ← NOT ≈1.0
  a_cov(14, R=3) ≈ 0.21
  a_cov(14, R=2) ≈ 0.14
```

Durability **permits roughly half arc at the alpha.** What actually holds the alpha at full arc is that **the memory term is slack** — `C` is small enough that `a_mem ≥ 1`, so `a = 1.0` and `a_cov = 0.5` sits harmlessly below, never binding. The spec must say *full-arc-at-14 is permitted-but-unnecessary because memory is slack*, not *forced by durability*. The **shrink trigger at any N is memory pressure** (`a_mem < 1`), clamped from below by `a_cov`. The **james lever, concrete:** at fixed N=14, as james's corpus grows, `a_mem` drops and Auto shrinks his arc — but clamped at `a_cov = 0.5`. You can halve his foreign working set, **no further**, without more peers. To go below 0.5 you must grow N (N=70 ⟹ `a_cov = 0.1`; N=700 ⟹ `a_cov = 0.01`). That is the protocol-honest statement of "grow the network to lower the floor."

### Re-derivation

Runs at **boot** and on an **explicit signal** (a cgroup-limit-change event, or an arc commitment from §5) — **never** silently on every read, and **cooldown-gated** so a flapping `mem_ceiling` or churning `N` cannot thrash conductor restarts (vision §3 lines 124–126). Each re-derive that changes the value is actuated through §5, i.e. it triggers a controlled restart with the new computed arc.

## 4. The coverage invariant — the never-violate floor

> **For every key, the number of distinct nodes whose authority arc covers that key must stay ≥ `R_floor`. Equivalently, per-key `N·a ≥ R` for every key — not merely on network average. Auto may shrink a node's arc only while this holds for every key it would vacate.**

The floor is on **distinct holders**, matching the substrate's own live durability check `can_survive = parity.min(distinct_peers − 1)` (`elohim/elohim-storage/src/api/resilience.rs:215`) — peers, not copies. The conductor-DHT plane has **no pinned R of its own**, so the policy *adopts* one by the analogy the scaling note licenses (line 23 — RS-sharded stewardship applied to the conductor working set). The adopted parameters, honestly labeled:

- **`R_floor ∈ {2, 3}`** — the hard safe-floor. VERIFIED from the data-plane: `default_min_replicas() = 2` / `min_replicas_for_eviction: 2` (`elohim/elohim-storage/src/config.rs:215-216, 265`); `replica_health` flags `AtRisk` at `1..=2` and `Healthy` only at `≥3` (`graph_views/shefa/distribution.rs:45-48`).
- **`R_target = 7`** — the desired per-key redundancy, mirroring RS(7,4)'s `replica_target: 7` (`graph_views/shefa/distribution.rs:53`; shard split `data=4, parity=3` at `peer_capacity_service.rs:428-429`). Carry a churn margin `δ`: `R_target = R_floor + δ`, so transient peer-loss dips stay ≥ `R_floor`.

These are **recommended policy parameters, not a protocol mandate** — adopted from the data/durability plane (RS(7,4) erasure coding, `genesis/graphos/vocabulary.md:99-110`; the conductor DHT is a *separate plane* from this and from libp2p Kademlia's `kad_replication: 4` at `p2p/mod.rs:438`). The spec must say it is borrowing, not pretending the conductor layer already pins an R.

### The invariant is OPERATIONAL, not just a static inequality

The static `N·a ≥ R` is necessary but insufficient, because **actuation transiently zeroes a node's coverage.** On (re)join kitsune2 sets `set_cur_storage_arc(DhtArc::Empty)` (`kitsune2_core-0.4.1/src/factories/core_space.rs:451`) and grows the *current* arc toward target only as gossip collects data. Since the only actuation is restart (§2), **restarting a node to shrink it drops its served authority to nothing until re-convergence.** Therefore:

- **Stagger restarts. Never shrink multiple nodes at once.** A re-derive that shrinks node X is admissible only if the *remaining* mesh covers the whole keyspace at `≥ R_floor` for the entire window of X's reconvergence — treat X as a transient leecher during that window and require the others to cover the gap.
- **Bias toward over-replication on uncertainty.** If shrinking would (statically or during a reconvergence window) drop any key below `R_floor`, do **NOT** shrink. The failure-safe is always the larger arc.

### How a node knows coverage is met (the readability split)

- **This node's own `a`** — trivially local (its own derived/config value).
- **Collective coverage** (the invariant's numerator) — an **aggregate** the node cannot compute purely locally; it needs each peer's *current* arc on the wire. Two design-open paths: (i) extend the existing health-attestation channel — `record_health_attestation` (`doorway/doorway-service/src/services/federation.rs:275`; vision §2g already proposes adding a `capacity` field to that DHT entry) to also advertise `arc_factor`; or (ii) read kitsune2's own arc/coverage telemetry if 0.3.2 exposes one (TBD with the §2 spike). The local arc ships P0-cheap; the **collective-coverage gauge is the harder, negotiated piece (P2).** Until it exists, the conservative coverage proxy at P0 is `a_cov(observed_N) = min(1, R/N)` with the over-replication bias — i.e. trust only `N` and never shrink below `a_cov`.

**`SAFE_MIN_ARC > 0` depends on the §2 spike.** The intended safe floor is a *minimum contributing shard* (never 0 — arc=0 is a leecher, violating both coverage and the laptop-full-participant floor; the direct analog of the precedent's `worker_threads >= 4` / "never 0 workers" rail, vision §7 line 198). **But under T1 (`{0,1}`) the only sub-full value available is 0** — a contributing fractional floor is *not expressible* until the T2/T3 fractional lever is confirmed. State this dependency plainly: `SAFE_MIN_ARC` is a real rail only once fractional actuation lands; under a spike-fail T1 world, the floor degrades to the binary choice (full anchor vs accountable leecher).

**Precedence at the floor.** Auto may never breach `R_floor`. An **operator-override may** consciously breach it (an explicit leecher), but only because it is an accountable, bounded, revocable REA commitment (§5) — never a silent flip (scaling note line 57).

## 5. REA actuation model — a commitment, not an admin edit

Changing the arc is the **fulfillment of a `Mishpat::Commitment`**, not an admin key or a config toggle (vision §5 lines 150–171; `project_rea_compute_commitment_primitive`; canon `genesis/docs/architecture/rea-compute-commitment-primitive.md`). p2p-class **A — notarized**; CID = `entry_hash` per gospel (`project_mishpat_commitment_cid_is_entry_hash`). The commitment grants bounded/scoped/revocable authority; the actuation is recorded and projectable. Coordinator path: mishpat `create_commitment` post_commit → `MishpatSignal::CommitmentCommitted` → `mishpat_commitments` projection (`elohim/elohim-storage/src/main.rs:922`; parsed at `elohim/elohim-storage/src/mishpat_projection.rs:222-240`). Blast radius = exactly the granted, revocable scope; revoking the commitment (keyed on CID) instantly removes the authority.

### Why NOT an admin config edit

An admin YAML edit is silent, un-audited, irrevocable-by-record, and unaccountable — and arc-factor is precisely the knob that must NOT be flipped silently (scaling note line 57; vision §12 line 312: *"a resilience↔memory trade, never a silent flip"*). The commitment makes the trade visible, bounded, and reversible, and lets an on-device AI agent (the real target, vision §5 line 162) actuate within a grant rather than holding a root key.

### The VERIFIED schema gap — present the fork (parent author decides)

§5 of the vision claims the actuation scope "rides the commitment's `scope` field (e.g. `scope: \"doorway.warmup.timeout\"`)." **The shipped schema does not support a knob.** In `elohim/sdk/schemas/v1/commitments/delegates-compute.schema.json`: `scope` (lines 23–26) is *"the class of EconomicEvent the recipient is authorized to emit"* (examples `republish-epr`, `serve-url-projection`) — a content/economic-event class, not a knob namespace. `bounds` is `required` (lines 38–46) with four required sub-fields — **`epr_scope`, `reach_ceiling`, `rate_per_hour`, `rotation_ttl_days`** — every one a content-publishing concept, enforced by the Rust validator (`elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs:467-490`). There is **no slot** for a numeric topology/memory knob, no "value", no "knob_id". So the vision's `scope: "conductor.arc_factor"` is aspirational. Three options:

- **(a) New action discriminator** — `sets-authority-arc` (or `tunes-resource-knob`), reusing the existing `Commitment` DHT entry type (no new entry type — consistent with the primitive's "one entry, action discriminator distinguishes" rule, schema `$id` line 5), with its own bounds shape `{ knob: "conductor.target_arc_factor", min, max, coverage_floor, valid_until }`. Cleanest semantically.
- **(b) Extend `delegates-compute`** via its `bounds.additionalProperties: true` (schema line 47), leaving the four required content fields vestigial/sentinel. Least-invasive, semantically muddy (a memory knob carrying a `reach_ceiling`).
- **(c) A distinct commitment kind entirely.**

This also resolves vision §11 open-question #7 (canonical `scope` string namespace for knobs) — but it is **more** than a vocabulary task, because the bounds object itself has the wrong shape for a knob. Recommendation: **(a)**, but the parent author owns the call.

### The actuation contract (template: `WriteThroughState`, `elohim/elohim-storage/.../write_through.rs:233` — the cleanest runtime-mutable pattern in the tree)

1. A commitment authorizing an arc set within `{min, max, coverage_floor, valid_until}` is created and notarized.
2. `derive()` (or an operator/agent) proposes a new `a` *within those bounds*.
3. The proposed `a` is checked against the **coverage invariant operationally** (§4): both the static `N·a ≥ R` for vacated keys AND the staggered-restart rule (no concurrent shrink; remaining mesh covers the reconvergence window). **If shrinking would open a keyspace gap below the floor, the actuation is REFUSED and a finding ELEVATED** — the exact analog of quarantine's anti-self-partition refuse-and-elevate (vision §2h line 86, §7 line 197).
4. On pass, the new arc is written to conductor-config and the conductor is **restarted** (the only actuation path, §2), staggered across the mesh.
5. The fulfillment is recorded against the commitment CID; the §6 gauge reflects the new value and its "why".

### Who may actuate (must respect the dual: node-local RAM AND collective coverage)

Arc-factor is **unlike every other §5/§8 knob**: CPU/warm-up knobs are purely node-local (over-tightening starves only your node). Arc-factor is **dual** — shrinking your arc frees your RAM *and* removes keyspace coverage the mesh relied on.

- **node-self / node Auto** — may shrink/grow *only within the coverage floor it can prove safe*; unilateral shrink below the floor is forbidden. Arc-shrink-below-floor is a **negotiated** action (peers coordinate so no gap opens — Pillar 1 over storage topology, vision §12 line 311), not unilateral. This is the operational meaning of "negotiated quantity."
- **operator** — override-wins (§3c precedence); may set arc below the local-safe floor as a deliberate trade, with the coverage invariant still elevating-and-refusing if the *mesh* would partition.
- **future controls-UI** — a consumer of the same typed actuation surface; add `set_arc_factor(value)` / `re_derive_auto()` to the §5 tool list (vision lines 162–170).
- **on-device AI agent** — the real target, scoped by its `delegates-compute` (or §5(a) successor) grant.

## 6. Observability — arc-factor + coverage as a readable projection

`/api/v1/peer-statuses` does **not** answer this: it is served (`elohim/elohim-storage/src/http.rs:11029`, `list_peer_statuses`) and carries peer health + `last_seen` (vision §2a) but **no arc or coverage**. So this is a **NEW Cat-C node-local gauge**, sibling of the proposed `/admin/auto-preset` read-model (vision §3f line 134, §6 line 184) and `/admin/capability`. It surfaces the §3f shape, with the derivation "why" first-class:

```json
{
  "resources": { "mem_limit_bytes": 3221225472, "corpus_docs": 3654, "observed_N": 14 },
  "derived":   { "target_arc_factor": 0.5, "lever_tier": "T2-hint" },
  "overrides": {},
  "coverage":  { "r_floor": 3, "r_target": 7, "a_cov_floor": 0.21, "mesh_coverage": "unknown@P0" },
  "reasons":   [ "a=0.50: clamped at coverage floor a_cov(N=14,R=7)=0.5; a_mem would permit lower as corpus grows",
                 "archetype=device-chromebook-edu floor 384Mi/768Mi; current limit 8Gi is the bump being unwound" ]
}
```

The "why" string is the design requirement (vision §3f line 134): the agent/UI/operator must see the *derivation*, not just the value. **Two readability tiers, matching §4:** this node's own `target_arc_factor` is locally readable and ships P0-cheap; **collective `mesh_coverage` is an aggregate** needing peer-arc advertisement on the wire (the `record_health_attestation` extension or kitsune2 telemetry), so it is P2 and reads `unknown@P0` until then. Pair the gauge with M4's working-set-trajectory → elevate (vision line 295) so a monotonic climb-toward-ceiling auto-files the finding the operator caught by hand at 1am.

## 7. Phasing (mapped to the self-healing P0/P1/P2)

- **P0 — safe static derive + coverage floor + operator override + the precondition.** Ships **regardless of the §2 spike outcome**:
  - **M1: the cgroup MEMORY reader** (`/sys/fs/cgroup/memory.max` v2 + v1) in `system_metrics.rs` — the gate (`total_memory_bytes()` reads host RAM today). Same as vision §9 #13 / §11 #5; vision §12 puts it P0-adjacent (line 298).
  - `derive()` wired to the **safe static default = full arc**, returning a shrink only when M1 is present *and* the §2 spike has confirmed a fractional lever. Operator override honored. The **coverage floor `a_cov = min(1, R/N)`** as the conservative proxy (over-replication bias, no mesh-coverage gauge yet).
  - The **§6 local-arc gauge** (own value + "why").
  - **M4: working-set-trajectory → elevate** (vision line 295) — auto-files the climb.
  - **The §2 P0 SPIKE:** confirm whether the `network.advanced` `LocalAgent` target-arc hint (T2) works on the deployed kitsune2 0.3.2. Run the two cheap confirms first (discriminator #3 `target_arc_factor: 0` to prove the field moves the resident set; read the vendored 0.4.1 `network.advanced` surface to bound the spike).
- **P1 — REA actuation + per-archetype Auto + observability deepening** (rides the vision's `derive()` build, P1 item 2, lines 254; actuation spine is P1 item 1 / map #12, line 253):
  - The §5 actuation contract — the `sets-authority-arc` (or extended `delegates-compute`) grant/scope/revoke + typed tool surface, templated on `WriteThroughState`, with the operational coverage check (refuse-and-elevate) and staggered restart.
  - Per-archetype arc targets in the §3 formula (the `archetype_base_arc`/`arc_share` constants); `target_arc_factor` becomes the missing row in the device-archetypes "Operational Envelope" table.
  - `SAFE_MIN_ARC > 0` becomes a real rail **iff the spike passed** (T2/T3 fractional lever live).
- **P2 — negotiated collective coverage + dynamic adjust if/when kitsune2 supports it** (vision P2, lines 258–261):
  - The **collective-coverage gauge** — peer-arc advertisement on the wire (`record_health_attestation` `arc_factor` field, or kitsune2 telemetry) so a node can verify the mesh invariant, not just its local `a_cov` proxy. This is the genuinely-negotiated piece.
  - **Dynamic adjust** — only if a future kitsune2 exposes a runtime arc-resize API (none today, §2). Until then, "adjust" = re-derive-on-restart, hysteresis/cooldown-gated, staggered. The hysteresis-band design lives here, dormant, until the API exists; do not design continuous live adjust against a runtime that has no resize path.

## 8. Open questions & risks

- **RESOLVED (the §2 spike, 2026-06-13): NO fractional arc lever exists** — not on deployed holochain_p2p 0.6.0 / kitsune2 0.3.2, not on the 0.4.1 upgrade. `target_arc_factor` is `{0,1}` only; `network.advanced` exposes no target-arc key (and silently drops an invented one). Fractional `SAFE_MIN_ARC` and per-archetype fractional targets are therefore **not actuatable today** — the policy runs T1 (full-anchor vs accountable-leecher) while `derive()`'s fractional aim serves as the gauge signal. **The remaining watch-item is T3**: upstream kitsune2 shipping sharding (watch for the removal of the "not yet allowed until sharding is implemented" guard in holochain_p2p `local_agent.rs`) or a deliberate holochain_p2p fork. Everything else (`derive()`, coverage floor, gauge, operator `{0,1}` override) ships regardless.
- **WATCH-ITEM — T3 trigger:** when holochain_p2p drops the `target_arc_factor > 1` clamp / kitsune2 exposes a `coreSpace` (or sibling) target-arc config key, the fractional path opens and §3's computed `a` becomes directly actuatable. Until then, re-run the spike on each holochain/kitsune2 bump.
- **Restart transiently zeroes coverage.** The only actuation is restart, and a rejoining node starts at `DhtArc::Empty` and grows via gossip (`core_space.rs:451`). The invariant is therefore operational — stagger restarts, never shrink concurrently, require the remaining mesh to cover the reconvergence window (§4, §5). Mis-handled, the cure causes the partition.
- **Hot-key skew.** The arc floor is a *durability* floor, not a performance ceiling. Hot keys want *more* than `R` holders for read throughput; the Auto-derive must **not** shrink a hot key's coverage on a memory argument. Hot-key detection feeding an upward arc adjustment is out of scope here (P2-adjacent).
- **Non-uniform arc placement.** kitsune2 arcs are ring intervals, not perfect partitions; `E[holders] ≈ N·a` is an average. A network-average `N·a ≥ R` can pass while a gap key sits below `R`. Enforce **per-key** coverage, not the average — which depends on the P2 collective-coverage gauge to verify properly.
- **Churn at small N.** At N near R, losing one peer is a large coverage fraction; transient dips can drop a key below `R_floor`. The churn margin `δ` (`R_target = R_floor + δ`) and the over-replication bias absorb this; it reinforces full-arc-at-small-N on *robustness* grounds, independent of memory.
- **The `corpus ∝ N` assumption breaks at the alpha.** The alpha is **fixed N≈14 with corpus climbing** (the james OOM is exactly this — more content, not more peers). Under super-linear corpus growth or hot authors, `C/N` grows and arc-shrink alone cannot hold the floor below `a_cov`; only *adding peers* lowers `a_cov`. The honest operational statement: at fixed N, Auto can shrink james only to `a_cov = 0.5`; deeper relief needs network growth.
- **The local-authored `L` term bounds the win for heavy authors.** james's 3654 authored docs stay resident regardless of arc. Confirm `L` against kitsune2's actual working-set composition before treating `(1+R)·(C/N)` as exact; the structural claim (arc-shrink helps light-authoring lean devices more than heavy authors) holds either way.
- **The REA schema fork (§5) is unresolved** — (a) new discriminator / (b) extend `delegates-compute` / (c) distinct kind. Parent author decides; the bounds object needs the right *shape* for a knob, not just a new scope string.
- **Leak vs bounded-large is still open** (scaling note 59–64) — but this policy bounds the class either way (§1). Do not let the spec read as a settled leak diagnosis; the `ps -o rss,comm` / `target_arc_factor: 0` discriminators are the operator's confirm.

## 9. P2P Design Gate

Run because this spec proposes a commitment action, a route, and a wire message. All three pass; the two that touch the DHT are **entry-type REUSE** (no new entry types), so DNA capacity is not engaged (Mishpat 11/~100, Infrastructure 6/~100 have headroom regardless).

### Entity: Arc-set actuation (commitment)
- **Classification**: Notarized (A). A bounded/revocable authority grant — the protocol would be lying if it were silently changed; must be witnessable and revocable.
- **Address**: Content-Derived — `CID = entry_hash` per gospel (`project_mishpat_commitment_cid_is_entry_hash`); revoke/bounds-gate key on the CID. NOT slug/UUID (returning a non-entry-hash CID silently breaks every bounds-gate per that memory); NOT agent-scoped-composite (it is a notarized grant, not a private stance).
- **Source of truth**: Holochain DHT (mishpat). **Entry-type REUSE** — the existing `Mishpat::Commitment` entry with a NEW action discriminator `sets-authority-arc` (no new entry type), consistent with the primitive's "one entry, action discriminator distinguishes" rule.
- **Coordinator / signal / projection**: `mishpat::create_commitment` → `MishpatSignal::CommitmentCommitted` → `mishpat_commitments` (dht_anchor_hash: yes; CID=entry_hash, action_hash=dht_anchor_hash per the gospel).
- **HTTP route**: none new for mutation — actuation rides the existing commitment-create path.
- **Anti-pattern check**: none (no new entry type; CID=entry_hash not UUID; revoke keyed on CID). **One genuine design-open carried in §5**: the `delegates-compute` bounds shape has no slot for a knob → the action needs a knob-shaped bounds (recommended option (a), a new `sets-authority-arc` bounds schema on the reused entry). That is a schema-shape decision within the existing entry, not a new entity.

### Entity: Arc/coverage observability gauge
- **Classification**: Operational (C). Reconstructable from live state (cgroup mem read + `observed_N` + corpus stats + conductor config = the `derive()` inputs); a node-local diagnostic, no community-witness need.
- **Address**: Slug/UUID — justified: operational endpoint, no content to hash (a stats read-model, sibling of `/admin/auto-preset`, `/admin/capability`).
- **Source of truth**: SQLite/in-memory operational; **no `dht_anchor_hash`**. Reconstruction strategy: recompute `derive()` from cgroup `memory.max` + conductor `network.peer_count` + corpus working-set + config.
- **Coordinator / signal**: none (operational read; no zome).
- **HTTP route**: `GET /api/v1/<arc-status>` (read-only, serves the projection) — designed LAST, the thinnest layer (§6).
- **Anti-pattern check**: none (explicitly Cat-C with a documented reconstruction strategy).

### Entity: Peer-arc wire advertisement
- **Classification**: Notarized (A) — **entry-type REUSE**: extend the existing `record_health_attestation` DHT entry (infrastructure zome; vision §2g already proposes a `capacity` field) with an `arc_factor` field. Peers rely on the advertised arc to verify the coverage invariant, so it must be witnessable.
- **Address**: inherits the health-attestation entry's existing identity (per-agent self-claim); no new identity.
- **Source of truth**: Holochain DHT (infrastructure). Projection: `peer_statuses` (dht_anchor_hash: yes).
- **Coordinator / signal / projection**: `infrastructure::record_health_attestation` (existing, add `arc_factor`) → health signal → `peer_statuses`.
- **HTTP route**: served via the existing peer-statuses surface (the collective-coverage read is the §6 P2 gauge).
- **Anti-pattern check**: none (field addition to an existing entry, not a new type).

### Design constraints discovered
- **No new DHT entry types** — both notarized surfaces reuse existing entries (Commitment + health-attestation). DNA capacity is not engaged.
- **M1 (cgroup memory reader) is the hard precondition** for the operational gauge and for any mem-derived shrink (§3 — `total_memory_bytes()` reads HOST RAM today).
- **Ordering honored**: coordinator/entry-shape first (mishpat action + bounds; infra health field), then signal/projection, then the read route last (§7 phasing already follows this).
- **The one open schema decision** (commitment bounds shape, §5 fork) is the parent author's call; it does not change any classification here.
