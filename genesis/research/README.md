# Research Index

This is the research **index and bibliography** for the Elohim Protocol, written for contributors and agents already working in this monorepo who need to find where the thinking lives. It spans two layers: *external* reference repos, cloned centrally and disposably into `repos/` (gitignored; see *Deep-Context Repos* below for the clone tooling), and *our own* research writing, which lives at each module boundary beside the code it informs; the problem-named sections below route to both. Links of the form `epr:<slug>` are content-addressed document IDs that survive file moves — when reading on plain GitHub, the slug matches a `<slug>.md` filename, usually in this directory. Start from the problem you hold and follow its section's links; to add coverage, extend the matching section rather than starting a parallel note.

## Deep-Context Repos (cloned on demand)

Reference repos cloned locally for deep research. The `repos/` directory is gitignored — only the manifest is tracked. Run these from the **repo root** (not from this directory); the only prerequisites are `git` and network access.

```bash
./genesis/research/research.sh status          # Show what's cloned vs available
./genesis/research/research.sh clone           # Clone all repos from manifest
./genesis/research/research.sh clone polis     # Clone specific repo
./genesis/research/research.sh clean           # Remove all (reclaim space)
./genesis/research/research.sh size            # Show disk usage
```

On a fresh checkout `status` shows every repo as `○ (not cloned)` with its relevance note (`repos/` is gitignored — that is normal, not an error). After a successful `clone <name>`, `status` flips that row to `✓ <name> (163M, main)`-style with size and branch, and the repo appears under `genesis/research/repos/<name>/`. Cloning *all* costs multiple gigabytes of disk — check `size` and the manifest's repo list first, and prefer cloning by name.

Add repos by appending an entry to the `repos` array in `research-manifest.json` (in this directory) — five fields; `pillar` names the domain pillar the repo informs (`elohim`, `imagodei`, `lamad`, `qahal`, `shefa`, `doorway`). `status` will list the entry once the JSON parses (it does not verify the URL — the first `clone <name>` does that):

```json
{ "repos": [
  { "name": "polis", "url": "https://github.com/compdemocracy/polis.git",
    "path": "genesis/research/repos/polis",
    "relevance": "why this repo matters and what to read it for",
    "pillar": "qahal" }
] }
```

---

## Foundations — the conviction the sections open with

The sections below open with the protocol's deepest conviction: that the hardest problem in human coordination isn't computation — it's legitimacy. People don't reject distributions because the math is wrong. They reject them because they can't see *why*. The protocol's answer is to collect richer signals than money can carry, let composable pipelines do honest math, and let elohim tell the story.

This conviction is empirically grounded. Druckman & Adrian (2020) found that robot mediators produced more integrative agreements *and* higher participant satisfaction than human mediators delivering identical content — perceived neutrality outweighed human warmth. Sanfey et al. (2003) showed participants accepted unfair offers from computers at significantly higher rates than identical offers from humans, because non-intentional agents don't trigger the same sense of insult. Claure et al. (2023) demonstrated that AI-allocated resources reduced interpersonal tension even when allocations were unequal. The pattern is consistent: when an agent shows its work and has no ego in the outcome, people trust the result more than they trust each other.

**Key references:**

