# Elohim Steward

Run your own node as a steward of co-creation.

Elohim Steward is a Tauri 2.x desktop application that runs the full Elohim P2P stack on the user's machine - making them a direct participant in the network's web of mutual accountability, rather than a consumer of hosted services.

## What the Steward Does

The Elohim Protocol is a P2P framework working toward socially-resilient stewardship: technology that helps humans understand the complex interdependencies of relationships that make thriving possible. Multiple layers work together to make this real:

- **Holochain conductor** provides cryptographic identity and provenance - every action is signed, every claim is verifiable by peers. This is the trust layer that makes mutual accountability possible without central authority
- **elohim-storage** holds content (learning material, blobs, mastery records, sessions) in a local SQLite database. This is where the substance of learning and collaboration lives
- **Angular UI** delivers the full platform experience: learning paths (lamad), identity (imagodei), economics (shefa), governance (qahal). These pillars map to the dimensions of human thriving the protocol is designed to support
- **Tauri shell** ties it all together as a native desktop app with deep linking and OS integration

The same Angular app runs in the browser (via doorway) and in steward. The difference is where the services live - hosted vs. local - and how deeply the user participates in the network that everyone depends on.

## Progressive Stewardship

Users graduate through deepening levels of participation and responsibility:

| Stage | Keys | Infrastructure | Relationship to Network |
|-------|------|----------------|------------------------|
| Visitor | None | Browser only | Exploring the vision |
| Hosted User | Custodial (doorway holds) | Hosted conductor | Learning, contributing content |
| **App Steward** | **Self-custodied** | **Steward app (this)** | **Full peer: validating, holding DHT shards** |
| App + Node Steward | Self-custodied | Steward app + elohim-node | Resilient peer: two nodes backing each other up |
| Doorway Host | Self-custodied | elohim-node + doorway gateway | Serving the wider community: Web 2.0 on-ramps for those still graduating |

The steward app is full participation. It runs a conductor, validates peers, holds DHT shards - it is a complete node in the network. The practical reality is that laptops close. An [elohim-node](../elohim-node/) is a peer stewarded by the same person, providing internal resilience - when the app sleeps, the node carries the load, and vice versa.

A doorway is not a separate category. It's a node steward who goes above and beyond by running a federated gateway that provides Web 2.0 conveniences (HTTP access, custodial keys, bootstrap/signal) for users who haven't graduated yet. Consider if you want to become a doorway host one day.

## Prerequisites

- [Nix](https://nixos.org/download/) with flakes enabled
- The steward nix devshell provides all build tooling (Rust, Node.js, Holochain, Tauri CLI)

## Quick Start

```bash
# Enter the nix development shell
nix develop ./steward --accept-flake-config

# Start the app in dev mode (connects to Angular dev server)
just steward-dev

# Or equivalently:
npm run tauri:dev
```

Dev mode expects the Angular dev server running on `localhost:4200`. Start it in another terminal:

```bash
just app-dev
```

## Build

```bash
nix develop ./steward --accept-flake-config

# Production build (outputs .deb package)
just steward-build
```

### Build Prerequisites

The hApp bundle (`workdir/elohim.happ`) and UI assets (`ui/`) must be present:

```bash
# Build Holochain DNAs
just dna-build

# Build Angular UI (outputs to ui/)
just app-build
```

In CI, these are built and placed automatically before `tauri build`.

## Doorway Integration

A doorway is a federated thin-client gateway - a node steward who provides Web 2.0 conveniences (HTTP access, custodial keys, bootstrap/signal, account recovery) so users can participate before they're ready to run their own infrastructure, and have a portal to recover when disaster happens.

The steward app can operate standalone or connected to a doorway:

- **Standalone**: Fresh identity, local-only. The user generates a new agent key and participates as a new peer
- **Connected**: Login to a doorway, import existing identity + network config, join the same DHT space as that doorway's community
- **Graduated**: After confirming stewardship (proving key possession), the doorway retires its custodial cell. The custodial training wheels come off - the steward's node now participates directly in peer validation and DHT gossip, deepening their contribution to the shared infrastructure

## Project Structure

```
steward/
  src-tauri/
    src/
      lib.rs        # App setup, conductor, IPC commands, WebSocket interceptor
      doorway.rs    # Doorway HTTP client (auth, handoff, stewardship)
      identity.rs   # Key bundle crypto (Argon2id + ChaCha20)
      main.rs       # Entry point
    Cargo.toml
    tauri.conf.json
  ui/               # Angular build output (populated by CI)
  workdir/          # hApp bundle (populated by CI)
  flake.nix         # Nix devShell (Holochain + Tauri + P2P toolchain)
  package.json      # npm scripts (tauri:dev, tauri:build)
```

## Dev Workflows

All commands are available from the project root via `just`:

| Command | Description |
|---------|-------------|
| `just steward-dev` | Build + run in dev mode |
| `just steward-build` | Production build |
| `just dna-build` | Build Holochain DNAs |
| `just storage-build` | Build elohim-storage |
| `just app-dev` | Start Angular dev server |
| `just status` | Check all service health |
| `just session` | Check local session state |

Run `just --list` from the project root for the full list.
