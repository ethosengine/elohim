# Elohim Protocol SDK

The TypeScript surface of the Elohim Protocol — types and client libraries that applications use to interact with the protocol's distributed infrastructure.

## What Belongs in the SDK

**Could this capability be captured at scale for rent extraction?**

If someone could become the bank, the credential authority, the governance board, or the content landlord by controlling it — it's a protocol primitive and belongs here. Applications compose these primitives; they don't own them.

### Protocol Primitives (SDK types)
- Economic: Agreement, Commitment, EconomicEvent, Measure
- Identity: Human, Agent, Attestation, ContributorPresence
- Content: Content, LearningPath, ContentMastery
- Governance: Proposals, Votes, Challenges, Appeals
- Substrate signals: attention, compute, storage, bandwidth, energy, time, resource

### NOT SDK (app layer)
- What `PathMetadata.thumbnailUrl` means → that's `app/lamad/`
- Doorway projection/cache types → web2 bridge, not protocol primitive
- UI state, dashboard aggregations, theme preferences
- Quiz session state, streak tracking → app-level compositions

## Two-Layer Type System

```
Protocol Layer (this directory)              App Layer (app/lamad/)
─────────────────────────────               ────────────────────────
schemas/  → protocol JSON schemas            manifest.json → domain vocabulary
  enums/     ContentType, Reach, ...          schemas/  → PathMetadata, EprCompositeBody
  inputs/    CreateContentInput               scripts/  → codegen from manifest + schemas
  views/     ContentView, EconomicEventView   generated/ → discriminated unions, type guards
  manifest/  app-manifest.schema.json

storage-client-ts/ → Rust ts-rs generated     Both layers generate to:
  generated/  → runtime types from views.rs    app/elohim-app/src/app/lamad/generated/
                                               genesis/seeder/src/generated/
```

The protocol owns the **envelope** (wire shape, enums, generic metadata bag). The app manifest owns the **payload** (what metadata means per content type). Generated types compose both — the seeder and Angular share identical files.

## Type Generation

Two codegen paths:

```bash
# Protocol types (enums, wire types) → 3 distribution locations
pnpm run schema:codegen:ts

# App domain types (metadata, body, coupling map) → 2 distribution locations
pnpm run lamad:codegen
```

See `schemas/CLAUDE.md` for protocol codegen details.
See `../../app/lamad/CLAUDE.md` for app codegen details.

## storage-client-ts

Rust-generated TypeScript types from `elohim-storage/src/views.rs`. These are the runtime representation of the API boundary.

```
Rust structs (views.rs)
  → #[derive(TS)] + #[serde(rename_all = "camelCase")]
  → cargo test export_bindings
  → storage-client-ts/src/generated/*.ts
```

**Key rule:** snake_case never leaves the Rust boundary. TypeScript receives camelCase with parsed JSON and proper booleans. No `JSON.parse()`, no case conversion, no `toWire/fromWire` in TypeScript.

When protocol schemas and ts-rs types disagree, the protocol schema is authoritative for field names and enum values.

## Signal Harness Pattern

The protocol enforces that every content interaction produces the declared economic event. The signal harness (in the app) reads the manifest's coupling declarations:

```
Renderer emits RendererCompletionEvent
    ↓
SignalHarnessService reads manifest coupling for contentType
    ↓ translates to typed CreateEconomicEventInput
EconomicEventsApiService.createEconomicEvent()
    ↓
DHT notarization → storage projection → governance signals
```

Apps can't skip economic events because the harness IS the render-to-protocol bridge.

## Feedback as Information Flow

Feedback is not a fourth coupling leg — it's information flowing through all three legs. The manifest requires every content type to declare:

1. **Claims** — what outcomes it asserts (with validity horizon)
2. **Observations** — positive and negative evidence (must include negatives)
3. **Obligation accumulation** — REA-native: accumulated negative observations shorten validity, expired validity generates review obligations

See `genesis/plans/2026-03-28-feedback-information-flows-design.md`.
