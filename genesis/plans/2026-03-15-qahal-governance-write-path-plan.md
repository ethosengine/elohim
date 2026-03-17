# Qahal Governance Write Path Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire proposals, votes, and discussions to real persistence — replacing the localStorage MVP in GovernanceService with HTTP calls to new POST routes on elohim-storage.

**Architecture:** Add a `votes` table and `voting_anonymous` column on proposals. Create InputView types for proposals, votes, and discussions. Add POST handlers to the governance API dispatcher. Replace localStorage calls in Angular GovernanceService with GovernanceApiService HTTP methods. Add CollectiveDetailComponent to navigate from directory to governance.

**Tech Stack:** Rust/Diesel (elohim-storage), Angular 19 standalone components, TypeScript, Vitest

---

### Task 1: Add votes table and voting_anonymous column

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-03-15-000002_add_votes_table/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-03-15-000002_add_votes_table/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs`
- Modify: `elohim/elohim-storage/src/db/models.rs`

**Step 1: Create migration**

```sql
-- up.sql
CREATE TABLE IF NOT EXISTS votes (
    id TEXT PRIMARY KEY NOT NULL,
    proposal_id TEXT NOT NULL,
    human_id TEXT NOT NULL,
    position TEXT NOT NULL,
    reason TEXT,
    anonymous INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(proposal_id, human_id)
);

