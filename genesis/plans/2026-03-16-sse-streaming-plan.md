# SSE Streaming — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Server-Sent Events to elohim-storage so browsers receive real-time typed events from the existing EventBus via `GET /api/v1/events`.

**Architecture:** Raw hyper 1.0 SSE using `StreamBody` + `tokio::broadcast` (reusing existing `EventBus`). Return type changes from `Response<Full<Bytes>>` to `Response<Either<Full<Bytes>, SseBody>>`. Angular `EventStreamService` wraps browser `EventSource` API.

**Tech Stack:** Rust (hyper 1.0, http-body-util, tokio-stream, tokio::broadcast), Angular 19 (RxJS, Vitest).

---

## Task 1: Add tokio-stream dependency

### Files
- Modify: `elohim/elohim-storage/Cargo.toml`

### Step 1: Add dependency

Add `tokio-stream` to the `[dependencies]` section of `Cargo.toml`:

```toml
tokio-stream = "0.1"
```

Place it alphabetically near the other tokio entries.

### Step 2: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -5
```

### Step 3: Commit

```bash
git add elohim/elohim-storage/Cargo.toml
git commit -m "chore(storage): add tokio-stream dependency for SSE streaming"
```

---

## Task 2: SSE Module — formatting + stream handler

### Files
- Create: `elohim/elohim-storage/src/sse.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` (or `main.rs` — add `pub mod sse;`)

### Step 1: Read these reference files first

- `elohim/elohim-storage/src/services/events.rs` — `EventBus`, `StorageEvent` enum (all variants)
- `elohim/elohim-storage/src/progress_ws.rs` — broadcast pattern with heartbeat and lagged handling
- `elohim/elohim-storage/src/http.rs` — understand the `Full<Bytes>` response pattern

### Step 2: Create `src/sse.rs`

```rust
//! Server-Sent Events (SSE) streaming handler
//!
//! Provides real-time event streaming to browser clients via the standard
//! SSE protocol (text/event-stream). Events come from the existing EventBus
//! (tokio::broadcast) which carries StorageEvent variants from all service
//! layer mutations.
//!
//! ## Usage
//!
//! ```
//! GET /api/v1/events
//! Accept: text/event-stream
//! ```
//!
//! ## Event Format
//!
//! ```text
//! event: content.created
//! id: 42
//! data: {"id":"abc","title":"My Content"}
//!
//! ```

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::StreamBody;
use hyper::body::Frame;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, trace};

use crate::services::events::{EventBus, StorageEvent};

/// Global event ID counter (monotonically increasing across all SSE connections)
static EVENT_ID: AtomicU64 = AtomicU64::new(1);

/// The SSE body type — a stream of `Frame<Bytes>` that never errors
pub type SseBody = StreamBody<Pin<Box<dyn futures_util::Stream<Item = Result<Frame<Bytes>, Infallible>> + Send>>>;

/// Map a StorageEvent to its SSE event type string
fn event_type(event: &StorageEvent) -> &'static str {
    match event {
        StorageEvent::ContentCreated { .. } => "content.created",
        StorageEvent::ContentUpdated { .. } => "content.updated",
        StorageEvent::ContentDeleted { .. } => "content.deleted",
        StorageEvent::ContentBulkCreated { .. } => "content.bulk-created",
        StorageEvent::PathCreated { .. } => "path.created",
        StorageEvent::PathUpdated { .. } => "path.updated",
        StorageEvent::PathDeleted { .. } => "path.deleted",
        StorageEvent::PathBulkCreated { .. } => "path.bulk-created",
        StorageEvent::RelationshipCreated { .. } => "relationship.created",
        StorageEvent::RelationshipDeleted { .. } => "relationship.deleted",
        StorageEvent::RelationshipBulkCreated { .. } => "relationship.bulk-created",
        StorageEvent::KnowledgeMapCreated { .. } => "knowledge-map.created",
        StorageEvent::KnowledgeMapUpdated { .. } => "knowledge-map.updated",
        StorageEvent::KnowledgeMapDeleted { .. } => "knowledge-map.deleted",
        StorageEvent::PathExtensionCreated { .. } => "path-extension.created",
        StorageEvent::PathExtensionUpdated { .. } => "path-extension.updated",
        StorageEvent::PathExtensionDeleted { .. } => "path-extension.deleted",
    }
}

