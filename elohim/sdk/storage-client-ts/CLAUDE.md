# Storage Client TypeScript - Generated Types

This package contains **auto-generated TypeScript types** from Rust via ts-rs.
These types define the API contract between TypeScript clients and elohim-storage.

## Core Principle

**DO NOT modify generated files manually.**

All types are generated from Rust View types in `elohim-storage/src/views.rs`.
Changes must be made in Rust, then regenerated.

---

## Generated Types Location

```
src/generated/
├── index.ts              # Re-exports all types
├── ContentView.ts        # Content response type
├── PathView.ts           # Path response type
├── RelationshipView.ts   # Relationship response type
├── CreateContentInputView.ts  # Content creation input
├── CreatePathInputView.ts     # Path creation input
└── ... (50+ type files)
```

---

## Using Generated Types

### Import Pattern

```typescript
// Import from generated module
import {
  ContentView,
  PathView,
  CreateContentInputView
} from '@elohim/storage-client/generated';

// Or via the package root
import { ContentView } from '@elohim/storage-client';
```

### Type Characteristics

All generated types have:

1. **camelCase field names** - Converted from Rust snake_case
2. **Parsed JSON fields** - `metadata: JsonValue` not `metadataJson: string`
3. **Proper boolean types** - `isActive: boolean` not `isActive: number`
4. **Optional fields** - Correctly typed as `field?: Type`

---

## What You Get (No Transformation Needed)

```typescript
// Response from API is ready to use
const content: ContentView = await api.getContent(id);

// All fields are properly typed and cased
console.log(content.contentType);      // string, not content_type
console.log(content.metadata);         // object, not JSON string
console.log(content.isActive);         // boolean, not 0/1

// Input types work the same way
const input: CreateContentInputView = {
  id: 'new-content',
  title: 'My Content',
  contentType: 'concept',              // camelCase
  metadata: { author: 'John' },        // object, not string
};
```

---

## Anti-Patterns (DO NOT DO)

### Wrong: Manual JSON parsing

```typescript
// BAD - JSON is already parsed
const meta = JSON.parse(content.metadataJson);

// GOOD - Use directly
const meta = content.metadata;
```

### Wrong: Snake_case field access

```typescript
// BAD - Types use camelCase
const type = content.content_type;

// GOOD - Use camelCase
const type = content.contentType;
```

### Wrong: Manual type transformations

```typescript
// BAD - Creating wrapper functions
function fromWireContent(wire: any): Content { ... }
function toWireContent(content: Content): any { ... }

// GOOD - Use types directly
const content: ContentView = await api.getContent(id);
await api.createContent(inputView);
```

---

## Regenerating Types

When Rust types change:

```bash
# From elohim-storage directory
cd holochain/elohim-storage
cargo test export_bindings

# Types are written to:
# holochain/sdk/storage-client-ts/src/generated/

# Rebuild the package
cd ../sdk/storage-client-ts
npm run build
```

---

## Type Categories

### View Types (API Responses)

| Type | Purpose |
|------|---------|
| `ContentView` | Single content item |
| `ContentWithTagsView` | Content with tags array |
| `PathView` | Learning path |
| `PathWithDetailsView` | Path with chapters and steps |
| `RelationshipView` | Content relationship |
| `HumanRelationshipView` | Person-to-person relationship |
| `ContributorPresenceView` | Contributor identity |
| `EconomicEventView` | Economic event (REA) |
| `StewardshipAllocationView` | Content stewardship |

### InputView Types (API Requests)

| Type | Purpose |
|------|---------|
| `CreateContentInputView` | Create content |
| `CreatePathInputView` | Create path with chapters/steps |
| `CreateRelationshipInputView` | Create relationship |
| `CreateHumanRelationshipInputView` | Create human relationship |
| `CreateContributorPresenceInputView` | Create contributor |
| `CreateEconomicEventInputView` | Create economic event |
| `CreateAllocationInputView` | Create stewardship allocation |

---

## JsonValue Type

Parsed JSON fields use the `JsonValue` type:

```typescript
type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

// Usage
interface ContentView {
  metadata?: JsonValue;  // Can be any valid JSON
}
```

---

## Package Structure

```
storage-client-ts/
├── package.json          # @elohim/storage-client
├── src/
│   ├── index.ts          # Main exports
│   └── generated/        # Auto-generated from Rust
│       ├── index.ts      # Re-exports
│       └── *.ts          # Type files
├── dist/                 # Built output
└── CLAUDE.md             # This file
```
