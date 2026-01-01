# Doorway DID Federation

## Overview

Doorways use W3C Decentralized Identifiers (DIDs) to discover each other and locate content across the network. This enables content federation without centralized registries.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         DID FEDERATION LAYER                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   ┌─────────────┐         ┌─────────────┐         ┌─────────────┐      │
│   │  Doorway A  │◄───────►│  Doorway B  │◄───────►│  Doorway C  │      │
│   │             │   DID   │             │   DID   │             │      │
│   │ did:web:a.. │  resolve│ did:web:b.. │  resolve│ did:web:c.. │      │
│   └──────┬──────┘         └──────┬──────┘         └──────┬──────┘      │
│          │                       │                       │              │
│          └───────────────────────┼───────────────────────┘              │
│                                  │                                      │
│                                  ▼                                      │
│                    ┌─────────────────────────┐                         │
│                    │      Holochain DHT      │                         │
│                    │  (manifests, identity)  │                         │
│                    └─────────────────────────┘                         │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## The Problem: Content Location

When a doorway receives a request for content it doesn't have locally:

```
Browser → Doorway A → "GET /api/v1/blobs/Qm123..."
                    → Local cache? MISS
                    → Local Holochain? Has manifest, but not the blob
                    → Where are the actual bytes?
```

The Holochain manifest knows *who owns* the content, but not *where to fetch* the bytes.

## Solution: DIDs as Location Pointers

Holochain manifests include `storage_dids` - a list of DIDs that have the blob:

```rust
// In Holochain DNA (content_store_integrity)
#[hdk_entry_helper]
pub struct BlobManifest {
    pub hash: String,                    // Content hash (Qm...)
    pub owner: AgentPubKey,              // Who created it
    pub size_bytes: u64,
    pub content_type: Option<String>,
    pub storage_dids: Vec<String>,       // Who has the bytes
    pub created_at: Timestamp,
}
```

DIDs are resolved to service endpoints:

| DID | Resolves To |
|-----|-------------|
| `did:web:doorway-a.elohim.host` | HTTPS fetch `/.well-known/did.json` → service endpoints |
| `did:key:z6Mk...` | Decode ed25519 pubkey → lookup in P2P DHT |

## Doorway DID Document

Each doorway serves its DID Document at `/.well-known/did.json`:

```json
{
  "@context": [
    "https://www.w3.org/ns/did/v1",
    "https://w3id.org/security/suites/ed25519-2020/v1",
    "https://elohim-protocol.org/ns/v1"
  ],
  "id": "did:web:doorway-a.elohim.host",

  "verificationMethod": [{
    "id": "did:web:doorway-a.elohim.host#signing-key",
    "type": "Ed25519VerificationKey2020",
    "controller": "did:web:doorway-a.elohim.host",
    "publicKeyMultibase": "z6Mkq..."
  }],

  "authentication": ["did:web:doorway-a.elohim.host#signing-key"],
  "assertionMethod": ["did:web:doorway-a.elohim.host#signing-key"],

  "service": [
    {
      "id": "did:web:doorway-a.elohim.host#blobs",
      "type": "ElohimBlobStore",
      "serviceEndpoint": "https://doorway-a.elohim.host/api/v1/blobs"
    },
    {
      "id": "did:web:doorway-a.elohim.host#holochain",
      "type": "HolochainGateway",
      "serviceEndpoint": "wss://doorway-a.elohim.host/app/4445"
    },
    {
      "id": "did:web:doorway-a.elohim.host#humans",
      "type": "ElohimHumanRegistry",
      "serviceEndpoint": "https://doorway-a.elohim.host/api/v1/humans"
    }
  ],

  "elohim:capabilities": ["blob-storage", "gateway", "seeding"],
  "elohim:region": "us-west-2",
  "elohim:holochainCellId": "uhCAk..."
}
```

### Service Types

| Service Type | Purpose | Endpoint Format |
|--------------|---------|-----------------|
| `ElohimBlobStore` | Fetch/store blobs | `https://.../api/v1/blobs/{hash}` |
| `HolochainGateway` | WebSocket to Holochain | `wss://.../app/{port}` |
| `ElohimHumanRegistry` | Human profile lookups | `https://.../api/v1/humans/{id}` |
| `LibP2PNode` | Direct P2P connection | `/ip4/.../tcp/.../p2p/...` |

