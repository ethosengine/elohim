# Sprint 7: Polis Sensemaking — Opinion Clustering & Bridging Statements

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build Layer C — the Polis-inspired sensemaking layer. Accumulated governance signals (from Sprint 6) are clustered into opinion groups, bridging statements are surfaced, and the elohim synthesizes justified brackets for structured deliberation.

**Architecture:** Polis is the sensing organ; elohim is the deliberative mind. This sprint implements the sensing: signal → statement → cluster → bridge → bracket. The elohim synthesizing brackets is a seam for Sprint 9's inference integration.

**Tech Stack:** Angular 19, TypeScript, elohim-storage Rust backend, lightweight clustering (no ML dependency — rule-based for now, replaced with proper dimensionality reduction later)

**Depends on:** Sprint 6 (signals accumulated, aggregation queries)

---

### Task 1: Backend — statements table and CRUD

**Files:**
- Create migration: `elohim/elohim-storage/migrations/YYYY-MM-DD_add_statements/`
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Modify: `elohim/elohim-storage/src/db/governance.rs`
- Modify: `elohim/elohim-storage/src/views.rs`

New table `statements`:
- id, entity_type, entity_id, human_id, text, agree_count, disagree_count, pass_count, group_id, is_bridging, created_at

New table `statement_votes`:
- id, statement_id, human_id, vote (agree/disagree/pass), created_at
- UNIQUE(statement_id, human_id)

CRUD: create_statement, vote_on_statement, query_statements, get_statement_votes

View types: StatementView, StatementVoteView, CreateStatementInputView, VoteOnStatementInputView

HTTP routes:
- `POST /sensemaking/statements` — create statement
- `GET /sensemaking/statements?entityType=X&entityId=Y` — list statements
- `POST /sensemaking/statements/{id}/vote` — vote on statement
- `GET /sensemaking/clusters?entityType=X&entityId=Y` — get opinion clusters (Task 2)

**Commit:** `feat(storage): add statements and statement_votes tables with CRUD`

---

### Task 2: Backend — opinion clustering algorithm

**Files:**
- Create: `elohim/elohim-storage/src/sensemaking/mod.rs`
- Create: `elohim/elohim-storage/src/sensemaking/clustering.rs`
- Modify: `elohim/elohim-storage/src/lib.rs`

Implement a rule-based clustering algorithm (MVP, not ML):
1. Build a vote matrix: rows = humans, columns = statements, values = agree(1)/disagree(-1)/pass(0)
2. Compute pairwise similarity between humans (cosine similarity on vote vectors)
3. Simple agglomerative clustering (merge most-similar pairs until threshold)
4. For each cluster: find characteristic statements (high within-cluster agreement)
5. Find bridging statements (high agreement across ALL clusters)

Output type:
```rust
pub struct SensemakingResult {
    pub clusters: Vec<OpinionCluster>,
    pub bridging_statements: Vec<StatementView>,
    pub total_participants: usize,
    pub total_statements: usize,
}

pub struct OpinionCluster {
    pub id: String,
    pub member_count: usize,
    pub characteristic_statements: Vec<StatementView>,
    pub internal_agreement: f64,
}
```

**Commit:** `feat(storage): add opinion clustering algorithm for sensemaking`

---

### Task 3: SensemakingService — Angular service

**Files:**
- Create: `app/elohim-app/src/app/qahal/services/sensemaking.service.ts`

Methods:
- `submitStatement(entityType: string, entityId: string, text: string): Observable<StatementView>`
- `voteOnStatement(statementId: string, vote: 'agree'|'disagree'|'pass'): Observable<StatementVoteView>`
- `getStatements(entityType: string, entityId: string): Observable<StatementView[]>`
- `getClusters(entityType: string, entityId: string): Observable<SensemakingResult>`

**Commit:** `feat(qahal): add sensemaking service`

---

### Task 4: ContributeStatementComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/contribute-statement/contribute-statement.component.ts`

Simple form: text input + submit button. Shows existing statements below with agree/disagree/pass vote buttons per statement. Polis-style: one statement at a time, vote, see next.

**Commit:** `feat(qahal): add contribute statement component`

---

### Task 5: OpinionClusterVisualizationComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/opinion-cluster-visualization/opinion-cluster-visualization.component.ts`

Renders clusters as cards/groups:
- Each cluster shows member count and characteristic statements
- Bridging statements highlighted across clusters
- No 2D projection for MVP — just grouped lists with visual grouping
- Later: replace with proper t-SNE/UMAP visualization

**Commit:** `feat(qahal): add opinion cluster visualization component`

---

### Task 6: BridgingStatementComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/bridging-statement/bridging-statement.component.ts`

Highlights statements with cross-cluster agreement. Shows:
- Statement text
- Agreement percentage per cluster
- Overall agreement score
- "Common ground" badge

These bridging statements are the raw material for elohim bracket synthesis (Sprint 9).

**Commit:** `feat(qahal): add bridging statement component`

---

### Task 7: ElohimBracketSynthesisSeam

**Files:**
- Create: `app/elohim-app/src/app/qahal/services/bracket-synthesis.service.ts`

This is the seam for Sprint 9. For now, it's a stub that takes bridging statements and packages them into a ProposalView with ranked-choice options. The options are the bridging statements themselves.

In Sprint 9, this will be replaced with actual elohim inference that synthesizes a justified bracket from the sensemaking data.

```typescript
@Injectable({ providedIn: 'root' })
export class BracketSynthesisService {
  // Sprint 9: replace with inference call
  synthesizeBracket(sensemakingResult: SensemakingResult): Observable<ProposalView> {
    // MVP: create proposal with bridging statements as options
  }
}
```

**Commit:** `feat(qahal): add bracket synthesis seam (Layer B preparation)`

---

### Task 8: Integrate sensemaking into gateway

**Files:**
- Modify: FeedbackMechanismGatewayComponent
- Add sensemaking route to qahal routing

When signal accumulation passes the "ready for sensemaking" threshold (Sprint 6), the gateway can show a "Sensemaking in progress" indicator and link to the sensemaking view.

Route: `governance/sensemaking` → shows ContributeStatementComponent + OpinionClusterVisualizationComponent

**Commit:** `feat(qahal): integrate sensemaking into governance routes`

---

### Task 9: Tests and clustering algorithm tests

- Clustering algorithm: test with known vote patterns → expected clusters
- Bridging statement detection: test cross-cluster agreement scoring
- ContributeStatementComponent: submit and vote flow
- SensemakingService: API integration

**Commit:** `test(qahal): add sensemaking and clustering tests`

---

### Task 10: A2O scenarios

- "Community opinion clustering reveals groups" — statements cluster into 2 groups
- "Bridging statement surfaces common ground" — high cross-cluster agreement highlighted
- "Elohim synthesizes bracket from sensemaking" — bridging statements become ranked-choice options
- "Learner contributes statement to sensemaking" — statement added, vote recorded
- "Sensemaking readiness triggered by signal threshold" — N signals → sensemaking begins

**Commit:** `feat(a2o): add sensemaking and opinion clustering scenarios`

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | Statements table + CRUD + routes | Rust |
| 2 | Opinion clustering algorithm | Rust |
| 3 | SensemakingService | Service |
| 4 | ContributeStatementComponent | Component |
| 5 | OpinionClusterVisualizationComponent | Component |
| 6 | BridgingStatementComponent | Component |
| 7 | BracketSynthesisService (seam) | Service |
| 8 | Integration into gateway | Integration |
| 9 | Tests | Testing |
| 10 | A2O scenarios | Scenarios |
