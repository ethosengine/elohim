# Seeder Registry Coherence & Relationship Idempotency — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `genesis/orchestrator/data/deployments.json` the single source of truth for "which humans exist on the cluster" — consumed by both the Jenkinsfile and the seeder — and make account imports idempotent so Adam↔Eve (and every rerun) converges instead of crashing on UNIQUE constraint.

**Architecture:** A small registry-loader module on the seeder side filters packages before import; a bidirectional-aware pre-check on the storage side dedupes relationships at the account-import layer without changing the directional `human_relationships` unique index. A2O scenarios execute the three-way agreement (registry ↔ seeder ↔ storage) so drift fails loudly.

**Tech Stack:** TypeScript (seeder, vitest), Rust (elohim-storage, diesel, ts-rs), Gherkin/Cucumber (a2o).

**Spec:** `genesis/docs/superpowers/specs/2026-04-18-seeder-registry-coherence-design.md`

---

## File Structure

**New files:**
- `genesis/seeder/src/deployment-registry.ts` — registry loader (flag → env → file precedence)
- `genesis/seeder/src/__tests__/deployment-registry.test.ts` — unit tests
- `genesis/a2o/features/deployment/seeder-registry-coherence.feature` — coherence scenarios
- `genesis/a2o/features/content/relationship-idempotency.feature` — idempotency + Adam-Eve regression
- `genesis/a2o/steps/seeder.steps.ts` — step definitions for the two new features

**Modified files:**
- `genesis/seeder/src/seed-accounts.ts` — consume registry, partition `{toSeed, staged}`, new output line
- `genesis/seeder/src/__tests__/seed-accounts.test.ts` — tests for partition behavior
- `elohim/elohim-storage/src/db/human_relationships.rs` — add `find_existing_relationship`
- `elohim/elohim-storage/src/views.rs:2107-2114` — add `relationships_skipped` to `AccountImportResultView`
- `elohim/elohim-storage/src/http.rs:5460-5502` — rewrite relationship loop with pre-check + UNIQUE-catch
- `elohim/elohim-storage/tests/` — new integration test for Adam-Eve idempotency
- `genesis/a2o/features/deployment/human-device-mapping.feature:91` — remove `@wip` after steps pass

---

## Implementation Notes

- `AccountImportResultView` does **not** have a JSON schema file under `elohim/sdk/schemas/v1/views/` (verified: no `account-import-result.schema.json` exists). The ts-rs `#[derive(TS)]` attribute handles TypeScript generation; no schema-contract update is needed. The spec mentioned schema — it was wrong. Just add the Rust field, regenerate bindings.
- `do_account_import` creates relationships with `party_a = human_id` (the importer), `party_b = target_id`. When Adam's package declares Eve as target, row is `(adam, eve, spouse)`. When Eve's package declares Adam, row would be `(eve, adam, spouse)`. The directional index allows both; bidirectional-aware dedupe at import time prevents both.
- Seeder tests use vitest: `pnpm --filter @elohim/seeder exec vitest run` (check seeder's package.json name). Tests live in `genesis/seeder/src/__tests__/`.
- Storage tests: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test <name>`.

---

## Task 1: Add `find_existing_relationship` helper (Rust, TDD)

**Files:**
- Modify: `elohim/elohim-storage/src/db/human_relationships.rs`

- [ ] **Step 1: Write the failing unit test**

Append to `elohim/elohim-storage/src/db/human_relationships.rs` (or add to existing `#[cfg(test)] mod tests` block if present):

```rust
#[cfg(test)]
mod find_existing_tests {
    use super::*;
    use crate::db::test_helpers::setup_test_db;  // existing helper in the crate

    fn ctx() -> AppContext {
        AppContext { h_app_id: "test-app".to_string() }
    }

    fn insert_rel(conn: &mut SqliteConnection, a: &str, b: &str, t: &str) {
        create_human_relationship(conn, &ctx(), CreateHumanRelationshipInput {
            id: None,
            party_a_id: a.to_string(),
            party_b_id: b.to_string(),
            relationship_type: t.to_string(),
            intimacy_level: "acquaintance".to_string(),
            is_bidirectional: true,
            consent_given_by_a: true,
            consent_given_by_b: false,
            initiated_by: a.to_string(),
            governance_layer: None,
            reach: "private".to_string(),
            context_json: None,
            expires_at: None,
        }).unwrap();
    }

    #[test]
    fn finds_directional_match() {
        let mut conn = setup_test_db();
        insert_rel(&mut conn, "adam", "eve", "spouse");
        let found = find_existing_relationship(&mut conn, &ctx(), "adam", "eve", "spouse", false).unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn directional_lookup_ignores_reverse() {
        let mut conn = setup_test_db();
        insert_rel(&mut conn, "adam", "eve", "spouse");
        let found = find_existing_relationship(&mut conn, &ctx(), "eve", "adam", "spouse", false).unwrap();
        assert!(found.is_none(), "directional lookup must not match reversed pair");
    }

    #[test]
    fn bidirectional_lookup_matches_reverse() {
        let mut conn = setup_test_db();
        insert_rel(&mut conn, "adam", "eve", "spouse");
        let found = find_existing_relationship(&mut conn, &ctx(), "eve", "adam", "spouse", true).unwrap();
        assert!(found.is_some(), "bidirectional lookup must match reversed pair");
    }

    #[test]
    fn different_relationship_type_does_not_match() {
        let mut conn = setup_test_db();
        insert_rel(&mut conn, "adam", "eve", "spouse");
        let found = find_existing_relationship(&mut conn, &ctx(), "adam", "eve", "sibling", true).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn different_app_does_not_match() {
        let mut conn = setup_test_db();
        insert_rel(&mut conn, "adam", "eve", "spouse");
        let other = AppContext { h_app_id: "other-app".to_string() };
        let found = find_existing_relationship(&mut conn, &other, "adam", "eve", "spouse", true).unwrap();
        assert!(found.is_none());
    }
}
```

If `setup_test_db` / `test_helpers` doesn't exist under that exact name, find the crate's existing in-memory SQLite test setup (look for `:memory:` in `src/db/*.rs` tests) and use that. Do NOT create a new test harness.

- [ ] **Step 2: Run tests and verify they fail**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib find_existing_tests 2>&1 | tail -20
```

Expected: compile error "cannot find function `find_existing_relationship`" OR "unresolved import".

- [ ] **Step 3: Implement `find_existing_relationship`**

Add to `elohim/elohim-storage/src/db/human_relationships.rs`, after the existing Read Operations section, before Write Operations:

```rust
/// Look up an existing relationship between two parties, with optional
/// direction-insensitivity. Used by account import to converge when both
/// parties independently author the same symmetric social edge (Adam + Eve
/// both declaring a spouse relationship).
///
/// - `is_bidirectional = false`: checks (party_a, party_b, type) only.
/// - `is_bidirectional = true`: checks (party_a, party_b, type) OR (party_b, party_a, type).
pub fn find_existing_relationship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    party_a: &str,
    party_b: &str,
    relationship_type: &str,
    is_bidirectional: bool,
) -> Result<Option<HumanRelationship>, StorageError> {
    use crate::db::diesel_schema::human_relationships;
    use diesel::prelude::*;

    let mut query = human_relationships::table
        .filter(human_relationships::h_app_id.eq(&ctx.h_app_id))
        .filter(human_relationships::relationship_type.eq(relationship_type))
        .into_boxed();

    if is_bidirectional {
        query = query.filter(
            (human_relationships::party_a_id.eq(party_a)
                .and(human_relationships::party_b_id.eq(party_b)))
            .or(human_relationships::party_a_id.eq(party_b)
                .and(human_relationships::party_b_id.eq(party_a))),
        );
    } else {
        query = query
            .filter(human_relationships::party_a_id.eq(party_a))
            .filter(human_relationships::party_b_id.eq(party_b));
    }

    query
        .first::<HumanRelationship>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}
