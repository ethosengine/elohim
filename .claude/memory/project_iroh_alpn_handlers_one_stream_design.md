---
name: Phase 5–10 ALPN handlers — one-stream design + post-fix loop pattern
description: All custom iroh ALPN handlers (sync/EPR/EPR-atom/shard/view-fed/identity/trust) were originally one-stream-per-connection by design; bench discovery forced a loop-on-accept_bi pattern that supports both. Records the discovery and the resulting bench scenarios.
type: project
originSessionId: a343d895-dee4-491c-a4db-adda4c79312f
---
Every Phase 5–10 ALPN ProtocolHandler::accept in elohim/elohim-storage/src/p2p_iroh/ — sync, epr, epr_atom, shard, view_fed, identity, trust — was originally shaped:

```
accept_bi() once → handle one request → write one response → connection.closed().await → return
```

Production clients (e.g. `IrohSyncClient::request`) match this by always opening a fresh QUIC connection per request. That's internally consistent but means every chatty protocol pays full QUIC handshake cost per request.

**Discovery (May 2026):** the new bench_sync_perf tried to amortize handshake cost by reusing one connection across many `open_bi()` calls. Stream 1 succeeded, then the handler returned and parked on `closed()` — stream 2 onward had no `accept_bi` waiting, so the fetcher's `read_frame_default` waited forever (no read timeout in the bench either, which is why it hung silently for 86 minutes before being noticed). Diagnosed via `/proc/<pid>/task/*/{stack,wchan}` and per-FD UDP socket inspection — gdb wasn't available in the Eclipse Che container.

**Fix:** wrap each handler's body in `loop { match accept_bi { Ok(s) => ...; Err(_) => return Ok(()) } }`. Sequential, breaks cleanly on connection close. Single-stream callers (production today) keep working unchanged; multi-stream callers (bench reuse, future Phase 11 conn pool) now work too. Validated against existing `iroh_*_parity` integration tests.

**Why:** This change was discovered via perf-bench expansion, not feature work. The handlers were correct for production usage but not benchmarkable in stream-reuse mode. Keep both modes working — production uses one shape today and may use the other after Phase 11.

**How to apply:**
- Any new ALPN ProtocolHandler in p2p_iroh should use the loop pattern from the start.
- Any benchmark or test that wants to reuse a QUIC connection across requests must rely on this loop.
- All bench fetcher reads on iroh side must be wrapped in `tokio::time::timeout(30s, ...)` — silent infinite hangs are unsafe for unattended runs (this was the 86-minute lesson).
- The bench reports two scenarios: `fresh` (handshake per request — matches `IrohSyncClient::request` today) and `reuse` (engine ceiling — what stream multiplexing can deliver once handshake is amortized). They tell different stories; pick the right one when comparing to libp2p (which always reuses by default).
