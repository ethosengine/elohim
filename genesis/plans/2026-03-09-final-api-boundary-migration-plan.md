# Final API Boundary Migration — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Delete the last 3 fat Angular services (2,900 lines of direct Holochain zome calls), replacing them with 4 thin API services backed by Rust controllers in elohim-storage.

**Architecture:** Each fat service is replaced by: (1) DB tables in elohim-storage SQLite via schema migration v6→v7, (2) View types in `views.rs` with `ts-rs` export, (3) API handler in `src/api/`, (4) doorway proxy route, (5) Angular thin API service with InjectionToken. The proven pattern from the 15 already-migrated services is followed exactly.

**Tech Stack:** Rust (diesel, hyper, serde, ts-rs), Angular 19 (HttpClient, InjectionToken), SQLite

**Design doc:** `genesis/plans/2026-03-09-final-api-boundary-migration-design.md`

---

## Task 1: Schema Migration v6→v7 — Governance Tables

**Files:**
- Modify: `holochain/elohim-storage/src/db/schema.rs` (add migration v6→v7, new tables)
- Modify: `holochain/elohim-storage/src/db/diesel_schema.rs` (add diesel table! macros)
- Modify: `holochain/elohim-storage/src/db/models.rs` (add model structs)
- Modify: `holochain/elohim-storage/src/db/mod.rs` (add module declarations)
- Create: `holochain/elohim-storage/src/db/governance.rs` (CRUD queries)

**Step 1: Add 5 governance tables to schema.rs migration**

In `migrate_schema()`, add v6→v7 case. Tables:
- `governance_states` (id TEXT PK, entity_type TEXT, entity_id TEXT, reach TEXT, labels TEXT json, voting_state TEXT, signal_count INTEGER, created_at TEXT, updated_at TEXT)
- `challenges` (id TEXT PK, content_id TEXT, challenger_presence_id TEXT, reason TEXT, status TEXT, evidence TEXT json, created_at TEXT, updated_at TEXT)
- `proposals` (id TEXT PK, content_id TEXT, proposer_presence_id TEXT, proposal_type TEXT, title TEXT, body TEXT, status TEXT, votes_for INTEGER, votes_against INTEGER, created_at TEXT, updated_at TEXT)
- `precedents` (id TEXT PK, content_id TEXT, principle TEXT, interpretation TEXT, established_by TEXT, created_at TEXT)
- `discussions` (id TEXT PK, content_id TEXT, author_presence_id TEXT, body TEXT, parent_id TEXT nullable, created_at TEXT, updated_at TEXT)

Bump `SCHEMA_VERSION` to 7.

Also add to `create_tables()` for fresh installs.

**Step 2: Add diesel table! macros to diesel_schema.rs**

Add `diesel::table!` blocks for all 5 tables matching the SQL columns.

**Step 3: Add model structs to models.rs**

Add `#[derive(Queryable, Insertable)]` structs for each table: `GovernanceState`, `Challenge`, `Proposal`, `Precedent`, `Discussion`, plus `New*` insert structs.

**Step 4: Create db/governance.rs with CRUD queries**

Pattern: follow `db/contributor_presences.rs`. Functions:
- `get_governance_state(conn, entity_type, entity_id) -> Option<GovernanceState>`
- `query_governance_states(conn, entity_type) -> Vec<GovernanceState>`
- `get_challenge(conn, id) -> Option<Challenge>`
- `query_challenges(conn, content_id) -> Vec<Challenge>`
- `get_proposal(conn, id) -> Option<Proposal>`
- `query_proposals(conn, content_id, status) -> Vec<Proposal>`
- `get_precedent(conn, id) -> Option<Precedent>`
- `query_precedents(conn, content_id) -> Vec<Precedent>`
- `get_discussion(conn, id) -> Option<Discussion>`
- `query_discussions(conn, content_id) -> Vec<Discussion>`

**Step 5: Register module in db/mod.rs**

Add `pub mod governance;` and re-export.

**Step 6: Build and verify**

Run: `cd holochain/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release`
Expected: compiles cleanly.

**Step 7: Commit**

```
git add holochain/elohim-storage/src/db/
git commit -m "feat(storage): add governance tables in schema v7 migration"
```

---

## Task 2: Schema Migration v7 — Attestation, Steward, Contributor Tables

**Files:**
- Modify: `holochain/elohim-storage/src/db/schema.rs` (add remaining tables to v7 migration)
- Modify: `holochain/elohim-storage/src/db/diesel_schema.rs`
- Modify: `holochain/elohim-storage/src/db/models.rs`
- Modify: `holochain/elohim-storage/src/db/mod.rs`
- Create: `holochain/elohim-storage/src/db/content_attestations.rs`
- Create: `holochain/elohim-storage/src/db/steward_operations.rs`
- Create: `holochain/elohim-storage/src/db/contributors.rs`

