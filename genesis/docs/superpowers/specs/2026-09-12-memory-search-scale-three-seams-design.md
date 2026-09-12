---
title: Memory, search and scale — an index is a measure, reach at replication, pool folds on arcs (the three seams)
id: memory-search-scale-three-seams-design
status: proposed
class: protocol-canonical
serves: recall-reaches-authority
date: 2026-09-12
requires_env: household-nodes
context-tier: disclosed
steward: agent:orchestrator@claude-fable-5-1
graduation-trigger: the governed-discovery plan's station 3 landed and the standing reader running; then station 4 (native providers) is scheduled from §8 as a plan, and this spec's seam register is re-read against the tree before any row is picked
cites:
  - "governed-discovery-journey-lens-graduation-design | the carrier: four roles, five seams, reader lens, Provider trait and the L0→L3 ladder this spec's three seams sit inside; this spec adds providers, index-as-measure, reach at replication and the pool fold, never a new role | sha256:77030654da24c3e0 | path: genesis/docs/superpowers/specs/2026-09-11-governed-discovery-journey-lens-graduation-design.md"
  - "governed-discovery-stations-0-3-plan | stations 0–3 in flight; this spec registers stations 4–9 for its successor and is not scheduled until station 3 lands | sha256:a0be77a4bc3f6dc7 | path: genesis/docs/superpowers/plans/2026-09-11-governed-discovery-stations-0-3-plan.md"
  - "middot-measure-primitive-design | the measure primitive an index instantiates — IndexMeasure is a middot measure whose fold cache is the shard and whose fold receipt is a FoldAttestation | sha256:336ab2b4619b9144 | path: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md"
  - "reach-ontology-vocabulary-split-spec | the eight-ring vocabulary every index bound (reach ceiling, pool floor) and every replication grant is expressed in; the household is named by ring, no new vocabulary | sha256:2a1ef52c1ced3c48 | path: genesis/docs/superpowers/specs/2026-07-22-reach-ontology-vocabulary-split-spec.md"
  - "sense-respond-governance-classifier | the frame/intent classifier the AttentionTending classification variants (ValuesForward, Fatigue, ScopeMismatch, Safety) inherit from; private chain only | sha256:c716a519ee6cc953 | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md"
  - "eprfs-witnessed-interaction-primitive | the witnessed-event primitive custody heat rides — which shards are drawn from is counted as care, who drew is never recorded above the requester's holon | sha256:6a24773ffd7b83f4 | path: genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md"
  - "plural-mishpat-lenses-over-epr-design | lenses plural by construction; the reader lens bound at retrieval is one lens over the same atoms, never a collapse of standing into a score | sha256:ab0055896398ef95 | path: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md"
  - "quilt-evidence-temperature-composition-design | the never-sum-into-an-unlabeled-number law: rank fusion for order under a recipe CID is allowed, standing never enters the fusion | sha256:d278e960b5c8a15d | path: genesis/docs/superpowers/specs/2026-07-24-quilt-evidence-temperature-composition-design.md"
  - "digital-memory-standing-ontology-design | EpistemicStanding, which gains bi-temporal as_of and demotion-never-deletion here; standing prints as a shape on every candidate | sha256:0b6b222a8cf4745f | path: genesis/docs/superpowers/specs/2026-08-05-digital-memory-standing-ontology-design.md"
  - "native-content-graph-seam-design | the native ContentGraphResolver the graph provider sits behind — no Cozo/Kuzu, one more impl behind the trait | sha256:d03683cd30aef91c | path: genesis/docs/superpowers/specs/2026-06-08-native-content-graph-seam-design.md"
  - "unified-memory-loop-design | the collective-memory owner split; the household is the smallest collective and the palace is a visitor, never an owner | sha256:07e941a325cc49c2 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md"
  - "mutual-storage-replication-dwelling-hub-design | replicates-* commitments, custody and ReplicationStatus that the pool's arc-sharded folds, replication factor per arc and hedged requests ride on | sha256:1acbeeec8b7a3956 | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md"
  - "trust-compute-gradient-brainstorm | the compute/trust gradient under which custodians hold shards under delegates-compute grants; high-trust edges fast, commons browsing witnessed-safe | sha256:89c493c73ff6b06b | path: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md"
  - "elohim-seam-map-concern-routing | the atlas that routes each concern: providers are bridges, the lens is at the client/SDK seam, the recipe at the SDK seam, custodians on T1/T2, the doorway a T4 projection | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "private-thought-governed-fruit | journeys, receipts and AttentionTending never enter a fold that can leave the device; only attested outcomes and tier digests cross into a pool | sha256:5b6f5cdb858277e4 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "search-epic | the vision this spec makes practical: found not crawled, the index as a pool, amplification earned, standing as a shape, the fruits held in trust | sha256:7ca54d18d954aae4 | path: genesis/docs/content/elohim-protocol/search/epic.md"
  - genesis/docs/content/elohim-protocol/living_memory/epic.md
  - genesis/research/local-first-to-council-memory-search-seams-2026-09-12.md
  - genesis/research/commons-data-pools-hot-path-and-external-tooling-2026-09-11.md
  - genesis/research/search-discovery-incumbent-power-and-p2p-inversion-2026-09-11.md