```

- [ ] **Step 4: Run tests and verify they pass**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib find_existing_tests 2>&1 | tail -20
```

Expected: `test result: ok. 5 passed`.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/db/human_relationships.rs
git commit -m "$(cat <<'EOF'
feat(storage): add find_existing_relationship with bidirectional lookup

Pre-check helper for account-import idempotency. Directional by default
(matches the unique index semantics); bidirectional mode checks the
reversed pair, needed when both parties author a symmetric social edge.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Add `relationships_skipped` to `AccountImportResultView`

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs:2107-2114`

- [ ] **Step 1: Update the view struct**

Edit `elohim/elohim-storage/src/views.rs` lines 2107-2114. Replace:

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AccountImportResultView {
    pub human_id: String,
    pub content_updated: usize,
    pub relationships_created: usize,
    pub stewardship_created: usize,
    pub collectives_joined: usize,
    pub errors: Vec<String>,
}
```

with:

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AccountImportResultView {
    pub human_id: String,
    pub content_updated: usize,
    pub relationships_created: usize,
    pub relationships_skipped: usize,
    pub stewardship_created: usize,
    pub collectives_joined: usize,
    pub errors: Vec<String>,
}
```

- [ ] **Step 2: Compile**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -20
```

Expected: compile error(s) in `http.rs` — the construction of `AccountImportResultView` is missing the new field. This is expected; Task 3 fixes it.

- [ ] **Step 3: Do not commit yet**

Leave uncommitted; Task 3 will fix the breakage and the two changes commit together.

---

## Task 3: Rewrite the relationship loop in `do_account_import` with idempotent pre-check

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:5460-5502`

- [ ] **Step 1: Find and replace the relationship loop**

Edit `elohim/elohim-storage/src/http.rs`. Near line 5460 you'll find `// Phase 2: Create human relationships`. Replace the entire Phase 2 block (from the comment through the closing `}` after the `info!(...)` call, currently lines 5460-5502) with:

```rust
        // Phase 2: Create human relationships (idempotent)
        //
        // Both Adam and Eve's packages may declare the same symmetric edge
        // (e.g. spouse). The directional unique index on human_relationships
        // would reject the second import; here we pre-check with bidirectional
        // awareness, and catch UNIQUE violations as a race-condition fallback
        // (same pattern as stewardship allocations below).
        let mut relationships_skipped: usize = 0;
        if !package.relationships.is_empty() {
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))?;

            for rel_seed in &package.relationships {
                let existing = human_relationships::find_existing_relationship(
                    &mut conn,
                    &ctx,
                    &human_id,
                    &rel_seed.target_id,
                    &rel_seed.relationship_type,
                    rel_seed.is_bidirectional,
                )?;

                if existing.is_some() {
                    relationships_skipped += 1;
                    continue;
                }

                let input = human_relationships::CreateHumanRelationshipInput {
                    id: None,
                    party_a_id: human_id.clone(),
                    party_b_id: rel_seed.target_id.clone(),
                    relationship_type: rel_seed.relationship_type.clone(),
                    intimacy_level: rel_seed.intimacy_level.clone(),
                    is_bidirectional: rel_seed.is_bidirectional,
                    consent_given_by_a: true,
                    consent_given_by_b: false,
                    initiated_by: human_id.clone(),
                    governance_layer: None,
                    reach: rel_seed
                        .reach
                        .clone()
                        .unwrap_or_else(|| "private".to_string()),
                    context_json: None,
                    expires_at: None,
                };

                match human_relationships::create_human_relationship(&mut conn, &ctx, input) {
                    Ok(_) => relationships_created += 1,
                    Err(StorageError::Internal(msg)) if msg.contains("UNIQUE constraint") => {
                        // Race: another concurrent import inserted between
                        // our pre-check and our insert. Same-shape row —
                        // treat as skipped, not an error.
                        relationships_skipped += 1;
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Failed to create relationship {} -> {}: {}",
                            human_id, rel_seed.target_id, e
                        ));
                    }
                }
            }

            info!(
                human_id = %human_id,
                relationships_created = relationships_created,
                relationships_skipped = relationships_skipped,
                "Human relationship creation complete"
            );
        }
```

