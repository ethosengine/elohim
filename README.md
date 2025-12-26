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
│   ├── admin-proxy/          # Authenticated WebSocket proxy
│   ├── manifests/            # K8s deployments for Edge Nodes
│   └── Jenkinsfile           # CI/CD for Holochain components
│
├── data/                     # Content and documentation
│   ├── content/              # Learning content by domain
│   └── humans/               # Human-readable documentation
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

## Jenkins Mono-Repo CI/CD Pipeline

This monorepo uses a three-pipeline Jenkins architecture to manage builds efficiently while maintaining clean separation of concerns. The pipelines coordinate through intelligent changesets and artifact sharing.

### Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ GITHUB WEBHOOK (on monorepo push)                           │
│ Triggers all 3 Jenkins multibranch jobs                     │
└─────────────────────────────────────────────────────────────┘
        ↓
    ┌───┴────┬──────────┬───────────┐
    │        │          │           │
    ▼        ▼          ▼           │
┌────────┐┌──────────┐┌─────────┐  │
│Elohim  ││Holochain││Steward  │  │
│App     ││Pipeline ││Pipeline │  │
│Job     ││Job      ││Job      │  │
└────────┘└──────────┘└─────────┘  │
    │        │          │          │
    │    ┌───┴──────────┘          │
    │    │                         │
    │    │  Artifact Sharing       │
    │    │  (hApp from holochain)  │
    │    │                         │
    │    └─────────┐               │
    │              │               │
    ▼              ▼               │
   Steward        (Builds with     │
   Fetches        fetched artifact)│
   hApp           30 sec vs 40 min │
                                  │
ROOT JOB ORCHESTRATOR:             │
┌──────────────────────────────────┘
│ 🚀 Detects changesets
│ 📊 Shows build matrix (what will run)
│ 📡 Updates description with status
│ ℹ️  Provides visibility into decisions
└──────────────────────────────────

Each pipeline respects its own when{} conditions:
    ▼
   Docker Images → Harbor Registry
   (Production Deployment)
