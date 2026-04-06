# Type Integration Sprint — Doorway + elohim-app

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the type gaps between the three layers — Holochain coordinator (MessagePack), doorway/storage HTTP (JSON), and Angular frontend — so the seeder works and we stop guessing at wire formats.

**Architecture:** Three layers with two boundaries:

```
Layer 1: Holochain Conductor        Layer 2: doorway / elohim-storage        Layer 3: elohim-app (Angular)
 (MessagePack, snake_case)           (JSON, camelCase)                        (TypeScript)
                                                                              
 imagodei-types (Rust)    ←→    doorway parse_zome_response    ←→    @elohim/storage-client/generated
 lamad-types (Rust)       ←→    elohim-storage views.rs        ←→    @app/{domain}/generated/
 qahal-types (Rust)       ←→         (ts-rs export)            ←→    @app/{domain}/models/ (hand-written)
 shefa-types (Rust)       ←→                                   
 avodah-types (Rust)      ←→                                   
```

The seeder failure (`missing field 'action_hash'`) is at the Layer 1↔2 boundary. This sprint fixes that, then ensures the Layer 2↔3 boundary is also sound.

**Tech Stack:** Rust (rmpv, rmp-serde, holochain_client), TypeScript (ts-rs), Angular

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `doorway/doorway-service/src/services/zome_caller.rs` | Modify | Fix conductor response parsing |
| `doorway/doorway-service/src/worker/zome_call.rs` | Read | Reference for correct parsing pattern |
| `elohim/elohim-storage/src/views.rs` | Audit | Verify View types match domain wire types |
| `elohim/sdk/domains/*/types/Cargo.toml` | Modify | Enable ts feature for TypeScript generation |

---

### Task 1: Fix conductor response deserialization in ZomeCaller

**Files:**
- Modify: `doorway/doorway-service/src/services/zome_caller.rs`
- Read: `doorway/doorway-service/src/worker/zome_call.rs` (reference for correct pattern)

The seeder fails because `parse_zome_response` doesn't correctly handle the conductor's response envelope. The conductor returns a WireMessage with nested MessagePack layers, and the current parser has a fallthrough case that returns the wrong layer of bytes.

**Background:** The Holochain conductor WebSocket protocol uses `holochain_websocket::WireMessage`. A successful zome call response looks like:

```
WireMessage (outer):
  { id: N, type: "response", data: <binary> }

The "data" binary contains the AppResponse (inner):
  { type: "zome_called", data: <binary ExternIO> }
  
The ExternIO binary contains the actual zome output:
  (MessagePack-encoded HumanOutput, serialized via rmp_serde::to_vec_named)
```

The current `parse_zome_response` looks for the field name `"value"` in the inner response, but the Holochain conductor may use `"data"` instead. If neither `Binary` nor `Map` matches for the `"value"` key, it falls through and returns the full inner envelope (including `type` and `value/data` fields) — which then fails to deserialize as `HumanOutput`.

- [ ] **Step 1: Study the official holochain_client response parsing**

Read how the `holochain_client` crate (0.9.0-dev.5, already a doorway dependency) deserializes `AppResponse`. Check:

```bash
find /projects/elohim -path "*/holochain_client*" -name "*.rs" 2>/dev/null | head -20
# If not found in source, check Cargo.lock for version and look in registry
grep -r "ZomeCalled\|AppResponse\|zome_called" ~/.cargo/registry/src/*/holochain_client-*/src/ 2>/dev/null | head -20
```

The key question: what field name does the conductor use for the ExternIO bytes in the AppResponse? Is it `value`, `data`, or something else?

- [ ] **Step 2: Study the existing working parser in worker/zome_call.rs**

Read `doorway/doorway-service/src/worker/zome_call.rs` — its `parse_response` method works for other zome calls. Understand:
1. How many layers does it strip?
2. What field names does it look for?
3. Does it handle the ExternIO → typed deserialization differently?

Compare with `parse_zome_response` in `zome_caller.rs`. The difference is the bug.

- [ ] **Step 3: Fix parse_zome_response**

Based on Steps 1-2, update `parse_zome_response` to correctly handle the conductor's actual response format. The fix should:
1. Try both `"value"` and `"data"` field names for the inner response
2. Handle ExternIO bytes correctly (they may be wrapped in another layer)
3. Never fall through to returning the full inner envelope
4. Log a clear error if the response format is truly unexpected

