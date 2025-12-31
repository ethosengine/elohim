# Doorway Federation: Fediverse Patterns on P2P Infrastructure

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