/// Serialize a StorageEvent to its JSON data payload
fn event_data(event: &StorageEvent) -> String {
    match event {
        StorageEvent::ContentCreated { id, title, content_type } => {
            serde_json::json!({ "id": id, "title": title, "contentType": content_type }).to_string()
        }
        StorageEvent::ContentUpdated { id } => {
            serde_json::json!({ "id": id }).to_string()
        }
        StorageEvent::ContentDeleted { id } => {
            serde_json::json!({ "id": id }).to_string()
        }
        StorageEvent::ContentBulkCreated { count, ids } => {
            serde_json::json!({ "count": count, "ids": ids }).to_string()
        }
        StorageEvent::PathCreated { id, title } => {
            serde_json::json!({ "id": id, "title": title }).to_string()
        }
        StorageEvent::PathUpdated { id } => {
            serde_json::json!({ "id": id }).to_string()
        }
        StorageEvent::PathDeleted { id } => {
            serde_json::json!({ "id": id }).to_string()
        }
        StorageEvent::PathBulkCreated { count, ids } => {
            serde_json::json!({ "count": count, "ids": ids }).to_string()
        }
        StorageEvent::RelationshipCreated { id, source_id, target_id, relationship_type } => {
            serde_json::json!({
                "id": id,
                "sourceId": source_id,
                "targetId": target_id,
                "relationshipType": relationship_type,
            }).to_string()
        }
        StorageEvent::RelationshipDeleted { id } => {
            serde_json::json!({ "id": id }).to_string()
        }
        StorageEvent::RelationshipBulkCreated { count } => {
            serde_json::json!({ "count": count }).to_string()
        }
        StorageEvent::KnowledgeMapCreated { id, map_type, owner_id } => {
            serde_json::json!({ "id": id, "mapType": map_type, "ownerId": owner_id }).to_string()
        }
        StorageEvent::KnowledgeMapUpdated { id } => {
            serde_json::json!({ "id": id }).to_string()
        }
        StorageEvent::KnowledgeMapDeleted { id } => {
            serde_json::json!({ "id": id }).to_string()
        }
        StorageEvent::PathExtensionCreated { id, base_path_id, extended_by } => {
            serde_json::json!({ "id": id, "basePathId": base_path_id, "extendedBy": extended_by }).to_string()
        }
        StorageEvent::PathExtensionUpdated { id } => {
            serde_json::json!({ "id": id }).to_string()
        }
        StorageEvent::PathExtensionDeleted { id } => {
            serde_json::json!({ "id": id }).to_string()
        }
    }
}

/// Format a single SSE event as a string
///
/// Output format:
/// ```text
/// event: content.created
/// id: 42
/// data: {"id":"abc","title":"My Content"}
///
/// ```
pub fn format_sse_event(event: &StorageEvent) -> String {
    let id = EVENT_ID.fetch_add(1, Ordering::Relaxed);
    let etype = event_type(event);
    let data = event_data(event);
    format!("event: {}\nid: {}\ndata: {}\n\n", etype, id, data)
}

/// Format a heartbeat comment (keeps connection alive through proxies)
pub fn format_heartbeat() -> String {
    ": heartbeat\n\n".to_string()
}

