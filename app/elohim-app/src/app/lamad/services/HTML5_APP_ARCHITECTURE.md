# HTML5 App Plugin Architecture

## Overview

Serve HTML5 applications (like Evolution of Trust) from zip blobs stored in Holochain,
using **Doorway's P2P content publisher model** for server-side zip extraction.

**Key insight**: Kolibri uses server-side WSGI to serve from zips. Our equivalent is
Doorway - a federated gateway that discovers content publishers from the DHT and
serves extracted files on demand.

**Pattern**: Layered architecture:
- **doorway-client crate**: Publishing traits (Publishable, ContentServer, PublishSignal)
- **holochain-cache-core**: Zip extraction, file serving, content resolution (TODO)
- **doorway**: Host registry (like DNS) - registers hosts that serve Holochain content
  - Enables Web2.0 clients to access Holochain graph
  - Validation happens in Holochain, not doorway
- **elohim-app**: Angular renderer that loads iframe from registered host

## Package Structure

```
holochain/
├── crates/doorway-client/
│   └── src/
│       ├── lib.rs              # Caching traits (Cacheable, CacheRule)
│       └── publish.rs          # Publishing traits (Publishable, ContentServer, PublishSignal)
│
├── holochain-cache-core/       # TODO: Move app serving here
│   └── src/
│       └── app_server.rs       # Zip extraction, file serving (to be implemented)
│
└── doorway/                    # Infrastructure ONLY
    └── src/
        └── ...                 # Registry, federation, trust - NOT content serving

elohim-app/
└── src/app/lamad/content-io/plugins/
    └── html5-app/              # Angular adapter
        ├── index.ts
        ├── html5-app-format.plugin.ts    # ContentFormatPlugin impl
        └── html5-app-renderer.component.ts  # Iframe renderer
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Holochain DNA                                │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  content_store zome                                          │    │
│  │  - ContentNode { id, contentFormat: 'html5-app', blobs }    │    │
│  │  - ContentServer { content_hash, capability: Html5App }     │    │
│  │  - Uses doorway-client::Publishable trait                   │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
              │                                          │
              │ PublishSignal                           │ Query ContentServer
              │ (online/offline/heartbeat)               │ entries by hash
              ▼                                          ▼
┌─────────────────────────────────────────────────────────────────────┐
│                 holochain-cache-core (Content Layer)                 │
│  ┌───────────────────────────────────────────────────────────┐      │
│  │  app_server (TODO)                                         │      │
│  │  - GET /apps/{app-id}/{path}                               │      │
│  │  - Discovers publishers from DHT                           │      │
│  │  - Fetches zip from nearest publisher                      │      │
│  │  - Extracts and caches files (LRU eviction)                │      │
│  │  - Serves with proper Content-Type and CSP headers         │      │
│  └───────────────────────────────────────────────────────────┘      │
│  ┌───────────────────────────────────────────────────────────┐      │
│  │  AppCache                                                  │      │
│  │  - In-memory cache of extracted apps                       │      │
│  │  - LRU eviction when max size reached                      │      │
│  │  - Files: HashMap<path, ExtractedFile>                     │      │
│  └───────────────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────────────┘
              ↑
              │ implements doorway-client contract
              │
┌─────────────────────────────────────────────────────────────────────┐
│              Doorway (Infrastructure Layer - Host Registry)          │
│  - Registers "hosts" that can serve Holochain content               │
│  - Like DNS records - points Web2.0 to content hosts                │
│  - Trust tier computation (Emerging → Anchor)                       │
│  - Validation happens in Holochain, not doorway                     │
│  - Enables Web2.0 clients to access Holochain graph                 │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                      IframeRendererComponent                         │
│  - Detects contentFormat === 'html5-app'                            │
│  - Sets iframe.src = '{cacheLayerUrl}/apps/{app-id}/index.html'     │
│  - Sandbox: allow-scripts allow-same-origin                         │
│  - No client-side zip handling required                             │
└─────────────────────────────────────────────────────────────────────┘
```

## Data Flow

### 1. Content Node Structure

```typescript
interface ContentNode {
  id: 'simulation-evolution-of-trust';
  contentFormat: 'html5-app';
  content: {
    entryPoint: 'index.html';  // File to load in iframe
    appId: 'evolution-of-trust';  // URL namespace
  };
  blobs: [{
    hash: 'sha256-abc123...',
    sizeBytes: 6800000,
    mimeType: 'application/zip'
  }];
}
```

### 2. Publisher Registration (DNA side)

