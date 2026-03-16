# Sprint 7: Polis Sensemaking — Opinion Clustering & Bridging Statements (v2)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build Layer C — the Polis-inspired sensemaking layer. Accumulated governance signals (from Sprint 6) are clustered into opinion groups, bridging statements are surfaced, and the elohim synthesizes justified brackets for structured deliberation.

**Architecture:** Polis is the sensing organ; elohim is the deliberative mind. This sprint implements the sensing: signal → statement → cluster → bridge → bracket. The elohim synthesizing brackets is a seam for Sprint 9's inference integration.

**What already exists:**
- `OpinionClusterComponent` (727 lines) — Canvas-based 2D scatter visualization with PCA, cluster identification, consensus/divisive statement highlighting. Uses `GovernanceSignalService.OpinionCluster` types. Currently renders from in-memory data (no real backend).
- `governance-deliberation.model.ts` — Rich models: `SensemakingContext`, `OpinionCluster`, `ConsensusStatement`, `DivisiveStatement`, `Statement`, `StatementVote`, `ClusterVisualizationData`
- `SignalAccumulationService` (Sprint 6) — Detects `readyForSensemaking` threshold
- `FeedbackMechanismGateway` (Sprint 4) — Shows "sensemaking available" badge when threshold crossed

**What's missing:**
- Backend: statements + statement_votes tables, clustering algorithm, sensemaking routes
- Frontend: Statement contribution UI (Polis-style one-at-a-time voting), bridging statement highlighting, bracket synthesis seam
- Wiring: Connect OpinionClusterComponent to real backend data

**Tech Stack:** Rust (Diesel, SQLite), Angular 19, TypeScript

**Depends on:** Sprint 6 (signal accumulation thresholds)

---