- [ ] **Step 2: Add the new field to the result construction**

Still in `http.rs`, find the place where `AccountImportResultView` is constructed and returned from `do_account_import` (search for `AccountImportResultView {` in the file — it should be near the end of the function, after all four phases). Add `relationships_skipped,` between `relationships_created,` and `stewardship_created,`.

If the field was being constructed via a long-form `AccountImportResultView { human_id: ..., content_updated: ..., }` block, match the exact style and add the line. Use `cargo check` output from Task 2 to pinpoint the exact line number if needed.

- [ ] **Step 3: Compile**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check 2>&1 | tail -20
```

Expected: clean compile, 0 errors.

- [ ] **Step 4: Commit Task 2 + Task 3 together**

```bash
cd /projects/elohim
git add elohim/elohim-storage/src/views.rs elohim/elohim-storage/src/http.rs
git commit -m "$(cat <<'EOF'
feat(storage): idempotent relationship import with bidirectional dedupe

Pre-check existing relationships before insert; treat UNIQUE violations
as skipped (race-condition fallback, matches stewardship pattern).
Both parties' packages can now author a symmetric spouse edge without
the second import crashing on the directional unique index.

Adds relationships_skipped to AccountImportResultView so the seeder
can surface convergence in its output.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Storage integration test — Adam-Eve regression

**Files:**
- Create: `elohim/elohim-storage/tests/account_import_idempotency.rs` (or append to existing account-import integration test if one exists — check `elohim/elohim-storage/tests/` first)

- [ ] **Step 1: Check for existing account-import integration test**

```bash
ls elohim/elohim-storage/tests/ | grep -i account
```

If one exists, append to it. Otherwise create a new file.

- [ ] **Step 2: Write the integration test**

Create `elohim/elohim-storage/tests/account_import_idempotency.rs`:

```rust
//! Integration test: Adam-Eve bidirectional relationship authorship converges.
//!
//! Regression guard for 2026-04-17 seed pipeline failure where the directional
//! human_relationships unique index rejected the second import with
//! "UNIQUE constraint failed: human_relationships.h_app_id, party_a_id,
//! party_b_id, relationship_type".

use elohim_storage::views::{AccountPackageInputView, RelationshipSeedView, AccountIdentityView};
// NOTE: Use whatever in-process import helper the crate exposes. If
// `do_account_import` is private, call the HTTP handler via axum::Router test
// harness (see existing tests in this directory for the pattern).

#[tokio::test]
async fn adam_eve_bidirectional_spouse_converges() {
    let ctx = setup_test_context().await;  // reuse existing test harness

    let adam_pkg = make_package("human-adam-firstman", vec![
        rel("human-eve-firstwoman", "spouse", true),
    ]);
    let eve_pkg = make_package("human-eve-firstwoman", vec![
        rel("human-adam-firstman", "spouse", true),
    ]);

    let adam_result = import_package(&ctx, adam_pkg).await.unwrap();
    assert_eq!(adam_result.relationships_created, 1);
    assert_eq!(adam_result.relationships_skipped, 0);
    assert!(adam_result.errors.is_empty(), "adam errors: {:?}", adam_result.errors);

    let eve_result = import_package(&ctx, eve_pkg).await.unwrap();
    assert_eq!(eve_result.relationships_created, 0);
    assert_eq!(eve_result.relationships_skipped, 1);
    assert!(eve_result.errors.is_empty(), "eve errors: {:?}", eve_result.errors);

    // Exactly one row in the database
    let count = count_relationships(&ctx).await;
    assert_eq!(count, 1, "expected exactly one human_relationships row for Adam-Eve");
}

#[tokio::test]
async fn rerunning_import_is_idempotent() {
    let ctx = setup_test_context().await;
    let pkg = make_package("human-adam-firstman", vec![
        rel("human-eve-firstwoman", "spouse", true),
    ]);

    let first = import_package(&ctx, pkg.clone()).await.unwrap();
    assert_eq!(first.relationships_created, 1);

    let second = import_package(&ctx, pkg).await.unwrap();
    assert_eq!(second.relationships_created, 0);
    assert_eq!(second.relationships_skipped, 1);
    assert!(second.errors.is_empty());
}

// --- helpers ---

fn rel(target: &str, rel_type: &str, bidirectional: bool) -> RelationshipSeedView {
    RelationshipSeedView {
        target_id: target.to_string(),
        relationship_type: rel_type.to_string(),
        intimacy_level: "acquaintance".to_string(),
        is_bidirectional: bidirectional,
        reach: Some("private".to_string()),
    }
}

fn make_package(human_id: &str, relationships: Vec<RelationshipSeedView>) -> AccountPackageInputView {
    AccountPackageInputView {
        schema_version: 1,
        identity: AccountIdentityView {
            human_id: human_id.to_string(),
            display_name: human_id.to_string(),
            // Fill required fields using whatever defaults the existing
            // integration tests use. Consult an existing test for the pattern.
            ..Default::default()
        },
        content: vec![],
        relationships,
        stewardship: vec![],
        collectives: vec![],
        conductor_group: None,
        manifest: None,
    }
}

// setup_test_context, import_package, count_relationships: reuse existing
// patterns from other integration tests in this directory. If no such
// helpers exist, look at tests/*.rs for an axum Router-based test that POSTs
// to /account/import and adapt.
```

