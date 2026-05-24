---
title: Google Drive — substrate-native file store + collaboration
tier: architecture
status: Composition draft (primitives mapped; full walkthrough pending)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: lamad (document content), elohim (substrate), imagodei (collaborator identity), qahal (multi-party reach gating)
realizes:
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (data sovereignty for everyday productivity)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md (document bodies in iroh-blob)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md (web2 projection for shared docs)
  - (planned reference to Automerge CRDT skill for real-time collab)
informs:
  - app/elohim-app/src/app/lamad/ + elohim pillar for document store
  - elohim/sdk/domains/lamad/manifest.json (content_types: document, folder; action verbs: edit, share)
defers:
  - Office-suite editing UI (Docs / Sheets / Slides equivalents — application layer, not substrate)
  - Cross-format conversion (substrate is format-agnostic; converters are app-layer)
---

## The grandma test

A user opens the app. They see: their folders, their documents, recent edits, shared-with-me. They click a document — it opens. They edit; someone else editing the same doc sees changes in real time. They share a folder with three colleagues; the colleagues see it appear. Drive-shape — but each user owns their bytes, no Google indexing them.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Document | EPR (`content_type: "document"`) | small bodies inline; large in `media_cid → iroh-blob` |
| Folder | EPR (`content_type: "folder"`) | `parent_epr_cid` references; subordinate child documents |
| Edit | Event (`action: "edit"`) | graduates from edit-keystroke Observations at session boundaries; carries Automerge delta in payload |
| Real-time sync | Automerge CRDT delta on libp2p direct-message ALPN | only between actively-open editors; not gossiped |
| Sharing | Event (`action: "grant-reach"`) | new collaborator gains reach to folder/document |
| Comment | FeedbackSignal (`signal_kind: "comment"`) | DHT-notarized; threaded via parent_signal_cid |
| Suggestion | FeedbackSignal (`signal_kind: "suggestion"`) | + accept/reject Event resolves it |
| Activity log | derived view over edit / share / comment Events under document | |
| Search index | local SQL FTS over document body + metadata + OCR Attestations | |
| Trash | document EPR with `lifecycle_state: "shelved"` (Gap 4) | recoverable until purge |
| Permanent delete | Event (`action: "dispose"`) → `closed` (Gap 8) | future Events targeting closed doc fail validation |

## Stress points the substrate handles

- **Working-set caching**: documents you're actively touching cache in SQL projection (~GB on laptop); cold corpus in quilt with on-demand fetch
- **Search everything**: local-FTS over body + metadata + OCR Attestations + auto-tag Attestations; no Google-shape central index needed because each user searches their own corpus
- **Real-time collab**: Automerge delta-sync on libp2p direct-message; only active editors exchange CRDT ops; document EPR records edit-Events at session boundaries (not per-keystroke DHT writes)
- **Cross-account sharing**: `grant-reach` Event widens document reach to specific recipients; collaborator's local sync starts pulling from the document's iroh-blob; their elohim-node maintains a local projection
- **Versioning**: every edit-Event preserves payload Automerge ops → full history reconstructable; Resource (`resource_classified_as: "document-state"`) derived from event-history (same shape as Monarch's balance-from-events)

## Scale answer

- Per-user working set: ~100 GB iroh-blob (typical Drive usage); ~1 GB SQL projection (metadata + recent-doc bodies + search indices)
- Cold corpus: in quilt with K-of-N erasure; retrievable on demand
- Per-document gossip cost: doc EPR ~10 KB; edit-Events at session boundaries ~few KB each
- Collab traffic: libp2p direct-message between active editors only; nothing gossiped globally during typing
- 8B users × 100 GB = 800 EB working sets — but it's all peer-distributed; no central storage layer

## Bridges to legacy

- **bridges/google-drive/** (import) — Google Drive Takeout export → batch-graduated document creation under stewardship-elohim
- **bridges/dropbox/** (import) — same pattern for Dropbox / Box / OneDrive
- **Cash-out**: every document exports to standard format (Markdown, PDF, ODT, ...); folder structure exports as zip preserving parent_epr_cid hierarchy as folder hierarchy

## Code anchors

| Surface | Path |
|---|---|
| Document store services | `app/elohim-app/src/app/lamad/` + `elohim` pillar |
| Content views | `elohim/elohim-storage/src/views.rs` (`ContentView`) |
| Automerge integration | see automerge-sync skill at `.claude/skills/automerge-sync/` |
| Lamad pillar manifest | `elohim/sdk/domains/lamad/manifest.json` |

*Full draft pending.*
