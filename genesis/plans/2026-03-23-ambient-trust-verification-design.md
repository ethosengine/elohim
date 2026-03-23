# Ambient Trust Verification — Per-Connection DHT-Verified Authorization

**Date:** 2026-03-23
**Status:** Approved
**Scope:** Close the gap between per-request SQLite auth and the vision's per-connection DHT-verified ambient trust model. Covers verification zome functions, connection handshake protocol, per-connection context cache, and graceful degradation.

## The Gap

The distributed trust design (2026-03-23) describes a four-layer access model where reach is "ambient in the connection context" — peers negotiate trust once on connection, and EPR Heads flow freely within the verified ceiling. The current implementation checks reach per-request against local SQLite, with no DHT verification and no connection-level caching. This is "server auth that happens to run on every peer," not distributed trust.

The bridge exists — `hc_client.rs` already makes signed zome calls to the conductor via WebSocket. The conductor participates in the shared DHT. The gap is application-level: no verification zome function, no handshake protocol, no context cache.

## Design

### Layer A: Low-Hanging Fruit (Existing Patterns)

These close obvious holes using patterns already established in the codebase.

**A1. Attestation gate on P2P path.** Mirror the HTTP attestation check in `handle_epr_request`. After reach auth passes, query `content_attestations` for prerequisite types, check requester's credentials. Return `AccessDenied` if missing. Same logic, same DB functions, different handler.

**A2. Policy enforcement initialization.** In `main.rs`, when the content DB is initialized and policy tables exist, create a `PolicyEnforcement` instance and pass it to both `HttpServer` and `P2PNode` via their existing `with_policy_enforcement()` builders.

**A3. EPR Head attestation requirements.** Add `attestation_requirements` to `EprQahalContext`:

```rust
pub struct EprQahalContext {
    pub reach: Option<String>,
    pub layer: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attestation_requirements: Vec<String>,
}
```

Format: `"type:reference"` — compact for the ~500B EPR Head. Types include:
- `prerequisite-mastery:content-id` — learning progression gate
- `consent:requirement-id` — governance consent gate
- `payment:tier-id` — economic access gate (pragmatic necessity)
- `age-verification:threshold` — safety gate
- `community-endorsement:collective-id` — collective endorsement gate

This tells receiving peers what credentials they'll need before fetching the content body. Discovery (EPR Heads) still flows freely — requirements are metadata, not a gate at Layer 1.

### Layer B: Three-Pillar Verification via Conductor

Verification maps to the three coupled pillars, not to DNA boundaries. When a peer presents credentials, the serving peer asks one question through three lenses: "Is this person who they claim, with the relationships they claim, carrying the standing they claim?"

**Verification zome functions.** Each DNA that holds credential entries exposes a coordinator function:

```rust
#[hdk_extern]
fn verify_credentials(hashes: Vec<ActionHash>) -> ExternResult<Vec<CredentialVerification>>
```

Under the hood this calls `get(hash, GetOptions)` for each entry on the DHT. Returns: entry exists, revoked status, entry type, creating agent. Lightweight — Holochain caches DHT entries locally after first fetch. Subsequent verifications of the same CID are conductor-side cache hits.

**Storage verification module.** New `src/trust_verification.rs` wraps `hc_client.rs`:

```rust
pub struct TrustCredentials {
    pub agent_pubkey: String,
    pub membership_cids: Vec<String>,     // qahal: collective standing
    pub relationship_cids: Vec<String>,   // qahal: interpersonal trust
    pub attestation_cids: Vec<String>,    // any pillar: mastery, consent, payment
    pub stewardship_cids: Vec<String>,    // shefa: value commitment
}

pub struct VerifiedTrustContext {
    pub agent_verified: bool,
    pub reach_ceiling: String,
    pub verified_memberships: Vec<VerifiedMembership>,
    pub verified_relationships: Vec<VerifiedRelationship>,
    pub verified_attestations: Vec<VerifiedAttestation>,
    pub verified_stewardship: Vec<VerifiedStewardship>,
    pub verified_at: Instant,
    pub ttl: Duration,
}

pub async fn verify_trust_context(
    hc_client: &HcClient,
    credentials: &TrustCredentials,
) -> Result<VerifiedTrustContext, StorageError>
```

Storage doesn't care which DNA answered. The verification module routes CIDs to the correct DNA internally. The three-pillar coupling is maintained conceptually — one verification, three dimensions.

### Layer C: Connection Handshake Protocol + Context Cache

**New protocol: `/elohim/trust/1.0.0`**

Same wire format as EPR and shard protocols — 4-byte BE length prefix + MessagePack body. Request-response, one exchange per connection.

**The exchange:**

1. `ConnectionEstablished` event fires
2. Connecting peer sends `TrustHandshake`: agent pubkey + credential CIDs (membership, relationship, attestation, stewardship)
3. Receiving peer calls conductor: `verify_trust_context(credentials)`
4. Conductor checks DHT for each CID (~200ms one-time)
5. Receiving peer sends `TrustResponse`: verified reach ceiling + TTL
6. Both peers cache: `PeerId → PeerTrustContext`

