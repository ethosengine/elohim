# Sprint 8: Governance Disposition & Proxy Voting — Elohim as Faithful Proxy (v2)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the GovernanceDisposition — the persistent profile that captures each human's governance values, preferences, and history. This is what the elohim carries into deliberation. Also build the proxy voting infrastructure where elohim vote on behalf of humans who haven't engaged.

**P2P Coherence (from March 2026 refactor):**

The P2P coherence refactor established classifications for all governance tables. Sprint 8 must follow these patterns:

| Entity | Classification | Rationale |
|--------|---------------|-----------|
| `governance_dispositions` | **B (Agent-Scoped)** | Personal governance values — private to the human and their elohim. Never published to DHT. Lives on agent's source chain only. |
| Proxy votes (existing `ranked_votes`) | **B2 (Agent-Scoped + Attestation)** | Raw vote is private (source chain), but tally Attestation is public (A). Proxy votes use the existing B2 pattern — `proxy_elohim_id` and `proxy_justification` fields already exist from Sprint 3. |
| Disposition computation | **Aggregates B2 data** | Reads private votes/signals/challenges to compute disposition. The disposition itself is B, not published. |

**Key P2P principle:** Dispositions are NEVER notarized on the DHT. They are agent-scoped intelligence — the elohim's understanding of its human. This is a feature, not a gap: governance values are private until the elohim acts on them publicly (via proxy vote).

**Mishpat DNA headroom:** 11/~100 entry types. GovernanceDisposition does NOT need a DHT entry type — it's B (agent-scoped, source chain only). No new entry types needed for this sprint.

**Architecture:** GovernanceDisposition is the seam between human governance behavior and elohim proxy representation. It accumulates from: voting history (B2), feedback patterns (B2), challenge stances (A), sensemaking positions (B2). The elohim reads this disposition to act faithfully on behalf of its human.

**Tech Stack:** Rust (Diesel, SQLite), Angular 19, TypeScript

**Depends on:** Sprint 7 (sensemaking), Sprint 6 (signals), Sprint 5 (challenges)

---

