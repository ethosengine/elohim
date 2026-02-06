---
name: hc-dev-orchestrator
description: Start and manage the Elohim P2P Framework local development environment. Orchestrates conductor (identity/provenance), storage (content), and doorway (unified API). Use when starting dev servers, debugging service connections, or checking stack health.
---

# Elohim P2P Framework Orchestrator

This skill manages the local development stack for the Elohim Protocol. The framework combines three core components:

- **Holochain Conductor** - Cryptographic identity and provenance (agent-centric)
- **elohim-storage** - SQLite content database + blob storage (content-centric)
- **Doorway Gateway** - Unified HTTP/WebSocket API

**Note:** This is a P2P framework where Holochain provides the cryptographic/agentic layer. Content lives in elohim-storage, not the DHT.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Elohim P2P Framework                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│   Angular App (4200)                                                  │
│        ↓                                                              │
│   Doorway Gateway (8888)  ←── Unified API for all services           │
│        ↓                   ↓                                          │
│   Holochain Conductor     elohim-storage (8090)                       │
│   └─ Agent identity       └─ SQLite content DB                        │
│   └─ Cryptographic         └─ Blob storage                            │
│      provenance            └─ Import API                              │
│        ↓                                                              │
│   Lair Keystore (in-process dev mode)                                │
│                                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

## Quick Start Commands

```bash
cd /projects/elohim/elohim-app

# Start full stack (conductor + storage + doorway)
npm run hc:start

# Start with content seeding
npm run hc:start:seed

# Start + Angular dev server
npm run dev

# Check status
npm run hc:status

# Stop all services
npm run hc:stop
```

## Command Reference

| Command | Description |
|---------|-------------|
| `npm run hc:start` | Start full stack (conductor + storage + doorway) |
| `npm run hc:start:seed` | Full stack + seed sample content |
| `npm run hc:start:conductor` | Conductor only (debug mode, rare) |
| `npm run hc:stop` | Stop all services |
| `npm run hc:reset` | Stop + clear all data + restart |
| `npm run hc:status` | Show all service status |
| `npm run hc:build` | Build all DNAs |
| `npm run hc:build:all` | Rebuild everything (DNAs + binaries) |
| `npm run hc:seed` | Seed content to local stack |
| `npm run dev` | Start full stack + Angular dev server |

### Component-Specific Commands

| Command | Description |
|---------|-------------|
| `npm run storage:start` | Start storage service |
| `npm run storage:start:foreground` | Start storage (logs visible) |
| `npm run storage:stop` | Stop storage service |
| `npm run storage:build` | Build storage binary |
| `npm run storage:stats` | Show content database stats |
| `npm run doorway:start` | Start doorway |
| `npm run doorway:stop` | Stop doorway |
| `npm run doorway:build` | Build doorway binary |
| `npm run doorway:logs` | Show doorway status |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `STORAGE_PORT` | 8090 | elohim-storage HTTP port |
| `STORAGE_DIR` | /tmp/elohim-storage | Storage data directory |
| `SEED_LIMIT` | 20 | Items to seed with --seed flag |
| `ADMIN_PORT` | (auto) | Override conductor admin port |

## Health Checks

```bash
# Full status
npm run hc:status

# Individual checks
curl http://localhost:8888/status        # Doorway (full status)
curl http://localhost:8888/health        # Doorway (quick health)
curl http://localhost:8888/db/stats      # Content database stats
curl http://localhost:8090/health        # Storage direct
curl http://localhost:8090/db/stats      # Storage DB stats
```

## Port Reference

| Service | Port | Protocol | Description |
|---------|------|----------|-------------|
| Angular | 4200 | HTTP | Dev server (`ng serve`) |
| Doorway | 8888 | HTTP/WS | Unified gateway API |
| Conductor App | 4445 | WebSocket | Holochain app interface |
| Conductor Admin | dynamic | WebSocket | Saved in `.hc_ports` |
| Storage | 8090 | HTTP/WS | Content DB + blobs |