ALTER TABLE proposals ADD COLUMN voting_anonymous INTEGER NOT NULL DEFAULT 0;
```

```sql
-- down.sql
DROP TABLE IF EXISTS votes;
ALTER TABLE proposals DROP COLUMN voting_anonymous;
```

**Step 2: Add votes table to diesel_schema.rs**

After the `discussions` table macro:

```rust
diesel::table! {
    votes (id) {
        id -> Text,
        proposal_id -> Text,
        human_id -> Text,
        position -> Text,
        reason -> Nullable<Text>,
        anonymous -> Integer,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

Add `voting_anonymous -> Integer,` to the existing `proposals` table macro, after `votes_against`.

**Step 3: Add Vote and NewVote models to models.rs**

After the `NewDiscussion` struct:

```rust
/// Governance vote on a proposal
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = votes)]
pub struct Vote {
    pub id: String,
    pub proposal_id: String,
    pub human_id: String,
    pub position: String,
    pub reason: Option<String>,
    pub anonymous: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// New vote for INSERT
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = votes)]
pub struct NewVote<'a> {
    pub id: &'a str,
    pub proposal_id: &'a str,
    pub human_id: &'a str,
    pub position: &'a str,
    pub reason: Option<&'a str>,
    pub anonymous: i32,
}
```

Add `pub voting_anonymous: i32,` to the existing `Proposal` struct and `pub voting_anonymous: Option<i32>,` to `NewProposal` (with `#[diesel(column_name = voting_anonymous)]`). Actually — `NewProposal` uses `&'a str` references and is minimal. Since `voting_anonymous` has a DEFAULT in SQL, we can leave `NewProposal` unchanged and let the DB default handle it. BUT the `Proposal` queryable struct MUST include the column or diesel will fail.

Add to `Proposal` struct (after `votes_against`):
```rust
    pub voting_anonymous: i32,
```

**Step 4: Verify it compiles**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -10`
Expected: Errors in views.rs (ProposalView missing field). Fixed in Task 2.

**Step 5: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-03-15-000002_add_votes_table/ elohim/elohim-storage/src/db/diesel_schema.rs elohim/elohim-storage/src/db/models.rs
git commit -m "feat(storage): add votes table and voting_anonymous column on proposals"
```

---

### Task 2: Add InputView and VoteView types

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

**Step 1: Add voting_anonymous to ProposalView**

In the `ProposalView` struct, add after `votes_against`:
```rust
    pub voting_anonymous: bool,
```

In the `From<Proposal> for ProposalView` impl, add:
```rust
            voting_anonymous: p.voting_anonymous == 1,
```

**Step 2: Add VoteView**

After the `DiscussionView` block:

```rust
/// Vote on a governance proposal — API response
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct VoteView {
    pub id: String,
    pub proposal_id: String,
    pub human_id: Option<String>,  // null when anonymous
    pub position: String,
    pub reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl VoteView {
    /// Convert from Vote, optionally hiding identity for anonymous votes
    pub fn from_vote(v: Vote, hide_identity: bool) -> Self {
        Self {
            id: v.id,
            proposal_id: v.proposal_id,
            human_id: if hide_identity { None } else { Some(v.human_id) },
            position: v.position,
            reason: v.reason,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}
```

**Step 3: Add CreateProposalInputView**

```rust
/// Create a proposal — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateProposalInputView {
    pub id: String,
    pub content_id: String,
    pub proposer_presence_id: String,
    pub proposal_type: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub voting_anonymous: bool,
}
```

**Step 4: Add CastVoteInputView**

```rust
/// Cast or update a vote — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CastVoteInputView {
    pub human_id: String,
    pub position: String,
    #[serde(default)]
    pub reason: Option<String>,
}
```

**Step 5: Add CreateDiscussionInputView**

```rust
/// Start a discussion — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateDiscussionInputView {
    pub id: String,
    pub content_id: String,
    pub author_presence_id: String,
    pub body: String,
    #[serde(default)]
    pub parent_id: Option<String>,
}
```

**Step 6: Add PostMessageInputView**

```rust
/// Post a message (reply) in a discussion — API request
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PostMessageInputView {
    pub id: String,
    pub author_presence_id: String,
    pub body: String,
}
```

**Step 7: Commit**

```bash
git add elohim/elohim-storage/src/views.rs
git commit -m "feat(storage): add governance InputView and VoteView types"
```

---

### Task 3: Add vote CRUD functions

**Files:**
- Modify: `elohim/elohim-storage/src/db/governance.rs`

**Step 1: Add vote functions**

After the discussion functions, add:

```rust
// ============================================================================
// Votes
// ============================================================================

/// Get all votes for a proposal
pub fn query_votes(
    conn: &mut SqliteConnection,
    proposal_id: &str,
) -> Result<Vec<Vote>, StorageError> {
    use crate::db::diesel_schema::votes::dsl;
    dsl::votes
        .filter(dsl::proposal_id.eq(proposal_id))
        .order(dsl::created_at.asc())
        .load::<Vote>(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get a specific human's vote on a proposal
pub fn get_vote(
    conn: &mut SqliteConnection,
    proposal_id: &str,
    human_id: &str,
) -> Result<Option<Vote>, StorageError> {
    use crate::db::diesel_schema::votes::dsl;
    dsl::votes
        .filter(dsl::proposal_id.eq(proposal_id))
        .filter(dsl::human_id.eq(human_id))
        .first::<Vote>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Cast or update a vote (upsert via delete+insert for SQLite)
pub fn cast_vote(
    conn: &mut SqliteConnection,
    new: &NewVote,
) -> Result<Vote, StorageError> {
    use crate::db::diesel_schema::votes::dsl;
    // Delete existing vote if any (UNIQUE constraint enforcement)
    diesel::delete(
        dsl::votes
            .filter(dsl::proposal_id.eq(new.proposal_id))
            .filter(dsl::human_id.eq(new.human_id)),
    )
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Delete failed: {}", e)))?;

    diesel::insert_into(dsl::votes)
        .values(new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    dsl::votes
        .filter(dsl::id.eq(new.id))
        .first(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
```

Add the necessary imports at the top of the file: `Vote`, `NewVote` from models.

**Step 2: Commit**

```bash
git add elohim/elohim-storage/src/db/governance.rs
git commit -m "feat(storage): add vote CRUD functions (query, get, cast/upsert)"
```

---

### Task 4: Add POST handlers to governance API

**Files:**
- Modify: `elohim/elohim-storage/src/api/governance.rs`

**Step 1: Add POST handlers**

The governance dispatcher (`handle` function) currently only has GET arms. Add POST arms BEFORE the catch-all `_ =>` arm:

```rust
        // POST /api/v1/governance/proposals — Create a proposal
        (&Method::POST, "/proposals") => {
            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
            let input: CreateProposalInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let now = chrono::Utc::now().to_rfc3339();
            let new = NewProposal {
                id: &input.id,
                content_id: &input.content_id,
                proposer_presence_id: &input.proposer_presence_id,
                proposal_type: &input.proposal_type,
                title: &input.title,
                body: &input.body,
            };

            let mut conn = get_conn(pool)?;

            // Set voting_anonymous after insert if true
            let result = governance::create_proposal(&mut conn, &new)?;

            if input.voting_anonymous {
                use crate::db::diesel_schema::proposals::dsl;
                diesel::update(dsl::proposals.filter(dsl::id.eq(&input.id)))
                    .set(dsl::voting_anonymous.eq(1))
                    .execute(&mut conn)
                    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
            }

            // Re-fetch to get updated voting_anonymous
            let final_result = governance::get_proposal(&mut conn, &input.id)?
                .ok_or_else(|| StorageError::Internal("Created proposal not found".to_string()))?;
            Ok(response::created(&ProposalView::from(final_result)))
        }

        // POST /api/v1/governance/proposals/{id}/votes — Cast or update a vote
        (&Method::POST, p) if p.starts_with("/proposals/") && p.ends_with("/votes") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/votes"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
            let input: CastVoteInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let mut conn = get_conn(pool)?;

            // Get proposal to check anonymity setting
            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;

            let vote_id = format!("vote-{}-{}", id, input.human_id);
            let now = chrono::Utc::now().to_rfc3339();
            let new_vote = NewVote {
                id: &vote_id,
                proposal_id: id,
                human_id: &input.human_id,
                position: &input.position,
                reason: input.reason.as_deref(),
                anonymous: proposal.voting_anonymous,
            };

            let vote = governance::cast_vote(&mut conn, &new_vote)?;
            let hide = proposal.voting_anonymous == 1;
            Ok(response::created(&VoteView::from_vote(vote, hide)))
        }

        // GET /api/v1/governance/proposals/{id}/votes — List votes
        (&Method::GET, p) if p.starts_with("/proposals/") && p.ends_with("/votes") => {
            let id = p
                .strip_prefix("/proposals/")
                .and_then(|s| s.strip_suffix("/votes"))
                .ok_or_else(|| StorageError::InvalidInput("Proposal ID required".to_string()))?;

            let mut conn = get_conn(pool)?;
            let proposal = governance::get_proposal(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Proposal {} not found", id)))?;
            let hide = proposal.voting_anonymous == 1;
            let votes = governance::query_votes(&mut conn, id)?;
            let views: Vec<VoteView> = votes
                .into_iter()
                .map(|v| VoteView::from_vote(v, hide))
                .collect();
            Ok(response::ok(&views))
        }

        // POST /api/v1/governance/discussions — Create a discussion
        (&Method::POST, "/discussions") => {
            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
            let input: CreateDiscussionInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            let new = NewDiscussion {
                id: &input.id,
                content_id: &input.content_id,
                author_presence_id: &input.author_presence_id,
                body: &input.body,
                parent_id: input.parent_id.as_deref(),
            };

            let mut conn = get_conn(pool)?;
            let result = governance::create_discussion(&mut conn, &new)?;
            Ok(response::created(&DiscussionView::from(result)))
        }

        // POST /api/v1/governance/discussions/{id}/messages — Reply to discussion
        (&Method::POST, p) if p.starts_with("/discussions/") && p.ends_with("/messages") => {
            let discussion_id = p
                .strip_prefix("/discussions/")
                .and_then(|s| s.strip_suffix("/messages"))
                .ok_or_else(|| {
                    StorageError::InvalidInput("Discussion ID required".to_string())
                })?;

            let body = req
                .collect()
                .await
                .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
            let input: PostMessageInputView = serde_json::from_slice(&body.to_bytes())
                .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

            // A message is just a discussion with parent_id set
            let new = NewDiscussion {
                id: &input.id,
                content_id: discussion_id, // Use discussion's content_id context
                author_presence_id: &input.author_presence_id,
                body: &input.body,
                parent_id: Some(discussion_id),
            };

            let mut conn = get_conn(pool)?;
            let result = governance::create_discussion(&mut conn, &new)?;
            Ok(response::created(&DiscussionView::from(result)))
        }
```

**Step 2: Add necessary imports**

At the top of `governance.rs`, add:
```rust
use crate::views::{
    CreateProposalInputView, CastVoteInputView, CreateDiscussionInputView,
    PostMessageInputView, VoteView,
};
use crate::db::models::{NewVote};
```

Also add `use diesel::prelude::*;` if not already present (needed for the `voting_anonymous` update).

**Step 3: Register POST routes in http.rs**

In the route registration section (around line 5200), add POST routes alongside the existing GET routes:

```rust
        .route(
            Route::post("/api/v1/governance/proposals")
                .handler("create_proposal")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/proposals/{id}/votes")
                .handler("cast_vote")
                .build(),
        )
        .route(
            Route::get("/api/v1/governance/proposals/{id}/votes")
                .handler("list_votes")
                .cache_ttl(30)
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/discussions")
                .handler("create_discussion")
                .build(),
        )
        .route(
            Route::post("/api/v1/governance/discussions/{id}/messages")
                .handler("post_message")
                .build(),
        )
```

**Step 4: Verify it compiles**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -10`
Expected: No errors.

**Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/governance.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add POST handlers for proposals, votes, and discussions"
```

---

### Task 5: Regenerate TypeScript types

**Step 1: Run export_bindings**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -10`

**Step 2: Verify new types exist**

Check that these files were generated/updated in `elohim/sdk/storage-client-ts/src/generated/`:
- `VoteView.ts` (new)
- `CreateProposalInputView.ts` (new)
- `CastVoteInputView.ts` (new)
- `CreateDiscussionInputView.ts` (new)
- `PostMessageInputView.ts` (new)
- `ProposalView.ts` (updated with `votingAnonymous: boolean`)

**Step 3: Add new types to index.ts**

In `elohim/sdk/storage-client-ts/src/generated/index.ts`, add exports for the new types.

**Step 4: Commit**

```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "chore: regenerate TypeScript types with governance write path types"
```

---

### Task 6: Add POST methods to GovernanceApiService

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.ts`
- Modify: `app/elohim-app/src/app/elohim/services/governance-api.service.spec.ts` (if exists)

**Step 1: Add write methods**

Add these methods to `GovernanceApiService`:

```typescript
createProposal(input: CreateProposalInputView): Observable<ProposalView> {
  return this.http.post<ProposalView>('/api/v1/governance/proposals', input);
}

castVote(proposalId: string, input: CastVoteInputView): Observable<VoteView> {
  return this.http.post<VoteView>(
    `/api/v1/governance/proposals/${proposalId}/votes`,
    input
  );
}

getVotes(proposalId: string): Observable<VoteView[]> {
  return this.http
    .get<VoteView[]>(`/api/v1/governance/proposals/${proposalId}/votes`);
}

createDiscussion(input: CreateDiscussionInputView): Observable<DiscussionView> {
  return this.http.post<DiscussionView>('/api/v1/governance/discussions', input);
}

postMessage(discussionId: string, input: PostMessageInputView): Observable<DiscussionView> {
  return this.http.post<DiscussionView>(
    `/api/v1/governance/discussions/${discussionId}/messages`,
    input
  );
}
```

Import the new types from `@elohim/storage-client` (or from the generated types path).

**Step 2: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/governance-api.service.ts
git commit -m "feat(elohim): add POST methods to GovernanceApiService"
```

---

### Task 7: Replace localStorage in GovernanceService

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/governance.service.ts`
- Modify: `app/elohim-app/src/app/elohim/services/governance.service.spec.ts`

**Step 1: Replace submitProposal**

Change `submitProposal()` from localStorage save to:

```typescript
submitProposal(submission: ProposalSubmission): Observable<ProposalRecord> {
  const id = `proposal-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const input: CreateProposalInputView = {
    id,
    contentId: submission.relatedEntityId ?? '',
    proposerPresenceId: this.identityService.humanId() ?? '',
    proposalType: submission.proposalType,
    title: submission.title,
    body: `${submission.description}\n\n**Rationale:** ${submission.rationale}`,
    votingAnonymous: false,
  };

  return this.governanceApi.createProposal(input).pipe(
    map(view => this.proposalViewToRecord(view)),
    tap(() => this.clearCache()),
  );
}
```

**Step 2: Replace voteOnProposal**

Change from localStorage to:

```typescript
voteOnProposal(vote: Vote): Observable<boolean> {
  const input: CastVoteInputView = {
    humanId: this.identityService.humanId() ?? '',
    position: vote.position,
    reason: vote.reasoning ?? null,
  };

  return this.governanceApi.castVote(vote.proposalId, input).pipe(
    map(() => true),
    catchError(() => of(false)),
  );
}
```

**Step 3: Replace getMyVote**

```typescript
getMyVote(proposalId: string): Observable<Vote | null> {
  const humanId = this.identityService.humanId();
  if (!humanId) return of(null);

  return this.governanceApi.getVotes(proposalId).pipe(
    map(votes => {
      const mine = votes.find(v => v.humanId === humanId);
      if (!mine) return null;
      return { proposalId, position: mine.position as Vote['position'], reasoning: mine.reason ?? undefined };
    }),
    catchError(() => of(null)),
  );
}
```

**Step 4: Replace postMessage**

```typescript
postMessage(message: DiscussionMessage): Observable<boolean> {
  const input: PostMessageInputView = {
    id: `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    authorPresenceId: this.identityService.humanId() ?? '',
    body: message.content,
  };

  return this.governanceApi.postMessage(message.discussionId, input).pipe(
    map(() => true),
    tap(() => this.clearCache()),
    catchError(() => of(false)),
  );
}
```

**Step 5: Remove localStorage helpers**

Delete:
- `saveLocalChallenge()` method
- `saveLocalProposal()` method
- `getLocalMessages()` method (replace with API call)
- All `localStorage.getItem/setItem` calls with `lamad-governance-` prefix
- The `STORAGE_PREFIX` constant

**Step 6: Add GovernanceApiService injection**

Add `private readonly governanceApi = inject(GovernanceApiService);` to the service constructor area.

**Step 7: Update tests**

Update `governance.service.spec.ts` to mock `GovernanceApiService` instead of `localStorage`. The tests that verified localStorage saves should now verify HTTP calls.

**Step 8: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "governance.service" 2>&1 | tail -20`

**Step 9: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/governance.service.ts app/elohim-app/src/app/elohim/services/governance.service.spec.ts
git commit -m "feat(elohim): replace localStorage governance MVP with real API calls"
```

---

### Task 8: Build CollectiveDetailComponent

**Files:**
- Create: `app/elohim-app/src/app/qahal/components/collective-detail/collective-detail.component.ts`
- Create: `app/elohim-app/src/app/qahal/components/collective-detail/collective-detail.component.spec.ts`

**Step 1: Write the test**

```typescript
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute } from '@angular/router';
import { of } from 'rxjs';
import { CollectiveDetailComponent } from './collective-detail.component';

describe('CollectiveDetailComponent', () => {
  let component: CollectiveDetailComponent;
  let fixture: ComponentFixture<CollectiveDetailComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CollectiveDetailComponent, HttpClientTestingModule],
      providers: [
        {
          provide: ActivatedRoute,
          useValue: { paramMap: of(new Map([['id', 'household-dowell']])) },
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(CollectiveDetailComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should default to members tab', () => {
    expect(component.activeTab()).toBe('members');
  });

  it('should switch tabs', () => {
    component.setTab('proposals');
    expect(component.activeTab()).toBe('proposals');
  });
});
```

**Step 2: Write the component**

Shows:
- Collective header (name, description, governance layer badge)
- Three tabs: Members | Proposals | Discussions
- Members tab: grid of FaceCards (reusing from Sprint 1)
- Proposals tab: list of active proposals with ProposalVoteComponent
- Discussions tab: discussion threads
- "New Proposal" button on proposals tab
- "New Discussion" button on discussions tab

Use signals for state. Load collective, participants, proposals, and discussions on init via the route param `:id`.

The component should use:
- `CollectiveService.getCollective(id)` and `.getParticipants(id)` for members
- `GovernanceApiService.queryProposals(collectiveId)` for proposals
- `GovernanceApiService.queryDiscussions(collectiveId)` for discussions
- `FaceCardComponent` for member display
- Existing `ProposalVoteComponent` for voting UI

**Step 3: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "collective-detail" 2>&1 | tail -15`

**Step 4: Commit**

```bash
git add app/elohim-app/src/app/qahal/components/collective-detail/
git commit -m "feat(qahal): add CollectiveDetailComponent with members/proposals/discussions tabs"
```

---

### Task 9: Wire collective detail route

**Files:**
- Modify: `app/elohim-app/src/app/qahal/community.routes.ts`
- Modify: `app/elohim-app/src/app/qahal/components/community-directory/community-directory.component.ts`
- Modify: `app/elohim-app/src/app/qahal/index.ts`

**Step 1: Add route**

In `community.routes.ts`, add child route:

```typescript
{
  path: 'collective/:id',
  loadComponent: async () =>
    import('./components/collective-detail/collective-detail.component').then(
      m => m.CollectiveDetailComponent
    ),
  data: { title: 'Collective' },
},
```

**Step 2: Link from directory**

In `CommunityDirectoryComponent`, update the groups view so that clicking a group header navigates to `/community/collective/{id}`. Add `Router` injection and a `navigateToCollective(id: string)` method.

**Step 3: Update barrel exports**

Add `CollectiveDetailComponent` export to `qahal/index.ts`.

**Step 4: Commit**

```bash
git add app/elohim-app/src/app/qahal/community.routes.ts app/elohim-app/src/app/qahal/components/community-directory/ app/elohim-app/src/app/qahal/index.ts
git commit -m "feat(qahal): wire collective detail route and link from directory"
```

---

### Task 10: Write a2o scenario

**Files:**
- Create: `genesis/a2o/features/qahal/collective-governance.feature`

**Step 1: Write the scenario**

```gherkin
Feature: Collective governance
  As a member of a small group
  I want to propose and vote on group decisions
  So that our group self-governs through consent

  Background:
    Given I am "Matthew" in the "Valley Bible Study" collective

  Scenario: Create a proposal
    When I create a proposal titled "Study Romans next quarter"
    With type "sense-check"
    And description "Romans provides foundational theology for our group's next season"
    Then the proposal appears in the collective's proposals tab
    And the proposal status is "voting"

  Scenario: Vote on a proposal
    Given a proposal "Study Romans next quarter" exists in my collective
    When I vote "agree" on the proposal
    Then my vote is recorded
    And the vote count updates

  Scenario: Block a proposal with justification
    Given a proposal "Study Romans next quarter" exists in my collective
    When I vote "block" on the proposal
    Then I must provide a written reason
    And the block is visible to other members

  Scenario: Anonymous voting
    Given a proposal with anonymous voting enabled
    When members vote on the proposal
    Then vote counts are visible
    But individual voters are not identified

  Scenario: Change a vote
    Given I have voted "agree" on a proposal
    When I change my vote to "disagree"
    Then my previous vote is replaced
    And the vote counts update accordingly
```

**Step 2: Commit**

```bash
git add genesis/a2o/features/qahal/
git commit -m "feat(a2o): add collective governance scenarios — first qahal a2o coverage"
```

---

### Task 11: Run full test suite and verify

**Step 1: Run governance tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "governance|collective-detail|proposal-vote" 2>&1 | tail -20`

**Step 2: Run Rust check**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5`

**Step 3: Run lint**

Run: `cd app/elohim-app && pnpm run lint 2>&1 | tail -10`

**Step 4: Fix any issues and commit**

---

### Reference: Key file paths

**Rust (elohim-storage):**
- `src/db/diesel_schema.rs` — table macros (add votes table, update proposals)
- `src/db/models.rs` — Vote, NewVote structs + Proposal update (~line 1466)
- `src/db/governance.rs` — CRUD functions (add vote functions)
- `src/views.rs` — VoteView + InputView types (~line 3340)
- `src/api/governance.rs` — POST handlers (add after existing GET handlers)
- `src/http.rs` — route registration (~line 5200)

**Angular:**
- `app/elohim-app/src/app/elohim/services/governance-api.service.ts` — add POST methods
- `app/elohim-app/src/app/elohim/services/governance.service.ts` — replace localStorage
- `app/elohim-app/src/app/qahal/components/collective-detail/` — new component
- `app/elohim-app/src/app/qahal/community.routes.ts` — add route

**A2O:**
- `genesis/a2o/features/qahal/collective-governance.feature` — new