---

# Memory, search and scale — the three seams

> **One-line:** the same seven recall stages run on one machine, in a household, in a collective, and at the council that stewards a pool; what changes between those sizes happens at exactly three seams, and this spec names each seam, the primitive it rests on, the spec that already owns that primitive, and the one addition this spec makes. It is a *consolidation and a register*, not a plan: nothing here is in flight, and its purpose is that the memory, search, and scale design is present, cited, and reconciled every time these systems are revisited, so the eventual implementation composes with what exists and duplicates nothing.

**What this spec does not restate.** The four roles, five seams, honesty and content floors, reader lens, and the L0→L3 graduation ladder belong to the governed-discovery design (the carrier). Measures as content-addressed observation procedures belong to middot. Pools as Collective EPRs on committed compute, the six-part external-tooling admission contract, and the consented-broker surface belong to the pools research and the specs it cites. Reach vocabulary belongs to the reach-ontology split. This spec cites those and adds three things: **an index is a measure** (§2), **reach is enforced at replication** (§4), and **a pool's index is a council-stewarded measure sharded along DHT arcs** (§5). Evidence for every claim below is in the local-first-to-council research document; this spec carries the decisions, not the survey.

---

## 1. The shape at four sizes (the invariant this spec protects)

The carrier's own test is that a station needing a new seam at L2 got the L0 seam wrong. Read downward, it says the thing that runs at the council must be the thing that runs on one machine, with only the substrate, the providers, and the scope changed. The seams are the three places where a *hand-off of authority* happens, not three different systems:

| Seam | Between | Authority that changes hands | Primitive it rests on | Owning spec | This spec adds |
|---|---|---|---|---|---|
| **1 — the local membrane** | the native recall executor and any outside memory tool (MemPalace today) | who defines the method a candidate was produced by | middot measure; the carrier's `Provider` trait | middot; governed discovery | §2 index-as-measure; §3 native providers; the visitor rule |
| **2 — between holons** | one household or collective and another | who may replicate, not only who may render | DHT private entries and capability grants; iroh-docs read capabilities; `reach_earning` | reach-ontology split; dwelling-hub replication; trust-compute gradient | §4 reach at replication; the viewer; per-peer fan-out |
| **3 — the council pool** | contributing collectives and the pool's custodians | who declares the index bounds and who executes them | Collective EPR; `delegates-compute` and `replicates-*` commitments; DHT arcs | pools research §3–§5; dwelling-hub | §5 pool index as council measure; arc-sharded folds; scatter/gather |

---

## 2. An index is a measure

A middot measure is a named, versioned, content-addressed observation procedure over EPRs whose result is a fold. An index is exactly that: a procedure (chunk, embed, rank) over atoms whose fold cache is the shard a query runs against. Naming it a measure is what gives every result a method CID for free, puts the reach ceiling in the declaration rather than in a mining policy, and makes freshness a reconciliation the storage controller owns. No new ontology is minted.

