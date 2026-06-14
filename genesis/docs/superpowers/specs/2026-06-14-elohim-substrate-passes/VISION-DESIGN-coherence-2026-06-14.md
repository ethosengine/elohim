# VISION DESIGN PASS — Collective Coherence: Convergence Across Fractal Stewards

> Working draft. NOT cite-sealed. A PROPOSAL for operator blessing — not a decision, not code.
> Escalates D5 beyond the tactical `2026-06-14-federation-coherence-plan.md` (edge-detector vs edge-reconcile).
> Author: rust-architect (truth layer). 2026-06-14.

---

## The escalation in one breath

The 2026-06-14 plan asks: *should the doorway DETECT divergence or RECONCILE it?* That is the wrong altitude. **Coherence is not an edge concern at all — it is a convergence property the substrate already half-has and half-fakes.** The Automerge CRDT plane (`src/sync/`) genuinely converges; the EPR-head plane (`src/p2p/mod.rs` Kademlia `put_record`) is **last-writer-wins, no merge, no quorum** — and *the EPR head is exactly what the doorway serves*. Two stewards of the same collective can serve divergent truth not because gossip is broken, but because **the thing the edge reads (the head) was never run through the thing that converges (the CRDT)**. That is a layering artifact, in the precise sense of the arc worked-example. The escalation is: **make the served head a convergent quantity, and govern its convergence policy (quorum/coverage) as an REA commitment.** The edge then only OBSERVES — correctly, because there is finally one truth to observe.

---

## 1. What the VISION REQUIRES here

North-star clauses in play, weighed by what coherence *requires* (not what is cheap):

- **"collectives continue to serve the humans that use it"** — REQUIRES that *any* steward of a collective serves the *same* truth. If matthew-edge and adam-edge serve different heads for `church/`, the collective has silently forked. Serving-humans is a coherence guarantee, not a routing detail.
- **"maintains that high-integrity of the Holochain DHT… build trust on the values negotiated through it"** — REQUIRES that the value/governance plane (small validated entries) converges with DHT-grade integrity. Trust is *built on agreement*; divergence with no detection is the erosion of the very thing the DHT exists to protect.
- **"fractal stewards… household to factory… governance contracts that set policies, enforce decisions"** — REQUIRES that convergence policy is itself *governed and fractal*: a household converges among 2–3 devices with a low bar; a collective among its co-stewards; a factory among many — each level setting its own quorum/coverage policy through a contract, not a hardcoded constant.
- **"capture-resistant stasis… against the real world, its externalities, and its messiness"** — REQUIRES that no single edge, doorway, or steward can BE the head. The moment "matthew's head is the truth because matthew is the read-proxy target" (the live A/B reality, see `project_doorway_ab_edge_islanding`), the system is captured by whoever runs that pod. Stasis means: the head is what the stewards *agreed*, recomputable by anyone, owned by no one.
- **"donut-like commons / value is minted"** — REQUIRES that coverage of the agreed truth is a *committed, reciprocal, minted* act: a steward who holds-and-serves the convergent head is performing care-economy work that should accrue standing — not a silent best-effort.

The vision does NOT require a central coherence authority. It requires the *opposite*: convergence as an emergent, governed, recomputable substrate property.

---

## 2. Is the substrate CAPABLE? Dig to WHY — the exact layer

**Two planes. One converges. One fakes it. The fake one is what the edge serves.**

### Plane A — CRDT doc sync: GENUINELY CONVERGENT (and underused)

- `src/sync/mod.rs:1` — "offline-first document sync using **Automerge CRDTs**." `SyncManager::apply_changes` (`mod.rs:74`) does `doc.load_incremental` — true CRDT merge, commutative, conflict-free, returns merged heads.
- `src/sync/doc_store.rs` — Automerge docs persisted in sled, tracked with `heads: Vec<String>` (`doc_store.rs:50`). This is a real Merkle-DAG-of-changes; two peers applying each other's changes provably reach the *same* heads regardless of order.
- It is **proactively driven**: `src/p2p/mod.rs:2203` `sync_interval = interval(60s)` → `mod.rs:2333` `initiate_sync_round()` → `mod.rs:6653` sends `ListDocuments`/`SyncChanges` to every connected peer. Both transport stacks carry it (`p2p/sync_protocol.rs`, `p2p_iroh/sync_backend.rs`). **This plane is capture-resistant by construction.**
- **WHY underused:** it carries "graph subgraphs, user state, content metadata" (`mod.rs:20`). It does **NOT** carry the EPR head. The collective's *served truth* never enters the one plane that converges.

