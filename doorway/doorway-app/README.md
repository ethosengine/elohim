# doorway-app

Operator dashboard for doorway stewards. In the Elohim Protocol, doorways are community-run gateways that bridge hosted users to the distributed network. This Angular app gives the humans who run those gateways visibility into what they are stewarding -- the nodes, the people served, the trust relationships, and the path toward full agency.

## Features

- **Operator Dashboard** -- Real-time node health, cluster metrics, resource utilization, and custodian network status via WebSocket. Displays both technical metrics (CPU, memory, storage, bandwidth) and human-scale metrics (trust scores, humans served, impact scores, steward tiers).
- **Federation** -- View federated doorway peers and P2P connections across the network.
- **Graduation Pipeline** -- Track people on their way from hosted accounts to stewardship of their own keys and devices (registered, hosted, graduating, steward). Graduation is the doorway's purpose: a hosted account is a starting point, not a destination.
- **Hosted Accounts** -- Care for hosted accounts, quotas, and permissions while people are still on the doorway's keys.
- **Account Self-Service** -- Users can view their own agency pipeline progress and usage.

## Routes

| Path | View |
|------|------|
| `/` | Landing page |
| `/dashboard` | Operator dashboard (nodes, cluster, resources, federation, graduation) |
| `/login` | Threshold login |
| `/register` | Account creation |
| `/doorways` | Doorway browser (select gateway) |
| `/account` | User account and quota status |

## Development

```bash
cd doorway-app
npm install
npm start          # Dev server at localhost:4200
npm run build      # Production build
npm run lint       # ESLint
npm run format:check  # Prettier check
```

The app connects to a running doorway Rust service for its `/admin/*` and `/auth/*` API endpoints.
