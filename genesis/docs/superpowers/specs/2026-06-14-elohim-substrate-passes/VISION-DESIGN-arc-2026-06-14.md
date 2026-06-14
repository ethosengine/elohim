# VISION DESIGN PASS — Fractal Arc: The Quilt-Tier DHT

**Date:** 2026-06-14
**Status:** PROPOSAL for operator blessing — working draft, NOT cite-sealed, NOT a decision, NOT code.
**Escalates FROM:** the tactical layer in `genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-auto-policy.md` (T1/T2/T3 verdict: "fractional arc is not available today") and `…-memory-scaling.md` (james OOM treadmill).
**Escalates TO:** the path the north star *requires*, not the lever that is cheap.

> North-star clauses this pass carries: **quilt-tier replicated dataplane · high-integrity Holochain DHT · fractal stewards & hubs (households→factories) · mutual compute agreements · governance contracts that set policy/enforce decisions · donut care-economy where value is minted · capture-resistant stasis.**

---

## 1. What the VISION REQUIRES here

The north star asks for a **quilt-tier replicated dataplane** that "maintains that high-integrity of the Holochain DHT" while still letting "hubs — households to factories — scale the sensemaking needed across the fractal stewards." Read literally against the substrate, that is a **contradiction at one layer that only resolves at another**:

- **High-integrity DHT** wants every validated entry witnessed and held redundantly — the trust/value/governance plane must not thin out, or the "trust built on negotiated values" erodes.
- **Fractal stewards (laptop → household → factory)** requires that a *lean* device be a **full participant** (`project_hub_optional_floor`: "one device, no hub; hubs are convenience, never gate participation") — but a lean device **cannot hold a whole-corpus authority arc**. James (3654 docs, the lean chromebook-edu archetype) OOM-flaps at 3Gi every ~9min because per-node RAM ∝ total corpus at `arc=FULL`.

The vision does not want "every node mirrors everything" (that is centralization-by-uniformity, capture-*prone* — the heaviest node sets the floor). It wants **differential stewardship**: a factory hub holds a large arc, a household holds a medium arc, a chromebook holds a small-but-real arc, and **∪ all arcs = full coverage**. That is the **fractal** in "fractal stewards." Arc-size *is* the technical face of "how much of the commons do you steward."

And critically — the vision wants arc to be **negotiated, audited, revocable** ("trust built on the values negotiated through it," "governance contracts that set policies, enforce decisions"). A node's coverage should not be a silent config default; it should be a **commitment** the mesh can see, account for, and hold to — exactly the REA shape the protocol already uses for compute, custody, and care (`project_rea_compute_commitment_primitive`). **Coverage is care.** A steward holding arc-range X for the commons is performing exactly the donut-economy act of "tending the commons so others can draw from it."

So the vision REQUIRES three things the `{0,1}` switch cannot give:
1. **Fractional, peer-aware arc** — each node targets `a ∈ (0,1)` sized to its resources and the live N.
2. **A coverage invariant** — `∪ arcs ⊇ FULL` enforced as a *governance contract*, not hoped-for.
3. **Two quilts, not one** — the validated trust-plane (small entries, wants near-full redundancy longer) and the heavy byte-plane (photos/blobs/corpus, RS-erasure-coded, CID-addressed) must be **sharded on different curves**. Conflating them is *why* lean nodes die: they are forced to hold the byte-weight of the corpus inside the DHT arc.

---

## 2. Is the substrate CAPABLE? — dig to the EXACT layer

**Verdict: the substrate ALREADY SPEAKS the quilt at the keyspace layer. The limit is a missing POLICY MODULE, gated by one hard clamp. This is a fork-the-policy candidate, not a wall.** Three levels, each read from real source:

### Level 1 — The keyspace is continuous (the quilt is native)
`DhtArc` is **not** a boolean. It is a continuous range:
```
kitsune2_api-0.4.1/src/arc.rs:14-26
  pub enum DhtArc { Empty, Arc(u32, u32) }
  pub const FULL: DhtArc = DhtArc::Arc(0, u32::MAX);
```
Any bounded shard `Arc(start, end)` is expressible *today*. `arc_span()` (arc.rs:161), `contains()` (108), `overlaps()` (138) all operate on arbitrary ranges. **The substrate's data model is already fractal.** There is no physics here forbidding `a=0.5`.