Landed 2026-09-12 as `elohim/epr-rea/src/index.rs` (protocol vocabulary, zero new DHT entry types; the declaration is a Manifest EPR and the attestation an Attestation EPR), composed from the crate's existing `PinnedRef`, `Bound`, `Sense`, `AgentRef` and `atom_cid`, and from `elohim_epr::{Reach, EprKind, Period}`:

```rust
/// A measure whose fold is a searchable shard. Declared once; a change is a new version.
pub struct IndexMeasure {
    pub measure: PinnedRef,               // the middot declaration, id@version; the CID is IndexMeasure::cid()
    pub chunk_rule: Cid,                  // the chunking procedure, content-addressed
    pub embedding: Option<ModelPin>,      // None for a lexical-only or graph-only index
    pub ranking: RankingMethod,           // Bm25 | Vector{metric} | Graph{resolver} | Fused{recipe, producers}
    pub reach: ReachBound,                // ONE bound with a Sense: a ceiling on one machine, a floor at a pool
    pub surfaces: SurfaceRule,            // constructor refuses PRIVATE_CHAIN_KINDS (AttentionTending) by construction
    pub retention: Retention,             // DemoteAfter{count, per} | Keep — no delete variant exists
    pub fold_lag: Bound,                  // how far behind the heads a fold may lag; the controller reconciles it
}
pub struct ModelPin { pub model_bytes: Cid, pub license: String, pub dims: u32 }

/// Every custodian of a fold proves its own completion. This is the fold receipt.
pub struct FoldAttestation {
    pub measure: Cid, pub shard: ShardManifest, pub heads_at: Vec<(Cid, Cid)>,
    pub state: FoldState,                 // Complete | Degraded{retried} | Failed{why}; is_complete() only for Complete
    pub attested_by: AgentRef, pub at: i64,
}
pub struct ShardManifest { pub arc: Option<ArcRange>, pub atoms: u64, pub bytes: u64, pub manifest: Cid }
```

A *band* of rings (floor and ceiling together) was drafted and withdrawn under the requisite-variety guidestar's admission rule (§3a: a band is one framework, therefore a hold); `IndexMeasure` and `FoldAttestation` themselves pass it, since middot and SuperLocalMemory's per-owner completion proofs are two independent frameworks wanting the same distinction. `validate()` refuses a vector ranking without a pin and a pin no ranking uses, so one meaning has one encoding.

**Bounds carried by the declaration (the reach discipline applied to how far it indexes and compacts):**

| Bound | One machine | Household / collective | Council pool | Who declares |
|---|---|---|---|---|
| reach bound (one, with a sense) | ceiling `SelfScope`; `Intimate` when devices are shared | ceiling at the ring the collective is named by | floor `Commons` (or `Public` for a narrower pool) | reader → collective → council |
| surfaces | recipe `exclude_directories` today; per-atom kind tomorrow | same | contributed atoms admitted under the four conditions | recipe author; council for admission |
| model pin | in the measure; a model change is a new version | shared measure | council measure; changes through the governance ceremony | recipe author; council |
| retention | demotion, never deletion; `as_of` preserved | tier digests | digests per tier per holon; DROP lands as attested demotion everywhere the fold reaches | constitutional floor plus the declarer |

Three invariants follow and are restated in §7: the private chain (journeys, receipts, `AttentionTending`) never enters a fold that can leave the device; compaction is demotion with a timestamp; and the manual mine gate is replaced by the fold-lag bound.

---

## 3. Seam 1 — the local membrane: native providers, and the visitor

**Decision.** Build the EPR-native memory layer; keep MemPalace as a watched visitor. The two are not alternatives. The palace is an appendage today because it does a job that belongs to the host (the only semantic route), and that route is the one part of the harness that cannot climb the ladder: its ranking is opaque by declaration, its model is not swappable, it cannot see the actor sidecar, and it runs as one Python process on one developer machine.

**The native layer is providers behind the carrier's trait, with the lens bound at retrieval:**