### Plane B — EPR head: LAST-WRITER-WINS, the layering artifact

This is the exact-layer dig (the arc-pattern move):

1. **Config/served surface:** the doorway's `EprRouter` (`projection/epr_router.rs`) is seeded *only* from its own `STORAGE_URL` (federation-coherence-plan §1), refreshed every 30s. Each edge's head = whatever its one backing conductor last resolved. Island by construction.
2. **One layer down — the head's home is a bare DHT key:** `src/p2p/mod.rs:800` `PublishEprHead` → `put_record` on Kademlia keyed by EPR id; `mod.rs:806`. Resolution is `ResolveEpr` → `get_record` (`mod.rs:807`, `:1376`). **Kademlia `put_record` is last-writer-wins on the key** — the persisted `StoredRecord` (`kad_store.rs:18`) is a raw `{key, value, publisher}` with **no version vector, no merge, no quorum read**. Whoever wrote the key last, at whichever replica answers first, *is* the head. Two stewards republishing the same EPR id race; the loser's truth vanishes with no signal.
3. **What's missing (precisely):** the head is a *pointer that should be a convergent value*. The substrate already speaks the convergent language one module over (Automerge, `src/sync/`). What is absent is a **policy that routes the EPR head through CRDT convergence — or, where last-writer-wins is genuinely correct (single-author content), a quorum-read + coverage invariant so divergence is impossible to serve silently.**

This is *exactly* the arc finding: `target_arc_factor∈{0,1}` was a clamp over a continuous `DhtArc`; here, "the head is whatever my edge resolved" is a clamp over a substrate that already has a continuous, convergent merge engine. **Fractional arc is impossible / coherence is an edge problem** — both are layering artifacts, not physics.

### The bootstrap floor under both planes

`doorway/.../bootstrap/store.rs:5` — "**In-memory** storage… DashMap, fresh per boot, no cross-replica sync" (`project_doorway_kitsune2_bootstrap_protocol`). Even a perfect convergence policy can't converge peers that never discover each other. Convergence is gated by a **shared, persistent bootstrap** (F-BOOTSTRAP territory) — a hard prerequisite, not part of this design, but named so the dependency is honest.

**Verdict:** substrate is *capable* — the convergent engine exists and runs. The limit is a **missing convergence policy on the served-head plane**, plus a bootstrap floor. Both are fork/build candidates, not walls.

---

## 3. PATH / PIVOT / FORK LADDER (cheapest → deepest)