### Level 2 — The per-agent hint accepts ANY arc (the continuous lever exists)
```
kitsune2_api-0.4.1/src/agent.rs:69-76  (trait LocalAgent)
  fn get_tgt_storage_arc(&self) -> DhtArc;
  fn set_tgt_storage_arc_hint(&self, arc: DhtArc);   // "The sharding module will
      // attempt to determine an ideal target … may update to FULL or a true target"
```
The hint takes a `DhtArc` — a *range*, not a factor. The docstring is the upstream marker: a **sharding module** is *supposed* to compute a true target. **The seam is built; the module that drives it to anything but FULL is the hole.**

### Level 3 — Gossip (which owns the target) is a SWAPPABLE FACTORY, and the core impl is a STUB
```
kitsune2_api-0.4.1/src/builder.rs:64    pub gossip: DynGossipFactory,
kitsune2_core-0.4.1/src/factories/core_gossip.rs:8-16
  /// This factory returns stub gossip instances that do nothing.
```
kitsune2 is **modular by construction** (`Builder` exposes 14 `Dyn*Factory` slots; gossip is one). The real gossip/sharding logic lives in a *separate* `kitsune2_gossip` crate (the `k2Gossip` module — **not vendored in elohim-storage's tree**, confirmed: the registry holds only `kitsune2_api`/`_core`/`_bootstrap_client`). On join, `core_space.rs:451` sets `set_cur_storage_arc(DhtArc::Empty)` and gossip grows `cur → tgt`; the target the deployed `k2Gossip` chases is `DhtArc::FULL` with **no peer-count shrink**.

### The exact clamp — the one line that makes `{0,1}` the *effective* domain
The config field is the participation switch, not the size dial:
```
holochain_conductor_api-0.7.0-dev.21/src/config/conductor.rs:305-306
  #[serde(default = "default_target_arc_factor")]
  pub target_arc_factor: u32,        // default 1 (line 362)
```
and one layer down (`holochain_p2p-0.6.0/src/local_agent.rs`, cited by the auto-policy P0 spike — crate not in elohim-storage's local tree because storage embeds only `kitsune2_api/core`, not the conductor's `holochain_p2p`): `apply_arc_factor` **HARD-CLAMPS `factor > 1` to `1`** with the log *"not yet allowed until sharding is implemented."* So `{0, 1}` is the effective domain **at this field**.

### The config-key escape is REAL plumbing but currently DEAD-ENDS
`NetworkConfig.advanced` is genuine module-config plumbing, not a dead field:
```
conductor.rs:312-317   pub advanced: Option<serde_json::Value>,
conductor.rs:477-525   to_k2_config(): merges advanced + injects per-module keys
                       ("coreBootstrap"→serverUrl, "tx5Transport"→serverUrl/timeoutS/…)
```
and kitsune2 reads it per-module:
```
kitsune2_api-0.4.1/src/config.rs:126-137  get_module_config / set_module_config
```
**This is the decisive nuance the tactical spec under-weighted:** the *plumbing to configure a gossip/sharding module already exists and is exercised today* (tx5Transport gets its serverUrl/timeouts this exact way). The reason T2 reads "NOT-FEASIBLE" is **not** that the config path is missing — it is that **the deployed `k2Gossip` module exposes no `tgtStorageArc` key to read.** An invented key is silently dropped (the spec's correct warning). **So the gap is precisely: a gossip/sharding module that (a) reads an arc-policy key from `advanced` and (b) drives `set_tgt_storage_arc_hint` to a bounded range.** That is a *module*, and kitsune2's whole architecture is "supply your own module."

**Conclusion: NOT a wall. The substrate is fractal-capable at Levels 1–2; the only missing piece is a Level-3 policy module — and the slot for it is a first-class `DynGossipFactory`.**

---

## 3. PATH / PIVOT / FORK LADDER (cheapest → deepest)

| Rung | What | Cost | Blast radius | Unlocks for the vision |
|---|---|---|---|---|
| **R0 — Two-quilt split (byte-plane NOW)** | Stop forcing corpus *bytes* into the DHT arc. Heavy content (photos/blobs/corpus, >64MB → `rs-4-7`, `sharding.rs:28`) lives in the **RS byte-plane quilt**, CID-addressed, REA-custody-tracked (`custody-blob`/`serve-blob` already in `REA_ACTIONS`, lib.rs:254-255). The DHT holds only the *manifest* (validated, small). | **Buildable now** — `sharding.rs` exists; the work is *discipline* (keep blobs off the DHT) + projection. | elohim-storage only; no conductor/kitsune change. | Relieves the dominant RAM driver immediately; makes "lean device = full participant" *true for the trust-plane* even before fractional arc. The first quilt of "a quilt-tier's replicated dataplane." |
| **R1 — T1 leecher stopgap** | `target_arc_factor: 0` on the largest-corpus lean node (james). | Trivial config. | One node drops served coverage. | Buys RAM headroom **only while the mesh holds coverage** (§4 floor). Does NOT serve the fractal vision — a leecher is not a steward. Honest stopgap, named as such. |
| **R2 — `advanced` probe (verify the module surface)** | Before forking, *confirm* the deployed `k2Gossip`/`coreSpace` truly exposes no arc key, using `set_module_config` round-trip + discriminator #3 (set arc-via-key, watch working-set; the spike says it's dropped — re-verify on the 0.4.1 line we actually ship). | Low (a spike). | None. | Either finds a hidden key (→ no fork, pure config — *unlikely* per spike) or **proves the fork is necessary** with evidence. Cheap insurance against forking unnecessarily. |
| **R3 — CUSTOM kitsune2 gossip/sharding module (THE vision path)** | Supply our own `DynGossipFactory` (or a thin wrapper over the upstream `k2Gossip`) that reads an `elohimArcPolicy` key from `advanced` and drives `set_tgt_storage_arc_hint(Arc(start,end))` from `derive(mem_ceiling, archetype, N)` (the auto-policy spec's §3 function — *already designed*). This is **"write our policy module," not "fork Holochain."** kitsune2's `Builder.gossip` slot exists for exactly this. | **Medium** — a new crate (`kitsune2_elohim_gossip`) + wiring the conductor's `Builder` to use it. Carries a kitsune2-API version-tracking cost. | Conductor build + the K2 module graph. Self-contained; reversible by swapping the factory back. | **Fractional, peer-aware arc.** A chromebook holds `a≈0.1`, a household `a≈0.4`, a factory `a≈0.9`; ∪ = full. THIS is "fractal stewards." Lean devices become *contributing* stewards, not leechers. |
| **R3′ — holochain_p2p fork (only if Builder isn't reachable)** | If the conductor binary doesn't let us inject a custom gossip factory without forking, override `apply_arc_factor`/`get_tgt_storage_arc` in a `holochain_p2p` fork to honor a fractional target. | Medium-high — a maintained fork of a fast-moving crate. | Whole conductor; version-gated. | Same unlock as R3, heavier carry. **Prefer R3** (module injection) and treat R3′ as the fallback the spike (R2) decides between. |
| **R4 — Upstream contribution (on-mission)** | Contribute the sharding/coverage module upstream to kitsune2. The "not yet allowed until sharding is implemented" log is the literal upstream invitation. | High (community process, review). | Upstream + our fork retires. | Removes our carrying cost permanently; advances the *whole* Holochain ecosystem toward fractal DHTs. The protocol's values made into ecosystem infrastructure. Long-horizon. |
| **R5 — Arc-as-REA-coverage-commitment (the OPERATOR-NATIVE pivot)** | The *target arc a node chases* is the projection of a `Mishpat::Commitment` with a new action `commits-arc-coverage`. The node's `derive()` output is its *intent*; the **commitment is its promise**; `∪ committed arcs ⊇ FULL` is a **governance contract** (qahal-witnessed coverage invariant); under-coverage raises a `FeedbackSignal`; revoking a commitment shrinks a node's served arc — *audited, negotiated, revocable*. | **Layers on R3** — a `signal_kind`-style action add (NO new entry type) + a coverage projector/invariant. | DNA `REA_ACTIONS` array (+1 string) + a storage projection + a governance view. | **Arc becomes a negotiated value of the commons.** Coverage is no longer a silent default — it is a steward's *promise*, accountable through the same trust machinery as everything else. This is the vision's core move. |

---

## 4. RECOMMENDED ESCALATION (defended)

**Escalate to: R0 + R3 + R5, sequenced — with R2 as the fork-or-not decider and R1 as the only sanctioned stopgap.**

> **The headline is NOT "fork is too expensive, accept `{0,1}`."** The headline is: **the DHT is the trust/value/governance quilt and must NOT be forced to carry corpus bytes (R0, now); fractional arc is a kitsune2 POLICY MODULE we write into a first-class factory slot (R3), not a Holochain fork; and a node's arc is the projection of an REA coverage COMMITMENT governed by a ∪=full invariant (R5).** One substrate, three instantiations of the same primitive: **arc-as-commitment ≡ compute-as-commitment ≡ care-as-commitment.**

**Why this and not the tactical T1 stopgap:** the auto-policy spec correctly verified T2 dead and T3 "substantial." But it framed T3 as *a fork of Holochain* — a wall. Reading the real source reverses that framing: `Builder.gossip` is a **public factory slot**, `core_gossip` is an explicit **stub**, and `advanced`→`get_module_config` is **live module plumbing** (tx5Transport uses it every boot). kitsune2's authors built the seam for *exactly* a custom sharding policy. R3 is **"supply the module the substrate is waiting for,"** which is on-mission protocol work, not maintenance debt on someone else's core. The spec's own §3 `derive()` is the module's body, already designed — we are wiring an existing function into an existing slot.

**Why R0 first, unconditionally:** even perfect fractional arc on the trust-plane does not help if corpus *bytes* are inside that arc. The RS byte-plane (`sharding.rs`, rs-4-7, 64MB threshold) already exists and is REA-custody-aware. Splitting the two quilts is the **single highest-leverage RAM move** and needs no conductor change. It also makes the conceptual model honest: **DHT = notary (small validated entries), byte-plane = the heavy replicated dataplane** — the `project_principle_p1_reconciliation_controller` and `project_inventory_exchange_not_byte_replication` disciplines already point here ("inventory agreement ≠ byte replication").

### What this COMMITS US TO
1. **A new crate: `kitsune2_elohim_gossip`** (or `…_sharding`) — a maintained custom kitsune2 module behind `DynGossipFactory`. **This is a genuine FORK-CLASS commitment** (version-tracked against kitsune2's API), justified by being a first-class extension point, not a core patch. *Gate it on the R2 spike confirming no hidden config key on the shipped 0.4.x line.*
2. **A new REA action `commits-arc-coverage`** in `REA_ACTIONS` (lib.rs:224, currently `[&str; 25]` → 26). **NOT a new DHT entry type** — it rides the existing `Commitment` entry (lib.rs:1381), using `resource_quantity_value`/`unit` for arc-span, `in_scope_of_json` for `{start,end}`, `has_beginning/has_end` for the coverage window. Cheap, additive, reversible. (P2P-design-gate: this is the signal_kind-extension archetype, *not* a new-entry-type spend.)
3. **A roadmap item: the ∪arcs=full coverage invariant** as a qahal-governed contract + storage projection + `FeedbackSignal` on under-coverage. This is the governance half; it can land *after* R3 proves fractional arc works.
4. **An upstream watch (R4)** — track the "sharding is implemented" upstream marker; retire our fork to upstream when it lands. Stated as long-horizon, not a near commitment.

**Buildable-now vs commitment, marked honestly:** R0 (byte-plane split) and R1 (leecher stopgap) are **buildable now**. R2 (probe) is a **cheap spike now**. R3 (custom module) is a **fork-class commitment** gated on R2. R5 action-add is **buildable now** (one array entry + projection); the R5 governance invariant is a **roadmap item**.

---

## 5. COUPLING — story + value + governance as one cloth

This is where arc stops being plumbing and becomes the donut.

**Story (the felt).** The vision's floor — "a laptop is a full participant" (`project_hub_optional_floor`) — becomes *literally true*. Today a chromebook either OOM-dies holding the whole corpus or becomes a freeloading leecher (arc=0) that gives nothing back. With fractional arc it holds a **real, proportionate shard**: it *is* a steward, just a small one. The felt experience is "my modest device genuinely carries part of our commons" — dignity, not charity. A factory hub holding `a≈0.9` is *visibly* doing more of the sensemaking work, and that visibility is the point.

**Value (the donut, minted).** Arc-coverage is **care made measurable**. A steward committing to hold arc-range X for the window W is tending the commons so others can draw from it — the exact donut-economy act where value is minted. Because it rides the **same `Commitment` primitive** as `delegates-compute`, `custody-blob`, and the care-class streams, coverage flows into the **same care-class REA ledger** — and (per `project_compute_commitments_bounded`) stays **isolated from compute-class breach signals**: a coverage shortfall is a *care* signal (the commons thinned), never a *compute* placement gate. The donut's inner ring (the commons floor = the coverage invariant) and outer ring (resource ceiling = each node's `mem_ceiling` in `derive()`) are *both expressed in one unit: arc-span.* Coverage IS the donut, drawn in keyspace.

**Governance (capture-resistance + stasis).** `∪ committed arcs ⊇ FULL` is a **governance contract** — a qahal-witnessed invariant, not a hope. This is the capture-resistance mechanism: **no single node can be made load-bearing**, because coverage is a *distributed, negotiated, revocable* set of commitments. If a factory hub defects or is captured, its arc-commitment is revoked, the invariant detects the gap, and other stewards' commitments expand to re-cover — the system **actuates back into stasis** against the defection. That is precisely "stay in stasis when actuating a capture-resistant state against the real world, its externalities, and its messiness." The high-integrity DHT is *preserved* (validation unchanged; we only change *who holds which range*), so "trust built on negotiated values" is intact — and now the *coverage of that trust* is itself a negotiated value.

**The unifying claim:** arc is not a config knob. **Arc is REA coverage commitment.** One substrate primitive (`Mishpat::Commitment`), three faces: *compute* you lend, *care* you give, *coverage* you steward. The technical fork (a kitsune2 policy module) exists to make a **felt** truth (the laptop is a real steward), an **economic** truth (coverage is minted care), and a **governance** truth (coverage is a revocable contract enforcing capture-resistance) into one cloth — woven on the quilt the substrate already speaks.

---

### Source citations (read firsthand 2026-06-14)
- `kitsune2_api-0.4.1/src/arc.rs:14-26` — `DhtArc::{Empty, Arc(u32,u32)}`, `FULL`
- `kitsune2_api-0.4.1/src/agent.rs:69-76` — `LocalAgent::{get_tgt_storage_arc, set_tgt_storage_arc_hint}`
- `kitsune2_api-0.4.1/src/builder.rs:64` — `pub gossip: DynGossipFactory`
- `kitsune2_core-0.4.1/src/factories/core_gossip.rs:8-16` — gossip factory is a do-nothing STUB
- `kitsune2_core-0.4.1/src/factories/core_space.rs:451` — `set_cur_storage_arc(DhtArc::Empty)` on join
- `kitsune2_api-0.4.1/src/config.rs:126-137` — `get_module_config/set_module_config`
- `holochain_conductor_api-0.7.0-dev.21/src/config/conductor.rs:305-317,362,477-525` — `target_arc_factor:u32`, `advanced`, `to_k2_config` per-module merge
- `holochain_p2p-0.6.0/src/local_agent.rs` `apply_arc_factor` clamp (via auto-policy P0 spike; crate not in storage's local tree)
- `elohim/elohim-storage/src/sharding.rs:18,28,97` — RS byte-plane (`rs-4-7`, 64MB threshold, 4 data shards)
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:224-268` (REA_ACTIONS[25]), `1381-1410` (Commitment entry)
- `elohim/holochain/edgenode/conductor-config.yaml:24-36` — `network:` block has no `target_arc_factor` → defaults to FULL
