---
name: hc-dev-orchestrator
description: Start and manage local Holochain development environment with conductor, doorway, and storage services. Use when starting dev servers, debugging service connections, checking stack health, or setting up local environment.
---

# Holochain Development Orchestrator

This skill manages the local Holochain development stack for the Elohim project. It orchestrates multiple services that must start in the correct order.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Local Development Stack                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│   Angular App (4200)                                                  │
│        ↓                                                              │
│   Doorway Gateway (8888)  ←── HTTP/WS proxy, storage integration     │
│        ↓                   ↓                                          │
│   Holochain Conductor     elohim-storage (8090)                       │
│   Admin: dynamic          Blob storage, import API                    │
│   App: 4445               WebSocket progress                          │
│        ↓                                                              │
│   Lair Keystore (in-process dev mode)                                │
│                                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

## Quick Start Commands

### Full Stack (Recommended for Import Testing)
```bash
cd /projects/elohim/elohim-app

# Start conductor + doorway + storage (required for seeding)
npm run hc:start:full

# Start + run sample seed
npm run hc:start:full:seed

# Or with environment overrides
SEED_LIMIT=100 npm run hc:start:full:seed
```

### Minimal Stack (Conductor + Doorway only)
```bash
npm run hc:start
```

### Individual Services
```bash
# Storage only (after conductor is running)
npm run storage:start

# Storage in foreground (for debugging)
npm run storage:start:foreground

# Doorway only (reads dynamic admin port)
npm run doorway:start
```

## Command Reference

| Command | Description |
|---------|-------------|
| `npm run hc:start` | Start conductor + doorway (basic) |
| `npm run hc:start:full` | Start conductor + doorway + storage |
| `npm run hc:start:full:seed` | Full stack + seed sample content |
| `npm run hc:stop` | Stop all services |
| `npm run hc:reset` | Stop + clear data + restart |
| `npm run hc:status` | Show all service status |
| `npm run hc:build` | Build all DNAs |
| `npm run storage:start` | Start storage service |
| `npm run storage:stop` | Stop storage service |
| `npm run storage:build` | Build storage (with correct RUSTFLAGS) |
| `npm run doorway:start` | Start doorway (reads dynamic port) |
| `npm run doorway:stop` | Stop doorway |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `STORAGE_PORT` | 8090 | elohim-storage HTTP port |
| `STORAGE_DIR` | /tmp/elohim-storage | Blob storage directory |
| `SEED_LIMIT` | 20 | Items to seed with --seed flag |
| `ADMIN_PORT` | (auto) | Override conductor admin port |

## Script Help

All scripts are self-documenting:
```bash
./scripts/hc-start-full.sh --help
./scripts/storage-start.sh --help
```

## Health Checks

```bash
# All services
npm run hc:status

# Individual checks
curl http://localhost:8888/health   # Doorway
curl http://localhost:8888/status   # Doorway + conductor info
curl http://localhost:8090/health   # Storage
```

## Port Reference

| Service | Port | Protocol | Notes |
|---------|------|----------|-------|
| Angular | 4200 | HTTP | `ng serve` |
| Doorway | 8888 | HTTP/WS | Gateway to conductor + storage |
| Conductor App | 4445 | WebSocket | App interface |
| Conductor Admin | dynamic | WebSocket | Saved in `.hc_ports` |
| Storage | 8090 | HTTP/WS | Blob storage + import API |

## Storage Service Details

The `elohim-storage` service provides:
- **Blob Storage**: `PUT/GET /blob/{hash}`
- **Import API**: `POST /import/queue`, `GET /import/status/{id}`
- **Progress WebSocket**: `ws://localhost:8090/import/progress`
- **Health Check**: `GET /health`

### Building Storage

```bash
# Build with required RUSTFLAGS (handled by npm script)
npm run storage:build

# Or manually
cd holochain/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

## Seeding Content

```bash
# After starting full stack
cd genesis/seeder

# Local seeding
DOORWAY_URL=http://localhost:8888 \
HOLOCHAIN_ADMIN_URL=ws://localhost:$ADMIN_PORT \
npx tsx src/seed.ts --limit 20

# Or use npm scripts (from elohim-app)
npm run hc:seed:sample
```

## Troubleshooting

### "Conductor not found"
```bash
# Check if conductor is running
cat holochain/local-dev/.hc_ports
hc sandbox call --running <ADMIN_PORT> list-apps
```

### "Storage not responding"
```bash
# Check storage logs
npm run storage:start:foreground

# Verify it built correctly
npm run storage:build
```

### "Doorway wrong port"
```bash
# Restart doorway (auto-reads correct port)
npm run doorway:restart
```

### "WebSocket errors during import"
The storage service uses runtime isolation to prevent HTTP/WebSocket starvation during heavy imports. If you see "Connection reset" errors:
1. Ensure storage is running: `curl http://localhost:8090/health`
2. Ensure doorway has storage URL: Check doorway startup logs for `--storage-url`
3. The full stack script (`hc:start:full`) configures this correctly

### Port Conflicts
```bash
# Check what's using ports
fuser 8888/tcp 8090/tcp 4445/tcp

# Kill specific port
fuser -k 8888/tcp
```

### Fresh Start
```bash
npm run hc:stop
rm -rf holochain/local-dev/conductor-data
rm -rf holochain/dna/elohim/workdir
npm run hc:start:full
```

## Key File Locations

| File | Purpose |
|------|---------|
| `elohim-app/scripts/hc-start.sh` | Basic startup (conductor + doorway) |
| `elohim-app/scripts/hc-start-full.sh` | Full startup (+ storage) |
| `elohim-app/scripts/storage-start.sh` | Storage service startup |
| `elohim-app/scripts/hc-build.sh` | Multi-DNA build |
| `holochain/local-dev/.hc_ports` | Dynamic port configuration |
| `holochain/target/release/elohim-storage` | Storage binary |
| `holochain/target/release/doorway` | Doorway binary |

## Multi-DNA Architecture

The Elohim hApp contains three DNAs:

| DNA | Purpose | Zomes |
|-----|---------|-------|
| `lamad` | Learning content | content_store, content_store_integrity |
| `imagodei` | Identity/sovereignty | identity, session |
| `infrastructure` | Network services | doorway_registry |

## Common Workflows

### Testing Import Pipeline
```bash
# 1. Start full stack with storage
npm run hc:start:full

# 2. Run seeder
cd genesis/seeder
DOORWAY_URL=http://localhost:8888 \
HOLOCHAIN_ADMIN_URL=ws://localhost:$(cat ../holochain/local-dev/.hc_ports | grep admin_port | grep -o '[0-9]*') \
npx tsx src/seed.ts --limit 50
```

### Debugging Storage Service
```bash
# Run in foreground to see logs
npm run hc:start  # Start conductor + doorway first
npm run storage:start:foreground  # See storage logs directly
```

### Connect to Remote Environment
```bash
# Dev
export HOLOCHAIN_ADMIN_URL='wss://doorway-dev.elohim.host?apiKey=dev-elohim-auth-2024'
export DOORWAY_URL='https://doorway-dev.elohim.host'
```