### Task 1: Migration — governance_dispositions table

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-17-000001_add_governance_dispositions/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-17-000001_add_governance_dispositions/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`

```sql
-- Source of truth: agent source chain (private governance profile).
-- Classification: B (Agent-Scoped). NOT published to DHT.
-- This is the elohim's understanding of its human's governance values.
-- dht_anchor_hash is intentionally NULL — dispositions are never notarized.
CREATE TABLE IF NOT EXISTS governance_dispositions (
    id TEXT PRIMARY KEY NOT NULL,
    human_id TEXT NOT NULL UNIQUE,
    risk_tolerance REAL NOT NULL DEFAULT 0.5,
    change_openness REAL NOT NULL DEFAULT 0.5,
    consensus_preference REAL NOT NULL DEFAULT 0.5,
    priority_values TEXT NOT NULL DEFAULT '[]',
    voting_pattern_summary TEXT NOT NULL DEFAULT '{}',
    total_votes_cast INTEGER NOT NULL DEFAULT 0,
    total_challenges_filed INTEGER NOT NULL DEFAULT 0,
    total_signals_recorded INTEGER NOT NULL DEFAULT 0,
    dht_anchor_hash TEXT,
    last_computed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_governance_dispositions_human ON governance_dispositions(human_id);
```

Note: `priority_values` is JSON array, `voting_pattern_summary` is JSON object. SQLite stores both as TEXT.

Manually update diesel_schema.rs. Add to allow_tables_to_appear_in_same_query!.

**Commit:** `feat(storage): add governance_dispositions table (B — agent-scoped)`

---

### Task 2: Models, views, CRUD

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/src/db/governance.rs`

**Models:** GovernanceDisposition (Queryable) + NewGovernanceDisposition (Insertable). Include source-of-truth comment: `/// Classification: B (Agent-Scoped). Private governance profile.`

**Views (ts-rs):**
- `GovernanceDispositionView` — all fields, with `priorityValues` as `Vec<String>` (parse from JSON)
- `UpdateDispositionInputView` (Deserialize) — optional overrides for risk_tolerance, change_openness, consensus_preference, priority_values

**CRUD:**
- `get_disposition(conn, human_id)` — returns Option<GovernanceDisposition>
- `upsert_disposition(conn, human_id, new)` — insert or update
- `update_disposition_overrides(conn, human_id, overrides)` — update specific fields from human manual adjustments

**Commit:** `feat(storage): add GovernanceDisposition model, views, and CRUD`

---

### Task 3: Disposition computation service

**Files:**
- Create: `elohim/elohim-storage/src/services/disposition_service.rs`

Compute GovernanceDisposition from historical B2 data:

```rust
pub fn compute_disposition(
    conn: &mut SqliteConnection,
    human_id: &str,
) -> Result<GovernanceDisposition, StorageError>
```

Aggregation logic (rule-based for Sprint 8, inference-based in Sprint 9):

1. **Voting patterns** → `voting_pattern_summary`
   - Query ranked_votes + votes for this human
   - Count by mechanism type: {ranked_choice: N, approval: N, consent: N, ...}
   - Compute average position (do they tend to vote for change or status quo?)

2. **Risk tolerance** (0.0 = conservative, 1.0 = progressive)
   - High challenge_filed count → higher risk tolerance (willing to rock the boat)
   - Frequent "block" votes → lower risk tolerance (cautious)
   - Normalize to 0.0-1.0

3. **Change openness** (0.0 = resist change, 1.0 = embrace change)
   - Voting for new proposals vs abstaining → change openness
   - Signal patterns (positive reactions to novel content) → change openness

4. **Consensus preference** (0.0 = individualist, 1.0 = consensus-seeker)
   - Proportion of consent/consensus votes vs competitive votes
   - Sensemaking participation (statement contribution) increases consensus_preference

5. **Priority values** (keywords extracted from challenge grounds and signal patterns)
   - Frequent "factual-error" challenges → values accuracy
   - Frequent "bias" challenges → values fairness
   - Frequent "harmful" challenges → values safety

**P2P design decision:** Dispositions are B (Agent-Scoped) — no DHT entry type, no mishpat coordinator function. These routes serve data from the local SQLite projection only. The disposition is never published to the DHT; it's the elohim's private understanding of its human.

**Route:** `POST /dispositions/{human_id}/compute` — triggers computation from local B2 data, returns updated disposition
**Route:** `GET /dispositions/{human_id}` — get current disposition from local projection

Register in http.rs.

Run `cargo test export_bindings` for TypeScript types.

**Commit:** `feat(storage): add disposition computation service`

---

### Task 4: Proxy voting routes

**Files:**
- Modify: `elohim/elohim-storage/src/api/governance.rs`

The ranked_votes table already has `proxy_elohim_id` and `proxy_justification` fields (Sprint 3). Now wire the logic:

**P2P design decision:** No new DHT entry types. Proxy votes use the existing `RankedVote` entry type (B2 — already in mishpat DNA, `ProposalVote`). The `proxy_elohim_id` field distinguishes proxy from direct votes at the storage projection level. Override uses the existing `GovernanceReaction` entry type (B2) for the override signal. All routes below serve existing DHT-backed entity types through the storage projection.

- `POST /proposals/{id}/proxy-votes` — elohim casts via existing `RankedVote` (B2, mishpat `ProposalVote` entry type), with `proxyElohimId` + `proxyJustification` set
  - Validates: human hasn't already voted directly (proxy can't override direct vote)
  - Validates: proxy_elohim_id is provided

- `GET /proposals/{id}/proxy-votes` — filters existing ranked_votes where proxy_elohim_id IS NOT NULL

- `POST /proposals/{id}/override-proxy` — deletes proxy vote, inserts direct vote (same `RankedVote` B2 entry type), records override via existing `GovernanceReaction` (B2, mishpat entry type)

**Commit:** `feat(storage): add proxy voting and override routes`

---