```rust
// In parse_zome_response, after decoding the inner response:
// Try both "value" and "data" — conductor may use either field name
let result_field = get_field(inner_map, "value")
    .or_else(|| get_field(inner_map, "data"));

match result_field {
    Some(Value::Binary(result_bytes)) => {
        // ExternIO bytes — the actual zome output
        return Ok(result_bytes.clone());
    }
    Some(Value::Map(ref result_map)) => {
        // Inline map — re-encode
        let mut buf = Vec::new();
        rmpv::encode::write_value(&mut buf, &Value::Map(result_map.clone()))?;
        return Ok(buf);
    }
    Some(other) => {
        return Err(format!(
            "Unexpected value type in zome response: {:?}",
            other.to_string().chars().take(200).collect::<String>()
        ));
    }
    None => {
        return Err(format!(
            "Inner response has no 'value' or 'data' field. Keys: {:?}",
            inner_map.iter().map(|(k, _)| format!("{k}")).collect::<Vec<_>>()
        ));
    }
}
```

- [ ] **Step 4: Write a test that simulates the conductor response**

Create a test in `zome_caller.rs` that builds a realistic conductor response envelope containing a `HumanOutput`, then verifies `parse_zome_response` correctly extracts it:

```rust
#[test]
fn test_parse_zome_response_with_human_output() {
    use imagodei_types::{Human, HumanOutput};
    
    // Simulate what the zome serializes
    let human_output = HumanOutput {
        action_hash: ActionHash::from_raw_39(vec![0xA4; 39]).unwrap(),
        human: Human {
            id: "test-123".to_string(),
            display_name: "Test User".to_string(),
            bio: None,
            affinities: vec![],
            profile_reach: "public".to_string(),
            location: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        },
    };
    
    // Serialize as the zome would (named map format, like SerializedBytes)
    let extern_io_bytes = rmp_serde::to_vec_named(&human_output).unwrap();
    
    // Build inner response: { type: "zome_called", data: <extern_io_bytes> }
    // (or "value" — test both)
    let inner = Value::Map(vec![
        (Value::String("type".into()), Value::String("zome_called".into())),
        (Value::String("data".into()), Value::Binary(extern_io_bytes)),
    ]);
    let mut inner_buf = Vec::new();
    rmpv::encode::write_value(&mut inner_buf, &inner).unwrap();
    
    // Build outer WireMessage: { id: 1, type: "response", data: <inner_buf> }
    let outer = Value::Map(vec![
        (Value::String("id".into()), Value::Integer(1.into())),
        (Value::String("type".into()), Value::String("response".into())),
        (Value::String("data".into()), Value::Binary(inner_buf)),
    ]);
    let mut response_bytes = Vec::new();
    rmpv::encode::write_value(&mut response_bytes, &outer).unwrap();
    
    // Parse and verify
    let result_bytes = parse_zome_response(&response_bytes).unwrap();
    let parsed: HumanOutput = rmp_serde::from_slice(&result_bytes).unwrap();
    assert_eq!(parsed.human.id, "test-123");
    assert_eq!(parsed.human.display_name, "Test User");
}
```

- [ ] **Step 5: Run tests**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo test --lib --bins zome_caller 2>&1 | tail -10
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -5
```

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/services/zome_caller.rs
git commit -m "fix(doorway): correctly parse conductor zome call response envelope

parse_zome_response now checks both 'value' and 'data' field names
in the inner AppResponse, matching the conductor's actual WireMessage
format. Eliminates the fallthrough case that returned the full
envelope instead of the ExternIO bytes.

Includes integration test with simulated conductor response."
```

---

### Task 2: Verify the full seeder → doorway → conductor round-trip

**Files:**
- Read: `genesis/seeder/src/seed-humans.ts` (or wherever the seeder calls doorway)
- Read: `doorway/doorway-service/src/routes/auth_routes.rs` (the registration endpoint)
- Read: `doorway/doorway-service/src/routes/zome_helpers.rs` (the ZomeCaller wrapper)

This task verifies the complete chain from seeder HTTP request through doorway to conductor.

- [ ] **Step 1: Trace the seeder's HTTP request**

Read the seeder code that calls the doorway registration endpoint. Document:
1. What HTTP endpoint does it call?
2. What JSON body does it send?
3. What response does it expect?

- [ ] **Step 2: Trace the doorway registration handler**

Read `auth_routes.rs` to trace:
1. How does the handler parse the HTTP body?
2. How does it construct `CreateHumanInput`?
3. How does it call `call_create_human`?
4. How does it use the `HumanOutput` response?

- [ ] **Step 3: Verify type agreement at each boundary**

Check that:
1. The seeder's JSON body matches what the handler expects
2. `CreateHumanInput` fields match what the zome expects (now guaranteed by imagodei-types)
3. `HumanOutput` fields match what the handler uses (now guaranteed by imagodei-types)
4. The handler's HTTP response matches what the seeder expects

Document any mismatches.

- [ ] **Step 4: Write a doorway integration test for the registration flow**