If the `RelationshipSeedView` / `AccountIdentityView` fields differ from what's shown above, match them exactly by reading `views.rs` first.

- [ ] **Step 3: Run the test**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test account_import_idempotency 2>&1 | tail -30
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add elohim/elohim-storage/tests/account_import_idempotency.rs
git commit -m "$(cat <<'EOF'
test(storage): Adam-Eve bidirectional import regression + rerun idempotency

Covers the 2026-04-17 seed pipeline failure: two parties authoring the
same symmetric spouse edge must produce exactly one row.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Regenerate TypeScript bindings

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/generated/AccountImportResultView.ts` (auto-generated)

- [ ] **Step 1: Regenerate**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -10
```

Expected: binding tests pass; `AccountImportResultView.ts` updated with `relationshipsSkipped: number`.

- [ ] **Step 2: Build the storage-client TypeScript package**

```bash
cd /projects/elohim
pnpm --filter @elohim/storage-client build 2>&1 | tail -20
```

Expected: clean build.

- [ ] **Step 3: Verify the field made it**

```bash
grep -A2 "relationshipsSkipped" elohim/sdk/storage-client-ts/src/generated/AccountImportResultView.ts
```

Expected: line matches `relationshipsSkipped: number,` (or similar).

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add elohim/sdk/storage-client-ts/src/generated/AccountImportResultView.ts
# Also commit any other regenerated files the build updated:
git add -u elohim/sdk/storage-client-ts/
git commit -m "$(cat <<'EOF'
chore(storage-client): regenerate bindings for relationshipsSkipped

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Seeder — `deployment-registry.ts` module (TDD)

**Files:**
- Create: `genesis/seeder/src/deployment-registry.ts`
- Create: `genesis/seeder/src/__tests__/deployment-registry.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `genesis/seeder/src/__tests__/deployment-registry.test.ts`:

```typescript
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { loadDeploymentRegistry } from '../deployment-registry.js';

describe('loadDeploymentRegistry', () => {
  let tmpDir: string;
  let registryPath: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'dep-reg-'));
    registryPath = join(tmpDir, 'deployments.json');
    writeFileSync(registryPath, JSON.stringify({
      humans: [
        { humanId: 'human-adam-firstman', name: 'adam' },
        { humanId: 'human-matthew-manager', name: 'matthew' },
      ],
    }));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it('reads humanIds from a registry file', () => {
    const reg = loadDeploymentRegistry({ registryPath });
    expect(reg.source).toBe('file');
    expect(reg.path).toBe(registryPath);
    expect(reg.deployedHumanIds.has('human-adam-firstman')).toBe(true);
    expect(reg.deployedHumanIds.has('human-matthew-manager')).toBe(true);
    expect(reg.deployedHumanIds.size).toBe(2);
  });

  it('explicit deployedHumans list overrides registryPath', () => {
    const reg = loadDeploymentRegistry({
      registryPath,
      deployedHumans: ['human-eve-firstwoman'],
    });
    expect(reg.source).toBe('flag');
    expect(reg.deployedHumanIds.has('human-eve-firstwoman')).toBe(true);
    expect(reg.deployedHumanIds.has('human-adam-firstman')).toBe(false);
  });

  it('rejects short names in deployedHumans with a clear error', () => {
    expect(() =>
      loadDeploymentRegistry({ deployedHumans: ['adam'] })
    ).toThrow(/full humanId/i);
  });

  it('throws if registryPath does not exist', () => {
    expect(() =>
      loadDeploymentRegistry({ registryPath: '/nonexistent/path.json' })
    ).toThrow(/deployment registry/i);
  });

  it('throws if registry JSON has no humans array', () => {
    writeFileSync(registryPath, JSON.stringify({}));
    expect(() =>
      loadDeploymentRegistry({ registryPath })
    ).toThrow(/humans/i);
  });
});
```

- [ ] **Step 2: Run tests and verify they fail**

```bash
cd genesis/seeder
pnpm exec vitest run src/__tests__/deployment-registry.test.ts 2>&1 | tail -20
```

Expected: FAIL with "Cannot find module '../deployment-registry.js'".

- [ ] **Step 3: Implement the module**

Create `genesis/seeder/src/deployment-registry.ts`:

```typescript
/**
 * Deployment Registry Loader
 *
 * Single source of truth for "which humans have StatefulSets provisioned
 * on the cluster right now." Consumed by the seeder to filter packages;
 * also the file the elohim-edge Jenkinsfile stage reads for provisioning.
 *
 * Resolution order:
 *   1. opts.deployedHumans (explicit list, from --deployed-humans flag
 *      or SEEDER_DEPLOYED_HUMANS env)
 *   2. opts.registryPath (from --registry flag or SEEDER_REGISTRY env)
 *   3. default: genesis/orchestrator/data/deployments.json relative to
 *      this module
 */

import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

export interface DeploymentRegistry {
  deployedHumanIds: Set<string>;
  source: 'file' | 'flag' | 'env';
  path?: string;
}

export interface LoadRegistryOptions {
  registryPath?: string;
  deployedHumans?: string[];
}

interface RegistryFile {
  humans?: Array<{ humanId: string; name?: string }>;
}

const DEFAULT_REGISTRY_RELATIVE = '../../orchestrator/data/deployments.json';