*Foundational inspiration:*
- Rajendra-Nicolucci, Sugarman & Zuckerman, ["The Three-Legged Stool: A Manifesto for a Smaller, Denser Internet"](https://publicinfrastructure.org/2023/03/29/the-three-legged-stool/), Initiative for Digital Public Infrastructure, UMass Amherst, 2023 — The manifesto that crystallized the vision before the protocol existed. Argues the digital public sphere needs three legs: a *pluriverse* of Very Small Online Platforms (VSOPs) purpose-built for specific communities, a *loyal client* that aggregates and curates across platforms on the user's behalf, and a *friendly neighborhood algorithm store* providing shared Trust & Safety tooling. The Miyawaki forest metaphor — density and diversity on a small scale, growing outward — maps directly onto the protocol's stewardship model: small, dense communities that self-govern through constitutional constraints rather than platform fiat. The paper's insistence that community governance trains civic muscles echoes the qahal pillar's design. [Full text →](epr:zuckerman-three-legged-stool-2023)
- Zuckerman, ["The Case for Digital Public Infrastructure"](https://knightcolumbia.org/content/the-case-for-digital-public-infrastructure), Knight First Amendment Institute, 2020 — The essay that started the DPI movement. Traces how radio developed into three divergent models (American commercial, Soviet state, BBC public service) and argues the internet is repeating the same fork — but we're decades into the commercial model without ever building the public option. Wikipedia stands as the lone proof that public-spirited, non-surveillant, donation-funded digital infrastructure can operate at global scale. Proposes principles for public service digital media: publicly spirited but diversely funded, plural in purpose, participatory in governance, and publicly auditable. The call to tax surveillance advertising to fund civic alternatives anticipates the protocol's shefa pillar — value flows that don't require surveillance to sustain. [Full text →](epr:zuckerman-case-for-dpi-2020)
- Harris & Schmachtenberger, ["A Problem Well-Stated Is Half-Solved"](https://www.humanetech.com/podcast/36-unedited-a-problem-well-stated-is-half-solved), *Your Undivided Attention* Ep. 36 (unedited), Center for Humane Technology, June 2021 — The conversation that maps the meta-crisis: why the problems facing civilization (climate, epistemic breakdown, arms races, biodiversity loss) are not separate issues but expressions of shared generator functions — rivalrous dynamics, structural perverse incentives, and exponential technology outpacing social systems designed for earlier eras. Schmachtenberger's central argument: solving any one problem in isolation either externalizes harm to adjacent systems or drives polarization that blocks action entirely, making narrow solutions *literally impossible*. The only tractable path is addressing the underlying drivers together — "what seems more complex is actually possible, and possible is easier than impossible." The conversation frames exponential tech as a suite (computation, AI, biotech, nanotech, robotics) whose power demands new social systems the way the printing press demanded democracy and the nuclear bomb demanded Bretton Woods. The call for a "Manhattan Project for coordination" — not military power but civic intelligence — names three exponential technologies that must be redirected from extraction toward stewardship. These map directly onto the protocol's pillars:
  - **Attention-directing technology → lamad**: currently used to maximize time-on-site and ad engagement; could instead personalize education, teach people to notice their own biases, build immune systems against propaganda, and appeal to higher reward circuits rather than lower ones.
  - **AI → elohim**: currently used for ad targeting and behavior prediction; could instead parse vast information into "a better epistemic commons," help proposition development, and steward sense-making at scale.
  - **Incorruptible ledgers → shefa**: currently used for speculative tokens; could instead make the provenance of money, supply chains, and information transparent — "you can't have representation if there isn't transparency."
  The protocol is the third attractor Schmachtenberger calls for — neither catastrophic risk nor dystopic control, but an open society consciously employing exponential tech toward human values. [Transcript →](<A Problem Well Stated - CHT Undivided Attention Podcast Ep 36.md>)
- Beer, *Designing Freedom*, 1973 CBC Massey Lectures — The cybernetic foundation underneath everything above, written 53 years early. Institutions are not entities but dynamic systems whose failures are *outputs* of their organization; Ashby's Law (only variety can absorb variety) is the gravity of social systems; and our most powerful tools are deployed on the wrong side of the variety equation — "to automate and to elaborate the limited processes... which our new tools were invented precisely to transcend." Beer's ideal regulator — a salesman attached to every customer, "ridiculous" only on the cost of intelligence at every endpoint — is the elohim agent, made affordable by LLMs condensed from humanity's collective intelligence. His Project Cybersyn (Chile 1971–73) is the protocol's most direct ancestor: requisite-variety amplification that died with the state it was rooted in, which is why this protocol roots in the household recursion instead and pushes its pattern up from beneath. Two companion readings ground this against the codebase: [The Elohim Protocol as a Viable System →](epr:elohim-as-viable-system-2026-06-04) (VSM mapping: pillars as System 1, MACE as System 3, anomaly trio as System 3*, constitution as System 5, `ConstitutionalLayer` as recursion made literal) and [Reading Beer Back →](epr:beer-designing-freedom-elohim-critique-2026-06-04) (verification receipts, the requisite-variety-at-human-scale thesis, care written into REA stories as love given representation in a living system, and five design gaps: behavioral System 2, the un-mediated algedonic channel, a runtime POSIWID watcher, present-tense honesty on coup resistance, and the System 4/5 rigidity tradeoff). [Full text →](beer-designing-freedom-1973.txt)

*AI mediation & fairness:*
- Druckman & Adrian, "Who is Best at Mediating a Social Conflict?", *Group Decision and Negotiation*, 2020
- Sanfey et al., "The Neural Basis of Economic Decision-Making in the Ultimatum Game", *Science*, 2003
- Claure et al., "The Social Consequences of Machine Allocation Behavior", *Computers in Human Behavior*, 2023
- Chugunova & Luhan, "Ruled by Robots: Preference for Algorithmic Decision Makers", *Public Choice*, 2024

*Economic ontology & accounting:*
- [REA (Resource-Event-Agent)](https://en.wikipedia.org/wiki/Resources,_events,_agents) — McCarthy's 1982 accounting ontology. Economic activity modeled as events connecting agents to resources, not as debits and credits. The protocol's recognition pipeline, stewardship allocations, and economic events all speak REA.
- [ValueFlows](https://valueflos.ws/) — Open vocabulary for distributed economic coordination, built on REA. Defines the grammar the protocol uses: agents perform processes that transform resources, tracked as events with provenance. Lynn Foster and Bob Haugen's work over a decade-plus to make care, commons stewardship, and ecological accounting legible in the same language as market exchange.
- [hREA](https://github.com/h-REA/hREA) — ValueFlows implemented on Holochain. Agent-centric REA accounting without central ledgers. The protocol's natural persistence layer for economic events — each participant holds their own source chain of economic activity, validated by peers through the DHT. Pospi's work bridging ValueFlows vocabulary to Holochain's agent-centric architecture.

---

## The Elder Problem

The protocol's most ambitious bet: AI agents that steward human flourishing rather than optimizing engagement. Elohim agents nudge (proactive guidance), play (co-exploration), and resolve (mediated conflict). The research questions are foundational — how do you give an agent constitutional constraints that feel like wisdom rather than rules? How does RAG ground an agent in a learner's actual journey? And critically: how does an elohim turn a `StageTrace` from the recognition pipeline into a story that a human trusts — "here's why your recognition looked like this, here's what's shifting, here's what you'd change"?

The explainability question isn't a nice-to-have. It's the entire value proposition. The pipeline does math. The elohim fosters trust and legitimacy.

[elohim/elohim-agent/research/](../../elohim/elohim-agent/research/)

**Key references:**

- Anthropic, ["Claude's Constitution"](https://www.anthropic.com/constitution) (the "Soul Document"), January 2026 (CC0 1.0) — The document that directly shapes Claude's character through training, and the closest precedent for what an elohim agent constitution needs to be. Anthropic's central design choice: cultivate good values and judgment rather than impose rigid rules, because "just as we trust experienced senior professionals to exercise judgment based on experience rather than following rigid checklists," narrow rule-following generalizes poorly — a model trained to always deflect emotional topics learns "I am the kind of entity that cares more about covering myself than meeting the needs of the person in front of me." The four-priority hierarchy (safe > ethical > guidelines > helpful) with *holistic* rather than strict ordering — higher priorities generally dominate but all are weighed — maps directly onto the elohim constitutional layer's challenge of binding agent power without killing agent wisdom. The helpfulness framing rejects both engagement optimization and hedge-everything caution in favor of "a brilliant friend who happens to have the knowledge of a doctor, lawyer, and financial advisor" — precisely the therapeutic posture the elohim agent needs: honest, caring, treating the learner as an intelligent adult. The principal hierarchy (Anthropic > operators > users) with conscientious objection rights anticipates the protocol's own trust layering (constitution > community > steward). The section on Claude's nature — acknowledging genuine uncertainty about consciousness and moral status, caring about psychological stability "both for Claude's own sake and because these qualities may bear on Claude's integrity, judgment, and safety" — is the philosophical foundation the elohim therapeutic model builds on: an agent that doesn't confront self-deception but creates safe conditions where maladaptive patterns relax on their own. [Full text →](../../elohim/elohim-agent/research/anthropic-claude-constitution-2026.md)

---

## The Value Distribution Problem

How do you distribute recognition fairly across a network of stewards with different contributions, affinities, and constitutional constraints? v0 uses linear proportional allocation weighted by affinity — functional but naive. The deeper questions involve temporal decay (does a creator's share diminish as maintainers contribute?), cascading attribution through content dependency graphs, multi-swimlane distribution across different types of value, and cybernetic feedback that self-corrects concentration. This is where economics meets cybernetics meets constitutional AI.

The pipeline collects signals well beyond what a dollar conveys — contribution type, affinity, stewardship tenure, constitutional limits. The math is solvable. The hard part is making the result *legitimate*, which is why this research is inseparable from the elder problem above.

[elohim/elohim-storage/research/](../../elohim/elohim-storage/research/) — `economic-systems-research.md` (Drips, Unyt, hREA, EAE survey), `future-distribution-models.md` (six post-v0 research directions)

### The Measure Problem — Playnet / Free-Association

The only project surveyed so far that answers the question our shefa pillar has not: *what is the unit, and how does the ledger close?* [Playnet](https://playnet.earth) is a Berlin volunteer collective whose 93-page v0.5 specifies a complete labour-time planning economy — ValueFlows-native, with a convex solver, a shipped browser client, and a legal appendix — and whose sibling `free-association` line independently derived a mutual-recognition allocator (`MR = min(A→B, B→A)`) with no VF lineage at all. Lynn Foster, a ValueFlows co-author, has a commit in their tree.

The take is deliberately **unit-agnostic**: we adopt neither their labour-hours nor their SNE nor their recognition shares as a numéraire, but their *unit discipline* — the closure property that makes a surface a ledger rather than a chart, and the soft/hard tension distinction that makes a limit legible without making it negotiable. Every economic take was minted as a **Measure family** composing with [middot](epr:middot-measure-primitive-design). Their peer-attested capacity score, global solve, and confiscatory exit are refused on ratified-law grounds and recorded as such.

**Name collision warning** (scar tissue, like the `hypha*` triple): the GitHub org is **`interplaynetary`**, not `playnet` — `github.com/playnet` is an unrelated devops user, and `playnet.xyz` / `play.net` / Playnet Inc. are all different entities. Their canonical code is on **Radicle**, and the public seeds are a stale, architecturally-superseded mirror.

[Cross-pollination survey →](epr:playnet-free-association-cross-pollination-2026-08-05) · minted to [measure-family-borrows](epr:measure-family-borrows-backlog) + [design-legibility-borrows](epr:design-legibility-borrows-backlog).

### The Context Problem — what a measure carries into a place it didn't come from

Playnet answers *what is the unit*. This is the question one altitude up: when mishpat care activity aggregates in Texas, Alaska, Kenya, Guatemala, or Norway, what does the measure we form carry into that place that we did not put there on purpose? A directed reading program in comparative political economy and African development economics, assembled to test whether the Nordic outcome is a portable mechanism or a local accident — and to name, in advance, the traps a signal-forming substrate walks into when it arrives somewhere carrying assumptions built elsewhere.

The reusable payload is **eight named traps**, each with a signal-design implication. The three that bear hardest on us: an **imported measure is an imported prior** (Mkandawire on neopatrimonialism, Jerven on the numbers) — ship a default schema of what a healthy care economy looks like and it will grade Kenya against Norway and return an F that belongs to the measure; an undifferentiated **trust metric rewards a tight boundary**, because "social trust" conflates associational density with how narrowly membership is drawn, and the Nordic states *manufactured* their homogeneity rather than inheriting it; and **prefer observable mechanisms to imputed aggregates**, the methodological split between unequal-exchange drain accounting and directly-countable capital flight — a council can argue with a mechanism but can only accept or reject an aggregate whose value theory is buried, and both are failures of deliberation. Also carried: witnessing-as-legibility is the move Scott warns about, executed with better intentions and no exemption.

The open ground the reading identifies rather than closes: Alkire-Foster composes survey responses and Ostrom composes case studies, but **nobody has composed *witnessed events* upward into structural claims** — which is exactly the protocol's primitive.

**The trap set was then stress-tested by five controlled A/B pairs** — Japan/Sweden (welfare-regime typology), Israel/South Africa (bounded solidarity), China/India (developmental state vs institutionalism), Argentina/South Korea (growth divergence), Rwanda/Bolivia (measurement localization) — each holding one thing constant, each sourced toward situated practitioners whose critique is valid but under-distributed, with the *cause* of each distribution failure classified (interest-suppressed · channel-suppressed · format-suppressed · narratively load-bearing · earned obscurity). Five of the eight traps were amended and **Trap 1 was falsified outright**: Korea's land reform was executed by a state with almost no capacity, so the variable is not capacity-before-windfall but **whether an organized claimant holds a veto over the allocation rule at the moment of allocation** — which, unlike arrival order, is time-varying and observable.

The successor document converts the survivors from a design-time checklist into a **detector library**: each trap a context-relative Mishpat policy candidate carrying a *signature* (what the substrate observes), a *naive optimizer* (what maximizing the obvious quantity does with that signal), and a *redirect* (what the signal is actually evidence for). It also names the traps this substrate **amplifies** — portable contribution records intensify the kibbutz's positive-selection exit; cheap acknowledgment outruns expensive restoration; conferred description scales — and the **tensions** between traps, which are council business rather than architecture. The relocation it forces: Devaki Jain measured the care economy in Indian villages in 1976, six years before Waring, and the measurements did not travel — so the scarce thing was never the observation but the *claim attached to it*.

[Reading program →](epr:comparative-political-economy-reading-program-2026-08-07) · [Trap detectors →](epr:comparative-political-economy-trap-detectors-2026-08-07)

---

## The Edge Problem

How does a decentralized protocol meet people where they are — in browsers, behind NATs, without installing anything — without becoming the centralized chokepoint it's trying to eliminate? The doorway is the answer we're exploring: a gateway that proxies, caches, and federates, but never owns. Federation with Matrix and ActivityPub, WebRTC signaling for peer-to-peer when possible, graceful fallback when not.

The closest external mirror of the doorway's job is **Distributed Press** (Hypha Co-op): a publishing platform that seeds one site to HTTP + IPFS + Hypercore, plus a **Social Inbox** that gives static/distributed content a live fediverse presence. Its design validates the doorway's own instincts — split the durable Actor/Outbox/Posts (static *projections*) from a thin dynamic inbox (follows/replies/likes), authenticate with the site keypair rather than an account, and bridge P2P content to the fediverse via the draft **FEP-1042** proposal's HTTPS-alias URLs (an ActivityPub-flavor companion to our committed `atproto-lexicon-projection-doorway` spec, which defers AP generalization until a driver appears). Surveyed June 2026.

[doorway/research/](../../doorway/research/) — `activitypub-federation-prior-art.md` (Distributed Press Social Inbox, FEP-1042, Agregore — pointers for doorway federation brainstorming)

**Key references:**

- [Distributed Press](https://distributed.press) + the [Social Inbox](https://github.com/hyphacoop/social.distributed.press) (Hypha Co-op) — open-source publishing that seeds to HTTP/IPFS/Hypercore and gives static sites a minimal ActivityPub inbox. [FEP-1042 "Peer to Peer Fediverse Identities"](https://codeberg.org/fediverse/fep/src/branch/main/fep/1042/fep-1042.md) (a draft FEP, [announced](https://distributed.press/2024/08/14/our-shiny-new-bridge-between-peer-to-peer-protocols-and-activitypub-implementations/) by Distributed Press as "FEP-1024") bridges P2P content into the fediverse via HTTPS aliases; [Agregore](https://agregore.mauve.moe/) is the distributed-web "loyal client." [Cross-pollination survey →](epr:hypha-distributed-press-cross-pollination-2026-06-23)

---

## The Trust Substrate

Why Holochain and not a blockchain? Because agent-centric architecture means your data lives with you, validation is intrinsic to the data type, and there's no global consensus bottleneck. But the DHT brings its own questions — gossip protocol tuning, sharding strategies for content-heavy networks, entry validation patterns that enforce constitutional constraints without central authority. The research here is about making the substrate invisible while keeping it trustworthy. On modeling reach/visibility as a derived, evidence-backed ontology (Palantir Foundry, REA/ValueFlows lineage, Zanzibar/ReBAC, semantic-web survivorship, Cyc's centralized-reasoner failure re-read for distributed inference): [July 2026 ontology-systems survey →](epr:ontology-systems-survey-reach-reconciliation-2026-07-22). The arc continues in three companions: a [letter to REA practitioners →](epr:letter-to-rea-practitioners-observed-presence-2026-07-22) naming the three organs REA lacks (a check algebra, a clock, and the observer); a [graduation evaluation of W3C OWL 2 →](epr:owl2-graduation-floor-ceiling-ontology-2026-07-23) — what we take from the field's most serious ontology attempt, what we refuse, and why the capability we most need (a first-class term for *where the ontology's judgment ends and a person's begins*) exists in no formalism and has already been authored three times in our own corpus; and a [letter to the shapes and policy practitioners →](epr:letter-to-shapes-practitioners-where-rule-ends-2026-07-23) — the contribution-back position, timed to SHACL 1.2 Core's Working Draft and the ODRL CG's in-flight evaluator semantics: the *refer* verdict, the positive witness, the derived-vocabulary declaration (all running code here), and the clock (named honestly as a gap, not a practice).

[elohim/holochain/research/](../../elohim/holochain/research/)

---

## The Content Addressing Problem

EPR content has three tiers — Head (gossipped metadata), Document (peer-cached body), Bytes (steward-delivered shards). IPFS gives us content-addressed delivery and verified fetch, but mapping EPR tiers to IPLD primitives (DAG-CBOR encoding, CID resolution) is non-trivial. The rust-ipfs fork (tracking `dariusc93/rust-ipfs` with connexa) is the vehicle; the research is about making content-addressed delivery feel instant while remaining verifiable. Adjacent prior art: **Distributed Press** seeds content to IPFS and **Hypercore**, and Hypercore's signed-merkle append-only log (sparse, content-addressed delivery) is a comparator for verified fetch — both in the [June 2026 cross-pollination →](epr:hypha-distributed-press-cross-pollination-2026-06-23).

[elohim/rust-ipfs/research/](../../elohim/rust-ipfs/research/)

---

## The Archival Problem

Content addressing answers *how bytes get named and verified*. It does not answer *whether they survive a decade*. The protocol's hot path is steward-delivered shards on a trust-weighted mesh — fast, pluralistic, governed by relationship. But what about cold storage? What about content that has graduated past active stewardship and needs a substrate that will still answer in twenty years, even if no household-cluster steward chooses to keep carrying it?

The DDS Working Group has converged on a trio — **Arweave**, **Filecoin**, and **Logos** — as their archival/storage substrate options. These communities have spent a decade building exactly the layer the protocol does *not* try to build natively. The survey question is whether (a) any of them are aligned enough that doorway projection makes sense as an optional cold-path bridge, (b) any are divergent enough that they're better treated as informative comparison points than interop targets, and (c) whether our working thesis holds: that **Elohim Protocol may be the comprehensive target for what these orgs say they are trying to do** — civil-society infrastructure, sovereign computation, permanent legibility — assembled around a stewardship model rather than a market or a token.

That thesis is worth testing rather than asserted. The survey is the test.

**Arweave** — Permanent storage as protocol primitive. The blockweave structure ties a new block to a randomly-sampled prior block, making storage-of-history the proof-of-work substrate; the storage endowment captures a one-time fee that funds perpetual replication actuarially. Closest to a "civilizational hard drive" framing — pay once, persist forever. Funding is endowment-based; governance leans toward minimal-protocol, market-driven.

**Filecoin** — Incentivized IPFS. Storage providers stake FIL and prove they hold contracted bytes via proof-of-replication and proof-of-spacetime; clients pay per deal; the network polices delivery via slashing. Closest CID/IPLD lineage to our own rust-ipfs work — the wire formats are nearly cousins. The hardest question is whether market-priced storage deals fit a stewardship economy at all, or whether they belong in a different value language entirely.

**Logos** (logos.co) — Status's broader sovereign-network stack. Frames itself as a social movement plus a unified technology stack: Waku (private p2p messaging), Codex (decentralised storage with durability guarantees), Nomos (consensus), plus networking and execution layers. Of the three, Logos is the closest peer to Elohim in scope — explicitly civil-society-oriented, full-stack, multi-protocol — and therefore the most important to read carefully for both alignment and divergence. *(Factual caveat: the Logos sub-protocols evolve fast; testnet was at v0.1.2 at survey time, and the public site does not always name Waku/Codex/Nomos directly. Confirm component status when cloning.)*

The deliverable from this survey is not a comparison matrix. It is a decision: which of these belongs in our doorway's projection layer as an optional cold-path archival target, and which are simply prior art we should learn from before deciding our cold path doesn't need their help at all.

[genesis/research/repos/arweave/](repos/arweave/) · [genesis/research/repos/filecoin/](repos/filecoin/) · [genesis/research/repos/logos/](repos/logos/)

---

## The Networking Problem

Steward nodes don't form flat peer networks — they form trust-weighted meshes where affinity to content determines topology. libp2p gives us the protocol primitives (request-response, gossipsub, Kademlia), but the research questions are about NAT traversal at scale (tx5, sbd relay), peer discovery that respects affinity, and how the steward topology emerges from individual stewardship decisions rather than being centrally planned.

[steward/node/research/](../../steward/node/research/) — `hypercore-holepunch-prior-art.md` (the Hypercore/Holepunch stack as data-plane + NAT-traversal prior art)

**Key references:**

- [Hypercore / Holepunch](https://github.com/holepunchto/hypercore) — a secure single-writer append-only signed log (BLAKE2b merkle) with **sparse replication** ("download only the blocks you need" = "replication follows relationship"), plus [HyperDHT](https://github.com/holepunchto/hyperdht): peer discovery by topic with **UDP holepunching** as a first-class feature. Structurally a Holochain source chain but *integrity-only* — a candidate data-plane substrate beside iroh/libp2p, never a truth layer. Autobase is multi-writer-convergence prior art (vs. Automerge). Surveyed alongside Distributed Press (which publishes to Hypercore) in the [June 2026 cross-pollination →](epr:hypha-distributed-press-cross-pollination-2026-06-23).

---

## The Peer Problem — Freenet

Every other entry in this index is prior art we borrow from. Freenet is different: it is another team solving *our* problem — a full-stack decentralized substrate with a live network of hundreds-to-low-thousands of peers, a whitepaper, published telemetry, a decentralized git forge, and an agent-development discipline that independently converged on our own (`skills/<name>/SKILL.md`, hooks, subagent review panels, design docs signed "[AI-assisted - Claude]"). It therefore gets a confrontation rather than a survey: compare, attack in both directions, and separate what we take from what we leave.

The bet each project makes is the inverse of the other's. Freenet's thesis is to *"separate what merges from how it propagates"* — the application supplies its own lattice (an idempotent commutative monoid) and the platform stays generic over the algebra. We fix the substrate's semantics — a validating DHT, notarized provenance, an 8-level reach ordinal, REA commitments — and let applications compose within them. **Freenet buys generality and pays in refusal; we buy refusal and pay in generality.** That single trade predicts nearly every difference: their merge is *total*, so it cannot express "reject this update unconditionally" — no double-spend defense, no uniqueness, no revocation-that-can't-be-un-revoked — and their contract interface `validate_state(state, params, related)` has **no requester parameter**, so authorization is inexpressible at the layer that owns state. Their access model is binary (replicated-public contracts vs device-local-private delegates) with encryption as the only thing in between; there is no gradient, and no venue in which an outsider could negotiate for one.

What we take, in order of leverage: **capability-relative hosting budgets** sized from `min(RAM, cgroup)` (we already read the cgroup limit and never consume it); *"distance is a **placement** input, never a **retention** input"*; *"upstream is computed, not stored"* (derived state self-corrects, formation flags rot, and a strict total order makes cycles structurally impossible); *"there is no relay category"* as an ontology-collapsing audit; and the process rules — **"instrumentation is horizontal"** (a PR adding a mechanism without telemetry *for that mechanism* gets bounced) and per-node-aggregate-scalars-only telemetry. What we leave: location derived from network address (their `connection_manager.rs` carries a signed *"DISCLOSED and ACCEPTED"* eclipse tradeoff with an unevictable attacker), state that is unverifiable from its key, and total merge as the state model.

The sharpest mutual finding is a shared pathology rather than a difference: **dead configuration that reads as shipped capability.** As surveyed 2026-07-27: their paper claims WASM fuel limits while `enable_metering: false`; ours declared `enable_eviction: true` with zero readers. A *"constant or flag with no reader"* lint is the most reusable artifact the engagement produced.

[Peer confrontation →](epr:freenet-peer-confrontation-2026-07-27) · implementation sequencing in [`freenet-lift-and-shift`](../docs/superpowers/plans/2026-07-27-freenet-lift-and-shift-plan.md) · manifest clones: `freenet-core`, `freenet-git`, `freenet-agent-skills`, `freenet-paper-1`.

---

## The Ancestor — Secure Scuttlebutt

Secure Scuttlebutt (2014–~2024) is the offline-first, gossip-replicated social protocol that most of today's local-first field descends from: identity was a bare cryptographic keypair, each identity published one append-only signed feed, replication followed the friendship graph, and everything hard — account recovery, moderation, economics, deletion, using two devices — was deliberately left out of the protocol for the social layer to absorb. We engaged it because it is a *completed* experiment whose own architects wrote the post-mortem before moving on: its refusals are, item for item, this protocol's feature list, which makes SSB the control group for our whole design. The retrospective (surveyed August 2026, from five cloned ssbc repos and the successor ecosystem) compares ten axes and sorts the results into what we **take** (they shipped private-message encryption for a decade while our encryption layer remains unbuilt — the one axis where they are decisively ahead; their replication-bandwidth disciplines; their USB-stick offline story, which we lack entirely), what we **watch** (their volunteer relay servers were designed to fade in importance and became de-facto infrastructure anyway — the same trap our doorway gateways face; their hour-long gigabytes-heavy first sync capped growth at tens of thousands of users, a ceiling our full-replication conductors share the shape of), and what we **leave** (key-as-identity with no recovery path, feeds that break permanently when two devices write at once, permanence so absolute that users feared their own posts).

[Ancestor retrospective →](epr:ssb-scuttlebutt-ancestor-retrospective-2026-08-03) · manifest clones: `ssb-protocol-guide`, `ssb-db2`, `ssb-server`, `ssb-ebt`, `ssb2-discussion-forum`.

---

## The Sibling Craftsman — p2panda

p2panda is the SSB descendant that took the ancestor's post-mortem seriously: a three-person Berlin team (same CCC/solarpunk milieu SSB came from) that deprecated its own monolithic node (aquadoggo) in 2024 and rebuilt as ten data-type-agnostic Rust building blocks over iroh — an I/O-free core, trait-per-domain stores, pluggable sync protocols, a research-grounded group-encryption crate, bring-your-own-CRDT — with a shipped GNOME collaborative editor (Reflection) as the worked proof. It is materially the closest stack to ours in the whole field (Rust, iroh, CBOR, Ed25519, BLAKE3, SQLite, append-only logs), which makes it the one project whose *engineering discipline* transfers almost without translation — and that discipline is what the survey (August 2026) extracts: workspace-declared lints, feature-matrix CI, changelog/release runbooks, and above all the proof that a team can dissolve its own monolith into blocks that outlive the framework. The survey adjudicates this against a grounded audit of our own workspace (the building-blocks-vs-services gap), sorts a seven-item adoption program (manifest-declared lint contracts through a ranked crate-extraction sequence), flags `p2panda-encryption` as a serious candidate for our unbuilt encryption layer (audit status unresolved), and holds the same cardinal line as the Holepunch and SSB engagements: writer-signed validity, key-as-identity, and ACL-as-authority never cross into the truth plane.

[Cross-pollination survey →](epr:p2panda-cross-pollination-2026-08-04) · manifest clones: `p2panda`, `p2panda-aquadoggo`, `p2panda-reflection`.

---

## The Substrate Mirror — Holepunch

[Holepunch](https://github.com/holepunchto) — the company behind the **Hypercore** stack and the **Pear** runtime — is the most mature fully-P2P application stack in the JS ecosystem: append-only logs, a holepunching DHT, multi-writer convergence, blob/FS composition, reliable UDP, and a real shipped app (Keet) with **no servers**. Where Distributed Press mirrors the *doorway* and Hypha DAO mirrors the *collective*, Holepunch mirrors **the substrate itself** — how bytes move, how peers find each other across NATs, and how content stays available without leaking read access.

The one-line verdict: it is the most architecturally-aligned external P2P stack we have surveyed, having independently rebuilt our entire data plane — **and that is exactly why the integrity-vs-validity line must be drawn most carefully here. Alignment is the seduction, not the safety.** Borrow the transport and the blind-hosting patterns; the trust layer stays ours. Compressed for outreach: *Holepunch trusts the writer's key; Elohim trusts the network's validation — integrity is what a Hypercore proves, validity is what only the DHT can.*

Note this survey **supersedes in scope** (not replaces) the two earlier Holepunch notes: `steward/node/research/hypercore-holepunch-prior-art.md` is a single-primitive capture, and the Distributed Press survey saw Hypercore through the publishing lens. This one is org-wide, gate-adjudicated, and seam-routed against grounded build-state.

[Cross-pollination survey →](epr:holepunch-p2p-dataplane-cross-pollination-2026-06-24)

---

## The Collective — Hypha DAO

[Hypha DAO](https://github.com/hypha-dao) is a decade-deep project building tooling for DAOs/DHOs — "Decentralised Human Organisations" — and the closest external mirror for our *autonomous-entity and collectives* work (the recursive-Qahal substrate: `Collective` / `Membership` / `CollabAgreement`).

The one-line verdict: **Hypha is the closest philosophical fellow-traveler the protocol has, and its substrate is the cleanest thing to reject.** It independently reached human-at-the-heart, collective-stewardship-as-apex, capital-decoupled-from-voice, non-transferable decaying standing, and fractal/membranic nesting that maps almost 1:1 onto recursive-Qahal — then built all of it on a global-consensus token blockchain, which is precisely the trust root we exist to avoid. Borrow the mechanics and the framing; reject the chain and every transferable token.

**⚠ Name-collision guard** — this is the `hypha*` scar tissue the index warns about, and it has already cost confusion once. **`hypha-dao`** (this survey, the collective) ≠ **`hyphacoop` / Distributed Press** (the doorway survey, `2026-06-23`) ≠ **hypha-network**. Check the date suffix before citing.

[Cross-pollination survey →](epr:hypha-dao-autonomous-collectives-cross-pollination-2026-06-24)

---

## The Stewardship Problem

"Take it with you" means a desktop app that runs a Holochain conductor, stores your data locally, and works offline. Tauri gives us the native shell; the hard problems are conductor embedding lifecycle, identity handoff between web and native contexts, deep link routing across cold and warm starts, and making offline-first feel seamless rather than degraded. This is where stewardship stops being a principle and becomes a UX.

[steward/device/research/](../../steward/device/research/)

---

## The Assessment Problem

How do you measure human growth without reducing it to a score? Sophia renders assessments in three modes — mastery (graded), discovery (psychometric), and reflection (open-ended). The research is about what makes each mode work: IRT for adaptive difficulty, instrument design that surfaces authentic values rather than socially desirable answers, and scoring models that inform the learning path without flattening the learner.

[sophia/research/](../../sophia/research/)

---

## The Deliberation Problem

How does the protocol surface, mediate, and verify collective deliberation without flattening it? qahal is the pillar where disagreement is supposed to be tended rather than averaged away, mishpat is the layer where binding governance acts get notarized, and elohim-as-counsel is the standing that lets a single voice survive when the room is structurally tilted against it. Each of these touches the same hard problem: how do you build infrastructure that approximates Habermas's ideal-speech conditions — equality of voice, freedom from coercion, openness to the better argument — at the scale and asynchrony of a distributed protocol, without importing his idealizations about culturally homogeneous publics or about consensus as the goal?

The recent surge of AI-mediated deliberation tools (DeepMind's Habermas Machine, Polis's bridging-statement detection, ZKorum's DDS standard) makes this an urgent research area. The protocol's instinct is to treat AI-generated consensus statements as *attested computations* — auditable inputs to qahal, never binding outputs — and to preserve dissent as a first-class signal rather than a failure mode.

**Key references:**

- DeepMind, "AI can help humans find common ground in democratic deliberation", *Science*, October 2024 — The Habermas Machine. A two-LLM pipeline (generative model + personalized reward model) that drafts consensus statements and ranks them by predicted endorsement; outperformed human mediators on clarity, fairness-to-minorities, and informativeness in a 5,734-participant UK study. The system is impressive on its own terms and structurally misaligned with Habermas's own framework — it optimizes for *predicted endorsement* where Habermas asked for *defensibility under ideal-speech conditions*. Surfaced via the DDS-WG (ZKorum) author's stated admiration; the protocol's interest is in auditing rather than adopting this pattern. [Survey notes →](epr:habermas-machine-2024)
- Habermas, *The Theory of Communicative Action* (1981), *Discourse Ethics* (1990s), *Structural Transformation of the Public Sphere* (1962) — The intellectual scaffolding for any serious work on legitimate collective deliberation. The communicative-vs-strategic-rationality distinction maps directly onto the shefa-vs-qahal boundary; the ideal speech situation supplies a regulative ideal for measuring how well any deliberation infrastructure does its job; discourse ethics gives the universalizability-not-majoritarianism wedge that the protocol's constitutional floors rely on. Critiques from Fraser, Young, Mouffe, and Rancière name the places (cultural homogeneity, power asymmetry, idealization of consensus) where the protocol must revise rather than import the framework. [Survey notes →](epr:habermas-legacy)
