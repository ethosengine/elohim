# Reach-Aware Caching Integration Guide

## For Holochain DNA Developers

This guide shows how to integrate reach-aware caching into your Holochain DNA to enable content gating based on social and geographic proximity.

## Quick Start

### Step 1: Add Reach Field to Your Entry Types

In your Holochain DNA, add a `reach` field to content entries:

```rust
#[hdk::entry_def]
pub struct ContentNode {
    pub id: String,
    pub title: String,
    pub content: String,
    pub reach: String,  // NEW: "private", "local", "commons", etc.
    pub beneficiary: String,  // Owner/creator
}
```

### Step 2: Emit Reach in Zome Responses

When returning content from zome functions, include the `reach` field:

```rust
#[hdk::extern]
fn get_content(input: GetContentInput) -> ExternResult<ContentNode> {
    let content = retrieve_content(&input.id)?;

    // Return with reach field for Doorway caching
    Ok(content)  // Already has reach field
}
```

The response should be JSON with the reach field:

```json
{
  "id": "content-123",
  "title": "My Content",
  "content": "...",
  "reach": "commons",
  "beneficiary": "alice-agent-id"
}
```

### Step 3: Declare Cache Rules (Optional)

Implement the `__doorway_cache_rules` function to declare caching policy:

```rust
#[hdk::extern]
fn __doorway_cache_rules() -> ExternResult<Vec<CacheRule>> {
    Ok(vec![
        CacheRule {
            function: "get_content".to_string(),
            public: false,  // Requires authentication
            reach_field: Some("reach".to_string()),  // Read reach from response
            ttl_secs: Some(3600),  // Cache for 1 hour
        },
        CacheRule {
            function: "list_content".to_string(),
            public: false,
            reach_field: Some("reach".to_string()),
            ttl_secs: Some(300),  // Cache lists for 5 minutes
        },
    ])
}
```

If you don't implement `__doorway_cache_rules`, Doorway uses defaults:
- Functions matching `get_*` or `list_*` are cached
- Default TTL: 1 hour for gets, 5 minutes for lists
- Authentication is required

## Reach Levels Explained

### Private (Most Restrictive)

Only the content beneficiary can access:

```rust
ContentNode {
    id: "diary-001".to_string(),
    title: "My Personal Thoughts".to_string(),
    content: "...".to_string(),
    reach: "private".to_string(),
    beneficiary: "alice-id".to_string(),
}
```

**Access Rules**:
- ✅ Requester agent ID == beneficiary ID
- ❌ Other authenticated users
- ❌ Anonymous users

**Use Cases**: Personal diaries, medical records, financial data

### Invited

Only explicitly invited individuals:

```rust
ContentNode {
    reach: "invited".to_string(),
    // ... invite list stored in custom metadata
}
```

**Access Rules**:
- ✅ Explicitly invited users (requires invite list in metadata)
- ❌ Other authenticated users
- ❌ Anonymous users

**Use Cases**: Shared journals, private group conversations

### Local (Household/Family)

Family/household members:

```rust
ContentNode {
    reach: "local".to_string(),
    beneficiary: "alice-id".to_string(),  // Household ID
}
```

**Access Rules**:
- ✅ Any authenticated user with local relationship
- ❌ Anonymous users
- ⚠️ Implementation depends on relationship system

**Use Cases**: Household chores, family schedules, family resources

### Neighborhood

Street block or neighborhood level:

```rust
ContentNode {
    reach: "neighborhood".to_string(),
}
```

**Access Rules**:
- ✅ Any authenticated user (future: within geographic range)
- ❌ Anonymous users

**Use Cases**: Local events, neighborhood coordination, community notices

### Municipal

City/town level:

```rust
ContentNode {
    reach: "municipal".to_string(),
}
```

**Access Rules**:
- ✅ Any authenticated user
- ❌ Anonymous users

**Use Cases**: City announcements, municipal services, local government

### Bioregional

Watershed/ecosystem level:

```rust
ContentNode {
    reach: "bioregional".to_string(),
}
```

**Access Rules**:
- ✅ Any authenticated user
- ❌ Anonymous users

**Use Cases**: Environmental data, water management, ecosystem resources