export function loadDeploymentRegistry(opts: LoadRegistryOptions = {}): DeploymentRegistry {
  // 1. Explicit list
  if (opts.deployedHumans && opts.deployedHumans.length > 0) {
    for (const id of opts.deployedHumans) {
      if (!id.startsWith('human-')) {
        throw new Error(
          `Invalid --deployed-humans entry "${id}": expected full humanId ` +
          `(e.g. "human-adam-firstman"), not a short name. This keeps the ` +
          `contract unambiguous and avoids a circular dependency on the registry file.`
        );
      }
    }
    return {
      deployedHumanIds: new Set(opts.deployedHumans),
      source: 'flag',
    };
  }

  // 2. & 3. File path (explicit or default)
  const __dirname = dirname(fileURLToPath(import.meta.url));
  const path = opts.registryPath ?? resolve(__dirname, DEFAULT_REGISTRY_RELATIVE);

  if (!existsSync(path)) {
    throw new Error(`Deployment registry not found: ${path}`);
  }

  let parsed: RegistryFile;
  try {
    parsed = JSON.parse(readFileSync(path, 'utf-8'));
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    throw new Error(`Failed to parse deployment registry ${path}: ${msg}`);
  }

  if (!Array.isArray(parsed.humans)) {
    throw new Error(`Deployment registry ${path} has no "humans" array`);
  }

  const deployedHumanIds = new Set<string>();
  for (const h of parsed.humans) {
    if (h && typeof h.humanId === 'string') {
      deployedHumanIds.add(h.humanId);
    }
  }

  return {
    deployedHumanIds,
    source: opts.registryPath ? 'file' : 'file',  // distinguished by `path` field
    path,
  };
}

/**
 * Parse --deployed-humans value (comma-separated) into an array.
 * Empty/undefined returns undefined (so loadDeploymentRegistry falls through).
 */
export function parseDeployedHumansArg(raw: string | undefined): string[] | undefined {
  if (!raw) return undefined;
  const list = raw.split(',').map(s => s.trim()).filter(Boolean);
  return list.length > 0 ? list : undefined;
}
```

- [ ] **Step 4: Run tests and verify they pass**

```bash
cd genesis/seeder
pnpm exec vitest run src/__tests__/deployment-registry.test.ts 2>&1 | tail -20
```

Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/seeder/src/deployment-registry.ts genesis/seeder/src/__tests__/deployment-registry.test.ts
git commit -m "$(cat <<'EOF'
feat(seeder): deployment-registry loader (flag → env → file precedence)

Consumes genesis/orchestrator/data/deployments.json — the same file the
elohim-edge Jenkinsfile stage reads for StatefulSet provisioning —
making it the single source of truth for what's deployed on the cluster.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Seeder — integrate registry filtering into `seed-accounts.ts`

**Files:**
- Modify: `genesis/seeder/src/seed-accounts.ts`
- Modify: `genesis/seeder/src/__tests__/seed-accounts.test.ts`

- [ ] **Step 1: Write failing tests for partition behavior**

Append to `genesis/seeder/src/__tests__/seed-accounts.test.ts`:

```typescript
import { partitionByRegistry } from '../seed-accounts.js';
import type { DeploymentRegistry } from '../deployment-registry.js';

describe('partitionByRegistry', () => {
  function mkPkg(humanId: string): AccountPackageInputView {
    return {
      identity: { humanId, displayName: humanId } as AccountPackageInputView['identity'],
      content: [], relationships: [], stewardship: [], collectives: [],
    } as unknown as AccountPackageInputView;
  }

  function mkRegistry(ids: string[]): DeploymentRegistry {
    return { deployedHumanIds: new Set(ids), source: 'file' };
  }

  it('returns deployed packages in toSeed and others in staged', () => {
    const pkgs = [
      mkPkg('human-adam-firstman'),
      mkPkg('human-eve-firstwoman'),
      mkPkg('human-matthew-manager'),
    ];
    const reg = mkRegistry(['human-adam-firstman', 'human-matthew-manager']);

    const { toSeed, staged } = partitionByRegistry(pkgs, reg);

    expect(toSeed.map(p => p.identity.humanId)).toEqual([
      'human-adam-firstman', 'human-matthew-manager',
    ]);
    expect(staged.map(p => p.identity.humanId)).toEqual(['human-eve-firstwoman']);
  });

  it('stages all packages when registry is empty', () => {
    const pkgs = [mkPkg('human-adam-firstman')];
    const { toSeed, staged } = partitionByRegistry(pkgs, mkRegistry([]));
    expect(toSeed).toHaveLength(0);
    expect(staged).toHaveLength(1);
  });

  it('reports registry entries that have no package as an orphan list', () => {
    const pkgs = [mkPkg('human-adam-firstman')];
    const reg = mkRegistry(['human-adam-firstman', 'human-ghost']);
    const { orphanRegistryEntries } = partitionByRegistry(pkgs, reg);
    expect(orphanRegistryEntries).toEqual(['human-ghost']);
  });
});
```

- [ ] **Step 2: Run tests and verify they fail**

```bash
cd genesis/seeder
pnpm exec vitest run src/__tests__/seed-accounts.test.ts 2>&1 | tail -20
```

Expected: FAIL on `partitionByRegistry` import.

- [ ] **Step 3: Add `partitionByRegistry` export to `seed-accounts.ts`**

Edit `genesis/seeder/src/seed-accounts.ts`. After the `loadPackages` function (around line 59), add:

```typescript
// =============================================================================
// Registry-based partitioning
// =============================================================================

import type { DeploymentRegistry } from './deployment-registry.js';

export interface PartitionResult {
  toSeed: AccountPackageInputView[];
  staged: AccountPackageInputView[];
  orphanRegistryEntries: string[];
}

