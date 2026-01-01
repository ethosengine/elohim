---
name: hc-dev-orchestrator
description: Start and manage local Holochain development environment with conductor, doorway, and storage services. Use when starting dev servers, debugging service connections, checking stack health, or setting up local environment.
---

# Holochain Development Orchestrator

This skill manages the local Holochain development stack for the Elohim project. It orchestrates multiple services that must start in the correct order.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Local Development Stack                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   Angular App (4200)                                         │
│        ↓                                                     │
│   Doorway Gateway (8888)  ←──  HTTP/WebSocket proxy          │
│        ↓                                                     │
│   Holochain Conductor     ←──  Admin: dynamic, App: 4445     │
│        ↓                                                     │
│   Lair Keystore           ←──  In-process (dev mode)         │
│                                                              │
│   Storage Service (8090)  ←──  Blob storage (optional)       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start Commands

### Start Everything
```bash
cd /projects/elohim/elohim-app
npm run hc:start
```
This runs `scripts/hc-start.sh` which:
1. Builds hApp if missing (3 DNAs: lamad, imagodei, infrastructure)
2. Starts Holochain sandbox with in-process lair
3. Starts Doorway gateway pointing to conductor
4. Verifies hApp installation

### Individual Service Commands

| Command | Purpose |
|---------|---------|
| `npm run hc:start` | Full stack startup |
| `npm run hc:stop` | Stop all services |
| `npm run hc:reset` | Stop + clear data + restart |
| `npm run hc:sandbox` | Sandbox only (no doorway) |
| `npm run hc:build` | Build DNAs and pack hApp |
| `npm run doorway:start` | Doorway gateway only |
| `npm run doorway:stop` | Stop doorway |
| `npm run storage:start` | Blob storage service |
| `npm run storage:stop` | Stop storage |

## Service Health Checks

### Check Conductor Status
```bash
# Get admin port from saved state
cat /projects/elohim/holochain/local-dev/.hc_ports

# List installed apps
hc sandbox call --running <ADMIN_PORT> list-apps
```

### Check Doorway Health
```bash
curl http://localhost:8888/health
curl http://localhost:8888/status
```

### Check Storage Health
```bash
curl http://localhost:8090/health
```

## Port Reference

| Service | Port | Protocol | Notes |
|---------|------|----------|-------|
| Angular | 4200 | HTTP | Dev server |
| Doorway | 8888 | HTTP/WS | Gateway to conductor |
| Conductor App | 4445 | WebSocket | App interface |
| Conductor Admin | dynamic | WebSocket | Saved in `.hc_ports` |
| Storage | 8090 | HTTP | Blob storage |

## Troubleshooting

### Conductor Won't Start
```bash
# Check for zombie processes
ps aux | grep holochain
ps aux | grep hc

# Kill and restart
pkill -f holochain
pkill -f "hc sandbox"
npm run hc:start
```

### Doorway Wrong Port
If doorway connects to wrong conductor:
```bash
# Check current state
curl http://localhost:8888/status

# Restart doorway with correct port
npm run doorway:stop
ADMIN_PORT=$(cat /projects/elohim/holochain/local-dev/.hc_ports | grep admin_port | grep -o '[0-9]*')
npm run doorway:start
```

### Port Already in Use
```bash
# Find what's using a port
fuser 8888/tcp
fuser 4445/tcp

# Kill specific port
fuser -k 8888/tcp
```

### hApp Not Installed
```bash
# Rebuild and reinstall
npm run hc:stop
rm -rf /projects/elohim/holochain/dna/elohim/workdir/*.dna
rm -rf /projects/elohim/holochain/dna/elohim/workdir/*.happ
npm run hc:start
```

## Key File Locations

| File | Purpose |
|------|---------|
| `elohim-app/scripts/hc-start.sh` | Main orchestration script |
| `elohim-app/scripts/hc-build.sh` | Multi-DNA build script |
| `holochain/local-dev/.hc_ports` | Saved port configuration |
| `holochain/local-dev/.sandbox_log` | Conductor startup log |
| `holochain/dna/elohim/workdir/` | Built artifacts |
| `holochain/doorway/target/release/doorway` | Gateway binary |

## Multi-DNA Architecture

The Elohim hApp contains three DNAs:

| DNA | Purpose | Zomes |
|-----|---------|-------|
| `lamad` | Learning content | content_store, content_store_integrity |
| `imagodei` | Identity/sovereignty | identity, session |
| `infrastructure` | Network services | doorway_registry |

Build order matters - all three must compile before packing:
```bash
# Manual build (usually not needed)
cd /projects/elohim
./elohim-app/scripts/hc-build.sh
```

## Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `HOLOCHAIN_ADMIN_PORT` | (from .hc_ports) | Conductor admin port |
| `CONDUCTOR_URL` | ws://localhost:4445 | App interface URL |
| `DOORWAY_URL` | http://localhost:8888 | Gateway URL |

## Common Workflows

### Fresh Start (Clean Slate)
```bash
npm run hc:stop
rm -rf /projects/elohim/holochain/local-dev/conductor-data
rm -rf /projects/elohim/holochain/dna/elohim/workdir
npm run hc:start
```

### Rebuild After Zome Changes
```bash
npm run hc:stop
npm run hc:build
npm run hc:start
```

### Connect to Remote Doorway (Dev/Staging)
```bash
# Dev environment
export HOLOCHAIN_ADMIN_URL='wss://doorway-dev.elohim.host?apiKey=dev-elohim-auth-2024'

# Staging environment
export HOLOCHAIN_ADMIN_URL='wss://doorway-staging.elohim.host?apiKey=...'
```
