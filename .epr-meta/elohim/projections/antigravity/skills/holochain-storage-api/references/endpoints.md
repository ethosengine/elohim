# HTTP API Endpoint Reference

All endpoints return **camelCase JSON**. All JSON fields are pre-parsed (no `JSON.parse()` needed).

## Content

| Method | Path | Description |
|--------|------|-------------|
| GET | `/db/content` | List content (query: `contentType`, `limit`, `offset`) |
| GET | `/db/content/:id` | Get content by ID |
| POST | `/db/content` | Create content (body: `CreateContentInputView`) |
| PUT | `/db/content/:id` | Update content |
| GET | `/db/content/:id/tags` | Get content tags |
| GET | `/db/content/search?q=...` | Full-text search |

## Paths

| Method | Path | Description |
|--------|------|-------------|
| GET | `/db/paths` | List learning paths |
| GET | `/db/paths/:id` | Get path with chapters, steps, attestations |
| POST | `/db/paths` | Create path (body: `CreatePathInputView`) |

## Relationships

| Method | Path | Description |
|--------|------|-------------|
| GET | `/db/relationships` | List relationships |
| GET | `/db/relationships/:id` | Get relationship by ID |
| POST | `/db/relationships` | Create relationship |
| GET | `/db/content/:id/relationships` | Get relationships for content |

## Mastery

| Method | Path | Description |
|--------|------|-------------|
| GET | `/db/mastery/:humanId` | Get all mastery for a human |
| GET | `/db/mastery/:humanId/:contentId` | Get specific mastery record |
| POST | `/db/mastery` | Create/update mastery (body: `CreateMasteryInputView`) |

## Economic Events

| Method | Path | Description |
|--------|------|-------------|
| POST | `/db/economic-events` | Create economic event |
| GET | `/db/economic-events` | List events (query: `provider`, `action`, `contentId`) |

## Contributor Presence

| Method | Path | Description |
|--------|------|-------------|
| POST | `/db/contributor-presences` | Create contributor presence |
| GET | `/db/contributor-presences/:id` | Get presence by ID |
| POST | `/db/contributor-presences/:id/claim` | Initiate claim |

## Stewardship Allocations

| Method | Path | Description |
|--------|------|-------------|
| POST | `/db/stewardship-allocations` | Create allocation |
| GET | `/db/content/:id/stewardship` | Get content stewardship |
| PUT | `/db/stewardship-allocations/:id` | Update allocation |

## Human Relationships

| Method | Path | Description |
|--------|------|-------------|
| POST | `/db/human-relationships` | Create human relationship |
| GET | `/db/human-relationships` | List relationships |

## Sessions

| Method | Path | Description |
|--------|------|-------------|
| POST | `/session` | Create local session |
| GET | `/session/active` | Get active session |

## Blobs

| Method | Path | Description |
|--------|------|-------------|
| PUT | `/blob/` | Upload blob (binary body, returns manifest) |
| GET | `/blob/:hashOrCid` | Get blob by SHA256 hash or CID |
| HEAD | `/shard/:hashOrCid` | Check if blob exists |
| GET | `/manifest/:hashOrCid` | Get blob manifest |

## Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Quick health check |
| GET | `/db/stats` | Database statistics |

## Batch Import

| Method | Path | Description |
|--------|------|-------------|
| POST | `/store/batch` | Batch import content, paths, relationships |

---

## View Types (API Responses)

| Type | Purpose |
|------|---------|
| `ContentView` | Content item with parsed metadata |
| `ContentWithTagsView` | Content + tags array |
| `PathView` | Learning path |
| `PathWithDetailsView` | Path + chapters + steps + attestations |
| `RelationshipView` | Content relationship |
| `HumanRelationshipView` | Person-to-person relationship |
| `ContributorPresenceView` | Contributor identity |
| `EconomicEventView` | REA economic event |
| `ContentMasteryView` | Mastery record |
| `StewardshipAllocationView` | Content stewardship allocation |
| `LocalSessionView` | Active session |

## InputView Types (API Requests)

| Type | Purpose |
|------|---------|
| `CreateContentInputView` | Create content |
| `CreatePathInputView` | Create path with chapters/steps |
| `CreateRelationshipInputView` | Create relationship |
| `CreateMasteryInputView` | Create/update mastery |
| `CreateEconomicEventInputView` | Create economic event |
| `CreateContributorPresenceInputView` | Create contributor |
| `CreateAllocationInputView` | Create stewardship allocation |
| `CreateHumanRelationshipInputView` | Create human relationship |
| `InitiateClaimInputView` | Initiate contributor claim |

## Transformation Pattern

In `views.rs`, the boundary layer handles all transformations:

```rust
// Output: DB model -> View (snake_case -> camelCase, String -> Value, i32 -> bool)
impl From<Content> for ContentView {
    fn from(c: Content) -> Self {
        Self {
            metadata: parse_json_opt(&c.metadata_json),  // String -> serde_json::Value
            is_active: c.is_active == 1,                 // i32 -> bool
            // serde(rename_all = "camelCase") handles field names
        }
    }
}

// Input: InputView -> DB input (camelCase -> snake_case, Value -> String)
impl From<CreateContentInputView> for CreateContentInput {
    fn from(v: CreateContentInputView) -> Self {
        Self {
            metadata_json: serialize_json_opt(&v.metadata),  // Value -> String
            content_type: v.content_type.unwrap_or("concept".to_string()),
        }
    }
}
```
