# Reach-Aware Caching in Doorway

## Overview

Doorway implements a **reach-aware caching system** that gates content access by geographic and social proximity, preventing **information colonization** while enabling efficient distributed content delivery.

This system is integrated with the Holochain `CustodianCommitment` model to enable organic account protection through social relationships.

## Reach Levels

Content is classified into 8 reach levels, ordered from most private to most public:

| Level | Access | Use Cases |
|-------|--------|-----------|
| **private** | Only the beneficiary (content owner) | Personal data, medical records, diaries |
| **invited** | Explicitly invited individuals | Shared journals, collaborative spaces |
| **local** | Family/household members | Household decisions, chore schedules |
| **neighborhood** | Geographic community (block/neighborhood) | Local events, neighborhood news |
| **municipal** | City/town level | Public transit info, municipal services |
| **bioregional** | Watershed/ecosystem level | Environmental data, regional resources |
| **regional** | State/province level | Regional news, state resources |
| **commons** | Global/public (everyone) | Public knowledge, open resources |

## API Endpoints

### GET /api/v1/{dna_hash}/{zome}/{fn}

Query cached Holochain zome functions. The Doorway API gates responses based on the requester's authentication and the content's reach level.

#### Query Parameters

- Standard zome function arguments (passed to the zome function)
- `_conductor=true` - Bypass projection cache and read directly from conductor (DEV_MODE only)

#### Authorization

- **No Authorization Header**: Requester is treated as anonymous
  - Can access `commons` reach only
  - Cannot access `private`, `local`, `neighborhood`, `municipal`, `bioregional`, or `regional` content

- **With Authorization Header** (format: `Bearer <agent-id>`): Requester is authenticated
  - Can access `commons`, `neighborhood`, `municipal`, `bioregional`, `regional`, and `invited` content
  - Can access `private` content only if `agent_id` matches the content beneficiary

#### Response Headers

The API includes additional headers for reach-aware responses:

```
X-Reach: commons|private|local|neighborhood|municipal|bioregional|regional|invited
X-Cache: HIT|MISS
Cache-Control: public, max-age=300, stale-while-revalidate=60
ETag: "hash"
```

## Content Format

Content responses must include an optional `reach` field for reach-aware gating:

```json
{
  "id": "content-id",
  "title": "Content Title",
  "reach": "commons",
  "data": {...}
}
```

Or as an array:

```json
[
  {"id": "item1", "reach": "commons", "data": {...}},
  {"id": "item2", "reach": "local", "data": {...}}
]
```

If the `reach` field is absent, content is served based on projection cache availability with no reach-level gating.

## Examples

### Example 1: Public Content (Commons)

**Request:**
```bash
curl -X GET "http://localhost:3000/api/v1/uhC0kabc/content_store/get_content?id=news-001"
```

**Response:**
```http
HTTP/1.1 200 OK
X-Reach: commons
X-Cache: MISS
Cache-Control: public, max-age=300, stale-while-revalidate=60

{
  "id": "news-001",
  "title": "Breaking News",
  "reach": "commons",
  "content": "..."
}
```

**Accessible to:** Everyone (authenticated or not)

### Example 2: Private Content - Owner Access

**Request:**
```bash
curl -X GET "http://localhost:3000/api/v1/uhC0kabc/content_store/get_content?id=diary-001" \
  -H "Authorization: Bearer alice-agent-id"
```

**Response:**
```http
HTTP/1.1 200 OK
X-Reach: private
X-Cache: MISS
Cache-Control: public, max-age=300, stale-while-revalidate=60

{
  "id": "diary-001",
  "title": "My Personal Thoughts",
  "reach": "private",
  "beneficiary": "alice-agent-id",
  "content": "..."
}
```

**Accessible to:** Alice only (the content beneficiary)

### Example 3: Private Content - Non-Owner Denial

**Request:**
```bash
curl -X GET "http://localhost:3000/api/v1/uhC0kabc/content_store/get_content?id=diary-001" \
  -H "Authorization: Bearer bob-agent-id"
```

**Response:**
```http
HTTP/1.1 403 Forbidden
Content-Type: application/json

{
  "error": "Content exists but is not accessible to you",
  "code": "REACH_DENIED"
}
```

**Not accessible to:** Bob (not the beneficiary)

