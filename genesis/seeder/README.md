# Genesis Seeder

Seeds pre-structured JSON content from `/genesis/data/lamad` into Holochain.

Part of the **Genesis project** - the meta-infrastructure layer for source → seed → validate → feedback.

## Pipeline

```
genesis/docs/ → Claude + MCP → genesis/data/lamad/ → genesis/seeder → Holochain DHT
```

## Quick Start

### Local Development

Seed to local Holochain sandbox:

```bash
# From elohim-app directory
npm run hc:seed              # Full seed
npm run hc:seed:sample       # Sample (10 items)

# Or from seeder directory
cd genesis/seeder
npm run seed                 # Full seed
npm run seed:sample          # Sample (10 items)
```

### Remote Seeding

Seed to deployed Holochain conductors:

```bash
# Dev environment (holochain-dev.elohim.host)
npm run hc:seed:dev          # Full seed
npm run hc:seed:dev:sample   # Sample (10 items)
npm run hc:stats:dev         # View stats

# Production (holochain.elohim.host)
# Requires ELOHIM_PROD_API_KEY environment variable
export ELOHIM_PROD_API_KEY="your-production-api-key"
npm run hc:seed:prod
npm run hc:stats:prod
```

### Manual Remote Seeding

For custom configurations:

```bash
# Only HOLOCHAIN_ADMIN_URL is required for remote seeding
# The seeder automatically builds the app URL via /app/:port routing
HOLOCHAIN_ADMIN_URL="wss://holochain-dev.elohim.host?apiKey=dev-elohim-auth-2024" \
  npx tsx src/seed.ts
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `HOLOCHAIN_ADMIN_URL` | Admin WebSocket URL | `ws://localhost:4444` (from .hc_ports) |
| `HOLOCHAIN_APP_URL` | App WebSocket URL (optional) | Auto-resolved from admin URL |
| `DATA_DIR` | Path to lamad data | `/projects/elohim/genesis/data/lamad` |
| `LOCAL_DEV_DIR` | Local dev directory | `/projects/elohim/holochain/local-dev` |
| `HC_PORTS_FILE` | Ports file for local dev | `$LOCAL_DEV_DIR/.hc_ports` |

## Remote URL Resolution

When connecting to remote Holochain instances through Doorway:

1. Connect to admin interface: `wss://hostname?apiKey=...`
2. Seeder calls `listAppInterfaces()` to get dynamic port (e.g., 43733)
3. Seeder builds app URL: `wss://hostname/app/43733?apiKey=...`
4. Doorway routes `/app/:port` to `ws://localhost:port` inside pod

**Important:** Do NOT set `HOLOCHAIN_APP_URL` for remote seeding. The seeder's
`resolveAppUrl()` function automatically builds the correct URL with the dynamic
port returned by the conductor.

## Available Scripts

### Seeding

| Script | Description |
|--------|-------------|
| `seed` | Full seed to local conductor |
| `seed:sample` | Sample seed (10 items) to local |
| `seed:dev` | Full seed to dev environment |
| `seed:dev:sample` | Sample seed to dev environment |
| `seed:prod` | Full seed to production |

### Statistics

| Script | Description |
|--------|-------------|
| `stats` | Show stats from local conductor |
| `stats:dev` | Show stats from dev environment |
| `stats:prod` | Show stats from production |

### Snapshots (Local Only)

| Script | Description |
|--------|-------------|
| `snapshot:save` | Save current conductor state |
| `snapshot:restore` | Restore from snapshot |
| `snapshot:status` | Show snapshot status |
| `snapshot:list` | List available snapshots |

### Migrations

| Script | Description |
|--------|-------------|
| `migrate` | Run DNA migrations |
| `migrate:dry-run` | Preview migrations |
| `migrate:verify` | Verify migration state |

## Data Structure

The seeder expects JSON files in:

```
genesis/data/lamad/
├── content/           # Concepts and content nodes
│   ├── *.json
│   └── epic/
│       └── *.json
└── paths/             # Learning paths
    └── *.json
```

### Concept JSON Schema

```json
{
  "id": "unique-concept-id",
  "title": "Display Title",
  "content": "Markdown content...",
  "contentFormat": "markdown",
  "contentType": "concept",
  "description": "Brief description",
  "summary": "Card preview text",
  "tags": ["tag1", "tag2"],
  "relatedNodeIds": ["other-concept-id"],
  "estimatedMinutes": 5,
  "thumbnailUrl": "https://..."
}
```

### Path JSON Schema

```json
{
  "id": "path-id",
  "title": "Learning Path Title",
  "description": "Path description",
  "difficulty": "beginner",
  "estimatedDuration": "2 hours",
  "chapters": [
    {
      "id": "chapter-1",
      "title": "Chapter Title",
      "modules": [
        {
          "id": "module-1",
          "title": "Module Title",
          "sections": [
            {
              "id": "section-1",
              "title": "Section Title",
              "conceptIds": ["concept-1", "concept-2"]
            }
          ]
        }
      ]
    }
  ]
}
```

## Troubleshooting

### Connection Errors

**Error:** `UnknownMessageType: incoming message has unknown type - error`

This usually means the seeder is connecting to the wrong interface. Make sure:
- Only `HOLOCHAIN_ADMIN_URL` is set (not `HOLOCHAIN_APP_URL`)
- API key is included in the URL: `?apiKey=...`

**Error:** `Failed to connect to admin WebSocket`

- Check the conductor is running
- Verify the URL and port
- For remote: ensure Doorway gateway is deployed and healthy

### Auth Errors

**Error:** `401 Unauthorized`

- Verify API key is correct
- Check the key has appropriate permissions (authenticated vs admin)

### Port Range Errors

**Error:** `Invalid route` or connection rejected

- Doorway has a port range limit (4445-65535)
- Ensure the conductor's app interface port is within range