| Provider | Store | Method | `ranking_known` | Notes |
|---|---|---|---|---|
| Lexical | SQLite FTS5 in the storage service's own database | BM25 | true | zero new crates; the reach ceiling is a predicate in the query |
| Semantic | sqlite-vec, same database | cosine under `ModelPin` | true | the model bytes are CID'd; a change is a new measure version |
| Graph | the native `ContentGraphResolver` | adjacency and standing edges with validity windows | true | validity is bi-temporal: `as_of` on every edge, demotion never deletion |
| Palace (visitor) | MemPalace, external | its own | **false** | one declared provider among many; invoked when a stage asks for a second opinion; admitted under the six-part contract |

**The lens as a query parameter.** The carrier resolves a lens from recipe defaults, the reader's stated tier, and revealed history. Native providers take the resolved lens *as input*: candidate count and density per tier, the reach ceiling per reader, and the negotiation lands in the receipt. This is what makes discovery negotiable to each reader's capability rather than filtered after the fact. The human half of lens negotiation is the one unbuilt line in the resolver and is a station, not a nicety: a child's lens is co-authored with a guardian and obliged to surface opportunity; an elder's protection is the reach floor, never a narrower lens.

**Rank fusion discipline.** Fusing producer ranks *for order* under a recipe with a CID, with each producer's rank printed beside the result, is a recipe. Fusing *standing* or any human signal into that order is the sum the epic forbids. The native layer prints the shape; it never folds standing into a score.

**The visitor rule.** An outside memory tool may be a declared provider with `ranking_known: false`, budgeted by `provider_bytes` and `provider_seconds`, enveloped, attested, granted by a Mishpat commitment rather than a key, and retirable. It is never the index. The host role stays real only while a visitor is actually in the house, so the palace stays, and the test that says the native layer is done is that the recall contract's `semantic_provider` no longer names it and removing it loses only the second opinion.

**What the field confirms, and the one thing it does not.** SuperLocalMemory 4.0 (AGPL-3.0 Python; pattern, never code) independently converged on the same canonical store (SQLite with FTS5 and a vector virtual table, derived projections behind staged verification), on scopes enforced at write admission and as query predicates, on bi-temporal `as_of`, and on per-owner completion proofs. All ten of its published negative results sit in its learned-from-behavior layer, which this design refuses on the epic's grounds and now on engineering grounds. Its two mechanical invariants (prior-distance, join-liveness) are adopted as habit probes. No surveyed tool has a graduated reach model as a first-class primitive; that part has no template.

---

## 4. Seam 2 — between holons: reach at replication

**Finding.** Reach is enforced when content is authored (`reach_earning::evaluate`) and again when the doorway serves it (`reach_aware_serving`), and never at replication. A household node that replicates a Trusted-ring record holds bytes it may not render; render-gating is where an honest-but-curious node is not looking. No production entry in any DNA declares `EntryVisibility::Private`; no zome creates a capability grant; the substrate's own primitives for this are unused. `evaluate` takes the local agent, the author, and the requested ring, and no viewer.

**Decision.** Enforce reach at the replication boundary at three grains, in this order, and never as a substrate swap:

1. **Inner rings on the DHT's own primitives.** `Private`, `SelfScope`, and `Intimate` content as private entries with capability grants to the household's agents. The substrate already refuses to serve what it was not granted.
2. **Blob ranges under read capabilities.** iroh-docs namespaces and read-capability keys at the blob layer, one namespace per ring per holon, so a replica cannot *request* out-of-ring bytes. Coarser than sub-atom grain; adequate for the middle rings.
3. **Sub-atom reach as a capability shape.** Meadowcap (Willow) is the only published system gating sub-document ranges at the sync boundary; it is the *grain reference* for records whose fields sit at different rings, implemented over the existing transports when that day comes.

And **a viewer in `evaluate`**: per-viewer effective reach is the epic's central promise and the smallest runnable move on this whole thread. Its scenario is a household node requesting a Trusted-ring record it holds no grant for, refused *before bytes move*.

**Field convergence.** Keyhive/Beelay, Jazz/CoJSON, and p2panda's encryption all landed on group-membership-as-a-CRDT gating a symmetric key, with an untrusted relay that moves ciphertext it never decrypts. The confidentiality-plane cluster already holds the KeyEnvelope and reader-key decisions this composes with; nothing here reopens them.