### Example 4: Local/Neighborhood Content - Authenticated Access

**Request:**
```bash
curl -X GET "http://localhost:3000/api/v1/uhC0kabc/content_store/get_content?id=neighborhood-event-001" \
  -H "Authorization: Bearer bob-agent-id"
```

**Response:**
```http
HTTP/1.1 200 OK
X-Reach: neighborhood
X-Cache: HIT
Cache-Control: public, max-age=300, stale-while-revalidate=60

{
  "id": "neighborhood-event-001",
  "title": "Community Garden Meeting",
  "reach": "neighborhood",
  "location": "37.7749,-122.4194",
  "content": "..."
}
```

**Accessible to:** Any authenticated user (regardless of location)

### Example 5: Local/Neighborhood Content - Unauthenticated Denial

**Request:**
```bash
curl -X GET "http://localhost:3000/api/v1/uhC0kabc/content_store/get_content?id=neighborhood-event-001"
```

**Response:**
```http
HTTP/1.1 403 Forbidden
Content-Type: application/json

{
  "error": "Content exists but is not accessible to you",
  "code": "REACH_DENIED"
}
```

**Not accessible to:** Anonymous users

## Caching Behavior

### Cache Key Structure

The cache uses a hierarchical key structure that includes reach level:

```
{dna_hash}:{zome}:{fn}:{args_hash}:{reach}
```

Example:
```
uhC0kabc:content_store:get_content:9f86d081:commons
uhC0kabc:content_store:get_content:9f86d081:private
```

### Cache Hit/Miss

- **Cache HIT** (`X-Cache: HIT`): Content served from in-memory cache
- **Cache MISS** (`X-Cache: MISS`): Content retrieved from MongoDB projection, then cached

### Cache Invalidation

Caches are invalidated using pattern-based rules:

```
Pattern: {dna_hash}:{zome}:{fn}:*:{reach}
Example: uhC0kabc:content_store:get_content:*:commons
```

This invalidates all cache entries for a specific zome function at a given reach level, regardless of arguments.

### TTL (Time-To-Live)

Default cache TTLs:

| Content Type | TTL | Example |
|--------------|-----|---------|
| Immutable content (`get_*`) | 1 hour | Content by ID |
| Lists/aggregates (`list_*`) | 5 minutes | List of content items |
| User-specific data | 1 minute | User preferences |

TTLs can be configured via environment variables:

```bash
CACHE_CONTENT_TTL_SECS=3600      # 1 hour (default)
CACHE_LIST_TTL_SECS=300          # 5 minutes (default)
CACHE_USER_TTL_SECS=60           # 1 minute (default)
CACHE_MAX_ENTRIES=10000          # Max cache entries (default)
```

## Requester Context

The Doorway API extracts requester context from:

1. **Authorization Header**: Extracts agent ID from `Bearer <agent-id>` format
2. **IP Address**: Extracts client IP for geographic routing (future enhancement)
3. **Derived Authentication**: Whether the requester is authenticated

Example extraction:

```rust
let auth_header = "Bearer alice-pubkey-123";
let requester = extract_requester_context(Some(auth_header), Some(ip_addr));

// Result:
// RequesterContext {
//     agent_id: "alice-pubkey-123",
//     location: Some("127.0.0.1"),
//     authenticated: true,
// }
```

## Integration with CustodianCommitment

Reach-aware caching works seamlessly with the `CustodianCommitment` model:

1. **Relationship-Based Access**: Custodian relationships determine which reach levels are accessible to a requester
2. **Category Overrides**: Professional specialists (doctors, firefighters) can custody content at specific reach levels through category overrides
3. **Geographic Prioritization**: Custodians near the requester are prioritized for serving content
4. **Emergency Protocols**: Emergency activation changes reach-level access rules temporarily

### Example: Medical Data With Specialist Override

```json
{
  "id": "patient-medical-record-001",
  "reach": "private",
  "beneficiary": "patient-alice",
  "title": "Medical History",

  "custodian_commitment": {
    "custodian_id": "dr-bob",
    "relationship": "medical_professional",
    "category_override": "medical",
    "access_level": "professional",
    "approved_reach_levels": ["private"],
    "shards": [...],
    "watermark_signatures": [...]
  }
}
```

