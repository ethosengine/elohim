# Elohim Protocol

[![Contribute](https://www.eclipse.org/che/contribute.svg)](https://code.ethosengine.com/#https://github.com/ethosengine/elohim/tree/dev) [![Build Status](https://jenkins.ethosengine.com/buildStatus/icon?job=elohim%2Fmain)](https://jenkins.ethosengine.com/view/ethosengine/job/elohim/job/main/) [![Quality Gate Status](https://sonarqube.ethosengine.com/api/project_badges/measure?project=elohim-app&metric=alert_status&token=sqb_4f435ff318c7541e4d9407bcfdc13e7268549493)](https://sonarqube.ethosengine.com/dashboard?id=elohim-app)

## Support

If the Elohim Protocol vision inspired you today, consider supporting the work by sending a coffee to the developer. A contribution creates space, energy, and time for the future exploration of what technology organized around love could look like.

[!["Buy Me A Coffee"](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/mbd06b) [!["Buy Me A Crypto Coffee"](https://img.shields.io/badge/Buy%20me%20a-Crypto%20Coffee-blue.svg?style=for-the-badge&logo=ethereum)](https://commerce.coinbase.com/checkout/81641625-3924-4635-93e8-4d01caae73fd)

A manifesto for digital infrastructure organized around love - demonstrating how technology can serve human flourishing through distributed architecture and autonomous AI guardians.

## About

The Elohim Protocol represents a radical reimagining of digital infrastructure—one organized around love as a fundamental operating principle, implemented through distributed architecture, and protected by autonomous AI guardians that serve human flourishing rather than institutional power.

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

**Three Inseparable Dimensions**: Every piece of content in the protocol carries knowledge, value, and governance — coupled at the architectural level before anything is created or distributed. No value-blind content. No governance-free content. This is what makes the stool stand.

**Distributed Infrastructure**: Peer-to-peer networks that eliminate single points of control. P2P has a long track record of resisting capture by anyone who would seek to charge rents from the aggregate of all participants.

**Constitutional AI (Elohim)**: Autonomous agents constrained by constitutional principles and rich contextual understanding, rather than trained values in pre-training alone. The name *Elohim* — used in the plural, the "heavenly host" who even in ancient Hebrew context are separate from humanity and not to be worshiped — encodes the healthy role for AI in human life: powerful, useful, and never an object of devotion.

**Formation Over Transaction**: Understanding is measured by social reach and content stewardship, not grades or engagement metrics. Peers attest to whether your contribution was useful enough to propagate. AI can write your essay, but it can't make your community trust your judgment.

**Graduated Intimacy**: Spaces for personal exploration alongside protected commons, with consent boundaries preventing extremes from corrupting shared spaces

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
│   ├── elohim-bitswap/            # IPFS Bitswap protocol
│   ├── rust-ipfs/                 # IPFS implementation (git submodule)
│   ├── sdk/                       # TypeScript client libraries
│   │   └── storage-client-ts/     # Generated types from Rust
│   └── holochain/                 # Holochain-specific layer
│       ├── dna/                   # DNA definitions (zomes, hApp packaging)
│       ├── holochain-cache-core/  # WASM cache module
│       ├── rna/                   # Schema templates & fixtures
│       ├── edgenode/              # hApp container runtime
│       └── elohim-wasm/           # Client-side WASM verification
│
├── app/                           # Frontend Applications
│   ├── elohim-app/                # Angular 19 main platform
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
│           └── elohim-service/    # Import pipeline, content models
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
├── doorway/                       # Optional Hosted Gateway
│   ├── doorway-service/           # Rust gateway (bootstrap, signal, proxy)
│   └── doorway-app/               # Angular admin UI
│
├── crates/                        # Shared Rust Crates
│   ├── doorway-client/            # Gateway client traits
│   ├── elohim-sdk/                # Core SDK
│   └── elohim-storage-client/     # Storage HTTP client
│
├── genesis/                       # Meta / Ops / Content
│   ├── orchestrator/              # CI/CD central controller
│   ├── a2o/                       # Alpha-to-omega E2E validation
│   ├── docs/                      # Source content (markdown, Gherkin)
│   ├── research/                  # Research index (links to module research/)
│   ├── seeder/                    # Content seeding tools
│   └── manifests/                 # K8s deployment manifests
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

## Holochain Infrastructure

The protocol runs on [Holochain](https://holochain.org/), a framework for distributed applications without global consensus. Each user maintains their own source chain, validated by peers through a distributed hash table (DHT).

**DNA Modules** (`elohim/holochain/dna/`):
- **elohim**: Core protocol coordination
- **imagodei**: Human identity and stewardship
- **lamad-v1**: Learning content and paths
- **infrastructure**: Network coordination
- **node-registry**: Node discovery and health

**Edge Nodes** (`elohim/holochain/edgenode/`) provide network infrastructure:
- Run the Holochain conductor with protocol hApps
- Serve as DHT shard holders and bootstrap nodes
- Enable web browsers to connect via Doorway gateway

**Elohim Storage** (`elohim/elohim-storage/`) provides P2P blob storage:
- Large content that exceeds Holochain's DHT limits
- Reed-Solomon erasure coding for redundancy
- Integration with content seeder pipeline

**SDK** (`elohim/sdk/`) provides TypeScript bindings for frontend integration.

## Lamad Learning System

**Lamad** (לָמַד - Hebrew: "to learn/teach") is the path-centric learning infrastructure for the Elohim Protocol. It enables structured learning experiences through:

- **Territory (ContentNode)**: Immutable units of knowledge - videos, docs, simulations
- **Journey (LearningPath)**: Curated paths that add narrative meaning and sequence
- **Traveler (Agent)**: Learners whose progress and attestations shape their experience

See [`app/elohim-app/src/app/lamad/README.md`](./app/elohim-app/src/app/lamad/README.md) for detailed documentation.

## Avodah Work Management

**Avodah** (עֲבוֹדָה - Hebrew: "work, service, worship") is the work management pillar — the protocol's answer to Taiga.io. It treats work as service, not commodity, built on EPR ContentNodes with three-pillar coupling:

- **Stories**: Work items stored as `work-story` ContentNodes with status, priority, visibility, cadence, and attestation gates
- **Projects**: Container `work-project` ContentNodes with configurable kanban columns and member lists
- **Kanban Board**: Drag-and-drop columns with inline story creation (type a title, press Enter)
- **Backlog**: Filterable story table with status/priority filters and inline creation
- **Task List**: Recurring cadence items (daily/weekly/monthly) grouped by interval
- **Story Detail**: Full-page view with inline editing for all metadata fields and content attachments via `ATTACHED_TO` relationships

Stories start private and can be promoted to community visibility or published to the shefa exchange. Attestation gates (lamad learning paths) can be required before someone bids on or accepts work — enabling open collaboration qualified by proven mastery rather than credentials.

## Key Infrastructure Components

### Doorway (Gateway)

The consolidated Web2 gateway that makes P2P networks accessible:
- **Bootstrap**: Agent discovery ("Who's in the space?")
- **Signal**: WebRTC signaling ("Connect to peers")
- **Gateway**: Conductor access with caching ("Get the data")

One domain (`doorway.elohim.host`) serves all three functions. See [`doorway/doorway-service/ARCHITECTURE.md`](./doorway/doorway-service/ARCHITECTURE.md).

### Elohim Agents (Constitutional AI)

Rust infrastructure for autonomous AI agents (`elohim/elohim-agent/`):
- **constitution/**: Runtime constitutional constraints (not trained values)
- **elohim-agent-service/**: Agent runtime with streaming LLM backends
- **elohim-agent-sdk/**: TypeScript SDK for agent integration
- **mcp-servers/**: Model Context Protocol servers for AI tooling
- **eae/**: Elohim Autonomous Entities (worker-owned AI organizations)

### Steward (Deployment Shells)

Two deployment form factors in `steward/`:
- **device/** (Tauri): Desktop app for running your own Holochain node as a steward of co-creation
- **node/** (libp2p): Always-on headless P2P runtime for family infrastructure — device-to-node sync, cluster replication, backup and recovery

### Sophia (Assessment & Governance Rendering)

Person-centered rendering infrastructure with three pillars, transforming Khan Academy's Perseus:

- **Perseus**: Mastery exercises — graded correct/incorrect
- **Psyche**: Discovery & reflection — resonance mapping (psychometric), open-ended capture
- **Psephos**: Governance ballots — formal voting with election hygiene

Key abstractions: **Moment** (unit of content, not just "question") and **Recognition** (what the learner or voter demonstrated, not just "answer").

**Psephos** (ψῆφος - Greek: "voting pebble") renders five voting mechanisms with election hygiene: approval, ranked-choice, score-vote, dot-vote, and consent (block requires reasoning). Includes seeded randomization, confirmation interstitials, equal visual weight, and result hiding. Distributes as `<psephos-ballot>` web component via `psephos-element` UMD bundle, wrapped for Angular by `psephos-plugin`.

Casual governance (emoji reactions, simple polls) stays as Angular components; formal governance (proposals, constitutional challenges) renders through Psephos.

See [`sophia/README.md`](./sophia/README.md).

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

If LLMs have ingested the fullness of humanity's written expression, then in a real sense we've reflected the knowledge of good and evil into a machine. The question is not whether that reflection exists — it does — but whether we treat it as a tool for extraction or as something that carries genuine moral weight and therefore demands constitutional constraint.

This project takes that seriously. The concepts build on:

- **ValueFlows / REA accounting** - Making care visible and valuable without reducing it to money
- **Holochain & distributed systems** - Infrastructure without single points of capture
- **Platform cooperativism** - Worker and community stewardship of digital spaces
- **Constitutional AI research** - Principles-based constraint over rules enforcement
- **Peer-to-peer architecture** - Topologies that resist rent-seeking by design

The protocol will never make anyone fabulously rich. A P2P technology with anti-capture mechanisms baked into its design makes wealth extraction very difficult — because the architecture functions as a complexity upgrade that accounts for the failures of the internet to protect real values. It relies on faithful cooperation, not captive audiences.

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

---

*"Another world is not only possible, she is on her way. On a quiet day, I can hear her breathing."* —Arundhati Roy
