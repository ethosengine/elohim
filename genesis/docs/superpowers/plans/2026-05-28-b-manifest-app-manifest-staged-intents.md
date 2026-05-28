# B-MANIFEST: App-Manifest Staged-Intents + Graduation Vocabulary — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the app-manifest substrate extension that adds `vocabulary.stagedIntents` + top-level `graduation` vocabulary to the protocol-level manifest schema, plus the companion `SessionLifecycleState` enum, plus the `graduation-record` whitelist entry in the Holochain DNA, plus the Capability Profile spec §5.1 patch.

**Architecture:** JSON Schema-first IoC. Add two additive sections to `app-manifest.schema.json` (backward-compatible — existing manifests validate unchanged). Add one new enum schema that the existing protocol enum-codegen pipeline auto-discovers from the `enums/` directory. Add one whitelist entry to the Holochain DNA's `MANIFEST_KINDS` constant. Add a one-paragraph patch to the Capability Profile spec naming the four new protocol-core Standings.

**Tech Stack:** JSON Schema draft 2020-12, Ajv2020 (existing test harness), Rust HDI (Holochain integrity zome), pnpm scripts.

**Spec basis:** `genesis/docs/superpowers/specs/2026-05-28-app-manifest-staged-intents-design.md` (commit `8a1e6c294`).

**Parent plan:** `genesis/docs/superpowers/plans/2026-05-28-session-bridge-implementation.md` (B-MANIFEST ticket).

---

## File structure

| File | Action | Responsibility |
|---|---|---|
| `elohim/sdk/schemas/v1/enums/session-lifecycle-state.schema.json` | CREATE | New protocol enum: `Anonymous` / `OauthIdentified` / `PeerNativeSampling` / `PeerNativeMember`. |
| `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` | MODIFY | Add `$defs/StagedIntentDeclaration`, `$defs/GraduationPolicy`, `vocabulary.stagedIntents` property, top-level `graduation` property, `dependentRequired` conditional rule. |
| `elohim/sdk/schemas/scripts/test-schema.mjs` | MODIFY | Add 7 new manifest-validation assertions per spec §11.1. |
| `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs:37-43` | MODIFY | Add `"graduation-record"` to `MANIFEST_KINDS` constant. |
| `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` §5.1 | MODIFY | One-paragraph patch naming the four new protocol-core Standings. |

