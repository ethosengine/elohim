# Elohim Protocol SDK

This directory contains the TypeScript surface of the Elohim Protocol —
the types and client libraries that applications (including elohim-app)
use to interact with the protocol's distributed infrastructure.

## What Belongs in the SDK

The SDK boundary is defined by one question:

**Could this capability be captured at scale for rent extraction?**

If a capability could let someone become the bank, the credential
authority, the governance board, or the content landlord — it's a
protocol primitive and its types belong here. Applications compose
these primitives into experiences; they don't own the primitives.

### Protocol Primitives (SDK types)
- Economic types: Agreement, Commitment, EconomicEvent, Measure
- Identity types: Human, Agent, Attestation, ContributorPresence
- Content types: Content, LearningPath, ContentMastery
- Governance types: (coming — consent records, proposals)

### NOT SDK (application or bridge layer)
- Doorway projection/cache types — doorway is a web2 bridge for hosted
  humans progressing toward stewardship, not a protocol primitive
- UI state, dashboard aggregations, theme preferences
- Quiz session state, streak tracking — app-level compositions

## Type Generation Pipeline

```
Rust structs (views.rs)
  → #[derive(TS)] + #[serde(rename_all = "camelCase")]
  → cargo test export_bindings
  → storage-client-ts/src/generated/*.ts
```

Types flow from Rust to TypeScript. Never hand-write TypeScript types
that mirror Rust structs — they will drift.

## storage-client-ts

The generated types in `storage-client-ts/src/generated/` are the
canonical TypeScript representation of the protocol's API boundary.
snake_case never leaves Rust. TypeScript receives camelCase with parsed
JSON and proper booleans.
