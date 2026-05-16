# Seeder Registry Coherence & Relationship Idempotency

**Date**: 2026-04-18
**Status**: Design approved, awaiting implementation plan
**Related**: `genesis/orchestrator/data/deployments.json`, `genesis/seeder/src/seed-accounts.ts`, `elohim/elohim-storage/src/http.rs`

## Context

The 2026-04-17 `dev` branch seed run against `doorway-alpha.elohim.host` surfaced two coupled failures:

1. **Phantom seed targets** — the seeder iterated all 33 account packages in `genesis/data/account-packages/`, but only 6 humans have StatefulSets provisioned on the cluster (those listed in `deployments.json`). The other 27 packages hash-routed to nonexistent storage pods (`elohim-matthew-alpha` as temporary fallback) and failed with `HTTP 502`.
2. **Adam ↔ Eve UNIQUE constraint** — Adam's account package declared a spouse relationship with Eve; the directional unique index `(h_app_id, party_a_id, party_b_id, relationship_type)` on `human_relationships` rejected a second import attempt with identical `(a, b, type)`, halting relationship creation mid-package.

Both failures share a root shape: **the system lacks a single executable contract that ties together what is deployed, what gets seeded, and what can be re-imported idempotently.** The deployment registry exists, but only the Jenkinsfile consults it; the seeder discovers accounts via `readdirSync`; a2o asserts the intended agreement only via `@wip` scenarios.

## Goals

- Make `deployments.json` the **single source of truth** for "which humans exist on the cluster right now," consulted by both the edge stage and the seeder
- Make account imports **idempotent**: reruns converge, bidirectional authorship creates one row, regression-tested against the Adam-Eve case
- Make the **agreement executable** via a2o scenarios that fail loudly when the three surfaces drift (registry, packages, seeder behavior)

## Non-goals

