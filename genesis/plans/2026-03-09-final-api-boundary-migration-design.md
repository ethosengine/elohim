# Final API Boundary Migration Design

## Context

The v0 Phase 3 migration moved business logic from Angular "fat" services (direct Holochain zome calls) to Rust controllers in elohim-storage, with doorway as a transparent proxy and Angular consuming thin API clients via InjectionTokens.

**Progress so far**: 15 thin API services migrated, ~10,700 lines of fat services eliminated (61%). Three fat services remain with ~2,900 lines and 34 zome calls.

## What Remains

### holochain-content.service (1,568 lines, 23 zome calls) — DELETE

Three domains in one service:

| Domain | Calls | Notes |
|--------|------:|-------|
| Content reads | 3 | Dead code — already served by `/db/content` projection |
| Attestation CRUD | 7 | Trust claims on content (create, revoke, query) |
| Governance | 10 | Challenges, proposals, precedents, discussions, governance state |
| Agent CRUD | 3 | Fold into existing `identity-api.service` |

Consumers: `data-loader.service`, `governance.service`, `content-viewer.component`

### steward.service (866 lines, 12 zome calls) — DELETE

| Domain | Calls | Notes |
|--------|------:|-------|
| Steward credentials | 4 | Stewardship positions + affinity coefficients |
| Premium gates | 3 | Content access gating |
| Access grants | 3 | Granted access records |
| Revenue summary | 1 | Steward reward aggregation |
| Access check | 1 | Existing access verification |

Steward credentials determine who stewards content at what affinity level. This drives reward distribution to stewards via governance mechanisms — it's economic/governance data, not cryptographic verification.

### contributor.service (466 lines, 5 zome calls) — DELETE

| Domain | Calls | Notes |
|--------|------:|-------|
| Dashboard queries | 3 | Aggregated contributor metrics |
| Impact query | 1 | Reporting |
| Recognition history | 1 | Reporting |

All read-only aggregations — belongs in the projection layer.

## Design

### New Thin API Services (Angular)

| Service | Interface | Route | Replaces |
|---------|-----------|-------|----------|
| `governance-api.service` | `IGovernance` | `/api/v1/governance` | 10 governance zome calls |
| `content-attestation-api.service` | `IContentAttestation` | `/api/v1/attestations` | 7 attestation zome calls |
| `steward-api.service` | `ISteward` | `/api/v1/steward` | 12 steward zome calls |
| `contributor-api.service` | `IContributor` | `/api/v1/contributors` | 5 contributor zome calls |

Each follows the established pattern: `@Injectable`, implements interface, uses `HttpClient`, provided via `InjectionToken`.

### Rust Layer (elohim-storage)

All tables added to elohim-storage's existing SQLite schema (diesel).

#### New DB Tables

| Table | Domain | Key Columns |
|-------|--------|-------------|
| `governance_states` | Per-entity governance | entity_type, entity_id, reach, labels, voting_state |
| `challenges` | Content challenges | content_id, challenger_id, reason, status |
| `proposals` | Governance proposals | content_id, proposer_id, proposal_type, status |
| `precedents` | Constitutional precedents | content_id, principle, interpretation |
| `discussions` | Content discussions | content_id, author_id, body, parent_id |
| `content_attestations` | Trust attestations | content_id, attestor_id, scope, evidence, revoked |
| `steward_credentials` | Stewardship positions | presence_id, content_id, affinity_coefficient, credential_type |
| `premium_gates` | Content access gates | resource_type, resource_ids, gate_title, credential_id |
| `access_grants` | Granted access | gate_id, grantee_id, granted_at, expires_at |
| `contributor_dashboards` | Contributor metrics | presence_id, total_contributions, impact_score (materialized view) |

#### New API Handlers (src/api/)

| Handler | Endpoints |
|---------|-----------|
| `governance.rs` | GET/POST challenges, proposals, precedents, discussions; GET governance state |
| `attestations.rs` | POST create/revoke attestation; GET query attestations |
| `steward.rs` | CRUD credentials, gates, grants; GET revenue summary |
| `contributors.rs` | GET dashboard, impact, recognition |

#### New Doorway Proxy Routes (src/routes/)

| Route | Pattern |
|-------|---------|
| `governance.rs` | `/api/v1/governance/*` → storage `/api/v1/governance/*` |
| `attestations.rs` | `/api/v1/attestations/*` → storage `/api/v1/attestations/*` |
| `steward.rs` | `/api/v1/steward/*` → storage `/api/v1/steward/*` |
| `contributors.rs` | `/api/v1/contributors/*` → storage `/api/v1/contributors/*` |

### Consumer Rewiring

| Consumer | Current Dependency | New Dependency |
|----------|-------------------|----------------|
| `data-loader.service` | `holochain-content.service` | `governance-api`, `content-attestation-api` via tokens |
| `governance.service` | `data-loader` → `holochain-content` | `governance-api` directly |
| `content-viewer.component` | `governance.service` | No change (governance.service rewires internally) |
| Various steward consumers | `steward.service` | `steward-api` via `STEWARD_OPERATIONS` token |
| Various contributor consumers | `contributor.service` | `contributor-api` via `CONTRIBUTOR` token |

### Agent CRUD (3 remaining calls)

The 3 agent zome calls (`getAgentByHumanId`, `getAllAgents`, `getAttestations`) fold into `identity-api.service` as new methods, extending its `IIdentityApi` interface.

## Post-Migration State

### Deleted
- `holochain-content.service` (1,568 lines)
- `steward.service` (866 lines)
- `contributor.service` (466 lines)
- `holochain-client.service` (1,034 lines) — no consumers remain
- **Total: ~3,934 lines deleted**

### Created
- 4 thin API services (~600 lines total)
- 4 Rust API handlers
- 4 doorway proxy routes
- 10 DB tables + diesel schema migration

### Final Architecture
- **Zero fat services** — no direct zome calls from Angular
- **19 thin API services** behind InjectionTokens
- **5 integrity services** correctly client-side (write-buffer, identity, blob-manager, doorway-registry, session-migration)
- **3 integrity anchors** in `elohim/integrity/` (blob-metadata, federation-registry, node-registry)
- `holochain-client.service` deletable (zero consumers)

### Migration Complete
```
Thin API:   ███████████████████████████████  19/19 data-boundary services (100%)
Fat lines:  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  0 remaining (was ~17,500)
```
