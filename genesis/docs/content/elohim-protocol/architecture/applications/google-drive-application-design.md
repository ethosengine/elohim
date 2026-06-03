---
title: Google Drive — substrate-native file store + collaboration
id: google-drive-application-design
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

Grandma opens the app on her laptop. She sees her folders, her documents, and "shared with me" from her book club. She taps a shared document — it opens. Her friend across town is already in it; grandma watches the cursor move and text appear in real time. She drags a photo into a folder; it uploads silently. She searches "minutes from March" — results appear instantly, drawn from her own device. Nothing goes to Google. No one indexes her writing. When her book club ends, she shares the whole folder with the new chair and removes herself. Her bytes stay on her node; the new chair's bytes stay on hers.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Document | EPR (`content_type: "document"`) | body ≤64 KB inline; body >64 KB in `media_cid → iroh-blob` |
| Folder | EPR (`content_type: "folder"`) | root of a `parent_epr_cid` hierarchy; D.1 subordination |
| Subfolder | EPR (`content_type: "folder"`, `parent_epr_cid = parent_folder_cid`) | D.1 link adds edge to `epr_resource_edges` adjacency table |
| Edit session | Event (`action: "edit"`) | graduates from keystroke Observations at session close; carries Automerge delta blob in `payload_json` |
| Real-time CRDT sync | Automerge delta on libp2p sync plane (`/elohim/sync/2.0.0` ALPN) | only between actively-open editors; not gossiped; no DHT writes during typing |
| Version snapshot | Resource (`resource_classified_as: "document-state"`) | derived from Event history; D.12 checkpoint Commitment compacts after 90 days or 50k edit-Events |
| Sharing grant | Event (`action: "grant-reach"`) | D.9 reach-mutation Event; widens document's reach to specific recipient agent-key |
| Share revocation | Event (`action: "revoke-reach"`) | D.9 reach-mutation; recipient's projection stops updating; cached bytes age out |
| Comment | FeedbackSignal (`signal_kind: "comment"`, `signal_class: "care"`) | DHT-notarized; threaded via `parent_signal_cid`; D.18 care/compute isolation |
| Suggestion | FeedbackSignal (`signal_kind: "suggestion"`, `signal_class: "care"`) | accept → Event(`action: "edit"`); reject → Event(`action: "close"`) resolves it |
| Activity log | Derived SQL view over edit / grant-reach / revoke-reach Events under document EPR | `SELECT * FROM events WHERE parent_epr_cid = $doc_cid ORDER BY created_at DESC` |
| Search results | Local SQL FTS5 over document body + metadata + OCR Attestation text | no central index; each user searches only their reach-visible corpus |
| Trash | Document EPR with `lifecycle_state: "shelved"` | recoverable; D.3 submerge operation |
| Permanent delete | Event(`action: "dispose"`) → `lifecycle_state: "closed"` | D.7 dissolution; future Events targeting closed doc fail zome validation |
| Storage-tier collab | Collective Commons EPR (D.20) | optional pay-per-capacity tiers via Layered Commons fee-split; no class-coded pricing |

Eight primitives, ~9 discriminator values, no special-casing for file storage.

## How one collaborative edit flows

Two members of the book club — Maya in Oakland, James in Portland — both have the document open.