- Growing deployments.json to cover all 33 humans (that's a separate sprint — see the `$comment` in deployments.json about the matthew-alpha temp bump)
- Changing the directional semantics of the `human_relationships` unique index — per-party custody/consent flags legitimately distinguish A→B from B→A
- Changing how the seeder distributes work across peers (hash-mod routing stays)

## Architecture

```
         genesis/orchestrator/data/deployments.json
                    (registry: 6 humans, authoritative)
                              │
           ┌──────────────────┼──────────────────┐
           ▼                  ▼                  ▼
    elohim/holochain/   genesis/seeder/     genesis/a2o/features/
    Jenkinsfile         seed-accounts.ts    deployment/*.feature
    (provisions         (imports only        (asserts the
     StatefulSets)       deployed humans)     three agree)
```

`deployments.json` is the IoC seam. Three consumers; one file. The story the operator lives: *"I add a human to deployments.json. Edge provisions their StatefulSet. Seeder imports their package. A2O verifies the three are coherent. Reruns converge."*

**Naming**: *deployment registry* = `genesis/orchestrator/data/deployments.json` (what's on the cluster). *Persona registry* = `genesis/data/humans/humans.json` (who the persona is). Terms match `human-device-mapping.feature`.

## Component 1: Deployment registry loader (new)

**File**: `genesis/seeder/src/deployment-registry.ts`

```ts
export interface DeploymentRegistry {
  deployedHumanIds: Set<string>;  // e.g. {"human-adam-firstman", ...}
  source: 'file' | 'flag' | 'env';
  path?: string;
}

export function loadDeploymentRegistry(opts: {
  registryPath?: string;      // --registry=<path>
  deployedHumans?: string[];  // --deployed-humans=adam,matthew,...
}): DeploymentRegistry;
```

**Resolution order** (mirrors the seeder's existing `targetPeers` precedence):

1. `--deployed-humans=` flag or `SEEDER_DEPLOYED_HUMANS` env → explicit comma-separated list (tests, one-offs)
2. `--registry=` flag or `SEEDER_REGISTRY` env → read that JSON file
3. Default: resolve `genesis/orchestrator/data/deployments.json` relative to the seeder module

The explicit-list form requires full humanIds (`human-adam-firstman`). Short names are rejected with an error listing the expected format — this keeps the contract unambiguous and avoids a name-resolution dependency cycle between the flag and the registry file.

## Component 2: Seeder filtering

**File**: `genesis/seeder/src/seed-accounts.ts`

`loadPackages(packagesDir, humanFilter)` signature grows a `registry: DeploymentRegistry` parameter. Return shape becomes `{ toSeed: AccountPackageInputView[]; staged: AccountPackageInputView[] }`, partitioned by `registry.deployedHumanIds.has(pkg.identity.humanId)`.

**Output contract** — the operator sees:

```
=== Seed Accounts ===
Target:    https://doorway-alpha.elohim.host
Registry:  genesis/orchestrator/data/deployments.json (6 deployed humans)
Packages:  genesis/data/account-packages (33 found, 6 deployed, 27 staged)

  [+] Adam      -> ...  content=2848 rels=0 skipped=0 stew=0 coll=3
  [+] Matthew   -> ...  content=3200 rels=1 skipped=0 stew=0 coll=2
  ...
  [-] Bub       (staged — not in deployment registry)
  [-] Charlie   (staged — not in deployment registry)
  ...

=== Results: 6 imported, 0 failed, 27 staged ===
```

`[-]` is a third outcome alongside `[+]`/`[X]`. Staged packages do not count toward `failed` and do not cause `exit 1`.

**Drift warning**: if a registry entry references a humanId with no matching package file, the seeder emits `WARNING: registry references human-X, no package found` and continues. This catches typos in deployments.json.

## Component 3: Relationship idempotency

**Where**: `elohim/elohim-storage/src/http.rs` → `do_account_import` (the relationship loop, currently ~line 5486). **Not** in the low-level `create_human_relationship` — that function keeps strict-insert semantics because direct (non-import) callers want "fail if exists."

**New helper in** `elohim/elohim-storage/src/db/human_relationships.rs`:

```rust
/// Returns Some(existing) if a relationship already exists between the two
/// parties for this (app, type). When is_bidirectional is true, checks both
/// (a,b) and (b,a); when false, checks only (a,b).
pub fn find_existing_relationship(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    party_a: &str,
    party_b: &str,
    relationship_type: &str,
    is_bidirectional: bool,
) -> Result<Option<HumanRelationship>, StorageError>;
```

**Rewritten relationship loop in `do_account_import`**:

```rust
for rel in package.relationships {
    match find_existing_relationship(conn, ctx, &rel.party_a_id, &rel.party_b_id,
                                     &rel.relationship_type, rel.is_bidirectional)? {
        Some(_) => relationships_skipped += 1,
        None => match create_human_relationship(conn, ctx, rel.into()) {
            Ok(_) => relationships_created += 1,
            // Race: another concurrent import inserted between our check
            // and our insert. Treat UNIQUE violation the same as "already
            // exists" — matches the stewardship pattern at http.rs:5551.
            Err(StorageError::Internal(msg)) if msg.contains("UNIQUE constraint") => {
                relationships_skipped += 1;
            }
            Err(e) => errors.push(format!("Failed to create relationship ...: {e}")),
        }
    }
}
```

**View type change**: `AccountImportResultView` in `views.rs` grows `pub relationships_skipped: i32` (serialized as `relationshipsSkipped`). JSON schema at `elohim/sdk/schemas/v1/views/account-import-result.schema.json` updated first (schema-first workflow per storage CLAUDE.md). Then `cargo test export_bindings` regenerates TS types; `pnpm run schema:codegen:ts` propagates.

**Why bidirectional dedupe at the application layer, not the DB index**: the unique index is intentionally directional because `custody_enabled_by_a`, `consent_given_by_a`, `initiated_by`, and sibling per-party flags distinguish A→B from B→A. Collapsing at the DB would lose that. At account-import time, however, we know the package is describing a symmetric social edge that both parties will independently author — deduping there preserves per-party DB semantics while making the importer converge.

## Component 4: A2O scenarios

### 4.1 Unblock `human-device-mapping.feature:91`

Remove `@wip` from the scenario "The six protocol humans are all represented in the deployment registry." This scenario is the spiritual anchor for the registry-as-gate decision; making it executable is part of the work.

### 4.2 New feature: `genesis/a2o/features/deployment/seeder-registry-coherence.feature`

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
    When the seeder runs with "--deployed-humans=adam"
    Then the seeder attempts import for "adam" only
```

### 4.3 New scenarios in `genesis/a2o/features/content/relationship-idempotency.feature`

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

### 4.4 Step definitions

New file: `genesis/a2o/steps/seeder.steps.ts`. Reuses existing E2EWorld patterns for doorway interaction; adds registry-stubbing helpers (write a temp deployments.json and point the seeder at it via `--registry=`).

## Testing & verification

**Rust (elohim-storage)**
- Unit test `find_existing_relationship` — directional and bidirectional lookups
- Integration test in `do_account_import`: import Adam + Eve sequentially, assert one row, `relationshipsSkipped=1` on the second
- Schema contract test updated for `AccountImportResultView.relationshipsSkipped` in `tests/schema_contract.rs`
- `cargo test export_bindings` regenerates TS types

**TypeScript (seeder)**
- Unit tests in `genesis/seeder/src/deployment-registry.test.ts` (new): flag precedence, env fallback, file-read default, missing-file error
- Unit tests in `genesis/seeder/src/seed-accounts.test.ts` (existing): `loadPackages` partitions correctly; existing `resolveTargetUrl`/`stableHash` tests stay green
- Dry-run output format test — the new `[-] staged` line shape

**A2O**
- Step definitions in `genesis/a2o/steps/seeder.steps.ts`
- `scripts/scan-coverage.ts` run to confirm the new scenarios register
- Remove `@wip` from `human-device-mapping.feature:91` only after the scenario's steps pass

**Pipeline**
- `pnpm run schema:validate` after schema change
- `pnpm run schema:codegen:ts` to propagate `relationshipsSkipped`
- Green pre-push hook on `genesis/seeder/` and `elohim/elohim-storage/`

**Manual golden-path verification**
- Run seeder against alpha doorway: expect 6 imported, 27 staged, 0 failed, no 502s, no UNIQUE errors
- Rerun immediately: expect 6 imported with `skipped=N` per package, 0 failed

## Open questions / follow-ups (out of scope)

- **Peer distribution beyond matthew-alpha**: the temp-bump comment in deployments.json notes all traffic currently routes through one pod. When that's resolved (seeder splits across peers, or backpressure lands in elohim-storage), the resource bumps come back down to the archetype floors.
- **Growing deployments.json to match fixture-humans expectations**: `auth/fixture-humans.feature` expects 19+ humans to log in. That's a separate deployment-expansion sprint; this spec preserves the "registry is the gate" contract so that growth happens by editing deployments.json, not by scattered edits across seeder + Jenkinsfile + a2o.
- **Bidirectional collapse in the UI**: if the Angular relationship view starts treating `(a,b,spouse)` and `(b,a,spouse)` as the same edge, revisit whether the import-layer dedupe is still sufficient or if a projection table is warranted.
