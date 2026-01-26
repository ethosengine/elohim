# Elohim Protocol

[![Contribute](https://www.eclipse.org/che/contribute.svg)](https://code.ethosengine.com/#https://github.com/ethosengine/elohim) [![Build Status](https://jenkins.ethosengine.com/buildStatus/icon?job=elohim%2Fmain)](https://jenkins.ethosengine.com/view/ethosengine/job/elohim/job/main/) [![Quality Gate Status](https://sonarqube.ethosengine.com/api/project_badges/measure?project=elohim-app&metric=alert_status&token=sqb_4f435ff318c7541e4d9407bcfdc13e7268549493)](https://sonarqube.ethosengine.com/dashboard?id=elohim-app)

## Support

If the Elohim Protocol vision inspired you today, consider supporting the work by sending a coffee to the developer. A contribution creates space, energy, and time for the future exploration of what technology organized around love could look like.

[!["Buy Me A Coffee"](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/mbd06b) [!["Buy Me A Crypto Coffee"](https://img.shields.io/badge/Buy%20me%20a-Crypto%20Coffee-blue.svg?style=for-the-badge&logo=ethereum)](https://commerce.coinbase.com/checkout/81641625-3924-4635-93e8-4d01caae73fd)

A manifesto for digital infrastructure organized around love - demonstrating how technology can serve human flourishing through distributed architecture and autonomous AI guardians.

## About

The Elohim Protocol represents a radical reimagining of digital infrastructure—one organized around love as a fundamental operating principle, implemented through distributed architecture, and protected by autonomous AI guardians that serve human flourishing rather than institutional power.

This repository contains an Angular application that presents the vision, technical principles, and implementation pathways for building technology that:

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

**Distributed Infrastructure**: Peer-to-peer networks that eliminate single points of control and enable community sovereignty

**Graduated Intimacy**: Spaces for personal exploration alongside protected commons, with consent boundaries preventing extremes from corrupting shared spaces

**Love as Technology**: AI agents trained on patterns of human flourishing, cryptographically autonomous and incorruptible by institutional power

**Transparency as Immune System**: Open governance that makes manipulation visible while preserving privacy and dignity

## Repository Structure

```
├── devfile.yaml              # Eclipse Che workspace configuration
├── Jenkinsfile               # CI/CD pipeline definition
├── VERSION                   # Semantic versioning (1.0.0)
│
├── genesis/                  # Meta-infrastructure: source → seed → validate
│   ├── Jenkinsfile           # Seed + validate pipeline
│   ├── docs/                 # Raw source documentation
│   │   └── content/          # Markdown, Gherkin source files
│   ├── data/                 # Structured seed data
│   │   └── lamad/            # Learning content JSON
│   └── seeder/               # Holochain seeding tools
│
├── elohim-app/               # Angular application (Main Platform)
│   └── src/app/
│       ├── components/       # Landing page components
│       ├── elohim/           # Core infrastructure services
│       │   ├── models/       # Holochain connection, protocol types
│       │   ├── services/     # Holochain client, Kuzu graph DB
│       │   └── components/   # Navigator, settings tray
│       ├── imagodei/         # Human identity & sovereignty
│       │   ├── models/       # Sovereignty stages, data residency
│       │   └── services/     # Session management, sovereignty state
│       ├── lamad/            # Learning infrastructure module
│       │   ├── models/       # ContentNode, LearningPath, mastery
│       │   ├── services/     # Data loading, progress tracking
│       │   ├── components/   # Path navigator, content viewer
│       │   └── renderers/    # Markdown, video, assessment
│       ├── qahal/            # Community governance (planned)
│       └── shefa/            # Resource flows (planned)
│
├── elohim-library/           # Shared Libraries & Services
│   └── projects/
│       ├── elohim-service/   # Import pipeline, content models
│       └── lamad-ui/         # UI Pattern Library
│
├── holochain/                # Holochain Edge Node Infrastructure
│   ├── doorway/              # Gateway service (auth, routing, caching)
│   ├── dna/                  # DNA definitions and zomes
│   ├── manifests/            # K8s deployments for Edge Nodes
│   └── Jenkinsfile           # CI/CD for Holochain components
│
└── manifests/                # Kubernetes deployment manifests
    ├── *-deployment.yaml     # Environment-specific deployments
    ├── ingress.yaml          # Ingress configuration
    └── service.yaml          # Service definitions
```

## Progressive Sovereignty

The Elohim Protocol meets users where they are, providing a gradual path from curious visitor to fully sovereign node operator:

| Stage | Description | Data Location |
|-------|-------------|---------------|
| **Visitor** | Anonymous browsing, no account | Browser memory only |
| **Hosted User** | Account on elohim.host, custodial keys | DHT network (hosted) |
| **App User** | Desktop app, self-sovereign keys | Local device + DHT |
| **Node Operator** | Always-on infrastructure | Self-hosted + DHT |

This progressive model ensures no one is excluded due to technical barriers, while incentivizing deeper participation over time. Keys can always be exported for migration between stages.

## Holochain Infrastructure

The protocol runs on [Holochain](https://holochain.org/), a framework for distributed applications without global consensus. Each user maintains their own source chain, validated by peers through a distributed hash table (DHT).

**Edge Nodes** provide the network infrastructure:
- Run the Holochain conductor with the Lamad hApp
- Serve as DHT shard holders and bootstrap nodes
- Enable web browsers to connect via authenticated WebSocket proxy

See [`holochain/claude.md`](./holochain/claude.md) for Edge Node setup and configuration.

## Lamad Learning System

**Lamad** (לָמַד - Hebrew: "to learn/teach") is the path-centric learning infrastructure for the Elohim Protocol. It enables structured learning experiences through:

- **Territory (ContentNode)**: Immutable units of knowledge - videos, docs, simulations
- **Journey (LearningPath)**: Curated paths that add narrative meaning and sequence
- **Traveler (Agent)**: Sovereign agents whose progress and attestations shape their experience

See [`elohim-app/src/app/lamad/README.md`](./elohim-app/src/app/lamad/README.md) for detailed documentation.

## CI/CD

See [`orchestrator/README.md`](./orchestrator/README.md) for pipeline architecture and configuration.

## Development

### Quick Start

This repository is configured for development with Eclipse Che / OpenShift Dev Spaces:

[![Contribute](https://www.eclipse.org/che/contribute.svg)](https://code.ethosengine.com/#https://github.com/ethosengine/elohim)

### Local Development

```bash
cd elohim-app
npm install
npm start
```

The application will be available at `http://localhost:4200/`

### Environment Configuration

The project includes:
- **devfile.yaml**: Eclipse Che workspace configuration (root level)
- **Jenkinsfile**: CI/CD pipeline for automated builds, testing, and deployment
- **Angular dev server**: Configured for remote development with host checking disabled
- **NPM environment**: Optimized for containerized development with `/tmp` directories
- **Kubernetes manifests**: Production deployment configurations in `manifests/`

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
- [Digital Infrastructure for Human Flourishing Manifesto](./elohim-app/src/app/components/) - Full vision document
- [Scandinavian Social Democracy](https://en.wikipedia.org/wiki/Nordic_model) - Proven high-trust governance
- [AI Alignment Research](https://www.anthropic.com/research) - Values-based AI development

## License

This project is open source, dedicated to advancing human flourishing through technology organized around love.

---

*"Another world is not only possible, she is on her way. On a quiet day, I can hear her breathing."* —Arundhati Roy