### Regional

State/province level:

```rust
ContentNode {
    reach: "regional".to_string(),
}
```

**Access Rules**:
- ✅ Any authenticated user
- ❌ Anonymous users

**Use Cases**: Regional news, state resources, provincial announcements

### Commons (Least Restrictive)

Global/public - everyone can access:

```rust
ContentNode {
    reach: "commons".to_string(),
}
```

**Access Rules**:
- ✅ Any authenticated user
- ✅ Anonymous users

**Use Cases**: Public knowledge, open resources, published work

## Integration Examples

### Example 1: Simple Content with Fixed Reach

```rust
#[hdk::extern]
fn create_content(input: CreateContentInput) -> ExternResult<ContentNode> {
    let content = ContentNode {
        id: crate::utils::generate_id(),
        title: input.title,
        content: input.content,
        reach: "commons".to_string(),  // Always public
        beneficiary: input.beneficiary,
    };

    // Store in DHT...

    Ok(content)
}
```

### Example 2: Content with Configurable Reach

```rust
#[hdk::extern]
fn create_private_content(
    input: CreatePrivateContentInput
) -> ExternResult<ContentNode> {
    let content = ContentNode {
        id: crate::utils::generate_id(),
        title: input.title,
        content: input.content,
        reach: input.reach.clone(),  // User chooses: private/local/neighborhood
        beneficiary: input.beneficiary.clone(),
    };

    // Validate reach level
    if !["private", "local", "neighborhood", "commons"].contains(&content.reach.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Invalid reach level".to_string()
        )));
    }

    // Store in DHT...
    Ok(content)
}
```

### Example 3: List Responses with Reach

For list responses, Doorway extracts reach from the first item:

```rust
#[hdk::extern]
fn list_content(filter: ContentFilter) -> ExternResult<Vec<ContentNode>> {
    let mut items = vec![];

    // Query DHT for matching items...
    // For each item, filter based on requester permissions

    // Return items - Doorway will extract reach from first item
    Ok(items)
}
```

**Important**: When returning a list, Doorway extracts the `reach` field from the first item in the array. All items in the list should have the same reach level for consistent caching.

### Example 4: Dynamic Reach Based on Relationships

```rust
#[hdk::extern]
fn get_content_for_user(input: GetContentInput) -> ExternResult<ContentNode> {
    let content = retrieve_content(&input.id)?;
    let requester = input.requester_id;

    // Determine reach based on relationships
    let reach = if content.beneficiary == requester {
        "private".to_string()  // Owner sees full private content
    } else if has_family_relationship(&requester, &content.beneficiary) {
        "local".to_string()
    } else if in_same_neighborhood(&requester, &content.beneficiary) {
        "neighborhood".to_string()
    } else {
        "commons".to_string()
    };

    // Update reach (or use overrides)
    let response = ContentNode {
        reach,
        ..content
    };

    Ok(response)
}
```

## Working with CustodianCommitments

If using the `CustodianCommitment` model, integrate reach awareness with custody:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContentWithCustody {
    pub content: ContentNode,
    pub custodian_commitment: Option<CustodianCommitment>,
    pub custodian_shards: Vec<Shard>,
}

#[hdk::extern]
fn get_content_with_custody(input: GetContentInput) -> ExternResult<ContentWithCustody> {
    let content = retrieve_content(&input.id)?;

    // Load custody info if exists
    let commitment = load_custodian_commitment(&content.id)?;
    let shards = if commitment.is_some() {
        load_shards(&content.id)?
    } else {
        vec![]
    };

    Ok(ContentWithCustody {
        content,
        custodian_commitment: commitment,
        custodian_shards: shards,
    })
}
```

## Testing Reach-Aware Caching

### Test 1: Verify Reach Field in Response

```rust
#[test]
fn test_content_includes_reach() {
    let content = create_test_content("commons");
    let json = serde_json::to_value(&content).unwrap();

    assert_eq!(json["reach"], "commons");
}
```

### Test 2: Verify Caching via HTTP API

```bash
# Create content with "commons" reach
curl -X POST "http://localhost:3000/api/v1/dna/zome/create_content" \
  -H "Content-Type: application/json" \
  -d '{"title":"Test","reach":"commons"}'

