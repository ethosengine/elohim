# Connection Module

Provides connection strategies for different Holochain deployment modes in the Elohim Protocol.

## Overview

The connection module implements a **Strategy Pattern** that encapsulates the logic for connecting to Holochain conductors in different deployment environments:

| Mode | Strategy | Use Case |
|------|----------|----------|
| **Doorway** | `DoorwayConnectionStrategy` | Browser/web deployments via proxy |
| **Direct** | `DirectConnectionStrategy` | Native/Tauri/CLI deployments |

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Connection Architecture                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   Browser/Web                           Native/Tauri/CLI             │
│        │                                      │                      │
│        ▼                                      ▼                      │
│  ┌─────────────┐                       ┌─────────────┐              │
│  │   Doorway   │                       │   Direct    │              │
│  │  Strategy   │                       │  Strategy   │              │
│  └──────┬──────┘                       └──────┬──────┘              │
│         │                                     │                      │
│         ▼                                     ▼                      │
│  ┌─────────────┐                       ┌─────────────┐              │
│  │   Doorway   │                       │    Local    │              │
│  │    Proxy    │                       │  Conductor  │              │
│  │ (wss://...) │                       │(ws://4444)  │              │
│  └──────┬──────┘                       └──────┬──────┘              │
│         │                                     │                      │
│         ▼                                     ▼                      │
│  ┌─────────────┐                       ┌─────────────┐              │
│  │ Projection  │                       │   elohim-   │              │
│  │   (Mongo)   │                       │   storage   │              │
│  └──────┬──────┘                       │  (8090)     │              │
│         │                              └──────┬──────┘              │
│         ▼                                     │                      │
│  ┌─────────────┐                              │                      │
│  │ Conductor   │◄─────────────────────────────┘                      │
│  │    (DHT)    │                                                     │
│  └─────────────┘                                                     │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Quick Start

### Basic Usage (Framework-Agnostic)

```typescript
import {
  createConnectionStrategy,
  type ConnectionConfig,
} from '@elohim/service/connection';

// Auto-detect mode based on environment
const strategy = createConnectionStrategy('auto');

// Configure connection
const config: ConnectionConfig = {
  mode: strategy.mode,
  adminUrl: 'wss://doorway-alpha.elohim.host',
  appUrl: 'wss://doorway-alpha.elohim.host',
  appId: 'elohim',
  proxyApiKey: 'your-api-key',
};

// Connect to Holochain
const result = await strategy.connect(config);

if (result.success) {
  console.log('Connected!', {
    agentPubKey: result.agentPubKey,
    cellCount: result.cellIds?.size,
  });
}

// Get blob storage URL
const blobUrl = strategy.getBlobStorageUrl(config, 'sha256-hash');
```

### Angular Integration

```typescript
import { Component, inject } from '@angular/core';
import { CONNECTION_STRATEGY } from '@app/elohim/providers';

@Component({...})
export class MyComponent {
  private strategy = inject(CONNECTION_STRATEGY);

  async connect() {
    // Strategy is automatically selected based on environment.holochain.connectionMode
    console.log(`Using ${this.strategy.name} strategy`);

    const result = await this.strategy.connect({
      mode: this.strategy.mode,
      adminUrl: environment.holochain.adminUrl,
      appUrl: environment.holochain.appUrl,
      appId: 'elohim',
    });
  }
}
```

## Strategies

### DoorwayConnectionStrategy

For browser/web deployments where WebSocket connections must route through a proxy.

**URL Resolution:**
- Admin: `wss://doorway-alpha.elohim.host?apiKey=...`
- App: `wss://doorway-alpha.elohim.host/app/{port}?apiKey=...`
- Blob: `https://doorway-alpha.elohim.host/api/blob/{hash}?apiKey=...`

**Content Sources:**
| Source | Tier | Priority | Description |
|--------|------|----------|-------------|
| `indexeddb` | Local | 100 | Browser cache (offline-capable) |
| `projection` | Projection | 80 | Doorway's MongoDB cache (fast) |
| `conductor` | Authoritative | 50 | Holochain DHT (source of truth) |

**Eclipse Che Support:**
The strategy auto-detects Eclipse Che environments and routes through the dev-proxy:
```
https://{workspace}-hc-dev.code.ethosengine.com/admin
https://{workspace}-hc-dev.code.ethosengine.com/app/{port}
```

### DirectConnectionStrategy

For native/Tauri deployments with direct conductor access.

**URL Resolution:**
- Admin: `ws://localhost:4444`
- App: `ws://localhost:{port}`
- Blob: `http://localhost:8090/store/{hash}`

**Content Sources:**
| Source | Tier | Priority | Description |
|--------|------|----------|-------------|
| `indexeddb` | Local | 100 | Local cache |
| `conductor` | Authoritative | 90 | Direct conductor access |
| `elohim-storage` | Authoritative | 85 | Blob storage sidecar |

**Note:** Direct mode skips the Projection tier entirely since there's no Doorway proxy.

## Auto-Detection

The factory can auto-detect the appropriate mode:

```typescript
import { detectConnectionMode, createConnectionStrategy } from '@elohim/service/connection';

// Detection order:
// 1. Tauri (window.__TAURI__) → direct
// 2. Node.js (process.versions.node) → direct
// 3. Browser → doorway

const mode = detectConnectionMode();
const strategy = createConnectionStrategy('auto'); // Uses detected mode
```

## Connection Flow

Both strategies implement the same 11-step connection flow:

```
1. Connect to AdminWebsocket
2. Generate signing credentials (Ed25519 keypair + cap secret)
3. Generate agent public key
4. Check if app installed, install if needed
5. Extract cell IDs (multi-DNA support)
6. Grant zome call capability for all cells
7. Register signing credentials with conductor
8. Find or create app interface
9. Authorize signing credentials
10. Issue app authentication token
11. Connect to AppWebsocket with token
```

## Configuration

### Environment Types

```typescript
// environment.types.ts
export type ConnectionMode = 'auto' | 'doorway' | 'direct';

export interface HolochainEnvironmentConfig {
  adminUrl: string;
  appUrl: string;
  connectionMode?: ConnectionMode;  // Default: 'auto'
  storageUrl?: string;              // elohim-storage URL for direct mode
  proxyApiKey?: string;             // API key for doorway mode
}
```

### Environment Files

**Development (environment.ts):**
```typescript
export const environment = {
  holochain: {
    adminUrl: 'wss://doorway-alpha.elohim.host',
    appUrl: 'wss://doorway-alpha.elohim.host',
    connectionMode: 'auto',
    proxyApiKey: 'dev-api-key',
  },
};
```

**Native (environment.native.ts):**
```typescript
export const environment = {
  holochain: {
    adminUrl: 'ws://localhost:4444',
    appUrl: 'ws://localhost:4445',
    connectionMode: 'direct',
    storageUrl: 'http://localhost:8090',
  },
};
```

## ContentResolver Integration

Initialize the content resolver with mode-aware sources:

```typescript
import { ContentResolverService } from '@app/elohim/services';
import { CONNECTION_STRATEGY } from '@app/elohim/providers';

@Injectable()
export class AppInitService {
  private resolver = inject(ContentResolverService);
  private strategy = inject(CONNECTION_STRATEGY);

  async initialize() {
    // Initialize resolver
    await this.resolver.initialize();

    // Configure sources based on connection mode
    await this.resolver.initializeForMode(this.strategy, {
      mode: this.strategy.mode,
      adminUrl: environment.holochain.adminUrl,
      // ...
    });
  }
}
```

## API Reference

### IConnectionStrategy Interface

```typescript
interface IConnectionStrategy {
  // Identity
  readonly name: string;                              // 'doorway' | 'direct'
  readonly mode: Exclude<ConnectionMode, 'auto'>;     // 'doorway' | 'direct'

  // Environment Detection
  isSupported(): boolean;

  // URL Resolution
  resolveAdminUrl(config: ConnectionConfig): string;
  resolveAppUrl(config: ConnectionConfig, port: number): string;
  getBlobStorageUrl(config: ConnectionConfig, blobHash: string): string;

  // Content Sources
  getContentSources(config: ConnectionConfig): ContentSourceConfig[];

  // Connection Lifecycle
  connect(config: ConnectionConfig): Promise<ConnectionResult>;
  disconnect(): Promise<void>;
  isConnected(): boolean;

  // Signing Credentials
  getSigningCredentials(): SigningCredentials | null;
}
```

### ConnectionResult

```typescript
interface ConnectionResult {
  success: boolean;
  adminWs?: AdminWebsocket;
  appWs?: AppWebsocket;
  cellIds?: Map<string, CellId>;  // Role name → CellId
  agentPubKey?: AgentPubKey;
  appInfo?: AppInfo;
  appPort?: number;
  error?: string;
}
```

### ContentSourceConfig

```typescript
interface ContentSourceConfig {
  id: string;                    // Unique source identifier
  tier: SourceTier;              // Local, Projection, Authoritative, External
  priority: number;              // Higher = preferred
  contentTypes: string[];        // Content types this source provides
  baseUrl?: string;              // URL for URL-based sources
  available: boolean;            // Whether source is initially available
}
```

## Testing

Run unit tests:
```bash
npm test -- --testPathPattern=connection-strategy
```

Tests cover:
- Factory creation and auto-detection
- URL resolution for both strategies
- Blob storage URL construction
- Content source configuration
- Connection state management

## Migration Guide

### From HolochainClientService Direct Usage

**Before:**
```typescript
// Old: Direct URL construction in service
const adminUrl = this.resolveAdminUrl();
const adminWs = await AdminWebsocket.connect({ url: new URL(adminUrl) });
// ... 160+ lines of connection logic
```

**After:**
```typescript
// New: Delegate to strategy
const result = await this.strategy.connect(config);
if (result.success) {
  this.updateState({
    adminWs: result.adminWs,
    appWs: result.appWs,
    cellIds: result.cellIds,
    // ...
  });
}
```

### From Hardcoded Blob URLs

**Before:**
```typescript
const blobUrl = `https://doorway-alpha.elohim.host/api/blob/${hash}`;
```

**After:**
```typescript
const blobUrl = this.strategy.getBlobStorageUrl(config, hash);
// Doorway: https://doorway-alpha.elohim.host/api/blob/{hash}
// Direct:  http://localhost:8090/store/{hash}
```

## Unified HTTP API Access

Both strategies access the **same** elohim-storage HTTP API - the only difference is the host:

```
┌─────────────────────────────────────────────────────────────┐
│                    elohim-storage                           │
│              (http.rs / views.rs unified API)               │
│                                                             │
│  Endpoints:                                                 │
│  • /db/content, /db/paths, /db/relationships               │
│  • /session (native auth)                                   │
│  • /store/{hash} (blob storage)                            │
│                                                             │
│  All responses: camelCase JSON                             │
└─────────────────────────────────────────────────────────────┘
                          ▲
            ┌─────────────┴─────────────┐
            │                           │
   ┌────────┴────────┐        ┌────────┴────────┐
   │    Doorway      │        │     Tauri       │
   │ (proxies to     │        │ (direct HTTP    │
   │  storage)       │        │  localhost:8090)│
   └─────────────────┘        └─────────────────┘
```

**Key Insight**: Tauri does NOT use Rust FFI or direct SQLite bindings. It makes standard HTTP fetch calls to the same `http.rs` endpoints. This ensures:

- **Single API boundary** (`views.rs`) for all clients
- **Consistent camelCase transformation** regardless of access path
- **Same validation and error handling** for all modes

## Architecture Decision Records

### ADR-001: Strategy Pattern for Connection Modes

**Context:** The elohim-app needs to support both browser (via Doorway proxy) and native (via direct conductor) deployments.

**Decision:** Implement a Strategy Pattern that encapsulates URL resolution, blob storage, and content source configuration per deployment mode.

**Consequences:**
- ✅ Single `connect()` call handles all deployment modes
- ✅ New modes (e.g., Holo hosting) can be added without modifying services
- ✅ ContentResolver sources are mode-aware
- ✅ HolochainClientService reduced from ~840 lines to ~300 lines

### ADR-002: HTTP API Over IPC for Tauri

**Context:** Tauri apps could use Rust IPC for direct database access or HTTP for API consistency.

**Decision:** Use HTTP fetch to elohim-storage sidecar (port 8090) for all data operations.

**Consequences:**
- ✅ Single API boundary in `views.rs` for all clients
- ✅ Consistent camelCase transformation
- ✅ Same validation/error handling code paths
- ✅ Easier testing (HTTP is more observable than IPC)
- ⚠️ Slightly more latency than direct IPC (minimal in practice)
