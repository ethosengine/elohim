# Cache Stream Warm-Up Design

**Date:** 2026-03-25
**Status:** Approved
**Replaces:** HTTP pull warm-up (`doorway-service/src/projection/warm.rs`)

## Problem

The projection cache warm-up is a hack: doorway fetches `/db/content?limit=10000` from each storage URL on startup. This doesn't scale, doesn't respect cache eligibility (pulls ALL content including private/local-reach), and pulls blindly with an arbitrary limit cap.

## Solution

Replace the HTTP pull with an SSE stream from elohim-storage. Storage streams all cacheable content (filtered by reach) as Server-Sent Events. Doorway consumes the stream and projects each item into MongoDB via `ProjectionStore::set()`.

## Data Flow

```
┌─────────────────┐    GET /api/v1/cache/stream     ┌─────────────────┐
│  elohim-storage  │◄───────────────────────────────│    doorway       │
│   (SQLite)       │                                 │                  │
│                  │── event: cache.content ────────▶│ ProjectionStore  │
│  WHERE reach     │── event: cache.content ────────▶│   .set(doc)      │
│  IN ('commons')  │── event: cache.path ──────────▶│                  │
│                  │── event: cache.human ──────────▶│  MongoDB         │
│                  │── event: cache.relationship ──▶│                  │
│                  │── event: cache.done ──────────▶│                  │
│                  │── : heartbeat (30s) ──────────▶│                  │
└─────────────────┘                                 └─────────────────┘
```

## Design Decisions

### Why SSE (not WebSocket, not NDJSON)
- Storage already has SSE infrastructure (`SseBody`, heartbeat pattern)
- SSE has built-in event types — doorway knows the doc_type from `event:` field
- HTTP/1.1 compatible, works through proxies, trivial to debug with `curl`
- Backpressure via TCP flow control
- Warm-up is unidirectional — WebSocket bidirectionality is unused complexity

### Why storage (not a zome function)
- Storage has the data indexed in SQLite with proper indexes — fast queries
- Source chain scan is linear and expensive, competes with real-time conductor operations
- Reach field is already in the data (DNA validation set it)
- Storage is the fast index layer — reads belong here

### No incremental (`?since=` parameter)
- The signal subscriber handles all changes after connection
- Warm-up only backfills content that exists at stream time
- Re-streaming is idempotent (MongoDB upserts) and fast at current scale
- Avoids timestamp tracking complexity and clock skew edge cases

### Re-stream on subscriber reconnect
- Signals emitted while subscriber was disconnected are lost
- Re-streaming after reconnect covers the gap
- Idempotent upserts mean no harm if MongoDB already has the content
- Gossipped entries from other agents (not covered by signals) are a deeper gap solved by the libp2p sync protocol, not warm-up

## Storage Endpoint

`GET /api/v1/cache/stream`

No parameters. Queries each cacheable table in sequence:

| Table | Filter | SSE event type |
|-------|--------|----------------|
| `content` | `reach = 'commons'` | `cache.content` |
| `paths` | all (paths are public per cache rules) | `cache.path` |
| `humans` | `profile_reach = 'public'` | `cache.human` |
| `relationships` | `reach = 'commons'` | `cache.relationship` |

Each event's `data:` field is the View-layer JSON (camelCase, parsed metadata — same as what `/db/content/{id}` returns).

Stream ends with:
```
event: cache.done
data: {"content":342,"paths":12,"humans":5,"relationships":89}
```

Implementation: new `cache_stream.rs` file. Spawns a task that reads from SQLite in batches (500 rows per query via LIMIT/OFFSET to avoid holding DB connections), converts to View types, pushes SSE frames into a `tokio::mpsc` channel consumed by `StreamBody`. Heartbeats every 30s (merged stream, same pattern as existing `sse.rs`).

## Doorway Warm-Up Client

New `warm_stream.rs` replaces the HTTP pull logic.

```
warm_stream::stream_from_peer(store, storage_url)
  → GET {storage_url}/api/v1/cache/stream
  → reqwest streaming response
  → parse SSE lines (event type + data)
  → for each event:
      ProjectedDocument::new(doc_type, id, "cache-stream", "cache-stream", data)
      store.set(doc).await
  → log final counts from cache.done event
```

### Trigger Points

1. **Startup** — for each peer storage URL, stream cacheable content
2. **Subscriber reconnect** — after `AppWebsocket::connect()` succeeds, re-stream from that peer's storage

### Reconnect Wiring

Add `storage_url: Option<String>` and `projection_store: Option<Arc<ProjectionStore>>` to `SubscriberConfig`. After successful `AppWebsocket::connect()` in the subscriber's `run()` method, if both are set, spawn the cache stream as a background task. Non-blocking — subscriber continues receiving signals while backfill runs.

## Changes Summary

| Component | File | Change |
|-----------|------|--------|
| **Storage** | `src/cache_stream.rs` (new) | SSE stream of cacheable content |
| **Storage** | `src/http.rs` | Route `GET /api/v1/cache/stream` |
| **Storage** | `src/http.rs` | Add to `build_manifest()` |
| **Doorway** | `src/projection/warm_stream.rs` (new) | SSE client consuming stream |
| **Doorway** | `src/projection/subscriber.rs` | Trigger warm stream after reconnect |
| **Doorway** | `src/projection/mod.rs` | Add `warm_stream` module |
| **Doorway** | `src/main.rs` | Replace `spawn_warm_task` with stream |
| **Doorway** | `src/projection/warm.rs` | Deprecate then remove |

No DNA changes. No Angular changes. No new crates.
