---
title: "Search, Discovery and the Commons — how the incumbents' machines work, and what a reach-first p2p substrate can and cannot answer with"
status: Capture (co-authored; the operator is learning this domain alongside the synthesis — sections marked ❓ are open for their hand)
date: 2026-09-11
serves: recall-reaches-authority
realizes:
  - genesis/docs/content/elohim-protocol/search/epic.md (the search epic — vision first, this document second, by design)
informed-by:
  - genesis/docs/superpowers/specs/2026-09-11-governed-discovery-journey-lens-graduation-design.md
  - genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md
  - genesis/data/timeline/backlog/progressive-discovery-substrate-emergent-concern.md
---

# Search, Discovery and the Commons

*The research companion to the search epic. The epic sets the bar; this document says what the bar is made of, what the incumbents actually do, what every decentralized attempt before us learned, where the protocol already has a home for each instrument, and where the design is most likely to fail.*

**Method.** Three passes, synthesized by the orchestrating session (Fable 5.1). A Sonnet explorer grounded every protocol-side claim with `path:line` evidence against the tree. An Opus researcher wrote the outside primer (§2, §4, §5, §9) from primary sources with a verification key. The operator co-authors: this is the first area where the operator has said plainly that they are learning the domain with the synthesis rather than ahead of it, so §8 is written as questions with a recommended answer each, not as decisions.

**Verification key:** ✅ verified in source (theirs or ours) · ◐ single-source, plausible · ⚠ UNVERIFIED / from memory.

---

## 1. The problem, as the operator stated it

The protocol was approached substrate-first: EPR (the content-addressed record), eprfs (the local filesystem of records), epr-meta (directory-local governance), epr-rea (the economic fold), and viable-system modelling above them. Social reach — content earning its distribution before it spreads — is the floor. Integrity and requisite variety are handled at the intimate edge, so that aggregates can be pluralistic and honest, and wisdom can operate over the process embedded in the fabric rather than imposed on it.

The question this document exists to answer: **what does that stack mean as an answer to the power that is Search?** Not "how do we build a search box" but "what is the power, what is it made of, and what does a network built the way ours is built do with it" — in a world where search and surveillance remain powerful, will still exist, and will want to be brought inside.

A second question arrived mid-flight and turned out to be the sharper one. Switching from Google Maps to an alternative, the operator felt a tangible loss: Street View, the ability to *see* and walk the world from a corner. Some aggregates are valuable precisely because they are global and explorable. A reach-first answer that only shrinks the world is not an answer. So the problem has two halves that must be held together: **refuse the taken aggregates** (the crawl of what was not offered, the log of what was looked at) **and hold the contributed ones** (the map, the street, the shared record) at least as well as the incumbents do.

---

## 2. How the incumbents' machines work

*Merged verbatim from the sourced primer (Opus researcher, 2026-09-11); subsection numbers are the primer's own. Bracketed numbers point at the References in §9.*

### 1. Web search as three machines

**Crawler.** Google's 1998 design is a fleet, not a program: "A single URLserver serves lists of URLs to a number of crawlers (we typically ran about 3)" ✅, each holding "roughly 300 connections open at once," peaking at "over 100 Web pages per second using four crawlers" ✅ [1]. Note what the crawler asks of authors: *nothing*. That asymmetry is load-bearing for this whole document.

**Index.** The indexer parses each document into *hits* and distributes them into "barrels," creating "a partially sorted forward index" ✅; the sorter resorts the barrels by wordID to produce the inverted index [1]. For each word the lexicon points at "a list of docIDs together with their corresponding hit lists. This list is called a doclist" ✅ — the posting list. The ordering trade-off is explicit: sorting a doclist by docID "allows for quick merging of different doclists for multiple word queries"; sorting by in-document rank makes single-word queries trivial ✅ [1]. The 1998 barrels are partitioned by *wordID range* ✅; the later move to **document-sharded** indexes with scatter-gather fan-out and top-*k* merge is ⚠ (no primary source fetched here).

**Ranker.** Two ideas do most of the work. PageRank treats the link graph as a Markov chain: "The parameter d is a damping factor which can be set between 0 and 1. We usually set d to 0.85" ✅, justified by a "random surfer" who "keeps clicking on links, never hitting 'back' but eventually gets bored and starts on another random page" ✅. Anchor text is the other: Google "put[s] the anchor text into the forward index, associated with the docID that the anchor points to" ✅ — a document is described by what *others* say about it [1].

The lexical baseline is **BM25**: a term's contribution saturates (its "contribution to the document score cannot exceed a saturation point" ✅), tuned by `k1`, and is length-normalized by `b`, where "setting b = 1 will perform full document-length normalisation, while b = 0 will switch normalisation off" ✅; the survey notes "a common combination would be b = 0.5 and k1 = 2" while "many experiments suggest a somewhat lower value of k1 and a somewhat higher value of b" ✅ [2].

**Learning from behaviour.** The best primary source on modern ranking is, oddly, a court record. Judge Mehta found: "At every stage of the search process, user data is a critical input that directly improves quality" ✅ [3]. Concretely, **Navboost** "pairs queries and documents through memorizing user click data," trained on 13 months of it; **QBST** is "a memorization system" on ~13 months of user data ✅ [3]. And the scale gap is quantified: "Thirteen months of user data acquired by Google is equivalent to over 17 years of data on Bing" ✅, with 93% of unique query phrases seen only by Google ✅ [3].

**What scale buys.** The index parallelizes cleanly — shard it and fan out. The *log* does not: click and dwell signals are valuable only when aggregated globally across all users of one system. Two aggregations, one shardable and one not — decentralization is cheap for the first, brutal for the second.

### 2. What the monopoly actually is

August 5, 2024 (D.D.C., Mehta): "Google is a monopolist, and it has acted as one to maintain its monopoly. It has violated Section 2 of the Sherman Act" ✅ [3]. The mechanism is defaults bought with cash: "In 2021, Google paid out a total of $26.3 billion in revenue share under these contracts, an expense listed in its financial statements as 'traffic acquisition costs'" — "four times more than the company's other search-related costs combined, including research and development" ✅ [3]. The defaults deliver the queries: over 65% of searches on Apple devices flow through the Safari default; on Android, 80% ✅. Queries make the log; the log makes the ranker; the ranker keeps the queries.

Remedies, September 2, 2025: the court ordered **data sharing** — Google must make available to "Qualified Competitors" certain Search Index data, three sets of User-side Data, and Ads Data ✅ — plus **search and text-ads syndication** ✅ [4]. It declined the structural ask: the Android divestiture "suffers from similar legal infirmities as the Chrome divestiture," and "There can be no remedy absent a factual basis to support it" ✅ [4]. It narrowed the data remedy where signals are engineered rather than logged — "The court will not sanction forced sharing of signals that are so attenuated from raw user data" ✅ — compelling index disclosure of only six fields (DocID, DocID→URL map, first-seen, last-crawled, spam score, device-type flag) ✅ [4]. A further opinion on the judgment's details followed December 5, 2025 ✅ [5].

In the EU, the parallel move is ex-ante: on 6 September 2023 the Commission designated six gatekeepers under the DMA, with **Google Search** among Alphabet's designated core platform services ✅ [6].

The lesson for a protocol designer: the court's own theory of harm is that *the behavioural log is the moat*, and its remedy is to force-copy the log rather than break up the company.

