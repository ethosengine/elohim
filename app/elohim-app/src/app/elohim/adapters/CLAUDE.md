# Adapters - Derived Field Layer

Adapters add **computed/derived fields** to API response types.
They do NOT transform wire format - that's already handled by Rust.

## Core Principle

**Adapters compute new values from existing fields.**

They don't parse JSON, convert case, or transform types.
Those transformations happen in Rust's views.rs before data reaches TypeScript.

---

## When to Use Adapters

### DO Use Adapters For:

1. **Cross-field computation** - Combining multiple fields into one
2. **UI convenience flags** - Booleans for template conditions
3. **Derived state** - Computing state from multiple sources

### DO NOT Use Adapters For:

1. **JSON parsing** - Already done in Rust
2. **camelCase conversion** - Already done in Rust
3. **Type coercion** - Already done in Rust
4. **Field renaming** - Already done in Rust

---

## Pattern Examples

### Good: Computing derived boolean

```typescript
// HumanRelationshipView has two consent flags
// Adapter computes convenience flag

export interface HumanRelationshipView extends HumanRelationshipViewBase {
  isFullyConsented: boolean;  // DERIVED
}

export function withFullyConsentedFlag(
  view: HumanRelationshipViewBase
): HumanRelationshipView {
  return {
    ...view,
    isFullyConsented: view.consentGivenByA && view.consentGivenByB,
  };
}
```

### Good: Parsing typed array from generic JSON

```typescript
// ContributorPresenceView has establishingContentIds as JsonValue
// Adapter types it correctly

export interface ContributorPresenceView extends ContributorPresenceViewBase {
  establishingContentIds: string[];  // TYPED
}

export function withEstablishingContentIds(
  view: ContributorPresenceViewBase
): ContributorPresenceView {
  return {
    ...view,
    establishingContentIds: Array.isArray(view.establishingContentIds)
      ? view.establishingContentIds
      : [],
  };
}
```

---

## Anti-Patterns

### Wrong: JSON parsing (Rust does this)

```typescript
// BAD - JSON is already parsed by Rust
function withParsedMetadata(view: ContentView) {
  return {
    ...view,
    metadata: JSON.parse(view.metadataJson),  // WRONG
  };
}

// GOOD - Just use the field directly
const metadata = view.metadata;  // Already an object
```

### Wrong: Case conversion (Rust does this)

```typescript
// BAD - camelCase is already applied by Rust
function toCamelCase(view: any) {
  return {
    contentType: view.content_type,  // WRONG
  };
}

// GOOD - Fields are already camelCase
const type = view.contentType;
```

### Wrong: Type coercion (Rust does this)

```typescript
// BAD - Booleans are already proper type
function withBooleans(view: any) {
  return {
    isActive: view.isActive === 1,  // WRONG
  };
}

// GOOD - Already boolean
const active = view.isActive;  // Already boolean
```

---

## Current Adapters

| Function | Purpose | Source Fields → Derived |
|----------|---------|------------------------|
| `withFullyConsentedFlag` | UI consent indicator | `consentGivenByA + consentGivenByB → isFullyConsented` |
| `withEstablishingContentIds` | Type string array | `establishingContentIds: JsonValue → string[]` |

---

## Adding New Adapters

1. **Check if Rust should do it** - If it's type transformation, do it in views.rs
2. **Define extended interface** - Add derived fields to base type
3. **Write pure function** - `(base) => extended`
4. **Apply in service** - Use in API service's pipe/map

Example:

```typescript
// 1. Extended interface
export interface ContentViewWithScore extends ContentView {
  popularityScore: number;  // Derived from multiple fields
}

// 2. Pure adapter function
export function withPopularityScore(view: ContentView): ContentViewWithScore {
  return {
    ...view,
    popularityScore: (view.viewCount ?? 0) * 0.5 + (view.shareCount ?? 0) * 2,
  };
}

// 3. Apply in service
getContent(id: string): Observable<ContentViewWithScore> {
  return this.http.get<ContentView>(`/api/content/${id}`).pipe(
    map(withPopularityScore)
  );
}
```

---

## File Structure

```
adapters/
├── CLAUDE.md                    # This file
└── storage-types.adapter.ts     # All adapter functions
```

The single file pattern keeps all derived field logic in one place for easy discovery.
