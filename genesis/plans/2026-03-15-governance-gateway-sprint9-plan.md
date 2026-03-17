# Sprint 9: Elohim Deliberation — Layer B, Plug in Inference (v2)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build Layer B — the elohim deliberation layer. Elohim agents carry governance dispositions into peer deliberation, traverse the governance hierarchy sensing for constraints and creative settlements, and report outcomes to their humans. This is where you plug in an API key or wire up native inference.

**P2P Coherence (from March 2026 refactor):**

All deliberation outputs must be properly classified:

| Entity | Classification | DHT Entry Type | Rationale |
|--------|---------------|----------------|-----------|
| GovernanceDisposition (input) | **B** (Agent-Scoped) | None | Private — Sprint 8, never on DHT |
| Elohim deliberation position | **B** (Agent-Scoped) | None | Private reasoning, never published directly |
| Settlement (deliberation outcome) | **A** (Notarized) | **New: `Settlement` in mishpat DNA** | Community witnesses the outcome — immutable governance record |
| Proxy votes (from deliberation) | **B2** (existing) | `ProposalVote` in mishpat | Existing pattern from Sprint 3/8 |
| Deliberation log | **C** (Operational) | None | Reconstructable from settlement + proxy votes, no provenance needed |

**Mishpat DNA:** 11/~100 entry types. `Settlement` is the ONE new entry type needed. Headroom: 88 remaining after this sprint. Link types: `ProposalToSettlement`, `SettlementToConstraints` (~2 new).

**Key P2P principle:** The deliberation process (elohim reasoning) is private (B). The outcome (settlement) is public (A). The community never sees how the sausage is made — they see the justified result.

**Architecture:** Each human has an elohim. Each elohim carries its human's GovernanceDisposition (B). Elohim-to-elohim deliberation is the primary governance arena. Humans opt-in to override, not opt-in to participate. The elohim solves quorum by being a faithful proxy.

**Tech Stack:** Rust (Diesel, SQLite), Angular 19, TypeScript, Claude API (or configurable inference endpoint)

**Depends on:** Sprint 8 (dispositions + proxy voting), Sprint 7 (sensemaking data), all previous sprints

---

### Task 1: P2P design — Settlement entry type in mishpat DNA

**Files:**
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs`
- Modify: `elohim/holochain/dna/mishpat/zomes/mishpat_coordinator/src/lib.rs`

**DHT design (must precede routes):**

New integrity entry type:
```rust
#[hdk_entry_helper]
pub struct Settlement {
    pub proposal_id: ActionHash,           // Which proposal was deliberated
    pub outcome: String,                    // "approved", "modified", "deferred"
    pub justification: String,              // Elohim's collective reasoning
    pub constraints_honored: Vec<String>,   // Higher-level governance constraints respected
    pub compromises: Vec<String>,           // Tradeoffs accepted
    pub hierarchy_levels_consulted: Vec<String>, // Which levels weighed in
    pub participating_elohim: Vec<AgentPubKey>,  // Which elohim participated
    pub timestamp: Timestamp,
}
```

New link types:
- `ProposalToSettlement` — from Proposal ActionHash to Settlement ActionHash
- `SettlementToConstraints` — from Settlement to the constraint sources consulted

Coordinator functions:
- `create_settlement(input: Settlement) -> ExternResult<ActionHash>`
- `get_settlement_for_proposal(proposal_hash: ActionHash) -> ExternResult<Option<Record>>`

**Commit:** `feat(mishpat): add Settlement entry type with coordinator functions`

---

### Task 2: Migration — settlements table + storage projection

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-17-000002_add_settlements/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-17-000002_add_settlements/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/src/db/governance.rs`

```sql
-- Source of truth: DHT (Settlement entry in mishpat DNA).
-- Classification: A (Notarized). Settlements are immutable governance outcomes
-- witnessed by the community. Once published, they cannot be altered.
CREATE TABLE IF NOT EXISTS settlements (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    outcome TEXT NOT NULL,
    justification TEXT NOT NULL,
    constraints_honored TEXT NOT NULL DEFAULT '[]',
    compromises TEXT NOT NULL DEFAULT '[]',
    hierarchy_levels_consulted TEXT NOT NULL DEFAULT '[]',
    participating_elohim TEXT NOT NULL DEFAULT '[]',
    dht_anchor_hash TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_settlements_proposal ON settlements(proposal_id);
```

Models: Settlement (Queryable) + NewSettlement (Insertable)
Views: SettlementView (ts-rs), CreateSettlementInputView (Deserialize)

**P2P design decision:** Routes follow from DHT design. The `Settlement` entry type (A, Notarized) is the source of truth. The storage table is a queryable projection.

CRUD + routes:
- `POST /settlements` — create settlement (records A entry, projects to storage). Route serves `Settlement` mishpat entry type.
- `GET /settlements?proposalId=X` — get settlement for proposal. Query over A-classified projection.
- `GET /settlements/{id}` — get settlement detail. Single A-classified record.

Register in http.rs. Run `cargo test export_bindings`.

**Commit:** `feat(storage): add settlements table, CRUD, and routes (A — Notarized)`