**A search between holons** never leaves the requester's holon except as a bounded request to a peer, which answers from its own index under its own reach check and keeps its own receipt. There is no aggregator; the self-hosted metasearch proxy is the cautionary case, relocating trust to whoever runs it. Cryptographic private search is not available at this scale (PIR is server-shaped and heavy; searchable encryption is unmaintained and wrong-licensed), so **reach-tier filtering is the practical private search**, with private set intersection as a narrow, Rust-embeddable prototype for overlap discovery without disclosure.

**Sensemaking** between collectives takes algorithms, not tools: the bridging score (helpful across opposed rater clusters) printed as a shape over qahal ratings; the W3C Web Annotation data model for commentary across holons; a self-hostable clustering pipeline on a pool's committed compute.

**The household is the smallest collective.** A household is a Collective EPR at its smallest size, named by its ring, with no new kind; the index bounds of §2 then belong to a collective at every size, and the ladder's L1 row is a household by construction.

**Metadata has reach too.** Custody announcements, inventory gossip, and observation gossip are facts about a holon visible to peers, and each needs a ring like everything else.

---

## 5. Seam 3 — the council pool: an index sharded along arcs

**The pool's index is an `IndexMeasure` whose declaration is a Collective EPR at council level.** The council stewards the declaration (chunk rule, model pin, ranking method, reach floor, retention) and the shard admission rule; changes go through the governance ceremony that earned reach already requires for high-stakes artifacts. The elohim execute faithfully as a utility: custodians index, answer, and re-project; they propose and never rule.

**Sharding.** The DHT already partitions by hash range into arcs. The pool's fold cache shards along the same arcs: each custodian, under a `delegates-compute` commitment and a `replicates-*` commitment, indexes the range it already holds. **Scatter and gather**: a query is a scatter over arcs under the pool recipe and a gather under the same recipe, and the root that merges is the requester's own node. That is why explorer-blind holds by construction: a custodian sees a bounded request and never the reader's journey; the receipt stays home. **Replication and hedging**: a replication factor per arc, a request hedged to a second custodian after a short delay with the first answer winning; hedged work is counted twice so the requester's bound prices it, and which custodian answered prints on the result. **Tiering**: the Commons-ring shard first, deeper only when it is thin, which is "amplification is earned" as a latency feature. **Freshness**: a custodian re-folds its arc on head change; the pool-level freshness measure is fold lag per arc. **Ranking**: known methods only, with a declared static order carrying its own CID for early termination, printed beside standing and never summed into it; there is no learned ranker. **Custody heat as care**: which shards are drawn from is counted as REA care toward the custodians; who drew is never recorded above the requester's holon.

**What crosses upward and what comes back.** Upward, only what constitutional revealability allows: atoms contributed at Community, Public, or Commons under the epic's four conditions; attested outcomes; tier digests. Never journeys, never `AttentionTending`, never a behavior log. Downward: results carrying the pool recipe's CID, per-unit provenance to the contributing holon, the custodian that answered, and standing as a shape. **Dispute**: the recipe CID, the custodian, and the provenance name three parties a Mishpat verdict can be sought against.

**The point-by-point crosswalk** against the incumbent's mechanisms (crawl, offline build, memory-resident index, sharding, fan-out, replication and hedging, tiering, early termination, caching, rerank, geography, freshness, query understanding, payment, sight, dispute), with labor and provenance answered on both sides and a status per row, is §3a of the pools research document and is not restated here. Its conclusion stands as this spec's scale claim: none of the incumbent's speed is a database feature, and within a ring where shards are warm and replicated nothing structural stands between the pool and that latency.

**The doorway.** What a doorway holds is a projection, notarized by the holonic authorities that govern its relationship to the people it serves, plural and federated in contract so that nothing on the dataplane can be captured by any single doorway. A doorway may hold a pool shard only as a custodian under the pool's grant, like any node, and never as the pool's warehouse. Thinness is the guard: federation must stay cheap enough that many operators run a doorway, or the fediverse's recentralization returns by the route of operators paying a cloud.

---

## 6. Seam register — where each concern lives today, and what owns it

The dedupe guard. Before designing anything in this space, find the row; if it is here, extend the owner, do not mint.

