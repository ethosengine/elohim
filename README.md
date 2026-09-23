# Elohim Protocol

[![Contribute](https://eclipse.dev/che/contribute.svg)](https://code.ethosengine.com/#https://github.com/ethosengine/elohim/tree/dev) [![Build Status](https://jenkins.ethosengine.com/buildStatus/icon?job=elohim%2Fdev)](https://jenkins.ethosengine.com/view/ethosengine/job/elohim/job/dev/) [![Quality Gate Status](https://sonarqube.ethosengine.com/api/project_badges/measure?project=elohim-app&metric=alert_status&token=sqb_4f435ff318c7541e4d9407bcfdc13e7268549493)](https://sonarqube.ethosengine.com/dashboard?id=elohim-app)

This is the monorepo for the Elohim Protocol: digital infrastructure organized around love, built as a peer-to-peer substrate with constitutionally bounded AI agents. The vision is set out in the [manifesto](./genesis/docs/content/elohim-protocol/manifesto.md). If you are not a developer, start reading at [elohim.host](https://elohim.host), the project site, or let an AI orient you with the badges below. If you want to run the code, go to [Development](#development).

**Bringing your own context?** Let an AI read this repo, ask what *you* care about, then tell you why the Elohim Protocol might matter to you and how you could get involved.

[![Ask Claude about elohim](https://img.shields.io/badge/Ask_Claude_about_elohim-D97757?style=for-the-badge&logo=claude&logoColor=white)](https://claude.ai/new?q=Explore%20https%3A%2F%2Fgithub.com%2Fethosengine%2Felohim.%20Before%20explaining%20anything%2C%20get%20a%20sense%20of%20who%20I%20am%20and%20what%20I%20care%20about.%20Then%20tell%20me%20why%20the%20Elohim%20Protocol%20might%20matter%20to%20me%20and%20how%20I%20could%20get%20involved%2C%20through%20that%20lens.) &nbsp;&nbsp;*recommended*

[![Ask ChatGPT about elohim](https://img.shields.io/badge/Ask_ChatGPT_about_elohim-000000?logo=openai&logoColor=white)](https://chatgpt.com/?q=Explore%20https%3A%2F%2Fgithub.com%2Fethosengine%2Felohim.%20Before%20explaining%20anything%2C%20get%20a%20sense%20of%20who%20I%20am%20and%20what%20I%20care%20about.%20Then%20tell%20me%20why%20the%20Elohim%20Protocol%20might%20matter%20to%20me%20and%20how%20I%20could%20get%20involved%2C%20through%20that%20lens.)

[![Ask Perplexity about elohim](https://img.shields.io/badge/Ask_Perplexity_about_elohim-20808D?logo=perplexity&logoColor=white)](https://www.perplexity.ai/search?q=Explore%20https%3A%2F%2Fgithub.com%2Fethosengine%2Felohim.%20Before%20explaining%20anything%2C%20get%20a%20sense%20of%20who%20I%20am%20and%20what%20I%20care%20about.%20Then%20tell%20me%20why%20the%20Elohim%20Protocol%20might%20matter%20to%20me%20and%20how%20I%20could%20get%20involved%2C%20through%20that%20lens.)

<p align="center">
  <img src="app/graphos/design-assets/vision/citizen-assembly-p2p-substrate-v4-story-value-governance.png" alt="A nighttime community: neighbors sharing a meal and one another's company around a long table while others garden, build, teach, and make; at the center a luminous woman writes with a quill among cards marked with attestation checks; overhead a constellation of AI agents carries the deliberative load; roots spread underground." width="900">
</p>

<p align="center"><em>At the center, an elohim sits beside us — a young Wisdom with a quill, a witness to our life — and writes the ledger of care: who tended, who taught, who mended, every entry attested. Near her, a life well lived; above, the council carries the deliberative weight, balance, and judgments no single person need hold, so the people are freed to live while the machines absorb the variety. She keeps the substrate of a commons new but ancient, and tends with us its living tree of knowledge. (Concept illustration.)</em></p>

## About

The Elohim Protocol is digital infrastructure organized around love as a fundamental operating principle, implemented through distributed architecture and protected by AI agents that serve human flourishing rather than institutional power. Its network is peer-to-peer and designed so that no single party, including its creator, can own or control it. Its AI agents are designed to be bound by a written constitution and to act under each person's authorization.

The name comes from *[elohim](https://www.youtube.com/watch?v=U5iyUik97Lg)* (Hebrew, plural), the "heavenly host" of ancient texts, used here to mark the healthy role of AI in human life: powerful, useful, and never an object of devotion. Each person's *[elohim](./genesis/docs/content/elohim-protocol/glossary.md)* acts on their behalf and their community's, under their authorization, holding to what their best self would want in a calm moment. The *Elohim Protocol* is the substrate those agents share. In this document's prose, lowercase *elohim* refers to an agent and the capitalized *Elohim Protocol* refers to the substrate; `elohim` in code font is a directory or package name, and names and titles keep their own capitalization.

> The Judeo-Christian ontology and thought here come *with the founder* (introduced under [Why This Exists](#why-this-exists)): a chosen lens, not a claim on the territory. A peer project approaches the same entity through a different tradition, where it is called a Kami: the Shinto word for the spirits that inhabit places and things, and the word Botao "Amber" Hu reaches for in *[Kami of the Commons](https://arxiv.org/abs/2602.14940)* (arXiv:2602.14940, Feb 2026). That project lives at [civic.ai](https://civic.ai) ([audreyt/civic.ai](https://github.com/audreyt/civic.ai)), from Audrey Tang and Caroline Green. Different ontology, kindred aim: two lenses arriving separately at the same shape of bounded, inspectable, community-governed AI held as a commons.

This monorepo holds the substrate being built toward that vision, in Rust, Holochain, libp2p, iroh, Angular and Tauri. Some of it runs today on a household mesh (every kind of peer the network has, run together on one machine) and on an alpha fleet (the deployed test network); some is designed and not yet running. [What runs today](#what-runs-today-september-2026) says which. The design aims at technology that:

- Serves love rather than engagement metrics
- Protects vulnerability through capture-resistant design
- Enables community wisdom to scale through distributed governance
- Makes extraction expensive by design
- Supports human creativity without algorithmic manipulation

## Why This Exists

The vision is far older than the tools, and none of the core ideas here are new: this work stands in a lineage, not ahead of it. Henry George named the mechanism and its corrective in *Progress and Poverty* (1879): rent extracted from the commons, returned to the community. W. Ross Ashby and Stafford Beer named the mechanics of a viable, freedom-preserving system — requisite variety, the algedonic signal, *[Designing Freedom](https://en.wikipedia.org/wiki/Designing_Freedom)* (1973) — cybernetics for human liberty rather than control. And in recent decades brilliant people have built pieces of what a love-organized digital infrastructure could look like. Lynn Foster and Bob Haugen's [ValueFlows](https://valueflo.ws/) ([Mikorizal](https://mikorizal.org/)) ontology for honest economic accounting. The [platform cooperativism](https://platform.coop/) movement for worker-owned digital spaces. Arthur Brock and Eric Harris-Braun's [Metacurrency Project](https://metacurrency.org/) and its runtime lineage, [Holochain](https://holochain.org/) (agent-centric, no global consensus) and [Unyt](https://unyt.co/) (Holochain-native payment rails for mutual credit and community currencies). Jürgen Habermas's account of legitimate deliberation, made mechanical in DeepMind's [Habermas Machine](https://www.science.org/doi/10.1126/science.adq2852) (2024), a machine that mediates a divided group toward common ground without flattening the minority. The [Center for Humane Technology](https://www.humanetech.com/) for articulating what's broken. The pieces have been on the table for a long time; what has been missing is the substrate to hold them together.

Before AI, the cooperative and commons projects among these were small and idealistic, built by under-resourced people with no way to scale. Their problem was complexity. You need a framework that holds information, values, and responsible governance in tension with each other, and the classic build-fast, build-cheap, build-quality triangle breaks down when you refuse to sacrifice any of them. Every previous attempt has either produced a limited solution that neglects the whole (one beautiful leg of a three-legged stool) or had its idealism handed to capitalism, which sees ideals without profit as worthless to pursue.

AI changes that equation. It collapses the coordination cost that made the full stool impossible for small teams: the overhead of holding information, value, and governance in tension all at once.

This protocol is being built by a father of three in San Antonio, working evenings after bedtime, using the same AI tools that may eventually consume his professional role — to build infrastructure for human flourishing after displacement. That is the moment we are in: the tools that threaten to flatten human work can also, if we build the right architectures, make the things that matter most — thinking, caring, teaching, governing together — structurally valuable in ways that can't be extracted away.

> "The radical proposition at the heart of this manifesto is that love—not as sentiment but as committed action toward mutual flourishing—can be encoded into technological systems."
>
> — [The manifesto](./genesis/docs/content/elohim-protocol/manifesto.md), "Conclusion: Love as Technology"

## Key Concepts

These are the design's commitments, stated as the design states them; [What runs today](#what-runs-today-september-2026) marks which of them the running network enforces.

**Three Inseparable Dimensions**: Every piece of content in the protocol carries knowledge, value and governance, coupled at the architectural level before anything is created or distributed. The specification's first two rules follow from that: no value-blind content, and no governance-free content. These three-part records are called **EPRs** (Elohim Protocol Records; the [specification](./genesis/docs/content/elohim-protocol/protocol-specification.md) also reads the R as Reference). An EPR is the protocol's primitive unit of meaning, and its three dimensions map to three pillars: knowledge to `lamad`, value to `shefa`, governance to `qahal`. [How Content Links Work](./genesis/docs/content/elohim-protocol/epr-developer-guide.md) explains it in plain language.

**Distributed Infrastructure**: Peer-to-peer networks have no single point of control, and peer-to-peer designs have often resisted anyone who would charge rent on the combined participation of everyone in them. The protocol will never make anyone fabulously rich. A P2P technology with anti-capture mechanisms baked into its design makes wealth extraction very difficult, because the architecture functions as a complexity upgrade that accounts for the failures of the internet to protect real values. It relies on faithful cooperation, not captive audiences.

**A Constitution as the System Prompt**: If LLMs have ingested most of humanity's written expression, they reflect us back, the wisdom and the cruelty alike, and that reflection needs constitutional constraint. The protocol keeps its values in a [constitution](./genesis/docs/content/elohim-protocol/constitution.md) that a community can read, argue with and change, rather than in trained weights. The agents are designed to run bound by that text, the way a model follows a system prompt, and to act under their person's authorization.

**Formation Over Transaction**: Understanding is measured by social reach and content stewardship, not grades or engagement metrics. It accrues as **standing**: a relational track record of trust that grows as peers attest that your contributions were useful enough to pass on. Standing is a shape of relationships, never a single score. AI can write your essay, but it can't make your community trust your judgment.

**Reach and Belonging**: **Reach** is how far a person's contributions travel beyond their intimate circle, on a scale that runs from intimate through trusted, familiar and community to commons and public. It is earned, and gated by standing. **Belonging** is never gated; only reach is.

**Graduated Intimacy**: The design separates spaces for personal exploration from protected commons, with consent boundaries that keep extremes from corrupting shared spaces.

## Domain Pillars

The protocol's vocabulary draws from Hebrew and Latin to name the human practices it serves rather than the engineering it requires. There are five pillars, and three of them anchor the three dimensions of an EPR: knowledge to `lamad`, value to `shefa`, governance to `qahal`. Each pillar declares its vocabulary as an SDK domain in `elohim/sdk/domains/` (see the [domain index](./elohim/sdk/domains/README.md)) and has an app surface: four live under `app/elohim-app/src/app/<pillar>/`, and `lamad` is its own bundle at [`app/lamad/`](./app/lamad/).

- [`imagodei`](./genesis/docs/content/elohim-protocol/imagodei.md) (Latin, "image of God") is identity: humans, presence, relationships, the attestations they make about one another, and the stewardship of self.
- [`lamad`](./genesis/docs/content/elohim-protocol/lamad.md) (Hebrew, "to learn / to teach") is learning, organized around paths. Knowledge is structured as territory (content-addressed, reusable ContentNodes: videos, documents, simulations), journeys (curated paths that give the territory sequence and narrative meaning), and travelers (learners whose progress and attestations shape what they encounter). The same node can be met on many paths, each giving it a different context. Learning is something you do through relationships and contributions; no platform certifies it.
- `avodah` (Hebrew, "work / service / worship") is work as service. A work item is an EPR, so it carries the three coupled dimensions, and it can declare attestation gates: lamad content a person masters before the task begins, so that open collaboration can be qualified by demonstrated mastery instead of credentials.
- `qahal` (Hebrew, "assembly") is community: collectives, consent, proposals and collective decision.
- [`shefa`](./genesis/docs/content/elohim-protocol/shefa.md) (Hebrew, "abundance / overflow") is economy: economic events recorded in REA (Resources, Events, Agents: the accounting vocabulary ValueFlows builds on), stewardship, resource flows.

Two more names appear in the code, and neither is a pillar. `mishpat` (Hebrew, "judgment") is a substrate domain with its own Holochain DNA (a term explained under [Architecture at a Glance](#architecture-at-a-glance)) and no app surface; qahal has no DNA of its own, and its collectives, proposals and decisions are recorded and validated in mishpat. `elohim` is the cross-pillar core the others compose on: infrastructure, data loading, trust. `doorway`, beside the pillars in the app, is the gateway integration. The agents themselves live in [`elohim/elohim-agent/`](./elohim/elohim-agent/).

## How Ubiquitous Wisdom Rebuilds the Internet

Every platform we have today shares a hidden architectural constraint: moderation is centralized because, before AI, intelligence was expensive. You couldn't put wisdom at every endpoint, so you concentrated it in moderation teams, spam-filter consortia and content review boards. That concentration became the platform's most valuable asset, its rent-extraction point and its capture vector. Whoever owns the chokepoint owns the network. [Ubiquitous Wisdom Dissolves the Chokepoint](./genesis/docs/content/elohim-protocol/architecture/ubiquitous-wisdom-dissolves-chokepoint.md) makes the full argument.

When each person's interaction with the network is mediated by their own elohim, wisdom can move from chokepoint to fabric. The protocol is designed to gate content in three places, with the elohim taking part in each:

- At authoring, the elohim reviews what its person is about to publish against their stated values and the protocol's coupling of knowledge, value and governance, before any reach is earned.
- At relay, each node's elohim decides whether to pass content on, given the recipient's stewardship contracts, trust context and the trail the content has already travelled.
- At consumption, the elohim shapes what surfaces for its person according to context, standing and care.

The coordination tools underneath stop being policy chokepoints and become shared infrastructure that wisdom uses. Content addressing gives each thing an identity derived from what it is. The Holochain distributed hash table (DHT) notarizes timing and lineage: it keeps a tamper-evident record, checked by peers, of when something was made and what it came from. libp2p and iroh move bytes. Doorways project to the legacy web.

In the design, who passed what to whom is recorded at every hop and sealed so that no single peer can open it alone. Feedback travels back along that path one hop at a time, like nerves carrying pain to a hand on a stove; the [social-reach nervous system](./genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md) specifies that mechanism. Quarantine signals travel alongside the content they flag. Restitution is designed as repair with real economic weight. Accountability is meant to land proportional to position in the chain (primary actor, accessory propagator, edge node), and the network self-heals at the edge. There is no central moderator to co-opt. [The Elohim Medium](./genesis/docs/content/elohim-protocol/social_medium/epic.md) tells the same story as daily life.

The capture target shrinks to something the protocol can defend. The old internet was capturable because information, value and governance shipped separately, and platforms sat in the propagation path of all three. A platform could absorb the medium of communication and run for years without economic accountability, scaling on borrowed conviction that returns would arrive eventually, while shedding responsibility for the harm flowing across it onto distant statutes and overburdened courts. Here, every EPR carries all three together over a peer-to-peer substrate, and no platform sits in the path to unbundle them. The design lets any single layer be routed around, so capturing one layer does not capture the system. What remains to capture is each person's authorization of their own elohim, which is that person's agency. That's a much higher bar. It will hold in full for a person who keeps their own keys; a hosted person's doorway keeps theirs and remains a target until they move up the [stewardship ladder](#progressive-stewardship), which is why hosting is a stage and not a destination.

AI deployed one way flattens human work; deployed another, it makes human judgment the part the network depends on. The protocol is a bet that the second deployment is possible.

## Watch

[![Elohim Protocol: From Digital Chaos to Collective Flourishing](https://img.youtube.com/vi/sVXwZ087ffA/maxresdefault.jpg)](https://www.youtube.com/watch?v=sVXwZ087ffA)

This is an AI-generated deep-dive conversation over the manifesto. It runs from the meta-crisis driving the need, through the protocol's design principles, to what daily life could feel like when technology is organized around care. *50 minutes. No code. Just the vision and the reasoning behind it.*

## Architecture at a Glance

An EPR couples knowledge, value and governance into one record whose address is derived from its content. The stack below exists to carry that record: to encode it so that its value and governance travel with its knowledge, to notarize when it was made and by whom, to move its bytes between peers, and to show it to the web. Two words recur. A projection is a derived, read-only view that can be regenerated from the layer beneath it and is never the source of truth; a layer that projects from another is a projection of it, and a doorway that projects to the web serves one to the web. Agent, on its own, means an elohim; Holochain also calls the keyholder whose key signs a record an agent, and where that meaning is needed this document says key or cell.

### Substrate

- Codec. [`elohim/epr/`](./elohim/epr/) (`elohim-epr`) is the root: canonical DAG-CBOR envelopes, CIDv1, Ed25519. An address is a hash of what a thing is, so the same identifier resolves anywhere. The envelope carries the record's reach and a coupling that names its value and governance records alongside its knowledge, which is how the specification's first two rules (no value-blind content, no governance-free content) are meant to hold at the level of the bytes. The codec has no Holochain dependency; every other layer builds on it, and ts-rs (a Rust-to-TypeScript type generator) produces its TypeScript bindings in `elohim/sdk/epr-ts/` for the browser.
- Notary floor. [Holochain](https://holochain.org/) 0.7. Each node runs a conductor, the Holochain runtime that runs DNAs. A DNA is a Holochain application's validation rules and data types, organized in modules called zomes, and a cell is one person's instance of a DNA. Each person keeps their own source chain (a signed, append-only history of their actions), validated by peers through a distributed hash table, with no global consensus and no central server in the path. Five DNAs in [`elohim/holochain/dna/`](./elohim/holochain/dna/) hold the rules for the protocol's structural commitments. imagodei holds humans, their attestations about one another, presence and relationships. lamad, packed from [`elohim/holochain/dna/elohim/`](./elohim/holochain/dna/elohim/), is the content store that lamad, shefa and avodah records share. mishpat holds qahal's collectives, proposals and judgments; qahal has no DNA of its own. infrastructure and node-registry hold which nodes exist, what capacity and capabilities they offer, and how healthy they are. Identity, stewardship contracts (who has taken on the care of which content or resource, and on what terms), economic commitments and reach all live on this floor. A record is checked against the DNA's rules when its author commits it and again by the peers that validate it; once the DHT holds it, it is notarized.
- Dataplane. `elohim-storage` ([`elohim/elohim-storage/`](./elohim/elohim-storage/)) stores and moves the bytes: chunked blobs, capability advertisement (peers announcing what they can serve), redundancy. It runs over two transports, libp2p and iroh, and the same content identifier resolves whichever transport delivered the bytes. Storage treats the DHT as the list of what should exist and keeps pulling its own holdings toward that list.
- Projection. The doorway ([`doorway/`](./doorway/)) is the layer that faces the legacy web. It brings browser traffic into the mesh and serves canonical content back out over HTTP as a read-optimized view. The doorway service does not itself join the libp2p or iroh swarms; it proxies to a conductor and caches projections, and it is never the source of truth.

The same codec reaches the repository itself. [`elohim/eprfs/`](./elohim/eprfs/) reads the repository's files as content-addressed records and carries the `epr` CLI, the repo's own governance tool. [brit](./elohim/brit/) is a fork of gitoxide (a Rust implementation of git) whose commits carry EPR provenance trailers; its brit-epr crate checks content-addressed citations between documents. [rakia](./elohim/rakia/) is a distributed build system (rakia-core, rakia-brit, rakia-executor) designed to turn build manifests into peer-attested, content-addressed artifacts.

### How a node takes part

The [seam map](./genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md) names four participation tracks. They are not tiers: a device can be on several at once, and what hardware it has decides which.

- Notary: keeping a source chain and validating other people's records on the DHT. Every person with an account is on this track, through their own conductor or through one that hosts them.
- Storage peer: a device with the capacity for it (a laptop or larger) runs `elohim-storage` and holds and moves bytes for others over libp2p or iroh.
- Spoke: a device too small to run a conductor or a storage peer (a wearable, a sensor, a phone) reaches a nearby peer over HTTP or WebSocket and contributes through it.
- Doorway: a well-provisioned node may additionally run a doorway that projects to browsers; the browsers consume through it without becoming peers.

Read against the [Progressive Stewardship](#progressive-stewardship) ladder: a Visitor only consumes through a doorway; a Hosted person is on the notary track through the doorway's conductor and consumes through its projection; an App Steward runs a conductor and a storage sidecar on their own device, so notary and storage peer; a Node Steward runs both always-on and may host spokes and a doorway. The hardware specification's device tiers are a third axis, what a device can run, distinct from tracks (how it takes part) and stages (who keeps the keys).

### The witness ladder

Across these layers the design follows a witness ladder. A claim starts as a local witness, one node's own record. It becomes peer-validated when other peers check it against the DNA's rules. It ends notarized when the DHT holds it across enough peers that no one of them can alter or drop it, and withdrawing a claim then means issuing a superseding one. The ladder is what lets a projection be fast without being trusted: what is true lives on the notary floor, bytes move on the dataplane, and a doorway or a storage database serves a legible view of the truth and is never its source.

### Surfaces

- Doorway ([`doorway/`](./doorway/)) is the projection layer's running service. Today it serves browsers and publishes its identity through a W3C DID bridge. It signs its endpoint record in the pkarr format, where records are addressed by public key; publishing that record is not yet wired. AT Protocol and ActivityPub bridges are planned. Doorways are plural and replaceable: several can serve the same content, and none of them owns it.
- Steward shells ([`steward/`](./steward/)) are the software a person runs at the App Steward and Node Steward stages of the [Progressive Stewardship](#progressive-stewardship) ladder. On that ladder a steward is someone who runs their own peer; elsewhere in the protocol, stewardship is a caretaking relation to a resource or content. `device/` is a Tauri desktop app that embeds a Holochain conductor for self-custodied keys; `node/` is a headless always-on runtime for household hardware.
- elohim agents ([`elohim/elohim-agent/`](./elohim/elohim-agent/)) are implemented as a Rust agent crate with a streaming LLM backend, bound to the constitution at runtime. They are not yet deployed as a running service. `elohim/eae/` is a separate crate: the monitor, analyze, decide, execute loop of an Elohim Autonomous Entity, the agent-run organization in [the autonomous entity story](./genesis/docs/content/elohim-protocol/autonomous_entity/epic.md).
- Sophia ([`sophia/`](./sophia/), a fork of Khan Academy's Perseus) renders three kinds of human moments: **Perseus** for mastery exercises (graded), **Psyche** for discovery and reflection (psychometric, open-ended), and **Psephos** (Greek: "voting pebble") for governance ballots with election hygiene.

### What runs today (September 2026)

Running:

- The household mesh, set up as a test household. `just mesh start` launches it (see [Development](#development)).
- The alpha fleet of doorways, storage and conductors. [alpha.elohim.host](https://alpha.elohim.host) serves the Angular apps, which bundle Sophia's renderers, and [doorway-alpha.elohim.host](https://doorway-alpha.elohim.host) is its doorway: it answers the apps' API calls and projects content to the web.
- The Visitor and Hosted stages of the [stewardship ladder](#progressive-stewardship), on the alpha doorways, including the doorway's side of moving a hosted person's key to their own device.
- The first stage of the authoring gate, with no agent in the loop: on the path that stores an EPR, `elohim-storage` classifies whether the signer is authorized for the reach the record declares, and refuses to store it if not. Public and commons reach are always allowed; intimate, trusted, familiar and community reach need a signer the node already knows, since a claim on someone's circle is accepted only from a signer the node holds an identity record for. This checks who may declare a reach, not whether reach has been earned; earning is the designed second stage listed below.
- Feedback back-propagation and the standing projection, in `elohim-storage`: when a feedback signal arrives, a peer updates the standing it sees and passes the signal one hop back toward where the content came from.
- Quarantine signals, a kind of feedback signal that travels the same path.

Designed, not yet running:

- The agent runtime. The crate is implemented, but no service runs it, so no elohim takes part in any gate yet.
- Reach gated by standing. The evaluator that weighs an author's standing is written and tested but not yet on the storage path; cases it cannot decide are meant to go to the person's elohim.
- Enforcement of avodah's attestation gates at bid or acceptance; a work item declares them and its page shows them.
- The relay and consumption gates.
- The quarantine short-circuit at relay, and restitution as economic events.
- The feed protocol (the specification's `/elohim/feed/1.0.0`).
- The AT Protocol and ActivityPub bridges.

The desktop steward app (`steward/device`) and the node runtime (`steward/node`) build from source, and their pipeline runs only on demand; no packaged release is linked from this README.

Production deploys are paused, and the apex elohim.host is served by an alpha doorway. The live source of truth is the habit register ([How we work](#how-we-work) explains it): `just status habits` lists what the system reliably does, each entry bound to a runnable check, and it is where the pillar-level answer (what a person can do in each app today) lives; the list above is scoped to the substrate.

For the full picture, read the architecture [index](./genesis/docs/content/elohim-protocol/architecture/INDEX.md) and [map](./genesis/docs/content/elohim-protocol/architecture/MAP.md), the [seam map](./genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md), and [How Content Links Work](./genesis/docs/content/elohim-protocol/epr-developer-guide.md).

## Repository Structure

Organized by system boundary: core runtime, frontend apps, interop bridges, deployment shells, the web2 doorway, and content and operations. Each directory's own README.md or CLAUDE.md is the authority for what it holds; this tree reflects September 2026. To read the code, start with `elohim/epr/` (the codec every layer builds on), then `elohim/elohim-storage/`, then `doorway/`. Git submodules and forks are marked. Build output (`target/`, `target-pool/`, `node_modules/`) is omitted.

```
├── elohim/                        # Core runtime (Rust + Holochain)
│   ├── epr/                       # elohim-epr: canonical EPR codec (DAG-CBOR, CIDv1, Ed25519), the content-addressing root
│   ├── epr-rea/                   # REA / ValueFlows economic domain layer over EPRs
│   ├── eprfs/                     # Filesystem projection layer: eight crates, including eprfs-cli and the epr CLI (epr-cli)
│   ├── elohim-storage/            # P2P content storage service: chunked blobs, redundancy, reconciliation
│   ├── elohim-error/              # Storage error types, split out of elohim-storage
│   ├── elohim-settings/           # Storage boot settings and hot-reloadable runtime settings, split out of elohim-storage
│   ├── elohim-cache-core/         # Caching primitives (blob, chunk, extraction), native and WASM
│   ├── elohim-compute/            # Compute capacity and health reporting types
│   ├── elohim-facings/            # Read-only summaries of who holds what, where (no database access)
│   ├── elohim-peer-fabric/        # Peer ranking and traffic defense shared by storage and doorway
│   ├── elohim-render/             # Server-side JavaScript renderer for pages served before the app loads
│   ├── elohim-chrome-asset/       # Pre-built omnibar widget served by doorway and storage
│   ├── elohim-token/              # Interface for bridging value to outside settlement rails
│   ├── elohim-hub/                # Reserved name for a future home-node cluster runtime (README only)
│   ├── elohim-bitswap/            # IPFS Bitswap block exchange
│   ├── constitution/              # The crate that loads the constitution and enforces it at runtime
│   ├── eae/                       # Elohim Autonomous Entity: agent decision loop and governance escalation
│   ├── elohim-agent/              # Agent runtime
│   │   ├── elohim-agent-service/  # Rust agent crate: LLM backends, streaming, capability registry
│   │   ├── elohim-agent-sdk/      # TypeScript agent sidecar
│   │   ├── gate-client/  gate-client-zome/  gate-types/   # Authoring and relay gate: native client, zome shim, shared types
│   │   ├── specialists/  research/  spec/                 # Specialist agents, research notes, gate interface spec
│   │   └── mcp-servers/           # Model Context Protocol server that turns docs into content-graph seed data
│   ├── ark/                       # Process supervisor that launches and watches conductors
│   ├── lvi/                       # Peer-to-peer dev workspace runtime (design stage, no code)
│   ├── conductor-image/           # CI manifest that dispatches the custom Holochain conductor build
│   ├── sdk/                       # Protocol SDK
│   │   ├── schemas/               # Protocol JSON Schemas (v1), the source of truth for generated types
│   │   ├── domains/               # Domain vocabularies: the five pillars plus mishpat, infrastructure, elohim, elohim-agent
│   │   ├── epr-ts/                # TypeScript bindings for the EPR codec
│   │   ├── storage-client-ts/     # Generated TypeScript wire types plus a hand-written HTTP and dataplane client
│   │   ├── scripts/  fixtures/    # App packaging scripts; conformance test vectors
│   │   └── src/                   # Legacy Holochain zome-call SDK (@elohim/holochain-sdk)
│   ├── holochain/                 # Holochain layer
│   │   ├── dna/                   # Five DNAs: lamad, imagodei, mishpat, infrastructure, node-registry (archive/ holds v1)
│   │   ├── tools/hc-dbtool/       # Operator tool: see and lift a blocked cell in a conductor's databases
│   │   ├── edgenode/              # Container packaging for one edge node (conductor + storage)
│   │   ├── elohim-wasm/           # Browser-side WASM blob verification
│   │   ├── rna/                   # DNA-to-DNA data migration toolkit
│   │   └── docs/  tests/          # Developer docs for this layer; multi-conductor sweettests and DNA manifest checks
│   ├── brit/                      # Fork of gitoxide whose commits carry EPR provenance (git submodule)
│   ├── rakia/                     # Distributed build system: rakia-core, rakia-brit, rakia-executor (git submodule)
│   └── holochain-conductor/       # The pinned conductor fork the fleet's conductor image is built from (git submodule)
│
├── app/                           # Frontend applications (Angular 22)
│   ├── elohim-app/                # Main Angular platform
│   │   └── src/app/<pillar>/      # elohim · imagodei · avodah · qahal · shefa · doorway
│   ├── lamad/                     # Lamad learning app, built and served as its own bundle
│   ├── imagodei-portal/           # Sign-in portal for stewards running their own peer
│   ├── elohim-elements/           # Lit web components, one package per pillar
│   ├── graphos/                   # Design spec, vocabulary and design assets (the design system's home)
│   ├── elohim-library/            # Shared Angular libraries (eight projects, including elohim-service, elohim-identity, graphos)
│   ├── scripts/                   # Lint rules shared by every app (route literals, SSR entry, cross-workspace imports)
│   └── workspace-runtime/         # The one place that knows the dev-workspace hostname convention
│
├── bridges/                       # Interop crates that translate external protocols to and from EPR-REA
│   ├── did/                       # W3C DID documents for elohim identities
│   ├── k8s/                       # Renders and checks Kubernetes resource manifests (no live cluster access)
│   ├── pkarr/                     # Signed endpoint records for doorways (pkarr)
│   └── valueflows/                # ValueFlows / hREA (VF-GraphQL)
│
├── crates/                        # Published Rust SDK crates: elohim-views (the ts-rs wire types), doorway-client, elohim-sdk, elohim-storage-client, seam-contracts
│
├── sophia/                        # Assessment engine, forked from Khan Academy's Perseus (git submodule, 18 packages)
│   └── packages/                  # sophia-element/-core/-editor/-linter/-score, perseus-core/-score,
│                                  #   psyche-survey, psephos/-element, plus math and markdown utilities
│
├── steward/                       # Deployment shells
│   ├── device/                    # Tauri desktop app (embeds a Holochain conductor)
│   └── node/                      # Headless always-on runtime for household hardware (libp2p)
│
├── doorway/                       # Web2 doorway (bootstrap, signal, conductor gateway, projection cache)
│   ├── doorway-service/           # Rust service
│   ├── doorway-app/               # Angular dashboard for doorway operators
│   ├── iroh-relay/                # Container packaging for the stock iroh relay that conductors home to, run beside the doorway
│   └── relay-addr-beacon/         # Keeps a home relay's changing WAN address published in DNS and pkarr
│
├── genesis/                       # Content, operations and CI
│   ├── orchestrator/              # CI controller: webhook → changeset → downstream pipelines
│   │   └── manifests/             # Kubernetes deployments per service and environment; ci-infra/ holds operator-applied CI infra and its runbooks
│   ├── manifests/                 # cluster-state.yaml and the generated habits.yaml register
│   ├── a2o/                       # Alpha-to-omega end-to-end tests (BDD scenarios, page-render tools)
│   ├── docs/                      # Protocol writing in content/ (manifesto, specification, epics); design specs and plans (superpowers/)
│   ├── data/                      # Seed inputs, fixtures, timeline backlog
│   ├── agentic/                   # Scripts and resource guards for agent-driven development
│   ├── seeder/                    # Content seeding pipeline (TypeScript)
│   ├── plans/                     # Archive of earlier plans (live plans: genesis/docs/superpowers/plans/)
│   └── research/  blobs/  assets/  scripts/   # Research surveys, seed blob pack, images, ops scripts
│
├── che-devworkspaces/             # Eclipse Che, Jenkins and dev container images (git submodule)
├── vendor/                        # One patched crates.io crate (iroh-quinn-proto 0.13, a fix elohim-storage applies through [patch])
├── scripts/                       # Repo-wide tooling (CI job bodies, local dev, Sophia releases)
└── patches/                       # pnpm patch for @angular/build
```

## Progressive Stewardship

The Elohim Protocol meets people where they are, with a gradual path from curious visitor to node steward, so that no one is excluded by technical barriers.

| Stage | Description | Data location |
|-------|-------------|---------------|
| **Visitor** | Anonymous browsing, no account | Browser memory only |
| **Hosted** | Account with a doorway (elohim.host runs the first ones); the doorway keeps your keys and runs your cell, your instance of the protocol's DNAs, on its conductor; the design lets you move doorways without losing anything | DHT network (hosted) |
| **App Steward** | Desktop app, self-custodied keys | Local device + DHT |
| **Node Steward** | Always-on infrastructure; a node steward may also host a doorway for others | Self-hosted + DHT |

Moving up a stage is designed to keep your identity, content, history and standing. The same signing key (your Holochain agent key) is designed to move with you: a hosted person exports it from their doorway to their own device, then confirms stewardship with the doorway. The doorway's side of that move runs today; the device side is in the desktop steward app, which builds from source. Recovery from a lost device is designed to be social: the people you trust confirm it's you. Hosted people sign in at their doorway, stewards sign in at their own runtime's portal, and apps never own a login.

The [hardware specification](./genesis/docs/content/elohim-protocol/hardware-spec.md) holds the canonical ladder, where the stages are named Visitor, Hosted User, App User and Node Operator; this README uses the names the app's code and the desktop steward app use. [Stewardship Over Sovereignty](./genesis/docs/architecture/stewardship-over-sovereignty.md) explains why the top rung is community-grounded autonomy rather than key custody.

## The Choice

We can accept digital feudalism, or we can build something structurally different.
We can encode extraction, or we can encode love.

**The time to build technology organized around love is now.**

## Development

### How we work

Humans and coding agents work in this repository side by side, and the way of working is written down once for both. [CLAUDE.md](./CLAUDE.md) is the operating manual: it carries the commands, the traps that bite, and where each concern lives, and [`AGENTS.md`](./AGENTS.md) projects the same text for other agent runtimes.

The scenario is the specification. Feature work starts from a Gherkin scenario under [`genesis/a2o/features/`](./genesis/a2o/features/) that describes the experience a person should have; the implementation is done when the scenario passes, and the two land together.

What the system reliably does is kept in a habit register, each habit bound to a runnable check. A habit's status flips only on evidence (a build, a live probe, a test run), never on intention, and `just status habits` renders the register (`just` is the command runner; the root `justfile` defines its recipes). The work at any moment is to move the top red habit toward green with proof.

Verification is scoped to the tree you touched: `just gate` finds the projects a change affects and runs their build and test checks, and the pre-push hook runs the same gates. The household mesh proves first and the alpha fleet confirms: a change is shown working on the multi-peer mesh on one machine before it is expected to work on the deployed network.

Agents work here under directory-local governance. A directory's `.epr-meta` declares the rules an edit there must satisfy, a hook checks them before a write lands, and READMEs and scenarios pass a fresh-context blind reader (a reviewer given the finished document and nothing else) before they land. The root developer interface is eight verbs, which `just --list` shows and CLAUDE.md explains: `gate`, `test`, `dev`, `mesh`, `seed`, `look`, `status` and `codegen`.

### Running it

There are three ways to run it, and CLAUDE.md carries the variants and the traps around each.

- A hosted workspace. The Contribute badge at the top opens an Eclipse Che workspace on code.ethosengine.com for people with an account there; the workspace image carries the toolchain, and the commands below run from its terminal.
- The frontend against live alpha data, with no local backend and no submodules (Sophia's renderers arrive as a prebuilt bundle that a build fetches, and the dev server runs without it). From a clone of [the repository](https://github.com/ethosengine/elohim) with Node 24 and pnpm 10 (pinned in the root `package.json`) and a Rust toolchain with the `wasm32-unknown-unknown` target and `wasm-pack`, run `pnpm install`, then `pnpm --filter elohim-app run prestart` once (it compiles a browser-side WASM module), then `pnpm --filter elohim-app start:alpha`, and open http://localhost:4200/.
- The full local stack: a conductor, a storage peer and a doorway on your machine. Beyond the frontend's needs it takes `just`, Holochain 0.7 (`holochain` and `hc` on your `PATH`) and `git submodule update --init --recursive`; `just dev start` launches it and, from the repository root, `pnpm app:dev` serves the app against it. `just mesh start` runs the multi-peer household mesh instead, which needs the pinned conductor fork that CLAUDE.md covers.

### Contributing

Open an issue on [GitHub](https://github.com/ethosengine/elohim), or fork the repository and open a pull request against `dev`. Run `just gate` before you push. Feature work lands together with the scenario that specifies it; fixes and docs need only the gate, and the blind-reader pass on a README runs on the maintainers' side at review. The license a contribution lands under is not yet settled; see [License](#license).

### CI

All GitHub webhooks go to one orchestrator job, which picks the pipelines a push affects and runs them in dependency order; [`genesis/orchestrator/README.md`](./genesis/orchestrator/README.md) explains how. `dev` deploys to the alpha fleet and `main` to production, whose deploys are paused (see [What runs today](#what-runs-today-september-2026)); the orchestrator README carries the full branch-to-environment rules.

## Further Reading

### Start here

- [Manifesto](./genesis/docs/content/elohim-protocol/manifesto.md): the vision.
- [Glossary](./genesis/docs/content/elohim-protocol/glossary.md): plain definitions of the terms the other documents use.
- [Values Forward](./genesis/docs/content/elohim-protocol/values-forward.md): where the protocol stands, so you can refuse it before entering.
- [Constitution](./genesis/docs/content/elohim-protocol/constitution.md): the law, written as a system prompt the agents are bound by.
- [The Confession](./genesis/docs/content/elohim-protocol/confession.md): the theology beneath the vision, stated plainly.
- [The Theology](./genesis/docs/content/elohim-protocol/theology.md): the same theology argued out as a disputation.
- [Succession Without Conquest](./genesis/docs/content/elohim-protocol/succession.md): the mutualist lineage read for what it could not afford; read it first if you arrive with a formed politics.

### Stories

- [The Elohim Value Scanner: How a Boy Buying Strawberries Changes Everything](./genesis/docs/content/elohim-protocol/value_scanner/epic.md): a family's care work made visible, starting at the corner store.
- [The Elohim Medium: When Social Media Becomes Sacred Space](./genesis/docs/content/elohim-protocol/social_medium/epic.md): a day in a social medium where reach is earned.
- [The Elohim Economic Coordination: Beer's Cybersyn on P2P](./genesis/docs/content/elohim-protocol/economic_coordination/epic.md): Stafford Beer's Cybersyn completed on a peer-to-peer network, told through one family.
- [The Elohim Autonomous Entity: When AI Buys Your Boss](./genesis/docs/content/elohim-protocol/autonomous_entity/epic.md): an exit to community for a franchise restaurant and its crew.
- [The Elohim Governance Story: When Agents Serve Without Ruling](./genesis/docs/content/elohim-protocol/governance/epic.md): how people keep powerful agents accountable.
- [The Elohim Public Observer: When AI Becomes Your Town's Memory](./genesis/docs/content/elohim-protocol/public_observer/epic.md): a school board meeting a parent can finally follow.
- [Living Memory](./genesis/docs/content/elohim-protocol/living_memory/epic.md): what human-scale data means, and why "right to be forgotten" means something different here.
- [Found, Not Crawled](./genesis/docs/content/elohim-protocol/search/epic.md): what search means in the protocol, and why the question stays home.
- [The Elohim Global Orchestra: When the World Learns to See Itself](./genesis/docs/content/elohim-protocol/global-orchestra.md): planetary coordination without a world government, built from aggregated value flows.

### Go deeper

- [Protocol Specification](./genesis/docs/content/elohim-protocol/protocol-specification.md): the v0.2 design spec for EPRs.
- [Shefa](./genesis/docs/content/elohim-protocol/shefa.md): the economic whitepaper on value accounting, stewardship and commons held in trust (a 2025 draft with a 2026 reading note).
- [Imago Dei](./genesis/docs/content/elohim-protocol/imagodei.md): a framework for human-centered digital identity.
- [Lamad](./genesis/docs/content/elohim-protocol/lamad.md): the learning pillar as a reference implementation of the protocol.
- [The Elohim Observer](./genesis/docs/content/elohim-protocol/observer-protocol.md): observation as witness rather than surveillance.
- [Resilience](./genesis/docs/content/elohim-protocol/resilience/README.md): mutual aid as substrate.
- [Hardware specification](./genesis/docs/content/elohim-protocol/hardware-spec.md): the device tiers and the four stages of participation.
- [Constitutional AI](https://arxiv.org/abs/2212.08073) (Bai et al., 2022): the training-time approach whose values this protocol moves out of the weights and into editable text (constitution, Appendix B).

### Build

- [Architecture index](./genesis/docs/content/elohim-protocol/architecture/INDEX.md) and [map](./genesis/docs/content/elohim-protocol/architecture/MAP.md): the graph of architecture specs, and a developer's walk through it.
- [Seam map](./genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md): where each concern lives, from smartwatch to home storage rack.
- [How Content Links Work](./genesis/docs/content/elohim-protocol/epr-developer-guide.md): what an EPR link carries, explained without jargon.
- [SDK domains](./elohim/sdk/domains/README.md): the vocabulary each pillar declares.
- [elohim.host](https://elohim.host): the project site.

## License

Licensing is per crate and package today (see each `Cargo.toml` and `package.json`); a root LICENSE and a single policy are an open decision.

## Support

If the Elohim Protocol vision inspired you today, consider supporting the work by sending a coffee to the developer. A contribution creates space, energy, and time for the future exploration of what technology organized around love could look like.

![Buy me a](https://img.shields.io/badge/Buy_me_a-6B6157?style=for-the-badge&labelColor=6B6157) [![Buy me a coffee](https://img.shields.io/badge/-%24_COFFEE-B8664F?style=for-the-badge)](https://www.buymeacoffee.com/mbd06b)

---

*"Another world is not only possible, she is on her way. On a quiet day, I can hear her breathing."* —Arundhati Roy
