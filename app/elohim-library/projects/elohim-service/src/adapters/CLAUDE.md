# Adapters - Wire Format Normalization

Content arrives from three transports with different wire formats. This module normalizes them into one shape before the typed pipeline sees anything.

```
Conductor zome (snake_case)  ──→  normalizeConductorContent()  ──→  ContentView
Storage HTTP   (camelCase)   ──→  already ContentView           ──→  ContentView
IndexedDB      (pre-transformed) ──→  already ContentNode       ──→  ContentNode
```

## Two Kinds of Adapters in This Repo

| Location | Purpose | Transforms |
|----------|---------|------------|
| **Here** (`elohim-service/src/adapters/`) | Wire format normalization | snake_case → camelCase, JSON strings → parsed objects, field renames |
| `elohim-app/src/app/elohim/adapters/` | Derived field computation | Existing camelCase fields → new computed fields |

These are different jobs. **Wire normalization** happens at the transport boundary before the typed pipeline. **Derived fields** happen after, in Angular services. Don't mix them.

## Why This Exists

The Rust-to-TypeScript boundary has two paths:

1. **Storage HTTP path** (elohim-storage → doorway → Angular): Rust `#[serde(rename_all = "camelCase")]` + `parse_json_opt()` handles normalization. TypeScript receives clean camelCase ContentView. No work needed.

2. **Conductor zome path** (conductor → WebSocket → Angular): Rust uses default serde (snake_case). `metadata_json` is a stringified JSON field. The `content` field is named `content`, not `contentBody`. TypeScript receives raw wire format.

Doorway does NOT normalize conductor responses — it passes them through as-is (`api.rs`: "Return whatever the DNA returned — no interpretation"). So when doorway falls back to conductor, or when Tauri/Direct modes call zome functions, the Angular side must normalize.

## The Pattern

### ConductorContentResponse (wire type)

```typescript
interface ConductorContentResponse {
  id: string;
  content_type: string;       // snake_case
  content: string | null;     // NOT contentBody
  content_format: string;     // snake_case
  metadata_json: string | null; // stringified, NOT parsed
  blob_hash: string | null;   // snake_case
  // ... other snake_case fields
}
```

### ContentView (schema-generated contract)

```typescript
interface ContentView {
  id: string;
  appId: string;              // added by storage layer
  contentType: ContentType;   // camelCase, typed enum
  contentBody?: string | null; // renamed from content
  contentFormat: ContentFormat; // camelCase, typed enum
  metadata?: unknown;          // parsed object
  blobHash?: string | null;   // camelCase
  validationStatus: ValidationStatus; // added by storage layer
  // ...
}
```

### normalizeConductorContent()

Maps one to the other. One function, one place. Fields not on the conductor wire type get defaults:
- `appId: 'lamad'` (only content DNA today)
- `validationStatus: 'valid'` (conductor is authoritative)

### ensureContentView()

Safe entry point when you don't know which format you have. Detects snake_case via `content_type` field presence, normalizes if needed, passes through if already ContentView.

### isConductorResponse()

Type guard. Uses `content_type` (snake_case) as discriminator — ContentView has `contentType` (camelCase) instead.

## Relationship to transformZomeResponse()

`zome-wire-types.ts` has a generic `transformZomeResponse()` that handles agent-centric zome data (mastery, practice pools, points, recognitions). It does generic snake→camel + `*_json` parsing.

The conductor normalizer handles **content specifically**, mapping to the schema-generated `ContentView` type with correct field renames (`content` → `contentBody`) and typed enums. Don't use `transformZomeResponse()` for content — use `normalizeConductorContent()`.

## When to Add a New Normalizer

If a new transport emerges (Helia P2P, custom protocol) that returns content in a different wire format, add a normalizer here following the same pattern:

1. Define the wire type interface
2. Write a pure function mapping wire type → ContentView
3. Add a type guard for format detection
4. Export from `index.ts`
5. Test with the same pattern as `conductor-normalizer.spec.ts`

The return type is always `ContentView`. That's the contract.

## Files

```
adapters/
├── CLAUDE.md                       # This file
├── conductor-normalizer.ts         # Conductor → ContentView normalizer
├── conductor-normalizer.spec.ts    # 32 tests
└── index.ts                        # Barrel exports
```

Exported via `connection/index.ts` and top-level `index.ts`.
