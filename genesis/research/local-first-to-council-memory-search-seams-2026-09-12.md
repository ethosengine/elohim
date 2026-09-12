---
title: "From One Machine to the Council — local-first memory, search and discovery across three seams"
status: Capture (co-authored; two horizon ledgers appended verbatim with reconciliation notes)
date: 2026-09-12
serves: recall-reaches-authority
realizes:
  - genesis/docs/content/elohim-protocol/search/epic.md (the search epic — this is the shape on the local machine that the epic's rings assume)
informed-by:
  - genesis/research/search-discovery-incumbent-power-and-p2p-inversion-2026-09-11.md
  - genesis/research/commons-data-pools-hot-path-and-external-tooling-2026-09-11.md
  - genesis/docs/superpowers/specs/2026-09-11-governed-discovery-journey-lens-graduation-design.md
  - genesis/docs/superpowers/plans/2026-09-11-governed-discovery-stations-0-3-plan.md
  - genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md
  - genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md
  - genesis/docs/content/elohim-protocol/living_memory/epic.md
---

# From One Machine to the Council

*The transitional document between the native memory work and the commons pools. The search epic set rings from Private to Commons; the pools document said what an index is above the household. Neither said what happens on one machine, then in one house, then in one collective, then at the council that stewards a pool. This document draws that progression as one shape at four sizes, answers the question of what an EPR-native memory layer gives that hosting an external one cannot, and names the three seams where the shape changes hands.*

**Method.** A Sonnet explorer grounded the tree (`path:line`, 100 tool uses); two Sonnet horizon surveys graded adjacent work TAKE / TAKE-DISC / REFUSE / WATCH (local-first agent memory; CRDTs, sync engines, private retrieval, and small-group sensemaking); three 2026 papers the surveys leaned on were fetched and read by the orchestrating session; synthesis by the orchestrating session with the operator co-authoring. §6 is questions with a recommendation each.

**Verification key:** ✅ verified in source · ◐ single-source · ⚠ unverified.

**Vocabulary.** *Bounds* are the mishpat side of a recipe: declared limits a stage may not exceed. *Measures* are the middot side: named, versioned, content-addressed observation procedures over EPRs (`2026-08-04-middot-measure-primitive-design.md`). The governed-discovery spec composes both under each stage ✅ (`spec:89`). This document adds nothing to that vocabulary; its one claim is that an index *is* a measure.

---

## 0. The shape at four sizes

The governed-discovery spec's test is that a station needing a new seam at L2 got the L0 seam wrong ✅ (`spec:326-330`). Read downward, the same test says: whatever runs on one machine must be the thing that runs at the council, with only the substrate, the providers, and the scope changed. This table is that claim made visible.

| | **One machine** (L0, today) | **Household** (L1; the smallest collective) | **Collective** (L2 native peer) | **Council / pool** (L3 network) |
|---|---|---|---|---|
| What is stored | one checkout; EPRFS files, CID-pinned contract | content replicated among a few devices under Intimate / Trusted rings | the collective's content projection in elohim-storage | contributed shards at Commons ring, held by custodians under compute grants |
| What discovers | deterministic lexical scan; MemPalace as optional opaque semantic route | the same stages over the household's replicated content | the same stages, plus peer inventory as a provider with `ranking_known` | the same stages, scattered over shard custodians, gathered under one pool recipe |
| Where reach is checked | directory scope (`Scope::Directory`) | at write admission and in every query predicate; **not yet at replication** | `Scope::Ring` from Intimate outward; authoring (`reach_earning`) and serving (doorway) | ring priced by standing; the pool's reach floor is a bound in its measure |
| Who stewards the index bounds | the reader, through the recipe's defaults | the household | the collective, for its ring's index measure | the council, for the pool measure and shard admission |
| What a receipt is | `continuation.json` under `.eprfs/status/recall/<session>/`, private, refused as import | same, per device, never synced | private event + attested digest | digests per tier per holon; the query's receipt stays home |
| What compacts | nothing yet; the mine gate is manual | temporal demotion, never deletion | tier digests | council-declared retention as a bound; fold cadence |
| MemPalace's role | the only semantic route (the appendage) | none | none | one declared outside provider among many, `ranking_known: false` |

Everything else is invariant: the four roles and five seams, the honesty and content floors, the CIDs on every view, the second-seat judgment, per-holon custody of journeys, provenance on every candidate.

---

## 1. Seam 1 — one machine: eprfs and the visitor

### 1.1 What runs today ✅

The recall contract is `bounded-evidence-recall` v12 at `.epr-meta/elohim/algorithms/recall-contract.json`; the executor is `elohim/eprfs/epr-cli/src/flow/memory/recall/`. Seven stages, refused if not exact: `scope, discover, filter, group, select, read, judge` (`recall/mod.rs:101-103`). Thirteen required budgets (`mod.rs:106-120`), among them `provider_bytes: 8192` and `provider_seconds: 15`. Eighteen operations (`mod.rs:135-154`). A session opens by creating and flocking a private continuation file, then reads, pins, and charges in that order: "the contract's raw CID is the algorithm's identity" (`mod.rs:534-546`); a changed contract or executor digest refuses continuation until `adopt`. The receipt records the method CID, the executor digest, baseline and close measurements, costs, per-excerpt evidence fingerprints, and a frontier. Receipts are refused as import, projection, witness, or feedback targets (`recall/receipts.rs:8-40`): "a recall session exposes what was read and what was concluded, and nothing else."

The lens (`recall/lens.rs`) composes three inputs: the recipe's defaults, the reader's *stated* tier from the actor sidecar (`role@model`, e.g. `claude-sonnet-5 → simple`, `claude-fable-5-1 → detail`), and the reader's *revealed* history. A `ReaderRef` is resolved from the sidecar, never inferred (`lens.rs:157-172`). The human half is one unbuilt line: "human lens not yet negotiated; recipe default" (`lens.rs:522`).

The classifier vocabulary (`AttentionTending` with `ValuesForward | Fatigue | ScopeMismatch | Safety`) is real Rust at L2, both the kind and its `Classification` enum (`elohim-storage/src/p2p/attention_tending.rs:61-70, 89`; integrity zome `content_store_integrity/src/attention_tending.rs`). An earlier draft of this document called the variants declared only; that was an inventory error, corrected 2026-09-12. The sense-respond classifier (`2026-07-15`) is the frame-and-intent design it inherits from.

No Rust crate anywhere in the tree declares full-text or vector search (grep across every `Cargo.toml`: zero hits for tantivy, sqlite-vec, fts5, hnsw, usearch, embedding) ✅.

### 1.2 What the visitor does for us, and where it fails the contract ✅

MemPalace runs as a stdio MCP server against `/projects/elohim/.mempalace/palace`, gitignored inside the tree. Its store is ChromaDB (vectors) plus SQLite (a temporal entity graph). Its embedding model is `all-MiniLM-L6-v2`, hardcoded in its `embedding.py`; the environment variable that appears to select a model is decorative. The recall contract names it as `discovery.semantic_provider`, with `ranking: null`, `version: null`, `freshness: null`, `optional: true`, and the restriction "candidates require source verification". A stage may call it at most once per packet with a limit of three. The librarian is the only agent with write access, and writing is described as a gated graduation act. Mining is operator-driven; the freshness bound `mempalace-surfaces-changed-ceiling@1` (`.claude/epr-meta/measures.yaml:1295-1321`) is red at the time of writing: twelve of 552 files are newer than the last mine.

Against the six-part admission contract from the pools document (declared, granted-not-keyed, budgeted, attested, enveloped, witnessed-or-retirable): it is declared, it is budgeted at the contract level, and it is retirable. It is not enveloped (no tevah; a bare process in the devspace), not attested (no in-toto envelope over its bytes or model), and its grant is the MCP registration itself rather than a Mishpat commitment. Three of six, which is acceptable on one developer machine and is the visitor test to close deliberately (§4, B13).

### 1.3 The answer: build the native layer; keep the visitor

The operator's question, put directly: what would an EPR-native memory layer give that hosting MemPalace as a watched visitor cannot? The answer is that the two are not alternatives. MemPalace feels like an appendage because it is doing a job that belongs to the host, and that job is the one thing in the harness that cannot climb the graduation ladder.

What a native layer gives that no hosting discipline can:

- **A method on every result.** The palace's ranking is opaque by its own declaration, and its model is not swappable. Every candidate it returns is `ranking_known: false` forever. A native provider pins the model bytes, the chunking rule, and the index build as one content-addressed procedure. That procedure is a middot measure, and the index is the measure's fold cache. No new ontology.
- **Reach enforced in the query, per chunk.** The palace is one flat store; the only reach control is which directories get mined. Native chunks carry the ring as a column in the same SQLite the storage service already uses (`reach_earning::evaluate` takes a `SqliteConnection` ✅), and the ceiling is a predicate in every query, not a mining policy.
- **The lens bound at retrieval and receipted.** The lens already composes defaults, stated, and revealed. The palace cannot see the actor sidecar, so today the lens can only filter after the fact. A native provider takes the lens as a query parameter, so a Haiku reader gets fewer and denser candidates, a Fable reader more, a human negotiates, and the negotiation lands in the receipt. This is the negotiability the operator asked for; the human half is `lens.rs:522`.
- **It runs on every rung.** The palace is a Python process on one developer machine. The ladder says the same stages run on the peer over its content projection and then across holons. Only Rust in the storage service does that on a phone, a household node, and a hosted cell. The index is a projection rebuilt per device from replicated content. The content is synced; the index never is.
- **Freshness as reconciliation, not a chore.** The red mine gate is the symptom. Native indexing is the storage controller re-projecting on a content-head change, the P1 posture the dataplane already holds.
- **The candidate is the provenance.** A drawer is a chunk that still needs source verification. A native candidate is an atom CID.

What the visitor still gives and should not be rebuilt:

- **A second opinion that does not share our method.** The sparring partner's five standing tests mean nothing against a ranker that shares our recipe.
- **The reference instance of the admission contract.** People will bring Letta, Mem0, Cognee, SuperLocalMemory. The host role stays real only while a visitor is actually in the house.
- **Exploration affordances** we have not built: a taxonomy, tunnels, a graph timeline. Study, do not embed.

**The test that says it is done.** The contract's `semantic_provider` no longer reads `mempalace`, and removing the palace changes nothing about the core experience except losing the second opinion.

### 1.4 SuperLocalMemory, read closely ✅ (arXiv 2608.08253 v2, 2026-08-25; repo `qualixar/superlocalmemory`, v4.1.15)

The operator finds it attractive, and it deserves the attention, with one hard line: it is **AGPL-3.0 Python** with commercial licensing. Pattern to take; code to refuse.

*Take, as shape.*
- **Canonical store is SQLite with FTS5 and a `vec0` virtual table (sqlite-vec)**; CozoDB and LanceDB are optional *derived* projections behind staged verification. This is the native shape §1.3 recommends, independently converged.
- **Scopes `personal / shared / global`, enforced at write admission and as predicate guards on every retrieval channel; cross-profile recall default-deny.** Three flat scopes are what our eight rings become when the ladder is collapsed to one machine. The enforcement points are the right ones.
- **Bi-temporal facts** (referenced / observation / interval dates) with `as_of` threaded through the Python engine, HTTP, MCP, and CLI; invalidation is "point-in-time demotion, not snapshot deletion". This is the Living Memory epic's forgetting made mechanical, and the Zep/Graphiti temporal-edge pattern applied to facts.
- **Completion manifests**: an obligation ledger, three projection owners (BM25, temporal, vector) each proving their own completion through a verify/apply/verify lifecycle, a reconciler, and a keyed hash over the manifest; verified erasure spans all owners. Our receipts already carry method CID and measurements; what we lack is *each index owner proving its own completion*, which is exactly what a measure's fold attestation should carry.
- **Two mechanical invariants**, offered because the authors' own learned layer failed silently: *prior distance* (assert a Bayesian posterior has moved off its prior after n observations) and *join liveness* (assert a schema-guarded path can actually execute against the store, naming where the data really lives when it cannot). Their sentence is worth keeping whole: "a signal derived from the mechanism it evaluates cannot detect that mechanism's failure." This is our habit register's `unwired` state and its flips-require-evidence rule, stated by strangers.

*Refuse, with the paper's own evidence.*
- **The "memory brain"**: a behavioral pattern miner, a LightGBM reranker, Thompson sampling over arms, Ebbinghaus decay applied after rank fusion, "Riemannian Langevin lifecycle positioning on a Poincaré ball". All ten of the paper's negative results live in this layer. The posterior sat at its prior after 5,657 plays across 459 arms; trust-weighted forgetting was arithmetically inert; an engagement signal never reached its learner because of a namespace disagreement. The deterministic layers (admission, SQLite core, FTS5, vec0, bi-temporal, manifests) worked; the learn-from-behavior layer was "implemented, reachable, and ineffective". We refuse a behavior-learned ranker on the epic's grounds (looking never offered to the log is never pooled). This paper adds the engineering ground: it is also where the silent failures live.
- **SLM-Mesh**: last-writer-wins state convergence under `(revision, node_id)`, advisory locks with fencing tokens, "coordination only, not automatic replicated memory or conflict resolution", validated on a two-node loopback. Household scale is not solved there; the seam-2 survey (§2) says nobody has solved it as a product.
- **Reciprocal-rank fusion of five producers into one order.** A discipline, not a refusal: fusing *for order* with the method CID printed is a recipe; fusing *standing* into a score is the thing the epic forbids. The native layer prints each producer's rank as a shape beside the fused order, and never folds standing into either.

*Verdict.* **TAKE-DISC**: adopt the store shape, the three enforcement points, bi-temporal `as_of`, per-owner completion proofs, and the two invariants; refuse the code, the learned layer, and the mesh.

### 1.5 How far it indexes, and how it compacts

The operator's framing: take something SuperLocalMemory-shaped and bring the reach and seam graduation discipline to *how far it indexes and compacts*, and *which steward collectives steward the bounds*. On one machine that reduces to four declarations in one measure:

| Bound | On one machine | Who declares |
|---|---|---|
| **Reach ceiling** of what the index may contain | `SelfScope` by default; `Intimate` when the household's devices replicate here | the reader; the household when devices are shared |
| **Surfaces** (which content kinds and paths) | the recipe's `discovery.exclude_directories` today; per-atom kind tomorrow | the recipe author |
| **Chunk rule and model CID** | pinned in the measure; a changed model is a new measure version, never a silent re-embed | the recipe author; a collective when the measure is shared |
| **Retention and demotion** | temporal demotion, never deletion; a journey is never indexed into anything shareable | constitutional (revealability principle 9) plus the reader's own retention |

Three things follow. First, `AttentionTending` records, journeys, and receipts never enter any index that can leave the device; the private-chain rule is a bound on the measure, not a mining choice. Second, compaction is *demotion with a timestamp*, so a fact that stopped being true is still findable `as_of` the day it was. Third, the mine gate goes away: an index whose inputs are content atoms with heads is a projection the controller reconciles, and the freshness measure becomes "how far behind the head is the fold".

### 1.6 The person at the machine

Two hazards from the field belong here. Microsoft Recall and Rewind both broke on the same axis: a device that captures everything captures people who are not its owner, with no consent model (◐, appendix A rows 20-21). Our answer is structural: nothing is indexed that was not *authored* into an atom with a reach, so the bystander in the video call is never a chunk. The second is the cloud vendors' background "dreaming" that rewrites a user model with no per-claim receipt, already a demonstrated injection vector (◐, row 25). Our answer is spec principle 7: consolidation is judgment, recorded as an event, never janitorial cleanup.

The lens must also serve the whole human life (spec principle 8): a child's lens co-authored with a guardian and obliged to surface opportunity, an elder's protection as the reach floor rather than a narrower lens. Neither is an autonomy tier, and both are why the human half of lens negotiation (§4, B6) is not a nicety.

### 1.7 Seam-1 ledger, summarized (appendix A verbatim)

| Verdict | Entries |
|---|---|
| **TAKE** | sqlite-vec · LanceDB · tantivy · usearch · llama.cpp/Ollama for local embeddings · libSQL embedded-replica *pattern* · Zep/Graphiti temporal edges · PROJECTMEM (event-sourced, deterministic projection, MIT) · Ink & Switch local-first ideals · Apple Private Cloud Compute as the attestation *pattern* only |
| **TAKE-DISC** | SuperLocalMemory (§1.4) · Letta's tier frame (refuse its LLM rewrite) · Mem0's audit sidecar · Obsidian Smart Connections' privacy bar (license too closed) · A-MEM atomic notes · MOSS (license unconfirmed) · Meilisearch (a server, heavier than tantivy) |
| **REFUSE** | Honcho (durable psych-profiling, no provenance) · Rewind · Microsoft Recall · OpenAI "Dreaming" consolidation · Quickwit (wrong scale) |
| **WATCH** | Cognee · LangMem · DuckDB VSS · Chroma · Khoj · Logseq · Anthropic per-topic memory UI as a lens precedent |

Pattern the survey names that we should say plainly: **no surveyed tool has a graduated reach model as a first-class primitive.** RBAC and per-topic exclusion are the nearest, and both are flat. This part has no template.

---

## 2. Seam 2 — between holons: private and inter-collective search, sync, sensemaking

### 2.1 What we have ✅

- **Sync.** `elohim-storage/src/sync/` is "offline-first document sync using Automerge CRDTs": a `DocStore` over sled, a `StreamTracker`, a `SyncManager`. Automerge appears nowhere else in the tree.
- **Transport.** libp2p 0.54 and iroh 0.92 (pinned) as parallel stacks behind `p2p` and `p2p-iroh` features; `src/p2p_iroh/` carries `sync.rs`, `sync_driver.rs`, `sync_backend.rs`. Iroh is the staged cutover.
- **Metadata-only gossip surfaces**: `custody_announce` ("a peer-announced row is a self-asserted custody claim"; no DHT entry for who-holds-what), `inventory_gossip`, `observation_gossip`, `attention_tending`, `identity_binding_gossip`, `feedback_signal`.
- **Receiver-side pre-authorization** in `private_receive.rs`: a peer keeps a private record only if the sender resolves to an agent and this peer holds a live custody spool naming that agent as ward, or a custody blob naming the record's digest; the ward is the transport-resolved sender, never the record's own `created_by` claim.
- **Projection reconciliation** (`projection_reconcile.rs`): peer SQL is discovery only; row content comes exclusively from this conductor's own DHT view; peer bytes are never written into the projection.
- **Reach.** `elohim_epr::Reach` has eight rings (`epr/src/reach.rs:18-35`). `reach_earning::evaluate(local_agent, author, requested_reach, conn, registry) -> ReachVerdict` (`services/reach_earning.rs:207-213`) has no viewer parameter. `reach_aware_serving` lives in the doorway (`doorway-service/src/cache/reach_aware_serving.rs`), applied at the proxy.
- **In the DNAs**: no production entry declares `EntryVisibility::Private` (the two hits are a loop over both visibilities in node-registry and a test asserting a private record is rejected in mishpat); no `create_cap_grant` call anywhere. `holon` has zero Rust hits; `household` is prose; `collective` is a content-type tag.

### 2.2 The finding: reach is enforced at authoring and at serving, never at replication

Read together, the facts above say that reach is decided when content is authored (`reach_earning`) and checked again when the doorway serves it. Between those two points, a household node that replicates a Trusted-ring record holds bytes it may not render. Rendering is where honesty is enforced, and rendering is exactly where an honest-but-curious node (the pools document's household threat model) is not looking. Metadata gossip is itself a reach question: a custody announcement is a fact about who holds what, visible to peers, and nothing today prices it by ring.

This is the gap the seam-2 survey names from the outside. Its strongest pattern (appendix B) is that Keyhive/Beelay, Jazz/CoJSON, and p2panda's encryption all landed on the same shape: **group membership as a CRDT gating a symmetric key, with an untrusted relay that moves ciphertext it never decrypts**, so that "who may read" is decided before any read is possible. And **Meadowcap** is the only published capability system that gates *sub-document ranges* at the sync boundary rather than whole documents, namespaces, or spaces; that granularity is what an eight-ring ladder needs.

What we should do with it is not a substrate swap. The survey's own last line on kitsune2 is that "nobody external beats a well-configured DHT plus capability-grant discipline here". We hold that discipline as an unused primitive: Holochain private entries and capability grants exist in the substrate and appear in no production zome. The recommendation, in order:

1. **Inner rings on the DHT's own primitives.** `Private`, `SelfScope`, and `Intimate` content as private entries with capability grants to the household's agents. Zero new machinery; the substrate already refuses to serve what it was not granted.
2. **Blob ranges under read capabilities.** iroh-docs' namespace and read-capability keys at the blob layer, one namespace per ring per holon, so a replica cannot *request* out-of-ring bytes. Coarser than Meadowcap; adequate for the middle rings.
3. **Meadowcap as the grain reference** for the day sub-atom ranges matter (a record whose fields sit at different rings), implemented as a capability shape over the existing transports, never as a new protocol.
4. **A viewer in `evaluate`.** Per-viewer effective reach is the epic's central promise and the signature does not carry a viewer. That parameter is the smallest runnable move on this whole thread, and the a2o scenario is a household drawing a record from a peer and being refused at the *replication* boundary, not at render.

### 2.3 The shape of a search between holons

A search never leaves the requester's holon except as a bounded request to a peer, and the peer answers from *its own* index under *its own* reach check, with *its own* receipt kept home. There is no aggregator. SearXNG is the cautionary case: a self-hosted, privacy-respecting metasearch that nonetheless relocates the point of trust to whoever runs it (◐). Per-peer fan-out with bounded requests is the only shape the survey found that does not recentralize, and it is what the spec's L3 row already says: "a household searches across the holons it participates in".

Cryptographic private search is not available at this scale. PIR is real but server-shaped and compute-heavy; searchable encryption implementations are unmaintained and GPL-family; private set intersection is Rust-embeddable and serverless but solves only set overlap (◐). So: **reach-tier filtering is the practical private search**, and PSI is a narrow, worthwhile prototype for *overlap discovery* ("do our households share a trusted steward?") without disclosure.

### 2.4 Sensemaking between collectives

The sensemaking tools worth taking are algorithms wrapped around ordinary infrastructure, and the algorithms port without the infrastructure (◐):

- **Community Notes' bridging score** (Apache-2.0): matrix factorization that surfaces what is found helpful *across* opposed rater clusters. Run over qahal ratings or mishpat disputes locally. Discipline: print it as a shape (which clusters found it helpful), never as a rank.
- **Polis** (AGPL, centralized): the opinion-clustering and bridging-statement discovery is the reusable idea; the tool is not.
- **Hypothesis' W3C Web Annotation data model** (BSD client): adopt the model verbatim for commentary across holons (targets, bodies, group-scoped permissions); never run the centralized server.
- **Talk to the City**: a self-hostable clustering pipeline that belongs on a pool's committed compute, not as a network service.

### 2.5 The household is the smallest collective

`holon` has no Rust; `household` has no struct. The pools document's Q3 asks whether the pool primitive is a Collective EPR. This document recommends the matching answer downward: **a household is a Collective EPR at its smallest size, named by its ring (Intimate or Trusted), with no new kind.** The index bounds of §1.5 then belong to that collective, and the ladder's L1 row is a household by construction rather than a special case.

### 2.6 Seam-2 ledger, summarized (appendix B verbatim)

| Verdict | Entries |
|---|---|
| **TAKE** | Automerge 3 (already ours; v3 removes the memory wall) · Meadowcap (grain reference) · p2panda encryption (production Rust, DCGKA) · iroh-docs (already on iroh; pair with app ACL) · Community Notes bridging algorithm · Hypothesis W3C annotation model |
| **TAKE-DISC** | Beelay (blind relay, pre-1.0) · Keyhive (convergent capabilities, pre-production) · yrs and cr-sqlite (engines with no ACL) · Willow/Earthstar (young Rust, ACL mid-migration) · Jazz/CoJSON (capability at validity, TS-first) · PSI (narrow) · Matrix state-resolution and SSB feeds as patterns · Polis and Talk to the City as algorithms |
| **REFUSE** | ElectricSQL · PowerSync · Rocicorp Zero · Triplit · InstantDB · Verdant (hub-and-spoke; server of record) · Ditto · Kialo (proprietary) · SearXNG (cautionary) |
| **WATCH** | Loro · Diamond Types · DXOS · Evolu · Fireproof · Braid-HTTP · PIR · SSE · Loomio |

Reconciliation with prior decisions: iroh is already the staged transport, and Holochain is already the collective-scale substrate; every TAKE above is a pattern or a library *under* those, never beside them. Nothing here reopens the ontology decision (no RDF), the content-graph decision (native resolver, not Cozo), or the UCAN ruling (attenuation law, refuse keypair root).

---

## 3. Seam 3 — the super aggregate: collective memory operations at pool scale

### 3.1 The seam named

The operator's phrasing: many stewards and many collectives expose *one* dataplane surface for collective memory operations at the scale the incumbents work at, over commons data pools stewarded at the elohim council level. The pools document says what a pool is and how it lives on committed compute. This section says how the *index* of a pool is stewarded, sharded, queried, and compacted, so that the shape in §0's last column is the same shape as its first.

### 3.2 What crosses upward, and what comes back

Upward, only what constitutional revealability allows: content contributed at Community, Public, or Commons rings under the epic's four conditions (contributed under declared reach, provenanced per unit, tended as counted care, explorer-blind); attested outcomes; tier digests. Never journeys, never `AttentionTending`, never a behavior log. Downward, results carrying the pool recipe's CID, per-unit provenance to the contributing holon, the custodian that answered, and standing printed as a shape. The query's receipt stays in the requester's holon.

### 3.3 The pool index as a council-stewarded measure

An index is a measure (§1.5). A pool's index is a measure whose declaration is a Collective EPR at council level, and whose bounds are the thing the council actually stewards:

| Bound | Declared by the council | Executed by |
|---|---|---|
| chunk rule, model CID, ranking method | in the measure; a change is a new version through the governance ceremony | elohim custodians under compute grants |
| reach floor of admissible shards | Commons (or Public for a narrower pool) | admission at contribution, checked against the four conditions |
| retention and compaction | fold cadence; digest form; demotion never deletion | custodians, on head change |
| query budget per requester | bytes, seconds, fan-out | the requester's own recipe, bounded by the pool's floor |

**Sharding.** The DHT already partitions by hash range into arcs. A pool's fold cache shards along the same arcs: each custodian, under a `delegates-compute` commitment and a `replicates-*` commitment, indexes the range it already holds. A query is a scatter over arcs under the pool's recipe and a gather under the same recipe. That is what "the scale the incumbents work at" means here: arcs times custodians, with no central index and no custodian that holds the whole. The pool's own pumps, in the epic's phrase, are the custodians' folds.

**Freshness.** A custodian re-projects its shard when a head in its arc moves; the reconciliation controller posture, not a crawl. The pool-level freshness measure is "how far behind the heads are the folds, per arc".

**Ranking.** Known methods only: BM25, vector similarity under the pinned model, graph adjacency from the native resolver, and the bridging classifier as a shape. Rank fusion is a recipe with a CID; standing is never summed. There is no learned ranker, on the epic's grounds and now on SuperLocalMemory's engineering evidence.

**Custody heat as care.** Which shards are drawn from is counted as REA care toward the custodians (the pools document's counted-care tending). Who drew is never recorded above the requester's holon. Explorer-blind is a property of the receipt's location, not a promise.

The point-by-point crosswalk against the incumbent's mechanisms (crawl, offline build, memory-resident index, sharding, fan-out, replication and hedging, tiering, early termination, caching, rerank, geography, freshness, query understanding, payment, sight, dispute), with labor and provenance answered on both sides, is in the pools document, §3a.

### 3.4 Who stewards what

- The **reader** stewards the recipe defaults on one machine.
- The **household** (smallest collective) stewards its index bounds and which devices share them.
- The **collective** stewards the index measure for its ring and admits its members' contributions.
- The **council** stewards the pool measure, the shard admission rule, and retention; it changes them only through the governance ceremony that earned reach already requires for high-stakes artifacts.
- The **elohim** execute faithfully as a utility: custodians index, answer, and re-project; they propose and never rule.

A dispute over a result has a path: the result's recipe CID names the method, the custodian field names who answered, the provenance names who contributed, and a Mishpat verdict can be sought against any of the three.

### 3.5 Compaction at scale

Digests per tier per holon, as the L3 row says. Demotion never deletion, `as_of` preserved, so the Living Memory epic's different right-to-be-forgotten holds at the council as it does on one machine: a fact that stopped being true remains findable as it was, and a person who withdraws reach (the DROP primitive from the data-broker pass) has their contribution demoted everywhere the fold reaches, with the demotion itself attested.

### 3.6 The visitor at pool scale

An outside engine, MemPalace-shaped or Google-shaped, may be a declared provider to a pool recipe with `ranking_known: false`, admitted under the six-part contract, and it is never the pool's index. The regenerative offering runs the other way: the public surface is free to crawl, and the fruits are held in trust and settled across a bridge, as the epic says.

The pools document's §3 build order, §4 risks, and §8 open questions are not restated; this section adds one build (B11) and two questions (Q4, Q5) to them.

---

## 4. What to build, in order

Small, dependency-ordered, each with the seam it lives in and a probe that would flip it green. Station numbering is the governed-discovery plan's.

| # | Seam | Build | Depends on | Probe |
|---|---|---|---|---|
| B1 | 1 | **Lexical provider on SQLite FTS5** behind the `Provider` trait; BM25 with `ranking_known: true` and a method CID | station 1 (trait lands) | a recall packet prints a lexical candidate with its method CID; no new crate in `Cargo.toml` |
| B2 | 1 | **Index as a measure**: one middot declaration carrying a reach bound, surfaces, chunk rule, model CID, retention — **vocabulary landed 2026-09-12** in `elohim/epr-rea/src/index.rs` (gate green); the receipt wiring is still open | B1 | the measure's CID appears in the receipt beside the recipe CID |
| B3 | 1 | **Semantic provider on sqlite-vec** with a pinned local model (llama.cpp/ggml or ONNX; model bytes CID'd in B2) | B2 | `recall-contract.json` `semantic_provider` no longer reads `mempalace`; a candidate carries the model CID |
| B4 | 1 | **Bi-temporal `as_of`** on `EpistemicStanding` and graph edges; demotion, never deletion | B2 | a query `as_of` yesterday returns a since-demoted fact with its demotion timestamp |
| B5 | 1 | ~~`AttentionTending` classification enum~~ **Withdrawn 2026-09-12:** the enum exists as real Rust (`elohim-storage/src/p2p/attention_tending.rs:61-70`, `Classification { ValuesForward, Fatigue, ScopeMismatch, Safety }`); the inventory's "declared only" was wrong. Remaining gap: the kind is not exported through `elohim-epr` for other crates | none | a consumer outside elohim-storage names the variants without a storage dependency |
| B6 | 1 | **Human lens negotiation** (`lens.rs:522`) | station 1 | a human `ReaderRef` resolves a stated level and the receipt records it |
| B7 | 1 | **Index freshness as reconciliation**: the controller re-folds on head change; retire the manual mine gate | B2, B3 | the session-start gate reads fold-lag, not files-newer-than-mine |
| B8 | 2 | **Household = smallest Collective EPR**, named by ring, no new kind | pools Q3 | a household index measure is declared as a collective's |
| B9 | 2 | **Reach at replication**: private entries + cap grants for inner rings; iroh-docs read caps per ring per holon; a `viewer` in `reach_earning::evaluate` | B8 | a2o: a household node requests a Trusted-ring record it lacks the grant for and is refused before bytes move |
| B10 | 2 | **Per-peer fan-out search** with bounded requests and home receipts | B1, B9 | two households search each other's Community ring; each receipt stays home; no aggregator process exists |
| B11 | 3 | **Pool index measure at council level**, arc-sharded folds under compute grants, scatter/gather under one recipe | B2, pools §3 pool primitive | a query to a two-custodian pool returns candidates naming both custodians and one recipe CID |
| B12 | 2 | **Bridging classifier** over qahal ratings, printed as a shape | none | a rating set yields "helpful across N clusters", never a rank |
| B13 | 1 | **Visitor test**: MemPalace enveloped (tevah) and attested (in-toto over bytes and model) | pools §5 contract | the admission ledger shows six of six for the palace |
| B14 | all | **Two invariants as habit probes**: join-liveness for every schema-guarded path in the recall executor; prior-distance wherever a posterior is ever introduced | none | a probe names where the data actually lives when a guard fails |

The largest single gap under the epic remains B9. The smallest runnable move is its viewer parameter.

---

## 5. Emergent risks in the middle

- **The shared device.** A household device is used by people at different rings (a child and a guardian; a guest). The index's reach ceiling is per reader, not per device, or the device becomes the aggregator the epic forbids.
- **The bystander.** Anything captured rather than authored indexes people who never consented (Recall, Rewind). Index atoms, never captures.
- **Silent consolidation.** Any background rewrite of a reader's model without a per-claim receipt is an injection vector (the "Dreaming" case) and a violation of principle 7.
- **The embedding model as a covert channel.** A hardcoded model that phones nowhere is safe today; a swappable one is a hop. Any hop beyond `Private` needs the Private Cloud Compute discipline: attestation of what code ran, published where a reader can check.
- **The doorway is a projection, never a holder.** Beelay's value is that it cannot decrypt. The doorway's guarantee is a different one: what it holds is a projection, notarized by the holonic authorities that govern its relationship to the people it serves, plural and federated in contract, so that nothing on the dataplane can be captured by any single doorway. A reach decision names the doorway as that contract, never as a "trusted tier" and never as a blind relay. Thinness is the guard: a doorway cheap enough that many operators run one is what prevents the fediverse's recentralization, where operators beg their audience for donations to pay a cloud. Anything that makes a doorway fat is a capture risk by that route.
- **Metadata reach.** Custody announcements, inventory gossip, and observation gossip are facts about a holon visible to peers. They need a ring like everything else.
- **Learned layers that stall silently.** Ten of ten in the closest prior art. If a posterior is ever introduced anywhere in discovery, B14's invariants go in first.

---

## 6. Open questions for co-authoring

Each with a recommendation; the operator decides.

**Q1. Is the household a Collective EPR at its smallest size, or its own kind?** Recommend: a collective, named by ring (§2.5). One kind at every size is the ladder's own invariant.

**Q2. Replace the palace's semantic route now, or after station 3?** Recommend: B1 immediately after station 1 (it is FTS5, nearly free), B3 before station 3, so that the standing reader's weekly routine is already running on a provider with a method CID.

**Q3. Reach at replication: DHT primitives, iroh-docs capabilities, or a Meadowcap-shaped layer?** Recommend: all three in that order (§2.2), each at the grain it is good at; no new protocol.

**Q4. Which embedding model is ours, and how is it pinned?** Recommend: a permissively licensed local model whose bytes are content-addressed and named in the index measure; a model change is a measure version, and every candidate names the model it was embedded under. The candidate models the survey names (nomic-embed-text, the MiniLM family, ggml-quantized alternatives) need a license and size pass before one is pinned.

**Q5. Does a household ever sync its *index* between devices, or only its content?** Recommend: only content, always. The libSQL embedded-replica pattern is attractive and is refused here: an index that travels is an aggregate that travels. Each device re-folds from what it holds.

**Q6. Rank fusion: is reciprocal-rank fusion a "sum"?** Recommend: no, when it orders candidates under a recipe with a CID and each producer's rank is printed beside the result; yes, and refused, the moment standing or any human signal enters the fusion.

**Q7. SQLite is right for the superlocal membrane; at the top membrane, do the doorway's MongoDB layers give the pools "big-data" capability?** Recommend: no, as the pool index, for two reasons that hold regardless of engine. First, authority. The doorway's Mongo holds the hosted account archive, custodial keys, the inter-replica signal bus, and the blob-metadata projection store ✅ (`doorway-service/src/{custodial_keys,signal/bus_mongo,cache/tiered}.rs`), and every one of those is a *projection*: the doorway is not its own thing, what it may hold is notarized by the holonic authorities that govern its relationship to the people it serves, and it is plural and federated in contract so that no single doorway can capture what lives on the dataplane. A pool index is a council-stewarded measure (§3.3), a different authority; a doorway may hold a pool shard only as a custodian under the pool's grant, like any other node. Second, thinness. The goal of federation here is to be *thin*: cheap, easy, and feasible for many operators, precisely to avoid the fediverse's recentralization, where operators must beg their audience for compute donations to pay a cloud. A warehouse-grade database is what makes a doorway fat. Scale in this design is *arcs times custodians* (§3.3), not one larger node, and the shard format (SQLite with FTS5 and sqlite-vec, the same on a phone, a rack, and a doorway) is what keeps a doorway cheap enough to be one custodian among many. SQLite is not the scaling bet; the measure is, and plural thin doorways are its custodians. What the top membrane does need: scatter/gather (designed); analytic folds as compute-to-data jobs with a safe-outputs check, run where the shards are, on columnar embeddable engines (Parquet with DataFusion or DuckDB; PMTiles for tiles), returning digests; and larger custodians (a doorway pool hosting many cells, a home storage rack) holding larger shards under the same grant discipline, free to run larger *derived* projections behind the canonical fold, proven complete by attestation, in exactly SuperLocalMemory's canonical-plus-verified-projection pattern. A doorway that wants to serve pool queries fast does so as a custodian under a grant, never as the pool's warehouse.

---

## 7. Mint outputs (folded 2026-09-12)

**Folded:** measure-family borrows rows 26–27 · commons-holonic stewardship rows 27–29 · dataplane borrows rows 12–15 · confidentiality plane rows 10–11 · scale risk rows 9–10 · agentic-context-tooling queue item 24. Consolidating spec: [memory, search and scale — the three seams](epr:memory-search-scale-three-seams-design) (its §6 is the seam register, the dedupe guard for this whole space). The pools and search documents' own §9/§10 proposals remain the operator's mint. The table below is the map that was folded.

| Cluster | Row |
|---|---|
| recall-reaches-authority | B1–B7 as stations after station 3 of the governed-discovery plan; B14 as probes on the recall executor's habit |
| reach vocabulary drift (OWL §6 plan) | B8 (household = collective by ring) and B9 (viewer in `evaluate`); the a2o scenario in B9's probe |
| dataplane / replication | B9's replication-boundary refusal; metadata reach for gossip surfaces (§5) |
| commons pools (pools doc §3) | B11 as the pool index measure; Q4 and Q5 as additions to the pools' §8 |
| qahal / mishpat | B12 bridging shape |
| external tooling admission | B13 visitor test closure |
| living memory | B4 bi-temporal demotion as the mechanical form of the epic's forgetting |

---

## Appendix A — Seam-1 horizon ledger (verbatim, Sonnet survey, 2026-09-12; ◐ unless marked)

*Reconciliation notes:* the three 2026 arXiv entries (SuperLocalMemory 4.0, PROJECTMEM, MOSS) were fetched and confirmed by the orchestrating session ✅; SuperLocalMemory's license is AGPL-3.0 (the ledger's "open-source" is corrected above in §1.4); every other row is single-source.


| # | Tool | What | License | Store/index | Offline | Scoping | Provenance on results | Household scale | Verdict |
|---|---|---|---|---|---|---|---|---|---|
| 1 | **Letta (MemGPT)** | OS-style tiered agent memory (core/archival/recall) | Apache-2.0 | Pluggable vector DB + relational DB | Yes, self-hostable | Per-agent; background "sleep-time" agent rewrites blocks | No — opaque LLM-rewritten prose | None native | **TAKE-DISC** — tiered model is a good frame; the LLM-rewrite step is the "sum not print" anti-pattern to avoid |
| 2 | **Mem0** | Add/search SDK over LLM-extracted facts | Apache-2.0 | 20+ vector backends; SQLite audit trail of ops | Yes, self-hostable | Caller-supplied user/agent id | Partial — ops logged, not per-fact | None built-in | **TAKE-DISC** — audit-sidecar pattern worth copying; extraction itself is opaque summarization |
| 3 | **Zep / Graphiti** | Temporal knowledge graph, every edge has a validity window | MIT (Graphiti) | Neo4j/FalkorDB graph + embeddings | Yes if self-hosted | Session/user graph namespaces | Yes — became-true/invalidated-at on every edge | None native | **TAKE** — temporal-edge invalidation is the closest prior art to our freshness/recall-contract semantics |
| 4 | **Cognee** | Graph-vector hybrid memory framework | Apache-2.0 | Pluggable graph DB + vector DB | Yes | Per-pipeline/dataset | Not first-class | None documented | **WATCH** — architecture rhymes with ours but under-documented on governance |
| 5 | **LangMem** | Extract/consolidate/optimize SDK for agent memory | MIT | Bring-your-own store | Yes if store is local | (user, thread) tuples | No | None native | **WATCH** — thin library; useful as a verb-split reference only |
| 6 | **Honcho** | Background reasoning builds a durable model of user psychology | Unclear (no explicit OSS license found) | Server DB, hosted or self-hosted | Cloud-first design | Per-"peer" | No receipt on derived claims | No | **REFUSE** for our private tier — durable psych-profiling with no provenance discipline, contra "thoughts private, only said/done judged" |
| 7 | **LanceDB** | Embedded columnar vector DB, multimodal | Apache-2.0 | Lance format; HNSW/IVF + FTS + SQL | Yes, in-process | N/A (storage only) | N/A | File-sync only | **TAKE** — embeddable Rust-native ANN+FTS+SQL substrate; no memory semantics to inherit |
| 8 | **sqlite-vec** | SQLite vector-search extension, pure C | MIT/Apache-2.0 | SQLite virtual table, brute-force + quantization | Yes, anywhere SQLite runs | N/A | N/A | N/A | **TAKE** — simplest local vector index; clean fit for a Rust/SQLite stack |
| 9 | **libSQL/Turso vector** | SQLite fork, native vector column + DiskANN | MIT | Embedded/server libSQL | Yes, embedded replicas offline | N/A | N/A | Edge-replica ↔ primary sync built in | **TAKE** — embedded-replica sync is a plausible template for private→self→intimate growth |
| 10 | **DuckDB VSS** | HNSW extension on DuckDB arrays | MIT | DuckDB + usearch-derived HNSW | Yes, single-file | N/A | N/A | N/A | **WATCH** — index WAL-recovery admittedly incomplete; not durable-store-safe yet |
| 11 | **tantivy** | Rust full-text search library (Lucene-like) | MIT | Custom inverted index, embeddable | Yes | N/A | N/A | N/A | **TAKE** — natural Rust lexical complement to a vector index; candidate for the lens keyword layer |
| 12 | **Meilisearch** | Turnkey full-text+hybrid search server | MIT | Own LMDB-based index | Yes, self-hosted binary | Per-index | No | Single-node only | **TAKE-DISC** — good household-facing search UX, but a server process, heavier than tantivy for the private tier |
| 13 | **Quickwit** | Log/trace search on object storage, stateless compute | Apache-2.0 | Tantivy segments on S3-compatible storage | Can run local, built for object storage | N/A | N/A | Cluster many-writer/reader shape | **REFUSE** for this scope — wrong problem (log scale); Datadog acquired it in 2025 |
| 14 | **usearch (Unum)** | Single-file, multi-language ANN engine | Apache-2.0 | Header-only HNSW-like graph | Yes | N/A | N/A | N/A | **TAKE** — smallest-footprint ANN option; worth a bake-off vs LanceDB |
| 15 | **hnswlib** | Reference HNSW implementation | Apache-2.0 | In-memory HNSW graph | Yes | N/A | N/A | N/A | **WATCH** — the algorithm everything else here reimplements; no reason to depend on directly |
| 16 | **Chroma** | Embedded/server vector DB for RAG | Apache-2.0 | Own engine, in-process or client-server | Yes, embedded mode | Per-collection | No | None built-in | **WATCH** — no differentiator over LanceDB/sqlite-vec; Python/JS-first, weak Rust story |
| 17 | **Khoj** | Self-hosted personal AI assistant over notes/docs | AGPL-3.0 | Configurable vector store | Yes, self-hostable | Single-user | No | Not documented | **WATCH** — AGPL network clause is a real constraint if ever exposed across household devices |
| 18 | **Obsidian Smart Connections** | Note-linking plugin, on-device embeddings | Source-available, not OSI-open | Local BGE-micro model + local vault index | Yes, notes never leave device by default | Single-vault = single-person | No | None; vault is one person's | **TAKE-DISC** — "privacy is not a premium feature" is exactly our private-tier bar; pattern worth copying, license too closed to depend on |
| 19 | **Logseq** | Local-first outliner/PKM, plain-text Markdown/Org | AGPL-3.0 | Flat files + in-memory graph query | Yes | Single-user graph | No | File-sync only, no CRDT merge | **WATCH** — validates plain-text-as-truth, but the missing CRDT story is a gap our design must not inherit |
| 20 | **Rewind.ai** (cautionary) | Continuous local capture + AI recall; acquired by Meta/Limitless late 2025 | Proprietary | Local store, shifted to cloud sync post-acquisition | Was local-by-default, broke it on acquisition | Single-user, always-on | No | No bystander consent model | **REFUSE** (cautionary) — "local-first" survives only as long as the company's incentives hold; also no consent for co-present others |
| 21 | **Microsoft Recall** (cautionary) | OS-level periodic screenshotting + on-device search | Proprietary | Local encrypted DB, decrypted "just in time" via Windows Hello | Yes, local processing | Single-user; captures anyone visible on screen | No | Explicitly broken: non-Recall users get captured too | **REFUSE** (cautionary) — capture-everything defeats the private tier by capturing OTHERS' content without their consent; argues for gating at authorship, not capture device |
| 22 | **Apple Private Cloud Compute** (pattern) | Verifiable-privacy offload tier for on-device AI requests | Proprietary server | Stateless nodes, no persistent storage | No — the "when local isn't enough" escape hatch | Per-request, never persisted | Yes — cryptographic attestation + published transparency log of server code | N/A | **TAKE (pattern only)** — best available model for "print, never sum": a result carries a verifiable statement of what code produced it |
| 23 | **llama.cpp / Ollama** | Local LLM inference + embeddings runtime | MIT (both) | N/A — feeds any vector store | Yes, this is the point | N/A | N/A | Per-device instance, no shared state | **TAKE** — settled correct choice for local embedding generation; Ollama wraps llama.cpp's ggml |
| 24 | **Anthropic Claude memory** (pattern) | Cloud structured-fact memory, per-Topic UI | Proprietary | Anthropic servers | No, cloud-only | Per-account, unified chat/Cowork | Partial — per-topic edit/delete, no per-fact receipt | N/A | **WATCH (pattern)** — per-topic UI and default-excluded sensitive categories are a good precedent for reach/lens tiers; storage itself out of scope |
| 25 | **OpenAI ChatGPT memory ("Dreaming V3")** (pattern) | Background synthesis rewrites a persistent user model | Proprietary | OpenAI servers, separate layer injected at inference | No, cloud-only | Per-account, project/chat toggles | No — 2024 "SpAIware" injection wrote a persistent exfil instruction into memory | N/A | **REFUSE (cautionary pattern)** — silent background rewriting with no per-claim provenance is a demonstrated injection vector |
| 26 | **A-MEM** (arXiv:2502.12110) | Zettelkasten-style agentic memory, atomic notes with dynamic linking | Code MIT-style (agiresearch/A-mem) | Any vector store; structured note attributes | Yes if store is local | Per-agent | Attributes legible, but link-generation is LLM-authored, unreceipted | Not addressed | **TAKE-DISC** — atomic-note-with-typed-links resembles our recipe/habit-atom shape; good citation for atomic-over-monolithic |
| 27 | **Generative Agents / MemoryBank** (papers) | Append-only memory stream + recency/importance/relevance retrieval + reflection; Ebbinghaus-decay updates | Research code, various | Simple vector/text store | Yes, reference impls are local | Per-simulated-agent | No | Not addressed | **TAKE (citation only)** — theoretical ancestor of retrieval scoring and decay-based forgetting; cite for our fatigue/scope-mismatch lens classes |
| 28 | **Ink & Switch, "Local-first software"** (2019 essay) | Defines 7 ideals (fast, multi-device, offline, collaboration, longevity, privacy, user control); names CRDTs as foundation | CC-BY (text) | N/A | N/A | N/A | N/A | Treats multi-device/collaboration as core, not afterthought | **TAKE** — canonical citation; our non-negotiables restate ideals 3/6/7 directly |
| 29 | **SuperLocalMemory 4.0** (arXiv:2608.08253, 2026) | Governed memory OS: hybrid retrieval, RBAC, bi-temporal recall, verifiable transactions, audit trails, EU AI Act checklist | Open-source (qualixar/superlocalmemory) | SQLite-based local store + hybrid indices | Yes — "no cloud provider in the memory path" | Personal/shared/global scopes with RBAC | Yes — hash-checkable completion manifests, verify/compensate/erase transaction owners | Explicit design target, multi-tenant isolation | **TAKE** — closest prior art to our governed-recall executor; nearly our recipe/receipt/reach shape, independently converged; read closely before finalizing our schema |
| 30 | **PROJECTMEM** (arXiv:2606.12329, 2026) | Local-first event-sourced memory + judgment layer for coding agents | MIT (riponcm/projectmem) | Plain-text event log (grep/diff/git-native), no vector DB | Yes — "no cloud, no telemetry" | Per-project | Yes by construction — deterministic projection, no LLM in retrieval | Not addressed | **TAKE** — validates "print, never sum": event-sourcing + deterministic projection + a pre-action gate close in spirit to our fatigue/scope-mismatch classifier |
| 31 | **MOSS** (arXiv:2607.04391, 2026) | Agent-driven retrieval over a relational DB instead of embedding similarity; symbolic, reproducible | Not confirmed OSS | Relational DB, model/storage-agnostic | Yes, can run local | Single-scholar corpus in the reported deployment | Yes — every retrieval step logged/inspectable, no LLM in the loop | Not demonstrated | **TAKE-DISC** — "auditable by construction" is the strongest match to "print, never sum"; confirm license before citing as adoptable |

### Patterns

- **Convergence on hybrid retrieval, not one index.** Zep, Cognee, SuperLocalMemory, MOSS all fuse vector + lexical + graph/relational + temporal rather than betting on embeddings alone — supports pairing tantivy/sqlite-vec/LanceDB rather than picking one.
- **2026 is visibly correcting toward auditability.** SuperLocalMemory, PROJECTMEM, and MOSS all explicitly reject "vector similarity as the whole answer" for deterministic, inspectable retrieval — the field is arriving at "print, never sum" independently, which validates the recipes/receipts design.
- **Only Apple PCC proves "no cloud" cryptographically.** Every local-memory tool asserts a no-cloud policy; PCC alone backs it with attestation and a published transparency log. Our reach model needs an equivalent proof for anything crossing private→self.
- **Household/multi-person scale is almost universally unsolved.** Only SuperLocalMemory names multi-scope RBAC (personal/shared/global) as first-class; everyone else ignores the case (Mem0, Letta, Chroma) or fakes it with file-sync (Logseq, Obsidian), which has no real conflict resolution.
- **Recall and Rewind both broke on the same axis: consent of people who aren't the device owner.** No surveyed tool solves this by construction except by exclusion (don't capture continuously) — a hazard to name explicitly in our own design, not just theirs.
- **Cloud vendor memory is converging on background "dreaming"/consolidation with no per-claim receipts**, and this is already a demonstrated prompt-injection vector (OpenAI's 2024 SpAIware). Strongest argument against silent memory consolidation anywhere in our design.
- **Licensing is friendlier at the infra layer than the app layer.** Apache-2.0/MIT dominate vector/search libraries (LanceDB, sqlite-vec, tantivy, usearch, hnswlib, llama.cpp); the application layer skews AGPL-3.0 (Khoj, Logseq) or source-available (Smart Connections) — fine to study, not to embed code from without review.
- **No surveyed tool implements a graduated reach model (private→self→intimate→…→commons) as a first-class primitive.** RBAC and per-topic exclusion are the closest analogs, both flat compared to our tiers — this looks like a genuinely novel contribution, not something to import.


---

## Appendix B — Seam-2 horizon ledger (verbatim, Sonnet survey, 2026-09-12; ◐ throughout)

*Reconciliation notes:* every row is single-source web research; the kitsune2 row is the surveyor's outside reading of our own substrate and is kept as such; the Automerge and iroh rows describe libraries already in the tree (§2.1), so their TAKE is a confirmation, not an adoption.


### Ledger

| Entry | What/License/Lang | Consistency | Access control | Server? | Provenance | Household→Collective | Verdict — why |
|---|---|---|---|---|---|---|---|
| **[Automerge 3](https://automerge.org/blog/automerge-2/)** | JSON-CRDT doc lib; MIT; Rust core+WASM/JS/C | Op-based CRDT, columnar storage since v3 (10x mem cut: 700MB→1.3MB doc) | None built-in — pure data structure | No | Per-op actor-id + causal history | Household: great, edge-viable now. Collective: many-doc fan-out was the weak point (see Beelay) | **TAKE** — matches our existing "lit" Automerge use; v3 removes the memory wall |
| **[Beelay](https://github.com/automerge/beelay)** | Sync relay for many Automerge docs; MIT/Apache-2.0; Rust | Sync layer only; "sedimentree" range sync | Relay forwards only what Keyhive authorizes, never decrypts | Optional untrusted relay | Delegated from Keyhive chain | Household: relay=home hub. Collective: relay can be untrusted (commons tier) | **TAKE-DISC** — right shape (blind forwarder) but pre-1.0, 2026 |
| **[Keyhive](https://www.inkandswitch.com/keyhive/notebook/)** (né Beehive) | Local-first access-control CRDT+E2EE; license TBD (Ink&Switch, likely MIT/Apache); Rust | "Convergent capabilities" — CRDT group membership, coordination-free revocation | BeeKEM group-key agreement gates decryption itself, before any read | No | Signed delegation chains, PCS+FS | Household: natural fit. Collective (1000s): BeeKEM claims log-perf common case | **TAKE-DISC** — closest thing to "reach enforced at replication"; pre-production |
| **[Yjs](https://docs.yjs.dev/)/[yrs](https://github.com/y-crdt/y-crdt)** | Sequence/map CRDT; MIT; `yrs` full Rust port, WASM/FFI/Python/Swift/.NET | Op-based CRDT (YATA), binary-compatible across ports | None — bring your own auth | Only for relay/awareness | None built-in | Household: great single-doc perf. Collective: no multi-doc/group/capability model | **TAKE-DISC** — excellent embeddable engine, ACL entirely bolt-on |
| **[Loro](https://loro.dev/)** | CRDT lib (text/list/tree/map), git-like history; MIT; Rust core, WASM/Swift/Python | Op-based CRDT + Fugue anti-interleaving; native movable-tree CRDT | None built-in | No | Inspectable op history, no signed identity | Household: strong editor fit. Collective: no sync-transport/ACL yet | **WATCH** — best-in-class primitives (rare tree CRDT), but library not system |
| **[Diamond Types](https://github.com/josephg/diamond-types)** | Fastest text CRDT; permissive license; Rust, JSON WIP | Op-based, range-tree + heavy RLE | None | No | Causal graph retained | Text-only; not evaluated at collective scale | **WATCH** — speed-research project, single-type, no ACL/sync |
| **[Willow protocol](https://willowprotocol.org/)** | Sync/data-model spec (namespace=time×path×subspace); open spec, `willow-rs` Apache-2.0/MIT; Rust | Range-based set reconciliation (iroh-docs' algorithm family), not a general CRDT | Capabilities first-class via paired Meadowcap | No (peer protocol) | Entries signed by subspace keys | Explicitly designed for small groups scaling up | **TAKE-DISC** — closest published protocol to reach-at-sync-boundary; young Rust impl |
| **[Meadowcap](https://gwil.garden/posts/meadowcap-intro.html)** | Capability system for Willow; same license family; TS+Rust | N/A (capability layer) | **At sync**: unforgeable tokens bound to a 3D sub-range — peer cannot request/serve out-of-capability data | No | Capability chains = provenance | Delegation composes household→collective naturally | **TAKE** — matches our reach-tiers exactly: gated at replication range, not render |
| **[Earthstar](https://earthstar-project.org/)** | Small-group local-first store (docs over Willow); LGPL-3.0 core; TS + shared willow-rs | Willow-based, LWW per path | Pre-Willow: weak shared-secret "shares"; Willow/Meadowcap migration in progress | No (P2P + optional pub relay) | Author keypair per doc | Explicitly a "share"≈household/community; not built for hundreds+ | **TAKE-DISC** — closest existing product to our target; ACL mid-migration |
| **[p2panda](https://p2panda.org/)** | P2P toolkit: append-only logs + `p2panda-encryption`; MIT/Apache-2.0; Rust native+WASM | Log-based CRDT (per-author append-only) + CRDT group membership for encryption | DCGKA group-key agreement (data + Signal-like ratchet); access=key possession, gated pre-decrypt | No | Every entry signed; causal log=provenance | Explicitly small-group local-first, composable upward | **TAKE** — production Rust, explicit local-first group encryption w/ PCS/FS |
| **[iroh-docs](https://github.com/n0-computer/iroh-docs)** | Multi-dim KV "replica" sync over iroh; Apache-2.0/MIT; Rust, `redb`-backed | Range-based set reconciliation, LWW per key | Namespace keypair=write cap; read-cap keys exist but whole-namespace grain, coarser than Meadowcap | No — direct peer sync over iroh QUIC | Double-signed (namespace+author) | Good for household doc sets; namespace-level grain limits fine reach tiers | **TAKE** — we already use iroh for blobs; pair with app-level ACL, don't rely alone |
| **[ElectricSQL](https://electric-sql.com/)** | Postgres→SQLite read-path sync via WAL "Shapes"; Apache-2.0; Elixir service | Not CRDT — Postgres is sole truth, shape-filtered replication | Shape query + Postgres RLS, decided server-side | **Yes, required** | None P2P | Central-Postgres model, no fit for no-server-middle | **REFUSE** — hub-and-spoke since 2024 rewrite (dropped CRDTs); shape-pattern reference only |
| **[PowerSync](https://powersync.com/)** | Postgres/Mongo/MySQL↔SQLite sync, "Sync Streams"; client SDKs Apache-2.0, service source-available FSL | Server-authoritative; local writes replay to server | Row-level rules/RLS server-side | **Yes, required** | None P2P | Same hub-and-spoke shape | **REFUSE** — central authority + FSL server fails our open-license bar |
| **[Rocicorp Zero](https://zero.rocicorp.dev/)** (successor to Replicache) | Full-stack sync engine, own query lang ZQL+server; Apache-2.0; TS | Optimistic client cache + server rebase-on-conflict, not CRDT | Permission model runs on server per query/mutation | **Yes, required** | None P2P | One-backend model, not peer households | **REFUSE** — same shape; good UX reference (optimistic/rebase) only |
| **[Jazz](https://jazz.tools/)** (CoJSON) | Local-first framework: CoValues (CRDT maps/lists/streams) + built-in groups/E2EE; MIT; TS (Rust core not primary) | Op-based CRDT (CoJSON) | Groups are first-class CRDTs holding roles (reader/writer/admin); `determineValidTransactions()` checks signatures before validity, not just at render | No (sync workers optional, self-hostable) | Every transaction signed inside a Group | Groups nest (household→group-of-groups) — matches reach-tier ladder | **TAKE-DISC** — best product-shaped match for capability-at-validity-boundary; TS-first, young |
| **[DXOS](https://dxos.org/)** (ECHO/HALO) | P2P framework: ECHO=Automerge graph DB, HALO=decentralized identity; MIT core (contributions FSL-1.1-Apache-2.0); TS | Automerge CRDT under the hood | HALO manages per-space keys; space invites gate join, but mostly all-or-nothing within a space | No (WebRTC P2P, optional signaling) | Identity=keypair via HALO + Automerge history | Space≈household; federating spaces for collective use is coarse | **WATCH** — solid identity/CRDT combo, ACL is space-level not reach-tier; TS-only |
| **[Evolu](https://www.evolu.dev/)** | Local-first SQLite w/ E2EE backup/sync; MIT; TS, SQLite-WASM | Custom timestamp-CRDT merge over SQLite rows | E2EE to a relay that can't read; no multi-party roles — single-owner-multi-device design | Optional dumb relay | Signed per-mutation timestamps | Household (single owner) yes; collective sharing out of scope | **WATCH** — good pattern, but not multi-party; no fit for collective tiers |
| **[cr-sqlite/vlcn](https://vlcn.io/)** | SQLite extension: CRDT columns (LWW/fractional-index/OR-set) + causal log; MIT; C/Rust core | Per-column CRDT merge (row=map of CRDTs) | None built-in — BYO network/auth | No | Causal length/site-id per change | Scale-agnostic mechanically, but zero ACL/transport shipped | **TAKE-DISC** — valuable drop-in CRDT-in-SQLite layer; we'd build ACL/transport ourselves |
| **[Ditto](https://ditto.live/)** | Commercial edge-sync, BLE/WiFi/LAN mesh+CRDT; proprietary EULA; C++/multi-SDK | CRDT strong eventual consistency | Vendor-controlled, opaque | No cloud, but vendor SDK dependency | Vendor-internal | Genuinely strong at household/mesh scale (its market) | **REFUSE** — proprietary EULA violates our open-license non-negotiable |
| **[Fireproof](https://use-fireproof.com/)** | Embedded doc DB, Merkle-CRDT + encrypted sync, browser-first; Apache-2.0/MIT; TS, no WASM | Merkle-CRDT (IPFS-flavored causal DAG) | Encrypted, but access = "who has the key," no role model | No (offline-first; sync optional/pluggable) | Cryptographic causal integrity built-in | Good single-app/household fit; no group-role concept | **WATCH** — nice embeddable DB w/ real provenance; TS-only, no capability layer |
| **[Braid-HTTP](https://braid.org/)** | IETF draft: HTTP versioning/patches/subscriptions; open draft; protocol not a lib | Protocol-agnostic — carries whatever merge semantics the app defines | None specified — app concern | Can run P2P, but most demos client-server | Not addressed by spec | Scale-agnostic transport idea, doesn't solve reach | **WATCH** — interesting transport idea; our libp2p/iroh already cover this need |
| **[Verdant](https://github.com/a-type/verdant)** | IndexedDB storage+sync+realtime for web apps; MIT; TS | Custom per-project CRDT-ish sync | App-defined; `@verdant-web/server` is sync authority | **Yes** (dedicated server) | None P2P | Small-team/hobby scale | **REFUSE** for our tier — server-mediated by design; DX prior-art only |
| **[Triplit](https://www.triplit.dev/)** | Full-stack syncing triple-store (server+client); source-available, not fully OSS | Server-mediated sync, schema-declared roles | Schema rules referencing JWT claims, server-side; can evict client cache | **Yes, required** | None P2P | One-backend-many-clients | **REFUSE** — central server + non-open license both fail our bar |
| **[InstantDB](https://github.com/instantdb/instant)** | BaaS w/ real-time triple-store sync; Apache-2.0; Clojure server, JS/RN clients | WAL-tailed Postgres→client cache, server=truth | CEL-based rule language, evaluated server-side | **Yes, required** | None P2P | Single shared backend | **REFUSE** — hub-and-spoke; CEL rule-language is the only reusable idea |
| **[Matrix](https://matrix.org/)** (pattern) | Federated chat/state protocol; Apache-2.0; multi-language | Room state = DAG resolved via deterministic state-resolution (CRDT-adjacent) | Power-levels model roles; join rules gate membership; E2EE (Olm/Megolm) separate from state ACL | **Yes** — homeservers federate, not serverless P2P | Every event signed by homeserver+sender | Federation is collective-scale by design, proven, but heavier than our target (server per household) | **TAKE-DISC** (pattern only) — state-res/power-levels worth studying; kitsune2 is our lighter analog |
| **[Secure Scuttlebutt](https://scuttlebutt.nz/)** (pattern) | Append-only signed feed+gossip mesh; mixed OSS licenses; JS mainly, some Rust ports | Per-author append-only log, gossiped opportunistically; no cross-author merge | None at protocol layer — public-by-default; private msgs bolted on via box2 crypto | No (fully P2P, optional pub relay) | Ed25519-signed feed = full provenance | Household/friend-mesh is its sweet spot; collective/public scale historically strains | **TAKE-DISC** (pattern only) — proven simple shape resembling our own P2P layer; its missing capability system is the gap we must not repeat |
| **[Holochain kitsune2](https://github.com/holochain/kitsune2)** (as reported, our own substrate) | Gossip/DHT layer, Rust; peer-selection-aware gossip (arc+recency) | Eventually-consistent DHT via validated actions, not a CRDT text type | DHT validation rules (our zomes) + capability grants enforce at action-validation boundary | No (P2P DHT peers) | Signed action, source-chain provenance | Already our collective-scale substrate; open Q is whether reach tiers are kitsune2-level policy or only app-filtering | **already-adopted; audit** — survey confirms nobody external beats a well-configured DHT+capability-grant discipline here |
| **PIR** — [Google private-retrieval](https://github.com/google/private-retrieval), OpenMined, [ZipPIR](https://arxiv.org/pdf/2603.09190)/Spiral-class | Cryptographic "fetch item i without revealing i"; Apache-2.0 (Google); C++/research, no Rust-native production lib | N/A (query-privacy, not data consistency) | N/A — orthogonal to access control | Needs a server holding full DB | N/A | Compute cost (GPU-class 2026 papers) impractical at household scale today | **WATCH** — real, but server-shaped and heavy; reach-tier filtering is our practical substitute |
| **PSI** — [OpenMined PSI](https://github.com/OpenMined/PSI), Facebook Private-ID, PSIttacus | Set-intersection without revealing non-matches; Apache-2.0/MIT class; Rust bindings exist (OpenMined) | N/A (matching primitive) | N/A | Peer-to-peer capable, no third server | N/A | Useful for "do peer A/B overlap in contacts/capabilities" without disclosure | **TAKE-DISC** — genuinely Rust-embeddable, serverless; prototype for narrow overlap-discovery, not general search |
| **SSE** — Clusion, OpenSSE (Sophos/Diana/Janus) | Encrypted-index keyword search over ciphertext; GPLv3/AGPLv3; Java/C++, no maintained Rust port | N/A (query-privacy over untrusted index-holder) | Confidentiality from index-holder, not multi-party roles | Assumes semi-trusted server holding index | N/A | Single-owner-outsources-storage model, not multi-peer household search | **WATCH** — right category, wrong license family and unmaintained; would need from-scratch Rust scheme |
| **[SearXNG](https://docs.searxng.org/)** (cautionary) | Self-hosted metasearch proxy; AGPL-3.0; Python | N/A | Trust concentrated in whoever runs the instance — a privacy-respecting **central proxy** | **Yes, required** | None | Doesn't map to peer/household federation | **REFUSE/cautionary** — re-centralizes trust; confirms true federated search needs per-peer fan-out, not one aggregator |
| **[Polis](https://compdemocracy.org/polis/)** | Opinion-clustering + bridging-statement discovery over a vote matrix (PCA+k-means); AGPL-3.0; Node/Python/R | N/A (analysis over live vote matrix) | Server-side only, no offline mode | **Yes, required** | Votes logged per participant, analysis centralized | Proven at national/large-collective scale (Taiwan); no household tier | **TAKE-DISC** — the bridging *algorithm* is the reusable idea; tool itself is centralized |
| **[Talk to the City](https://ai.objectives.institute/talk-to-the-city)** | LLM-based qualitative-response clustering/sensemaking; open source (AI Objectives Institute); Python | N/A (analysis pipeline) | None — assumes local/central corpus already | Self-hostable pipeline (calls LLM API or local model), no built-in network sync | Not addressed | Built for consultation scale (hundreds–thousands); needs downsizing to household/collective | **TAKE-DISC** — self-hostable and open; wire into our own data plane rather than as a network service |
| **[Loomio](https://github.com/loomio/loomio)** | Collaborative decision-making/proposal-voting app; AGPL-3.0; Ruby on Rails | N/A (server-hosted deliberation state) | Group-membership based, server-enforced | **Yes, required** | Decisions/votes logged per user | Built for org scale (tens–low-hundreds); one instance per org | **WATCH** — best fully-open self-hostable decision tool; server-backed Rails app, UX/workflow reference only |
| **Kialo** | Structured debate/argument-mapping platform; **proprietary**; web app | N/A | Vendor-controlled | Yes (SaaS) | N/A | Popular at classroom/community scale | **REFUSE** — closed source; Loomio/Polis are the open substitutes |
| **[Hypothesis](https://github.com/hypothesis/client)** | Web annotation layer, W3C Web Annotation Data Model; BSD-2-Clause; Python server (`h`) + TS client | N/A (discrete annotation records, not merged docs) | Server-side groups/permissions (public/private/group-scoped); no offline/P2P mode | **Yes** for hosted product; the W3C data model itself is transport-agnostic | Each annotation attributed + timestamped | Group-scoped model is genuinely household/collective-shaped; implementation centralized | **TAKE-DISC** — adopt the W3C data model (targets+bodies+permissions) verbatim; don't run their centralized server |
| **[Community Notes bridging algorithm](https://github.com/twitter/communitynotes)** | Matrix-factorization ranking surfacing notes helpful across opposed rater clusters; Apache-2.0; Python | N/A (ranking algorithm over a ratings matrix, not a sync system) | None — assumes one global ratings table exists already | Needs full corpus server-side to factorize | Every rating attributed to a contributor id | Math is scale-agnostic even though reference deployment is planetary | **TAKE** — most directly reusable *algorithm*: run bridging-scores over our own local vote/rating data (qahal poll, mishpat dispute), no need for X's infra |

### Patterns

- **Convergence on "capability CRDT + E2EE relay that can't read"**: Keyhive/Beelay, Jazz/CoJSON, and p2panda-encryption independently landed on group-membership-as-CRDT gating a ratchet/symmetric key, with an untrusted relay moving ciphertext it never decrypts — the strongest signal for meeting reach tiers at the replication boundary, not render.
- **Meadowcap/Willow is the only spec gating sub-ranges, not whole docs/namespaces.** Everything else (iroh-docs, DXOS spaces, Jazz groups) grants at document/namespace/space granularity. Our six-tier reach ladder needs the sub-document granularity Meadowcap already designed for.
- **The commercial sync-engine wave (ElectricSQL, PowerSync, Zero, Triplit, Instant, Verdant) converged on hub-and-spoke** — opposite of our no-central-index requirement. None fit our private tier; their permission-rule languages (CEL, schema-rules) are the only reusable part.
- **Nobody has shipped practical private search over a P2P mesh.** PIR is real but server-shaped and compute-heavy; SSE implementations are unmaintained and wrong-licensed; PSI is Rust-embeddable and serverless but solves only narrow set-overlap. Reach-tier filtering remains our practical substitute for cryptographic private search.
- **Sensemaking-at-scale tools (Polis, Talk to the City, Community Notes) are centralized-corpus algorithms wrapped around ordinary tech** — value is entirely in the ranking math, trivially portable to our own local vote/rating data. Adopt the algorithm, not the tool.
- **Federated-search's cautionary case (SearXNG) generalizes**: any aggregator, even self-hosted and privacy-respecting, just relocates the central point of trust. No working open-source per-peer fan-out search was found.
- **Matrix and SSB are the two proven append-only/gossip precedents at real scale**, and both confirm our own gap: power-levels and follow-graphs are coarse, bolted-on access models — neither enforces "who may replicate what" at the sync boundary the way Meadowcap/Keyhive attempt.
- **Automerge's own trajectory (3.0 → Beelay → Keyhive) is the most complete single path to our requirement** — doc CRDT, many-doc sync, capability-gated E2EE, all Rust-native and open-licensed — but pre-1.0 across the newer layers; track it, don't depend on it yet.
