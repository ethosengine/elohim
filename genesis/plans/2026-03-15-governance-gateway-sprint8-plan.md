# Sprint 8: Governance Disposition & Proxy Voting — Elohim as Faithful Proxy

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the GovernanceDisposition — the persistent profile that captures each human's governance values, preferences, and history. This is what the elohim carries into deliberation. Also build the proxy voting infrastructure where elohim vote on behalf of humans who haven't engaged.

**Architecture:** GovernanceDisposition is the seam between human governance behavior and elohim proxy representation. It accumulates from: voting history, feedback patterns, challenge stances, sensemaking positions. The elohim reads this disposition to act faithfully on behalf of its human.

**Tech Stack:** Angular 19, TypeScript, elohim-storage Rust backend

**Depends on:** Sprint 7 (sensemaking provides opinion data), Sprint 6 (signals provide behavior data), Sprint 5 (challenges provide stance data)

---

### Task 1: Backend — governance_dispositions table

**Files:**
- Create migration
- Modify: models.rs, views.rs, governance.rs

New table `governance_dispositions`:
- id, human_id, disposition_data (JSON TEXT — serialized governance profile)
- risk_tolerance (0.0-1.0), change_openness (0.0-1.0), consensus_preference (0.0-1.0)
- priority_values (JSON array of value keywords)
- voting_pattern_summary (JSON — aggregated from voting history)
- last_updated_at

CRUD: get_disposition, update_disposition, compute_disposition_from_history

View: GovernanceDispositionView with ts-rs export

**Commit:** `feat(storage): add governance_dispositions table and CRUD`

---

### Task 2: Disposition computation from history

**Files:**
- Create: `elohim/elohim-storage/src/services/disposition_service.rs`

Compute GovernanceDisposition from historical behavior:
- Voting patterns → risk_tolerance, change_openness
- Challenge history → which values the human consistently defends
- Sensemaking positions → consensus_preference
- Feedback patterns → quality priorities

This is rule-based for now. Sprint 9 replaces with inference-based computation.

**Commit:** `feat(storage): add disposition computation from governance history`

---

### Task 3: Proxy voting infrastructure

**Files:**
- Modify: `elohim/elohim-storage/src/api/governance.rs`
- Modify: `elohim/elohim-storage/src/db/governance.rs`

The ranked_votes table already has `proxy_elohim_id` and `proxy_justification` fields (Sprint 3). Now wire the logic:
- `POST /proposals/{id}/proxy-votes` — elohim casts votes on behalf of human, with justification
- Proxy votes are flagged in the UI (different visual treatment)
- Human can override proxy votes at any time
- Override carries proportional weight based on governance hierarchy level

**Commit:** `feat(storage): add proxy voting routes and override logic`

---

### Task 4: GovernanceDispositionComponent — view/edit disposition

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/governance-disposition/governance-disposition.component.ts`

Shows the human's governance profile:
- Risk tolerance slider (read-only, computed)
- Change openness slider (read-only, computed)
- Priority values tags
- Recent voting pattern summary
- "Your elohim would vote..." predictions based on disposition

Human can adjust explicit preferences that override computed values.

**Commit:** `feat(qahal): add governance disposition component`

---

### Task 5: ProxyVoteNotificationComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/proxy-vote-notification/proxy-vote-notification.component.ts`

When an elohim votes on behalf of a human:
- Notification appears: "Your elohim voted on '{proposal title}'. Here's why: {justification}"
- Actions: "Looks right" (confirm) / "I disagree" (opens override flow)
- Override flow shows the proposal + options + the elohim's choice + space to vote differently

This is the "we got a challenge at quorum about X, I thought you would say Y" interaction.

**Commit:** `feat(qahal): add proxy vote notification and override component`

---

### Task 6: GovernanceProfileService — Angular service

**Files:**
- Create: `app/elohim-app/src/app/qahal/services/governance-profile.service.ts`

Methods:
- `getDisposition(humanId: string): Observable<GovernanceDispositionView>`
- `updateDisposition(humanId: string, overrides: Partial<GovernanceDispositionView>): Observable<GovernanceDispositionView>`
- `getProxyVotes(humanId: string): Observable<RankedVoteView[]>` (filter by proxy_elohim_id != null)
- `overrideProxyVote(proposalId: string, ballot: CastRankedVoteInputView): Observable<RankedVoteView[]>`

**Commit:** `feat(qahal): add governance profile service`

---

### Task 7: Hierarchy-aware weight computation

**Files:**
- Modify: tally strategies or create helper

When a human overrides their elohim's proxy vote, the weight of that override depends on governance hierarchy level:
- Individual level: full weight
- Family/household level: proportional to household size
- Community level: proportional to community membership
- Network level: proportional to network reach

This is a seam — for Sprint 8, implement simple equal weights. Sprint 9 can add hierarchy-aware weighting.

**Commit:** `feat(storage): add hierarchy-aware vote weight seam`

---

### Task 8: Tests

- Disposition computation from history
- Proxy vote creation with justification
- Human override of proxy vote
- GovernanceDispositionComponent renders profile
- ProxyVoteNotificationComponent notification + override flow

**Commit:** `test(qahal): add governance disposition and proxy voting tests`

---

### Task 9: A2O scenarios

- "Elohim builds governance disposition from human's voting history"
- "Elohim votes as proxy when human hasn't engaged" — proxy_elohim_id set, justification provided
- "Human receives notification of proxy vote and confirms" — no override
- "Human overrides elohim proxy vote with different preference" — override recorded, weight applied
- "Governance disposition reflects human's consistent values" — priority_values match behavior

**Commit:** `feat(a2o): add governance disposition and proxy voting scenarios`

---

## Summary

| Task | What | Layer |
|------|------|-------|
| 1 | governance_dispositions table | Rust |
| 2 | Disposition computation | Rust |
| 3 | Proxy voting routes | Rust |
| 4 | GovernanceDispositionComponent | Component |
| 5 | ProxyVoteNotificationComponent | Component |
| 6 | GovernanceProfileService | Service |
| 7 | Hierarchy-aware weight seam | Rust |
| 8 | Tests | Testing |
| 9 | A2O scenarios | Scenarios |