| Concern | Lives today (evidence) | Owning spec / decision | This spec's addition | Status |
|---|---|---|---|---|
| recall stages, budgets, receipts, contract CID | `epr-cli/src/flow/memory/recall/` (7 stages, 13 bounds, 18 operations; receipts private under `.eprfs/status/recall/`) | governed discovery; recall habit | none | ✅ |
| lens (defaults / stated / revealed) | `recall/lens.rs`; human half unbuilt at `lens.rs:522` | governed discovery station 1 | lens as provider input; human negotiation as a station | ✅ agents · ⚠ humans |
| `AttentionTending` kind and its `Classification` enum | `elohim-storage/src/p2p/attention_tending.rs:61-70, 89` (`ValuesForward, Fatigue, ScopeMismatch, Safety`); integrity zome | witnessed-interaction primitive; sense-respond | none; the private-chain exclusion is a surface rule on the measure | ✅ |
| semantic candidates | MemPalace, `ranking: null`, model hardcoded | recall contract | replaced by §3 providers; palace stays a visitor | ⚠ |
| lexical / vector / graph search in Rust | none in any `Cargo.toml`; the vocabulary landed 2026-09-12 in `elohim/epr-rea/src/index.rs` (`IndexMeasure`, `FoldAttestation`, `ReachBound`, `SurfaceRule`, `Retention`) | native content-graph seam (graph); this spec §2 | FTS5 + sqlite-vec providers behind the trait | ◐ kinds · ⚠ providers |
| index freshness | manual mine; `mempalace-surfaces-changed-ceiling@1` | measures.yaml | fold-lag bound; controller re-fold on head change | ⚠ |
| measures and folds | `elohim/epr-rea` (`Stock`, fold, `with_uncertainty`) | middot; measure-family cluster row 1 | `IndexMeasure`, `FoldAttestation` | ◐ |
| standing with validity | `EpistemicStanding` | digital-memory standing ontology | bi-temporal `as_of`; demotion never deletion | ◐ |
| reach vocabulary | `elohim_epr::Reach` (8 rings) plus drift | reach-ontology split; OWL §6 plan | none; the household is named by ring | ✅ enum · ◐ drift |
| reach at authoring | `services/reach_earning.rs` `evaluate` (no viewer) | reach-ontology split | a viewer parameter | ⚠ |
| reach at serving | `doorway-service/src/cache/reach_aware_serving.rs` | doorway serving path | none | ✅ |
| reach at replication | absent; no private entries or cap grants in production | confidentiality-plane cluster (KeyEnvelope, reader key) | §4 three grains | ⚠ |
| sync and transport | Automerge over sled (`src/sync/`); libp2p 0.54 and iroh 0.92 parallel | content-sync plane; dwelling hub | iroh-docs read caps per ring per holon | ✅ transport · ⚠ caps |
| metadata gossip | `custody_announce`, `inventory_gossip`, `observation_gossip`, … | dwelling hub | a ring per gossip surface | ⚠ |
| compute grants and pools | `elohim-compute`, `compute_grants.rs`, `PoolConfig`, constitutional donut | pools research; trust-compute gradient | §5 pool `IndexMeasure`, arc-sharded folds | ✅ grants · ⚠ folds |
| custody and replication commitments | `ReplicationStatus`, `replicates-*` | dwelling hub | replication factor per arc; hedging | ✅ · ⚠ hedging |
| result caching | doorway tiered cache; Patron-CDN | doorway serving path; pools §2.3 | public results only; cache serves a CID'd result | ✅ · ◐ |
| external tooling admission | `.mcp.json`, per-agent allowlists, `REQUIRED_LIMITS` | pools §5 (six parts) | the visitor test closure for the palace | ◐ |
| household / holon as a kind | none in Rust | commons-holonic stewardship cluster | household = smallest Collective EPR | ⚠ decision |
| behavior-learned ranking | refused | search epic | refused again, with SuperLocalMemory's evidence | ✅ (by refusal) |

---

## 7. Invariants (the anti-capture set this spec adds to the carrier's)