In this case:
- Only Alice (beneficiary) can access this content as a consumer
- Dr. Bob can access this content as a custodian (professional duty)
- Cache entries are separate for each reach level/requester combination

## Security Considerations

### Information Colonization Prevention

The reach-aware system prevents **information colonization** - where centralized systems extract and monetize local/regional data without community consent:

1. **Geographic Scope**: Local and neighborhood content is gated to avoid unauthorized harvesting
2. **Community Control**: Only authenticated community members can access neighborhood-level data
3. **Professional Oversight**: Specialized custodians provide transparent access logs and governance

### Caching Security

1. **Separate Cache Entries**: Same content at different reach levels has completely separate cache entries
2. **No Reach Leakage**: Cache keys include reach level, preventing access escalation
3. **TTL-Based Expiry**: Cached content automatically expires, limiting stale data exposure
4. **Pattern-Based Invalidation**: Fine-grained invalidation prevents inconsistent cache state

## Development and Testing

### Running Reach-Aware Tests

```bash
cd holochain/doorway
cargo test --lib routes::api::tests
```

Test coverage includes:

- Reach extraction from responses
- Reach-aware cache key generation
- Access control for all reach levels
- Requester context extraction
- Cache hit/miss behavior
- Query parameter parsing

### Example Test

```rust
#[test]
fn test_should_serve_private_to_owner() {
    let response = br#"{"id":"content1","reach":"private"}"#.to_vec();
    let requester = RequesterContext {
        agent_id: "alice".to_string(),
        authenticated: true,
        location: None,
    };

    assert!(should_serve_response(&response, &requester, "alice"));
}
```

## Performance Characteristics

### Reach-Aware Cache Overhead

- **Key Generation**: O(1) - hash-based lookup
- **Access Control**: O(1) - pattern matching on reach field
- **Cache Lookup**: O(1) - DashMap concurrent hash table
- **Reach Extraction**: O(n) - JSON parsing of response (n = response size)

### Memory Usage

Each cached entry includes:

```rust
pub struct CacheEntry {
    pub data: Vec<u8>,                        // Response bytes
    pub etag: String,                          // ETag for revalidation
    pub created_at: Instant,                   // Creation time
    pub expires_at: Instant,                   // Expiration time
    pub content_type: String,                  // MIME type
    pub reach: Option<String>,                 // Reach level (NEW)
    pub cache_priority: u32,                   // Priority (0-100)
    pub bandwidth_class: Option<String>,       // Bandwidth hint
    pub geographic_affinity: Option<String>,   // Location hint
}
```

Typical memory per entry: ~1-5 KB (depending on reach field size)

Default max cache entries: 10,000 = ~10-50 MB total cache

## Future Enhancements

1. **Geographic-Aware Serving**: Prioritize custodians near the requester
2. **Reach-Level Updates**: Allow content to change reach level over time (e.g., private → local after sharing)
3. **Temporary Access**: Grant time-limited access to specific reach levels
4. **Audit Logging**: Log all access attempts for transparency
5. **Reach Refinement**: Nested reach levels with granular control (e.g., `local.household` vs `local.neighborhood`)

## Troubleshooting

### "Content exists but is not accessible to you" (403)

**Cause**: Your authentication doesn't match the content's reach level

**Solution**:
1. Verify you're authenticated: Include `Authorization: Bearer <your-id>` header
2. Check the content's reach level (available in error response context)
3. Verify your relationship to the beneficiary (if private content)
4. Request custodian access if needed (professional category override)

### Cache Not Updating

**Cause**: Content is cached with an old reach level

**Solution**:
1. Wait for TTL expiration (max 1 hour for content)
2. Force refresh by changing query parameters
3. Use projection store directly (DEV_MODE only): `?_conductor=true`
4. Check if cache invalidation rules are firing correctly

### Unexpected "commons" Access Denied

**Cause**: The reach field is not in the response or has a different name

**Solution**:
1. Verify response includes `"reach": "commons"` field
2. Use exact field name (case-sensitive): `reach` not `Reach` or `reachLevel`
3. For array responses, ensure first item has reach field
4. Add reach field to Holochain zome responses

---

**Last Updated**: 2025-12-22
**Status**: Production Ready
**Version**: 1.0