```
1. Maya types a sentence.
   Automerge: transaction opens locally; change appended to local CRDT doc
   in SQLite (`steward/node/src/sync/merge.rs` → AutoCommit + put).
   No network traffic yet.

2. Automerge produces a change blob (~200 B per keystroke batch).
   SyncCoordinator (`steward/node/src/sync/coordinator.rs`) detects
   local position advance.

3. libp2p direct-message to James's node on /elohim/sync/2.0.0 ALPN:
   SyncMessage::Announce { event: SyncEvent { kind: Local, ... } }
   Wire: 4-byte BE length prefix + MessagePack-framed change blob.
   Latency: <50 ms LAN / <200 ms cross-country.

4. James's SyncCoordinator receives the Announce; calls
   SyncMessage::DocRequest { doc_id, heads: [our_heads] }
   Maya's node responds SyncMessage::DocResponse { changes: [blob] }.

5. James's merge.rs calls AutoCommit::apply_changes(blob);
   Automerge merges deterministically (same-field conflict resolves by
   actor-ID ordering; different-field changes merge cleanly).
   James sees Maya's text appear; <300 ms end-to-end.

6. Session ends (browser tab closes / 5-min idle):
   graduation-evaluator (`elohim/elohim-storage/src/services/`) fires:
     Event {
       action: "edit",
       provider: maya_agent_key,
       receiver: document_epr_cid,
       parent_epr_cid: document_epr_cid,
       payload_json: { automerge_delta: "<base64 change blob>",
                       char_count_delta: 412 }
     }
   DHT write: ~3 KB. Neighborhood validates. Gossiped to both agents'
   reach scope (the shared document's current collaborators).

7. ReconcileController (`elohim/elohim-storage/src/reconcile/controller.rs`)
   receives post-commit signal → upserts `epr_event_edges` row
   (parent_epr_cid = document_cid) → updates SQL projection.

8. Both dashboards auto-refresh: "recent edits" view updates in <100 ms.
```

The document's full edit history is reconstructable at any point: apply all edit-Events in order from creation. After 90 days or 50,000 edit-Events, the `checkpoint-elohim` authors a D.12 checkpoint Commitment capturing the Automerge doc heads as a snapshot. Future balance queries start from the snapshot and sum only subsequent Events — orders of magnitude faster than replaying a 10-year typing history.

## Storage footprint per household

| Item | Count | Size | Total |
|---|---|---|---|
| Document EPRs (metadata only) | ~10k | 5 KB | 50 MB |
| Folder EPRs | ~500 | 2 KB | 1 MB |
| Edit-Events (10 yr × 5 sessions/day × avg 3 docs) | ~55k | 3 KB avg | 165 MB |
| Document bodies — working set (iroh-blob, hot) | ~1k docs | 100 KB avg | 100 MB |
| Document bodies — cold corpus (iroh-blob, quilt-tiered) | ~9k docs | 100 KB avg | ~900 MB quilt |
| FeedbackSignals (comments / suggestions) | ~5k | 500 B | 2.5 MB |
| OCR Attestations (for scanned PDFs, images) | ~2k | 2 KB | 4 MB |
| D.12 checkpoint Commitments | ~200 | 1 KB | 200 KB |
| SQL adjacency (epr_event_edges + epr_resource_edges) | ~65k rows | 200 B | 13 MB |
| **Total local SQL projection** | | | **~336 MB** |
| Cold archive in quilt (K-of-N erasure) | — | — | ~900 MB |

**Fits on a laptop with room to spare.** Hot documents load from SQL; cold documents fetch from quilt on demand in <2 s.

## Network bandwidth profile

- Edit session: Automerge deltas between N active editors, direct-message only. Typical session: ~50 KB of change blobs between 2 editors. No global gossip during typing.
- Session-close DHT write: ~3 KB per edit-Event. At 5 sessions/day: ~15 KB/day DHT writes.
- Inbound reach-scoped gossip: ~5 KB/day if collaborating on <20 documents.
- Monthly DHT bandwidth: **~500 KB/month** for a typical Drive participant — well inside household budgets.
- iroh-blob fetch (cold document): one quilt pull, ~100 KB per document, amortized across sessions.
- **Per household: <10 MB/month** for full Drive participation. The dominant cost is blob storage, not bandwidth.

## DHT entry impact

- 10k edit-Events per household × 100M households = 10¹² Events if everything went global.
- But: document EPRs carry `reach: "agent-private"` by default; edit-Events inherit parent EPR reach.
- A collaborator gaining reach via `grant-reach` Event is the only path to reach expansion; no accidental virality.
- Per-peer DHT entry visibility: only EPRs + Events in reach scope. Typical household peer holds ~70k entries — well inside validator budget.
- D.12 checkpoint Commitments replace thousands of replay-Events with a single snapshot. At 90-day cadence, a 10-year document emits ~40 checkpoints rather than 55k individually-queryable Events in hot storage.
- FeedbackSignal budget: 5k comments/year at 500 B = 2.5 MB DHT footprint, bounded by the document's `agent-private` reach scope.

