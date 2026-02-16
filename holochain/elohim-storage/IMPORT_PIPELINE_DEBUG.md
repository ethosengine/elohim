# Import Pipeline Debug Guide

## Overview

This document traces the complete import pipeline from the seeder CLI through to the Holochain conductor, documenting all component interactions, communication flows, and known issues.

## Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                              GENESIS SEEDER                                   │
│  genesis/seeder/src/seed.ts                                                   │
│  ├── Loads JSON content from data/lamad/content/*.json                        │
│  ├── Runs pre-flight verification (verification.ts)                           │
│  └── Sends HTTP POST to doorway /import/batch                                 │
└──────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼ HTTP POST /import/batch
┌──────────────────────────────────────────────────────────────────────────────┐
│                              DOORWAY (Rust)                                   │
│  holochain/doorway/src/routes/import.rs                                       │
│  ├── Receives import batch request                                            │
│  ├── Stores blob in elohim-storage via HTTP                                   │
│  └── Proxies to elohim-storage import API                                     │
└──────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼ HTTP POST /import/queue
┌──────────────────────────────────────────────────────────────────────────────┐
│                           ELOHIM-STORAGE (Rust)                               │
│  holochain/elohim-storage/src/import_api.rs                                   │
│  ├── Queues batch for async processing                                        │
│  ├── Spawns async task to process batch                                       │
│  ├── ensure_cell_id() → cell_discovery.rs                                     │
│  │   └── *** CURRENT FAILURE POINT ***                                        │
│  └── Calls conductor zome via conductor_client.rs                             │
└──────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼ WebSocket (msgpack)
┌──────────────────────────────────────────────────────────────────────────────┐
│                         HOLOCHAIN CONDUCTOR                                   │
│  Admin interface (ws://localhost:4444)                                        │
│  ├── list_apps → returns app info with cell_info                              │
│  │   └── Cell format varies by version/client                                 │
│  └── issueAppAuthenticationToken                                              │
│                                                                               │
│  App interface (ws://localhost:4445)                                          │
│  ├── call_zome("content_store", "create_content", ...)                        │
│  └── call_zome("content_store", "process_import_chunk", ...)                  │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Component Files

### 1. Genesis Seeder (TypeScript)

| File | Purpose |
|------|---------|
| `genesis/seeder/src/seed.ts` | Main seeder CLI, orchestrates content loading and import |
| `genesis/seeder/src/verification.ts` | Pre-flight checks (conductor connectivity, write capability) |
| `genesis/seeder/src/doorway-client.ts` | HTTP client for doorway API |
| `genesis/seeder/src/progress-client.ts` | WebSocket client for progress streaming |
| `genesis/seeder/src/diagnose.ts` | Diagnostic CLI for checking conductor/doorway health |

### 2. Doorway (Rust)

| File | Purpose |
|------|---------|
| `holochain/doorway/src/routes/import.rs` | Import batch HTTP endpoint |
| `holochain/doorway/src/routes/api.rs` | Zome call proxying |
| `holochain/doorway/src/server/http.rs` | HTTP server setup |
| `holochain/doorway/src/services/import_client.rs` | Client for elohim-storage import API |

### 3. Elohim-Storage (Rust) - **CRITICAL FOR DEBUGGING**

| File | Purpose |
|------|---------|
| `holochain/elohim-storage/src/import_api.rs` | Batch import processing, backpressure handling |
| `holochain/elohim-storage/src/cell_discovery.rs` | **ISSUE: Cell ID discovery from conductor** |
| `holochain/elohim-storage/src/conductor_client.rs` | WebSocket connection to conductor app interface |
| `holochain/elohim-storage/src/main.rs` | Server startup, config parsing |
| `holochain/elohim-storage/src/error.rs` | Error types |

### 4. Holochain DNA (Rust)

| File | Purpose |
|------|---------|
| `holochain/dna/elohim/zomes/content_store/src/lib.rs` | Content creation zome functions |
| `holochain/dna/elohim/zomes/content_store/src/import.rs` | Batch import chunk processing |

## Communication Flow

### Step 1: Seeder Pre-flight

```
Seeder (seed.ts)
    │
    ├─1─► AdminWebsocket.connect(ADMIN_URL)
    │     ├─► list_apps → get app/cell info
    │     ├─► issueAppAuthenticationToken
    │     └─► authorizeSigningCredentials
    │
    ├─2─► AppWebsocket.connect(APP_URL, token)
    │     └─► callZome("get_content_count") ← verifies conductor works
    │
    └─3─► callZome("create_content", test_entry) ← pre-flight write test
```

### Step 2: Content Import

```
Seeder (seed.ts)
    │
    ├─1─► POST doorway/import/blob ← upload content JSON blob
    │
    ├─2─► POST doorway/import/queue ← queue batch for processing
    │     Body: { batch_id, blob_ref, item_count }
    │
    └─3─► WS doorway/import/progress/{batch_id} ← stream progress
          OR GET doorway/import/status/{batch_id} ← poll status
```

### Step 3: Storage Processing (Where Issue Occurs)

```
Elohim-Storage (import_api.rs)
    │
    ├─1─► ensure_cell_id(batch_id)
    │     └─► cell_discovery.rs::discover_cell_id()
    │         ├─► WS connect to conductor admin
    │         ├─► Send list_apps request (msgpack)
    │         ├─► Parse response ← ** PARSING FAILS HERE **
    │         └─► Extract cell_id from provisioned cell
    │
    ├─2─► Connect to conductor app interface
    │     ├─► get_auth_token (via admin)
    │     └─► authenticate connection
    │
    └─3─► For each chunk:
          └─► call_zome("process_import_chunk", items)
```

## The Current Issue

### Symptom
- Batch queues successfully (3525 items)
- Batch immediately fails at 0/3525
- WebSocket progress connection resets

### Root Cause (Suspected)
Cell discovery in `cell_discovery.rs` fails to parse the conductor's `list_apps` response.

The conductor returns cell info in different formats depending on:
1. **Holochain version** (0.3, 0.4, 0.6+)
2. **Client library** (Rust msgpack vs JS @holochain/client)

### Known Cell Formats

```rust
// Format 1: Holochain 0.3+ native (via msgpack)
{ type: "provisioned", value: { cell_id: { dna_hash: <bytes>, agent_pub_key: <bytes> } } }

// Format 2: JS client format
{ provisioned: { cell_id: [<dna_bytes>, <agent_bytes>] } }

// Format 3: Legacy
{ cell_id: [<dna_bytes>, <agent_bytes>] }
```

The seeder (TypeScript) uses `@holochain/client` and sees Format 2.
The storage (Rust) connects directly via msgpack and may see Format 1.

### Recent Fix Attempts

1. **Commit 50031fbd**: Added `extract_js_provisioned_cell_id()` to handle Format 2
2. **Commit 710b271a**: Fixed seeder to check for both formats
3. **Commit 57f6db7b**: Added debug logging to see actual response structure

## Debugging Steps

### 1. Check Storage Logs

The debug logging added in commit 57f6db7b will output:
- Response type (Map vs Array)
- Available keys in the response
- Cell keys when no format matches

Look for these log lines in storage container:
```
"Parsing list_apps response"
"Map response with keys"
"Cell found but format not recognized"
"App found but no provisioned cells matched"
```

### 2. Run Diagnostics Locally

```bash
# Test cell discovery against alpha environment
HOLOCHAIN_ADMIN_URL='wss://doorway-alpha.elohim.host?apiKey=dev-elohim-auth-2024' \
DOORWAY_URL='https://doorway-alpha.elohim.host' \
npx tsx genesis/seeder/src/diagnose.ts --cells
```

### 3. Add Temporary Raw Response Logging

In `cell_discovery.rs`, you can add:
```rust
// After line 77 (after getting response)
info!(raw_response = ?response, "Raw list_apps response");
```

### 4. Check Conductor Format Directly

Use the debug WebSocket to see what the conductor actually returns:
```bash
# In holochain sandbox
hc sandbox call admin list_apps
```

## Backpressure Handling

The import API now includes adaptive backpressure (commit 57f6db7b):

1. **Adaptive Delay**: Adjusts chunk delay based on response times
2. **Exponential Backoff**: Doubles delay after errors (up to 5s max)
3. **Circuit Breaker**: Pauses 10s after 5 consecutive errors

Configuration in `ImportApiConfig`:
```rust
chunk_delay: Duration::from_millis(100),     // Base delay
max_chunk_delay: Duration::from_secs(5),     // Maximum delay
circuit_breaker_threshold: 5,                 // Errors before pause
circuit_breaker_pause: Duration::from_secs(10), // Pause duration
```

## Next Steps for Fresh Developer

1. **Verify the deployed image has the latest code**
   - Check edge pipeline built after commit 57f6db7b
   - Verify storage container is using new image

2. **Examine the debug logs**
   - Access storage container logs (kubectl logs or similar)
   - Look for the cell discovery logging added

3. **Identify the actual response format**
   - The logs should show which keys are in the conductor response
   - Add handler for the actual format if it's different from expected

4. **Consider msgpack vs JSON differences**
   - The seeder uses JSON over WebSocket
   - The storage uses msgpack directly
   - Formats may serialize differently

5. **Test with local conductor**
   - Spin up local holochain sandbox
   - Connect storage to local conductor
   - Add verbose logging to see raw bytes

## Related Documentation

- [Holochain Client API](https://docs.rs/holochain_client)
- [Holochain Admin API](https://docs.rs/holochain_conductor_api)
- [MessagePack Spec](https://msgpack.org/index.html)
- [Project Architecture](./EDGE-ARCHITECTURE.md)