# Fetch via REST API (first request = cache MISS)
curl -X GET "http://localhost:3000/api/v1/dna/zome/get_content?id=test-001" \
  -H "Authorization: Bearer user-1"
# Response: X-Cache: MISS, X-Reach: commons

# Fetch again (second request = cache HIT)
curl -X GET "http://localhost:3000/api/v1/dna/zome/get_content?id=test-001" \
  -H "Authorization: Bearer user-1"
# Response: X-Cache: HIT, X-Reach: commons

# Try accessing without auth (should be allowed for commons)
curl -X GET "http://localhost:3000/api/v1/dna/zome/get_content?id=test-001"
# Response: 200 OK, X-Cache: HIT, X-Reach: commons
```

### Test 3: Verify Private Content Gating

```bash
# Create private content
curl -X POST "http://localhost:3000/api/v1/dna/zome/create_private_content" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer alice" \
  -d '{"title":"Secret","reach":"private","beneficiary":"alice"}'

# Try accessing as owner (should succeed)
curl -X GET "http://localhost:3000/api/v1/dna/zome/get_content?id=secret-001" \
  -H "Authorization: Bearer alice"
# Response: 200 OK

# Try accessing as different user (should fail)
curl -X GET "http://localhost:3000/api/v1/dna/zome/get_content?id=secret-001" \
  -H "Authorization: Bearer bob"
# Response: 403 FORBIDDEN - "Content exists but is not accessible to you"

# Try accessing without auth (should fail)
curl -X GET "http://localhost:3000/api/v1/dna/zome/get_content?id=secret-001"
# Response: 403 FORBIDDEN
```

## Debugging

### Check Reach Extraction

Enable debug logging to see reach extraction:

```bash
RUST_LOG=doorway::cache::reach_aware_serving=debug cargo run
```

You'll see:
```
DEBUG doorway::cache::reach_aware_serving: extract_reach_from_response: Found reach level "commons"
```

### Monitor Cache Behavior

Watch cache statistics:

```bash
curl -X GET "http://localhost:3000/status" | jq '.cache'
```

Output:
```json
{
  "entries": 42,
  "hits": 1523,
  "misses": 287,
  "hit_rate": "84.2%"
}
```

### Verify Cache Keys

Check what's in the cache (development only):

```bash
# Check if specific entry is cached
curl -X GET "http://localhost:3000/api/v1/dna/zome/get_content?id=test" \
  -H "Authorization: Bearer user-1" \
  -v
# Look for "X-Cache: HIT" header
```

## Best Practices

1. **Always Include Reach**: Every content response should include a reach field
2. **Use Sensible Defaults**: Start with "commons" for public content, "local" for community content
3. **Respect User Choice**: Let users control reach level when creating content
4. **Document Reach Policy**: Clearly communicate what each reach level means in your DNA
5. **Monitor Performance**: Track cache hit rates and adjust TTLs if needed
6. **Test Access Control**: Verify reach-level gating works as expected
7. **Use CustodianCommitments**: Integrate custody for important/sensitive content
8. **Audit Access**: Log access attempts to sensitive content (future enhancement)

## Common Issues

### Issue: Cache Not Respecting Reach Changes

**Problem**: Content reach level changed, but cache still serves old value

**Solution**:
- Cache invalidation needs to fire on content update
- Implement `__doorway_invalidate_cache` hook to notify Doorway of changes
- Or wait for TTL expiration (max 1 hour)

### Issue: List Response Reach Extraction Fails

**Problem**: List endpoint returns items with different reach levels, Doorway only sees first

**Solution**:
- Ensure all items in list have same reach level
- Or use separate endpoints for each reach level
- Document that Doorway extracts reach from first array item

### Issue: Unauthenticated Access to Private Content

**Problem**: Private content accessible without authentication header

**Solution**:
- Verify content has `"reach": "private"` in response (case-sensitive!)
- Verify beneficiary ID matches requester agent ID
- Check that auth header is properly formatted: `Bearer <agent-id>`

---

**Last Updated**: 2025-12-22
**Status**: Production Ready
**Version**: 1.0