### 3. Recommender engines

Collaborative filtering and matrix factorization (the Netflix Prize lineage ⚠) learn latent factors from a sparse user×item matrix. Production systems have since converged on **two-stage retrieval**. YouTube's is canonical: candidate generation retrieves a few hundred videos from a corpus of millions, then a separate ranking network orders them ✅; candidate generation is posed as extreme multiclass classification, equivalent at serving time to nearest-neighbour search in a learned embedding space ✅; and the system optimizes **expected watch time rather than click-through** ✅ [7]. That is the two-tower structure the industry copied: embed user-context and item in one space, index items with ANN, re-rank only the survivors. Exploration exists because the system can only learn from what it showed.

The "filter bubble" claim (Pariser, 2011 ⚠) is weaker empirically than its fame suggests. Bakshy, Messing & Adamic (2015, *Science*, 10.1M US Facebook users) concluded that "individual choices more than algorithms limit exposure to attitude-challenging content" ◐ [8] — I could not fetch the primary text (paywall + image-encoded PDF), so treat that quote as single-source. The stronger evidence is the 2023 Meta/academic collaboration: assigning consenting users to reverse-chronological feeds during the 2020 US election "substantially decreased the time they spent on the platforms," increased political and untrustworthy content seen, and decreased uncivil content on Facebook — yet "the chronological feed did not significantly alter levels of issue polarization, affective polarization, political knowledge, or other key attitudes during the 3-month study period" ✅ [9]. Companion papers on reshares and like-minded exposure reached similarly null attitudinal results ⚠. Read honestly: ranking demonstrably moves *exposure* and *time*; it did not move *measured attitudes* within three months. Neither "algorithms cause polarization" nor "algorithms are innocent" survives this cleanly.

### 4. The semantic web, and why it lost

RDF (triples), OWL (ontologies) and SPARQL (query) ⚠ were the stack; Berners-Lee's 2006 Linked Data note gave the four rules: use URIs as names; use HTTP URIs so they can be looked up; serve useful information in standards (RDF, SPARQL) when looked up; and include links to other URIs "so that they can discover more things" ✅ [10]. The failure was incentive, not technology: the author does the modelling work and the *consumer* captures the value. What survived is the version where the consumer pays the author back — **schema.org**, "a joint effort" of "Google, Bing, Yandex and Yahoo!" ✅, whose stated payoff to the webmaster is that marked-up content may "surface more clearly or more prominently in search results" ✅ [11]. Authors adopted the lossy, shallow vocabulary because the indexer rewarded it, and ignored the expressive one because nothing did. **Solid** is the second attempt, moving the locus from schemas to storage: "Users have personal online data stores called Pods," and "Solid applications can read and write documents directly from your Pod" ✅ [12]. Solid relocates custody; it does not by itself solve discovery.

**Our reading of §4 (operator question, 2026-09-11: "was there an idea as to how that would actually drive search?").** Yes: search as graph-pattern matching over triples (SPARQL: a triple with holes → an exact answer set, not a ranked page list), federated by shared URIs across datasets nobody merged, with inference from the ontology (a pediatrician answers a query for physicians). Three failures killed it — authoring burden, vocabulary fragmentation, and the one the textbooks underplay: the layer-cake's *Trust* and *Proof* layers were never built, so an open triple store is a spam machine. Google then built it privately (Knowledge Graph, 2012) and schema.org survived only where the indexer captures the annotation's value. The protocol supplies exactly the missing pieces: the burden dissolves because an EPR *is* a typed record with coupling legs (authoring the page is authoring the triples; the elohim carries the rest at publish time); the trust layer is the substrate (notarized, standing-bearing, reach-earned assertions); and the value of the annotation stays with the network as fruits rather than with the indexer. So native search over the EPR graph gets what the semantic web wanted — exact answers over a typed, linked, trusted graph — **without its machinery**: the ontology arc already decided this (`owl2-graduation-floor-ceiling-ontology-2026-07-23.md` §6: *do not adopt an ontology language*; zero RDF crates, no triple store, no reasoner, no JSON-LD `@context` on canonical views; borrowed vocabulary lives in `bridges/`). The query model is the `ContentGraphResolver` over typed EPR couplings plus a Zanzibar-shaped check (`ontology-systems-survey-reach-reconciliation-2026-07-22.md` §5), and the "trust layer" is the derived verdict `{Allowed|Blocked|Pending, evidence, explain?}` — see §4b below for the point-by-point intersection. **Warning carried forward:** vocabulary fragmentation is what killed it, and the reach-enum drift (§8 Q1) is that disease in miniature — it is the difference between a graph you can query and a pile you can only search.

### 6. Privacy-preserving retrieval primitives

**PIR.** The foundational result is a hard wall, stated in the original: "when accessing a single database, to completely guarantee the privacy of the user, the whole database should be downloaded; namely n bits should be communicated" ✅; replication buys you out of it, with a two-server scheme at O(n^(1/3)) communication ✅ [28]. Practically: information-theoretic PIR needs **non-colluding replicas**, which is a governance assumption, not a cryptographic one. Modern single-server computational PIR is far better than O(n) but still costs server work linear in the database per query ⚠.

**On-device retrieval.** The primitive that actually works at household scale today. `sqlite-vec` is "a vector search SQLite extension that runs anywhere" — "Linux/MacOS/Windows, in the browser with WASM, Raspberry Pis, etc." ✅ [29]. Embed, index and query locally: the explorer is not logged because no one is there to log them. The ceiling is corpus size, not privacy.

**Federated learning** (train on-device, share updates) and **differential privacy** for aggregate counts ⚠ — both real and deployed at platform scale, but no primary source fetched here. Household-scale assessment: DP noise budgets assume millions of contributors; with a household's worth of peers, the noise needed for meaningful ε destroys the signal you wanted.

### 7. Agent context management is the same three machines

Retrieval-augmented generation pairs "parametric memory" (a pretrained seq2seq model) with "non-parametric memory" (a dense vector index of Wikipedia) reached by a neural retriever, and reports that RAG models "generate more specific, diverse and factual language than a state-of-the-art parametric-only seq2seq baseline" ✅ (NeurIPS 2020) [30]. Structurally this is the 1998 architecture with new parts: **ingest = crawl**, **embeddings = index**, **similarity = rank**. The regression is inspectability: a posting list and a BM25 score can be read and argued with; a cosine similarity over an opaque embedding cannot, and the top-*k* cut is invisible to whoever reads the answer. Agent "memory" products add a fourth machine — a write policy deciding what is worth remembering — a ranker no one audits ⚠.


---

## 3. The four instruments of search power, and their protocol homes

Reading §2 against the tree, search power decomposes into four instruments. Each has a protocol-side home already named in canon or spec, and a measurable distance from running code. This table is the map the epic's promises stand on.

