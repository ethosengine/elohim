# View Schema Conventions

View schemas define the JSON wire format for HTTP API responses. They are the
**single source of truth** for the shape of data that crosses the Rust→TypeScript
boundary via HTTP.

## Rules

### 1. camelCase field names
All properties use camelCase. The Rust struct uses `#[serde(rename_all = "camelCase")]`.

### 2. Source of truth declaration (REQUIRED)
The top-level `description` field MUST declare the entity's source of truth
and its P2P design gate category (A/A2/B/B2/C). Examples:

- Category A: `"Source of truth: DHT (Notarized, Category A)."`
- Category C: `"Source of truth: libp2p Swarm state (Operational, Category C). Reconstructed per request. Not persisted."`

The validation harness enforces this: a schema without "Source of truth:" in
the description fails the contract test.

### 3. additionalProperties: false
Every view schema MUST set `additionalProperties: false`. This prevents
undeclared fields from leaking through and makes the contract tight.

### 4. required array
Every non-nullable field MUST appear in the `required` array.

### 5. Nullable fields
Use JSON Schema nullable pattern: `{ "type": ["string", "null"] }`.
The `required` array determines whether the field must be present;
the type determines whether its value can be null.

### 6. $id format
Use EPR-style IDs: `epr:schema:view:{name}` (e.g., `epr:schema:view:p2p-status`).

### 7. Enum references
Use `$ref` to reference enum schemas in `../enums/`. Never inline enum values
in view schemas — the enum schema is the single source of truth.

### 8. Integer types
Use `"type": "integer"` for counts and indices. For large values (e.g.,
uptimeSeconds as u64), use `"type": "string"` with a `"pattern"` constraint
and document the bigint-as-string convention.

### 9. Nested objects
Use `$ref` to reference object schemas for nested types (e.g., replication
status, drain status). Define these as separate schemas in `views/` or
`objects/` as appropriate.

### 10. File naming
`{entity-name}.schema.json` in kebab-case matching the `$id` suffix.

### 11. Honest absence — a dual-provenance absence is never a bare nullable
A field whose absence has **two provenances** MUST NOT be a bare nullable.

Rule 5 gives you `{ "type": ["string", "null"] }`, which is right when `null`
means exactly one thing. It is wrong the moment `null` can mean *both* "the
responder answered and there is nothing there" **and** "we never got an answer."
Those are a fact about the world and a fact about the network; one bit cannot
carry both, and the collapse is silent — the client cannot tell whether to
render "nothing here" or to retry.

Ask, for every nullable you add: **can this be null because we could not ask?**

- **No** — one provenance. A bare nullable is correct; say so in the
  `description` so the next reader does not have to re-derive it.
- **Yes** — two provenances. Use the answer envelope
  (`../objects/answer.schema.json`): `state` is `present` / `absent` /
  `unreachable`, and `reason` says why. `absent` is a *positive claim* — absence
  was observed — and must never be emitted for an answer that did not arrive.

The Rust side of this is `seam_contracts::Answer<T>` (`crates/seam-contracts`),
which is generic; the wire stays monomorphic — a per-view envelope narrows
`value` and inherits `state`/`reason`. Do not add ts-rs generics to satisfy this
rule.

**This rule is forward-looking, not retroactive.** A majority of existing view
schemas sit on the bare-nullable surface, and several of their nullables are
genuinely single-provenance. Rule 11 binds *new* dual-provenance fields and any
existing one being touched; it does not condemn the whole tree, and there is no
validator that can tell the two cases apart — the judgement is the author's, at
design time. Known dual-provenance nullables already documented in prose (e.g.
`epr-pull-status`'s `total` / `caughtUp` "tri-state") are recorded as adoption
candidates in `../objects/answer.schema.json`'s `_adoptions` block.

Concern class C4 (honest absence). Forcing incidents: `dd1824e03` (2026-07-22,
*unreadable ≠ absent*), `d6c88e385` (2026-07-23, *unmeasured ≠ zero*),
`270dbafac` (2026-07-11, absent-because-misspelled ran the fleet with zero ICE
servers since inception).
