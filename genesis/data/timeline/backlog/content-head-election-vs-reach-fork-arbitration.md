---
id: "backlog-content-head-election-vs-reach-fork-arbitration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "content_head election among genuinely-competing heads (sub-commons fork/merge/compete) — DEFERRED, and the reach ≠ head ≠ replication guard that must precede it"
slug: "content-head-election-vs-reach-fork-arbitration"
written: "2026-07-09"
author: "frontend-eyes-sprint (C3 refocus)"
status: "deferred"
priority: "medium"
area: "substrate/content-versioning-authority"
domain: "operator"
jobs: [elohim]
---

## Origin

Decomposed from a premature `elohim/elohim-storage/src/content_head_election.rs` (written + unit-passing, then removed 2026-07-09) that reached for "elect among competing heads" to explain the `elohim-host-landing` divergence. Operator ontology-guard corrected the framing: that divergence is NOT a legitimate fork awaiting arbitration — it is ONE commons-reach head that failed to replicate coherently (each doorway independently built + notarized its own bundle at deploy). Election was the wrong tool AND carried a reach↔head conflation. This item preserves the genuinely-future work and the guard, so neither is lost or re-conflated.

## The guard (must hold before ANY election work) — three orthogonal planes

Do NOT conflate these. Each answers a different question; each has a different governor.

| Plane | Question | Governed by | Landing-page truth |
|---|---|---|---|
| **Reach** | *Who may see this EPR?* (audience/visibility) | earned attestation/standing (amber → … → commons) | **commons** (the protocol's public face — correct) |
| **content_head** | *Which VERSION is canonical?* | `declare_content_head` (authority over THIS EPR's versioning) | exactly **one** head |
| **Replication / custody** | *How many peers serve the head's bytes?* | custody commitments, salvage | **both** doorways serve that one head |

- Earning commons reach ≠ declaring the head. An EPR at commons reach still has exactly one head; a private-reach EPR also has a head. The two axes are independent.
- `elohim-host-landing` is a **replication-coherence** problem (one head, fractured by per-host deploy build+notarize), NOT a head-election problem and NOT a reach problem. Its cure is *one build → one head → replicated* (replication plane + the single-notarized-head arc on dev), never arbitration.
- **C3 near-term is reach-CLEAN**: `resolve_head` serves *the* declared head (declaration-over-recency, single head). It must carry no reach coupling. See the plan `2026-07-01-crdt-content-dataplane-full1c-implementation-plan.md` C3.

## The deferred work — fork arbitration (only when sub-commons forking is REAL)

When peers below the commons head genuinely diverge/fork/merge/compete (the [[project_earned_reach_governance_pr_ceremony_vision]] world), there WILL be multiple legitimate competing heads for one id, and the network must elect one by EARNED authority. The removed module's sound kernel, to be rebuilt then (reframed, reach-clean):

- **Recency is never the tiebreak.** A lone/declared head elects; ≥2 competing undeclared heads surface as an explicit `Divergent` (honest "no earned head yet"), never silently last-writer.
- **The winner is chosen by earned authority** (reach-cohort edit-membership + author/community signature — plan C5, blocked-by-env on the DNA/notary pipeline), tended by Elohim, NOT by a deterministic lock (first-writer-wins is a dev convenience, not the goal — see the notary-authority feature prose).
- **`Divergent` escalates to the earned-authority election**, it does not resolve locally.

## Definition of done (for the deferred item)

1. A real sub-commons fork scenario exists (collaborative/forked authoring of one id) with an a2o scenario.
2. The earned-authority criterion (C5 reach-cohort + signature) is wired and DNA-verifiable.
3. Only THEN: rebuild the election decision as a pure, reach-clean function, fed by the earned-authority signal — with the three-plane guard above cited in its module doc.

## Blocked-by

Earned-authority substrate (plan C5) — `@requires:alpha-cluster-6peer` / DNA-notary pipeline. Do not build election before the fork scenario and the earned criterion both exist.

## SHARPENED 2026-07-10 — replication exonerated; the near-term gap is precisely located

Live re-probe of `elohim-host-landing` refines (not contradicts) the guard above: this is ONE intended head that failed to CONVERGE — reach-clean, and NOT a legitimate fork awaiting C5 arbitration.

- **Replication is NOT the gap.** Both blobs are present on both peers — GET the elohim.host bundle on alpha-A and the alpha-A bundle on elohim.host both return **200**. The 2026-07-09 "per-host deploy replication-coherence" framing is revised: the *bytes* are coherent. The split is purely in **which head each peer elects**.
- **Three located code gaps** (all near-term C3, reach-clean — none is fork arbitration):
  1. **Serve ignores the declared head.** The EPR/SSR serve path resolves the blob from the content row's local `blobHash`; `read_head_blob_hash` (the C3 serve-side read) is called **only in tests** — dead. So even a stamped head isn't honored.
  2. **The stamp is per-node.** `heal_content_one` (`projection_reconcile.rs:999`) stamps the head each peer's OWN conductor resolves — so per-node resolution **re-affirms each peer's own head** (the "never adopt the peer's value" no-op the notary-authority feature names). declaredHead is `null` live because no cross-node canonical head is ever declared.
  3. **No canonical head-binding exists** for the id — nothing tells elohim.host that alpha-A's version is canonical. `stamp_declared_head`/`declare_content_head` is the write primitive, unwired to a cross-node declaration.
- **Cure = C3 near-term** exactly as this item already scoped it: a single declared head-binding, honored on serve over any node-local authoring. NOT more replication, NOT reach, NOT arbitration.

## The upgrade-lifecycle trajectory (operator framing, 2026-07-10) — two tiers

The operator frames this as **the epic: how EPR artifacts version, upgrade, and elect their own head** — with genesis carrying the whole upgrade lifecycle for archetype EPRs. Two tiers on one trajectory:

- **Tier 1 (near-term, simplified) — steward-declared binding.** For genesis seed, the steward (e.g. adam for genesis content) declares the canonical head; all peers honor it. This is the reach-clean C3 `resolve_head` + a declaration act. Cures the landing regression durably without the full social-grant substrate.
- **Tier 2 (long-term trajectory) — self-electing supersession-lineage.** A new version declares it **supersedes** its predecessor (cross-root lineage → the version becomes the DAG tip), so any peer holding both elects it **deterministically** — no per-peer divergence possible. This is the "artifacts elect themselves" model; it composes with (does not replace) the deferred earned-authority fork arbitration for genuinely-*competing* (non-supersession) heads.

**Executable contract added** as the RED that defines "durable upgrade": `genesis/a2o/features/dataplane/notary-authority.feature` — Scenario *"An archetype EPR upgrade elects the new head everywhere and does not regress"* (`@wip` until the promote/re-seed steps + a test-persona archetype are wired; the no-regression clause IS the live "deployed once, regressed to old" bug). Genesis lifecycle coverage for archetype EPRs graduates this scenario `@wip → RED → green` as Tier-1 lands.

**Ingest prerequisite (landed 2026-07-10):** a head cannot be stamped durably if the seed sheds — the projector-backpressure catching-up shed (`ec5f0f522` seeder retry + `58c0f05d7`/`660fbbeb6` admission read/write split) is the rung this epic stands on. See [[project_reach_head_replication_distinct_planes]] and [[project_versioned_entity_head_is_declared_dependency]].
