# Holochain Development Workflow

This document describes how to set up, reset, and work with the Holochain development environment.

## Quick Start

From the `elohim-app` directory:

```bash
# Full reset and fresh start
npm run hc:reset
npm run hc:start

# Bootstrap node steward account
npm run hc:bootstrap -- --email steward@elohim.host --password secureSteward123 --name "Node Steward"

# Seed content (optional)
npm run hc:seed:sample
```

## Architecture Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Angular App   │────▶│    Doorway      │────▶│    Holochain    │
│   (Browser)     │     │   (Port 8888)   │     │   Conductor     │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │
        │                       ▼
        │               ┌─────────────────┐
        │               │   Auth (JWT)    │
        │               │  + Worker Pool  │
        │               └─────────────────┘
        ▼
┌─────────────────┐
│  localStorage   │
│ (Session/Token) │
└─────────────────┘
```

### Components

- **Angular App**: Frontend UI, runs at `localhost:4200` (or via Che endpoint)
- **Doorway Gateway**: Rust WebSocket gateway with JWT auth and worker pool
- **Holochain Conductor**: DHT node running the elohim hApp

## Commands Reference

### Lifecycle Commands

| Command | Description |
|---------|-------------|
| `npm run hc:start` | Start Holochain sandbox + Doorway |
| `npm run hc:stop` | Stop all Holochain services |
| `npm run hc:reset` | Full reset (clears all data) |
| `npm run hc:status` | Check service status |

### User Management

| Command | Description |
|---------|-------------|
| `npm run hc:bootstrap -- [options]` | Create node steward account |
| `npm run hc:seed` | Seed all content from data/lamad |
| `npm run hc:seed:sample` | Seed sample content (10 items) |

### Bootstrap Options

```bash
npm run hc:bootstrap -- \
  --email <email>          # Required: Login email
  --password <password>    # Required: Password (min 8 chars)
  --name <name>            # Required: Display name
  --bio <bio>              # Optional: User bio
  --affinities <list>      # Optional: Comma-separated interests
```

### Snapshot Management

| Command | Description |
|---------|-------------|
| `npm run hc:snapshot:save` | Save current state |
| `npm run hc:snapshot:restore` | Restore from snapshot |
| `npm run hc:snapshot:list` | List available snapshots |
| `npm run hc:start:snapshot` | Start from saved snapshot |

## Workflows

### 1. Fresh Development Setup

```bash
# From elohim-app directory
npm run hc:reset              # Clean slate
npm run hc:start              # Start services (wait for ready)

# Create node steward (you)
npm run hc:bootstrap -- \
  --email steward@elohim.host \
  --password secureSteward123 \
  --name "Node Steward"

# Optionally seed content
npm run hc:seed:sample

# Start Angular
npm start
```

### 2. Resume After Restart

If you stopped the services but didn't reset:

```bash
npm run hc:start              # Services restore state automatically
# Auth credentials persist in data/users.json
# Holochain data persists in conductor-data/
```

### 3. Add a Hosted User

After node steward is set up, additional users can be created:

```bash
npm run hc:bootstrap -- \
  --email user@example.com \
  --password userPassword123 \
  --name "Test User" \
  --affinities "governance,learning"
```

### 4. Full Reset (Clean Slate)

```bash
npm run hc:reset              # Clears everything
npm run hc:start              # Fresh start
# Must re-bootstrap and re-seed
```

## User Types

### Node Steward (You)
- Operates the edge node infrastructure
- Has access to all admin functions
- Created via bootstrap script

### Hosted Human
- Regular user with email/password login
- Data stored in Holochain DHT
- Auth via JWT tokens from Doorway

### Session Visitor (Future)
- Browses without account
- Data stored locally in browser
- Can convert to Hosted Human (preserving session data)

## Eclipse Che Environment

In Che, the Angular app auto-detects the environment and routes to:
- **Admin WebSocket**: `wss://<workspace>-hc-dev.code.ethosengine.com/admin`
- **App WebSocket**: `wss://<workspace>-hc-dev.code.ethosengine.com/app/<port>`
- **Auth HTTP**: `https://<workspace>-hc-dev.code.ethosengine.com/auth/*`

## Troubleshooting

### "Conductor not running"
```bash
npm run hc:stop
npm run hc:start
```

### "Auth credentials already exist"
```bash
npm run hc:reset:auth
# Then re-bootstrap
```

### "Human already registered"
This is normal - bootstrap will use the existing Holochain human.

### CORS errors in browser
Check that Doorway is running and the Angular app detected the Che environment:
```bash
curl http://localhost:8888/health
```

### WebSocket connection failures
Verify the conductor admin port:
```bash
ss -tlnp | grep holochain
# Use the port shown for CONDUCTOR_URL
```

## File Locations

| Path | Description |
|------|-------------|
| `holochain/local-dev/conductor-data/` | Holochain persistent data |
| `holochain/local-dev/.hc_ports` | Dynamic port assignments |
| `holochain/seeder/data/lamad/` | Content seed data |
| `holochain/dna/elohim/` | Holochain hApp source (multi-DNA) |
| `holochain/doorway/` | Doorway gateway (Rust) |

## Test Credentials

Default node steward (after bootstrap):
- Email: `steward@elohim.host`
- Password: `secureSteward123`

These can be changed via the bootstrap command options.
