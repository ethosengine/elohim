# P2P-Native Build System Roadmap

**Status**: Backlog — vision and arc documented, not yet scheduled
**Priority**: After app scalability/reliability is proven
**Depends on**: Protocol schema contract (Phase 1 complete), EPR spec, steward network

## Problem

53 of the last 96 fix commits are build/CI/environment sync failures — not application bugs. Path drift after restructures, Dockerfile COPY mismatches, Rust version divergence, pnpm workspace boundary leaks, submodule location issues, OOMKill resource guessing. Each failure costs 6-20+ minutes of pipeline wait time to discover.

Root cause: the build system cannot express its own inputs. Jenkinsfile is imperative script pretending to be declarative. It discovers dependencies at runtime instead of declaring them. The gap between "what the developer's machine knows" and "what the build environment knows" is bridged by trial and error.

## Vision

Artifacts are ContentNodes in the protocol. A `BuildManifest` is a content type — addressed by CID, carrying three-pillar metadata (lamad: what it does, shefa: who contributed, qahal: who attested), gossipped through the network. The build system is legible from inside the app. Non-developers can trace a bug report to the exact artifact, its source, its build recipe, and its attestations.

The application becomes self-building: a manifest ContentNode arrives at a peer with the necessary hardware and toolchain, that peer builds the artifact, creates an attestation ContentNode, and that attestation gossips back. Enough attestations from diverse peers and the artifact is network-verified for distribution.

### ContentNode as Universal Container

Everything is a ContentNode with a content type:

- `ContentNode -> markdown` (learning content)
- `ContentNode -> package` (dependency)
- `ContentNode -> wasm` (compiled module)
- `ContentNode -> image` (Docker/OCI image)
- `ContentNode -> build-manifest` (build recipe)
- `ContentNode -> build-attestation` (peer verification)

Build manifests follow the EPR three-tier model: Head (~500B, gossipped) contains the manifest metadata and input CIDs. Document contains the full build recipe. Bytes are the compiled output.

### Anti-Capture Through Legibility

Developer ergonomics IS human ergonomics. If only developers can see/use the code and build process, that concentrates power through technical obscurity. The build manifest, artifact registry, and attestation layer must be legible to any protocol participant — bug reporters, feature requesters, stewards, and elohim agents alike.

## Approach: Protocol-Native Build Manifests

Design the manifest format as a ContentNode schema from day one (in `genesis/protocol-schema/`). The same manifest that fixes Jenkins today is the one peers build from tomorrow.

Rejected alternatives:
- **Artifact manifests + dumb orchestrator** (TOML files, Jenkins reads them): Solves immediate pain but creates migration debt to a format that isn't protocol-native.
- **Nix as content-addressed layer**: Proven technology, but introduces a foreign worldview that doesn't naturally become protocol-native. Nix's content-addressing model is relevant prior art for the manifest schema design.

## Four-Stage Arc

### Stage 0 — "The Seed" (Current State)

Conventional Jenkins builds. Artifacts trusted because Matthew built them. 6 pipelines, centralized orchestrator, monolithic stages, all-or-nothing execution.

**Current artifact inventory** (~10 distinct artifacts):
- elohim-app (Angular -> Docker image)
- doorway (Rust -> Docker image)
- elohim-storage (Rust -> Docker image)
- elohim-agent-sdk (Rust -> Docker image)
- edgenode (Rust -> Docker image)
- sophia UMD (React/TS -> JS bundle)
- holochain-cache-core (Rust -> WASM blob)
- elohim.happ (DNA compilation -> .happ bundle)
- doorway-app (Angular -> Docker image)
- genesis seed data (JSON transform -> seed payload)

### Stage 1 — "The Root"

Build manifests exist as ContentNode schemas in the protocol. Jenkins reads manifests, hashes inputs, checks cache, builds if changed, publishes artifact CID. Single trusted builder (Jenkins or a steward node) is sole attester.

**What changes**:
- Each artifact directory gets a `build-manifest.json` validated against protocol schema
- Manifest declares: input paths/CIDs, toolchain requirements (Rust version, Node version), build command, output content type, hardware constraints
- Jenkins becomes a manifest executor — read, hash inputs, cache check, build, publish
- Artifacts get CIDs on creation — bug reports can link to artifact CIDs
- Build graph is legible from the app (read-only, initially)

