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

## Vision

> "The radical proposition at the heart of this protocol is that love—not as sentiment but as committed action toward mutual flourishing—can be encoded into technological systems."

We stand at a crossroads in digital civilization. Current social media architectures, built on surveillance capitalism and engagement optimization, have failed to support human flourishing at scale. Yet we have proof that humans can build high-trust, pro-social systems—Scandinavian democracies demonstrate it's possible.

This manifesto proposes technology that actively defends against corruption while enabling human wisdom to scale through:

### Key Concepts

**Distributed Infrastructure**: Peer-to-peer networks that eliminate single points of control and enable community stewardship

**Graduated Intimacy**: Spaces for personal exploration alongside protected commons, with consent boundaries preventing extremes from corrupting shared spaces

**Love as Technology**: AI agents trained on patterns of human flourishing, cryptographically autonomous and incorruptible by institutional power

**Transparency as Immune System**: Open governance that makes manipulation visible while preserving privacy and dignity

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
│       └── psyche-survey/         # Discovery & reflection (psychometric)
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

### Sophia (Assessment Engine)

Person-centered assessment rendering infrastructure, transforming Khan Academy's Perseus with three assessment modes:
- **Mastery**: Graded exercises (correct/incorrect)
- **Discovery**: Resonance mapping to reveal affinities (psychometric aggregation)
- **Reflection**: Open-ended capture without grading

Key abstractions: **Moment** (unit of content, not just "question") and **Recognition** (what learner demonstrated, not just "answer"). See [`sophia/README.md`](./sophia/README.md).

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

This isn't just a website—it's a manifesto for a new kind of technological civilization that takes seriously both human vulnerability and human potential. The concepts presented build on:

- **Scandinavian social democratic models** - Proof that high-trust societies work at scale
- **Indigenous wisdom traditions** - Restorative justice and collective stewardship
- **Distributed systems research** - Holochain, IPFS, peer-to-peer architecture
- **AI alignment research** - Values-based training over rules enforcement
- **Community governance** - Cooperative economics and local autonomy

## The Choice

We can accept digital feudalism, or we can create digital democracy.
We can encode exploitation, or we can encode love.

The infrastructure we build today will shape human consciousness for generations. As AI development accelerates, the technical hurdles of building distributed, love-aligned systems are dropping rapidly. 

**The time to build technology organized around love is now.**

## Further Reading

- [Holochain](https://holochain.org/) - Distributed application framework
- [Shefa Economic Whitepaper](./genesis/docs/Shefa_Economic_Infrastructure_Whitepaper.md) - Economic layer philosophy
- [Constitution Documentation](./genesis/docs/content/elohim-protocol/constitution.md) - Governance architecture
- [Scandinavian Social Democracy](https://en.wikipedia.org/wiki/Nordic_model) - Proven high-trust governance
- [AI Alignment Research](https://www.anthropic.com/research) - Values-based AI development

## License

This project is open source, dedicated to advancing human flourishing through technology organized around love.

---

*"Another world is not only possible, she is on her way. On a quiet day, I can hear her breathing."* —Arundhati Roy
