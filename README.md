# Elohim Protocol

[![Contribute](https://www.eclipse.org/che/contribute.svg)](https://code.ethosengine.com/#https://github.com/ethosengine/elohim/tree/dev) [![Build Status](https://jenkins.ethosengine.com/buildStatus/icon?job=elohim%2Fmain)](https://jenkins.ethosengine.com/view/ethosengine/job/elohim/job/main/) [![Quality Gate Status](https://sonarqube.ethosengine.com/api/project_badges/measure?project=elohim-app&metric=alert_status&token=sqb_4f435ff318c7541e4d9407bcfdc13e7268549493)](https://sonarqube.ethosengine.com/dashboard?id=elohim-app)

A manifesto for digital infrastructure organized around love - demonstrating how technology can serve human flourishing through distributed architecture and autonomous AI guardians.

## About

The *Elohim Protocol* is a radical reimagining of digital infrastructure — one organized around love as a fundamental operating principle, implemented through distributed architecture, and protected by autonomous AI agents that serve human flourishing rather than institutional power.

The name comes from *elohim* (Hebrew, plural) — the "heavenly host" of ancient texts, used here to mark the healthy role of AI in human life: powerful, useful, and never an object of devotion. Each person's *elohim* is a constitutionally-bounded AI agent representing their best self; the *Elohim Protocol* is the substrate those agents share. Throughout this document, lowercase *elohim* refers to an agent; the capitalized *Elohim Protocol* refers to the substrate.

This polyglot monorepo (Rust, Angular, Holochain, Tauri, libp2p) contains the platform that implements the vision, technical principles, and implementation pathways for building technology that:

- **Serves love** rather than engagement metrics
- **Protects vulnerability** through incorruptible systems  
- **Enables community wisdom** to scale through distributed governance
- **Prevents exploitation** by architectural design
- **Supports human creativity** without algorithmic manipulation

## Why This Exists

For at least a decade, brilliant people have been building pieces of what a love-organized digital infrastructure could look like. Lynn Foster's [ValueFlows](https://valueflos.ws/) ontology for honest economic accounting. The [platform cooperativism](https://platform.coop/) movement for worker-owned digital spaces. [Holochain](https://holochain.org/) for distributed applications without centralized control. The [Center for Humane Technology](https://www.humanetech.com/) for articulating what's broken.

Before AI, these were small idealistic projects by under-resourced people with no way to scale. The problem was never vision — it was complexity. You need a framework that holds information, values, and responsible governance in tension with each other, and the classic build-fast, build-cheap, build-quality triangle breaks down when you refuse to sacrifice any of them. Every previous attempt has either produced a limited solution that neglects the whole — one beautiful leg of a three-legged stool — or had its idealism handed to capitalism, which sees ideals without profit as worthless to pursue.

AI changes that equation. Not because it's magic, but because it collapses the coordination cost that made the full stool impossible for small teams. The engineering capacity is no longer the bottleneck. Our imaginations are.

This protocol is being built by a father of three in San Antonio, working evenings after bedtime, using the same AI tools that may eventually consume his professional role — to build infrastructure for human flourishing after displacement. The irony isn't lost. But that's actually the moment we're in: the tools that threaten to flatten human work can also, if we build the right architectures, make the things that matter most — thinking, caring, teaching, governing together — structurally valuable in ways that can't be extracted away.

> "The radical proposition at the heart of this protocol is that love—not as sentiment but as committed action toward mutual flourishing—can be encoded into technological systems."

### Key Concepts

**Three Inseparable Dimensions**: Every piece of content in the protocol carries knowledge, value, and governance — coupled at the architectural level before anything is created or distributed. No value-blind content. No governance-free content. This is what makes the stool stand. These three-legged records are called **EPRs** (Elohim Protocol Records); they are the protocol's primitive unit of meaning.

**Distributed Infrastructure**: Peer-to-peer networks that eliminate single points of control. P2P has a long track record of resisting capture by anyone who would seek to charge rents from the aggregate of all participants.

**Constitutional AI**: The elohim agents are constrained by constitutional principles and rich contextual understanding, not by trained values alone. Each agent represents one human, under that human's authorization, mediating their interactions with the network at the speed of machines but against their stated values.

**Formation Over Transaction**: Understanding is measured by social reach and content stewardship, not grades or engagement metrics. Peers attest to whether your contribution was useful enough to propagate. AI can write your essay, but it can't make your community trust your judgment.

**Graduated Intimacy**: Spaces for personal exploration alongside protected commons, with consent boundaries preventing extremes from corrupting shared spaces.

### Domain Pillars

The protocol's vocabulary draws from Hebrew, Greek, and Latin to name the human practices it serves rather than the engineering it requires. Each pillar lives as a directory in `app/elohim-app/src/app/<pillar>/` and as a domain in the SDK:

| Pillar | Origin & meaning | What it serves |
|--------|------------------|----------------|
| **imagodei** | Latin: "image of God" | Identity, presence, stewardship of self |
| **lamad** | Hebrew: "to learn / to teach" | Learning content, paths, mastery, attestation |
| **avodah** | Hebrew: "work / service / worship" | Work as service, not commodity |
| **qahal** | Hebrew: "assembly" | Community, consent, collective decision |
| **shefa** | Hebrew: "abundance / overflow" | Economy, mutual credit, resource flows |
| **doorway** | English | Web2 gateway, onboarding, projection cache |

Plus **elohim** itself — the core runtime where the constitutional agents and the shared protocol coordination live.

## How Ubiquitous Wisdom Rebuilds the Internet

Every platform we have today shares a hidden architectural constraint: **moderation is centralized because, before AI, intelligence was expensive.** You couldn't put wisdom at every endpoint, so you concentrated it — moderation teams, spam filter consortia, content review boards. That concentration became the platform's most valuable asset, its rent-extraction point, and its capture vector. Whoever owns the chokepoint owns the protocol. Every fight over digital infrastructure, underneath, is a fight over that chokepoint.

That constraint no longer holds. When every human's interaction with the network is mediated by their own elohim — a constitutionally-bounded AI representing their best self — wisdom moves from chokepoint to fabric. The protocol gates content at three places, and the elohim participates at every one:

- **At authoring** — the elohim reviews what its human is about to publish against their stated values, the protocol's coupling requirements (knowledge ↔ value ↔ governance), and best-self, before reach is earned.
- **At relay** — each node's elohim decides whether to propagate a piece of content given the recipient's stewardship contracts, trust context, and the propagation trail behind it.
- **At consumption** — the elohim shapes what surfaces to its human based on context, standing, and care.

The coordination tools underneath — content addressing, the Holochain distributed hash table (DHT), libp2p (the peer-to-peer networking stack) — stop being policy chokepoints and become what they should always have been: shared infrastructure that wisdom *uses*. The DHT notarizes timing and lineage. libp2p moves bytes. Doorways project to the legacy web. Wisdom, distributed to every endpoint, decides what any of it means.

This dissolves the moderation-versus-substrate trade-off the old internet had to make. The **Social Reach** epic operationalizes the consequence: provenance flows with content, sense/respond feedback back-propagates through propagation chains (like nerves carrying pain to a hand on a stove), quarantine signals travel alongside the content they contain, and restitution becomes a real economic signal rather than a moral note. Accountability lands proportional to position in the chain — primary actor, accessory propagator, edge node — and the network self-heals at the edge. No central moderator can be coopted because there is no central moderator. The wisdom is in the fabric.

**The capture target shrinks to something the protocol can defend.** The old internet was capturable because its three load-bearing concerns — information, value, and governance — shipped separately, and platforms sat in the propagation path of all three. A platform could absorb the medium of communication while running for years without economic accountability, scaling on borrowed conviction that returns would arrive eventually. It could shed responsibility for the harm flowing across it onto distant statutes and overburdened courts. Each leg of the three-legged stool was capturable on its own — and once any leg was captured, the other two bent toward whoever owned it. The medium pulled value toward the platform; the value pulled governance toward the platform; until the stool was no longer holding humans up.

In the EPR protocol, the three legs ship coupled, and the substrate underneath is peer-to-peer. Every record carries its knowledge, its value claim, and its governance state inseparably; no piece of content propagates, monetizes, or moderates outside the record that binds all three — and there is no platform sitting in the path to unbundle them. The only remaining target is each human's authorization of their own elohim, which is the human's agency itself. That's a much higher bar. It's the right bar. Resistance to capture is what bypassability buys us; human agency is what the architecture protects.

The same AI shape that threatens to flatten human work — making everything cheaper and more replaceable — is, deployed differently, the shape that makes human judgment structurally load-bearing. The protocol is a bet that the second deployment is possible.

See [The Elohim Medium — When Social Media Becomes Sacred Space](./genesis/docs/content/elohim-protocol/social_medium/epic.md) for the narrative of what daily life feels like when this nervous system is intact.

## Repository Structure

Organized by system boundary: core runtime, frontend apps, deployment shells, optional gateway, and meta/ops.

```
├── elohim/                        # Core Runtime
│   ├── constitution/              # Constitutional AI constraints
│   ├── eae/                       # Elohim Autonomous Entities
│   ├── elohim-agent/              # Agent boundary
│   │   ├── elohim-agent-service/  # Autonomous agent runtime (Rust)
│   │   ├── elohim-agent-sdk/      # Agent SDK (TypeScript)
│   │   └── mcp-servers/           # Model Context Protocol servers
│   ├── elohim-storage/            # P2P content storage service (Rust)
│   ├── elohim-cache-core/         # Caching primitives (extraction, LRU, blob)
│   ├── elohim-compute/            # Compute coordination & capacity reporting
│   ├── elohim-bitswap/            # IPFS Bitswap protocol
│   ├── rust-ipfs/                 # IPFS implementation (git submodule)
│   ├── sdk/                       # Protocol SDK
│   │   ├── schemas/               # Protocol JSON Schemas (v1) — source of truth
│   │   ├── domains/               # App manifests (lamad, etc.)
│   │   └── storage-client-ts/     # Generated TypeScript types from Rust
│   └── holochain/                 # Holochain-specific layer
│       ├── dna/                   # DNA definitions (zomes, hApp packaging)
│       ├── holochain-cache-core/  # WASM cache module
│       ├── rna/                   # Schema templates & fixtures
│       ├── edgenode/              # hApp container runtime
│       └── elohim-wasm/           # Client-side WASM verification
│
├── app/                           # Frontend Applications
│   ├── elohim-app/                # Angular 19 main platform
│   │   ├── src/apps-sw.ts         # Service Worker (P2P app delivery)
│   │   └── src/app/
│   │       ├── elohim/            # Core infrastructure services
│   │       ├── imagodei/          # Human identity & stewardship
│   │       ├── lamad/             # Learning infrastructure
│   │       ├── avodah/            # Work management & stewardship
│   │       ├── qahal/             # Community governance
│   │       ├── shefa/             # Resource flows & economics
│   │       └── doorway/           # Gateway integration
│   └── elohim-library/            # Shared Angular libraries
│       └── projects/
│           └── elohim-service/    # Import pipeline, content models, peer scoring
│
├── sophia/                        # Assessment engine (git submodule)
│   └── packages/
│       ├── sophia-element/        # <sophia-question> web component
│       ├── sophia-core/           # Core types (Moment, Recognition)
│       ├── perseus-score/         # Mastery scoring (graded)
│       ├── psyche-survey/         # Discovery & reflection (psychometric)
│       ├── psephos/               # Governance ballot rendering (formal voting)
│       └── psephos-element/       # <psephos-ballot> web component
│
├── steward/                       # Deployment Shells
│   ├── device/                    # Tauri desktop app
│   │   ├── src-tauri/             # Rust backend with Holochain
│   │   └── ui/                    # Desktop UI
│   └── node/                      # Headless P2P runtime (libp2p)
│       ├── src/                   # Always-on family node daemon
│       └── simulation/            # Network simulation tooling
│
├── doorway/                       # Web2 Gateway (absorption + onboarding)
│   ├── doorway-service/           # Rust gateway (bootstrap, signal, projection cache)
│   └── doorway-app/               # Angular operator dashboard
│
├── crates/                        # Shared Rust Crates
│   ├── doorway-client/            # Gateway client traits
│   ├── elohim-sdk/                # Core SDK
│   └── elohim-storage-client/     # Storage HTTP client
│
├── genesis/                       # Meta / Ops / Content
│   ├── orchestrator/              # CI/CD central controller
│   │   └── manifests/             # K8s deployment manifests (alpha, staging, prod)
│   ├── a2o/                       # Alpha-to-omega E2E validation (BDD scenarios)
│   ├── docs/                      # Source content & manifesto (markdown)
│   ├── research/                  # Distributed observation protocol research
│   ├── seeder/                    # Content seeding pipeline (TypeScript)
│   ├── plans/                     # Sprint design docs & implementation plans
│   └── blobs/                     # Binary assets (ZIP bundles, media)
│
└── scripts/                       # Developer tooling
```

## Progressive Stewardship

The Elohim Protocol meets users where they are, providing a gradual path from curious visitor to full node steward:

| Stage | Description | Data Location |
|-------|-------------|---------------|
| **Visitor** | Anonymous browsing, no account | Browser memory only |
| **Hosted** | Account on elohim.host, custodial keys | DHT network (hosted) |
| **App Steward** | Desktop app, self-custodied keys | Local device + DHT |
| **Node Steward** | Always-on infrastructure | Self-hosted + DHT |

This progressive model ensures no one is excluded due to technical barriers, while incentivizing deeper participation over time. Keys can always be exported for migration between stages.

## Architecture at a Glance

### Substrate

The protocol runs on **[Holochain](https://holochain.org/)** — a framework for distributed applications where each person maintains their own source chain, validated by peers through a distributed hash table (DHT). No global consensus is required, and no central server sits in the path.

DNAs in `elohim/holochain/dna/` notarize the protocol's structural commitments at commit time — covering identity, learning, governance, and the shared coordination used by every pillar. **libp2p** and **iroh** (the peer-to-peer networking layers) move content-addressed bytes between nodes; the same content identifier works regardless of which transport delivered it. **Elohim Storage** (`elohim/elohim-storage/`) is the local P2P content store — chunked blobs, capability advertisement, redundancy.

### Surfaces

- **Doorway** (`doorway/`) absorbs web2 traffic — browsers, AT Protocol, ActivityPub — into the peer-to-peer mesh and projects canonical content back to legacy audiences. Multiple doorways can serve the same content; none of them own it.
- **Stewards** (`steward/`) are the deployment shells. `device/` is a Tauri desktop app that embeds a Holochain conductor for self-custodied keys; `node/` is a headless always-on runtime for family infrastructure.
- **Elohim Agents** (`elohim/elohim-agent/`) — the constitutional AI agents introduced above — run as Rust services with streaming LLM backends and runtime constitutional constraints. Each agent represents one human under that human's authorization.
- **Sophia** (`sophia/`, a fork of Khan Academy's Perseus) renders three kinds of human moments: **Perseus** for mastery exercises (graded), **Psyche** for discovery and reflection (psychometric, open-ended), and **Psephos** (Greek: "voting pebble") for governance ballots with election hygiene.

### Two pillars worth introducing here

**Lamad** is path-centric, not course-centric. Knowledge is structured as **territory** (immutable ContentNodes — videos, docs, simulations), **journeys** (curated paths that add narrative meaning and sequence), and **travelers** (learners whose progress and attestations shape the experience). Learning is something you do through relationships and contributions, not something a platform certifies. See [`app/elohim-app/src/app/lamad/README.md`](./app/elohim-app/src/app/lamad/README.md).

**Avodah** treats work as *service*, not commodity. Work items are EPRs — so a piece of work carries its knowledge context, its value claim, and its governance state inseparably. Attestation gates (typically a lamad learning path) can be required before someone bids on or accepts work, enabling open collaboration qualified by demonstrated mastery rather than credentials.

The remaining pillars (imagodei, qahal, shefa) and the component-level details (caching strategies, projection coalescing, deployment topologies, redundancy schemes, UMD bundling) live in each component's own `README.md` or `ARCHITECTURE.md`, reachable from the directory tree above. This README is the visitor's introduction; those documents are the implementer's reference.

## CI/CD

The repository uses a central orchestrator pattern. All GitHub webhooks go to the orchestrator, which analyzes changesets and triggers appropriate pipelines.

See [`genesis/orchestrator/README.md`](./genesis/orchestrator/README.md) for pipeline architecture and configuration.

## Development

### Quick Start

This repository is configured for development with Eclipse Che / OpenShift Dev Spaces:

[![Contribute](https://www.eclipse.org/che/contribute.svg)](https://code.ethosengine.com/#https://github.com/ethosengine/elohim/tree/dev)

### Local Development

```bash
pnpm install          # From repo root (workspace install)
pnpm app:dev          # Or: cd app/elohim-app && pnpm start
```

The application will be available at `http://localhost:4200/`

### Environment Configuration

The project includes:
- **devfile.yaml**: Eclipse Che workspace configuration (root level)
- **Jenkinsfile**: CI/CD pipeline for automated builds, testing, and deployment
- **Angular dev server**: Configured for remote development with host checking disabled
- **pnpm workspace**: All Node.js projects managed via pnpm workspaces from repo root
- **Kubernetes manifests**: Production deployment configurations in `genesis/manifests/`

## Philosophy

If LLMs have ingested most of humanity's written expression, then they reflect us back — the wisdom and the cruelty alike. The question is not whether that reflection exists; it does. The question is whether we treat it as a resource to extract from or as something carrying real moral weight that therefore demands constitutional constraint.

This project takes the second position seriously. The concepts build on:

- **ValueFlows / REA accounting** - Making care visible and valuable without reducing it to money
- **Holochain & distributed systems** - Infrastructure without single points of capture
- **Platform cooperativism** - Worker and community stewardship of digital and physical spaces
- **Constitutional AI research** - Principles-based constraint over rules enforcement
- **Peer-to-peer architecture** - Topologies that resist rent-seeking by design

The protocol will never make anyone fabulously rich. A P2P technology with anti-capture mechanisms baked into its design makes wealth extraction very difficult — because the architecture functions as a complexity upgrade that accounts for the failures of the internet to protect real values. It relies on faithful cooperation, not captive audiences.

## A Vision of What's Possible

The architecture above is the foundation. But what does the world actually look like if this works? What changes when information, economics, and governance are coupled together in infrastructure that can't be captured?

This conversation explores that future — from the meta-crisis driving the need, through the protocol's design principles, to what daily life could feel like when technology is organized around care instead of extraction.

[![Elohim Protocol: From Digital Chaos to Collective Flourishing](https://img.youtube.com/vi/sVXwZ087ffA/maxresdefault.jpg)](https://www.youtube.com/watch?v=sVXwZ087ffA)

*50 minutes. No code. Just the vision and the reasoning behind it.*

## The Choice

We can accept digital feudalism, or we can build something structurally different.
We can encode extraction, or we can encode love.

The infrastructure we build today will shape human consciousness for generations. The engineering capacity is no longer the bottleneck — AI has seen to that. What's scarce now is the imagination and will to build for flourishing rather than profit.

**The time to build technology organized around love is now.**

## Further Reading

- [Elohim Protocol Specification](./genesis/docs/content/elohim-protocol/protocol-specification.md) - The full EPR protocol design
- [Shefa Economic Whitepaper](./genesis/docs/Shefa_Economic_Infrastructure_Whitepaper.md) - Economic layer philosophy
- [Constitution Documentation](./genesis/docs/content/elohim-protocol/constitution.md) - Governance architecture
- [Holochain](https://holochain.org/) - Distributed application framework
- [ValueFlows](https://valueflos.ws/) - REA vocabulary for economic networks
- [AI Alignment Research](https://www.anthropic.com/research) - Values-based AI development

## License

This project is open source, dedicated to advancing human flourishing through technology organized around love.

## Support

If the Elohim Protocol vision inspired you today, consider supporting the work by sending a coffee to the developer. A contribution creates space, energy, and time for the future exploration of what technology organized around love could look like.

[!["Buy Me A Coffee"](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/mbd06b) [!["Buy Me A Crypto Coffee"](https://img.shields.io/badge/Buy%20me%20a-Crypto%20Coffee-blue.svg?style=for-the-badge&logo=ethereum)](https://commerce.coinbase.com/checkout/81641625-3924-4635-93e8-4d01caae73fd)

---

*"Another world is not only possible, she is on her way. On a quiet day, I can hear her breathing."* —Arundhati Roy