export function partitionByRegistry(
  packages: AccountPackageInputView[],
  registry: DeploymentRegistry,
): PartitionResult {
  const toSeed: AccountPackageInputView[] = [];
  const staged: AccountPackageInputView[] = [];
  const packageIds = new Set<string>();

  for (const pkg of packages) {
    packageIds.add(pkg.identity.humanId);
    if (registry.deployedHumanIds.has(pkg.identity.humanId)) {
      toSeed.push(pkg);
    } else {
      staged.push(pkg);
    }
  }

  const orphanRegistryEntries: string[] = [];
  for (const id of registry.deployedHumanIds) {
    if (!packageIds.has(id)) {
      orphanRegistryEntries.push(id);
    }
  }

  return { toSeed, staged, orphanRegistryEntries };
}
```

Move the `import type { DeploymentRegistry }` line to the top of the file with the other imports.

- [ ] **Step 4: Run tests and verify the new tests pass**

```bash
cd genesis/seeder
pnpm exec vitest run src/__tests__/seed-accounts.test.ts 2>&1 | tail -20
```

Expected: all tests pass, including the 3 new `partitionByRegistry` cases.

- [ ] **Step 5: Wire registry loading + partitioning into `main()`**

Edit the `main()` function in `genesis/seeder/src/seed-accounts.ts` (around line 173). Add arg parsing and registry loading *before* `loadPackages` is called:

At the top of `main()`, add imports:

```typescript
import { loadDeploymentRegistry, parseDeployedHumansArg } from './deployment-registry.js';
```

(Move to the top imports section.)

Inside `main()`, after the existing arg parsing block, add:

```typescript
  const registryPathArg = args.find(a => a.startsWith('--registry='))?.split('=')[1]
    ?? (args.includes('--registry') ? args[args.indexOf('--registry') + 1] : undefined);
  const deployedHumansArg = args.find(a => a.startsWith('--deployed-humans='))?.split('=')[1]
    ?? (args.includes('--deployed-humans') ? args[args.indexOf('--deployed-humans') + 1] : undefined);

  const registry = loadDeploymentRegistry({
    registryPath: registryPathArg ?? process.env.SEEDER_REGISTRY,
    deployedHumans: parseDeployedHumansArg(
      deployedHumansArg ?? process.env.SEEDER_DEPLOYED_HUMANS,
    ),
  });
```

Then replace the existing:

```typescript
  const packages = loadPackages(packagesDir, humanFilter);
  console.log(`Found ${packages.length} account packages\n`);
```

with:

```typescript
  const allPackages = loadPackages(packagesDir, humanFilter);
  const { toSeed: packages, staged, orphanRegistryEntries } = partitionByRegistry(allPackages, registry);

  console.log(
    `Registry:  ${registry.path ?? `(flag: ${registry.deployedHumanIds.size} humans)`} ` +
    `(${registry.deployedHumanIds.size} deployed humans)`,
  );
  console.log(
    `Packages:  ${packagesDir} (${allPackages.length} found, ${packages.length} deployed, ${staged.length} staged)\n`,
  );

  for (const pkg of staged) {
    console.log(`  [-] ${pkg.identity.displayName.padEnd(18)} (staged — not in deployment registry)`);
  }
  if (staged.length > 0) console.log('');

  for (const orphanId of orphanRegistryEntries) {
    console.warn(`  WARNING: registry references ${orphanId}, no package found`);
  }
  if (orphanRegistryEntries.length > 0) console.log('');
```

Then update the summary section. Find:

```typescript
  console.log(`=== Results: ${imported} imported, ${failed} failed ===`);
```

Change to:

```typescript
  console.log(`=== Results: ${imported} imported, ${failed} failed, ${staged.length} staged ===`);
```

Update the per-package log line format (around line 240-243) to include `skipped`:

```typescript
    console.log(
      `  [${icon}] ${result.displayName.padEnd(18)} -> ${result.targetUrl}${retried} ` +
      `content=${result.contentUpdated} rels=${result.relationshipsCreated} skipped=${result.relationshipsSkipped ?? 0} stew=${result.stewardshipCreated} coll=${result.collectivesJoined}${warnings}`
    );
```

Also add `relationshipsSkipped` tracking to the `SeedResult` interface (around line 67) and populate it from `outcome.result.relationshipsSkipped` in `importPackage` (around line 147):

In `SeedResult`:
```typescript
  relationshipsSkipped: number;
```

In the success return of `importPackage`:
```typescript
        relationshipsSkipped: outcome.result.relationshipsSkipped ?? 0,
```

In the failure return (around line 161):
```typescript
    relationshipsSkipped: 0,
```

- [ ] **Step 6: Compile & type-check**

```bash
cd genesis/seeder
pnpm exec tsc --noEmit 2>&1 | tail -20
```

Expected: 0 errors.

- [ ] **Step 7: Run all seeder tests**

```bash
cd genesis/seeder
pnpm exec vitest run src/__tests__/ 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 8: Smoke-test dry-run against the real registry**

```bash
cd genesis/seeder
npx tsx src/seed-accounts.ts --dry-run 2>&1 | head -50
```

Expected output includes:
- `Registry:  .../deployments.json (6 deployed humans)`
- `Packages:  .../account-packages (33 found, 6 deployed, 27 staged)`
- 27 `[-]` lines before the 6 `[DRY]` lines
- `exit 0`

- [ ] **Step 9: Commit**

