# Journal Routing — Design

**Goal:** Wire the journal "finish" event — the seam where the elohim reads intent from a journal entry and generates protocol artifacts. Everything wired except the inference sidecar (stubbed with keyword matching).

---

## The Insight

The journal is the protocol's mouth. Every human need starts as private writing. The "finish" flow helps the human reach a stopping point, feel good about where they left it, and see their words become protocol-legible artifacts.

Two kinds of output from a single journal entry:

1. **Filing card** — always present. The elohim suggests where to file the journal in the human's shefa drive. Default action. The inverse of Google Drive root-dumping — every journal lands in context.

2. **Derivative cards** — new artifacts *extracted from* the journal. REA requests, governance proposals, content contributions. Stories the elohim recognized and offers to spin off as protocol artifacts with their own lifecycle.

---

## State Machine

```
writing ──[Finish]──→ confirming ──[Looks good]──→ routing ──[cards posted]──→ routed
   ↑                     │                            │
   └──[Edit]─────────────┘                            │
   └──[Edit]──────────────────────────────────────────┘
```

| State | Main Panel | Editor | Sidebar |
|-------|-----------|--------|---------|
| `writing` | Editor active | Editable, "Finish" button | Available |
| `confirming` | Read-only text + intent summary + "Looks good" / "Edit" | Read-only | Available |
| `routing` | Collapsed text + EPR suggestion cards | Collapsed (title + first lines) | Available |
| `routed` | Posted confirmations + reach badges + "Write another" | Gone | Available |

---

## Data Model

```typescript
interface IntentAnalysis {
  summary: string;           // "This is about home maintenance and a question about community funds"
  detectedTypes: DestinationType[];
  suggestedPath: string;     // filing path for the journal itself
}

type DestinationType = 'content' | 'exchange-request' | 'governance-proposal';

interface RoutingSuggestion {
  id: string;
  kind: 'filing' | 'derivative';
  destinationType: 'journal-filing' | DestinationType;
  title: string;
  summary: string;
  suggestedPath: string;
  reach: ReachTier;
  contextMetadata: Record<string, unknown>;
  status: 'suggested' | 'posting' | 'posted' | 'dismissed';
}
```

---

## Component Architecture

### JournalPageComponent (existing — becomes orchestrator)
- Owns `JournalRoutingService` (component-scoped)
- `@switch` on service state to render the right view
- Passes content ID and text to children

### JournalEditorComponent (existing — minor changes)
- New `readonly` input for confirming/routing states
- Emits `finish` event when button clicked
- Collapsed view (title + first few lines) for routing state

### JournalConfirmComponent (new)
- Presentational: shows confirmed text, intent summary
- "Looks good" and "Edit" buttons
- Shimmer animation while analyzing (reuses GateArtifactCard pattern)

### JournalRoutingCardsComponent (new)
- Receives `RoutingSuggestion[]`
- Filing card first, visually distinct (subtle "already handled" feel)
- Derivative cards below with action buttons
- Each card emits `post` or `dismiss`

### JournalRoutedComponent (new)
- Completion state — what was posted where
- Reach badges per card (reuses GateArtifactCard pattern)
- "Write another" button

### JournalRoutingService (new, component-scoped)
- State: `writing | confirming | routing | routed`
- `finish(text: string)` → analyze → confirming
- `confirm()` → generate suggestions → routing
- `edit()` → writing
- `postCard(id)` → post individual card
- `dismissCard(id)` → mark dismissed

---

## Sidecar Seams

Two methods that get replaced when inference sidecar deploys:

### `analyzeIntent(text: string): Observable<IntentAnalysis>`

**Stub:** Keyword matching (like CannedResponseService).
- "need/want/broken/repair/help" → `exchange-request`
- "should/vote/policy/propose/fund" → `governance-proposal`
- "learned/discovered/guide/how-to/tutorial" → `content`
- Path guessed from title keywords or defaults to `/journal/general/`
- Returns after `timer(800)` to exercise shimmer UI

**Real:** POST to `/api/v1/elohim/invoke` with journal text + context.

### `generateSuggestions(text, intent): Observable<RoutingSuggestion[]>`

**Stub:** Always returns filing card. Adds derivative card for each detected type with generated title/summary from simple templates.

**Real:** POST to `/api/v1/elohim/invoke` with richer context (journal text + intent + human's drive structure + recent activity).

---

## Posting Behavior

### Filing card (`journal-filing`)
- `StorageApiService.updateContent()` — updates `parentPath` to move journal to suggested folder
- Fires `content.updated` through EventBus/SSE
- Reach: `private`
- Auto-posts after timeout if human doesn't interact (journal always saved somewhere sensible)

### Derivative cards
- `StorageApiService.createContent()` — creates NEW content node:
  - `contentType` matching destination
  - `contentBody` — elohim-generated text (stubbed: journal excerpt)
  - `parentPath` — pillar-appropriate path (`/lamad/contributions/`, `/shefa/exchange/`, `/qahal/proposals/`)
  - `metadata.sourceJournalId` — back-reference to journal
- Fires `content.created` through EventBus/SSE
- Reach: per card suggestion

All writes go through `StorageApiService` (storage HTTP). Conductor-first (DHT notarization) replaces this seam later.

---

## What This Sprint Does NOT Include

- Real inference (sidecar not deployed)
- Conductor-first writes (storage direct for now)
- Drive folder management UI (just the filing suggestion)
- Card editing before posting (post as-is or dismiss)
- SSE-driven card updates (cards update via HTTP response, not event stream)