## Why collaborative editing doesn't melt the network

**During typing — zero DHT writes.** Automerge change blobs flow only via libp2p direct-message between the editors who are open at the same time. The CRDT sync plane (`/elohim/sync/2.0.0` ALPN in `elohim/elohim-storage/src/p2p_iroh/sync.rs`) maintains per-agent stream positions and sends only the delta since each peer's last-known position. A 100-character paragraph edit between 2 editors: ~400 B on the wire, never touches DHT.

**Session close — one small DHT write.** The graduation-evaluator batches all in-session changes into a single edit-Event. The Automerge change blob for a typical session is 2–5 KB. That single DHT write is what gets gossiped; neighborhood validators see one entry, not 5,000 keystrokes.

**Long history — D.12 checkpoint compacts it.** Without checkpoints, a 5-year document needs to replay 100k edit-Events to reconstruct current state. With D.12's `checkpoint` Commitment:

```sql
-- Read-path with checkpoint optimization
WITH latest_checkpoint AS (
  SELECT payload_json, period_end
  FROM commitments
  WHERE action = 'checkpoint' AND subject_cid = :doc_cid
  ORDER BY period_end DESC LIMIT 1
)
SELECT
  lc.payload_json AS snapshot,   -- Automerge doc heads at checkpoint
  e.payload_json  AS delta
FROM latest_checkpoint lc
LEFT JOIN events e ON e.parent_epr_cid = :doc_cid
  AND e.created_at > lc.period_end
ORDER BY e.created_at;
```

Snapshot load: read one Commitment row + Automerge doc binary from iroh-blob (≤1 s). Delta replay: only Events since last checkpoint (typically <500 for a 90-day window). Total document reconstruction: **<2 s** for any document regardless of total edit history depth.

**Working set — hot/cold split.** Documents opened in the last 30 days stay in the SQL hot tier. Cold documents (quilt-tiered) fetch their iroh-blob on first open via `IrohBlobStore`. The quilt layer uses K-of-N Reed-Solomon: any 4 of 7 shards reconstruct a document chunk, so cold documents survive peer churn. Cold fetch latency: <2 s for a 100 KB document body.

## Why search renders fast

- Search is local SQL FTS5 over the user's own corpus. No central index. No Google-shape cross-user search.
- FTS5 index covers: document body (for inline docs), document metadata (title, tags, collaborator names), OCR Attestation text (scanned PDFs), comment FeedbackSignal text.
- Query shape:

```sql
SELECT d.id, d.title, d.parent_folder_cid, snippet(docs_fts, 0, '<b>', '</b>', '...', 10)
FROM docs_fts
JOIN epr_projection d ON docs_fts.doc_id = d.id
WHERE docs_fts MATCH :query
  AND d.lifecycle_state != 'closed'
ORDER BY rank;
-- Typical: <50 ms on 10k documents indexed.
```

- Cross-corpus search (shared drive, collective workspace): federated query via the collective-hub elohim-node → parallel libp2p RPC to each member's local FTS → aggregated ranked results. No data replication. Per-member privacy preserved by reach.

## Dissolution in practice

- User deletes a document → `lifecycle_state: "shelved"` (trash). FTS index excludes it. Query filter: `AND d.lifecycle_state != 'shelved'`.
- User empties trash → `Event(action: "dispose")` → `lifecycle_state: "closed"`. D.7 dissolution: future Events targeting the closed document fail zome validation at write time. iroh-blob bytes become eligible for quilt de-pinning after custody_ttl elapses.
- Folder deleted with children → D.1 subordination: child documents inherit `lifecycle_state: "shelved"` cascade. Each child needs its own `dispose` Event to close permanently; cascade-dispose is a coordinator function that fans out one `dispose` Event per child.
- User departs a collaboration → `Event(action: "revoke-reach")` narrows document reach; their local projection stops receiving updates. Their cached bytes are theirs (exit is structural); the document continues for remaining collaborators.
- `grant-reach` → `revoke-reach` lifecycle per D.9: the reach-mutation Events are the audit trail. Any collaborator can query "who had reach to this document, when?" via `SELECT * FROM events WHERE action IN ('grant-reach','revoke-reach') AND subject_cid = :doc_cid`.