**Step 1: Add 5 tables to the v7 migration in schema.rs**

Tables:
- `content_attestations` (id TEXT PK, content_id TEXT, attestor_presence_id TEXT, scope TEXT, attestation_type TEXT, evidence TEXT json, grantor TEXT json, is_revoked INTEGER DEFAULT 0, revocation TEXT json nullable, created_at TEXT, updated_at TEXT)
- `steward_credentials` (id TEXT PK, presence_id TEXT, content_id TEXT, affinity_coefficient REAL, credential_type TEXT, status TEXT, created_at TEXT, updated_at TEXT)
- `premium_gates` (id TEXT PK, steward_credential_id TEXT, steward_presence_id TEXT, gated_resource_type TEXT, gated_resource_ids TEXT json, gate_title TEXT, gate_description TEXT nullable, created_at TEXT)
- `access_grants` (id TEXT PK, gate_id TEXT, grantee_presence_id TEXT, contributor_presence_id TEXT nullable, granted_at TEXT, expires_at TEXT nullable, status TEXT)
- `contributor_dashboards` (presence_id TEXT PK, total_contributions INTEGER, total_recognitions INTEGER, impact_score REAL, last_contribution_at TEXT nullable, updated_at TEXT)

**Step 2: Add diesel table! macros**

**Step 3: Add model structs**

For each table: `ContentAttestation`, `StewardCredential`, `PremiumGate`, `AccessGrant`, `ContributorDashboard` + `New*` inserts.

**Step 4: Create db/content_attestations.rs**

Functions: `create_attestation`, `revoke_attestation`, `get_attestation`, `query_attestations_for_content`, `query_attestations_by_attestor`, `query_attestations_by_content_ids`.

**Step 5: Create db/steward_operations.rs**

Functions: `create_credential`, `get_credential`, `query_credentials_for_presence`, `query_credentials_for_content`, `create_gate`, `get_gate`, `query_gates_for_resource`, `create_grant`, `get_grant`, `query_grants_for_gate`, `check_access`, `get_revenue_summary`.

**Step 6: Create db/contributors.rs**

Functions: `get_dashboard`, `get_my_dashboard`, `get_impact`, `get_recognition_history`.

**Step 7: Register modules in db/mod.rs**

**Step 8: Build and verify**

Run: `cd holochain/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release`

**Step 9: Commit**

```
git add holochain/elohim-storage/src/db/
git commit -m "feat(storage): add attestation, steward, and contributor tables to v7 migration"
```

---

## Task 3: View Types + TypeScript Generation

**Files:**
- Modify: `holochain/elohim-storage/src/views.rs` (add View + InputView types)

**Step 1: Add governance view types**

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct GovernanceStateView { /* from GovernanceState model */ }