Create a test that exercises the full serialization chain without a live conductor:
1. Serialize `CreateHumanInput` with `rmp_serde::to_vec` (as ZomeCaller does)
2. Verify it's valid MessagePack that the zome would accept
3. Build a mock conductor response with a `HumanOutput`
4. Parse it through `parse_zome_response`
5. Deserialize as `HumanOutput`
6. Verify all fields are correct

- [ ] **Step 5: Commit**

---

### Task 3: Verify elohim-storage views.rs matches domain wire types

**Files:**
- Read: `elohim/elohim-storage/src/views.rs`
- Read: `elohim/sdk/domains/imagodei/types/src/lib.rs`
- Read: `elohim/sdk/domains/lamad/types/src/lib.rs`

The storage layer (`views.rs`) serves HTTP API responses to the Angular app. These View types should be consistent with the domain wire types, even though they're different structs (one is MessagePack for conductor, one is JSON for HTTP).

- [ ] **Step 1: Compare imagodei View types to imagodei wire types**

Read `views.rs` and find `CreateHumanInputView`, `HumanView`, etc. Compare field-by-field with `imagodei-types` to verify they agree on the same data shape (field names differ — camelCase vs snake_case — but field set and types must match).

- [ ] **Step 2: Compare lamad View types to lamad wire types**

Same comparison for `ContentView`, `PathView`, `StepView`, etc.

- [ ] **Step 3: Document any mismatches**

Create a report listing:
- Fields present in wire types but missing from views (data not exposed via HTTP)
- Fields present in views but missing from wire types (computed/derived fields)
- Type differences (e.g., `Option<String>` in wire vs `String` in view)

This is an audit, not necessarily a fix — some differences are intentional (views add derived fields). But any UNINTENTIONAL mismatches should be flagged.

- [ ] **Step 4: Commit audit notes**

If mismatches are found, create a brief document at `genesis/plans/2026-04-05-type-audit-findings.md` listing them.

---

### Task 4: Enable TypeScript generation from domain types crates

**Files:**
- Modify: `elohim/sdk/domains/imagodei/types/Cargo.toml`
- Modify: `elohim/sdk/domains/lamad/types/Cargo.toml`
- Modify: `elohim/sdk/domains/qahal/types/Cargo.toml`
- Modify: `elohim/sdk/domains/shefa/types/Cargo.toml`
- Modify: `elohim/sdk/domains/avodah/types/Cargo.toml`
- Modify: `elohim/sdk/domains/infrastructure/types/Cargo.toml`

Each types crate already has an optional `ts` feature with `ts-rs`. This task enables it and generates TypeScript output.

- [ ] **Step 1: Test TypeScript generation for imagodei-types**

```bash
cd elohim/sdk/domains/imagodei/types
cargo test --features ts 2>&1 | tail -10
```

Check if ts-rs generates `.ts` files. If it does, note the output location.

- [ ] **Step 2: Add ts-rs export test to each crate**

In each `types/src/lib.rs`, add a test that exports TypeScript bindings:

```rust
#[cfg(test)]
#[cfg(feature = "ts")]
mod ts_tests {
    use super::*;
    use ts_rs::TS;
    
    #[test]
    fn export_typescript_bindings() {
        // ts-rs generates files when derive(TS) types are referenced
        CreateHumanInput::export_all().unwrap();
        Human::export_all().unwrap();
        HumanOutput::export_all().unwrap();
    }
}
```

- [ ] **Step 3: Verify TypeScript output is valid**

Check the generated TypeScript files. Ensure:
1. Field names are correct (ts-rs respects `serde(rename)`)
2. Types are correct (ActionHash → string or Uint8Array)
3. Optional fields are `T | null` or `T | undefined`

- [ ] **Step 4: Decide output location**

The generated TypeScript could go to:
- `elohim/sdk/domains/{domain}/types/generated/` (next to the Rust crate)
- `elohim/sdk/storage-client-ts/src/generated/` (alongside existing View types)
- `app/elohim-app/src/app/{domain}/generated/` (directly into Angular)

Read the existing codegen scripts (`elohim/sdk/domains/{domain}/scripts/codegen.mjs`) to understand how domain types flow to consumers. Follow the established pattern.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/*/types/
git commit -m "feat(sdk): enable TypeScript generation from domain wire types crates

ts-rs feature generates TypeScript bindings from the same Rust types
that the compiler enforces between zomes and doorway."
```

---

## Execution Order

```
Task 1 (fix deserialization) → Task 2 (verify round-trip) → Task 3 (audit views) → Task 4 (TS generation)
```

Task 1 is the **critical path** — it unblocks the seeder. Tasks 2-4 are quality/cohesion improvements.

**Important dependency:** Task 1 Step 1 requires studying the conductor's actual response format. If the diagnostic logging from the previous commit has produced log output from alpha, read those logs first — they'll tell us exactly what field names the conductor uses.