**Per-connection context cache:**

```rust
struct PeerTrustContext {
    agent_pubkey: String,
    reach_ceiling: String,
    verified_memberships: Vec<VerifiedMembership>,
    verified_relationships: Vec<VerifiedRelationship>,
    verified_attestations: Vec<VerifiedAttestation>,
    verified_stewardship: Vec<VerifiedStewardship>,
    verified_at: Instant,
    ttl: Duration,
}

// Keyed by libp2p PeerId
peer_trust_cache: Arc<RwLock<HashMap<PeerId, PeerTrustContext>>>
```

Cache lifecycle:
- `ConnectionEstablished` → trigger handshake, populate on response
- EPR request → read cache for fast-path reach check
- TTL expires → evict; next request triggers re-handshake
- `ConnectionClosed` → remove entry
- Peer sends updated credentials → replace entry (re-verify)

**Reach ceiling calculation.** The ceiling is the highest reach tier this peer qualifies for given their verified credentials:

- `commons`/`public` → always (no credentials needed)
- `community` → has any consented membership
- `familiar` → has membership in collective that shares a steward (content-specific — requires steward lookup per request)
- `trusted` → has relationship at intimacy >= trusted (content-specific — steward match per request)
- `intimate` → has mutual intimate relationship (content-specific — steward match per request)
- `self`/`private` → agent_pubkey matches creator (always per-request, can't be ambient)

`familiar` through `private` require a content-specific steward check. The cache eliminates the relationship/membership DB lookups — only the steward allocation lookup remains (one indexed query).

**Fast-path authorization after handshake:**

```
EPR request arrives
  → cache lookup: peer_trust_cache.get(peer_id)          [memory]
  → ceiling check: cached_ceiling >= content_reach?       [comparison]
  → content-specific: cached credentials vs steward IDs   [one DB query]
  → serve EPR Head
```

From 5+ DB hits per request → 2 with warm cache. From local SQLite trust → DHT-verified trust.

### Graceful Degradation

```
Full trust mode:     handshake → conductor verification → DHT authority → cached context
Conductor offline:   handshake fails → per-request SQLite (current behavior, fallback only)
Doorway (no P2P):    serves commons content without auth (already validated)
                     warms projection cache for downstream peers
Doorway auth users:  conductor spun up on login (computationally expensive, on their behalf)
                     → full trust mode through web2 path
```

The SQLite fallback is NOT "trust mode lite" — it is specifically for:
1. Commons content via doorway (already validated, no auth needed)
2. Bootstrap path until the user's conductor is available

Authenticated doorway users eventually get their own conductor spun up by doorway on login, which transitions them into full trust mode. The fallback is temporary, not a permanent bypass.

### Revocation

No separate revocation mechanism needed. DHT gossip (200-2000ms) updates the conductor's local view. Next trust verification — whether via re-handshake on TTL expiry or per-request fallback — reflects the revoked state. Worst case: stale cache for TTL duration (configurable, default 1 hour). For safety-critical revocation, a future governance signal protocol can trigger immediate re-verification.

## Files Changed

| File | Layer | What |
|------|-------|------|
| `epr_codec.rs` | A | Add `attestation_requirements` to EprQahalContext |
| `p2p/mod.rs` | A | Attestation gate + policy in EPR handler |
| `main.rs` | A | Wire policy enforcement to P2P node |
| `p2p/trust_protocol.rs` | C | **NEW**: Trust handshake codec (/elohim/trust/1.0.0) |
| `trust_verification.rs` | B | **NEW**: Three-pillar verification via hc_client |
| `p2p/trust_cache.rs` | C | **NEW**: PeerTrustContext cache with TTL |
| `p2p/mod.rs` | B+C | Handshake handler + fast-path auth in check_reach_authorization |
| DNA coordinator zomes | B | **NEW**: verify_credentials() in imagodei + mishpat |

No new tables. No new DHT entry types. Verification uses existing `get()` on existing entry types. Cache is in-memory (rebuilt on restart via re-handshake).

## Execution Order

```
Layer A: Ship now (existing patterns, no design risk)
  A1: Attestation gate on P2P
  A2: Policy enforcement wiring
  A3: EPR Head attestation_requirements field

Layer B: Verification zome + storage module
  B1: verify_credentials() zome function in imagodei coordinator
  B2: verify_credentials() zome function in mishpat coordinator
  B3: trust_verification.rs module in storage (calls conductor via hc_client)

Layer C: Connection handshake + cache (depends on B)
  C1: trust_protocol.rs — TrustHandshake/TrustResponse codec
  C2: trust_cache.rs — PeerTrustContext cache with TTL
  C3: Wire into P2P event loop (ConnectionEstablished → handshake)
  C4: Fast-path in check_reach_authorization (cache → fallback)
```

Layer A is independent. B and C are designed as a unit but implemented incrementally: B first (verification works, called per-request via conductor), then C (handshake makes it ambient and fast).
