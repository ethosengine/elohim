---
title: Bring doorway auth wire shapes under the view-schema contract system
created: 2026-06-10
domain: D8 (doorway projection; wire-contract discipline)
source: arc plan Task 1.3 capture (2026-06-10)
severity: medium
---

Doorway auth responses (AuthResponse, MeResponse, ExchangeSessionResponse,
AccountResponse — auth_routes.rs:75-311) are NOT in the view-schema contract
system (elohim/sdk/schemas/v1/views/), so consumers hand-author the TS shapes.
Task 1.1's audit found 5 drift classes between a2o's hand-rolled types and the
Rust structs (missing isSteward/portalHostUrl/trustMode/authority; AccountResponse
inventing a usage/quota nesting that never existed on the wire). The consolidated
types now live in @elohim/identity matched-by-hand to the Rust source — the
schema-contract harness (schema_contract.rs + codegen-ts) would make that drift
structurally impossible. Note: auth shapes are operational session state
(Category C; no storage schema) — this is wire-contract discipline only.
Sources of truth audit + 10-rules conventions: elohim/sdk/schemas/v1/views/CONVENTIONS.md.
RESOLVED 2026-06-11 (auth-wire plan T1, commits 09becf281+3b3a89d4d, re-reviewed ✅): five auth
view schemas + doorway-side contract harness (14 tests); codegen distributes to six locations;
@elohim/identity types are generated re-exports. Drift is now structurally impossible.