## Where agentic intelligence carries the load

**Without writing-elohim:** version history is 55,000 edit-Events with no semantic anchors. "What changed in the Q3 draft between Monday and Friday?" requires replaying every Event.

**With writing-elohim** (`domain_specialization: "writing-assist"`): session-close Events receive semantic summaries in `payload_json`. The writing-elohim produces: "Added 2 paragraphs on Q3 results; restructured conclusion." The summary lands in the checkpoint Commitment's metadata. The "activity log" view shows human-readable history, not raw delta blobs.

**Without vision-elohim:** a scanned PDF in Drive is a blob. It has no searchable text.

**With vision-elohim** (`domain_specialization: "ocr"`): on document creation, vision-elohim authors an `Attestation(content_type: "attestation:ocr")` carrying extracted text. The attestation's `evidence_cid` points to the source document's iroh-blob. The attestation body indexes into FTS5. "Search minutes from March" returns the scanned PDF because its OCR attestation matched. This is D.6 elohim-authoring pattern: the elohim is the authorized attestor; the Attestation is DHT-notarized under its stewardship-commitment.

**Without stewardship-elohim during import:** a Google Drive Takeout export is 50 GB of ZIP files. A human can't manually create 10,000 EPRs with correct `parent_epr_cid` hierarchies.

**With stewardship-elohim at import:** `bridges/google-drive/` batch-graduates EPRs from Takeout manifest. The stewardship-elohim signs each Observation (`observation_kind: "drive:import-document"`); graduation evaluator crystallizes each into a `document` EPR with correct `parent_epr_cid` from the Takeout folder hierarchy. Import of 10,000 files runs overnight; user wakes to a fully-indexed Drive corpus with version history preserved as edit-Events.

## What the collective / multi-party view shows

A law firm's case workspace: 5 lawyers sharing 200 documents across 4 matter folders.

The Collective EPR (`content_type: "workspace"`) references each lawyer's agent key as a Membership. Reach on each document EPR is set to `reach: "collective"` via `grant-reach` Events targeting the workspace membership. Each lawyer's elohim-node maintains their own local SQL projection of the shared documents.

Render path for the shared workspace dashboard:
- Collective-hub elohim-node issues parallel libp2p RPC to each of 5 member nodes.
- Each node runs: `SELECT id, title, updated_at FROM epr_projection WHERE parent_epr_cid IN (matter_folder_cids) ORDER BY updated_at DESC LIMIT 20`
- Hub aggregates and ranks by `updated_at`. Returns in <300 ms.
- No document bodies replicated to the hub. The hub holds metric-projections only.

If a lawyer departs the firm → `revoke-reach` Event → their local projection stops syncing. Their cached bytes remain on their node (exit is structural). The workspace continues for remaining members.

Layered Commons (D.20) for any capacity-tier subscription: a workspace EPR may declare a `collective_commons_cid` where storage-tier fees flow. The Collective Commons EPR's Membership governs allocation decisions (e.g., expanding cold-storage budget). Fee split: 0.5% of each file-storage Commitment fee flows to Global Commons; the remainder to the workspace's Collective Commons. No "premium tier" gatekeeping by a platform; the fee split is manifest-declared and substrate-enforced.

## Bridges (legacy interop / cash-out)