// Same for ChallengeView, ProposalView, PrecedentView, DiscussionView
```

Add `From<Model>` impls with `parse_json_opt` for JSON fields, `== 1` for booleans.

**Step 2: Add attestation view types**

`ContentAttestationView` with `From<ContentAttestation>`, plus `CreateAttestationInputView` and `RevokeAttestationInputView`.

**Step 3: Add steward view types**

`StewardCredentialView`, `PremiumGateView`, `AccessGrantView`, `StewardRevenueSummaryView` with From impls. Input views for create operations.

**Step 4: Add contributor view types**

`ContributorDashboardView`, `ContributorImpactView`, `ContributorRecognitionView` with From impls.

**Step 5: Generate TypeScript types**

Run: `cd holochain/elohim-storage && cargo test export_bindings`
Verify: check `holochain/sdk/storage-client-ts/src/generated/` for new `.ts` files.

**Step 6: Commit**

```
git add holochain/elohim-storage/src/views.rs holochain/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): add view types for governance, attestation, steward, contributor"
```

---

## Task 4: Governance API Handler (storage)

**Files:**
- Create: `holochain/elohim-storage/src/api/governance.rs`
- Modify: `holochain/elohim-storage/src/api/mod.rs` (register module + route)

**Step 1: Create governance.rs handler**

Pattern: copy `src/api/presence.rs` structure. Routes:
- `GET /api/v1/governance/state?entityType=X&entityId=Y` → `get_governance_state`
- `GET /api/v1/governance/states?entityType=X` → `query_governance_states`
- `GET /api/v1/governance/challenges?contentId=X` → `query_challenges`
- `GET /api/v1/governance/challenges/{id}` → `get_challenge`
- `GET /api/v1/governance/proposals?contentId=X&status=Y` → `query_proposals`
- `GET /api/v1/governance/proposals/{id}` → `get_proposal`
- `GET /api/v1/governance/precedents?contentId=X` → `query_precedents`
- `GET /api/v1/governance/precedents/{id}` → `get_precedent`
- `GET /api/v1/governance/discussions?contentId=X` → `query_discussions`
- `GET /api/v1/governance/discussions/{id}` → `get_discussion`

Use `GovernanceService` (create in `src/services/governance_service.rs`) for business logic between handler and DB.

**Step 2: Register in api/mod.rs**

Add `pub mod governance;` and add route match for `/api/v1/governance`.

**Step 3: Build and verify**

Run: `cd holochain/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release`

**Step 4: Commit**

```
git add holochain/elohim-storage/src/api/governance.rs holochain/elohim-storage/src/api/mod.rs
git commit -m "feat(storage): add governance API handler"
```

---

## Task 5: Attestation, Steward, Contributor API Handlers (storage)

**Files:**
- Create: `holochain/elohim-storage/src/api/attestations.rs`
- Create: `holochain/elohim-storage/src/api/steward.rs`
- Create: `holochain/elohim-storage/src/api/contributors.rs`
- Modify: `holochain/elohim-storage/src/api/mod.rs`

**Step 1: Create attestations.rs handler**

Routes:
- `POST /api/v1/attestations` → create attestation
- `POST /api/v1/attestations/{id}/revoke` → revoke attestation
- `GET /api/v1/attestations?contentId=X` → query by content
- `GET /api/v1/attestations?attestorId=X` → query by attestor
- `GET /api/v1/attestations/{id}` → get by id

**Step 2: Create steward.rs handler**

Routes:
- `POST /api/v1/steward/credentials` → create credential
- `GET /api/v1/steward/credentials/{id}` → get credential
- `GET /api/v1/steward/credentials?presenceId=X` → query by presence
- `GET /api/v1/steward/credentials?contentId=X` → query by content
- `POST /api/v1/steward/gates` → create gate
- `GET /api/v1/steward/gates/{id}` → get gate
- `GET /api/v1/steward/gates?resourceType=X` → query gates
- `POST /api/v1/steward/grants` → create grant
- `GET /api/v1/steward/grants/{id}` → get grant
- `GET /api/v1/steward/grants?gateId=X` → query grants
- `GET /api/v1/steward/access?gateId=X&granteeId=Y` → check access
- `GET /api/v1/steward/revenue/{presenceId}` → revenue summary

**Step 3: Create contributors.rs handler**

Routes:
- `GET /api/v1/contributors/{id}/dashboard` → get dashboard
- `GET /api/v1/contributors/me/dashboard` → get my dashboard
- `GET /api/v1/contributors/{id}/impact` → get impact
- `GET /api/v1/contributors/{id}/recognition` → get recognition history

**Step 4: Register all in api/mod.rs**

**Step 5: Build and verify**

Run: `cd holochain/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release`

**Step 6: Commit**

```
git add holochain/elohim-storage/src/api/
git commit -m "feat(storage): add attestation, steward, and contributor API handlers"
```

---

## Task 6: Doorway Proxy Routes

**Files:**
- Create: `doorway/src/routes/governance.rs`
- Create: `doorway/src/routes/attestations.rs`
- Create: `doorway/src/routes/steward.rs`
- Create: `doorway/src/routes/contributors.rs`
- Modify: `doorway/src/routes/mod.rs`
- Modify: `doorway/src/server/http.rs` (add route matches)

**Step 1: Create 4 proxy route files**

Each follows the exact pattern from `doorway/src/routes/presence.rs`:
- `handle_*_request(req, state, path)` function
- `forward_to_storage(req, storage_url, path)` helper
- Query param passthrough
- Method forwarding (GET, POST, PUT, DELETE)
- `service_unavailable()` fallback when no STORAGE_URL

**Step 2: Register in mod.rs**

Add `pub mod governance;`, `pub mod attestations;`, `pub mod steward;`, `pub mod contributors;`.

**Step 3: Add route matches in server/http.rs**

Add match arms in the request router (near line ~1206 where presence/stewardship/etc are matched):

```rust
(_, p) if p.starts_with("/api/v1/governance") => {
    routes::governance::handle_governance_request(req, state.clone(), p).await
}
(_, p) if p.starts_with("/api/v1/attestations") => {
    routes::attestations::handle_attestations_request(req, state.clone(), p).await
}
(_, p) if p.starts_with("/api/v1/steward") => {
    routes::steward::handle_steward_request(req, state.clone(), p).await
}
(_, p) if p.starts_with("/api/v1/contributors") => {
    routes::contributors::handle_contributors_request(req, state.clone(), p).await
}
```

**Step 4: Build and verify**

Run: `cd doorway && RUSTFLAGS="" cargo build --release`

**Step 5: Commit**

```
git add doorway/src/routes/ doorway/src/server/http.rs
git commit -m "feat(doorway): add proxy routes for governance, attestation, steward, contributor APIs"
```

---

## Task 7: Angular Interfaces + InjectionTokens

**Files:**
- Create: `elohim-app/src/app/elohim/interfaces/governance.interface.ts`
- Create: `elohim-app/src/app/elohim/interfaces/content-attestation.interface.ts`
- Create: `elohim-app/src/app/lamad/interfaces/steward.interface.ts`
- Create: `elohim-app/src/app/lamad/interfaces/contributor.interface.ts`

**Step 1: Create IGovernance interface**

```typescript
import { InjectionToken } from '@angular/core';

