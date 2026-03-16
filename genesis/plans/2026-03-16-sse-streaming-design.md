# SSE Streaming — Design

**Goal:** Add Server-Sent Events infrastructure to elohim-storage so browsers can receive real-time event streams. Single multiplexed endpoint carrying typed events from the existing EventBus.

---

## Architecture

```
Browser ←──SSE──→ GET /api/v1/events ←── EventBus (tokio::broadcast)
                  Content-Type:              ↑
                  text/event-stream    Service layer emits StorageEvent
                                       (local mutations + peer-synced)
```

SSE is purely browser-facing. libp2p handles peer-to-peer propagation separately — when a remote peer syncs content, the local service layer processes it and emits `StorageEvent` through the same `EventBus`. SSE clients see both local and peer-synced changes automatically.

---

## Rust Backend

### Return Type Change

`handle_request` currently returns `Response<Full<Bytes>>`. SSE requires a streaming body. Solution: change the return type to use `http_body_util::Either`:

```rust
type SseBody = StreamBody<Pin<Box<dyn Stream<Item = Result<Frame<Bytes>, Infallible>> + Send>>>;
type ApiBody = Either<Full<Bytes>, SseBody>;

async fn handle_request(&self, req: Request<Incoming>) -> Result<Response<ApiBody>, hyper::Error>
```

All existing handlers wrap their `Full<Bytes>` response in `Either::Left(...)`. The SSE handler returns `Either::Right(StreamBody::new(...))`.

### SSE Module (`src/sse.rs`)

- `format_sse_event(event_type, data, id) -> String` — formats `event: {type}\nid: {id}\ndata: {json}\n\n`
- `handle_sse_stream(event_bus, last_event_id) -> Response<SseBody>` — subscribes to EventBus, maps StorageEvents to SSE text frames, interleaves heartbeat every 30s
- Maps `StorageEvent` variants to SSE event types: `content.created`, `content.updated`, `content.deleted`, `path.created`, etc.

### Route

```rust
(Method::GET, "/api/v1/events") => {
    // Check Accept header for text/event-stream
    self.handle_sse_events(req).await
}
```

### Heartbeat

`: heartbeat\n\n` comment line every 30 seconds. Keeps connections alive through proxies and load balancers. Uses `tokio::time::interval` interleaved into the stream via `tokio_stream::StreamExt::merge` or `tokio::select!`.

### Backpressure

`tokio::broadcast` drops old messages when a receiver lags (`RecvError::Lagged(n)`). This is correct for SSE — slow clients miss intermediate events. The `Last-Event-ID` header enables reconnection replay from a bounded buffer (future enhancement, not this sprint).

### Dependencies

Add `tokio-stream` to `Cargo.toml` (for `BroadcastStream` wrapper). Everything else (`http-body-util`, `hyper`, `tokio::sync::broadcast`, `bytes`) already present.

---

## Angular Frontend

### EventStreamService

New service in the **elohim** pillar (cross-pillar infrastructure):

```typescript
@Injectable({ providedIn: 'root' })
export class EventStreamService {
  private eventSource: EventSource | null = null;

  connect(url: string): void { ... }
  disconnect(): void { ... }
  on<T>(eventType: string): Observable<T> { ... }
}
```

- Wraps browser `EventSource` API
- `on('content.created')` returns typed Observable that filters by event type
- Auto-reconnects (EventSource does this natively with exponential backoff)
- Connects to `/api/v1/events` via the existing proxy config (already covers `/api/*`)

### Tests

- Service creates EventSource on connect
- Service cleans up on disconnect
- `on()` returns Observable that emits matching events
- Handles reconnection

---

## Event Type Mapping

| StorageEvent | SSE event type |
|-------------|---------------|
| ContentCreated | `content.created` |
| ContentUpdated | `content.updated` |
| ContentDeleted | `content.deleted` |
| ContentBulkCreated | `content.bulk-created` |
| PathCreated | `path.created` |
| PathUpdated | `path.updated` |
| PathDeleted | `path.deleted` |
| RelationshipCreated | `relationship.created` |
| RelationshipDeleted | `relationship.deleted` |
| KnowledgeMapCreated | `knowledge-map.created` |
| KnowledgeMapUpdated | `knowledge-map.updated` |
| PathExtensionCreated | `path-extension.created` |

Future event types (not this sprint): `gate.evaluation`, `observation.created`

---

## What This Sprint Does NOT Include

- Per-human filtering (all events broadcast; client filters)
- Last-Event-ID replay buffer
- Gate evaluation streaming (needs sidecar)
- Observation streaming (needs imagodei API)
- Gossipsub/libp2p integration

---

## Build Order

1. Add `tokio-stream` dependency
2. Create `src/sse.rs` — formatting utilities + stream handler
3. Change `handle_request` return type to `Either<Full<Bytes>, SseBody>`
4. Wire SSE route in `http.rs`
5. Angular `EventStreamService` with tests
6. Integration verification
