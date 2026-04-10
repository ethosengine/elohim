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
