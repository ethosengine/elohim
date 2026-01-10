# Doorway Federation: Fediverse Patterns on P2P Infrastructure

> **See also**: [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) for the overall P2P architecture

## The Bridge Problem

Web 2.0 clients (browsers, mobile apps) need:
- REST APIs with familiar semantics
- WebSocket connections with standard auth
- Discoverable servers with DNS names
- CDN-like content delivery

Holochain provides:
- Agent-centric cryptographic identity
- Content-addressed immutable data
- Validation rules enforced by peers
- No central servers

**Doorway bridges these worlds** - but the bridge must not become a bottleneck.

---

## Fediverse Patterns, Holochain Guarantees

Doorway adopts federation patterns familiar from ActivityPub/Mastodon, but with a
fundamental difference in where authority lives:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      TRADITIONAL FEDIVERSE                               │
│                                                                          │
│   ┌──────────────┐         ┌──────────────┐         ┌──────────────┐   │
│   │  Instance A  │◄───────►│  Instance B  │◄───────►│  Instance C  │   │
│   │              │         │              │         │              │   │
│   │ AUTHORITATIVE│         │ AUTHORITATIVE│         │ AUTHORITATIVE│   │
│   │ for its users│         │ for its users│         │ for its users│   │
│   └──────────────┘         └──────────────┘         └──────────────┘   │
│                                                                          │
│   Users are "locked in" to their home instance.                          │
│   If instance goes down, users lose access to their data.               │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                      DOORWAY FEDERATION                                  │
│                                                                          │
│   ┌──────────────┐         ┌──────────────┐         ┌──────────────┐   │
│   │  Doorway A   │         │  Doorway B   │         │  Doorway C   │   │
│   │              │         │              │         │              │   │
│   │  PROJECTION  │         │  PROJECTION  │         │  PROJECTION  │   │
│   │  of DHT      │         │  of DHT      │         │  of DHT      │   │
│   └──────┬───────┘         └──────┬───────┘         └──────┬───────┘   │
│          │                        │                        │            │
│          └────────────────────────┼────────────────────────┘            │
│                                   │                                      │
│                          ┌────────▼────────┐                            │
│                          │  HOLOCHAIN DHT  │                            │
│                          │                 │                            │
│                          │  AUTHORITATIVE  │                            │
│                          │  Source of Truth│                            │
│                          └─────────────────┘                            │
│                                                                          │
│   Users can use ANY doorway. Data lives in the DHT.                     │
│   If a doorway goes down, users switch to another.                      │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## What Doorways Federate (and What They Don't)

### Federated: Operational Concerns

| Concern | How Doorways Handle It |
|---------|------------------------|
| **Discovery** | Doorways advertise via mDNS, registry APIs |
| **Health** | Cross-doorway health probing (`GET /health`) |
| **Caching** | Content-addressed data cached locally, shared invalidation |
| **Auth Tokens** | JWTs include `doorway_id` + `doorway_url` for cross-validation |
| **Blob Routing** | Custodian selection across doorway-served nodes |

### NOT Federated: Authority

| Concern | Why Doorways Can't Own It |
|---------|---------------------------|
| **Identity** | Agent keys live in Holochain, not doorway DBs |
| **Data** | Entries are validated by DNA rules, not doorway logic |
| **Permissions** | Reach levels enforced by zome code, doorways just cache |
| **History** | DHT is append-only log, doorways are read projections |

---

## The Decentralization Boundary

A Doorway is **operationally useful** but **architecturally replaceable**:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        DOORWAY RESPONSIBILITIES                          │
│                                                                          │
│   ✓ Translate HTTP/REST to Holochain zome calls                         │
│   ✓ Cache content-addressed responses for performance                   │
│   ✓ Route blob requests to healthy custodians                           │
│   ✓ Issue short-lived JWTs for session convenience                      │
│   ✓ Provide Web2-friendly APIs (OpenAPI, CORS, etc.)                    │
│   ✓ Health-check other doorways and custodians                          │
│                                                                          │
├─────────────────────────────────────────────────────────────────────────┤
│                        DOORWAY LIMITATIONS                               │
│                                                                          │
│   ✗ Cannot create entries without agent signature                       │
│   ✗ Cannot modify data that fails DNA validation                        │
│   ✗ Cannot prevent users from switching to another doorway              │
│   ✗ Cannot access private data without agent authorization              │
│   ✗ Cannot forge identities (Ed25519 signatures required)               │
│   ✗ Cannot censor content that other doorways will serve                │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### The Escape Hatch