```rust
// In content_store zome, using doorway-client
use doorway_client::{ContentServer, ContentServerCapability, PublishSignal};

#[hdk_extern]
fn publish_html5_app(input: PublishInput) -> ExternResult<ActionHash> {
    // 1. Store the zip blob
    let hash = create_entry(&input.blob)?;

    // 2. Register as content server
    let server = ContentServer::new(
        input.blob.content_hash(),
        ContentServerCapability::Html5App,
    )
    .with_region("us-west");

    create_entry(&server)?;

    // 3. Signal to doorway
    emit_signal(PublishSignal::online(&server))?;

    Ok(hash)
}
```

### 3. Doorway Request Handling

```rust
// In doorway routes/apps.rs
pub async fn handle_app_request(
    req: Request,
    cache: Arc<AppCache>,
    content_lookup: impl Fn(&str) -> Option<AppMetadata>,
    publisher_fetch: impl AsyncPublisherFetch,
) -> Result<Response, AppError> {
    // 1. Parse path: /apps/{app-id}/{path}
    let (app_id, file_path) = parse_app_path(req.uri().path())?;

    // 2. Check cache
    if let Some(file) = cache.get_file(&app_id, &file_path).await {
        return Ok(serve_file(file));
    }

    // 3. Lookup content metadata
    let metadata = content_lookup(&app_id)?;

    // 4. Find publishers and fetch zip
    let zip_data = publisher_fetch.fetch(&metadata.blob_hash).await?;

    // 5. Extract and cache
    let cached_app = extract_and_cache(&app_id, &metadata, zip_data, cache).await?;

    // 6. Serve file
    cached_app.files.get(&file_path)
        .map(serve_file)
        .ok_or(AppError::FileNotFound(file_path))
}
```

### 4. Client-Side Loading (Simple!)

```typescript
// In IframeRendererComponent - no zip handling needed!
loadHtml5App(node: ContentNode) {
  const appConfig = node.content as Html5AppContent;
  const doorwayUrl = this.config.doorwayUrl;

  // Just set the iframe src - doorway handles everything
  this.iframeSrc = `${doorwayUrl}/apps/${appConfig.appId}/${appConfig.entryPoint}`;
}
```

## Caching Strategy

### Doorway-Side (Server)
The AppCache in doorway handles extracted files:

```rust
pub struct AppCache {
    /// Cached apps by app_id
    apps: RwLock<HashMap<String, Arc<CachedApp>>>,
    /// Maximum cache size in bytes
    max_size: usize,
    /// Current cache size
    current_size: RwLock<usize>,
}

pub struct CachedApp {
    pub app_id: String,
    pub blob_hash: String,
    pub entry_point: String,
    pub files: HashMap<String, ExtractedFile>,
    pub cached_at: u64,
    pub total_size: usize,
}
```

Features:
- LRU eviction when max size reached
- Apps cached until evicted or doorway restart
- First request extracts zip, subsequent requests hit cache
- Long cache headers (1 year, immutable) for browser caching

## Security Considerations

### Iframe Sandbox
```html
<iframe
  sandbox="allow-scripts allow-same-origin allow-forms"
  src="https://doorway.example.com/apps/evolution-of-trust/index.html"
></iframe>
```

### Content Security Policy (Doorway-Side)
Doorway adds CSP headers for HTML files:
```rust
"default-src 'self' 'unsafe-inline' 'unsafe-eval' data: blob:; frame-ancestors 'self'"
```

### Content Verification
- Zips are content-addressed by SHA256 hash
- Hash stored in Holochain DHT, immutable
- Publishers register via ContentServer entries
- Doorway verifies hash before serving

## Implementation Status

### ✅ Phase 1: doorway-client Publishing Extensions
1. Created `Publishable` trait for raw content serving
2. Created `ContentServer` entry type for DHT publisher registration
3. Created `PublishSignal` for publisher announcements
4. Added `Html5AppBundle` type for HTML5 app metadata

### ✅ Phase 2: Infrastructure DNA Integration
5. Added `ContentServer` entry type to `infrastructure_integrity` zome
6. Added `ContentServerCapability` constants (`blob`, `html5_app`, `media_stream`, etc.)
7. Added link types for publisher discovery

### ✅ Phase 3: Angular Renderer
8. Updated `IframeRendererComponent` to detect `contentFormat: 'html5-app'`
9. Added `Html5AppContent` interface for structured content (appId, entryPoint, fallbackUrl)
10. Added `Html5AppFormatPlugin` with import/export/validate/render capabilities
11. Registered plugin in `ContentIOModule`

### 🔲 Phase 4: holochain-cache-core App Server (TODO - Next Sprint)
**This is where HTML5 app serving belongs, NOT in doorway routes.**