## Content Fetch Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    FEDERATED CONTENT FETCH                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   1. Browser requests blob from Doorway A                              │
│   2. Doorway A: cache miss                                             │
│   3. Doorway A queries Holochain for BlobManifest                      │
│   4. Manifest.storage_dids = ["did:web:doorway-b...", "did:key:z6..."] │
│   5. Doorway A resolves did:web:doorway-b...                           │
│      → GET https://doorway-b.elohim.host/.well-known/did.json          │
│      → Extract ElohimBlobStore endpoint                                │
│   6. Doorway A fetches blob from Doorway B                             │
│      → GET https://doorway-b.elohim.host/api/v1/blobs/Qm123...         │
│   7. Doorway A caches blob locally                                     │
│   8. Doorway A returns blob to browser                                 │
│                                                                         │
│   ┌─────────┐      ┌───────────┐      ┌───────────┐      ┌───────────┐ │
│   │ Browser │─────►│ Doorway A │─────►│ Holochain │      │ Doorway B │ │
│   └─────────┘      └─────┬─────┘      └─────┬─────┘      └─────▲─────┘ │
│                          │                  │                  │       │
│                          │   GET manifest   │                  │       │
│                          │◄─────────────────┘                  │       │
│                          │                                     │       │
│                          │   resolve DID                       │       │
│                          │────────────────────────────────────►│       │
│                          │                                     │       │
│                          │   GET /.well-known/did.json         │       │
│                          │◄────────────────────────────────────│       │
│                          │                                     │       │
│                          │   GET /api/v1/blobs/Qm123...        │       │
│                          │◄────────────────────────────────────┘       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## Registering as a Storage Location

When a doorway stores a blob (from seeding, user upload, or replication):

```rust
// doorway/src/services/blob_registry.rs

async fn register_blob_location(
    conductor: &ConductorHandle,
    blob_hash: &str,
    our_did: &str,
) -> Result<()> {
    // Get or create manifest in Holochain
    let manifest = match get_manifest(conductor, blob_hash).await? {
        Some(m) => m,
        None => BlobManifest {
            hash: blob_hash.to_string(),
            owner: our_agent_pubkey(),
            size_bytes: 0,  // Will be updated
            content_type: None,
            storage_dids: vec![],
            created_at: now(),
        },
    };

    // Add our DID if not already present
    if !manifest.storage_dids.contains(&our_did.to_string()) {
        let mut updated = manifest.clone();
        updated.storage_dids.push(our_did.to_string());
        update_manifest(conductor, updated).await?;
    }

    Ok(())
}
```

## DID Resolution

### Resolving did:web

```rust
// doorway/src/services/did_resolver.rs

async fn resolve_did_web(did: &str) -> Result<DIDDocument> {
    // did:web:doorway-a.elohim.host
    // → https://doorway-a.elohim.host/.well-known/did.json

    let url = did_web_to_url(did)?;
    let response = reqwest::get(&url).await?;
    let doc: DIDDocument = response.json().await?;

    // Cache for future lookups (5 min TTL)
    cache_did_document(did, &doc).await;

    Ok(doc)
}

fn did_web_to_url(did: &str) -> Result<String> {
    // did:web:example.com → https://example.com/.well-known/did.json
    // did:web:example.com:path:to → https://example.com/path/to/did.json

    let parts: Vec<&str> = did.strip_prefix("did:web:")
        .ok_or("Invalid did:web")?
        .split(':')
        .collect();

    let domain = parts[0];
    let path = if parts.len() > 1 {
        format!("/{}/did.json", parts[1..].join("/"))
    } else {
        "/.well-known/did.json".to_string()
    };

    Ok(format!("https://{}{}", domain, path))
}
```

### Resolving did:key

```rust
async fn resolve_did_key(did: &str) -> Result<DIDDocument> {
    // did:key:z6Mkq... → decode multibase → ed25519 public key
    // Then lookup in P2P DHT for multiaddrs

    let pubkey = decode_did_key(did)?;

    // Query Holochain for StorageNodeRegistration with this key
    let registration = lookup_storage_node_by_key(&pubkey).await?;

    // Build minimal DID document from registration
    Ok(DIDDocument {
        id: did.to_string(),
        service: vec![
            Service {
                id: format!("{}#p2p", did),
                service_type: "LibP2PNode".to_string(),
                service_endpoint: registration.multiaddr,
            }
        ],
        ..Default::default()
    })
}
```

## Endpoint Selection

When multiple storage locations are available, choose based on:

```rust
async fn select_best_endpoint(storage_dids: &[String]) -> Result<String> {
    let mut candidates = vec![];

    for did in storage_dids {
        if let Ok(doc) = resolve_did(did).await {
            if let Some(endpoint) = extract_blob_endpoint(&doc) {
                let latency = ping_endpoint(&endpoint).await.unwrap_or(Duration::MAX);
                candidates.push((endpoint, latency));
            }
        }
    }

    // Sort by latency, pick fastest
    candidates.sort_by_key(|(_, latency)| *latency);

    candidates.first()
        .map(|(endpoint, _)| endpoint.clone())
        .ok_or_else(|| anyhow!("No reachable storage endpoints"))
}
```

### Selection Criteria

| Factor | Weight | Notes |
|--------|--------|-------|
| Latency | High | Ping endpoint before selection |
| Region affinity | Medium | Prefer same region (from DID doc `elohim:region`) |
| Protocol | Low | Prefer HTTPS for web clients, libp2p for P2P |
| Trust tier | Context | Steward nodes may be preferred for sensitive content |