```bash
cd /projects/elohim
git add genesis/seeder/src/seed-accounts.ts genesis/seeder/src/__tests__/seed-accounts.test.ts
git commit -m "$(cat <<'EOF'
feat(seeder): filter account imports by deployment registry

Seeder now partitions packages into {toSeed, staged} based on
deployments.json — the same file elohim-edge reads for StatefulSet
provisioning. Staged packages are logged as [-], not failed,
eliminating HTTP 502s against nonexistent storage pods.

Surfaces relationshipsSkipped from the import result so reruns
show convergence rather than suppressing the count.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: A2O step definitions for the new features

**Files:**
- Create: `genesis/a2o/steps/seeder.steps.ts`

- [ ] **Step 1: Inspect existing step patterns**

```bash
head -60 /projects/elohim/genesis/a2o/steps/common.steps.ts
ls /projects/elohim/genesis/a2o/steps/ | head -20
```

Note the import style, `E2EWorld` usage, and how existing steps shell out / read JSON / call doorway.

- [ ] **Step 2: Create `seeder.steps.ts`**

Create `genesis/a2o/steps/seeder.steps.ts`. Model it after `common.steps.ts` / `content-lifecycle.steps.ts`. It must implement these step phrases used in `seeder-registry-coherence.feature` and `relationship-idempotency.feature`:

- `Given the deployment registry lists humans "..."` — write a temp `deployments.json` with the given names (short names → `human-<name>-*` lookup via humans.json), store path on world
- `Given the deployment registry lists human "ghost-human"` — same as above, single entry
- `Given account packages exist for "..."` — assert files exist under `genesis/data/account-packages/`; if a test package needs to be synthesized, do it under a temp dir
- `When the seeder runs against "..."` / `When the seeder runs` / `When the seeder runs with "..."` — spawn `npx tsx src/seed-accounts.ts` with `--registry=<tmp>` and the flag args; capture stdout/stderr/exit
- `Then the seeder attempts import for "..."` — parse stdout for `[+]` / `[X]` lines, assert the matching names appear
- `Then the seeder marks "..." as staged` — parse `[-]` lines, assert names appear
- `Then the seeder exits with status 0`
- `Then the seeder emits warning "registry references ..."`
- `Given Adam's account package declares spouse relationship with Eve` / `Given Eve's account package...` — load real packages, assert relationship exists
- `When both packages are imported in sequence` / `When Adam's account package is imported` / `...a second time` — POST to `/account/import`, capture `AccountImportResultView`
- `Then exactly one human_relationships row exists for the pair` — query storage via admin endpoint (check `doorway-app` or `storage-client` for the appropriate read path) OR inspect the import result's `relationshipsCreated` across both calls
- `Then the second import reports relationshipsSkipped=1`
- `Then both imports exit successfully`
- `Then no errors mention "UNIQUE constraint"`
- `Then relationshipsCreated equals N` / `Then relationshipsSkipped equals ...`

Each step implementation should be <20 lines. If the doorway/storage query for "exactly one row" is awkward, fall back to asserting the import-result numbers (created + skipped sums to the relationship count across both imports).

- [ ] **Step 3: Verify step registration**

```bash
cd genesis/a2o
npx tsx scripts/scan-coverage.ts 2>&1 | tail -20
```

Expected: new step phrases appear registered; no "undefined step" warnings for the new scenarios.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/steps/seeder.steps.ts
git commit -m "$(cat <<'EOF'
test(a2o): step definitions for seeder registry + relationship idempotency

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: A2O feature — seeder-registry-coherence

**Files:**
- Create: `genesis/a2o/features/deployment/seeder-registry-coherence.feature`

- [ ] **Step 1: Write the feature file**

Create `genesis/a2o/features/deployment/seeder-registry-coherence.feature`:

```gherkin
@e2e @deployment @seeder @registry-coherence
Feature: Seeder respects the deployment registry
  As a protocol operator running the seed pipeline
  I want the seeder to import only accounts whose humans are deployed
  So that undeployed packages don't produce 502 errors, and the
  deployment registry remains the single source of truth for what
  exists on the cluster right now.

  Scenario: Seeder imports only deployed humans
    Given the deployment registry lists humans "adam, matthew, frank"
    And account packages exist for "adam, matthew, frank, charlie, eve"
    When the seeder runs against "https://doorway-alpha.elohim.host"
    Then the seeder attempts import for "adam, matthew, frank"
    And the seeder marks "charlie, eve" as staged
    And the seeder exits with status 0

  Scenario: Registry entry without a package is a warning
    Given the deployment registry lists human "ghost-human"
    And no account package exists for "ghost-human"
    When the seeder runs
    Then the seeder emits warning "registry references ghost-human, no package found"
    And the seeder exits with status 0

  Scenario: Seeder is idempotent across reruns
    Given a successful seed run completed
    When the seeder runs a second time with unchanged packages
    Then all deployed humans report outcome "imported"
    And no package reports outcome "failed"

  Scenario: --deployed-humans flag overrides the registry file
    Given the deployment registry lists humans "adam, matthew"
    When the seeder runs with "--deployed-humans=human-adam-firstman"
    Then the seeder attempts import for "adam" only
```

- [ ] **Step 2: Run the scenarios**

```bash
cd genesis/a2o
pnpm run cucumber -- features/deployment/seeder-registry-coherence.feature 2>&1 | tail -30
```

(If the exact cucumber command differs, check `genesis/a2o/package.json` scripts.)

Expected: 4 scenarios pass.

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/features/deployment/seeder-registry-coherence.feature
git commit -m "$(cat <<'EOF'
test(a2o): seeder-registry coherence scenarios

Executable contract: deployments.json is the gate; undeployed packages
are staged, not failed; reruns converge; --deployed-humans flag wins.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: A2O feature — relationship-idempotency

**Files:**
- Create: `genesis/a2o/features/content/relationship-idempotency.feature`

- [ ] **Step 1: Write the feature file**

Create `genesis/a2o/features/content/relationship-idempotency.feature`:

