# Avodah Attachments — Design

**Date:** 2026-03-16
**Status:** Approved

## Overview

Work items (stories) can have attachments — links to existing EPR ContentNodes via `ATTACHED_TO` relationships. No new content types, no blob upload. Any ContentNode in the system can be attached to a story.

---

## Data Model

```
Story (work-story)  --ATTACHED_TO-->  ContentNode (any type)
```

New relationship type `ATTACHED_TO` added to `ContentRelationshipType` enum. The story is the source, the attached content is the target.

---

## Service Layer

Three new methods on `AvodahApiService`:

- `getAttachments(storyId)` — fetches relationships of type `ATTACHED_TO` where source = storyId, resolves target ContentNodes
- `attachContent(storyId, contentId)` — creates `ATTACHED_TO` relationship via `StorageApiService.createRelationship()`
- `detachContent(relationshipId)` — deletes the relationship

---

## UI — Story Detail Attachments Section

Added to the bottom of the story detail view:

```
┌─ Attachments ────────────────────────────┐
│  📋 Quiz: Manifesto Foundations    [✕]   │
│  📄 Reference: Plumbing Guide     [✕]   │
│                                          │
│  [+ Attach content]                      │
└──────────────────────────────────────────┘
```

- **List**: Attached ContentNodes with type icon (from `CONTENT_TYPE_ICONS`) and title
- **Remove**: [✕] deletes the relationship
- **Add**: "+ Attach content" reveals an inline text input for content ID (MVP)

---

## What Stays Out of Scope

- Content search/browse picker (MVP uses raw content ID input)
- Blob upload (attachments are existing ContentNodes)
- Creating new content inline from the attachment flow
- Inverse relationship (`ATTACHED_TO` is one-directional from story to content)
