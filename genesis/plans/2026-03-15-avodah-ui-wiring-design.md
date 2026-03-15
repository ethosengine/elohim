# Avodah UI Wiring — Design

**Date:** 2026-03-15
**Status:** Approved

## Overview

Three features that make the avodah pillar interactive: drag-and-drop on the kanban board, a full-page story detail view, and inline story creation. Inspired by Taiga.io's beautifully simple, story-centered design.

---

## 1. Drag-and-Drop on Kanban Board

Angular CDK `DragDropModule`. Each column is a `cdkDropList`, each story card is a `cdkDrag`. On drop, call `avodahApi.updateStoryStatus(storyId, newColumnId, column.isTerminal)`.

**Optimistic UI:** move the card immediately in the local array, revert on error.

**Visual feedback:** subtle lift shadow while dragging, column background highlight on hover. No complex animations — CDK defaults.

---

## 2. Story Detail — Full-Page Route

Route: `/avodah/projects/:id/stories/:storyId`

Stories are EPR ContentNodes — the detail view follows the lamad content viewer pattern (full-page, not a slide-over panel).

### Layout

```
┌──────────────────────────────────────────────────┐
│  ← Back to Board          proj: Household 2026   │
├──────────────────────────────────────────────────┤
│                                                  │
│  Repair the back fence                    [Edit] │
│  ─────────────────────────────────────────────── │
│  Two panels came loose after the last storm...   │
│                                                  │
│  ┌─────────┐ ┌──────────┐ ┌──────────────────┐  │
│  │Status   │ │Priority  │ │Visibility        │  │
│  │●backlog │ │●high     │ │🔒private → 👥 → ⚖│  │
│  └─────────┘ └──────────┘ └──────────────────┘  │
│                                                  │
│  Assigned: @matthew        Points: 5             │
│  Tags: #maintenance #yard                        │
│                                                  │
│  ┌─ Cadence ─────────────────────────────────┐   │
│  │  (none — one-time story)                  │   │
│  └───────────────────────────────────────────┘   │
│                                                  │
│  ┌─ Attestation Gates ───────────────────────┐   │
│  │  (none — open to all)                     │   │
│  └───────────────────────────────────────────┘   │
│                                                  │
└──────────────────────────────────────────────────┘
```

### Interactions

- **Status/priority:** clickable dropdowns, inline edit, PATCH on change
- **Visibility:** 3-state toggle (private → community → exchange)
- **Title/description:** click to edit (input swap), PATCH on blur/Enter
- **Back button:** navigates to referrer view (board/backlog/tasks)

### Read-only sections (future editability)

- Assignee (needs member list UI)
- Attestation gates (needs lamad path browser)
- Cadence configuration (needs form design)
- Exchange publishing (needs shefa integration)

---

## 3. Inline Story Creation

Taiga-style: no modal, no form. Type a title, hit Enter, story appears.

### On board

"+ Add story" at column bottom becomes a text input on click. Enter creates story with `status: columnId` and defaults. Card appears immediately (optimistic).

### On backlog

"+ New Story" button reveals an inline row with title input. Enter creates with `status: 'backlog'`.

### Service call

Both call a new `avodahApi.createStory(projectId, title, status)` method which builds a `CreateContentInputView` with defaults and calls `storageApi.createContent()`.

---

## What Stays Out of Scope

- Assignee picker (needs member list UI)
- Attestation gate selection (needs lamad path browser)
- Exchange publishing flow (needs shefa integration)
- Cadence configuration form
- Story deletion