export interface IGovernance {
  getGovernanceState(entityType: string, entityId: string): Promise<GovernanceStateView | null>;
  queryGovernanceStates(entityType: string): Promise<GovernanceStateView[]>;
  getChallengeById(id: string): Promise<ChallengeView | null>;
  queryChallenges(contentId: string): Promise<ChallengeView[]>;
  getProposalById(id: string): Promise<ProposalView | null>;
  queryProposals(contentId: string, status?: string): Promise<ProposalView[]>;
  getPrecedentById(id: string): Promise<PrecedentView | null>;
  queryPrecedents(contentId: string): Promise<PrecedentView[]>;
  getDiscussionById(id: string): Promise<DiscussionView | null>;
  queryDiscussions(contentId: string): Promise<DiscussionView[]>;
}

export const GOVERNANCE = new InjectionToken<IGovernance>('Governance');
```

**Step 2: Create IContentAttestation interface**

Methods: `createAttestation`, `revokeAttestation`, `getAttestation`, `queryAttestationsForContent`, `queryAttestationsByAttestor`.

**Step 3: Create ISteward interface**

Methods: `createCredential`, `getCredential`, `queryCredentialsForPresence`, `queryCredentialsForContent`, `createGate`, `getGate`, `queryGatesForResource`, `createGrant`, `getGrant`, `queryGrantsForGate`, `checkAccess`, `getRevenueSummary`.

**Step 4: Create IContributor interface**

Methods: `getDashboard`, `getMyDashboard`, `getImpact`, `getRecognitionHistory`.

**Step 5: Commit**

```
git add elohim-app/src/app/elohim/interfaces/ elohim-app/src/app/lamad/interfaces/
git commit -m "feat(elohim-app): add interfaces and InjectionTokens for governance, attestation, steward, contributor"
```

---

## Task 8: Angular Thin API Services

**Files:**
- Create: `elohim-app/src/app/elohim/services/governance-api.service.ts`
- Create: `elohim-app/src/app/elohim/services/content-attestation-api.service.ts`
- Create: `elohim-app/src/app/lamad/services/steward-api.service.ts`
- Create: `elohim-app/src/app/lamad/services/contributor-api.service.ts`

**Step 1: Create governance-api.service.ts**

Follow `presence-api.service.ts` pattern exactly:
- `@Injectable({ providedIn: 'root' })`
- `implements IGovernance`
- `private readonly http = inject(HttpClient)`
- Each method: `firstValueFrom(this.http.get/post(...).pipe(catchError(...)))`
- Base URL: `/api/v1/governance`

**Step 2: Create content-attestation-api.service.ts**

Same pattern. Base URL: `/api/v1/attestations`.

**Step 3: Create steward-api.service.ts**

Same pattern. Base URL: `/api/v1/steward`.

**Step 4: Create contributor-api.service.ts**

Same pattern. Base URL: `/api/v1/contributors`.

**Step 5: Commit**

```
git add elohim-app/src/app/elohim/services/governance-api.service.ts \
       elohim-app/src/app/elohim/services/content-attestation-api.service.ts \
       elohim-app/src/app/lamad/services/steward-api.service.ts \
       elohim-app/src/app/lamad/services/contributor-api.service.ts