If a doorway misbehaves, users have options:

1. **Switch doorways** - Any doorway can serve any content from the DHT
2. **Run their own** - Doorway is open source, runs anywhere
3. **Go direct** - Advanced users can connect to Holochain directly
4. **Verify independently** - All data is content-addressed and signed

---

## Federation Protocol

### Cross-Doorway Authentication

JWTs include issuer identity for federation:

```json
{
  "human_id": "uhCAk...",
  "agent_pub_key": "uhCAk...",
  "identifier": "alice@example.com",
  "permission_level": "authenticated",
  "doorway_id": "alpha-elohim-host",
  "doorway_url": "https://alpha.elohim.host",
  "iat": 1703520000,
  "exp": 1703606400
}
```

When Doorway B receives a token issued by Doorway A:

1. Extract `doorway_url` from claims
2. Fetch `{doorway_url}/.well-known/doorway-keys` (cached)
3. Verify token signature against issuer's public key
4. Trust the `agent_pub_key` claim for Holochain operations

### Custodian Health Federation

Doorways share custodian health information:

```
Doorway A                                    Doorway B
    │                                            │
    │──── GET /health ──────────────────────────►│
    │◄─── { online: true, accepting: true } ─────│
    │                                            │
    │  (A now knows B is healthy and can         │
    │   route blob requests to B's custodians)   │
```

### Content Routing

When a client requests content:

```
1. Client ──► Doorway A: GET /api/blob/{hash}
2. Doorway A checks local cache → miss
3. Doorway A queries custodian registry (from DHT projection)
4. Doorway A probes Doorway B, C, D for health
5. Doorway A selects best custodian based on:
   - Latency (measured via health probe)
   - Bandwidth (self-reported)
   - Reach level (content access rules)
6. Doorway A fetches from selected custodian
7. Doorway A caches locally + returns to client
```

---

## Why This Scales Better Than Fediverse

### 1. No Home Instance Lock-In

**Fediverse problem**: Users must maintain relationship with home instance.
If `mastodon.social` goes down, those users are offline.

**Doorway solution**: Users can authenticate through any doorway.
Their agent key is their identity, not their doorway.

### 2. No Instance-to-Instance Replication

**Fediverse problem**: Each instance must replicate relevant content
from every other instance it federates with. O(n²) connections.

**Doorway solution**: All doorways project the same DHT. Content appears
everywhere automatically via Holochain gossip. Doorways just cache.

### 3. Content-Addressed Everything

**Fediverse problem**: URLs are instance-relative (`@alice@mastodon.social`).
Content identity tied to hosting instance.

**Doorway solution**: Content is addressed by hash. Same content has same
address regardless of which doorway serves it. CDN-like caching is trivial.

### 4. Validation at the Edge

**Fediverse problem**: Instances trust each other's content claims.
Malicious instance can send garbage.

**Doorway solution**: DNA validation rules run on every node. Invalid
content is rejected by the DHT itself, not just by doorways.

---

## Deployment Topologies

### Single Doorway (Development)

```
┌─────────────┐      ┌─────────────┐
│   Browser   │─────►│   Doorway   │─────► Holochain
└─────────────┘      └─────────────┘
```

### Regional Doorways (Production)

```
                    ┌─────────────────┐
         ┌────────►│  us-west.door   │────┐
         │         └─────────────────┘    │
         │                                │
┌────────┴───────┐ ┌─────────────────┐   ▼
│   GeoDNS /     │►│  eu-central.door│──► DHT
│   Load Balancer│ └─────────────────┘   ▲
└────────┬───────┘                        │
         │         ┌─────────────────┐    │
         └────────►│  ap-south.door  │────┘
                   └─────────────────┘
```

### Federated Community Doorways