1. **The content is synced; the index never is.** Each device re-folds from what it holds. An index that travels is an aggregate that travels.
2. **A method on every result.** Every candidate names the `IndexMeasure` CID and the producer that ranked it; a `ranking_known: false` provider is declared as such and is never the index.
3. **Reach is a predicate in the query and a grant at the replication boundary**, never only a render filter and never a mining policy.
4. **Standing prints as a shape; ranks may fuse for order under a recipe with a CID; standing never enters the fusion.**
5. **No behavior-learned ranker, anywhere, at any rung.** Where a posterior is ever introduced, the prior-distance and join-liveness probes go in first.
6. **Demotion, never deletion; `as_of` preserved.**
7. **The private chain never enters a fold that can leave the device.** Journeys, receipts, and `AttentionTending` are excluded by the measure's surfaces, not by policy.
8. **The root that merges is the requester's own node.** Custodians see bounded requests; receipts stay home.
9. **The doorway is a projection under a thin, plural, notarized federation contract.** Never a warehouse, never "the trusted tier", never a blind relay.
10. **The visitor is never the index**, and the host keeps a visitor in the house so the admission contract stays real.

---

## 8. Stations after station 3 (registered, not scheduled)

The governed-discovery plan scopes stations 0–3 and names 4 and 5 as out of scope. These are the stations this spec registers for that plan's successor, with the build order from the research document, so they are picked as rows rather than re-derived. None is in flight.

| Station | Builds | Seam | Depends on | Probe |
|---|---|---|---|---|
| 4 — native providers | FTS5 lexical provider; `IndexMeasure` declaration; sqlite-vec semantic provider with `ModelPin`; fold-lag freshness retiring the mine gate | 1 | station 1 (trait) | the contract's `semantic_provider` no longer reads `mempalace`; a candidate prints its measure CID |
| 5 — standing in time | bi-temporal `as_of` on `EpistemicStanding` and graph edges; human lens negotiation | 1 | station 4 | a query `as_of` yesterday returns a since-demoted fact with its timestamp; a human `ReaderRef` resolves a stated level |
| 6 — reach at replication | household as smallest Collective EPR; private entries and cap grants for inner rings; iroh-docs read caps per ring per holon; a viewer in `evaluate` | 2 | p2p-design-gate; confidentiality rows | a2o: a household node is refused a Trusted-ring record before bytes move |
| 7 — search between holons | per-peer fan-out with bounded requests and home receipts; bridging classifier as a shape | 2 | stations 4, 6 | two households search each other's Community ring; no aggregator process exists |
| 8 — the pool fold | pool `IndexMeasure` at council level; arc-sharded folds under grants; scatter/gather; hedging; tiering by ring | 3 | station 6; pools §3 pool primitive | a two-custodian pool returns candidates naming both custodians and one recipe CID |
| 9 — the visitor test | MemPalace enveloped and attested (six of six); embedding-model attestation for any hop beyond `Private` | 1 | pools §5 | the admission ledger shows six of six for the palace |
| probes | join-liveness on every schema-guarded path in the recall executor; prior-distance wherever a posterior appears | all | none | a probe names where the data actually lives when a guard fails |

---

## 9. Decisions still the operator's

The research document's §6 carries seven questions with a recommendation each. Restated here only by name so they are found: Q1 household as smallest collective (recommended yes); Q2 native semantic route timing (after station 1); Q3 replication grains (all three, in order); Q4 which embedding model and how it is pinned (license and size pass before pinning); Q5 index never synced (recommended never); Q6 rank fusion as a sum (only when standing enters it); Q7 the doorway's store as a pool index (no; projection under a thin plural contract).

---

## 10. Backlog rows this spec is cited from

Minted 2026-09-12 with the research document: measure-family borrows rows 26–27 (index-as-measure; the two invariants); commons-holonic stewardship rows 27–29 (household as smallest collective; pool index as council measure; bridging shape); dataplane borrows rows 12–15 (reach at replication; hedging; native providers; bi-temporal `as_of`); confidentiality-plane rows 10–11 (the visitor test; the model as a covert channel); scale-risk rows 9–10 (sled; the fat doorway); agentic-context-tooling queue item 24 (the palace's semantic route goes native). Each row cites this spec and the research document; when a row is picked for work, its plan cites the row, and this spec's §6 is re-read against the tree first.