Tasks for next sprint:
12. Create `app_server.rs` in holochain-cache-core
13. Implement AppCache with LRU eviction
14. Add zip extraction and file serving
15. Implement AsyncPublisherFetch for DHT queries
16. Wire up content_lookup to cached projections
17. Add MIME type detection and CSP headers

### 🔲 Phase 5: Content Seeding
18. Store Evolution of Trust zip in DHT
19. Register as content publisher
20. Test local app serving

## doorway-client API

### Publishable Trait

```rust
/// Content types that can be served as raw bytes
pub trait Publishable {
    fn content_hash(&self) -> String;
    fn content_type(&self) -> &'static str;
    fn mime_type(&self) -> &'static str;
    fn size_bytes(&self) -> u64;
    fn requires_auth(&self) -> bool { false }
    fn reach(&self) -> &str { "commons" }
    fn entry_point(&self) -> Option<&str> { None }
}
```

### ContentServer Entry

```rust
/// DHT entry registering an agent as content publisher
pub struct ContentServer {
    pub content_hash: String,
    pub capability: ContentServerCapability,
    pub serve_url: Option<String>,
    pub online: bool,
    pub priority: u8,
    pub region: Option<String>,
    pub bandwidth_mbps: Option<u32>,
    pub registered_at: u64,
    pub last_heartbeat: u64,
}

pub enum ContentServerCapability {
    Blob,        // Raw blob serving
    Html5App,    // Zip extraction + file serving
    MediaStream, // Range request support
    LearningPackage,
    Custom(String),
}
```

### PublishSignal

```rust
/// Signal for publisher announcements
pub struct PublishSignal {
    pub signal_type: PublishSignalType, // Online, Offline, Heartbeat, Removed
    pub content_hash: String,
    pub server: Option<ContentServer>,
}
```

## Angular Renderer (IframeRendererComponent)

The existing `IframeRendererComponent` was enhanced to support HTML5 apps:

```typescript
// iframe-renderer.component.ts
export interface Html5AppContent {
  appId: string;        // URL namespace (e.g., 'evolution-of-trust')
  entryPoint: string;   // File to load (e.g., 'index.html')
  fallbackUrl?: string; // External fallback if doorway unavailable
}

@Component({
  selector: 'app-iframe-renderer',
  template: `
    <div class="iframe-container" [class.loading]="loading">
      @if (loading) {
        <div class="loading-overlay">
          <div class="spinner"></div>
          <p>Loading application...</p>
        </div>
      }
      @if (errorMessage) {
        <div class="error-overlay">
          <p class="error-message">{{ errorMessage }}</p>
          @if (fallbackUrl) {
            <a [href]="fallbackUrl" target="_blank">Open in new tab</a>
          }
        </div>
      }
      <iframe
        [src]="safeUrl"
        [sandbox]="sandboxPolicy"
        [class.hidden]="loading"
        (load)="onIframeLoad()"
        (error)="onIframeError()"
      ></iframe>
    </div>
  `
})
export class IframeRendererComponent implements OnChanges {
  @Input() node!: ContentNode;

  safeUrl: SafeResourceUrl | null = null;
  loading = true;
  errorMessage: string | null = null;
  fallbackUrl: string | null = null;
  sandboxPolicy = 'allow-scripts allow-same-origin allow-forms';

  private configureIframe(): void {
    const { contentFormat, content, metadata } = this.node;

    // HTML5 App mode: content is Html5AppContent object
    if (contentFormat === 'html5-app' && this.isHtml5AppContent(content)) {
      const url = this.buildHtml5AppUrl(content);
      this.fallbackUrl = content.fallbackUrl || null;
      this.sandboxPolicy = this.getSandboxPolicy(metadata);
      this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(url);
      return;
    }

    // Direct URL mode: content is a string URL
    if (typeof content === 'string' && content.startsWith('http')) {
      this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(content);
    }
  }

  private buildHtml5AppUrl(content: Html5AppContent): string {
    const doorwayUrl = environment.doorwayUrl || '';
    return `${doorwayUrl}/apps/${content.appId}/${content.entryPoint}`;
  }
}
```

## Dependencies

### Doorway (Rust)
- `zip` crate for extraction
- `async-trait` for async publisher fetch trait
- `doorway-client` for Publishable/ContentServer types

### Angular
- No new dependencies - just uses iframe with doorway URLs

## References

- [Kolibri zip_wsgi.py](https://github.com/learningequality/kolibri) - Inspiration for server-side zip serving
- [Holochain Signals](https://developer.holochain.org/concepts/8_signals/) - For PublishSignal pattern