### Task 5: GovernanceApiService — disposition + proxy methods

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.ts`

Add methods:
- `getDisposition(humanId: string): Promise<GovernanceDispositionView | null>`
- `computeDisposition(humanId: string): Promise<GovernanceDispositionView>`
- `updateDisposition(humanId: string, overrides: UpdateDispositionInputView): Promise<GovernanceDispositionView>`
- `castProxyVotes(proposalId: string, ballot: CastRankedVoteInputView): Promise<RankedVoteView[]>`
- `overrideProxyVote(proposalId: string, ballot: CastRankedVoteInputView): Promise<RankedVoteView[]>`

**Commit:** `feat(qahal): add disposition and proxy voting API methods`

---

### Task 6: GovernanceDispositionComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/governance-disposition/governance-disposition.component.ts`

Standalone, inline template. Input: `humanId`.

Shows the human's governance profile:
- Three sliders (read-only, computed): Risk Tolerance, Change Openness, Consensus Preference
- Each with a label scale: e.g. "Conservative ← → Progressive"
- Priority values as tags
- Voting pattern summary: "42 votes cast — mostly consent (60%), some ranked-choice (25%)"
- Stats: challenges filed, signals recorded

Human can toggle "manual override" mode to adjust sliders. On save → calls `updateDisposition()`.

"Recompute from history" button → calls `computeDisposition()`.

Add to qahal barrel exports.

**Commit:** `feat(qahal): add governance disposition component`

---

### Task 7: ProxyVoteNotificationComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/proxy-vote-notification/proxy-vote-notification.component.ts`

Standalone, inline template. Input: `proxyVote: RankedVoteView` (a vote with proxyElohimId set).

Shows:
- "Your elohim voted on '{proposal title}'"
- The elohim's justification text
- Two action buttons: "Looks right" (dismiss) / "I disagree" (opens override form)
- Override form: shows proposal options, lets human vote directly
- On override → calls `overrideProxyVote()` → records proxy-override signal

**Commit:** `feat(qahal): add proxy vote notification and override component`

---

### Task 8: Integration — disposition route + proxy notifications

**Files:**
- Modify: `app/elohim-app/src/app/qahal/community.routes.ts`
- Modify: gateway or relevant layout component

Add route: `governance/disposition` → GovernanceDispositionComponent

For proxy notifications: these appear in a notification area (can be part of the community layout or a global notification service). For MVP, add a section to the community home that shows unreviewed proxy votes.

**Commit:** `feat(qahal): integrate disposition route and proxy vote notifications`

---

### Task 9: Tests

- Disposition computation: verify risk_tolerance/change_openness/consensus_preference from mock vote data
- Proxy vote creation: verify proxy_elohim_id set, human can't override with proxy
- Override flow: verify proxy deleted, direct vote created, override signal recorded
- GovernanceDispositionComponent: renders sliders, override mode, recompute button

**Commit:** `test(qahal): add governance disposition and proxy voting tests`

---

### Task 10: A2O scenarios

- "Elohim builds governance disposition from human's voting history" — disposition computed, values match behavior
- "Elohim votes as proxy when human hasn't engaged" — proxy vote recorded with justification
- "Human reviews proxy vote and confirms" — notification dismissed
- "Human overrides elohim proxy vote" — direct vote replaces proxy, override signal recorded
- "Governance disposition reflects human's consistent values" — priority values match challenge grounds

**Commit:** `feat(a2o): add governance disposition and proxy voting scenarios`

---

## Summary

| Task | What | P2P Class | Layer |
|------|------|-----------|-------|
| 1 | governance_dispositions table | B (Agent-Scoped) | Rust migration |
| 2 | Models, views, CRUD | — | Rust |
| 3 | Disposition computation service | Aggregates B2 → B | Rust service |
| 4 | Proxy voting routes | B2 (existing pattern) | Rust routes |
| 5 | GovernanceApiService methods | — | Angular service |
| 6 | GovernanceDispositionComponent | — | Angular component |
| 7 | ProxyVoteNotificationComponent | — | Angular component |
| 8 | Routes + notification integration | — | Integration |
| 9 | Tests | — | Testing |
| 10 | A2O scenarios | — | Scenarios |