```
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
│ learning.coop   │   │ bioregion.local │   │ elohim.host     │
│                 │   │                 │   │                 │
│ (education      │◄─►│ (local-first    │◄─►│ (global         │
│  community)     │   │  community)     │   │  commons)       │
└────────┬────────┘   └────────┬────────┘   └────────┬────────┘
         │                     │                     │
         └─────────────────────┼─────────────────────┘
                               │
                        ┌──────▼──────┐
                        │  Shared DHT │
                        └─────────────┘
```

---

## Doorway Discovery via DIDs

Doorways use W3C Decentralized Identifiers (DIDs) to discover each other and locate content across the network.

### The Content Location Problem

When a doorway receives a request for content it doesn't have locally:

```
Browser → Doorway A → "GET /api/v1/blobs/sha256-abc123..."
                    → Local cache? MISS
                    → Local Holochain? Has manifest, but not the blob
                    → Where are the actual bytes?
```

The Holochain manifest knows *who owns* the content, but not *where to fetch* the bytes.

### Solution: DIDs as Location Pointers

Holochain manifests include `storage_dids` - a list of DIDs that have the blob:

```rust
#[hdk_entry_helper]
pub struct BlobManifest {
    pub hash: String,                    // Content hash
    pub owner: AgentPubKey,              // Who created it
    pub size_bytes: u64,
    pub content_type: Option<String>,
    pub storage_dids: Vec<String>,       // Who has the bytes
    pub created_at: Timestamp,
}
```

DIDs resolve to service endpoints:

| DID | Resolves To |
|-----|-------------|
| `did:web:doorway-a.elohim.host` | HTTPS fetch `/.well-known/did.json` → service endpoints |
| `did:key:z6Mk...` | Decode ed25519 pubkey → lookup in P2P DHT |

### DID Document

Each doorway serves its DID Document at `/.well-known/did.json`:

```json
{
  "@context": ["https://www.w3.org/ns/did/v1"],
  "id": "did:web:doorway-a.elohim.host",
  "verificationMethod": [{
    "id": "did:web:doorway-a.elohim.host#signing-key",
    "type": "Ed25519VerificationKey2020",
    "publicKeyMultibase": "z6Mkq..."
  }],
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
    }
  ],
  "elohim:capabilities": ["blob-storage", "gateway", "seeding"],
  "elohim:region": "us-west-2"
}
```

### Federated Content Fetch

```
1. Browser requests blob from Doorway A
2. Doorway A: cache miss
3. Doorway A queries Holochain for BlobManifest
4. Manifest.storage_dids = ["did:web:doorway-b...", "did:key:z6..."]
5. Doorway A resolves did:web:doorway-b...
   → GET https://doorway-b.elohim.host/.well-known/did.json
   → Extract ElohimBlobStore endpoint
6. Doorway A fetches blob from Doorway B
7. Doorway A caches locally + returns to browser
```

### Endpoint Selection

When multiple storage locations are available, select based on:

| Factor | Weight | Notes |
|--------|--------|-------|
| Latency | High | Ping endpoint before selection |
| Region affinity | Medium | Prefer same region (from `elohim:region`) |
| Protocol | Low | Prefer HTTPS for web clients, libp2p for P2P |
| Trust tier | Context | Steward nodes preferred for sensitive content |

---

## Configuration

Doorway identity for federation is configured via CLI/environment:

```bash
doorway \
  --doorway-id "alpha-elohim-host" \
  --doorway-url "https://alpha.elohim.host" \
  --installed-app-id "elohim" \
  --conductor-url "ws://localhost:4444"
```

Or via environment:

```bash
DOORWAY_ID=alpha-elohim-host
DOORWAY_URL=https://alpha.elohim.host
INSTALLED_APP_ID=elohim
CONDUCTOR_URL=ws://localhost:4444
```

---

## Doorways as Content: Self-Validating Infrastructure

The final piece that closes the loop: **doorways themselves are entries in the DHT**.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    DOORWAY REGISTRATION ENTRY                            │
│                                                                          │
│   {                                                                      │
│     "id": "alpha-elohim-host",                                          │
│     "url": "https://alpha.elohim.host",                                 │
│     "operator_agent": "uhCAk...",      // Who runs this doorway         │
│     "operator_human": "uhCEk...",      // Link to Human entry           │
│     "capabilities": {                                                    │
│       "bootstrap": true,                                                 │
│       "signal": true,                                                    │
│       "gateway": true,                                                   │
│       "projection": true                                                 │
│     },                                                                   │
│     "reach": "commons",                // What reach level it serves    │
│     "region": "us-west",               // Geographic locality           │
│     "bandwidth_mbps": 1000,                                             │
│     "registered_at": "2024-01-01T00:00:00Z",                           │
│     "signature": "..."                 // Operator's Ed25519 sig        │
│   }                                                                      │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Everything Connects