- **bridges/google-drive/** (planned, import-only) — Google Drive Takeout JSON manifest + file blobs → batch Observations under stewardship-elohim signature → graduated document EPRs with `parent_epr_cid` hierarchy from Takeout folder structure. Version history from Takeout's revision metadata graduates as edit-Events. Format: `observation_kind: "drive:import-document"`. The bridge crate is a substrate participant (Collective EPR for Google LLC's bridge), not a passthrough adapter.
- **bridges/dropbox/** (planned, import-only) — Dropbox export (`.zip` with `manifest.json`) → same pattern. Dropbox revision history imports as edit-Events with `triggered_by` chaining. OneDrive and Box follow the same bridge shape.
- **Cash-out:** every document EPR exports to standard format on request — Markdown for plain text, PDF for richly-formatted, ODT for office-suite compatible. Folder hierarchy exports as a ZIP with the `parent_epr_cid` tree mirrored as a folder tree. The export is a local operation; no server round-trip. The user can re-import their export to any substrate node or to any legacy platform; the substrate imposes no exit penalty.

## Code anchors

| Surface | Path |
|---|---|
| Document store services | `app/elohim-app/src/app/lamad/` + `elohim` pillar |
| Content views (Rust→TS boundary) | `elohim/elohim-storage/src/views.rs` (`ContentView`, `EconomicEventView`) |
| View schemas | `elohim/sdk/schemas/v1/views/content-view.schema.json` |
| Lamad pillar manifest | `elohim/sdk/domains/lamad/manifest.json` (content_types: document, folder; action verbs: edit, grant-reach, revoke-reach, dispose, checkpoint) |
| Automerge CRDT core (Rust) | `elohim/elohim-storage/src/sync/mod.rs`, `steward/node/src/sync/merge.rs` |
| Automerge sync protocol (libp2p) | `elohim/elohim-storage/src/p2p/sync_protocol.rs` |
| iroh sync ALPN backend | `elohim/elohim-storage/src/p2p_iroh/sync.rs`, `sync_backend.rs` |
| iroh blob store | `elohim/elohim-storage/src/p2p_iroh/blob_store.rs` |
| ReconcileController (post-commit projection) | `elohim/elohim-storage/src/reconcile/controller.rs` |
| Checkpoint service (planned) | `elohim/elohim-storage/src/services/checkpoint_service.rs` |
| Google Drive bridge (planned) | `bridges/google-drive/` |
| Dropbox bridge (planned) | `bridges/dropbox/` |
| Doorway HTTP routes | `doorway/doorway-service/src/handlers/` |
| Steward SyncCoordinator | `steward/node/src/sync/coordinator.rs` |

## What this proves about the substrate

A skeptical systems architect should walk away from this archetype able to say:

- **Real-time collaboration without central coordination is solved by the existing Automerge CRDT infrastructure** (`/elohim/sync/2.0.0` ALPN, `steward/node/src/sync/`). No collaborative-editing server is needed; no operational transform server; no conflict-resolution service. The CRDT handles concurrent edits deterministically. The only cost is direct-message bandwidth between active editors — which is bounded by the number of humans who can simultaneously edit a single document (empirically <10).
- **Folder hierarchies emerge from `parent_epr_cid` + D.1 `EprToResource` link adjacency**, not from a special-purpose folder-table schema. The `epr_resource_edges` table is the folder tree; `SELECT * WHERE parent_epr_cid = $folder_cid` is the folder listing query. Drive-shape navigation is local SQL, not a tree-traversal API call.
- **The D.12 checkpoint primitive keeps edit-history queryable regardless of document age.** A 10-year document with 100k edit-Events renders current state in <2 s via snapshot + delta replay. Without checkpoints, the substrate would not be able to make this performance claim; with them, it can.
- **Local FTS5 search over the user's own corpus replaces Google's central search index.** Cross-corpus search is federated query (parallel libp2p RPC to member nodes, aggregated at hub) — no data replication, privacy preserved by reach scope. The substrate commits that "search everything" does not require a centralized index.
- **Data sovereignty is structural, not policy.** `revoke-reach` terminates a collaborator's live sync immediately because the hub never held the source data — only metric-projections. Exit is a substrate primitive, not a deletion request to a platform.
