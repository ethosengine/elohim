# Doorway Reach Enforcement

Doorway is the web gateway to the P2P network. Its role in reach enforcement is to gate HTTP/WebSocket requests from:

1. **Hosted humans** - Users with accounts on elohim.host (Stage 2 sovereignty)
2. **Public web visitors** - Anonymous browsers accessing commons content
3. **Federated doorways** - Other doorways fetching content on behalf of their users

For the system-wide reach concept, see [../REACH.md](../REACH.md).

## Doorway's Enforcement Rules

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     DOORWAY REACH GATING                                 │
│                                                                          │
│   Request arrives at Doorway                                            │
│        │                                                                 │
│        ▼                                                                 │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │  Extract requester context:                                      │   │
│   │  • Authorization header → agent_id, authenticated=true           │   │
│   │  • No header → anonymous, authenticated=false                    │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│        │                                                                 │
│        ▼                                                                 │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │  Fetch content (cache or conductor)                              │   │
│   │  Extract reach field from response                               │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│        │                                                                 │
│        ▼                                                                 │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │  Apply reach rules:                                              │   │
│   │                                                                  │   │
│   │  commons     → Allow everyone                                    │   │
│   │  regional    → Require authentication                            │   │
│   │  bioregional → Require authentication                            │   │
│   │  municipal   → Require authentication                            │   │
│   │  neighborhood→ Require authentication                            │   │
│   │  local       → Require authentication + relationship check       │   │
│   │  invited     → Require authentication + explicit invite          │   │
│   │  private     → Require authentication + beneficiary match        │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│        │                                                                 │
│        ▼                                                                 │
│   Allow (200) or Deny (403)                                             │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Access Matrix

| Reach Level | Anonymous | Authenticated | Beneficiary | Custodian |
|-------------|-----------|---------------|-------------|-----------|
| commons | ✅ | ✅ | ✅ | ✅ |
| regional | ❌ | ✅ | ✅ | ✅ |
| bioregional | ❌ | ✅ | ✅ | ✅ |
| municipal | ❌ | ✅ | ✅ | ✅ |
| neighborhood | ❌ | ✅ | ✅ | ✅ |
| local | ❌ | ⚠️ relationship | ✅ | ✅ |
| invited | ❌ | ⚠️ invite list | ✅ | ✅ |
| private | ❌ | ❌ | ✅ | ⚠️ custody |

## API Behavior

### Request Headers

```http
GET /api/v1/cache/Content/my-content-id
Authorization: Bearer <agent-pubkey>   # Optional - provides authentication
```

### Response Headers

```http
HTTP/1.1 200 OK
X-Reach: commons
X-Cache: HIT
Cache-Control: public, max-age=300
ETag: "sha256-abc123"
```

### Denied Access

```http
HTTP/1.1 403 Forbidden
Content-Type: application/json

{
  "error": "Content exists but is not accessible to you",
  "code": "REACH_DENIED"
}
```

## Reach-Aware Caching

Doorway caches content with reach-aware keys to prevent leakage:

```
Cache Key: {dna}:{type}:{id}:{reach}:{requester_class}

Examples:
  elohim:Content:manifesto:commons:anonymous     → Cached for anonymous users
  elohim:Content:manifesto:commons:authenticated → Cached for authenticated users
  elohim:Content:diary-001:private:alice         → Cached only for Alice
```

### Why Separate Cache Entries?

Same content might be served differently based on requester:
- Public view might omit sensitive fields
- Owner view includes everything
- Custodian view includes audit trail

Separate cache keys prevent serving the wrong view.

### Cache Configuration

```bash
CACHE_CONTENT_TTL_SECS=3600      # 1 hour for content
CACHE_LIST_TTL_SECS=300          # 5 minutes for lists
CACHE_MAX_ENTRIES=10000          # Max cache entries
```

## DNA Integration

### Content Must Include Reach Field

DNAs must include `reach` in their responses for doorway to enforce:

```json
{
  "id": "content-123",
  "title": "My Content",
  "reach": "commons",
  "beneficiary": "uhCAk..."
}
```

If `reach` is missing, doorway defaults to requiring authentication (safe default).

### Cache Rules Declaration

DNAs can declare caching policy via `__doorway_cache_rules`:

```rust
#[hdk_extern]
fn __doorway_cache_rules(_: ()) -> ExternResult<Vec<CacheRule>> {
    Ok(vec![
        CacheRule {
            function: "get_content".to_string(),
            public: false,                           // Requires auth by default
            reach_field: Some("reach".to_string()),  // Read reach from this field
            ttl_secs: Some(3600),
        },
    ])
}
```

## Serving Hosted Humans

Hosted humans (Stage 2) authenticate via JWT from their doorway:

```json
{
  "human_id": "uhCEk...",
  "agent_pub_key": "uhCAk...",
  "doorway_id": "alpha-elohim-host",
  "permission_level": "authenticated"
}
```

Doorway extracts `agent_pub_key` and uses it for:
1. Reach enforcement (is this agent the beneficiary?)
2. Relationship lookups (does this agent have local/invited access?)
3. Custody checks (is this agent a custodian?)

## Serving the Public Web

Anonymous requests (no Authorization header) can only access `commons` content:

```bash
# Works - commons content
curl https://doorway.elohim.host/api/v1/cache/Content/manifesto

# Fails - private content
curl https://doorway.elohim.host/api/v1/cache/Content/alice-diary
# 403 Forbidden
```

This is how the public web sees Elohim: only commons content is visible without authentication.

## Troubleshooting

### "Content exists but is not accessible to you" (403)

1. Check if you're authenticated: include `Authorization: Bearer <agent-id>`
2. Check the content's reach level
3. For private content, verify you're the beneficiary
4. For local/invited, verify your relationship

### Cache Shows Wrong Reach

1. Wait for TTL expiration (max 1 hour)
2. Content reach may have changed - cache still has old value
3. Use `?_conductor=true` in dev mode to bypass cache

### Commons Content Returning 403

1. Verify response includes `"reach": "commons"` (case-sensitive)
2. Check DNA is emitting reach field
3. Verify doorway cache rules are configured

## Related Documentation

- [../REACH.md](../REACH.md) - System-wide reach concept
- [FEDERATION.md](./FEDERATION.md) - Cross-doorway authentication
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Doorway internals