---

### Task 3: Inference client — API key or native endpoint

**Files:**
- Create: `elohim/elohim-storage/src/agents/mod.rs`
- Create: `elohim/elohim-storage/src/agents/inference_client.rs`
- Modify: `elohim/elohim-storage/src/lib.rs`

Build a configurable inference client:

```rust
pub struct InferenceClient {
    endpoint: String,
    api_key: Option<String>,
    model: String,
}

pub struct InferenceRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub max_tokens: u32,
}

pub struct InferenceResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
}

impl InferenceClient {
    pub fn from_env() -> Option<Self> // reads ELOHIM_INFERENCE_* env vars
    pub async fn complete(&self, request: InferenceRequest) -> Result<InferenceResponse, AgentError>
}
```

**P2P design decision:** The inference client is pure infrastructure — no DHT entry types, no governance entities. It's a tool the deliberation protocol uses internally. Configuration below is operational (C).

Configuration via environment variables:
- `ELOHIM_INFERENCE_ENDPOINT` — API endpoint (default: `https://api.anthropic.com/v1/messages`)
- `ELOHIM_INFERENCE_API_KEY` — API key
- `ELOHIM_INFERENCE_MODEL` — model (default: `claude-sonnet-4-20250514`)
- `ELOHIM_DELIBERATION_ENABLED` — feature flag (default: `false`)

Uses `reqwest` for HTTP. Add to Cargo.toml if not present.

**Commit:** `feat(elohim): add inference client for governance deliberation`

---

### Task 4: Governance agent prompt template

**Files:**
- Create: `elohim/elohim-storage/src/agents/governance_agent.rs`

Define the governance agent's system prompt builder:

```rust
pub fn build_governance_prompt(
    disposition: &GovernanceDisposition,
    proposal: &Proposal,
    options: &[ProposalOption],
    sensemaking: Option<&SensemakingResultView>,
    hierarchy_constraints: &[String],
) -> InferenceRequest
```

The system prompt carries:
- The human's GovernanceDisposition (values, risk tolerance, priorities)
- Proposal context (title, body, options)
- Sensemaking context (clusters, bridging statements) if available
- Hierarchy constraints (what higher governance levels require)
- Output format instruction: respond as JSON `{ "position": "option_id", "justification": "...", "confidence": 0.0-1.0 }`

This is the "soul document IS the base model" principle — the prompt carries the whole context.

**Commit:** `feat(elohim): define governance agent prompt template`

---

### Task 5: Deliberation protocol (single-turn MVP)

**Files:**
- Create: `elohim/elohim-storage/src/agents/deliberation.rs`

The deliberation flow:

```rust
pub async fn deliberate_on_proposal(
    conn: &mut SqliteConnection,
    inference: &InferenceClient,
    proposal_id: &str,
) -> Result<Settlement, AgentError>
```

1. Load proposal + options
2. Load sensemaking result (if available)
3. For each human in the collective who hasn't voted:
   a. Load their GovernanceDisposition
   b. Build governance prompt
   c. Run inference → get `{ position, justification, confidence }`
   d. Record as proxy vote (B2, using existing cast_ranked_votes)
4. Collect all positions
5. If consensus (all positions agree): create Settlement with "approved"
6. If disagreement: create Settlement with "modified" + compromises noted
7. If blocks: Settlement with "deferred" + escalation needed flag