The same validation rules that govern content govern infrastructure:

| Entity | Entry Type | Validated By | Trust Signal |
|--------|------------|--------------|--------------|
| Learning content | `Content` | DNA rules | Author reputation, reviews |
| Learning paths | `Path` | DNA rules | Completion rates, ratings |
| Humans | `Human` | DNA rules | Network position, contributions |
| **Doorways** | `Doorway` | DNA rules | Uptime, throughput, operator rep |

### Implications

1. **Decentralized Discovery**
   - No central doorway registry needed
   - Query the DHT: "give me doorways serving `reach: commons` in `region: eu-central`"
   - Same gossip protocol that spreads content spreads infrastructure info

2. **Operator Accountability**
   - Doorway entry is signed by operator's agent key
   - Operator has a `Human` entry with reputation
   - Misbehaving doorways affect operator's standing in the network

3. **Self-Healing Network**
   - Health probes update doorway status in projections
   - Unhealthy doorways naturally deprioritized
   - New doorways discovered automatically via DHT

4. **Reach-Aware Routing**
   - Local content → prefer local doorways
   - Commons content → any doorway
   - Doorway's declared reach affects what it's asked to serve

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         THE COMPLETE PICTURE                             │
│                                                                          │
│                        ┌─────────────────┐                              │
│                        │   Web2 Client   │                              │
│                        └────────┬────────┘                              │
│                                 │                                        │
│                                 ▼                                        │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                         DOORWAY                                  │   │
│   │                                                                  │   │
│   │   Registered in DHT ◄──────────────────────┐                    │   │
│   │   as Doorway entry                         │                    │   │
│   │                                            │                    │   │
│   └──────────────────────┬─────────────────────┼────────────────────┘   │
│                          │                     │                        │
│                          ▼                     │                        │
│   ┌──────────────────────────────────────────────────────────────────┐  │
│   │                       HOLOCHAIN DHT                              │  │
│   │                                                                  │  │
│   │   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │  │
│   │   │ Content  │  │  Paths   │  │  Humans  │  │ Doorways │       │  │
│   │   │ entries  │  │ entries  │  │ entries  │  │ entries  │       │  │
│   │   └──────────┘  └──────────┘  └──────────┘  └──────────┘       │  │
│   │                                                                  │  │
│   │   All validated by the same DNA rules.                          │  │
│   │   All signed by agent keys.                                     │  │
│   │   All subject to reach/trust mechanics.                         │  │
│   │                                                                  │  │
│   └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
│   The infrastructure that serves the network IS the network.            │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## P2P Bootstrap Role

Doorway serves a critical role in bootstrapping the native P2P network. The existing signal server for Holochain extends naturally to support libp2p peer discovery.

### The Bootstrap Problem

Native nodes need to find each other before they can sync:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      THE BOOTSTRAP PROBLEM                               │
│                                                                          │
│   Native Node A                                Native Node B             │
│   ┌──────────────────┐                        ┌──────────────────┐      │
│   │                  │                        │                  │      │
│   │  "I want to join │         ???            │  "I'm part of    │      │
│   │   the network"   │ ─────────────────────► │   the network"   │      │
│   │                  │                        │                  │      │
│   │  But how do I    │                        │                  │      │
│   │  find anyone?    │                        │                  │      │
│   │                  │                        │                  │      │
│   └──────────────────┘                        └──────────────────┘      │
│                                                                          │
│   Without a known starting point, P2P networks can't form.              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Solution: Signal Server as P2P Bootstrap

