---
name: seed-workflow
description: Validate, seed, and verify content in Holochain DHT. Use when seeding content, running validation, checking seed statistics, managing snapshots, or troubleshooting seeding issues.
---

# Content Seeding Workflow

This skill manages the complete content seeding pipeline for the Elohim project - from validation through seeding to verification.

## Pipeline Overview

```
┌──────────────────────────────────────────────────────────────┐
│                    Seeding Pipeline                           │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  1. VALIDATE        Schema + metadata validation              │
│       ↓                                                       │
│  2. DRY-RUN         Preview without writing                   │
│       ↓                                                       │
│  3. SEED            Write to Holochain DHT                    │
│       ↓                                                       │
│  4. VERIFY          Post-seed validation                      │
│                                                               │
│  Source: genesis/data/lamad/   →   Target: Holochain DHT      │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

## Quick Reference

### Complete Workflow (Recommended)
```bash
cd /projects/elohim/genesis/seeder

# Step 1: Validate schema
npm run validate

# Step 2: Preview changes
npm run seed:dry-run

# Step 3: Seed to local
npm run seed

# Step 4: Verify
npm run stats
```

### Environment-Specific Commands

| Environment | Seed Command | Stats Command |
|-------------|--------------|---------------|
| Local | `npm run seed` | `npm run stats` |
| Dev | `npm run seed:dev` | `npm run stats:dev` |
| Production | (via CI only) | `npm run stats:prod` |

## Detailed Commands

### Validation

```bash
# Quick validation (metadata only)
npm run validate

# Verbose validation (show all issues)
npm run validate:verbose

# Validate specific directory
npx tsx src/schema-validation.ts ../data/lamad/content --verbose
```

**What validation checks:**
- Required fields: `id`, `title`
- Format hints: `contentFormat`, `contentType`
- Blob references: `blobHash` integrity
- JSON syntax errors

### Seeding

```bash
# Full seed (DNA + blobs)
npm run seed

# Validate only (no writes)
npm run seed:validate

# Preview changes (dry run)
npm run seed:dry-run

# Skip blob upload (DNA entries only)
npm run seed:dna-only

# Skip DNA entries (blobs only)
npm run seed:blobs-only
```

### Statistics & Verification

```bash
# Show content counts
npm run stats

# Against dev environment
npm run stats:dev

# Against production
npm run stats:prod
```

### Migration & Recovery

```bash
# Run migrations
npm run migrate

# Preview migrations
npm run migrate:dry-run

# Verify migration state
npm run migrate:verify
```

### Snapshot Management

```bash
# Create snapshot of current state
npm run snapshot:create

# Save snapshot with name
npm run snapshot:save

# Restore from snapshot
npm run snapshot:restore

# List available snapshots
npm run snapshot:list

# Check snapshot status
npm run snapshot:status

# Clean old snapshots
npm run snapshot:clean
```

## Data Directory Structure

```
genesis/data/lamad/
├── content/           # ContentNode JSON files
│   ├── manifesto.json
│   ├── governance-epic.json
│   └── ...
├── paths/             # Learning path definitions
│   ├── index.json
│   ├── elohim-protocol.json
│   └── ...
├── assessments/       # Quiz/assessment files
│   └── ...
├── graph/             # Visualization data
│   └── ...
└── perseus/           # Interactive quiz format
    └── ...
```

## Environment Configuration

### Local Development
```bash
# Uses localhost doorway (started via hc:start)
npm run seed
```

### Dev Environment
```bash
# Preset environment variables
npm run seed:dev

# Or manually:
export DOORWAY_URL='https://doorway-dev.elohim.host'
export DOORWAY_API_KEY='dev-elohim-auth-2024'
export HOLOCHAIN_ADMIN_URL='wss://doorway-dev.elohim.host?apiKey=dev-elohim-auth-2024'
npm run seed
```

### Production
Production seeding is done via CI pipeline only. Use `stats:prod` for read-only verification.

## Troubleshooting

### Validation Failures

**Missing required field:**
```
ERROR: content/some-file.json missing required field 'id'
```
Fix: Add the missing field to the JSON file.

**Invalid JSON:**
```
ERROR: content/some-file.json - Unexpected token
```
Fix: Check JSON syntax (trailing commas, missing quotes).

### Connection Issues

**WebSocket timeout:**
```bash
# Check doorway is running
curl http://localhost:8888/health

# Check conductor
hc sandbox call --running $(cat /projects/elohim/holochain/local-dev/.hc_ports | grep admin_port | grep -o '[0-9]*') list-apps
```

**Auth failure (remote):**
```bash
# Verify API key
echo $DOORWAY_API_KEY

# Test connection
curl -H "X-API-Key: $DOORWAY_API_KEY" https://doorway-dev.elohim.host/health
```

### Partial Seed Failures

If seeding fails partway through:
```bash
# Check what was seeded
npm run stats

# Resume (idempotent - will skip existing)
npm run seed

# Or restore from snapshot and retry
npm run snapshot:restore
npm run seed
```

### Large Seed Operations

For bulk seeding:
```bash
# Create snapshot first (for rollback)
npm run snapshot:create

# Seed DNA entries first (faster)
npm run seed:dna-only

# Then blobs (can be slow)
npm run seed:blobs-only
```

## Pre-Seed Validation with hc-rna

For comprehensive schema validation using the Rust tooling:

```bash
cd /projects/elohim/holochain/rna/rust

# Analyze all seed data
RUSTFLAGS="" cargo run --features cli --bin hc-rna-fixtures -- \
  -f /projects/elohim/genesis/data/lamad/content \
  --analyze

# Specific directory
RUSTFLAGS="" cargo run --features cli --bin hc-rna-fixtures -- \
  -f /projects/elohim/genesis/data/lamad/paths \
  --analyze
```

## Key Files

| File | Purpose |
|------|---------|
| `genesis/seeder/src/seed-production.ts` | Main seeding orchestrator |
| `genesis/seeder/src/schema-validation.ts` | JSON schema validation |
| `genesis/seeder/src/verification.ts` | Post-seed verification |
| `genesis/seeder/src/snapshot.ts` | Snapshot management |
| `genesis/seeder/src/doorway-client.ts` | HTTP client for doorway |
| `genesis/seeder/src/storage-client.ts` | Blob upload client |

## Common Workflows

### Add New Content
```bash
# 1. Create content file
# genesis/data/lamad/content/new-content.json

# 2. Validate
npm run validate

# 3. Dry run
npm run seed:dry-run

# 4. Seed
npm run seed
```

### Update Existing Content
```bash
# 1. Edit content file
# 2. Validate
npm run validate

# 3. Seed (idempotent update)
npm run seed

# 4. Verify
npm run stats
```

### Disaster Recovery
```bash
# 1. List available snapshots
npm run snapshot:list

# 2. Restore
npm run snapshot:restore

# 3. Verify
npm run stats
```