/// Create an SSE streaming response from the EventBus
///
/// Returns a `Response` with `Content-Type: text/event-stream` and a streaming
/// body that emits events as they arrive from the broadcast channel, plus a
/// heartbeat comment every 30 seconds.
pub fn create_sse_stream(event_bus: &Arc<EventBus>) -> hyper::Response<SseBody> {
    let rx = event_bus.subscribe();
    let broadcast_stream = BroadcastStream::new(rx);

    debug!(
        subscribers = event_bus.subscriber_count(),
        "New SSE client connected"
    );

    // Merge event stream with heartbeat interval
    let heartbeat = tokio_stream::wrappers::IntervalStream::new(
        tokio::time::interval(std::time::Duration::from_secs(30)),
    );

    // Event frames from broadcast
    let events = broadcast_stream.filter_map(|result| {
        match result {
            Ok(event) => {
                trace!(event_type = event_type(&event), "SSE: emitting event");
                let text = format_sse_event(&event);
                Some(Ok(Frame::data(Bytes::from(text))))
            }
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                debug!(skipped = n, "SSE client lagged, skipped events");
                // Send a comment indicating lag
                let text = format!(": lagged, skipped {} events\n\n", n);
                Some(Ok(Frame::data(Bytes::from(text))))
            }
        }
    });

    // Heartbeat frames
    let heartbeats = heartbeat.map(|_| {
        Ok(Frame::data(Bytes::from(format_heartbeat())))
    });

    // Merge both streams — events + heartbeats
    let merged = tokio_stream::StreamExt::merge(events, heartbeats);
    let pinned: Pin<Box<dyn futures_util::Stream<Item = Result<Frame<Bytes>, Infallible>> + Send>> =
        Box::pin(merged);

    hyper::Response::builder()
        .status(200)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .body(StreamBody::new(pinned))
        .expect("SSE response builder should not fail")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_sse_event_content_created() {
        let event = StorageEvent::ContentCreated {
            id: "test-1".into(),
            title: "My Content".into(),
            content_type: Some("journal".into()),
        };
        let formatted = format_sse_event(&event);
        assert!(formatted.starts_with("event: content.created\n"));
        assert!(formatted.contains("id: "));
        assert!(formatted.contains(r#""id":"test-1""#));
        assert!(formatted.contains(r#""title":"My Content""#));
        assert!(formatted.ends_with("\n\n"));
    }

    #[test]
    fn test_format_sse_event_relationship_created() {
        let event = StorageEvent::RelationshipCreated {
            id: "rel-1".into(),
            source_id: "a".into(),
            target_id: "b".into(),
            relationship_type: "requires".into(),
        };
        let formatted = format_sse_event(&event);
        assert!(formatted.starts_with("event: relationship.created\n"));
        assert!(formatted.contains(r#""sourceId":"a""#));
        assert!(formatted.contains(r#""targetId":"b""#));
    }

    #[test]
    fn test_format_heartbeat() {
        let hb = format_heartbeat();
        assert_eq!(hb, ": heartbeat\n\n");
    }

    #[test]
    fn test_event_type_mapping() {
        assert_eq!(event_type(&StorageEvent::ContentCreated { id: "".into(), title: "".into(), content_type: None }), "content.created");
        assert_eq!(event_type(&StorageEvent::PathDeleted { id: "".into() }), "path.deleted");
        assert_eq!(event_type(&StorageEvent::KnowledgeMapCreated { id: "".into(), map_type: "".into(), owner_id: "".into() }), "knowledge-map.created");
    }

    #[test]
    fn test_event_ids_are_monotonic() {
        let e1 = StorageEvent::ContentUpdated { id: "a".into() };
        let e2 = StorageEvent::ContentUpdated { id: "b".into() };
        let f1 = format_sse_event(&e1);
        let f2 = format_sse_event(&e2);
        // Extract IDs
        let id1: u64 = f1.lines().find(|l| l.starts_with("id: ")).unwrap().strip_prefix("id: ").unwrap().parse().unwrap();
        let id2: u64 = f2.lines().find(|l| l.starts_with("id: ")).unwrap().strip_prefix("id: ").unwrap().parse().unwrap();
        assert!(id2 > id1);
    }
}
```

### Step 3: Register the module

Find the file that declares modules (`src/lib.rs` or `src/main.rs`). Add `pub mod sse;` alongside the other module declarations.

### Step 4: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -10
```

### Step 5: Run the tests

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test sse 2>&1 | tail -15
```

### Step 6: Commit

```bash
git add elohim/elohim-storage/src/sse.rs elohim/elohim-storage/src/main.rs
git commit -m "feat(storage): add SSE module — event formatting and stream handler

format_sse_event maps StorageEvent variants to typed SSE text frames.
create_sse_stream merges broadcast events with 30s heartbeat.
5 unit tests covering formatting, type mapping, and monotonic IDs."
```

---

## Task 3: Change handle_request return type to Either

### Files
- Modify: `elohim/elohim-storage/src/http.rs`

### Step 1: Read these carefully

- `elohim/elohim-storage/src/http.rs` — the full `handle_request` function and everywhere `Response<Full<Bytes>>` appears
- `elohim/elohim-storage/src/sse.rs` — the `SseBody` type alias

### Step 2: Change the return type

This is the most delicate task. The goal is to change `handle_request` from returning `Response<Full<Bytes>>` to `Response<Either<Full<Bytes>, SseBody>>` so it can return either normal responses or SSE streams.

**In `http.rs`:**

1. Add imports at the top:
```rust
use http_body_util::Either;
use crate::sse::SseBody;
```

2. Define a type alias for the combined body:
```rust
/// Response body type — Either a normal full body or an SSE stream
pub type ApiBody = Either<Full<Bytes>, SseBody>;
```

3. Change `handle_request` signature:
```rust
async fn handle_request(
    &self,
    req: Request<Incoming>,
) -> Result<Response<ApiBody>, hyper::Error> {
```

4. **Every existing response** that returns `Response<Full<Bytes>>` must be wrapped. The cleanest approach is a helper function:
```rust
fn wrap_response(resp: Response<Full<Bytes>>) -> Response<ApiBody> {
    resp.map(Either::Left)
}
```

Then at the bottom of `handle_request`, where the result is returned, wrap it:
```rust
let result = match (method, path.as_str()) {
    // ... all existing routes return Result<Response<Full<Bytes>>, ...>
};

// Wrap the Full<Bytes> response in Either::Left
result.map(|r| r.map(Either::Left))
```

5. The WebSocket upgrade handler also returns `Response<Full<Bytes>>` (the upgrade handshake response). This gets wrapped the same way via the `.map(Either::Left)` at the bottom.

6. Also update `add_cors_headers` and the error handler at the end of `handle_request` to work with the new type. The pattern is:
```rust
// After the match, before returning:
let response = result.unwrap_or_else(|e| {
    // error response
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Full::new(Bytes::from(format!("Error: {}", e))))
        .unwrap()
});
Ok(add_cors_headers(response).map(Either::Left))
```

**IMPORTANT:** Read the FULL `handle_request` function before making changes. There may be multiple places where `Response<Full<Bytes>>` is constructed. All must be wrapped. The `.map(Either::Left)` approach at the final return point is cleanest — it wraps everything at once.

### Step 3: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -10
```

This may require fixing type mismatches. Work through each compiler error — they will all be about the body type changing.

### Step 4: Run existing tests to verify no regressions

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -15
```

### Step 5: Commit

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "refactor(storage): change handle_request return type to Either<Full, SseBody>

Wraps all existing Full<Bytes> responses in Either::Left. Enables
Either::Right for SSE streaming responses without changing any
existing handler logic."
```

---

## Task 4: Wire SSE route in http.rs

### Files
- Modify: `elohim/elohim-storage/src/http.rs`

### Step 1: Add the SSE route

In `handle_request`, add a new match arm for the SSE endpoint. Place it **before** the general `/api/v1/` catch-all route:

```rust
// SSE event stream
(Method::GET, "/api/v1/events") => {
    if let Some(ref services) = self.services {
        let response = crate::sse::create_sse_stream(services.event_bus());
        return Ok(response.map(Either::Right));
    } else {
        Ok(response::service_unavailable("Event bus not available"))
    }
}
```

**IMPORTANT:** The SSE response uses `Either::Right` (not `Either::Left` like normal responses). This is the whole point of the `Either` return type. The SSE route must `return Ok(...)` directly (bypassing the `.map(Either::Left)` wrapper at the bottom).

### Step 2: Check that Services exposes the EventBus

Read `elohim/elohim-storage/src/services/mod.rs` to find how to access the EventBus from `Services`. It should have something like `pub fn event_bus(&self) -> &Arc<EventBus>`. If not, add a getter.

### Step 3: Verify it compiles

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -10
```

### Step 4: Run tests

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -15
```

### Step 5: Commit

```bash
git add elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/services/mod.rs
git commit -m "feat(storage): wire SSE route at GET /api/v1/events

Subscribes to EventBus, streams StorageEvents as typed SSE frames
with 30s heartbeat. Returns Either::Right for the streaming body."
```

---

## Task 5: Angular EventStreamService

### Files
- Create: `app/elohim-app/src/app/elohim/services/event-stream.service.ts`
- Create: `app/elohim-app/src/app/elohim/services/event-stream.service.spec.ts`

### Step 1: Write the failing tests

```typescript
// event-stream.service.spec.ts
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

import { EventStreamService } from './event-stream.service';

// Mock EventSource
class MockEventSource {
  static instances: MockEventSource[] = [];
  listeners: Record<string, ((event: MessageEvent) => void)[]> = {};
  url: string;
  readyState = 0;
  close = vi.fn();

  constructor(url: string) {
    this.url = url;
    this.readyState = 1; // OPEN
    MockEventSource.instances.push(this);
  }

  addEventListener(type: string, listener: (event: MessageEvent) => void): void {
    if (!this.listeners[type]) this.listeners[type] = [];
    this.listeners[type].push(listener);
  }

  removeEventListener(type: string, listener: (event: MessageEvent) => void): void {
    if (this.listeners[type]) {
      this.listeners[type] = this.listeners[type].filter(l => l !== listener);
    }
  }

  // Test helper: simulate an event
  emit(type: string, data: string): void {
    const event = new MessageEvent(type, { data });
    this.listeners[type]?.forEach(l => l(event));
  }
}

describe('EventStreamService', () => {
  let service: EventStreamService;
  let originalEventSource: typeof EventSource;

  beforeEach(() => {
    MockEventSource.instances = [];
    originalEventSource = globalThis.EventSource;
    (globalThis as unknown as Record<string, unknown>).EventSource = MockEventSource;

    TestBed.configureTestingModule({
      providers: [EventStreamService],
    });
    service = TestBed.inject(EventStreamService);
  });

  afterEach(() => {
    service.disconnect();
    (globalThis as unknown as Record<string, unknown>).EventSource = originalEventSource;
  });

  it('should create EventSource on connect', () => {
    service.connect('/api/v1/events');
    expect(MockEventSource.instances.length).toBe(1);
    expect(MockEventSource.instances[0].url).toBe('/api/v1/events');
  });

  it('should close EventSource on disconnect', () => {
    service.connect('/api/v1/events');
    const es = MockEventSource.instances[0];
    service.disconnect();
    expect(es.close).toHaveBeenCalled();
  });

  it('should emit matching events via on()', () => {
    service.connect('/api/v1/events');
    const values: unknown[] = [];
    service.on<{ id: string }>('content.created').subscribe(v => values.push(v));

    MockEventSource.instances[0].emit('content.created', '{"id":"abc"}');

    expect(values.length).toBe(1);
    expect(values[0]).toEqual({ id: 'abc' });
  });

  it('should not emit events after disconnect', () => {
    service.connect('/api/v1/events');
    const values: unknown[] = [];
    service.on('content.created').subscribe(v => values.push(v));
    service.disconnect();

    // The subscription should complete on disconnect
    expect(values.length).toBe(0);
  });

  it('should handle multiple event types independently', () => {
    service.connect('/api/v1/events');
    const created: unknown[] = [];
    const updated: unknown[] = [];

    service.on('content.created').subscribe(v => created.push(v));
    service.on('content.updated').subscribe(v => updated.push(v));

    MockEventSource.instances[0].emit('content.created', '{"id":"1"}');
    MockEventSource.instances[0].emit('content.updated', '{"id":"2"}');

    expect(created.length).toBe(1);
    expect(updated.length).toBe(1);
  });

  it('should not create duplicate connections', () => {
    service.connect('/api/v1/events');
    service.connect('/api/v1/events');
    expect(MockEventSource.instances.length).toBe(1);
  });
});
```

### Step 2: Run tests to verify they fail

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "event-stream" 2>&1 | tail -15
```

### Step 3: Write the implementation

```typescript
// event-stream.service.ts
import { Injectable, OnDestroy } from '@angular/core';
import { Observable, Subject } from 'rxjs';
import { filter, map, takeUntil } from 'rxjs/operators';

interface SseEvent {
  type: string;
  data: unknown;
}

@Injectable({ providedIn: 'root' })
export class EventStreamService implements OnDestroy {
  private eventSource: EventSource | null = null;
  private readonly events$ = new Subject<SseEvent>();
  private readonly destroy$ = new Subject<void>();
  private activeListeners: Array<{ type: string; listener: (e: MessageEvent) => void }> = [];

  connect(url: string): void {
    if (this.eventSource) return;
    this.eventSource = new EventSource(url);
  }

  disconnect(): void {
    if (!this.eventSource) return;

    // Remove all registered listeners
    for (const { type, listener } of this.activeListeners) {
      this.eventSource.removeEventListener(type, listener);
    }
    this.activeListeners = [];

    this.eventSource.close();
    this.eventSource = null;
    this.destroy$.next();
  }

  on<T = unknown>(eventType: string): Observable<T> {
    if (!this.eventSource) {
      return new Observable<T>(subscriber => subscriber.complete());
    }

    const listener = (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data);
        this.events$.next({ type: eventType, data });
      } catch {
        // Ignore malformed JSON
      }
    };

    this.eventSource.addEventListener(eventType, listener);
    this.activeListeners.push({ type: eventType, listener });

    return this.events$.pipe(
      takeUntil(this.destroy$),
      filter(e => e.type === eventType),
      map(e => e.data as T),
    );
  }

  ngOnDestroy(): void {
    this.disconnect();
    this.events$.complete();
    this.destroy$.complete();
  }
}
```

### Step 4: Run tests to verify they pass

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "event-stream" 2>&1 | tail -15
```

### Step 5: Add to barrel export

Check `app/elohim-app/src/app/elohim/services/index.ts` and add:
```typescript
export { EventStreamService } from './event-stream.service';
```

### Step 6: Commit

```bash
git add app/elohim-app/src/app/elohim/services/event-stream.service.ts \
       app/elohim-app/src/app/elohim/services/event-stream.service.spec.ts \
       app/elohim-app/src/app/elohim/services/index.ts
git commit -m "feat(elohim): add EventStreamService — browser SSE client

Wraps EventSource API with typed Observables per event type.
Auto-cleanup on disconnect. 6 tests covering connect, disconnect,
event filtering, and deduplication."
```

---

## Task 6: Integration Verification

### Step 1: Run all Rust tests

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins 2>&1 | tail -15
```

Expected: All tests pass including the new SSE tests.

### Step 2: Run Rust clippy

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings 2>&1 | tail -15
```

### Step 3: Run Angular event-stream tests

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "event-stream" 2>&1 | tail -15
```

### Step 4: Run full Angular test suite

```bash
cd app/elohim-app && pnpm exec vitest run --config vite.config.ts 2>&1 | tail -20
```

Expected: No regressions.

### Step 5: Run Angular lint

```bash
cd app/elohim-app && pnpm run lint 2>&1 | tail -15
```

### Step 6: Verify proxy covers the SSE endpoint

```bash
grep -n "api" app/elohim-app/proxy.conf.mjs
```

Expected: `/api` is in the context array, which covers `/api/v1/events`.

### Step 7: Show git log

```bash
git log --oneline feature/sse-streaming --not dev
```
