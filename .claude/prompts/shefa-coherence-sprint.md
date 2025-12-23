# Shefa Coherence Sprint - Handoff Prompt

## Context

Shefa is the economic/financial module of the Elohim Protocol - it handles transaction import, REA (Resource-Event-Agent) economic modeling, insurance mutuals, and family/community protection. The goal is to couple **information + currency(values) + limits(wealth/responsibility) + feedback(voting/reporting)** per Schmachtenberger's design principles.

## Current State

The Holochain coordinator zome has **22 build errors** in the Shefa transaction import code (lines 11763+ in `content_store/src/lib.rs`). These are blocking the build.

### Error Categories

1. **Hash type mismatches** (most common):
   ```
   HoloHash<AnyLinkable>: From<std::string::String>` is not satisfied
   HoloHash<AnyDht>: From<HoloHash<AnyLinkable>>` is not satisfied
   ```
   - Problem: Code is trying to convert strings directly to hashes, or mixing hash types
   - Fix: Use proper anchor patterns or `hash_entry()` calls

2. **EntryTypes serialization**:
   ```
   EntryTypes: std::convert::TryFrom<hdk::prelude::SerializedBytes>` is not satisfied
   ```
   - Problem: Trying to deserialize into the wrong entry type
   - Fix: Check the `#[hdk_entry_helper]` derives and use correct deserialization patterns

3. **Missing field**:
   ```
   no field `id` on type `StagedTransaction`
   ```
   - Problem: Code references a field that doesn't exist on the struct
   - Fix: Check `StagedTransaction` definition in integrity zome and update code

4. **Type annotations needed**:
   - Problem: Rust can't infer types in some expressions
   - Fix: Add explicit type annotations

## Files to Review

### Holochain (Rust)
| File | Purpose |
|------|---------|
| `holochain/dna/lamad-spike/zomes/content_store/src/lib.rs` | Coordinator zome - errors at lines 11763+ |
| `holochain/dna/lamad-spike/zomes/content_store_integrity/src/lib.rs` | Entry types & LinkTypes definitions |

### Angular Frontend
| File | Purpose |
|------|---------|
| `elohim-app/src/app/shefa/services/transaction-import.service.ts` | Transaction import UI service |
| `elohim-app/src/app/shefa/services/plaid-integration.service.ts` | Plaid bank connection |
| `elohim-app/src/app/shefa/models/transaction-import.model.ts` | Transaction models |
| `elohim-app/src/app/shefa/services/insurance-mutual.service.ts` | Mutual insurance logic |
| `elohim-app/src/app/shefa/services/family-community-protection.service.ts` | Protection circles |

## Design Principles (Schmachtenberger Alignment)

### Core Questions to Ask
1. **Is this a network signal?** → Keep in Holochain as LinkType/Entry
2. **Is this personal convenience?** → Consider making it a local query or frontend computation
3. **Does it enable trust/coordination?** → Essential, keep it
4. **Is it just filtering/sorting?** → Candidate for removal/simplification

### Shefa-Specific Design
- **Transaction Import**: Plaid → Staged → Categorized → Committed
  - Staged transactions are temporary (could be local-only)
  - Committed transactions become REA EconomicEvents (network signal)

- **Insurance Mutual**: Risk pooling across family/community
  - Risk profiles, coverage policies, claims = network signals
  - Personal claim status filtering = could be queries

- **Family Protection**: Multi-tier custody and backup
  - HumanRelationship with intimacy levels = network trust topology
  - Auto-custody settings = personal convenience (could be local)

## Sprint Approach

### Phase 1: Fix Build Errors (Immediate)
1. Read lines 11763-11900 of `content_store/src/lib.rs`
2. Identify the function(s) with errors
3. Check corresponding entry types in integrity zome
4. Fix hash conversion patterns
5. Fix missing field references
6. Build and verify: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown`

### Phase 2: Coherence Review
1. Map Shefa entry types to their purpose (network signal vs personal)
2. Identify any that should be simplified/removed
3. Check if frontend services match the Holochain API
4. Ensure models are in sync between frontend and backend

### Phase 3: Test and Document
1. Run any existing tests
2. Document the Shefa data flow
3. Update any outdated comments

## Build Commands

```bash
cd /projects/elohim/holochain/dna/lamad-spike

# Check only (faster)
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --target wasm32-unknown-unknown

# Full build
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --target wasm32-unknown-unknown

# Pack DNA after successful build
hc dna pack .
```

## Current LinkTypes Constraint

The LinkTypes enum is at exactly **255 variants** (the Holochain limit). If you need to add new LinkTypes, you must remove unused ones first. Check for unused types with:

```bash
# Find LinkTypes defined but not used in coordinator
grep -E "^    [A-Z][a-zA-Z]+," zomes/content_store_integrity/src/lib.rs | sed 's/,.*//' | while read type; do
  if ! grep -q "LinkTypes::$type" zomes/content_store/src/lib.rs; then
    echo "Unused: $type"
  fi
done
```

## Success Criteria

1. ✅ Build passes with 0 errors
2. ✅ Shefa transaction import functions compile
3. ✅ Entry types match between integrity and coordinator
4. ✅ Frontend models align with Holochain types
5. ✅ Code is self-documenting (no external docs needed)