git commit -m "feat(elohim-app): add thin API services for governance, attestation, steward, contributor"
```

---

## Task 9: Rewire Consumers + Delete Fat Services

**Files:**
- Modify: `elohim-app/src/app/elohim/services/data-loader.service.ts` (replace holochain-content dependency)
- Modify: `elohim-app/src/app/elohim/services/governance.service.ts` (inject governance-api instead of data-loader)
- Modify: All components/services that inject `steward.service` or `contributor.service`
- Delete: `elohim-app/src/app/elohim/services/holochain-content.service.ts` (1,568 lines)
- Delete: `elohim-app/src/app/lamad/services/steward.service.ts` (866 lines)
- Delete: `elohim-app/src/app/lamad/services/contributor.service.ts` (466 lines)
- Delete: corresponding `.spec.ts` files

**Step 1: Find all consumers**

Search for imports of the 3 fat services:
```bash
grep -r "holochain-content.service\|HolochainContentService" elohim-app/src/app --include='*.ts' -l
grep -r "steward.service\|StewardService" elohim-app/src/app --include='*.ts' -l
grep -r "contributor.service\|ContributorService" elohim-app/src/app --include='*.ts' -l
```

**Step 2: Rewire data-loader.service.ts**

Replace `HolochainContentService` injection with `GOVERNANCE` and `CONTENT_ATTESTATION` tokens. Update all methods that delegate to `this.holochainContent.*` to use the new thin API services instead.

**Step 3: Rewire governance.service.ts**

Replace `data-loader` governance delegation with direct `GOVERNANCE` token injection.

**Step 4: Rewire steward consumers**

Replace `StewardService` injection with `STEWARD` token in all consumers.

**Step 5: Rewire contributor consumers**

Replace `ContributorService` injection with `CONTRIBUTOR` token in all consumers.

**Step 6: Fold agent CRUD into identity-api.service**

The 3 agent zome calls (`getAgentByHumanId`, `getAllAgents`, `getAttestations`) → add to `IIdentityApi` interface and `IdentityApiService`.

**Step 7: Delete fat services and their specs**

```bash
rm elohim-app/src/app/elohim/services/holochain-content.service.ts
rm elohim-app/src/app/elohim/services/holochain-content.service.spec.ts
rm elohim-app/src/app/lamad/services/steward.service.ts
rm elohim-app/src/app/lamad/services/steward.service.spec.ts
rm elohim-app/src/app/lamad/services/contributor.service.ts
rm elohim-app/src/app/lamad/services/contributor.service.spec.ts
```

**Step 8: Update barrel exports**

Update `index.ts` files in elohim and lamad pillars to remove deleted services and export new API services.

**Step 9: Build and verify**

Run: `cd elohim-app && pnpm run build`
Expected: compiles cleanly with no references to deleted services.

**Step 10: Run tests**

Run: `cd elohim-app && pnpm test`
Expected: tests pass (some specs for deleted services will be gone, reducing count).

**Step 11: Commit**

```
git add -A
git commit -m "refactor(elohim-app): delete 3 fat services, rewire consumers to thin API boundary (-2,900 lines)"
```

---

## Task 10: Delete holochain-client.service (if zero consumers remain)

**Files:**
- Possibly delete: `elohim-app/src/app/elohim/services/holochain-client.service.ts` (1,034 lines)
- Possibly delete: `elohim-app/src/app/elohim/services/holochain-client.service.spec.ts`

**Step 1: Verify no consumers remain**

```bash
grep -r "HolochainClientService\|holochain-client.service" elohim-app/src/app --include='*.ts' -l
```

If only the service itself and its spec appear → delete both.

If integrity services (write-buffer, identity, blob-manager, session-migration) still import it → keep. Document which integrity services still need it.

**Step 2: Delete if safe**

**Step 3: Build and test**

**Step 4: Commit**

```
git commit -m "refactor(elohim): delete holochain-client.service — zero fat consumers remain"
```

---

## Task 11: Verify on Alpha

**Step 1: Deploy and run a2o tests**

```bash
cd genesis/a2o
E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host npx cucumber-js -p alpha --tags '@e2e and not @wip and not @browser-only'
```

**Step 2: Probe new endpoints**

```bash
curl -s https://doorway-alpha.elohim.host/api/v1/governance/states?entityType=content
curl -s https://doorway-alpha.elohim.host/api/v1/attestations
curl -s https://doorway-alpha.elohim.host/api/v1/steward/credentials
curl -s https://doorway-alpha.elohim.host/api/v1/contributors/me/dashboard
```

**Step 3: Run final audit**

Verify: 0 fat services, 19 thin API services, holochain-client.service deleted or documented.

**Step 4: Commit verification results**

```
git commit -m "test(a2o): verify final API boundary migration on alpha"
```