Doorway already provides `/signal/{pubkey}` for WebRTC signaling in Holochain networks. This extends naturally to libp2p peer exchange:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     SIGNAL SERVER BOOTSTRAP                              │
│                                                                          │
│   Native Node A                Doorway                 Native Node B     │
│   ┌────────────────┐      ┌────────────────┐      ┌────────────────┐    │
│   │                │      │                │      │                │    │
│   │  1. Connect    │─────►│  Signal Server │◄─────│  1. Connect    │    │
│   │     to signal  │      │                │      │     to signal  │    │
│   │                │      │  /signal/:id   │      │                │    │
│   │  2. Announce   │─────►│                │      │                │    │
│   │     peer_id    │      │  Maintains     │◄─────│  2. Announce   │    │
│   │     multiaddrs │      │  peer registry │      │     peer_id    │    │
│   │                │      │                │      │     multiaddrs │    │
│   │  3. Request    │─────►│                │      │                │    │
│   │     peer list  │      │  Returns       │      │                │    │
│   │                │◄─────│  active peers  │      │                │    │
│   │                │      │                │      │                │    │
│   │  4. Direct P2P │      │  (step aside)  │      │                │    │
│   │     connection │──────────────────────────────│  4. Direct P2P │    │
│   │                │      │                │      │     connection │    │
│   └────────────────┘      └────────────────┘      └────────────────┘    │
│                                                                          │
│   Once nodes find each other, they connect directly.                    │
│   Doorway only helps with initial discovery.                            │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Protocol Extension

The existing signal endpoint extends to support libp2p:

```
Existing (Holochain WebRTC):
  POST /signal/{agent_pubkey}
  Body: WebRTC signaling message

Extended (libp2p bootstrap):
  POST /signal/p2p/announce
  Body: {
    "peer_id": "12D3KooW...",
    "multiaddrs": [
      "/ip4/192.168.1.5/tcp/4001",
      "/ip4/98.42.23.1/tcp/4001"
    ],
    "protocols": ["/elohim/sync/1.0.0", "/elohim/shard/1.0.0"],
    "capabilities": ["storage", "sync"]
  }

  GET /signal/p2p/peers
  Response: {
    "peers": [
      {
        "peer_id": "12D3KooW...",
        "multiaddrs": [...],
        "last_seen": "2025-01-04T12:00:00Z"
      }
    ]
  }
```

### Bootstrap Flow

```
1. Native node starts
2. Node contacts well-known doorway signal server
3. Announces own peer_id and multiaddrs
4. Receives list of active peers
5. Connects directly to peers via libp2p
6. Joins P2P sync network
7. Periodically re-announces to signal server (heartbeat)
```

### NAT Traversal

For nodes behind NAT, doorway can facilitate hole-punching:

| Situation | Solution |
|-----------|----------|
| Both nodes have public IPs | Direct connection |
| One node behind NAT | STUN-style hole punch |
| Both behind NAT | Relay through doorway (temporary) |
| Symmetric NAT | Use doorway as relay until direct path found |

The goal is always to establish a direct P2P connection. Doorway relaying is a fallback, not the default.

### Multiple Bootstrap Nodes

For resilience, native nodes can use multiple doorways as bootstrap:

```rust
const BOOTSTRAP_NODES: &[&str] = &[
    "https://doorway.elohim.host/signal/p2p",
    "https://doorway-eu.elohim.host/signal/p2p",
    "https://doorway-asia.elohim.host/signal/p2p",
];
```

Any doorway can bootstrap any native node. No lock-in.

### Relationship to DHT

Once bootstrapped, native nodes can discover each other through:

1. **Signal server** - Quick bootstrap, always works
2. **Kademlia DHT** - Decentralized, works without doorway
3. **mDNS** - Local network discovery (same WiFi)

The signal server is the training wheels. As the network matures, DHT and mDNS take over.

---

## Summary

| Aspect | Traditional Fediverse | Doorway Federation |
|--------|----------------------|-------------------|
| Authority | Instances own user data | DHT owns all data |
| Lock-in | Users tied to home instance | People protected and served by doorways |
| Replication | Instance-to-instance | DHT gossip (automatic) |
| Validation | Trust between instances | Cryptographic, edge-enforced |
| Identity | Instance-relative (`@user@host`) | Agent keys (portable) |
| Scaling | O(n²) federation links | O(1) DHT participation |
| Censorship resistance | Switch instances (lose history) | Switch doorways (keep everything) |

**Doorway brings Fediverse UX to truly decentralized infrastructure.**
