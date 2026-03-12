# Doorway App (Angular)

Operator dashboard for doorway stewards. See `../CLAUDE.md` for the shared doorway architecture, routing model, and federation concepts.

## What This App Does

Gives doorway stewards visibility into what they are stewarding: nodes, hosted humans, trust relationships, federation peers, and the graduation pipeline from hosted accounts to full P2P agency.

## Build

```bash
cd doorway/doorway-app
pnpm install                  # Or from repo root (workspace install)
pnpm start                    # Dev server at localhost:4200
pnpm run build                # Production build
pnpm exec eslint src --ext .ts,.html
pnpm run format:check         # Prettier check
```

The app connects to a running doorway Rust service for its `/admin/*` and `/auth/*` API endpoints.

## Routes

| Path | View |
|------|------|
| `/` | Landing page |
| `/dashboard` | Operator dashboard (nodes, cluster, resources, federation, graduation) |
| `/login` | Threshold login |
| `/register` | Account creation |
| `/doorways` | Doorway browser (select gateway) |
| `/account` | User account and quota status |

## Key Services

| Service | Purpose |
|---------|---------|
| `doorway-admin.service.ts` | Calls `/admin/*` endpoints for node health, cluster metrics, federation peers |
| `doorway-auth.service.ts` | Login/logout, JWT management |
| `doorway.model.ts` | TypeScript types for nodes, federation peers, capabilities |

## Adding Admin Endpoints

When adding new admin API endpoints:
1. Add the handler in `doorway-service/src/routes/admin.rs`
2. Add the corresponding service method in `doorway-app/src/app/services/doorway-admin.service.ts`
3. Add TypeScript types in `doorway-app/src/app/models/doorway.model.ts`

Admin routes are doorway infrastructure — they are NOT subject to the dynamic route registry. They stay as explicit handlers in the Rust service.
