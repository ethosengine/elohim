# Monorepo Restructure Design

## Problem

The `holochain/` directory is a misleading catch-all containing Rust services, TypeScript SDKs, deployment infra, and shared crates alongside genuinely Holochain-specific code. Top-level directories don't reflect system boundaries or deployment relationships.

## Organizing Principle

Group by **system boundary / deployment unit**:

- **`elohim/`** — core runtime. Everything needed to run the protocol in any context (device, node, hosted).
- **`doorway/`** — optional hosted gateway add-on to core.
- **`steward/`** — deployment shells (device, node, future: mobile, wearable, observer, POS).
- **`crates/`** — shared Rust crates at root (large `target/` directories benefit from shared workspace).
- **`genesis/`** — meta layer: orchestrator, content pipeline, docs, deployment manifests.
- **Frontend** (`elohim-app/`, `elohim-library/`, `sophia/`, `elohim-ui-playground/`) — stays at root pending future frontend reorganization.

## Target Structure

```
elohim/                              <- core runtime
  constitution/                      (stays)
  eae/                               (stays)
  elohim-agent/                      <- agent boundary (restructured)
    elohim-agent-sdk/                <- from elohim/elohim-agent-sdk/
    elohim-agent-service/            <- from elohim/elohim-agent/
    mcp-servers/                     <- from /mcp-servers/
  elohim-storage/                    <- from holochain/elohim-storage/
  elohim-bitswap/                    <- from holochain/elohim-bitswap/
  rust-ipfs/                         <- from /rust-ipfs/ (git submodule)
  sdk/                               <- from holochain/sdk/ (TS client libs)
  holochain/                         <- zome/WASM layer
    dna/                             <- from holochain/dna/
    holochain-cache-core/            <- from holochain/holochain-cache-core/
    rna/                             <- from holochain/rna/
    edgenode/                        <- from holochain/edgenode/
    elohim-wasm/                     <- from holochain/elohim-wasm/

doorway/                             <- optional hosted gateway (already restructured)
  doorway-service/                   <- from /doorway/
  doorway-app/                       <- from /doorway-app/

steward/                             <- deployment shells
  device/                            <- from /steward/ (Tauri desktop)
  node/                              <- from /elohim-node/ (headless P2P)

crates/                              <- shared Rust crates
  doorway-client/                    <- from holochain/crates/doorway-client/
  elohim-sdk/                        <- from holochain/crates/elohim-sdk/
  elohim-storage-client/             <- from holochain/crates/elohim-storage-client/

genesis/                             <- meta/ops/content
  manifests/                         <- from holochain/manifests/
  (everything else stays)

elohim-app/                          (stays - frontend TBD)
elohim-library/                      (stays - frontend TBD)
elohim-ui-playground/                (stays - frontend TBD)
sophia/                              (stays - git submodule, frontend TBD)

research/                            <- distributed to nearest analogues
```

## Move Manifest

| Current Path | New Path | Notes |
|---|---|---|
| `holochain/elohim-storage/` | `elohim/elohim-storage/` | Core service |
| `holochain/elohim-bitswap/` | `elohim/elohim-bitswap/` | Core P2P |
| `holochain/sdk/` | `elohim/sdk/` | TS client libs |
| `holochain/crates/` | `crates/` | Shared Rust crates |
| `holochain/manifests/` | `genesis/manifests/` | Deployment manifests |
| `holochain/dna/` | `elohim/holochain/dna/` | Holochain zomes |
| `holochain/holochain-cache-core/` | `elohim/holochain/holochain-cache-core/` | WASM cache |
| `holochain/rna/` | `elohim/holochain/rna/` | Schema templates |
| `holochain/edgenode/` | `elohim/holochain/edgenode/` | hApp container runtime |
| `holochain/elohim-wasm/` | `elohim/holochain/elohim-wasm/` | WASM utilities |
| `holochain/local-dev/` | TBD | Dev tooling, decide during implementation |
| `elohim-node/` | `steward/node/` | Headless P2P steward |
| `steward/` | `steward/device/` | Tauri desktop steward |
| `elohim/elohim-agent-sdk/` | `elohim/elohim-agent/elohim-agent-sdk/` | Agent SDK |
| `elohim/elohim-agent/` | `elohim/elohim-agent/elohim-agent-service/` | Agent service |
| `mcp-servers/` | `elohim/elohim-agent/mcp-servers/` | AI tooling |
| `rust-ipfs/` | `elohim/rust-ipfs/` | Git submodule |
| `research/` | Distributed | Each subdirectory to nearest analogue |

## Cascading Updates Required

### Orchestrator Pipeline (`genesis/orchestrator/Jenkinsfile`)
- All `changePatterns` in `PIPELINES` map must update to new paths
- `jenkinsPath` references for moved Jenkinsfiles
- Pipeline artifact paths

### Cargo Workspaces
- Root `Cargo.toml` workspace members (if exists)
- `holochain/elohim-storage/Cargo.toml` internal path dependencies
- `doorway/doorway-service/Cargo.toml` path dependencies to crates
- `steward/device/src-tauri/Cargo.toml` path dependencies

### pnpm Workspace
- `pnpm-workspace.yaml` package paths
- Any `file:` references in `package.json` files
- `holochain/sdk/` → `elohim/sdk/` in workspace config

### CI/CD
- All Jenkinsfiles with hardcoded paths
- Docker build contexts
- Deployment scripts

### Angular
- `proxy.conf.mjs` if paths referenced
- `tsconfig.json` path aliases if any reference moved dirs
- Import paths in `elohim-app/` referencing `holochain/sdk/`

### CLAUDE.md & Memory
- Build commands with directory paths
- Architecture docs referencing old paths
- Memory files with old paths

## Research Distribution Plan

| Research Directory | Proposed New Location | Rationale |
|---|---|---|
| `research/economic/` | `genesis/research/economic/` | Shefa economics research |
| `research/bootstrap/` | `elohim/research/bootstrap/` | Core bootstrap research |
| `research/p2p-shipyard/` | `steward/research/p2p-shipyard/` | P2P runtime research |
| `research/sbd/` | `steward/research/sbd/` | P2P transport research |
| `research/tx5/` | `steward/research/tx5/` | P2P transport research |
| `research/tauri-plugin-holochain/` | `steward/device/research/` | Desktop steward research |
| `research/holo-envoy/` | `elohim/holochain/research/` | Holochain hosting research |
| `research/matrix/` | `elohim/research/matrix/` | Federation research |
| `research/web-sdk/` | `elohim/sdk/research/` | SDK research |

## Deferred Decisions

- **`holochain/local-dev/`** — decide during implementation
- **Frontend reorganization** — `elohim-app/`, `elohim-library/`, `sophia/`, `elohim-ui-playground/` stay at root for now
- **`images/`** (root) — not discussed, stays
- **`scripts/`** (root) — not discussed, stays
- **`patches/`** (root) — not discussed, stays