| Instrument | The power it confers | Protocol home (canon / spec) | State in the tree (2026-09-11) |
|---|---|---|---|
| **The crawl** — fetch everything reachable, ask nothing of authors | Coverage without consent; the whole link graph in one place | Reach earned at authoring (Stance III.2 ✅ `values-forward.md:203-213`); discoverability = standing, not crawler reach; crawlers enter as participants whose reads are receipts (epic §When the Crawler Knocks) | Reach vocabulary is drifted across ≥4 enums ✅ (`elohim/epr/src/reach.rs:1-25` protocol-owned 8 values; `eprfs-agent/src/memory.rs:16-19` 3 values; `doorway-service/src/services/custodian.rs:55` geographic 8 values; `brit-epr/.../reach.rs:8` attestation levels) — backlog `reach-vocabulary-frontend-strand.md`. No participant-crawler exists. |
| **The index** — inverted index / embeddings, sharded, fanned out | Sub-second recall over everything | Index is a rebuildable *projection*, never truth (governed-discovery spec §Graduation ladder ✅ `:317-325`); above the household it is a **commons pool** — contributed to, drawn from, governed by its members, run by elohim as a utility (operator, 2026-09-11; §5c); `Provider` seam with `ranking_known` ✅ `:136-142` | Only implemented search: bounded frontmatter+body prefix match in `epr-cli/src/flow/memory/recall.rs:711-802` ✅ (declared hit scores 2× a body hit; unreadable candidates skipped, counted, named). No FTS/tantivy/vector module in elohim-storage or doorway ✅ (grep clean). Local FTS5 + federated query is *design only* ✅ (`google-drive-application-design.md:167,253`). `graph_engine.rs:139` names the embedding-engine seam. |
| **The ranker** — link prior + learned-from-behavior model | Serving what people *do*; self-reinforcing with scale | Algorithm as artifact with a CID printed on every result; standing printed as a shape, never summed; honesty floor + content-class floor checked at render ✅ spec `:157-160`; `records-lifecycle-design.md:717` "feed ranks by standing + recency + signal-density, not predicted-attention" | Recipe CID exists at L0 (recall-contract v9, `composition: scope→discover→filter→group→select→read→judge` ✅). No ring-aware ranking anywhere. "Print, never sum" is a norm, not yet a test. |
| **The behavior log** — clicks, dwell, reformulations, pooled | The learning signal; the moat; the surveillance | Private thought rule; journeys per holon, no cross-holon aggregator (spec §Anti-capture invariants); `AttentionTending` on the private source chain, never gossiped ✅ `elohim-storage/src/p2p/attention_tending.rs:1-119` (classification ValuesForward/Fatigue/ScopeMismatch/Safety, TTL, `tended_at`) | Wire type + schema + `EprKind::AttentionTending` exist ✅. Receipts are private session records under `.eprfs/status/recall/` ✅. Nothing joins the actor sidecar to lens defaults yet (reader-context seam unwired). |

**Two absences worth stating as findings.** The concern-routing atlas has no row for content search, indexing or recommendation ✅ (`2026-06-21-elohim-seam-map-concern-routing.md` §8 crosswalk names analytics/ML but not search); and the manifesto never uses the word ✅ (grep). Search was, until the epic, an unrouted concern. The epic gives it a stance; this document gives it a row — §10 proposes the atlas entry.

---

## 4. Decentralized and federated search before us — why indexes recentralize

### 5. Decentralized and federated search attempts

**YaCy** — a full web crawler plus index per peer, where "search indexes can be exchanged with other YaCy installation[s] over a built-in peer-to-peer network protocol" ✅, with no central server and an explicit opt-out to a local-only index ✅ [13]. Its own framing is privacy-first ✅; I found no project statement quantifying result quality or spam resistance, and say so rather than guessing. What breaks: index freshness and adversarial spam — both cheap to fight with one global log, expensive without one.

**Freenet / Hyphanet** — anonymous content-addressed storage with no first-class keyword index ⚠ (not verified here); discovery historically ran through curated index pages. Anonymity and global ranking are close to incompatible: ranking wants to know what everyone requested.

**Secure Scuttlebutt** — the index is a *local derivation* over already-replicated feeds. JITDB is "a database on top of a log with automatic index generation and maintenance" that "lazily creates and maintains indexes based on the way the data is queried" ✅ [14]. No global index exists and none is needed: query scope *is* replication scope. That is the cleanest p2p answer available, and its ceiling is the same fact — you cannot search what you have not replicated.

**Nostr NIP-50** — search is delegated wholesale to relays. The spec defines a `search` filter as "a string describing a query in a human-readable form," says "Results SHOULD be returned in descending order by quality of search result," and is marked `draft optional relay`; clients "MAY send `search` filter queries to any relay, if they are prepared to filter out extraneous responses" ✅ [15]. Ranking semantics are deliberately undefined. Result: a protocol-shaped hole that whichever relay indexes best will fill.

**Mastodon 4.2** — the consent precedent. Users "can control if you want your posts to appear in search or on the Explore page — both are opt-in" ✅ [16]. This is the exact inversion of the web crawler: the indexed party, not the indexer, holds the switch.

**Bluesky / AT Protocol** — algorithmic choice as a market. "Feed Generators are services that provide custom algorithms to users through the AT Protocol" ✅; the operator subscribes to the firehose at `com.atproto.sync.subscribeRepos`, returns a *skeleton* of post URIs, and the hydration happens downstream ✅ [17]. So the ranking runs on the operator's machine, and the request is "authenticated by a JWT signed by the user's repo signing key" ✅ — the operator learns the requesting DID. Algorithm choice is unbundled; observation of the reader is not.

**IPFS** — CIDs and DHT provider records route to content you can already name; keyword discovery is out of scope and is supplied by external indexers such as ipfs-search ⚠ (I could not verify a explicit "no search" statement in the official FAQ, and say so).

**The recurring pattern.** All of these recentralize at the index, for two reasons: indexing is cheap centrally and expensive everywhere, and the crawler asks nothing of authors while a consent-based index asks something of everyone. Mastodon and SSB accept a smaller index as the price of consent. Nostr and Bluesky externalize the index and let a market form — honest, but a market of indexers is still a few parties who see every query.