### Task 1: Migration — statements and statement_votes tables

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-16-000002_add_statements/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-16-000002_add_statements/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`

```sql
CREATE TABLE IF NOT EXISTS statements (
    id TEXT PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    text TEXT NOT NULL,
    agree_count INTEGER NOT NULL DEFAULT 0,
    disagree_count INTEGER NOT NULL DEFAULT 0,
    pass_count INTEGER NOT NULL DEFAULT 0,
    group_id TEXT,
    is_bridging INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS statement_votes (
    id TEXT PRIMARY KEY NOT NULL,
    statement_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    vote TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(statement_id, human_id)
);

CREATE INDEX IF NOT EXISTS idx_statements_entity ON statements(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_statement_votes_statement ON statement_votes(statement_id);
```

Manually update diesel_schema.rs. Add both tables to allow_tables_to_appear_in_same_query!.

**Commit:** `feat(storage): add statements and statement_votes tables`

---

### Task 2: Diesel models, views, CRUD, and routes

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/src/db/governance.rs`
- Modify: `elohim/elohim-storage/src/api/governance.rs`
- Modify: `elohim/elohim-storage/src/http.rs`

**Models:** Statement, NewStatement, StatementVote, NewStatementVote (Queryable + Insertable)

**Views (with ts-rs export):**
- `StatementView` — id, entityType, entityId, humanId, text, agreeCount, disagreeCount, passCount, groupId, isBridging, createdAt
- `StatementVoteView` — id, statementId, humanId, vote, createdAt
- `CreateStatementInputView` — entityType, entityId, humanId, text
- `VoteOnStatementInputView` — humanId, vote (agree/disagree/pass)

**CRUD:**
- `create_statement(conn, new)` — insert + return
- `query_statements(conn, entity_type, entity_id)` — list ordered by created_at
- `vote_on_statement(conn, new)` — upsert (delete existing + insert), then update statement's agree/disagree/pass counts
- `get_statement_votes(conn, statement_id)` — list votes for a statement
- `get_all_votes_for_entity(conn, entity_type, entity_id)` — all votes for all statements of an entity (needed for clustering)

**Routes:**
- `POST /sensemaking/statements` — create statement
- `GET /sensemaking/statements?entityType=X&entityId=Y` — list statements
- `POST /sensemaking/statements/{id}/vote` — vote on statement (updates counts)
- `GET /sensemaking/votes?entityType=X&entityId=Y` — all votes for entity (for clustering)

Register in http.rs.

Run `cargo test export_bindings` and add new types to storage-client-ts index.

**Commit:** `feat(storage): add sensemaking CRUD and routes for statements`

---

### Task 3: Opinion clustering algorithm

**Files:**
- Create: `elohim/elohim-storage/src/sensemaking/mod.rs`
- Create: `elohim/elohim-storage/src/sensemaking/clustering.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` (add `pub mod sensemaking;`)

Implement rule-based clustering (MVP):

1. **Build vote matrix:** rows = human_ids, columns = statement_ids, values = agree(1)/disagree(-1)/pass(0)/unvoted(0)
2. **Cosine similarity:** Compute pairwise similarity between human vote vectors
3. **Agglomerative clustering:** Start with each human as own cluster. Merge most-similar pair. Repeat until similarity < threshold (0.3).
4. **Characteristic statements:** For each cluster, find statements where >70% of cluster members agree (or >70% disagree). These characterize the cluster's position.
5. **Bridging statements:** Statements where >60% of EVERY cluster agrees. These are common ground.

**Output types (with ts-rs export):**

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SensemakingResultView {
    pub entity_type: String,
    pub entity_id: String,
    pub clusters: Vec<OpinionClusterView>,
    pub bridging_statements: Vec<StatementView>,
    pub total_participants: usize,
    pub total_statements: usize,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct OpinionClusterView {
    pub id: String,
    pub member_count: usize,
    pub characteristic_statements: Vec<StatementView>,
    pub internal_agreement: f64,
}
```

**HTTP route:** `GET /sensemaking/clusters?entityType=X&entityId=Y` — loads all statements + votes, runs clustering, returns SensemakingResultView.

**Tests:** Add `#[cfg(test)] mod tests;` with:
- 2 clearly separated groups → 2 clusters
- Unanimous agreement → 1 cluster, bridging statement found
- No votes → empty result
- Single voter → 1 cluster

**Commit:** `feat(storage): add opinion clustering algorithm for sensemaking`

---

### Task 4: GovernanceApiService — sensemaking methods

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.ts`

Add methods:
- `submitStatement(input: CreateStatementInputView): Promise<StatementView>`
- `voteOnStatement(statementId: string, input: VoteOnStatementInputView): Promise<StatementVoteView>`
- `getStatements(entityType: string, entityId: string): Promise<StatementView[]>`
- `getClusters(entityType: string, entityId: string): Promise<SensemakingResultView>`

Import new types from `@elohim/storage-client/generated`.

**Commit:** `feat(qahal): add sensemaking API methods`

---

### Task 5: ContributeStatementComponent — Polis-style voting

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/contribute-statement/contribute-statement.component.ts`

Standalone, inline template. Inputs: `entityType`, `entityId`.

**Polis-style one-at-a-time flow:**
1. Load statements via API
2. Show one unvoted statement at a time (card with text)
3. Three buttons: Agree / Disagree / Pass
4. On vote → submit via API → show next unvoted statement
5. When all voted → show "Add your own statement" text input
6. After contributing → show "Thanks! Your statement will be shown to others"
7. Show progress: "Voted on 12 of 15 statements"

This is the core Polis interaction pattern — simple, addictive, low-friction.

Add to qahal barrel exports.

**Commit:** `feat(qahal): add contribute-statement component with Polis-style voting`

---

### Task 6: Wire OpinionClusterComponent to real data

**Files:**
- Modify: `app/elohim-app/src/app/qahal/components/opinion-cluster/opinion-cluster.component.ts`

The existing 727-line component renders from in-memory Statement/StatementVote arrays passed as inputs. It already has PCA, clustering, canvas rendering.

Wire it to real backend data:
1. Add `entityType` and `entityId` inputs
2. Load statements and votes via GovernanceApiService on init
3. Pass loaded data to existing rendering logic
4. Alternatively, load pre-computed SensemakingResultView from the clusters endpoint and map to component's internal types

Keep the existing canvas rendering — just change the data source from mock to API.

**Commit:** `feat(qahal): wire opinion cluster component to sensemaking backend`

---

### Task 7: BracketSynthesisService — Layer B seam

**Files:**
- Create: `app/elohim-app/src/app/qahal/services/bracket-synthesis.service.ts`

Stub that takes SensemakingResultView and creates a proposal with bridging statements as ranked-choice options.

```typescript
@Injectable({ providedIn: 'root' })
export class BracketSynthesisService {
  private readonly governanceApi = inject(GovernanceApiService);

  async synthesizeBracket(
    entityType: string,
    entityId: string,
    sensemakingResult: SensemakingResultView
  ): Promise<ProposalView> {
    // MVP: Create proposal where options are bridging statements
    const options = sensemakingResult.bridgingStatements.map((s, i) => ({
      id: `synth-${s.id}`,
      label: s.text,
      description: `Bridging statement supported across ${sensemakingResult.clusters.length} opinion groups`,
      position: i,
    }));

    // Create the proposal via API
    const proposal = await this.governanceApi.createProposal({
      // ... fields from CreateProposalInputView
      votingMechanism: 'ranked-choice',
      title: `Community synthesis: ${entityType} ${entityId}`,
      body: 'Ranked-choice vote on bridging statements from sensemaking',
    });

    // Create options
    await this.governanceApi.createProposalOptions(proposal.id, options);
    return proposal;
  }
}
```

In Sprint 9, this replaces with inference: the elohim reads all cluster positions and bridging statements, then synthesizes a more nuanced bracket with justifications.

Add to qahal barrel exports.

**Commit:** `feat(qahal): add bracket synthesis seam for Layer B preparation`

---

### Task 8: Integrate sensemaking into gateway + routes

**Files:**
- Modify: `app/elohim-app/src/app/qahal/components/feedback-mechanism-gateway/feedback-mechanism-gateway.component.ts`
- Modify: `app/elohim-app/src/app/qahal/community.routes.ts`

When `readyForSensemaking` is true, the gateway's "sensemaking available" badge becomes a link to the sensemaking view.

Add route: `governance/sensemaking` → A page that shows:
1. ContributeStatementComponent (top — participate first)
2. OpinionClusterComponent (below — see the landscape)
3. Bridging statements section
4. "Synthesize bracket" button (calls BracketSynthesisService)

Create a simple wrapper component for this route if needed.

**Commit:** `feat(qahal): integrate sensemaking into gateway and routes`

---

### Task 9: Tests

- Clustering algorithm: Rust unit tests (Task 3 includes these)
- SensemakingService API methods
- SignalAccumulationService → sensemaking flow
- BracketSynthesisService stub logic

**Commit:** `test(qahal): add sensemaking service tests`

---

### Task 10: A2O scenarios

**Files:**
- Modify: `genesis/a2o/features/qahal/collective-governance.feature`

Scenarios:
- "Learner contributes statement to sensemaking" — Polis-style vote flow
- "Community opinion clustering reveals groups" — 2 clusters with characteristic statements
- "Bridging statement surfaces common ground" — cross-cluster agreement highlighted
- "Sensemaking triggers bracket synthesis" — bridging statements become ranked-choice options
- "Sensemaking readiness activates from signal threshold" — gateway badge links to sensemaking view

**Commit:** `feat(a2o): add sensemaking and opinion clustering scenarios`

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | Migration — statements + statement_votes | Rust |
| 2 | Models, views, CRUD, routes + TS codegen | Rust |
| 3 | Opinion clustering algorithm + tests | Rust |
| 4 | GovernanceApiService sensemaking methods | Angular service |
| 5 | ContributeStatementComponent (Polis flow) | Angular component |
| 6 | Wire OpinionClusterComponent to backend | Angular integration |
| 7 | BracketSynthesisService (Layer B seam) | Angular service |
| 8 | Gateway + route integration | Integration |
| 9 | Tests | Testing |
| 10 | A2O scenarios | Scenarios |