```gherkin
@e2e @content @relationships @idempotency
Feature: Relationship import converges under bidirectional authorship

  Scenario: A spouse relationship authored by both parties is created once
    Given Adam's account package declares spouse relationship with Eve
    And Eve's account package declares spouse relationship with Adam
    When both packages are imported in sequence
    Then exactly one human_relationships row exists for the pair
    And the second import reports relationshipsSkipped=1

  Scenario: Re-importing an account package does not error
    Given Adam's account package has been imported successfully
    When Adam's account package is imported a second time
    Then the import exits successfully
    And relationshipsCreated equals 0
    And relationshipsSkipped equals the number of relationships in the package

  @regression
  Scenario: Adam-Eve UNIQUE constraint does not fail the seed
    # Regression guard for the 2026-04-17 seed pipeline failure where
    # Adam's package created (adam, eve, spouse) and subsequent Eve
    # imports crashed with UNIQUE constraint violation on the
    # human_relationships(h_app_id, party_a, party_b, type) index.
    Given a clean database
    When Adam's account package is imported
    And Eve's account package is imported
    Then both imports exit successfully
    And no errors mention "UNIQUE constraint"
```

- [ ] **Step 2: Run the scenarios**

```bash
cd genesis/a2o
pnpm run cucumber -- features/content/relationship-idempotency.feature 2>&1 | tail -30
```

Expected: 3 scenarios pass.

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/features/content/relationship-idempotency.feature
git commit -m "$(cat <<'EOF'
test(a2o): relationship idempotency + Adam-Eve regression

Regression guard for the 2026-04-17 seed pipeline failure on the
directional human_relationships unique index.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Unblock the `@wip` scenario in human-device-mapping.feature

**Files:**
- Modify: `genesis/a2o/features/deployment/human-device-mapping.feature`

- [ ] **Step 1: Check the step definitions are now implemented**

```bash
grep -B1 -A1 "the registry contains a record for each name" genesis/a2o/steps/*.ts
```

If the step isn't defined anywhere, add it to `genesis/a2o/steps/deployment.steps.ts` (or create if not present — check the file list). Implementation: read `genesis/orchestrator/data/deployments.json`, assert each name in the data table maps to a `humans[].name` entry.

If all 8 scenarios in that file have step definitions (not just this one), remove `@wip` from the whole feature. If only some do, remove `@wip` line-by-line — only from scenarios whose steps pass.

- [ ] **Step 2: Run the scenario**

```bash
cd genesis/a2o
pnpm run cucumber -- features/deployment/human-device-mapping.feature 2>&1 | tail -30
```

- [ ] **Step 3: For the scenario at line 91, remove `@wip`**

Edit `genesis/a2o/features/deployment/human-device-mapping.feature`. Find:

```gherkin
  @wip
  Scenario: The six protocol humans are all represented in the deployment registry
```

Delete the `@wip` line (only if Step 2 showed this specific scenario passing).

- [ ] **Step 4: Re-run to confirm it still passes without @wip**

```bash
cd genesis/a2o
pnpm run cucumber -- features/deployment/human-device-mapping.feature --tags "not @wip" 2>&1 | tail -20
```

Expected: the "six protocol humans" scenario runs and passes.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/features/deployment/human-device-mapping.feature genesis/a2o/steps/deployment.steps.ts 2>/dev/null || git add genesis/a2o/features/deployment/human-device-mapping.feature
git commit -m "$(cat <<'EOF'
test(a2o): unblock 'six protocol humans' deployment-registry scenario

Makes the spiritual anchor of the registry-as-gate contract executable.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: Manual golden-path verification

**Files:** none (verification only)

- [ ] **Step 1: Run seeder against alpha doorway**

```bash
cd genesis/seeder
DOORWAY_URL=https://doorway-alpha.elohim.host npx tsx src/seed-accounts.ts 2>&1 | tail -50
```

Expected:
- Header shows `Packages: ... (33 found, 6 deployed, 27 staged)`
- 6 `[+]` lines for deployed humans, each with `content=N rels=N skipped=0 stew=N coll=N`
- 27 `[-]` lines for staged humans
- `=== Results: 6 imported, 0 failed, 27 staged ===`
- `exit 0`
- No HTTP 502 errors
- No UNIQUE constraint errors

- [ ] **Step 2: Immediate rerun (idempotency check)**

```bash
cd genesis/seeder
DOORWAY_URL=https://doorway-alpha.elohim.host npx tsx src/seed-accounts.ts 2>&1 | tail -20
```

Expected:
- 6 `[+]` lines, each with `skipped=N` > 0 for humans that declared relationships
- 0 failed
- `exit 0`

- [ ] **Step 3: Document results**

If output matches expectations, no further action. If it deviates, debug — do NOT modify the plan; fix the implementation.

---

## Self-Review Notes

- **Spec coverage**: all four components of the spec (registry loader, seeder filtering, relationship idempotency, a2o scenarios) map to tasks 6, 7, 1-5, 8-11 respectively. Manual verification (Task 12) covers the spec's "golden-path" section.
- **Placeholder scan**: no TBDs; all code blocks concrete. Task 4 and Task 8 contain "adapt from existing patterns" instructions because the exact harness/world/helpers already exist in the codebase and copying their shape is more reliable than guessing. The task still names files, expected behavior, and verification commands.
- **Type consistency**: `DeploymentRegistry`, `PartitionResult`, `SeedResult`, `AccountImportResultView.relationshipsSkipped` used consistently across tasks 6, 7, 2, 3, 5.
- **Spec drift fix**: spec claimed `AccountImportResult` has a JSON schema. It doesn't (verified). Plan omits that step and notes this in the Implementation Notes.