**What this solves immediately**:
- Path drift: inputs are declared, not discovered
- Environment divergence: toolchain is in the manifest, not implicit
- Dependency boundary leaks: input CIDs make the boundary explicit
- Retriability: each artifact builds independently, failures don't cascade
- Cache hits: unchanged inputs = skip the build

**Sprint sketch** (when scheduled):
1. Define `BuildManifest` and `BuildAttestation` content types in protocol schema
2. Write manifests for 2-3 simplest artifacts (sophia UMD, holochain-cache-core WASM)
3. Build a manifest executor (can be a Jenkins shared library or standalone CLI)
4. Migrate remaining artifacts to manifests
5. Wire artifact CIDs into the app's version/health endpoints

### Stage 2 — "The Canopy"

Multiple steward nodes can pick up manifest ContentNodes and independently build + attest. The network has consensus on artifact viability. Jenkins becomes one builder among many.

**What changes**:
- Build manifests gossipped via DHT (or feed subscription)
- Steward nodes with matching hardware/toolchain claim and execute builds
- `BuildAttestation` ContentNodes published with: builder agent ID, hardware profile, input CID, output CID, test results hash, timestamp
- Threshold attestation: N of M diverse peers agree -> artifact is network-verified
- Attestation doesn't require bit-identical reproducibility — "I built this, tests passed on my hardware, here's my output CID." If two output CIDs differ but both pass, that's information about the build, not a failure
- Hardware capabilities are part of the peer's agent profile (same pattern as contributor presence)
- Builder reputation follows the same stewardship model as content: mastery gate + affinity lifecycle

**Prerequisites**:
- Steward network operational with sufficient peers
- Reproducible builds for at least Rust and WASM targets
- Multi-architecture Tauri builds driving the need

### Stage 3 — "The Forest"

The build system builds itself. A new version of elohim-storage is a manifest ContentNode that gets built, attested, and distributed through the same protocol it implements. Jenkins is gone.

**What changes**:
- The orchestrator is a coordinator zome, not a Jenkinsfile
- Build scheduling is governance-aware (qahal dimension)
- Resource allocation for builds follows stewardship economics (shefa dimension)
- The build graph is navigable content in the app (lamad dimension)
- Non-developers can propose changes (feature requests become manifest forks)
- Elohim agents are the natural reproducibility auditors — they re-verify attestations, flag divergences, and participate in build governance as part of the protocol's nervous system

**This stage is aspirational and will be designed when Stage 2 is operational.**

## Prior Art and Influences

- **Nix/Guix content-addressed derivations**: The model for "build as pure function, inputs -> outputs." Guix's reproducible builds + P2P substitutes research (NLnet-funded) is the closest prior art to Stage 2.
- **Bazel Remote Execution API**: Content-addressed actions with hermetic execution. The execution model, minus the centralized scheduler.
- **Radicle CI broker**: P2P attestation of build results. Closest existing implementation of "peer builds and signs results."
- **Warg registry** (Bytecode Alliance): Federated WASM registry with Certificate Transparency. The trust model for a package registry.
- **OCI + Dragonfly/Kraken**: P2P distribution of content-addressed container layers. The distribution layer.
- **Tea Protocol (cautionary tale)**: Attaching token incentives to package registration created 150K+ spam packages on npm. Financial incentives for builds must be designed carefully — stewardship affinity (not token rewards) is the right incentive model.

## Open Questions

1. **Manifest expressiveness**: How do you describe a Rust cross-compilation, an Angular build, and a WASM compilation in the same schema without inventing a Turing-complete build language? The manifest likely declares *what* to build (inputs, toolchain, output type) and references a *how* (a builder plugin/recipe per toolchain), rather than embedding build steps directly.
2. **Transition trigger**: What's the signal that it's time to move from Stage 0 to Stage 1? Likely when a second regular contributor joins and the "trusted because Matthew built it" model becomes insufficient. Or when Tauri multi-arch builds force the issue.
3. **Manifest-to-ContentNode migration**: Stage 1 manifests live on the filesystem alongside source. When they become ContentNodes in the DHT, does the filesystem version remain as the source-of-truth (published on commit), or does the DHT become canonical?