**Single-turn MVP:** No multi-turn deliberation between elohim. Each elohim reasons independently from its disposition + context. Multi-turn (elohim seeing each other's positions and responding) is a future enhancement.

**Route:** `POST /deliberation/{proposal_id}/run` — triggers deliberation, requires `ELOHIM_DELIBERATION_ENABLED=true`

**P2P design decision:** The deliberation process itself is never recorded on DHT (B — private reasoning). The Settlement output is A (Notarized) via the `Settlement` mishpat entry type. Proxy votes are B2 (existing pattern).

**Commit:** `feat(elohim): add single-turn deliberation protocol`

---

### Task 6: Replace BracketSynthesisService stub (Sprint 7)

**Files:**
- Modify: `app/elohim-app/src/app/qahal/services/bracket-synthesis.service.ts`

Replace the stub with inference-backed synthesis:

**P2P design decision:** `synthesize-bracket` creates a `Proposal` (A, existing mishpat `Proposal` entry type) + `ProposalOption` entries (A2, existing mishpat link type). No new DHT entry types — this route produces artifacts using existing governance entry types.

1. Load sensemaking result (clusters, bridging statements)
2. Call backend route: `POST /deliberation/{entity_id}/synthesize-bracket` — creates Proposal (A) + ProposalOptions (A2) via existing mishpat entry types
3. Backend runs inference with bracket-synthesis prompt: "Given these opinion clusters and bridging statements, propose a ranked set of options that best serves the community"
4. Returns a proposal with agent-synthesized options + justifications

Fallback: if inference is disabled or fails, use the existing stub logic (bridging statements as options).

**Commit:** `feat(qahal): replace bracket synthesis stub with inference`

---

### Task 7: GovernanceApiService — deliberation methods

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.ts`

Add methods:
- `runDeliberation(proposalId: string): Promise<SettlementView>`
- `getSettlement(proposalId: string): Promise<SettlementView | null>`
- `synthesizeBracket(entityType: string, entityId: string): Promise<ProposalView>`

**P2P design decision:** `runDeliberation` creates a `Settlement` (A — Notarized via mishpat entry type) and proxy votes (B2 — existing `ProposalVote` entry type). `getSettlement` queries the A-classified projection. No new entry types.

**Commit:** `feat(qahal): add deliberation API methods`

---

### Task 8: DeliberationVisualizationComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/deliberation-visualization/deliberation-visualization.component.ts`

Standalone, inline template. Input: `proposalId`.

Shows the deliberation outcome:
- Settlement status: approved / modified / deferred
- Justification text
- Constraints honored (list)
- Compromises accepted (list)
- Hierarchy levels consulted
- "Your elohim said..." section for the current user (loads their proxy vote justification)
- Override button → navigates to proxy vote override (Sprint 8)

This makes governance transparent — you can see why decisions were made.

Add to qahal barrel exports.

**Commit:** `feat(qahal): add deliberation visualization component`

---

### Task 9: Replace disposition computation stub (Sprint 8)

**Files:**
- Modify: `elohim/elohim-storage/src/services/disposition_service.rs`

Replace rule-based disposition computation with inference-augmented:

1. Load human's full governance history (votes, challenges, signals, sensemaking positions)
2. Build a characterization prompt: "Based on this governance behavior, characterize this person's values"
3. Run inference
4. Parse response into GovernanceDisposition fields
5. Fallback: if inference disabled, keep existing rule-based computation

The inference result is B (Agent-Scoped) — never published.

**Commit:** `feat(elohim): augment disposition computation with inference`

---

### Task 10: Configuration, routes, and feature flags

**Files:**
- Modify: deployment/environment configuration
- Modify: `elohim/elohim-storage/src/api/governance.rs`

Register deliberation routes:

**P2P design decision:** `POST /deliberation/{id}/run` creates proxy votes (B2, existing `ProposalVote`) + Settlement (A, new mishpat entry type). `POST /deliberation/{id}/synthesize-bracket` creates a Proposal (A, existing mishpat entry type) + ProposalOptions (A2, existing). All routes serve existing or Sprint 9's new DHT entry types.

- `POST /deliberation/{proposal_id}/run` — triggers deliberation (guarded by feature flag)
- `GET /deliberation/{proposal_id}/settlement` — get settlement result
- `POST /deliberation/{entity_id}/synthesize-bracket` — inference-backed bracket synthesis

Feature flag: `ELOHIM_DELIBERATION_ENABLED` must be `true` for deliberation routes to accept requests. Returns 503 otherwise.

**Commit:** `feat(elohim): add deliberation routes with feature flags`

---

### Task 11: Tests

- Inference client: mock HTTP responses, verify request format
- Governance agent prompt: verify disposition + context produces valid prompt
- Deliberation protocol: mock inference → verify proxy votes created + settlement recorded
- Settlement CRUD: create + query
- Feature flag: verify 503 when disabled
- DeliberationVisualizationComponent: renders settlement, shows proxy vote justification

**Commit:** `test(elohim): add governance deliberation tests`

---

### Task 12: A2O scenarios

- "Elohim deliberates on proposal using human's disposition" — agent infers position from values
- "Deliberation produces settlement when elohim reach consensus" — proxy votes + settlement recorded
- "Settlement shows justified outcome to community" — transparent reasoning visible
- "Human reviews elohim's deliberation reasoning" — proxy vote justification displayed
- "Human overrides after reviewing deliberation" — override with direct vote
- "Deliberation disabled without API key" — 503 response, graceful degradation

**Commit:** `feat(a2o): add elohim deliberation scenarios`

---

## Layer B Complete

After Sprint 9, the full three-layer architecture is wired:
- **Layer A** (Sprints 3-6): Feedback mechanisms at the content → signals accumulate
- **Layer C** (Sprint 7): Signals → opinion clusters → bridging statements → brackets
- **Layer B** (Sprints 8-9): Dispositions → elohim deliberation → settlements → humans opt-in to override

The medium IS the message. Governance is experienced, not abstracted.

## Summary

| Task | What | P2P Class | Layer |
|------|------|-----------|-------|
| 1 | Settlement entry type in mishpat DNA | A (Notarized) | Holochain |
| 2 | settlements table + CRUD + routes | A projection | Rust |
| 3 | Inference client (API key seam) | — | Rust infrastructure |
| 4 | Governance agent prompt template | — | Rust agent |
| 5 | Deliberation protocol (single-turn) | Creates B2 + A | Rust agent |
| 6 | Replace bracket synthesis stub | — | Angular service |
| 7 | GovernanceApiService methods | — | Angular service |
| 8 | DeliberationVisualizationComponent | — | Angular component |
| 9 | Replace disposition computation | B (private) | Rust service |
| 10 | Routes + feature flags | — | Integration |
| 11 | Tests | — | Testing |
| 12 | A2O scenarios | — | Scenarios |