### Rung 0 — Edge divergence DETECTOR (the tactical plan; buildable NOW)
`2026-06-14-federation-coherence-plan.md`. Pure read over `EprRouter`: checksum `sorted[(url_path, epr_id)]`, cross-edge compare via the existing federation peer probe. **Cost:** ~1 day, Cat-C, no DHT, no new entry type. **Blast radius:** doorway only; zero substrate change. **Unlocks:** divergence becomes *visible* (today it's invisible until a human reads a build glyph). **Limit:** detects; never fixes. The collective still forks — we just *see* it. Necessary, insufficient. **KEEP IT** — it is the observability floor the deeper rungs report through.

### Rung 1 — Quorum-read on EPR head resolution (substrate, buildable NOW, no fork)
Change `ResolveEpr` (`src/p2p/mod.rs:1376`) from "first `get_record` wins" to a **bounded quorum read**: query R replicas, require agreement among ≥Q, surface `Divergent{candidates}` when they disagree instead of silently returning one. Kademlia already supports `Quorum::N` on `get_record`; this is *using an existing libp2p knob*, not forking. **Cost:** ~3–4 days in `p2p/mod.rs` + `p2p_iroh/epr_backend.rs` parity. **Blast radius:** EPR resolution path on both stacks; the doorway gets a real "divergent" answer to surface. **Unlocks:** the served head is no longer "whoever answered first" — it's "what Q stewards agree on." First real capture-resistance: no single replica decides truth. **Limit:** read-side only; doesn't *merge* concurrent author writes, just refuses to silently pick.

### Rung 2 — Route the EPR head through the CRDT plane (substrate, the vision-aligned build)
Make the per-EPR head an **Automerge document** in the existing `DocStore`, doc_id = EPR id. `publish_epr_head` becomes an Automerge change applied locally + announced over the *already-running* sync round (`initiate_sync_round`, `mod.rs:6653`). Concurrent steward updates **merge** instead of race; `apply_changes` (`sync/mod.rs:74`) returns the converged heads; the doorway serves the merged head. **Cost:** ~1.5–2 weeks — a new doc-type in `sync/`, a head-as-CRDT projection, wiring `publish_epr_head` into the sync engine, both-stack parity. **NO new DNA entry type** (DHT stays the notary of the *atom*; convergence of the *head pointer* is the libp2p/CRDT controller plane — exactly `project_principle_p1_reconciliation_controller`: DHT = manifest, libp2p = reconciling controller). **Blast radius:** EPR head lifecycle; the most load-bearing path in the system — stage behind a `TransportBackend`-style flag. **Unlocks:** divergence becomes *structurally impossible to serve* — any two stewards who have gossiped reach the same head. This is "collectives serve the same truth" as a substrate guarantee. The edge detector (Rung 0) now reports a quantity that *cannot* legitimately differ → a non-zero divergence is a hard alarm (partition/attack), not normal skew.

### Rung 3 — Govern convergence as an REA COVERAGE COMMITMENT (the operator-native pivot; NEW PRIMITIVE)
The deepest move, the arc-as-coverage-commitment isomorph. Convergence has a *policy* — what quorum, what coverage, who must hold the converged head — and that policy is **fractal and governed**, not a constant. Instantiate `project_rea_compute_commitment_primitive`: a steward emits a `Mishpat::Commitment` with action **`covers-head`** (new discriminator, NOT a new entry type — `signal_kind`/action extension per the primitive's own rule), scoped to a collective's EPR namespace, bounded by `{quorum: Q, coverage: ∪stewards ⊇ collective, freshness: ≤T}`. The **∪coverage = collective invariant** is enforced through the governance contract exactly as the arc finding proposed `∪arcs = full`. **Cost:** roadmap — a new REA action + bounds-validator row + projector; weeks, and a governance-design pass with qahal. **Blast radius:** Mishpat governance + bounds-validator; high-value, high-care. **Unlocks:** coherence becomes *negotiated, audited, revocable, minted*. A household sets Q=2; a factory sets Q=7 with geographic coverage; a steward who chronically serves stale heads accrues a `FeedbackSignal` and loses coverage standing. **This is the donut**: holding-the-convergent-head is care work that mints value.

### Rung 4 (note, not recommended now) — upstream contribution
A general "CRDT-head over Kademlia value" pattern is genuinely upstream-able to the iroh-gossip / libp2p ecosystem. On-mission, but premature; revisit after Rung 2 proves the shape in-tree.

---

## 4. RECOMMENDED ESCALATION (defended)

**Adopt Rung 0 now (it is already planned and free), commit to Rung 2 as the path, and commit Rung 3 as the roadmap pivot that makes Rung 2 governed.**

Defense:
- **Rung 0 alone is a trap dressed as progress.** A detector over a last-writer-wins head reports *legitimate* skew as if it were a problem, and can never tell partition from normal race. Detection only becomes *meaningful* once there is a single legitimate truth to detect deviation from — i.e. once Rung 2 lands. Ship Rung 0, but name it as the observability surface for Rung 2, not the answer.
- **Rung 1 is a fast, fork-free correctness win** worth taking *en route* to Rung 2 (quorum-read is independently valuable and small) — but it refuses divergence rather than resolving it. Good for single-author content where LWW is *almost* right; insufficient for collectives with multiple co-stewards.
- **Rung 2 is the vision-aligned path** because it makes "collectives serve the same truth" a *substrate property* — automatic, capture-resistant, offline-capable (Automerge is offline-first by design, honoring `project_hub_optional_floor`: two laptops in a village converge with no hub, no doorway). It reuses the convergent engine the substrate *already runs every 60s* — this is "write our policy on top of an engine that already speaks the quilt," the arc-pattern's rung-(b).
- **Rung 3 is the operator-native pivot** because coherence-without-governance is still a hidden constant (Q and coverage hardcoded somewhere). Governing it as an REA coverage commitment is the *same primitive* as arc-as-coverage-commitment and compute-as-commitment — **one substrate, now four instantiations: arc-coverage ≡ compute ≡ care ≡ head-coverage.**

### What this COMMITS US TO
1. **A new substrate capability (Rung 2):** EPR head as an Automerge CRDT document over the existing sync plane. In-tree, no DNA entry-type spend, both-stack parity, staged behind a flag. *Build commitment.*
2. **A new REA action `covers-head` (Rung 3):** a `Mishpat::Commitment` discriminator + bounds-validator row enforcing `{quorum, ∪coverage ⊇ collective, freshness}`. *Roadmap + new-primitive commitment* (governance-design pass required; do NOT solo-author the entry-budget question — `covers-head` is an action discriminator, NOT a new entry type, which is why it's affordable).
3. **F-BOOTSTRAP as a hard prerequisite** (shared persistent bootstrap; `bootstrap/store.rs:5` in-memory islanding must be closed first, else nothing converges). Named, not owned here.
4. **The detector (Rung 0) re-scoped** from "the coherence answer" to "the alarm surface for the convergence guarantee."

---

## 5. COUPLING — story + value + governance as one whole

This is the point of the whole pass: the technical move *is* the felt/economic/governance move.

- **STORY (felt):** "I steward the church collective. When I open it on my phone, and my co-steward opens it on her laptop in another city, we see *the same thing* — and when we both edit while one of us is offline in a tunnel, our changes *merge* when we resurface, nobody's work silently lost." Convergence-as-substrate is what makes a collective *feel* like one shared place instead of N diverging mirrors. Rung 2 delivers exactly this felt experience; Rung 0 only lets an operator *notice* when it's broken.

- **VALUE (the donut, minting):** holding-and-serving the convergent head is **care work**. Today it's an invisible best-effort (and worse — captured: matthew's edge is *the* read target, so matthew silently bears it for free, `project_alpha_substrate_probe_rails`). Under Rung 3, every steward who commits `covers-head` and stays within `{freshness, coverage}` bounds is performing minted, reciprocal care: standing accrues on-chain, default surfaces as a `FeedbackSignal`. The commons (donut) is the **union of coverage commitments** — value is minted by *keeping the collective coherent*, the most fundamental care a steward can give.

- **GOVERNANCE (policy, enforcement, capture-resistance):** the quorum Q and the `∪coverage ⊇ collective` invariant are **set by the collective's governance contract**, fractal by level (household Q=2, factory Q=7+geo). Enforcement is real: the bounds-validator walks every served head back to its `covers-head` commitment (mirroring `bounded_by` in `project_rea_compute_commitment_primitive`); a head served outside coverage *fails validation*. Capture-resistance is structural — **no single edge, doorway, or steward can BE the head**, because the head is a CRDT-merged, quorum-covered, governance-bounded value that anyone with the change-DAG can recompute and no one can unilaterally overwrite (closing the `put_record` last-writer-wins hole at `mod.rs:800`).

**The unifying claim:** coherence, when built right, is not a feature bolted onto the edge. It is the substrate *staying in stasis* — `coupled story + value + governance` such that the collective's truth is convergent (story), minted by coverage care (value), and bounded by a fractal contract (governance), holding a capture-resistant state against a messy, partitioning, adversarial real world. The edge merely *watches a truth it cannot author* — which is exactly where the doorway belongs (`doorway/CLAUDE.md`: "views served THROUGH not BY").

---

### Buildable-now vs commitment, marked

| Rung | Status |
|---|---|
| 0 — edge detector | **Buildable now** (already planned) — re-scope to "alarm surface" |
| 1 — quorum-read on `ResolveEpr` | **Buildable now**, fork-free (libp2p `Quorum::N`) — optional fast win |
| 2 — EPR head as CRDT doc | **BUILD COMMITMENT** — in-tree, staged behind a flag, both-stack parity |
| 3 — `covers-head` REA commitment | **ROADMAP + NEW PRIMITIVE** — governance-design pass with qahal; action discriminator, NOT a new entry type |
| 4 — upstream CRDT-head-over-Kad | Note only — revisit after Rung 2 |