## Doorway Routes

### Serving DID Document

```rust
// doorway/src/routes/did.rs

async fn get_did_document(State(state): State<AppState>) -> impl IntoResponse {
    let doc = DIDDocument {
        context: vec![
            "https://www.w3.org/ns/did/v1".to_string(),
            "https://elohim-protocol.org/ns/v1".to_string(),
        ],
        id: state.config.did.clone(),
        verification_method: vec![
            VerificationMethod {
                id: format!("{}#signing-key", state.config.did),
                method_type: "Ed25519VerificationKey2020".to_string(),
                controller: state.config.did.clone(),
                public_key_multibase: encode_multibase(&state.signing_key.public()),
            }
        ],
        service: vec![
            Service {
                id: format!("{}#blobs", state.config.did),
                service_type: "ElohimBlobStore".to_string(),
                service_endpoint: format!("{}/api/v1/blobs", state.config.public_url),
            },
            Service {
                id: format!("{}#holochain", state.config.did),
                service_type: "HolochainGateway".to_string(),
                service_endpoint: format!("{}/app/{}", state.config.public_ws_url, state.config.app_port),
            },
        ],
        elohim_capabilities: state.config.capabilities.clone(),
        elohim_region: state.config.region.clone(),
    };

    Json(doc)
}

// Register route
fn did_routes() -> Router {
    Router::new()
        .route("/.well-known/did.json", get(get_did_document))
}
```

## Configuration

```toml
# doorway.toml

[did]
# The doorway's DID (derived from domain)
id = "did:web:doorway-a.elohim.host"

# Public URL for service endpoints
public_url = "https://doorway-a.elohim.host"
public_ws_url = "wss://doorway-a.elohim.host"

# Capabilities advertised in DID document
capabilities = ["blob-storage", "gateway", "seeding"]

# Region for affinity-based routing
region = "us-west-2"

[did.resolution]
# Cache TTL for resolved DID documents
cache_ttl_seconds = 300

# Timeout for DID resolution
timeout_seconds = 5

# Fallback resolvers (if direct resolution fails)
fallback_resolvers = [
    "https://resolver.elohim.host/1.0/identifiers/"
]
```

## Security Considerations

### DID Document Validation

```rust
fn validate_did_document(did: &str, doc: &DIDDocument) -> Result<()> {
    // 1. DID in document must match requested DID
    if doc.id != did {
        return Err(anyhow!("DID mismatch: requested {} but doc has {}", did, doc.id));
    }

    // 2. For did:web, verify TLS certificate matches domain
    //    (handled by HTTPS client)

    // 3. Verify at least one verification method exists
    if doc.verification_method.is_empty() {
        return Err(anyhow!("No verification methods in DID document"));
    }

    // 4. Optionally: verify signature if document is signed
    if let Some(proof) = &doc.proof {
        verify_did_document_proof(doc, proof)?;
    }

    Ok(())
}
```

### Trust Boundaries

| Scenario | Trust Level | Action |
|----------|-------------|--------|
| Fetching from known doorway (same operator) | High | Direct fetch |
| Fetching from federated doorway | Medium | Verify blob hash after fetch |
| Fetching from P2P storage node | Medium | Verify blob hash + check Holochain permissions |
| DID resolution failure | Low | Fall back to other storage_dids |

## Relationship to P2P Architecture

This DID federation layer sits **above** the elohim-storage P2P layer:

```
┌─────────────────────────────────────────────────────────────────────────┐
│   DID Federation (this document)                                       │
│   • Doorway discovery via did:web                                      │
│   • Content location via BlobManifest.storage_dids                     │
│   • Cross-doorway blob fetching                                        │
├─────────────────────────────────────────────────────────────────────────┤
│   elohim-storage P2P (see P2P-ARCHITECTURE.md)                         │
│   • Blob storage and replication                                       │
│   • RS shard distribution                                              │
│   • PeerId-based P2P networking                                        │
├─────────────────────────────────────────────────────────────────────────┤
│   Holochain DHT                                                        │
│   • BlobManifest entries                                               │
│   • Identity and permissions                                           │
│   • Provenance                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

- **Doorways** use DIDs for web-based discovery (`did:web`)
- **elohim-storage nodes** use PeerIds for P2P networking, but register DIDs (`did:key`) in Holochain for cross-topology discovery
- **Holochain** stores manifests that reference DIDs, enabling any client to locate content

## References

- [W3C DID Core Spec](https://www.w3.org/TR/did-core/)
- [did:web Method Spec](https://w3c-ccg.github.io/did-method-web/)
- [did:key Method Spec](https://w3c-ccg.github.io/did-method-key/)
- [P2P Architecture](../elohim-storage/P2P-ARCHITECTURE.md)
- [DID Enablement Plan](../../backlog/did-enablement.md)