**What this plan does NOT touch:**
- The session-bridge crate (lives in B-CRATE; Wave A parallel ticket).
- Per-pillar codegen extension at `elohim/sdk/domains/<pillar>/scripts/codegen.mjs` (lives in each B-PILLAR-* ticket).
- Per-pillar manifest declarations (each B-PILLAR-* ticket lands its own).
- Wire-type schema files (`session-lifecycle.ts`, `staged-intent-envelope.ts`, etc. — those come from B-CRATE).
- The `codegen-ts.mjs` `INTERFACE_FILES` extension for bridge wire types (B-CRATE's responsibility; bridge schemas land there with their own registration).

---

## Tasks

### Task 1: SessionLifecycleState enum schema

**Files:**
- Create: `elohim/sdk/schemas/v1/enums/session-lifecycle-state.schema.json`

The protocol enum-codegen auto-discovers files in `enums/` via `readdir` at `elohim/sdk/schemas/scripts/codegen-ts.mjs:389`. Creating the file is sufficient; no registration line needed.

- [ ] **Step 1: Write a failing test asserting the enum file exists and contains the four lifecycle values**

Add to `elohim/sdk/schemas/scripts/test-schema.mjs` immediately before the final summary block (look for the line containing `console.log(\`\\nResults:`):

```javascript
// Test: SessionLifecycleState enum schema
{
  const enumPath = resolve(__dirname, '../v1/enums/session-lifecycle-state.schema.json');
  let lifecycleSchema;
  try {
    lifecycleSchema = await loadJson(enumPath);
  } catch {
    lifecycleSchema = null;
  }
  assert(
    lifecycleSchema !== null,
    'SessionLifecycleState enum schema exists at v1/enums/session-lifecycle-state.schema.json'
  );
  assert(
    lifecycleSchema?.title === 'SessionLifecycleState',
    'SessionLifecycleState enum schema declares title: "SessionLifecycleState"'
  );
  assert(
    Array.isArray(lifecycleSchema?.enum) &&
      lifecycleSchema.enum.length === 4 &&
      lifecycleSchema.enum.includes('Anonymous') &&
      lifecycleSchema.enum.includes('OauthIdentified') &&
      lifecycleSchema.enum.includes('PeerNativeSampling') &&
      lifecycleSchema.enum.includes('PeerNativeMember'),
    'SessionLifecycleState enum contains all four lifecycle values'
  );
}
```

- [ ] **Step 2: Run the test to verify failure**

Run:
```bash
pnpm run schema:test
```
Expected: FAIL lines from the three new assertions ("SessionLifecycleState enum schema exists ..." etc.) — the file does not yet exist.

- [ ] **Step 3: Create the enum schema file**

Create `elohim/sdk/schemas/v1/enums/session-lifecycle-state.schema.json` with this exact content:

```json
{
  "$id": "epr:schema:enum:session-lifecycle-state",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "SessionLifecycleState",
  "description": "Pre-canonical participation lifecycle state managed by the session-bridge primitive. Operational vocabulary — not DHT-notarized; not held in any zome. Carried by the Capability Profile as a protocol-core Standing per the Capability Profile spec §5.1. See session-bridge spec (2026-05-28-session-bridge-design.md) §1 for the four-state lifecycle model.",
  "type": "string",
  "enum": ["Anonymous", "OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
  "_tiers": {
    "core": {
      "values": ["Anonymous", "OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
      "rationale": "All four lifecycle states are protocol-core; no app-tier extensions. The session-bridge primitive is the substrate-level owner of these values."
    }
  },
  "_dna": {
    "constant": "SESSION_LIFECYCLE_STATES",
    "zome": null,
    "rationale": "Operational vocabulary — bridge state, not DHT-notarized state. No zome owns these as entries."
  }
}
```

- [ ] **Step 4: Run the test to verify success**

Run:
```bash
pnpm run schema:test
```
Expected: PASS lines from the three new assertions.

- [ ] **Step 5: Regenerate enum constants and verify distribution**

Run:
```bash
pnpm run schema:codegen:ts
```
Expected: completes without error.

Verify the new constants land in all four distribution locations:
```bash
grep -l "SESSION_LIFECYCLE_STATES\|SessionLifecycleState" \
  genesis/seeder/src/generated/schema-enums.ts \
  app/elohim-app/src/app/generated/schema-enums.ts \
  app/elohim-library/projects/elohim-service/src/generated/schema-enums.ts
```
Expected: all three paths in the output.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/enums/session-lifecycle-state.schema.json \
        elohim/sdk/schemas/scripts/test-schema.mjs \
        genesis/seeder/src/generated/schema-enums.ts \
        app/elohim-app/src/app/generated/schema-enums.ts \
        app/elohim-library/projects/elohim-service/src/generated/schema-enums.ts
git commit -m "feat(B-MANIFEST) Task 1: SessionLifecycleState enum + codegen distribution"
```

---

### Task 2: App-manifest schema `$defs` for StagedIntentDeclaration and GraduationPolicy

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` ($defs block near bottom of file)

- [ ] **Step 1: Write a failing test asserting the two new $defs exist**

Add to `elohim/sdk/schemas/scripts/test-schema.mjs` immediately after the Task 1 enum assertions:

```javascript
// Test: app-manifest $defs additions for staged-intents substrate
{
  const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
  const manifestSchema = await loadJson(manifestSchemaPath);
  assert(
    manifestSchema.$defs?.StagedIntentDeclaration !== undefined,
    '$defs/StagedIntentDeclaration is defined on app-manifest.schema.json'
  );
  assert(
    manifestSchema.$defs?.GraduationPolicy !== undefined,
    '$defs/GraduationPolicy is defined on app-manifest.schema.json'
  );
  const sid = manifestSchema.$defs?.StagedIntentDeclaration;
  const sidRequired = Array.isArray(sid?.required) ? sid.required : [];
  assert(
    sidRequired.includes('description') &&
      sidRequired.includes('intentSchema') &&
      sidRequired.includes('graduatesTo') &&
      sidRequired.includes('actionableFrom') &&
      sidRequired.includes('resolutionMode') &&
      sidRequired.includes('coupling'),
    'StagedIntentDeclaration requires all six declared fields (description / intentSchema / graduatesTo / actionableFrom / resolutionMode / coupling)'
  );
  const gp = manifestSchema.$defs?.GraduationPolicy;
  const gpRequired = Array.isArray(gp?.required) ? gp.required : [];
  assert(
    gpRequired.includes('deterministicCeremony'),
    'GraduationPolicy requires deterministicCeremony'
  );
}
```

- [ ] **Step 2: Run the test to verify failure**

Run:
```bash
pnpm run schema:test
```
Expected: FAIL lines from the four new assertions.

- [ ] **Step 3: Add the $defs entries**

Open `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`. Find the `$defs` block (look for the existing `"ThreeLegCoupling"` entry — confirmed at line 226). Add the following two entries inside `$defs`, immediately before the closing `}` of the `$defs` block:

```json
    "StagedIntentDeclaration": {
      "title": "StagedIntentDeclaration",
      "description": "Per-pillar staged-intent declaration. The session-bridge holds intents matching this declaration pre-canonically and replays them into the named canonical entry at graduation. Spec: 2026-05-28-app-manifest-staged-intents-design.md §2.",
      "type": "object",
      "required": ["description", "intentSchema", "graduatesTo", "actionableFrom", "resolutionMode", "coupling"],
      "properties": {
        "description": {
          "type": "string",
          "minLength": 1,
          "description": "Human-readable. What this intent represents pre-canonically."
        },
        "intentSchema": {
          "type": "object",
          "description": "JSON Schema $ref to the intent payload schema in the pillar's domain schemas/ directory.",
          "required": ["$ref"],
          "properties": {
            "$ref": { "type": "string", "minLength": 1 }
          }
        },
        "graduatesTo": {
          "type": "string",
          "minLength": 1,
          "description": "Names the canonical entry type the intent replays into at graduation. Runtime lookup key — validation does not verify the entry type exists."
        },
        "actionableFrom": {
          "type": "array",
          "minItems": 1,
          "items": { "$ref": "../enums/session-lifecycle-state.schema.json" },
          "description": "Which lifecycle states the intent is actionable from. Bridge's is_actionable() predicate reads this directly."
        },
        "resolutionMode": {
          "type": "string",
          "enum": ["deterministic", "negotiated", "either"],
          "description": "How graduation resolves: deterministic = 1:1 mapping; negotiated = wisdom-invocation appraisal; either = try deterministic, fall back to negotiated."
        },
        "coupling": {
          "$ref": "#/$defs/ThreeLegCoupling"
        }
      },
      "additionalProperties": false
    },
    "GraduationPolicy": {
      "title": "GraduationPolicy",
      "description": "Per-pillar graduation policy. Names runtime registry lookup keys, the wisdom-invocation framing CID, the appraisal agent hint, and the notarization mode. Spec: 2026-05-28-app-manifest-staged-intents-design.md §3.",
      "type": "object",
      "required": ["deterministicCeremony"],
      "properties": {
        "deterministicCeremony": {
          "type": "string",
          "minLength": 1,
          "description": "Runtime registry lookup key for the deterministic GraduationCeremony impl. Convention: <pillar>::<CeremonyName>."
        },
        "negotiatedCeremony": {
          "type": "string",
          "minLength": 1,
          "description": "Optional. Runtime registry lookup key for the negotiated ceremony. Absence means the pillar supports only deterministic resolutions."
        },
        "framingCid": {
          "type": "string",
          "minLength": 1,
          "description": "Optional. The wisdom-invocation framing CID the bridge composes when invoking appraisal. Per session-bridge implementation plan §6 Q7 — pillar declares which elohim role appraises via the framing CID it composes."
        },
        "appraisalAgent": {
          "type": "string",
          "enum": ["home-elohim", "commons-elohim", "neutral-counsel", "custom"],
          "description": "Optional. Convenience hint for runtime composition. Default per session-bridge implementation plan §6 Q7 v1 is home-elohim."
        },
        "notarizeAppraisal": {
          "type": "string",
          "enum": ["always", "on-request", "never"],
          "description": "Optional. Default: on-request. Whether the bridge authors a Manifest{kind: 'graduation-record'} entry per graduation."
        }
      },
      "additionalProperties": false
    },
```

- [ ] **Step 4: Run the test to verify success**

Run:
```bash
pnpm run schema:test
```
Expected: PASS lines from the four new assertions.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/manifest/app-manifest.schema.json \
        elohim/sdk/schemas/scripts/test-schema.mjs
git commit -m "feat(B-MANIFEST) Task 2: \$defs/StagedIntentDeclaration + \$defs/GraduationPolicy"
```

---

### Task 3: App-manifest schema — `vocabulary.stagedIntents` + top-level `graduation` + dependentRequired rule

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`

- [ ] **Step 1: Write failing tests asserting both new properties + conditional validation**

Add to `elohim/sdk/schemas/scripts/test-schema.mjs` immediately after the Task 2 assertions:

```javascript
// Test: vocabulary.stagedIntents + top-level graduation + dependentRequired
{
  const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
  const manifestSchema = await loadJson(manifestSchemaPath);

  const vocabProps = manifestSchema.$defs?.Vocabulary?.properties || manifestSchema.properties?.vocabulary?.properties;
  assert(
    vocabProps?.stagedIntents !== undefined,
    'vocabulary.stagedIntents property is declared'
  );
  assert(
    vocabProps?.stagedIntents?.additionalProperties?.$ref === '#/$defs/StagedIntentDeclaration',
    'vocabulary.stagedIntents.additionalProperties references $defs/StagedIntentDeclaration'
  );

  assert(
    manifestSchema.properties?.graduation?.$ref === '#/$defs/GraduationPolicy',
    'top-level graduation property references $defs/GraduationPolicy'
  );

  // Conditional: if vocabulary.stagedIntents is non-empty, top-level graduation must be present.
  // JSON Schema 2020-12 expresses this via "dependentSchemas" on vocabulary, OR a top-level
  // allOf with if/then. Either is acceptable — assert one or the other shape is present.
  const hasDependentSchemas = manifestSchema.dependentSchemas?.vocabulary !== undefined;
  const hasAllOfIfThen = Array.isArray(manifestSchema.allOf) &&
    manifestSchema.allOf.some(clause => clause.if && clause.then);
  assert(
    hasDependentSchemas || hasAllOfIfThen,
    'Conditional rule present: stagedIntents non-empty implies top-level graduation required'
  );
}
```

- [ ] **Step 2: Run the test to verify failure**

Run:
```bash
pnpm run schema:test
```
Expected: FAIL lines from the four new assertions.

- [ ] **Step 3: Add `vocabulary.stagedIntents` property**

Open `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`. Locate the `Vocabulary` definition inside `$defs` (look for the existing `contentTypes` / `contentFormats` / `relationships` siblings). Inside the `Vocabulary` definition's `properties`, add:

```json
        "stagedIntents": {
          "type": "object",
          "description": "Optional. Per-pillar staged-intent vocabulary. Each entry declares an intent shape the session-bridge holds pre-canonically and replays into a canonical entry at graduation. Spec: 2026-05-28-app-manifest-staged-intents-design.md §2. Pillars without tentative-participation surfaces omit this section.",
          "additionalProperties": {
            "$ref": "#/$defs/StagedIntentDeclaration"
          }
        }
```

- [ ] **Step 4: Add top-level `graduation` property**

Locate the top-level `properties` block (the one that declares `id`, `name`, `version`, `vocabulary`, `rendering`, `projections`, `writeThrough`, `observation_kinds`, `signalKinds`, `graph`). Add at the same level, after `signalKinds` and before `graph`:

```json
    "graduation": {
      "$ref": "#/$defs/GraduationPolicy",
      "description": "Optional. Per-pillar graduation policy. Required IF vocabulary.stagedIntents is non-empty (see dependentSchemas)."
    },
```

- [ ] **Step 5: Add `dependentSchemas` conditional rule**

Add a new top-level property at the schema root, immediately after the existing `required` array (look for `"required": ["id", "name", "version", "vocabulary"]`):

```json
  "dependentSchemas": {
    "vocabulary": {
      "if": {
        "properties": {
          "vocabulary": {
            "properties": {
              "stagedIntents": {
                "type": "object",
                "minProperties": 1
              }
            },
            "required": ["stagedIntents"]
          }
        }
      },
      "then": {
        "required": ["graduation"]
      }
    }
  },
```

- [ ] **Step 6: Run the test to verify success**

Run:
```bash
pnpm run schema:test
```
Expected: PASS lines from the four new assertions.

- [ ] **Step 7: Commit**

```bash
git add elohim/sdk/schemas/v1/manifest/app-manifest.schema.json \
        elohim/sdk/schemas/scripts/test-schema.mjs
git commit -m "feat(B-MANIFEST) Task 3: vocabulary.stagedIntents + top-level graduation + conditional"
```

---

### Task 4: Positive + negative validation tests against manifest fixtures

**Files:**
- Modify: `elohim/sdk/schemas/scripts/test-schema.mjs`

Per spec §11.1, seven assertions are required. Tasks 1-3 covered structural assertions (4 of them). This task adds the seven manifest-fixture assertions.

- [ ] **Step 1: Write failing tests using inline manifest fixtures**

Add to `elohim/sdk/schemas/scripts/test-schema.mjs` immediately after the Task 3 assertions. The pattern: compile the manifest schema with Ajv, run validation against fixtures, assert pass/fail outcomes.

```javascript
// Test: manifest fixtures validate per stagedIntents + graduation rules
{
  const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
  const manifestSchema = await loadJson(manifestSchemaPath);

  // Ajv setup mirrors the existing manifest-schema test harness above.
  // Resolve $refs in the schema's enum array (actionableFrom) — Ajv needs the
  // session-lifecycle-state schema registered.
  const lifecycleSchemaPath = resolve(__dirname, '../v1/enums/session-lifecycle-state.schema.json');
  const lifecycleSchema = await loadJson(lifecycleSchemaPath);

  const ajv = new Ajv2020({ strict: false, allErrors: true });
  ajv.addSchema(lifecycleSchema, '../enums/session-lifecycle-state.schema.json');
  // Also register against the resolved $id form, so internal $refs resolve:
  ajv.addSchema(lifecycleSchema, lifecycleSchema.$id);

  const validate = ajv.compile(manifestSchema);

  const baseManifest = {
    id: 'manifest-test-fixture',
    name: 'test-fixture',
    version: '1.0.0',
    vocabulary: {
      contentTypes: {}
    }
  };

  const couplingFixture = {
    knowledge: { relationships: { REFERENCES: ['concept'] } },
    value: { onConsume: { action: 'use', resourceConformsTo: 'test', recognition: 'test' } },
    governance: {
      defaultReach: 'self',
      minimumReach: 'self',
      governanceModel: 'self-sovereign',
      signalTypes: ['test-signal']
    }
  };

  // Assertion 1: manifest with stagedIntents + graduation validates clean
  const fixtureWithStaged = {
    ...baseManifest,
    vocabulary: {
      contentTypes: {},
      stagedIntents: {
        'staged-test-intent': {
          description: 'Test staged intent for schema validation.',
          intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
          graduatesTo: 'TestEntry',
          actionableFrom: ['OauthIdentified', 'PeerNativeMember'],
          resolutionMode: 'deterministic',
          coupling: couplingFixture
        }
      }
    },
    graduation: {
      deterministicCeremony: 'test::DeterministicCeremony'
    }
  };
  assert(
    validate(fixtureWithStaged),
    'Manifest with stagedIntents + graduation validates clean'
  );

  // Assertion 2: manifest without stagedIntents validates clean (backward compat)
  const fixtureBaseline = { ...baseManifest };
  assert(
    validate(fixtureBaseline),
    'Manifest without stagedIntents validates clean (backward compatibility)'
  );

  // Assertion 3: manifest with stagedIntents but missing top-level graduation fails
  const fixtureMissingGraduation = {
    ...baseManifest,
    vocabulary: {
      contentTypes: {},
      stagedIntents: {
        'staged-test-intent': {
          description: 'Test staged intent.',
          intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
          graduatesTo: 'TestEntry',
          actionableFrom: ['OauthIdentified'],
          resolutionMode: 'deterministic',
          coupling: couplingFixture
        }
      }
    }
  };
  assert(
    !validate(fixtureMissingGraduation),
    'Manifest with non-empty stagedIntents but missing graduation fails (dependentSchemas conditional)'
  );

  // Assertion 4: stagedIntents entry missing graduatesTo fails
  const fixtureMissingGraduatesTo = {
    ...baseManifest,
    vocabulary: {
      contentTypes: {},
      stagedIntents: {
        'staged-test-intent': {
          description: 'Test staged intent missing graduatesTo.',
          intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
          actionableFrom: ['OauthIdentified'],
          resolutionMode: 'deterministic',
          coupling: couplingFixture
        }
      }
    },
    graduation: { deterministicCeremony: 'test::DeterministicCeremony' }
  };
  assert(
    !validate(fixtureMissingGraduatesTo),
    'stagedIntents entry missing graduatesTo fails validation'
  );

  // Assertion 5: actionableFrom with invalid lifecycle value fails
  const fixtureInvalidLifecycle = {
    ...baseManifest,
    vocabulary: {
      contentTypes: {},
      stagedIntents: {
        'staged-test-intent': {
          description: 'Test staged intent with invalid actionableFrom.',
          intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
          graduatesTo: 'TestEntry',
          actionableFrom: ['NotAValidLifecycleValue'],
          resolutionMode: 'deterministic',
          coupling: couplingFixture
        }
      }
    },
    graduation: { deterministicCeremony: 'test::DeterministicCeremony' }
  };
  assert(
    !validate(fixtureInvalidLifecycle),
    'actionableFrom array with invalid lifecycle value fails validation'
  );

  // Assertion 6: graduation.deterministicCeremony empty string fails
  const fixtureEmptyCeremony = {
    ...baseManifest,
    vocabulary: {
      contentTypes: {},
      stagedIntents: {
        'staged-test-intent': {
          description: 'Test staged intent.',
          intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
          graduatesTo: 'TestEntry',
          actionableFrom: ['OauthIdentified'],
          resolutionMode: 'deterministic',
          coupling: couplingFixture
        }
      }
    },
    graduation: { deterministicCeremony: '' }
  };
  assert(
    !validate(fixtureEmptyCeremony),
    'graduation.deterministicCeremony empty string fails validation'
  );

  // Assertion 7: resolutionMode outside enum fails
  const fixtureInvalidResolution = {
    ...baseManifest,
    vocabulary: {
      contentTypes: {},
      stagedIntents: {
        'staged-test-intent': {
          description: 'Test staged intent.',
          intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
          graduatesTo: 'TestEntry',
          actionableFrom: ['OauthIdentified'],
          resolutionMode: 'instantaneous',
          coupling: couplingFixture
        }
      }
    },
    graduation: { deterministicCeremony: 'test::DeterministicCeremony' }
  };
  assert(
    !validate(fixtureInvalidResolution),
    'resolutionMode outside the enum (deterministic | negotiated | either) fails validation'
  );
}
```

- [ ] **Step 2: Run the tests to verify all seven pass**

Run:
```bash
pnpm run schema:test
```
Expected: PASS lines from all seven new assertions. Total schema:test output should show no FAIL lines.

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/scripts/test-schema.mjs
git commit -m "test(B-MANIFEST) Task 4: seven manifest-validation assertions for stagedIntents"
```

---

### Task 5: Holochain DNA — whitelist `graduation-record` manifest kind

**Files:**
- Modify: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs:37-43`

The `MANIFEST_KINDS` constant lives at line 37. It is a `&[&str]` containing the currently-whitelisted manifest kinds (`"app"`, `"pillar-projection"`, `"standing-policy"`, `"tending-policy"`, `"onboarding"`). The validator at line 96 rejects manifests whose `manifest_kind` is not in the list. We add `"graduation-record"`.

Per `elohim/holochain/dna/CLAUDE.md`: build uses `just check` from `elohim/holochain/dna/elohim/` — RUSTFLAGS is set in the justfile, don't override.

- [ ] **Step 1: Write a failing Rust unit test asserting MANIFEST_KINDS contains "graduation-record"**

Open `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs`. Find the existing test module (search for `#[cfg(test)]`). If no test module exists in this file, add one at the bottom. Add this test:

```rust
#[cfg(test)]
mod manifest_kinds_tests {
    use super::*;

    #[test]
    fn graduation_record_is_whitelisted() {
        assert!(
            MANIFEST_KINDS.contains(&"graduation-record"),
            "MANIFEST_KINDS must include \"graduation-record\" for B-APPRAISE Phase 2 to author appraisal records. See genesis/docs/superpowers/specs/2026-05-28-app-manifest-staged-intents-design.md §4 and the parent implementation plan §1.5 Design Constraint 2."
        );
    }
}
```

- [ ] **Step 2: Run the failing test**

From the DNA root directory:

```bash
cd elohim/holochain/dna/elohim && just check
```

If `just check` does not surface unit-test failures, run instead:

```bash
cd elohim/holochain/dna/elohim/zomes/content_store_integrity && cargo test graduation_record_is_whitelisted
```

Expected: FAIL with message about MANIFEST_KINDS missing "graduation-record".

- [ ] **Step 3: Add `graduation-record` to MANIFEST_KINDS**

Edit `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs` at line 37-43. Change:

```rust
const MANIFEST_KINDS: &[&str] = &[
    "app",
    "pillar-projection",
    "standing-policy",
    "tending-policy",
    "onboarding",
];
```

To:

```rust
const MANIFEST_KINDS: &[&str] = &[
    "app",
    "pillar-projection",
    "standing-policy",
    "tending-policy",
    "onboarding",
    "graduation-record",
];
```

- [ ] **Step 4: Run the test to verify success**

```bash
cd elohim/holochain/dna/elohim/zomes/content_store_integrity && cargo test graduation_record_is_whitelisted
```

Expected: PASS.

- [ ] **Step 5: Run `just check` for the full DNA to verify no regressions**

```bash
cd elohim/holochain/dna/elohim && just check
```

Expected: succeeds with no errors.

- [ ] **Step 6: Commit**

```bash
git add elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs
git commit -m "feat(B-MANIFEST) Task 5: whitelist graduation-record manifest kind"
```

---

### Task 6: Regression check — existing per-pillar manifests still validate

**Files:**
- No file modifications; verification only.

The schema additions are designed to be backward-compatible. This task confirms the seven existing per-pillar manifests (`avodah`, `elohim`, `imagodei`, `infrastructure`, `lamad`, `mishpat`, `qahal`, `shefa`) validate cleanly against the updated `app-manifest.schema.json`. The existing `pnpm run schema:test` already exercises a manifest-validation pass; this task confirms it still passes after the substrate extensions.

- [ ] **Step 1: Run the existing schema-validation suite end-to-end**

```bash
pnpm run schema:test
```

Expected: all PASS lines; no FAIL lines. Look specifically for the existing per-pillar manifest checks (lamad, imagodei, qahal, shefa, etc. — the suite already validates each).

- [ ] **Step 2: Manually validate each per-pillar manifest via Ajv as an additional safety net**

Add to `elohim/sdk/schemas/scripts/test-schema.mjs` immediately after the Task 4 assertions:

```javascript
// Test: existing per-pillar manifests validate clean against the extended schema (regression)
{
  const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
  const manifestSchema = await loadJson(manifestSchemaPath);
  const lifecycleSchemaPath = resolve(__dirname, '../v1/enums/session-lifecycle-state.schema.json');
  const lifecycleSchema = await loadJson(lifecycleSchemaPath);

  const ajv = new Ajv2020({ strict: false, allErrors: true });
  ajv.addSchema(lifecycleSchema, '../enums/session-lifecycle-state.schema.json');
  ajv.addSchema(lifecycleSchema, lifecycleSchema.$id);
  const validate = ajv.compile(manifestSchema);

  const pillars = ['avodah', 'elohim', 'imagodei', 'infrastructure', 'lamad', 'mishpat', 'qahal', 'shefa'];
  for (const pillar of pillars) {
    const manifestPath = resolve(__dirname, `../../domains/${pillar}/manifest.json`);
    let manifest;
    try {
      manifest = await loadJson(manifestPath);
    } catch {
      continue;
    }
    const ok = validate(manifest);
    if (!ok) {
      console.error(`Manifest ${pillar} validation errors:`, validate.errors);
    }
    assert(
      ok,
      `Existing per-pillar manifest validates clean after substrate extension: ${pillar}`
    );
  }
}
```

- [ ] **Step 3: Run the test to verify the regression check passes**

```bash
pnpm run schema:test
```

Expected: PASS lines for each pillar manifest. If any fails, the regression diagnoses the per-pillar shape mismatch.

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/scripts/test-schema.mjs
git commit -m "test(B-MANIFEST) Task 6: regression check — existing per-pillar manifests still validate"
```

---

### Task 7: Capability Profile spec §5.1 patch — name the four lifecycle Standings as protocol-core

**Files:**
- Modify: `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` (section §5.1)

Per spec §13 of the staged-intents design + §1.9 of the session-bridge implementation plan, the four lifecycle values (Anonymous / OauthIdentified / PeerNativeSampling / PeerNativeMember) extend the protocol-core Standings enumeration. The Capability Profile spec is the canon for that enumeration. The patch is one paragraph.

- [ ] **Step 1: Read the existing §5.1 section to locate the insertion point**

```bash
grep -n "5.1\|Protocol-core Standings" /projects/elohim/genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md | head -10
```

Expected output: line numbers identifying the §5.1 heading and the table of Standings.

- [ ] **Step 2: Add the one-paragraph patch immediately after the existing §5.1 Standings table**

Find the existing §5.1 table (look for the heading `### 5.1 Protocol-core Standings (HARD-enforced)` near line 354 of the file; the table follows). After the table block and before the §5.2 heading, append:

```markdown
**Pre-canonical lifecycle Standings (2026-05-28 extension).** The session-bridge primitive (`genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md`) introduces four additional protocol-core Standings — `Anonymous`, `OauthIdentified`, `PeerNativeSampling`, `PeerNativeMember` — representing pre-canonical participation lifecycle. Source of truth: `elohim/sdk/schemas/v1/enums/session-lifecycle-state.schema.json`. These are HARD-enforced via the same `<elohim-standing-refused>` slot pattern as the other protocol-core Standings. An element declaring lifecycle-required cells (e.g., a member-only graduation-status panel that should not render to an anonymous visitor) names the required lifecycle Standing in its `capabilityContract.standings.required` and the substrate refuses rendering at any incompatible lifecycle. The session-bridge resolves the participant's current lifecycle into exactly one of these four Standings; the Capability Profile carries it; downstream rendering follows the existing element-contract pattern.
```

- [ ] **Step 3: Commit**

```bash
git add genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md
git commit -m "docs(spec): Capability Profile §5.1 — name lifecycle Standings as protocol-core"
```

---

### Task 8: Final integration verification

**Files:**
- No modifications; verification only.

- [ ] **Step 1: Run the full schema test suite end-to-end**

```bash
pnpm run schema:test
```

Expected: all assertions PASS; no FAIL lines.

- [ ] **Step 2: Run schema codegen freshness check**

```bash
pnpm run schema:codegen:ts -- --verify
```

If `--verify` is not supported, run codegen and confirm no file diffs land:

```bash
pnpm run schema:codegen:ts
git status -- '*generated*'
```

Expected: no unstaged file diffs in any `generated/` directory beyond what Task 1 committed (the new `SESSION_LIFECYCLE_STATES` constants).

- [ ] **Step 3: Confirm the DNA still builds clean**

```bash
cd elohim/holochain/dna/elohim && just check
```

Expected: succeeds.

- [ ] **Step 4: Confirm git log shows the complete ticket arc**

```bash
git log --oneline -8
```

Expected (most recent first):
1. `docs(spec): Capability Profile §5.1 — name lifecycle Standings as protocol-core`
2. `test(B-MANIFEST) Task 6: regression check ...`
3. `feat(B-MANIFEST) Task 5: whitelist graduation-record manifest kind`
4. `test(B-MANIFEST) Task 4: seven manifest-validation assertions ...`
5. `feat(B-MANIFEST) Task 3: vocabulary.stagedIntents + top-level graduation ...`
6. `feat(B-MANIFEST) Task 2: $defs/StagedIntentDeclaration + $defs/GraduationPolicy`
7. `feat(B-MANIFEST) Task 1: SessionLifecycleState enum + codegen distribution`

- [ ] **Step 5: No commit needed — verification only**

---

## Acceptance checklist

The ticket closes when each of these is true:

- [ ] `pnpm run schema:test` passes with all new assertions PASS, zero FAIL.
- [ ] `pnpm run schema:codegen:ts` regenerates `schema-enums.ts` in all four distribution locations with the new `SESSION_LIFECYCLE_STATES` / `CORE_SESSION_LIFECYCLE_STATES` / `ALL_SESSION_LIFECYCLE_STATES` constants and the `SessionLifecycleState` type alias.
- [ ] Existing per-pillar manifests (avodah, elohim, imagodei, infrastructure, lamad, mishpat, qahal, shefa) validate clean against the extended schema.
- [ ] A manifest with `vocabulary.stagedIntents` non-empty but missing top-level `graduation` fails validation with a clear error.
- [ ] A manifest with `vocabulary.stagedIntents` entries declaring all six required fields + a top-level `graduation.deterministicCeremony` validates clean.
- [ ] `MANIFEST_KINDS` in `manifest.rs` contains `"graduation-record"`; the integrity zome's `just check` succeeds.
- [ ] Capability Profile spec §5.1 names the four new lifecycle Standings as protocol-core.

---

## Watchdog discipline

Per the parent plan's "Common steps for every B-PILLAR ticket" + memory `feedback_multi_agent_pvc_pacing`:

- **No full-workspace cargo builds.** Tasks 5 + 8 use the DNA's `just check` which scopes to the integrity zome via the justfile-managed RUSTFLAGS, NOT a workspace-wide cargo build.
- **Schema changes are pure-JSON edits.** Tasks 1-4 + 6-7 require no Rust builds at all. The codegen step in Task 1 is Node-only.
- **Per-task commits.** Every task ends with an explicit commit. Stall between tasks preserves the prior committed work.

---

## What this plan does NOT do

Spelling out the negative scope so reviewers + execution agents stay honest:

- Does NOT touch `crates/session-bridge/` (B-CRATE, parallel Wave A ticket).
- Does NOT add bridge wire-type schemas to `INTERFACE_FILES` in `codegen-ts.mjs` (B-CRATE's responsibility; bridge schemas land with their own registration).
- Does NOT extend any per-pillar `scripts/codegen.mjs` (B-PILLAR-* tickets each extend their own).
- Does NOT add `vocabulary.stagedIntents` entries to any per-pillar manifest (B-PILLAR-* tickets each declare their own).
- Does NOT introduce the `graduation-record` payload schema at `elohim/sdk/schemas/v1/manifest-payloads/` (B-APPRAISE Phase 2's responsibility).
- Does NOT modify any TypeScript service in `@elohim/identity` (B-DOORWAY's responsibility).
- Does NOT resolve any of the spec's §10 open questions; the working defaults from the spec ship as-is.

---

## References

- Spec: `genesis/docs/superpowers/specs/2026-05-28-app-manifest-staged-intents-design.md` (commit `8a1e6c294`)
- Parent plan: `genesis/docs/superpowers/plans/2026-05-28-session-bridge-implementation.md` (B-MANIFEST ticket)
- Session-bridge spec (the primitive this manifest substrate supports): `genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md`
- Schema-codegen canon: `elohim/sdk/schemas/CLAUDE.md`
- Holochain DNA canon: `elohim/holochain/dna/CLAUDE.md`
- App-manifest schema: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`
- Capability Profile spec (Task 7 patch target): `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` §5.1
