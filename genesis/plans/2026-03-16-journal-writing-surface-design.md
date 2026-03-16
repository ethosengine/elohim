# Journal Writing Surface — Design

**Goal:** A distraction-free writing surface where humans create content, with an elohim sidebar available for conversation. The journal entry is a private content node. The elohim is present but never intrusive.

---

## Core Principle

The human is in the driver's seat. The artifact belongs to them. The elohim is in the sidebar, available but dormant until engaged. No interruptions, no badges, no "the elohim has something to say."

This is where content is *born* in the protocol. A journal entry is a private content node — the same content type that could eventually be published to a steward feed, expressed as a need on the exchange, or surfaced as governance feedback. But that routing is a future sprint. This sprint builds the writing space and the relationship with the elohim.

---

## Page Layout

```
┌─────────────────────────────────────┬──────────────────────┐
│                                     │   Elohim Sidebar     │
│         Writing Surface             │   (collapsed by      │
│                                     │   default — thin     │
│   Title field                       │   strip with icon)   │
│   ─────────────────                 │                      │
│                                     │  When expanded:      │
│   Body editor                       │  ┌────────────────┐  │
│   (textarea, markdown)              │  │ Message list    │  │
│                                     │  │ (scrollable)    │  │
│                                     │  │                 │  │
│                                     │  ├────────────────┤  │
│                                     │  │ GateArtifact    │  │
│                                     │  │ Card (input)    │  │
│                                     │  └────────────────┘  │
│                                     │                      │
│   Last saved: just now              │                      │
└─────────────────────────────────────┴──────────────────────┘
```

**Route:** `/shefa/journal/:id` — a child of the shefa layout, alongside resources/exchange/etc.

**Two panels:**
- **Main (left):** Writing surface — title + body, maximum space, no chrome
- **Sidebar (right):** Elohim chat — collapsed to ~40px strip by default, expands to ~300px

---

## Data Model

**No new table.** Journal entries are content nodes stored via the existing content CRUD.

A journal entry is a content node with:
- `contentType: 'journal'`
- Private by default (no reach beyond the author)
- `contentFormat: 'markdown'`
- Standard content node fields: `id`, `title`, `contentBody`, `tags`, `createdAt`, `updatedAt`

When the human chooses to publish (future sprint), the content node gains reach and enters shefa value flows like any other content.

**Autosave:** Debounced PATCH on the body field (1.5s after typing stops). Title saves on blur. No explicit save button — the writing just saves, like Google Docs. A subtle "Saved" / "Saving..." indicator at the bottom.

**Sidebar messages:** Component state only for this sprint. Canned responses don't need persistence. When real inference arrives, we add a message persistence layer.

---

## Component Architecture

All components live in the **shefa** pillar since journal artifacts are stewardship outputs.

```
JournalPageComponent (route host)
├── JournalEditorComponent (title + body + autosave)
└── ElohimSidebarComponent (collapsible chat panel)
    ├── ElohimMessageListComponent (conversation history)
    └── GateArtifactCardComponent (text input — no gateApiCall)
```

### JournalPageComponent
- Route: `/shefa/journal/:id`
- Owns the two-panel layout (CSS grid or flexbox)
- Loads the content node by ID on init
- Passes content to editor, passes sidebar collapse state

### JournalEditorComponent
- Title field (input, saves on blur)
- Body field (textarea, autosave on debounce)
- Injects `StorageApiService` for PATCH calls
- Emits save status for the "Saved" indicator
- Minimal styling — the writing is the focus

### ElohimSidebarComponent
- Collapsed state: thin vertical strip with elohim icon, click to expand
- Expanded state: ~300px panel with message list + input
- Contains `GateArtifactCardComponent` without `gateApiCall` — the card's draft→posted state machine drives each message turn
- On posted: appends human message + canned response to message list
- Injects `CannedResponseService` for response generation

### ElohimMessageListComponent
- Scrollable list of `{ role: 'human' | 'elohim', text: string }` messages
- Auto-scrolls to bottom on new message
- Simple bubble styling: human right-aligned, elohim left-aligned

---

## Elohim Sidebar — Canned Response Engine

For this sprint, the sidebar responds with keyword-matched canned messages. The tone is **present but unhurried** — never eager, never prescriptive.

### CannedResponseService

Injectable service with a `respond(text: string): string` method.

Response matching (first match wins):

| Keywords | Response |
|----------|----------|
| "what do you think", "thoughts" | "I can see you're working through something here. When you're ready, I can help you find where this belongs." |
| "help", "stuck", "don't know" | "Take your time. Sometimes the writing itself is the point." |
| "publish", "share", "post" | "When you're ready to share this, we can talk about where it would have the most impact. That's a conversation for when it feels right to you." |
| "done", "finished", "ready" | "It reads well. What would you like to do with it?" |
| "delete", "trash", "scrap" | "Your words, your call. Want to keep it as a draft instead?" |
| (default) | "I'm here when you need me." |

### GateArtifactCard Integration

The sidebar uses `GateArtifactCardComponent` — the same component used in comments and feedback modal — but:
- No `gateApiCall` (no gate evaluation, no HTTP call)
- `mutationType: 'journal-chat'`
- On posted → sidebar appends human message + canned response
- Card resets to draft for next message

This means when real inference arrives, we just add a `gateApiCall` that hits the sidecar — same wiring pattern as comments and feedback modal.

---

## Shefa Route Integration

Add to `shefa.routes.ts`:

```typescript
{
  path: 'journal/:id',
  loadComponent: async () =>
    import('./components/journal-page/journal-page.component')
      .then(m => m.JournalPageComponent),
  data: { title: 'Shefa - Journal' },
}
```

The journal page is a focused editor view — like opening a Google Doc from Drive. The resource explorer (already in shefa) serves as the "Drive view" where humans see their content nodes and can create new journal entries.

---

## What This Sprint Does NOT Include

- **Routing/publishing** — no destination picker, no gate evaluation on publish
- **Real inference** — canned responses only, no sidecar/Claude API
- **Background observation** — no imagodei memory extraction from journal content
- **Journal listing/management** — use existing resource explorer for now
- **Rich text editor** — plain textarea with markdown, not a WYSIWYG editor
- **Mobile layout** — desktop two-panel layout only

---

## Build Order

1. **Route + page scaffold** — `/shefa/journal/:id`, two-panel layout, load content node
2. **JournalEditorComponent** — title + body textarea, debounced autosave via StorageApiService PATCH
3. **CannedResponseService** — keyword-matched response engine
4. **ElohimMessageListComponent** — conversation bubble list
5. **ElohimSidebarComponent** — collapsible panel wiring message list + GateArtifactCard
6. **Integration + styling** — wire everything together, verify the writing experience