```

### Three-Pipeline Design

#### 1. **Elohim App Pipeline** (`/projects/elohim/Jenkinsfile`)
- **Triggers on:** `elohim-app/**`, `elohim-library/**`, `Jenkinsfile`, `VERSION`
- **Builds:** Angular web application + UI Playground
- **Deploys to:** Alpha (dev branches), Staging, Production (main)
- **Artifacts:** Docker images in Harbor registry
- **Key Features:**
  - Automatic builds only when app files change
  - Environment-specific builds (alpha/staging/prod)
  - E2E testing against deployed environments
  - SonarQube code quality analysis

#### 2. **Holochain Pipeline** (`/projects/elohim/holochain/Jenkinsfile`)
- **Triggers on:** `holochain/**`
- **Builds:** DNA, hApp bundle, Gateway service
- **Deploys to:** Dev edge node (dev/branches), Production (main)
- **Artifacts:**
  - `elohim.happ` (archived for steward to fetch)
  - Docker images (edgenode, doorway, happ-installer)
- **Key Features:**
  - Efficient Rust/WASM compilation with Nix caching
  - Automatic artifact archival for downstream consumption
  - Edge node deployment
  - **Dev-only: Automatic seeding** when DNA or seed data changes

**Sub-stage: 🔧 Seed Dev Database**
- Runs AFTER deployment, only when DNA is rebuilt
- Populates holochain-dev with test/prototype data
- Triggered by: `holochain/dna/**` or `holochain/seeder/**` changes (not other holochain modifications)
- Skipped on production (main branch) - dev branches only
- Useful for rapid prototyping iteration with fresh data

#### 3. **Steward Pipeline** (`/projects/elohim/steward/Jenkinsfile`)
- **Triggers on:** `steward/**`, `holochain/dna/**`, `elohim-app/src/**`, `VERSION`
- **Builds:** Tauri desktop app (AppImage, .deb)
- **Publishes:** GitHub Releases (main branch only, manual)
- **Key Features:**
  - Smart artifact fetching from holochain pipeline
  - Three-tier fallback: artifact fetch → local build → error
  - **99% faster** when artifacts available (30 sec vs 40 min)

### Build Matrix & Changeset Filtering

The root orchestrator detects changes and determines which pipelines to run:

| Changed Files | Pipelines Triggered | Extra Actions |
|---|---|---|
| `elohim-app/**`, `elohim-library/**` | ✅ Elohim App | - |
| `holochain/**` (other files) | ✅ Holochain | - |
| `holochain/dna/**` | ✅ Holochain | 🔧 Seed dev DB *after deploy* |
| `holochain/seeder/**` | ✅ Holochain | 🔧 Seed dev DB *after deploy* |
| `steward/**` | ✅ Steward | - |
| `docs/**`, `*.md` | ⏭️ None | (doc-only changes) |
| `VERSION` | ✅ All pipelines | 🔧 Seed dev DB (if DNA rebuilt) |

**Safety valves:** Main and dev branches always build, regardless of changesets.

**Seeding:** Runs AFTER deployment, only for dev/feature/claude branches when DNA or seed files change. Never runs on production (main). Ensures fresh test data every time DNA is rebuilt.

### Artifact Sharing Strategy

The steward pipeline uses a **three-tier fallback** to fetch the hApp:

```
1. FAST PATH (primary):
   ↓ Try to fetch from Jenkins artifacts (~30 seconds)
   ├─ Success: ✅ Use fetched artifact → Build steward
   │
2. FALLBACK (if fetch fails):
   ↓ Build locally from source (~20-40 minutes)
   ├─ Ensure build never fails
   │
3. VERIFY:
   └─ Error if neither path succeeded
```

**URL Pattern:**
```
https://jenkins.ethosengine.com/job/elohim-holochain/job/{BRANCH}/lastSuccessfulBuild/artifact/holochain/dna/elohim/elohim.happ
```

### Build Time Impact

| Scenario | Before | After | Improvement |
|---|---|---|---|
| Full monorepo build | 60+ min | 15-20 min | **67% faster** |
| Steward hApp fetch | 40 min | 30 sec | **99% faster** |
| Steward only (no holochain changes) | 40+ min | <2 min | **~95% faster** |
| Doc-only changes | 60+ min | <1 min | **Massive savings** |

### Self-Documenting Logs

All pipelines output structured logs with emojis and formatted sections:

```
═══════════════════════════════════════════════════════════
📋 ELOHIM MONO-REPO BUILD ORCHESTRATION
═══════════════════════════════════════════════════════════
Branch: dev
Build: 1234

📊 CHANGESET ANALYSIS
─────────────────────────────────────────────────────────────
holochain changes detected:
  - holochain/dna/zomes/...
  - holochain/manifests/...

🎯 BUILD MATRIX
─────────────────────────────────────────────────────────────
✅ BUILD elohim-app
✅ BUILD holochain
✅ BUILD steward
═══════════════════════════════════════════════════════════

🔄 TRIGGERING PIPELINES
─────────────────────────────────────────────────────────────
▶️  Triggering holochain pipeline for branch: dev
✅ Holochain pipeline triggered
▶️  Triggering steward pipeline for branch: dev
✅ Steward pipeline triggered
✅ Child pipelines triggered successfully (non-blocking)
═══════════════════════════════════════════════════════════
```

The build description shows a quick summary:
```
✅ elohim-app | ✅ holochain | ✅ steward
```

### Key Design Principles

1. **Webhook-driven triggers:** All 3 jobs fire from GitHub webhook, each respects its own when{} conditions
2. **Orchestrator provides visibility:** Root job analyzes changesets and shows what will run (no actual triggering)
3. **Smart changesets:** Each pipeline only runs when relevant files change (changeset filtering in when{})
4. **Artifact reuse:** No redundant builds; steward fetches hApp from holochain archives
5. **Graceful fallback:** Steward builds always succeed (fetch → build → fail)
6. **Self-documenting logs:** Clear explanations of what's happening and why
7. **No Jenkins config changes needed:** Orchestrator works with existing webhook setup

### Troubleshooting

**Q: Why is my dev database being reset after deployment?**
- The seeding stage automatically resets and repopulates the database when DNA changes
- This is intentional for rapid prototyping iteration (dev branches only)
- Seeding runs automatically AFTER deployment when: `holochain/dna/**` or `holochain/seeder/**` change
- Other holochain changes (manifests, doorway code, etc.) won't trigger seeding
- Production (main branch) is never seeded - only dev/feature/claude branches
- To disable seeding for a specific commit, don't change DNA or seeder files

**Q: Can I seed manually?**
- Yes! Seeding is just an npm script: `cd holochain/seeder && npx tsx src/seed.ts`
- Set `HOLOCHAIN_ADMIN_URL` env var to the target holochain instance
- Useful for testing without triggering the full pipeline

**Q: Steward building hApp locally instead of fetching?**
- This is expected if the holochain pipeline hasn't run yet
- Check Jenkins artifact URL: `https://jenkins.ethosengine.com/job/elohim-holochain/job/{BRANCH}/lastSuccessfulBuild/artifact/...`
- Holochain pipeline might have failed; check its logs

**Q: Why is my build much slower than expected?**
- If steward is rebuilding hApp (20-40 min), holochain pipeline probably didn't run
- Check the build matrix in orchestrator logs
- Ensure you changed files that trigger holochain pipeline

**Q: Can I force a fresh build?**
- Changing `VERSION` file triggers all pipelines (including seeding)
- Manually trigger specific pipeline from Jenkins UI

**Q: Why run separate pipelines instead of one monolithic pipeline?**
- Parallel execution: builds run concurrently, not sequentially
- Isolation: changes to steward don't block holochain builds
- Clarity: each pipeline has focused responsibility
- Reuse: holochain can be used independently
- Seeding separation: dev-only operations don't affect production

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