**Our reading.** Every attempt in §4 fails at the same joint: indexing is cheap to run centrally and the crawler asks nothing of authors, so whoever runs the biggest index wins and the index becomes the platform. The only precedents that held were the ones that changed *who does the work and why*: Mastodon's opt-in searchability moved consent to the author; Bluesky's feed generators moved the ranking objective into a market the reader can see (and moved the behavior log to whoever runs the generator — the same power, relocated, not dissolved). Two production-scale precedents for *propose, never rule* were found by the 2026-09-11 horizon survey and belong here by name: **Bluesky/AT Protocol feed generators** (a DID-identified service returns an unordered *skeleton* of post URIs; the reader's own server hydrates and renders — the generator proposes, never renders) and **Stract's Optics** (a reader-swappable ranking DSL applied at query time over one shared index — recipe-as-artifact, minus the ring walk). The live counter-example is **Mwmbl** (any user's curation writes straight into the shared ranking; works only while small). Ledger and verdicts: `commons-data-pools-hot-path-and-external-tooling-2026-09-11.md` §6a. The protocol's bet is that reach-earned-at-authoring is the first design where the consent-to-be-found is not an extra step but *is* publication, so the incentive failure that killed the semantic web does not apply. That bet is stated in the epic; it is unproven at any rung above L0.

---

## 4b. Where search intersects the ontology decision already made

The ontology arc (survey 2026-07-22 → OWL 2 graduation 2026-07-23) settled the substrate this epic's discovery rides on. Search does not reopen it; it *instantiates* it. Point by point:

| Ontology-arc decision | What it is in search |
|---|---|
| **Demarcation, not decision** — the ontology names the mechanical floor, the ceiling of delegated judgment, and the region between as *not the ontology's to decide*; one first-class term for that region (`ask` / `FlagForHuman` / `Pending`, to be named once) | The **frontier line**. "Rings not walked, no endorsers between" is the `Pending` verdict rendered to a reader: not denied, not granted, *a person must decide*. The friend's version — *"I can, if you want me to — but I'd be asking on your name"* — is that term spoken aloud. |
| **Explanation as a metered API** — `explain=false` cheap path, `explain=true` traced path, cost acknowledged (survey §4.3) | The **honesty floor** and the friend's-version render rule (§6). The friend's words are the cheap path; recipe CID + lens CID + provenance chain + counts are the traced path, one request away and priced. |
| **Announcement slot in the verdict** — anonymous→commons; authenticated→lens; announced→negotiable; said-vs-did variance feeds standing (survey §5) | The three ways a journey walks: the commons face needs no identity; a household journey walks under its lens; and *"asking on your name"* is the move from authenticated to **announced**, where reach becomes negotiable and the answer given is a commitment whose fulfilment is later measured. |
| **Observer / negotiation term** — the viewer's declared intent, the genuinely novel composition REA and Zanzibar both lack (survey §4.5) | The **reader lens** (AttentionTending + capability profile + intent). The unwired reader-context seam (§8 Q5) is this term unbuilt. |
| **Declared reach = schema-8 enum, DNA-notarized floor; effective reach = derived verdict, monotone-narrowing, hard floors sovereign** | Rings are *declared* reach; what a journey may read in each ring is *effective* reach — a verdict, never a stored fact. The public=commons ruling (§8 Q1) is the survey's "static commons end" of the bathtub curve: crypto-gate at the private end, tuple-check in the middle, a static serving contract at the commons end — which is exactly the crawlable public face (§5). |
| **Consistency / staleness contract** — verdicts served from replicas need explicit freshness (zookie / new-enemy) | Red-team row 7. A search result must carry the head CID *and* the freshness of the verdict that admitted it; a lagging peer prints its lag. |
| **Fixture-based verdict harness** — "given these tuples/commitments, agent A sees exactly {…}" | The standing reader's question bank (spec station 3) is the discovery-side twin: "given these rings and this lens, this reader reaches exactly {…}". Rows 1–3, 6 of the red team are fixtures in that harness, not norms. |
| **Vocabulary-drift prevention** — gate (`check-reach-drift.mjs` over `.ts`/`.rs`) *precedes* rename; geographic-8 → `LocalityLevel`; generated projections, never a fifth vocabulary | §8 Q1 is not a new decision: it is that plan, already written (OWL 2 doc §6 "Do now"), waiting on execution. The semantic web died of this disease; the plan is the cure and it is a regex. |
| **Cyc lesson, re-read** — never a *centralized* reasoner; inference lives at the edge | Per-holon indexes, no cross-holon aggregator, providers that propose and never rule. The "intelligence at every endpoint" is the elohim (Beer), not a commons-wide ranker. |

**Nuance to revive from the arc (operator question, 2026-09-11: "does that OWL work note any nuance we need to bring back?").** Six items, two of which correct promises the epic currently makes:

1. **Unknown ≠ absent ≠ let go — three states, not two.** The arc takes the open-world assumption *at the fact plane* (a peer's non-observation is not a negative fact; `BytePresence`/`VerificationStatus` already model unknown ≠ absent) and names the missing third term: `BytePresence` cannot distinguish *released by judgment* from `Missing`/lost — "the moral difference between witness and surveillance." The honesty floor must print all three: *could not be read*, *does not exist here*, and *was here and was let go, by whom, why*. Today the floor has the first only.
2. **Retention is the temporal twin of reach; the frontier line has a temporal twin.** Reach = disclosure across the social axis, retention = across the temporal axis, one axis-generic verdict surface. A graceful forget leaves a residue — a `CompositionNode` with CID and identity but no bytes, plus an attributed edge with rationale. Search over living memory renders the residue, never silence: Tomas's elohim can say *"there was a note about this last spring; it was let go in June, by you."* The Living Memory epic and this one meet exactly here, and the epic does not yet say it.
3. **The per-viewer premise is aspiration, not load** (§4 refuted claim 3, measured): `reach_earning.rs` `evaluate()` has no viewer parameter; its sole caller is author-side compose. Every ring walk in the epic is a *serve-side, per-viewer* effective-reach verdict, and that verdict does not exist in the tree. This is the largest gap under the epic and should be the first row of §3, not a footnote. The reader lens cannot be wired until the viewer parameter exists.
4. **Annotation-inertness law** — anything that gates a decision is a first-class term, never an annotation. This is the structural form of "propose, never rule": a provider's ranking is an *annotation* on its candidates and is inert by law; order is decided by the recipe's first-class terms. And `ranking_known`, the frontier, and the lens-hidden count are verdict fields, not metadata. Pair with profile discipline in Cedar's generative form: a recipe whose order could be decided by an unanalyzable provider is **rejected at seal**, not caught at runtime.
5. **Versioning is overclaimed** (`policy@version` is a mutable YAML row; 7 of 36 rules pinned). The epic promises the anti-bubble policy is "a published artifact with an address" and prints a recipe CID on every result. That is true of the recall contract (CID-pinned, v9) and **false today of the qahal policy** the epic describes — it would be an author-asserted integer. Search must print CIDs and never `id@version`; the anti-bubble policy row needs the `contentHash` the arc already prescribes before the epic's sentence is true.
6. **The strongest live idea in the arc is untested:** `vf:AgentRelationship`-as-tuple-store (survey; OWL 2 doc §7). The *trusted* ring is precisely a set of `AgentRelationship` tuples both sides affirmed; the ring walk is a Zanzibar-shaped check over them. Reviving it makes rings a query over relationships the substrate already models in REA, rather than a new store. Also unread: `all_vf.TTL` as the one-abstract-spec exemplar in our own lineage.

Two smaller carries: the arc's **two-semantics-with-no-theorem** obligation applies to search's graduation ladder — the L0 recall executor and an L2 native provider must be shown to agree on the fixture harness, or the "byte-identical" claim is a hope; and explanation is **partial, better than expected** (`bounds-validation-result-view` carries positive per-check witnesses; `StageTrace` ships named intermediates; `explain.rs` is live) — the traced path builds on these, it does not mint. Unrelated to search but surfaced by the same note: the constitution is the stale strand on "blockchain-anchored" (§8) — someone should fix it; not this epic.

**What search adds that the arc did not have to answer:** the *Provider* seam (opaque candidates inside an inspectable recipe, `ranking_known`), the *lens* as a TTL-bound, tended, revealed record, and the *commons-as-condition* ruling that no public reach — however small — is exempt from commons governance. None of these needs an ontology language either; each is a table, a verdict field, or a render rule.

---

## 5. Global aggregates that are commons, not surveillance

### 5b. Global aggregates that are commons, not surveillance

Losing Street View when you leave Google is a real loss — but be precise about *which* machine you lost. Street View is not ranking; it is a **global aggregate**, a tended and explorable corpus. Aggregates of that kind can be commons, and several already are.

**(a) OpenStreetMap.** "OpenStreetMap is open data," operated "by the OpenStreetMap Foundation (OSMF) on behalf of the community" ✅ [18], licensed under "Open Database License, 'ODbL' 1.0" with the attribution "© OpenStreetMap contributors" and a share-alike obligation ✅ [19]. Scale, from the project's own stats: ~10 million registered users (2025-Q2), 2.25 million distinct contributors, 10 billion nodes ✅ [20]. Provenance is per-unit by construction: every edit lands in a signed-in user's changeset, and history is public.

**(b) Street-level imagery commons.** **Panoramax** is "a digital resource for sharing and using street pictures" with "a completely open-source software stack, and fully managed by a growing open community," explicitly federated — you can "become a part of the Panoramax Federation and setup your own pictures server" — and open to anyone: "Anyone can take photographs of places visible from the public space and add them to the Panoramax database" ✅ [21]. Its blur API and excluded-areas features appear in the docs ✅, but I could **not** reach a primary page stating the per-instance licences; secondary sources report Etalab 2.0 on the IGN instance and CC-BY-SA-4.0 on the OSM-France instance, with ML-based face/plate blurring ◐ [22]. **Mapillary** is crowdsourced but proprietary: "© Mapillary from Meta" ✅, and it "use[s] technology designed to blur any faces and license plates our algorithms detect within the imagery" before publication, with removal handled by contacting support ✅ [23].

**(c) Google Street View.** Google likewise operates "face and license plate blurring technology," and a person may ask it to "blur your entire house, car, or body" via the report tool ✅ [24]. The surveillance-by-crawling precedent is the 2010 Wi-Fi incident, and the FCC's own record is unambiguous: the Street View cars "also collected 'payload' data — the content of Internet communications" including "e-mail and text messages, passwords, Internet usage history," which Google traced to code "mistakenly" included in its software ✅. The Bureau declined to act under Section 705(a) and instead found Google "apparently liable for a forfeiture penalty of $25,000" for obstructing the investigation ✅ [25]. The camera was consented-to as a *photograph*; the antenna was not consented to at all — and the fine was for obstruction, not collection.

**(d) Wikidata / Wikipedia.** Wikidata is "a free, collaborative, multilingual, secondary knowledge base" under CC0, and it "records not just statements, but also their sources" ✅ [26]. Per-unit provenance is doubled: each *statement* carries references, and each *edit* carries an attributed revision.

**(e) Crawl-as-commons.** Common Crawl is "a 501(c)(3) non-profit... dedicated to providing a copy of the Internet to Internet researchers, companies and individuals at no cost," whose CCBot "check[s] first the robots.txt" and honours `nofollow`; opting out means `User-agent: CCBot / Disallow: /` ✅ [27]. That corpus became foundational LLM training data, which exposes the flaw: **robots.txt is a directive to a crawler, not a licence grant from an author**, and it was never designed to express "index me but do not train on me." The Internet Archive raises the same question for preservation ⚠ (not verified here).

**(f) The pattern.** A global aggregate stays a commons rather than surveillance when four properties hold: (1) each unit is contributed with a **declared licence/consent**; (2) **provenance is per-unit**; (3) **tenders are credited**; (4) the **explorer is not logged**. OSM and Wikidata satisfy 1–3 fully. Panoramax satisfies 1–3 by design and is the only street-imagery option that also federates custody ✅/◐. Mapillary satisfies 2–3 for contributors, 1 only weakly for *subjects*, and 4 not at all. Street View satisfies none of 1–3 (subjects get a post-hoc blur appeal, not consent) and inverts 4. Common Crawl satisfies 2 and half of 1, fails 3, and satisfies 4 only because it serves no readers. **No system here satisfies 4.** That is the open seam: each of these commons is explorable only through a reader-logging front door — and the primitive for un-logged exploration is §6.


**Our reading — the four conditions.** A global aggregate stays legitimate on this substrate when all four hold, and the epic's map section is these four in prose:

1. **Contributed, not taken** — each unit enters under the contributor's declared reach and license (the OSM changeset / ODbL shape).
2. **Provenanced per unit** — who, when, from where, walkable backward (OSM edit history and Wikidata revision history already do this; Street View does not expose it).
3. **Tended as counted care** — stewards are credited as a valueflow the commons can see (this is the part no existing commons does economically; OSM and Wikipedia run on unpaid labor and both show the governance strain of it). No external mint decides what tending is worth: because the commons is REA over fractal stewards, the neighborhood that wants its streets current raises the bounty until someone answers, and an often-earned reward diminishes and diffuses back to the commons, inviting the next person to try something new (operator, 2026-09-11 — the mechanism is deliberately unspecified beyond that).
4. **Explorer-blind** — the one looking leaves a receipt only in their own journey (no existing commons guarantees this; Wikipedia logs reads in aggregate, OSM tile servers log requests).

**The commons is itself layered (operator, 2026-09-11).** A district's medical page is kept by humans, so it does not stand on their word: it is validated by a worldwide consortium of reviewers (the Cochrane shape — a global-reach collective that aggregates the latest evidence from across the network), and each link in that chain carries the reach it has earned. This is the glossary's reach layers (community → province → nation → global) applied to the commons ring, and it matters for two red-team rows: it is the honest, popularity-free *authority prior* the cold-start front door (§6 rows 1–2) can be built from, and it is what keeps a single commons index from becoming the new Google (§6 row 9) — validation reach is earned per layer, never held by one seat. Provenance per unit (condition 2) therefore includes the validation chain, not only authorship.

**The regenerative offering (operator, 2026-09-11).** The commons ring's public face — the pages the protocol projects outward (the T4 doorway projection; SSR'd EPR pages) — is *eligible for scraping by design*. Any legacy engine may index and rank it, and the expectation is that it ranks well: an EPR page carrying holon-notarized trust earns legacy ranking on human trust alone. What the network holds is the **fruits** — the value dimension of the EPR triad (story + value + governance) that the engagement web would have converted into ad revenue, engagement and analytics. The protocol stewards that value on behalf of the network, subject to the network's own commons governance, as a natural inheritance held in trust and pooled to the mutual flourishing of the network's parties and of the third parties who interface fairly through **bridge settlements** (the `bridges/` seam is the outward translation; Stances I.3–I.4 are the canon: intelligence built from the commons is owed to the commons, the person keeps the produce of their labor). So the crawl has two answers, not one: *above* the public face, take freely; *beneath* it — nearer rings, the looking, the fruits — enter as a participant or settle as a partner. This corrects an earlier draft of the epic that said the crawler "gets nothing."

The crawler that keeps such a commons coherent is legitimate under the same four conditions: its reads witnessed, its standing earned, its index a projection. **Crawling becomes tending.** This is the protocol's answer to the operator's Street View loss: the value of the aggregate is in its wholeness and explorability, not in the taking; hold the first two, refuse the third.

---

### 5c. Indexes and crawlers are commons pools (operator correction, 2026-09-11)

*Made practical — what the tree has, what must be built, the admission contract for external tooling, the risks — in the companion `commons-data-pools-hot-path-and-external-tooling-2026-09-11.md`.*

An earlier draft carried the sparring-partner frame — the *unconsented* index as the adversary's shape — over onto the network's own utilities, and it read as threatening. Corrected: **the network's own indexes and crawlers are commons pools.** Think liquidity pools, risk pools, R&D pools — but the pooled resource is a commons datastore rather than a token. A ring-level index (circle, community, commons) is a shared store its members *contribute to* (declared-public content, tending, compute), *draw from* (search across the pool), and *govern* (the pool's recipe is a published artifact; its rules are the commons' own), **administered and executed by elohim as utilities**. Its crawlers are the pool's pumps, not strangers at the gate. Consequences:

- **What is forbidden is narrower than "no cross-holon aggregator."** Journeys and behavior are never pooled (that is the log). A pooled index of *declared-public* content is a commons pool and is exactly allowed — indeed required, or the holon island (row 1) is real.
- **The pool is the front door.** Risk-pooling analogy: a newcomer draws before contributing; the pool absorbs cold start (row 2). Combined with the layered, consortium-validated commons (§5), the newcomer's first week has an honest answer the epic had marked as an unpaid debt.
- **R&D analogy:** the semantic provider with a declared model CID (the thing that retires MemPalace) is a pool investment — built once, held in common, its cost borne by the pool, its yield accruing to the pool's participants as fruits. This is where the operator's "no external mint" ruling (§5 condition 3) lands for tooling: the pool decides what to build.
- **Ostrom's design principles for common-pool resources** are the governance checklist a pool must pass: clear boundaries (who may draw — the ring), rules fit to local conditions (per-pool recipe), collective-choice by those affected (the qahal), monitoring (witnessed reads), graduated sanctions (standing loss), conflict resolution (appeal upward), recognized right to organize (fractal stewards), nested enterprises (pools within pools — the layered commons). ⚠ unverified citation to Ostrom 1990; worth a primary-source pass.
- **Pools are the hot path (operator, 2026-09-11).** GIS, tiles, directions, street-level imagery, shared reference — the utilities a person hits a hundred times a day — are served *from* the pool at incumbent speed, because they are pooled, pre-indexed, and under the survey's "static commons end" serving contract (crypto-gate at the private end, tuple-check in the middle, static serving at the commons end). Track-wise this is T3 spoke (HTTP/WS) and T4 doorway projection serving a pooled store, explorer-blind. The latency concession in the epic (§What We Will Not Match) is now scoped to *discovery across rings*; pool utilities carry no such concession, and the household-ring latency habit (§8 Q4) should name a second bound for pool hot-path reads. Red-team row 8 (the convenience tax) splits accordingly: the intimate ring must be effortless *and* the pools must be as fast as the incumbents', or people route around both.
- **The sparring partner stays,** but its target is corrected: MemPalace is the shape of an *unpooled* index, and the tests in §7 are the pool's discipline — does *our* pool print its recipe, count its unreadables, refuse to reorder across rings, stay out of the diary.

## 6. Red team — where the reach-first inversion breaks, and the risks

Condensed from the visioning conversation and sharpened against §2–§5. Each row names the failure, the mitigation the design already has or must build, and the *test* that would show the mitigation is real. A mitigation without a test is a promise.

| # | Risk | How it fails | Mitigation | Test that would prove it |
|---|---|---|---|---|
| 1 | **The holon island** | Per-holon custody of *journeys* is right; but if indexes are also per-holon only, the network's knowledge feels smaller than it is | **Commons pools** (§5c): ring-level indexes are pooled, contributed-content stores run by elohim as utilities — the forbidden aggregator is the behavior log and the taken crawl, never a pooled index of declared-public content; plus the *frontier line* printed on every journey (rings not walked, endorsers between) | A fresh-holon reader asks the standing question bank; the frontier line prints on every result; recall from the commons ring ≥ a stated floor |
| 2 | **Cold start** | A new participant has empty rings; recommenders solve this with popularity priors we refused | The pool is the front door: a newcomer draws on the commons pool (and the layered, consortium-validated commons, §5) before they have contributed — that is what a commons pool is *for*; the commons recipe stays popularity-free and public | New-participant sample in the standing reader: first-week recall against the question bank |
| 3 | **Standing collapses into a score** | Any cross-ring ranking tempts a scalar; a summed standing is PageRank + a social-credit score | "Print, never sum" as a *test* on the view contract, not a norm | A view-contract test that fails if any candidate carries a scalar rank derived from standing |
| 4 | **The lens becomes the profile** | AttentionTending is private, but presets fed into the nervous system are the data a recommender wants; one lazy join from a dossier | Aggregation per collective and anonymous; no seat may join presets to journeys (SDO/RWA boundaries 2 and 6) | A red-team join attempt across the two stores that must fail structurally, not by policy |
| 5 | **Sybil endorsement** | Identities are free; a synthetic ring manufactures reach | Reach beyond Dunbar costs standing humans bear; provenance unwound through governance | An a2o scenario where a sybil ring earns reach and the unwinding actually fires with restitution |
| 6 | **Providers as the back door** | An opaque model (MemPalace today, a hosted embedder tomorrow) re-inserts an opaque ranker inside an inspectable recipe | `ranking_known: false` printed; the *bound*: opaque providers propose within scope+budget and never decide cross-ring order | A test where the opaque provider supplies every candidate and the honesty floor still prints; a test that it cannot reorder across rings |
| 7 | **Staleness with confidence** | Freshness derived from content heads is right, but a holon that missed a deploy serves a stale head confidently | Inherits every dataplane-convergence red; search must print head + last-verified | Search results carry the head CID; a scenario where a lagging peer's result prints its lag |
| 8 | **The convenience tax** | Friction at the amplification boundary is the product; friction inside the household ring sends people back to the box | Zero-friction intimate ring: household search must be as fast as memory | A latency + tap-count bound on the household ring, measured, in the register |
| 9 | **Commons capture** (new, from §5) | The commons ring becomes the new Google: whoever tends the biggest commons index rules; or the commons ossifies into an admin oligarchy (Wikipedia's known failure shape) | Commons recipes are public artifacts; tending is a counted valueflow whose reward diminishes and diffuses with repetition, so no seat accumulates; the commons crawler is a participant with standing that can be unwound | Commons recipe CID printed on commons results; a governance scenario that unwinds a commons steward's standing |
| 10 | **Crawler-as-tender abuse** (new) | A participant crawler with earned standing reads more than it tends; witnessed reads are a receipt, not a limit | Reads are budgeted by standing in each ring, like any participant; over-budget reads shed standing | A crawler that exceeds its ring budget loses standing in the receipt ledger |
| 11 | **Hyperscaler-parity overclaim** | We imply global recall / sub-second cross-corpus / cold-start relevance and cannot deliver | Say plainly what we will not match (epic §What We Will Not Match) | The epic's disclosure survives blind-reader review without a "promise exceeds causal story" finding |
| 12 | **Dependence from inside** (from the 2026-09-11 co-authoring pass) | A well-resourced participant provider earns standing legitimately, becomes the only provider that answers well in most rings, and the commons comes to depend on it; "propose, never rule" holds formally while `select` becomes a rubber stamp | Plurality as a *measured* bound, not a norm: no ring where one provider supplies more than a declared share of accepted candidates; the share printed on the honesty line like absence is. The deeper defense is the epic's own frame — the wild thing is made safe by a *presence* (the reader's elohim, the community's assembly) trusted to act in the moment, not by a cage; dependence is what happens when the presence stops watching | Per-journey provider share printed; a standing-reader sample that fails when one provider's accepted share crosses the bound |
| 13 | **Provenance stripping on the open face** | A legacy site copies a public commons page without its trust chain and outranks the original on the old web; the fruits leak through the copy | Content addressing makes the chain the moat: a copy without the notarized chain is a weaker page, and the bridge is the only fair path to the value; the public projection carries the chain inline so the copy is visibly the poorer artifact | A scrape of the public face reproduces the chain verbatim or is detectably chainless; bridge-settled value is the only value a third party can show |

**The friend's version first (operator, 2026-09-11).** The honesty floor is a fact the view must carry, not a sentence the person must read. At the minimal lens it renders in a friend's words (*"I didn't go asking strangers; I can, but I'd be asking on your name"*), and the precise form (rings not walked, endorsers between) is one request away. A frightened parent must never have to learn a vocabulary to be told the truth. This is a render rule on the lens, and it belongs in the view contract beside the floor itself.

**Convenience, honestly.** Within a ring, faster than web search (small scope, local index). Across rings, slower and thinner — and it *should* be, but the reader has to feel that as intention rather than breakage. Rows 1, 2 and 8 are the whole UX problem, and none of them has a test today.

---

## 7. MemPalace as the standing sparring partner

The developers' memory tool is the adversary's shape at toy scale: an index nobody asked for, mined from surfaces it did not author, ranking by a method it cannot explain, freshness asserted by a stamp, a knowledge graph joining things whose authors never linked them. Every one of those is what a Google-class entity does to a network from outside. Keeping it beside the recipe as a declared `Provider` with `ranking_known: false` (spec `:334` ✅) makes each defense testable against a real opponent. The tests that must outlive its retirement, pointed at whatever opaque provider comes next:

1. The page prints `ranking_known: false` beside every candidate it supplied.
2. The honesty floor (counted, named, unreadable candidates; lens-hidden count) still prints when the opaque provider supplied *all* candidates.
3. It can never decide order across rings — the recipe's `select` stage reorders its proposals and the reorder is visible.
4. It can read nothing from the diary (private thought) or the journey store (per-holon receipts).
5. Its freshness claim is *replaced* by the content-head-derived stamp, never displayed as authority.

Where each MemPalace concern already has a protocol home (drawers → CID-derived projections; wings/rooms → epr-meta cascade placement; tunnels → authored links notarized / computed neighbours never stored; wake-up context → first screen at the minimal lens; diary → private thought), see the governed-discovery spec §MemPalace ✅ `:330-352`. It retires by a recipe version change when a native provider with a declared model CID answers the same seam.

---

## 8. Open questions — for co-authoring ❓

Each carries a recommended answer. The operator owns the decision; the recommendation is there so the question is not blank.

1. **One reach vocabulary — a return point, and the plan already exists** (operator, 2026-09-11: the epic stays at the level of vision; the practical consolidation lives here — and it was decided in `owl2-graduation-floor-ceiling-ontology-2026-07-23.md` §6: gate precedes rename; see §4b). The ring model is unimplementable at L2 while four enums disagree. *When we return:* canonize `elohim/epr/src/reach.rs` (protocol-owned, 8 values, matches the spec's rings) and make the geographic doorway family a *mapping* onto it, not a sibling. Two vision-level rulings to carry into that consolidation (operator, 2026-09-11): (a) *public* is a condition, not a ring or an audience size: anything that reaches a surface beyond the people one could name is public, and anything public is stewarded, so it can be *assumed* commons — whether it reached 1 person or 400,000 three degrees in; "nothing that reaches a public surface is ungoverned." The enum may keep `Public` and `Commons` as distinct values, but no reader-facing surface should make a person choose between them, and no implementation may treat a small public reach as exempt from commons governance; (b) *reaching* the public is easy and should stay easy (a single actor from an edge agent can address the whole network); what is earned is *amplification* beyond the author's own standing, and what keeps easy public speech safe is wisdom applied at the edge — stakes, who you are, intent — i.e. the second-seat judgment / lens, not a gate. Where to return: §3 row 1 of this document, `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md`, and `elohim/elohim-storage/CLAUDE.md:195`. This gates every mitigation in §6 at the peer rung, and nothing at the local rung.
2. **Does canon outrank lexical evidence?** In the sprint, the authoritative skill ranked second to a document that merely shared its words. *Recommend:* yes, by review — an `authority` weight declared in the recipe (a CID'd artifact), never an implicit boost; print it.
3. **Who tends the commons ring, and how is the tending paid?** *Settled in principle (operator, 2026-09-11):* tending is an REA valueflow (care counted, Stance III.1); no steward rotation is specified and no external mint decides value — the economy of tending raises the bounty until it is rewarding enough, and rewards diminish and diffuse to the commons with repetition. The commons, being REA over fractal stewards, settles for itself what its neighborhood's flourishing is worth. *Still open:* the first commons to hold — recommended: a street-level imagery commons federated from existing open instances, so the four conditions (§5) meet a real corpus.
4. **What are the latency bounds?** Two, not one. *Recommend:* declare a habit with a `retire-when:` carrying both — "household search returns in under one perceptual beat and one tap" and "pool hot-path reads (tiles, directions, reference) match the incumbent's p50 on the household mesh" — and measure both on the local mesh before any cross-ring work. Cross-ring discovery carries no latency bound by design; it carries the frontier line instead.
5. **Does the reader-context seam join the actor sidecar to lens defaults now, or after station 3?** *Recommend:* after station 3 (the standing reader) is green, so the join is measured against a rolling window rather than designed blind.
6. **Should the participant-crawler be a bridge crate or an SDK manifest?** *(Sharpened by the regenerative-offering decision: the bridge is also where value settles with fair third parties, so the outward crawler seam and the settlement seam are the same crate.)* By the seam-map disambiguator (a crate translates an external protocol; a manifest composes inward), an outside search engine entering as a participant is a **bridge** (`bridges/`), and the commons-tending crawler is a **manifest** (a declared recipe run by a peer). *Recommend:* build the tending crawler first; it is ours to test.

**A reading path for the operator**, in the order the concepts stack: §2 of this document (three machines) → Brin & Page 1998 (§9) → the YouTube 2016 recommender paper → the 2023 Meta/Science papers on algorithmic exposure → Berners-Lee's Linked Data note and *why it lost* → Mastodon 4.2 opt-in search (consent moved to the author) → Bluesky feed generators (objective moved to a market) → OSM's ODbL and Panoramax (the contributed aggregate) → then the governed-discovery spec, which will read as an answer rather than a design.

---

## 9. References

### 8. Glossary

*(definitional, not claims)*

- **Inverted index** — map from term → list of documents containing it.
- **Posting list / doclist** — the per-term document list, plus positions and weights.
- **BM25** — probabilistic lexical scorer with TF saturation (`k1`) and length normalization (`b`).
- **PageRank** — stationary distribution of a damped random walk on the link graph.
- **HITS** — Kleinberg's mutually-reinforcing hub and authority scores, computed per query.
- **Learning-to-rank** — training a ranking function on labels, often derived from behaviour.
- **Click model** — a model of *why* a result was clicked (position bias vs. relevance).
- **Two-tower** — separate user and item encoders into a shared space; retrieve by nearest neighbour.
- **ANN / HNSW** — approximate nearest-neighbour search; HNSW is a navigable small-world graph index.
- **Embedding** — a learned dense vector whose geometry encodes similarity.
- **Re-ranker** — an expensive model scoring only the shortlist retrieval produced.
- **Exploration/exploitation** — spending impressions on uncertain items to keep learning.
- **Cold start** — no behavioural history for a new user or item; fall back to content features.
- **Collaborative filtering** — recommend from co-occurrence in behaviour, not item content.
- **Filter bubble** — the claim that personalization narrows exposure (evidence in §3).
- **Feed generator** — an AT Protocol service returning a ranked skeleton of post URIs.
- **PIR** — retrieve record *i* without the server learning *i*.
- **schema.org** — search-engine-sponsored structured-data vocabulary embedded in pages.
- **Sharding** — partitioning the index by document or by term across machines.
- **Query fan-out** — broadcasting one query to all shards and merging their top-*k*.

### 9. References

1. Brin, S. & Page, L. (1998). *The Anatomy of a Large-Scale Hypertextual Web Search Engine*. — **fetched** (PDF, text-extracted) https://snap.stanford.edu/class/cs224w-readings/Brin98Anatomy.pdf
2. Robertson, S. & Zaragoza, H. *The Probabilistic Relevance Framework: BM25 and Beyond*. — **fetched** https://www.staff.city.ac.uk/~sbrp622/papers/foundations_bm25_review.pdf
3. *United States v. Google LLC*, No. 1:20-cv-03010-APM, Memorandum Opinion (D.D.C. Aug. 5, 2024), ECF 1033. — **fetched** https://www.texasattorneygeneral.gov/sites/default/files/images/press/Google%20Search%20Engine%20Monopoly%20Ruling.pdf
4. *United States v. Google LLC*, Remedies Memorandum Opinion (D.D.C. Sept. 2, 2025), ECF 1436. — **fetched** https://deadline.com/wp-content/uploads/2025/09/gov.uscourts.dcd_.223205.1436.0_4.pdf
5. *United States v. Google LLC*, Memorandum Opinion (D.D.C. Dec. 5, 2025), ECF 1461. — **fetched** https://justice.gov/atr/media/1421681/dl?inline=
6. European Commission, DMA gatekeepers page (first designations 6 Sept. 2023). — **fetched** https://digital-markets-act.ec.europa.eu/gatekeepers_en
7. Covington, P., Adams, J. & Sargin, E. (2016). *Deep Neural Networks for YouTube Recommendations*, RecSys '16. — **fetched** https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/45530.pdf
8. Bakshy, E., Messing, S. & Adamic, L. (2015). *Exposure to ideologically diverse news and opinion on Facebook*, Science 348(6239). — **not fetched (paywalled)**; quote via secondary ◐ https://doi.org/10.1126/science.aaa1160
9. Guess, A. et al. (2023). *How do social media feed algorithms affect attitudes and behavior in an election campaign?*, Science 381(6656). — abstract **fetched** via Semantic Scholar API https://doi.org/10.1126/science.abp9364
10. Berners-Lee, T. (2006, rev.). *Linked Data* design note. — **fetched** https://www.w3.org/DesignIssues/LinkedData.html
11. schema.org FAQ. — **fetched** https://schema.org/docs/faq.html
12. Solid Project, About. — **fetched** https://solidproject.org/about
13. YaCy, project README/site. — **fetched** https://github.com/yacy/yacy_search_server
14. JITDB (Secure Scuttlebutt). — **fetched** https://github.com/ssbc/jitdb
15. Nostr NIP-50, *Search Capability*. — **fetched** https://github.com/nostr-protocol/nips/blob/master/50.md
16. Mastodon 4.2 release announcement. — **fetched** https://blog.joinmastodon.org/2023/09/mastodon-4.2/
17. Bluesky `feed-generator` starter kit. — **fetched** https://github.com/bluesky-social/feed-generator
18. OpenStreetMap, About. — **fetched** https://www.openstreetmap.org/about
19. OpenStreetMap Foundation, Licence. — **fetched** https://osmfoundation.org/wiki/Licence
20. OpenStreetMap wiki, Stats. — **fetched** https://wiki.openstreetmap.org/wiki/Stats
21. Panoramax documentation. — **fetched** https://docs.panoramax.fr/
22. Panoramax instance licences / blurring — **secondary only** ◐ (IGN = Etalab 2.0; OSM-FR = CC-BY-SA-4.0) https://explore.panoramax.fr/fr/instances
23. Mapillary Privacy Policy. — **fetched** https://www.mapillary.com/privacy
24. Google Street View privacy policy. — **fetched** https://www.google.com/streetview/policy/
25. FCC Enforcement Bureau, *Notice of Apparent Liability*, DA-12-592 (Google Street View Wi-Fi). — **fetched** (PDF, text-extracted) https://docs.fcc.gov/public/attachments/DA-12-592A1.pdf
26. Wikidata: Introduction. — **fetched** https://www.wikidata.org/wiki/Wikidata:Introduction
27. Common Crawl FAQ. — **fetched** https://commoncrawl.org/faq
28. Chor, B., Goldreich, O., Kushilevitz, E. & Sudan, M. *Private Information Retrieval*, JACM 45(6) 1998 (FOCS 1995). — **fetched** (PDF, text-extracted) https://www.cs.umd.edu/~gasarch/TOPICS/pir/first.pdf
29. `sqlite-vec`. — **fetched** https://github.com/asg017/sqlite-vec
30. Lewis, P. et al. (2020). *Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks*, NeurIPS 2020. — abstract **fetched** https://arxiv.org/abs/2005.11401

**Sources I could not verify and deliberately left marked:** Pariser (2011) filter-bubble framing ⚠; Netflix Prize matrix-factorization lineage ⚠; Nyhan et al. (2023, Nature) companion results ⚠; Freenet/Hyphanet search behaviour ⚠; an official IPFS statement that keyword search is out of scope ⚠; Internet Archive scale/consent posture ⚠; federated learning and differential privacy primary sources ⚠; document-sharded modern index architecture ⚠; agent-memory write-policy products ⚠.


---

## 10. Outputs (mint pass — to fold at close, operator decision)

Per this directory's `mint-pass-at-close` rule, surviving takes fold as rows into cluster files citing this survey's slug. Proposed, not yet folded:

- **`agentic-context-tooling-consolidation-queue`** — extend item 23 (progressive-discovery substrate): this document is the research pass it asked for; add the §7 five standing tests as the retirement criteria for the MemPalace provider.
- **`commons-holonic-stewardship-backlog`** — new row: commons aggregate tending as a counted valueflow with bounty-rise and reward-diffusion, no external mint (§5 conditions 3–4; §8 Q3); first corpus = street-level imagery commons.
- **`arch-scale-risk-backlog`** — new risk row: commons capture (§6 row 9), trigger = any commons index whose recipe is not a public artifact.
- **Concern-routing atlas** — add a row for content search / indexing / recommendation: *seam* = SDK (recipe manifest) for native discovery, *bridge* for outside engines; *track* = T4 doorway projection for hosted readers, T1/T2 for per-holon indexes; parity line = "explanation, provenance, honest absence, per-ring latency; not global recall".
- **Habit candidate** (`genesis/.epr-meta` or the discovery spec's directory): `household-search-effortless` — red until measured on the local mesh; `retire-when:` the bound is held across a rolling window at L2.
