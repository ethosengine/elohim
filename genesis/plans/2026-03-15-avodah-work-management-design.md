# Avodah — Work Management Pillar Design

**Date:** 2026-03-15
**Status:** Approved

## Overview

Avodah (עֲבוֹדָה — work, service, worship) is a new Angular pillar for work management. It is the Elohim Protocol's equivalent of Taiga.io: projects, kanban boards, backlogs, and task lists — but built on EPR ContentNodes, gated by lamad attestations, and connected to the shefa exchange.

Stories start as private work items (personal chores, household tasks) and can be promoted to community or exchange visibility. The cadence system handles recurring work that never truly completes. The lamad attestation gate enables open work between any parties, qualified by proven mastery rather than credentials.

---

## Pillar Structure

```
app/elohim-app/src/app/avodah/
  avodah.routes.ts
  index.ts
  components/
    avodah-layout/        — shell with left sidebar nav
    avodah-home/          — overview / activity feed
    project-list/         — all projects for the user
    project-board/        — kanban view for a project
    project-backlog/      — flat story list / backlog
    task-list/            — recurring cadence items only
    story-detail/         — single story / work item
  models/
    work-story.model.ts
    work-project.model.ts
  services/
    avodah-api.service.ts
```

### Routes

```
/avodah                                     — home / activity feed
/avodah/projects                            — all projects
/avodah/projects/:id/board                  — kanban board
/avodah/projects/:id/backlog               — story list / backlog
/avodah/projects/:id/tasks                 — recurring task list
/avodah/projects/:id/stories/:storyId      — story detail
```

---

## Data Model

Stories and projects are EPR **ContentNodes**. Standard ContentNode fields carry EPR context (id, title, description, tags, relatedNodeIds). The `content` field holds the avodah-specific payload.

### `work-story` ContentNode

```typescript
// contentType: 'work-story'
// content field (JSON):
{
  projectId: string;
  status: 'backlog' | 'todo' | 'in-progress' | 'review' | 'done';
  visibility: 'private' | 'community' | 'exchange';
  priority: 'low' | 'medium' | 'high' | 'urgent';
  storyPoints?: number;
  assigneeId?: string;
  attestationGates?: string[];     // lamad ContentNode IDs required to bid/accept
  exchangeRequestId?: string;      // shefa ServiceRequest ref (set on exchange publish)
  cadence?: {
    interval: 'daily' | 'weekly' | 'monthly' | 'custom';
    customIntervalDays?: number;
    resetToStatus: 'backlog' | 'todo';
    nextOccurrence: string;        // ISO date
  };
}
```

### `work-project` ContentNode

```typescript
// contentType: 'work-project'
// content field (JSON):
{
  columns: {
    id: string;
    name: string;
    color?: string;
    isTerminal?: boolean;          // done-states reset cadence stories
  }[];
  visibility: 'private' | 'community';
  memberIds?: string[];
}
```

Default column set: **Backlog → To Do → In Progress → Review → Done**.

---

## Views

### Left Sidebar Nav (all project views)

```
┌─────────────────────┐
│  [▼ Project Name]   │  ← project switcher dropdown
├─────────────────────┤
│  ▦  Board           │
│  ≡  Backlog         │
│  ↺  Tasks           │  ← recurring items only
│  ⚙  Settings        │
├─────────────────────┤
│  + New Project      │
└─────────────────────┘
```

Modeled after shefa dashboard's panel-nav but rendered as a vertical sidebar.

### Board View (Kanban)

Columns rendered from `project.content.columns[]`. Stories are draggable cards. Column header shows count and inline "Add story" action. Story status updates on drag-drop.

**Story Card:**
```
┌──────────────────────────────────┐
│  🎓 ◈                    #work-7 │
│  Fix the kitchen faucet          │
│  ─────────────────────────────── │
│  ● High   ◷ 3pts   @unassigned   │
│  #plumbing  #home                │
└──────────────────────────────────┘
```

Card badges:
- 🎓 — has lamad attestation gate(s)
- ◈ — published to shefa exchange
- ↺ — recurring cadence configured

### Backlog View

Flat sortable/filterable list. Columns: title, status pill, priority dot, story points, assignee, cadence indicator. Filter bar: status, priority, visibility, tags. "New Story" opens inline form or slide-out panel.

### Task List View

Shows only stories with `cadence` set, grouped by interval (Daily / Weekly / Monthly). Each row: title, next occurrence, last completed, streak count. Completing a task resets status to `resetToStatus` and advances `nextOccurrence`. Cadence advance logic lives in the service layer.

---

## Cross-Pillar Integration

### Lamad — Attestation Gates

Story detail shows required learning path(s) with the viewer's mastery status. If unmet, Accept/Bid is disabled: "Complete [path] to unlock." Gate check is a read from lamad's mastery service at accept-time — no new backend work.

### Shefa — Exchange Publishing

Three-state visibility toggle: `private → community → exchange`. Promoting to `exchange`:
1. Creates a `ServiceRequest` in shefa
2. Stores ref in `story.content.exchangeRequestId`
3. Card shows ◈ badge

Demoting removes the request. One-click from story detail or card context menu.

### Qahal — Community Visibility

`visibility: 'community'` makes the story readable within the person's household/congregation, following the existing qahal community graph. No new infrastructure — the EPR visibility flag is honored by the existing system.

### Elohim Agent — Surfacing Needs

Out of scope for MVP UI. The model supports it: the agent reads private stories, detects life disruption signals via imagodei presence/health data, and can flip `visibility` to `community` with a generated description update (e.g., "Bob is recovering from surgery this week — these tasks need coverage"). The story is the same ContentNode; the agent changes two fields with consent.

---

## What Stays in Shefa

The exchange marketplace (`/shefa/exchange`) remains in shefa. Avodah is the work management layer. Publishing a story to the exchange is a cross-pillar action originating in avodah. Browsing and matching open requests from others remains a shefa concern.
