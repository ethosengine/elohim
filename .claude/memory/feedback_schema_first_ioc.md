---
name: Schema-first is IoC — never guess at implementation
description: For any wire contract (HTTP view, coordinator input, enum, etc.), write the JSON schema FIRST and make Rust/TS comply. The contract leads, code follows.
type: feedback
originSessionId: 6ec4bfae-b3f0-4040-8a90-6ae504910fe7
---
When adding a new wire type (View, Input, enum, etc.), the correct order is:

1. Write the JSON Schema in `elohim/sdk/schemas/v1/{views,inputs,enums}/`
2. Write the Rust struct that matches it (with `#[serde(rename_all = "camelCase")]` for views)
3. Add a schema contract test in `elohim/elohim-storage/tests/schema_contract.rs`
4. Register in codegen pipelines (`INTERFACE_FILES` in codegen-ts.mjs)
5. Run `pnpm run schema:codegen:ts` — both generated TS and Rust-derived TS must agree; schema is authoritative

**Why:** This is an Inversion of Control pattern — the schema is the interface; implementations comply. Doing it backwards (Rust-first, schema-maybe-later) produces drift, bugs at the boundary, and "whose type is right?" debates. Schema-first avoids whole classes of bugs all the way down.

**How to apply:**
- Any new `*View` struct → schema first
- Any new `CreateFooInput` wire type → schema first
- Any new enum at the protocol boundary → schema first
- Hand-rolled Rust types that mirror existing TypeScript or vice-versa are a smell; find or write the schema
- Never "guess at implementation" when a contract can be written

Flagged 2026-04-19 during Phase 4 Task 4.2 — I almost dispatched an implementer with "write the View struct, then add schema contract test if schema exists." User corrected: the schema must exist first.