## Content Database

elohim-storage provides a SQLite content database:

```bash
# List content
curl "http://localhost:8888/db/content?limit=10"

# Get content by ID
curl http://localhost:8888/db/content/manifesto

# List learning paths
curl http://localhost:8888/db/paths

# Get path with steps
curl http://localhost:8888/db/paths/elohim-protocol

# Database stats
curl http://localhost:8888/db/stats
```

## Blob Storage

```bash
# Upload blob
curl -X PUT "http://localhost:8090/blob/sha256-abc123" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @file.bin

# Get blob
curl http://localhost:8888/blob/sha256-abc123

# Check blob exists
curl -I http://localhost:8888/blob/sha256-abc123
```

## Seeding Content

```bash
# Quick seed (from elohim-app)
npm run hc:seed

# Or with options (from seeder directory)
cd genesis/seeder
DOORWAY_URL=http://localhost:8888 \
STORAGE_URL=http://localhost:8090 \
npx tsx src/seed.ts --limit 50

# Dry run (validate without writing)
npm run seed:dry-run

# Validate schema only
npm run seed:validate
```

## Troubleshooting

### Services Not Starting
```bash
# Check what's using ports
fuser 8888/tcp 8090/tcp 4445/tcp

# Kill stuck processes
npm run hc:stop
fuser -k 8888/tcp 8090/tcp

# Fresh start
npm run hc:reset
npm run hc:start
```

### Content Database 503 Errors
If you see `503 Service Unavailable` on `/db/*` endpoints:
```bash
# Ensure storage has ENABLE_CONTENT_DB=true
npm run storage:start:foreground
# Look for: "SQLite content database enabled"
```

### Conductor Not Responding
```bash
# Check ports file
cat ../holochain/local-dev/.hc_ports

# Test conductor
ADMIN_PORT=$(cat ../holochain/local-dev/.hc_ports | grep admin_port | grep -o '[0-9]*')
hc sandbox call --running $ADMIN_PORT list-apps
```

### Doorway Missing Storage
```bash
# Check doorway status
curl http://localhost:8888/status | jq '.storage'
# Should show: "configured": true, "healthy": true

# Restart doorway with storage
npm run doorway:stop
npm run doorway:start
```

## Key File Locations

| File | Purpose |
|------|---------|
| `elohim-app/scripts/hc-start.sh` | Main startup script |
| `elohim-app/scripts/storage-start.sh` | Storage service startup |
| `elohim-app/scripts/hc-build.sh` | Multi-DNA build |
| `holochain/local-dev/.hc_ports` | Dynamic port configuration |
| `holochain/target/release/elohim-storage` | Storage binary |
| `doorway/target/release/doorway` | Doorway binary |

## Multi-DNA Architecture

The Elohim hApp provides identity and provenance:

| DNA | Purpose | Zomes |
|-----|---------|-------|
| `lamad` | Learning provenance | content_store, content_store_integrity |
| `imagodei` | Identity/sovereignty | identity, session |
| `infrastructure` | Network services | doorway_registry |

## Common Workflows

### Full Development Session
```bash
# Start everything + Angular
npm run dev

# Or step by step
npm run hc:start
npm start
```

### Content Development
```bash
# Start stack
npm run hc:start

# Edit content in genesis/data/lamad/
# Then seed
npm run hc:seed

# Verify
curl http://localhost:8888/db/stats
```

### Debugging Storage
```bash
# Start conductor only
npm run hc:start:conductor

# Start storage in foreground (see all logs)
npm run storage:start:foreground

# In another terminal, start doorway
npm run doorway:start
```

### Connect to Remote Environment
```bash
# Dev environment
export DOORWAY_URL='https://doorway-dev.elohim.host'
export HOLOCHAIN_ADMIN_URL='wss://doorway-dev.elohim.host/admin?apiKey=dev-elohim-auth-2024'
```
