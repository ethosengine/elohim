---
title: Quilt-Policy Schema Substrate — Implementation Plan
id: quilt-policy-schema-substrate-plan
status: Draft
class: protocol-canonical
domain: D5
topic: [quilt, tier, manifest, schema, storage-policy]
cites:
  - tiered-quilt-stewardship-design | the canonical D5 spec whose §4 v0.2 amendment (declarative quilt-policy classes) this plan implements the schema substrate for — substrate-only; classifier/negotiation surfaces stay HELD | sha256:9f9c6a1c391712b3
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
---

# Quilt-Policy Schema Substrate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the validatable manifest substrate for the tiered-quilt §4 amendment — `vocabulary.quiltPolicies` named policy classes in the app-manifest schema, loader-enforced referential integrity, and fixture tests — while the consuming TierController/HeuristicClassifier stay HELD (staged-intents precedent: "substrate LANDED, feature HELD").

**Architecture:** Pure-additive JSON-Schema extension to `app-manifest.schema.json` (shape only — pillars own names/values), plus a small reusable `.mjs` referential-integrity check that both the test harness and `codegen-manifest.mjs` call, so a dangling `quiltPolicy` reference fails loud at validation/codegen time instead of silently never applying (closing the staged-intents typo-trap, not re-minting it). No pillar manifest declares a policy yet; no Rust changes.

**Tech Stack:** JSON Schema 2020-12 + AJV (`ajv/dist/2020.js`), plain-Node test harness (the `test-manifest-schema.mjs` pattern — no Vitest), ESM `.mjs`.

**Spec anchors:** `genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md` §4 "App-manifest extension — declarative quilt-policy classes (amended 2026-06-04)". Gap-items: `.claude/memory-kit/gap-items/architecture__2026-05-11-tiered-quilt-stewardship-design.json` items #1, #4, #5, #12 (items #2, #3, #6–#11 are classifier/CommitmentFactory/event-side — HELD with the tier runtime).

**Out of scope (HELD, do not build):** TierController, HeuristicClassifier, the "floor-above-any-pledge" static *warn* (needs constitutional_ratio_registry — Rust manifest-load path, lands with the tier epic), cost_class event-column writes, SDK warm()/release().

**Source-of-truth declaration (P2P design gate):** This plan creates NO new storage entity. `QuiltPolicyDeclaration` lives *inside the app manifest*, which is already **Category A** — an EPR, content-addressed, versioned, governed (per `app-manifest-sdk-boundary-design`: "manifest IS an EPR"). Identity is the policy name scoped by the manifest's CID — content-derived through its parent, no UUID, no slug-as-identity. No new DHT entry type (the Manifest entry type exists), no new coordinator function, no new SQL table, no new HTTP route, no new sync message — the schema validates the SHAPE of declarations the existing manifest plumbing already carries. The runtime projections this vocabulary will eventually drive (`quilt_tier_state`, Category C) are specified in tiered-quilt §3/§4 and stay HELD with the tier epic.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` | Modify | Shape: `Vocabulary.quiltPolicies` + `Vocabulary.quiltPolicyDefault` + `ContentTypeDeclaration.quiltPolicy` + `$defs/QuiltPolicyDeclaration` + `$defs/QuiltDuration` |
| `elohim/sdk/schemas/scripts/lib/manifest-quilt-refs.mjs` | Create | `validateQuiltPolicyRefs(manifest)` — the ONE reusable referential-integrity check |
| `elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs` | Create | Fixture tests: acceptance + negative schema cases + ref-check cases |
| `elohim/sdk/schemas/scripts/codegen-manifest.mjs` | Modify | Call `validateQuiltPolicyRefs` after manifest load; exit 1 on dangling refs |
| `package.json` (repo root) | Modify | Chain the new test into the `manifest:test` script |

---

### Task 1: Acceptance test for the schema extension (RED)

**Files:**
- Create: `elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs`

- [ ] **Step 1: Write the failing acceptance test**

Create `elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs`. The harness pattern (Ajv2020, `assert`, `loadJson`, helper fixtures) is copied from `test-manifest-schema.mjs` — same conventions, no Vitest.

```js
#!/usr/bin/env node
/**
 * Tests the vocabulary.quiltPolicies extension (tiered-quilt §4, amended 2026-06-04):
 * named declarative storage-policy classes + per-contentType references.
 * Schema validates SHAPE; referential integrity is loader-enforced
 * (see lib/manifest-quilt-refs.mjs, tested below in the REF CHECKS section).
 */
import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
let failures = 0;
let passes = 0;

function assert(condition, message) {
  if (!condition) {
    console.error(`FAIL: ${message}`);
    failures++;
  } else {
    console.log(`PASS: ${message}`);
    passes++;
  }
}

async function loadJson(filepath) {
  return JSON.parse(await readFile(filepath, 'utf8'));
}

function minimalCoupling() {
  return {
    value: {
      onConsume: { action: 'use' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
    },
    claims: [
      { asserts: 'comprehension', contradictedBy: 'comprehension-failure', validityHorizon: 'P30D' },
    ],
  };
}

function minimalObservations() {
  return {
    comprehension: { description: 'Learner demonstrated comprehension', instrument: 'retention-check', polarity: 'positive' },
    'comprehension-failure': { description: 'Learner failed to demonstrate comprehension', instrument: 'retention-check', polarity: 'negative' },
  };
}

/** Spec §4 example policies — the streaming class exercises every field. */
function quiltPolicies() {
  return {
    'long-term-personal': {
      defaultTierFloor: 'stocked',
      shelveAfter: '30d',
      holdWarmMin: '7d',
      preferDestinations: [
        'federated-dwelling://family/{family-id}',
        'peer-cellar://household/{any}',
      ],
    },
    'streaming-media-library': {
      defaultTierFloor: 'shelved',
      holdWarmMin: '2h',
      shelveAfter: '7d',
      drawLatencyBudget: '2s',
      draw: 'streamed',
      preferDestinations: ['peer-cellar://household/{any}'],
    },
  };
}

function manifestWithQuiltPolicies() {
  return {
    id: 'bafkreiquiltexample',
    name: 'Quilt Policy Test App',
    version: '1.0.0',
    vocabulary: {
      quiltPolicies: quiltPolicies(),
      quiltPolicyDefault: 'long-term-personal',
      contentTypes: {
        'photo-album': {
          description: 'A family photo album',
          coupling: minimalCoupling(),
          quiltPolicy: 'long-term-personal',
        },
        'family-video': {
          description: 'A family video',
          coupling: minimalCoupling(),
          quiltPolicy: 'streaming-media-library',
        },
      },
      observations: minimalObservations(),
    },
  };
}

async function main() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });

  // Load referenced schemas so AJV can resolve $ref — mirror the addSchema
  // block from test-manifest-schema.mjs main() exactly (run
  // `grep -n addSchema elohim/sdk/schemas/scripts/test-manifest-schema.mjs`
  // and replicate every line found there).
  const substrateSignalSchema = await loadJson(
    resolve(__dirname, '../v1/enums/substrate-signal.schema.json'),
  );
  ajv.addSchema(substrateSignalSchema, 'epr:enums/substrate-signal.schema.json');

  const instrumentArchetypeSchema = await loadJson(
    resolve(__dirname, '../v1/enums/instrument-archetype.schema.json'),
  );
  ajv.addSchema(instrumentArchetypeSchema, 'epr:enums/instrument-archetype.schema.json');

  const schema = await loadJson(resolve(__dirname, '../v1/manifest/app-manifest.schema.json'));
  const validate = ajv.compile(schema);

  // --- ACCEPTANCE ---
  {
    const valid = validate(manifestWithQuiltPolicies());
    if (!valid) console.error(JSON.stringify(validate.errors, null, 2));
    assert(valid, 'Accepts manifest with named quiltPolicies + per-type references + default');
  }
  {
    const m = manifestWithQuiltPolicies();
    delete m.vocabulary.quiltPolicies;
    delete m.vocabulary.quiltPolicyDefault;
    delete m.vocabulary.contentTypes['photo-album'].quiltPolicy;
    delete m.vocabulary.contentTypes['family-video'].quiltPolicy;
    assert(validate(m), 'quiltPolicies is fully optional — existing manifests stay valid');
  }

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
```

- [ ] **Step 2: Run it to verify it fails for the right reason**

Run: `node elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs`
Expected: `FAIL: Accepts manifest with named quiltPolicies…` — AJV errors must say `must NOT have additional properties` for `quiltPolicies` / `quiltPolicyDefault` / `quiltPolicy` (the `Vocabulary` and `ContentTypeDeclaration` $defs are `additionalProperties: false`). If it fails with a schema-load error instead, fix the addSchema block (Step 1 note) until the failure is the additional-properties rejection.

---

### Task 2: Schema extension (GREEN)

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`

- [ ] **Step 1: Add the two Vocabulary properties**

In the `"Vocabulary"` entry under `"$defs"` (it has `"additionalProperties": false` — locate `"stagedIntents"` inside its `"properties"`), add **after** the `stagedIntents` property:

```json
"quiltPolicies": {
  "type": "object",
  "minProperties": 1,
  "description": "Optional. Named declarative storage-policy classes (tiered-quilt §4, amended 2026-06-04). Content types reference them by name via contentTypes.<type>.quiltPolicy. The policy NAME is the cost class — qualified as <pillar>/<name> in cost_class event columns. Declarations are advisory desired-state a peer-local TierController reconciles toward; never an imperative command, never below a pledge-backed floor. Empty {} is not a meaningful declaration.",
  "propertyNames": {
    "pattern": "^[a-z][a-z0-9-]*$"
  },
  "additionalProperties": {
    "$ref": "#/$defs/QuiltPolicyDeclaration"
  }
},
"quiltPolicyDefault": {
  "type": "string",
  "description": "Optional. Name of the quiltPolicies entry that applies to content types that declare no quiltPolicy of their own. MUST reference a declared vocabulary.quiltPolicies key — referential integrity is enforced by the manifest loader (scripts/lib/manifest-quilt-refs.mjs), not expressible in this schema."
}
```

- [ ] **Step 2: Add the ContentTypeDeclaration property**

In the `"ContentTypeDeclaration"` entry under `"$defs"`, inside its `"properties"`, add:

```json
"quiltPolicy": {
  "type": "string",
  "description": "Optional. Name of a vocabulary.quiltPolicies entry governing stewarded custody of this type's payload blobs (derived projections — thumbnails, indexes — are Category C cache-core territory, never quilt-governed). Referential integrity is loader-enforced; a dangling name fails validation loud."
}
```

- [ ] **Step 3: Add the two new $defs**

Add to `"$defs"` (sibling of `Vocabulary` / `ContentTypeDeclaration`):

```json
"QuiltPolicyDeclaration": {
  "title": "QuiltPolicyDeclaration",
  "description": "A named declarative storage-policy class. Declares dynamics (floor + decay timings + draw QoS), not placement — the peer-local controller reconciles toward it and can veto (tier-below-floor-prevented). Spec: 2026-05-11-tiered-quilt-stewardship-design.md §4.",
  "type": "object",
  "required": ["defaultTierFloor"],
  "properties": {
    "defaultTierFloor": {
      "type": "string",
      "enum": ["drawn", "stocked-warm", "stocked", "shelved"],
      "description": "Minimum custody temperature for this class (tier order: shelved < stocked < stocked-warm < drawn). 'drawn' = no custody guarantee. Matched against a steward's pledge at CommitmentFactory negotiation — never silently degraded."
    },
    "shelveAfter": {
      "$ref": "#/$defs/QuiltDuration",
      "description": "Inactivity window after which content may demote toward the shelved tier (above the floor)."
    },
    "holdWarmMin": {
      "$ref": "#/$defs/QuiltDuration",
      "description": "Minimum stocked-warm dwell after a draw. Grants are capped at pledge capacity (the donut clamp applies to soft fields)."
    },
    "drawLatencyBudget": {
      "$ref": "#/$defs/QuiltDuration",
      "description": "Declared time-to-first-byte SLA. A shelf destination that cannot meet it is invalid for this class at negotiation time (the Glacier-retrieval-class analog)."
    },
    "draw": {
      "type": "string",
      "enum": ["atomic", "streamed"],
      "default": "atomic",
      "description": "streamed = ranged/progressive draw (iroh bao verified ranges); first bytes flow while the tail re-stocks. Drivers that cannot range fall back to atomic and log — the draw never fails for this reason."
    },
    "preferDestinations": {
      "type": "array",
      "items": { "type": "string", "minLength": 1 },
      "description": "Ordered shelf/cellar destination URI templates (peer-cellar://, federated-dwelling://, external-archive://)."
    }
  },
  "additionalProperties": false
},
"QuiltDuration": {
  "type": "string",
  "pattern": "^[0-9]+(ms|s|m|h|d)$",
  "description": "Compact duration literal: non-negative integer + unit (ms|s|m|h|d), e.g. \"5m\", \"30d\", \"2s\"."
}
```

- [ ] **Step 4: Run the acceptance test to verify it passes**

Run: `node elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs`
Expected: `2 passed, 0 failed`

- [ ] **Step 5: Run the existing manifest suite to verify nothing regressed**

Run: `pnpm run manifest:test && pnpm run schema:test`
Expected: all PASS (the extension is optional + additive).

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/v1/manifest/app-manifest.schema.json elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs
git commit -m "feat(schema): vocabulary.quiltPolicies named storage-policy classes (tiered-quilt §4 v0.2)"
```

---

### Task 3: Negative schema cases (tighten + prove each rejection)

**Files:**
- Modify: `elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs`

- [ ] **Step 1: Add the negative cases**

Append inside `main()` after the acceptance block (before the summary `console.log`):

```js
  // --- NEGATIVE: schema shape ---
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['streaming-media-library'].shelveAfter = '5 minutes';
    assert(!validate(m), 'Rejects non-compact duration ("5 minutes")');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['streaming-media-library'].defaultTierFloor = 'warm';
    assert(!validate(m), 'Rejects unknown tier name ("warm" is not a temperature class)');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies = {};
    assert(!validate(m), 'Rejects empty quiltPolicies {} (not a meaningful declaration)');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['streaming-media-library'].draw = 'progressive';
    assert(!validate(m), 'Rejects unknown draw mode ("progressive")');
  }
  {
    const m = manifestWithQuiltPolicies();
    delete m.vocabulary.quiltPolicies['long-term-personal'].defaultTierFloor;
    assert(!validate(m), 'Rejects policy without defaultTierFloor (the one required field)');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['long-term-personal'].costClassHint = 'long-term-personal';
    assert(!validate(m), 'Rejects retired costClassHint field (the policy name IS the cost class)');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['Bad_Name'] = { defaultTierFloor: 'stocked' };
    assert(!validate(m), 'Rejects non-kebab-case policy name (names become <pillar>/<name> cost classes)');
  }
```

- [ ] **Step 2: Run and verify all pass**

Run: `node elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs`
Expected: `9 passed, 0 failed`. If any negative case unexpectedly VALIDATES, the schema is too loose — fix the schema (Task 2 patterns/enums), not the test.

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs
git commit -m "test(schema): negative fixtures for quiltPolicies shape (duration/tier/draw/name/retired-field)"
```

---

### Task 4: Referential-integrity check (the loader-enforced rule)

**Files:**
- Create: `elohim/sdk/schemas/scripts/lib/manifest-quilt-refs.mjs`
- Modify: `elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs`

- [ ] **Step 1: Write the failing ref-check tests**

Append inside `main()` (before the summary):

```js
  // --- REF CHECKS (loader-enforced; not expressible in JSON Schema) ---
  const { validateQuiltPolicyRefs } = await import('./lib/manifest-quilt-refs.mjs');
  {
    const errs = validateQuiltPolicyRefs(manifestWithQuiltPolicies());
    assert(errs.length === 0, 'Ref check: clean manifest has zero ref errors');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.contentTypes['family-video'].quiltPolicy = 'streaming-media-libary'; // typo
    const errs = validateQuiltPolicyRefs(m);
    assert(
      errs.length === 1 && errs[0].includes('family-video') && errs[0].includes('streaming-media-libary'),
      'Ref check: typo’d contentType.quiltPolicy fails loud, naming type and ref',
    );
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicyDefault = 'does-not-exist';
    const errs = validateQuiltPolicyRefs(m);
    assert(
      errs.length === 1 && errs[0].includes('quiltPolicyDefault') && errs[0].includes('does-not-exist'),
      'Ref check: dangling quiltPolicyDefault fails loud',
    );
  }
  {
    const m = manifestWithQuiltPolicies();
    delete m.vocabulary.quiltPolicies;
    const errs = validateQuiltPolicyRefs(m);
    assert(
      errs.length === 3,
      'Ref check: references with NO quiltPolicies section at all → every reference is dangling (2 types + default)',
    );
  }
  {
    const m = manifestWithQuiltPolicies();
    delete m.vocabulary.quiltPolicies;
    delete m.vocabulary.quiltPolicyDefault;
    delete m.vocabulary.contentTypes['photo-album'].quiltPolicy;
    delete m.vocabulary.contentTypes['family-video'].quiltPolicy;
    const errs = validateQuiltPolicyRefs(m);
    assert(errs.length === 0, 'Ref check: manifest without any quilt vocabulary is clean (fully optional)');
  }
```

- [ ] **Step 2: Run to verify it fails**

Run: `node elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs`
Expected: crash with `Cannot find module './lib/manifest-quilt-refs.mjs'` — the right missing-API failure.

- [ ] **Step 3: Implement the check**

Create `elohim/sdk/schemas/scripts/lib/manifest-quilt-refs.mjs`:

```js
/**
 * Referential-integrity check for vocabulary.quiltPolicies references
 * (tiered-quilt §4, amended 2026-06-04).
 *
 * JSON Schema cannot express cross-key references, and the staged-intents
 * substrate documented exactly where that bites: a typo'd name "passes
 * manifest validation and fails later at runtime lookup". This check is the
 * loader-enforced rule that closes that trap for quilt policies: every
 * contentTypes.<type>.quiltPolicy and vocabulary.quiltPolicyDefault MUST name
 * a declared vocabulary.quiltPolicies key.
 *
 * Called by test-manifest-quilt-policy.mjs AND codegen-manifest.mjs (fails
 * codegen loud on a dangling reference).
 *
 * @param {object} manifest - parsed app manifest
 * @returns {string[]} human-readable errors; empty array = clean
 */
export function validateQuiltPolicyRefs(manifest) {
  const errors = [];
  const vocab = manifest?.vocabulary ?? {};
  const declared = new Set(Object.keys(vocab.quiltPolicies ?? {}));
  const dangling = (ref) => !declared.has(ref);

  if (vocab.quiltPolicyDefault !== undefined && dangling(vocab.quiltPolicyDefault)) {
    errors.push(
      `vocabulary.quiltPolicyDefault "${vocab.quiltPolicyDefault}" references no declared vocabulary.quiltPolicies entry`,
    );
  }
  for (const [typeName, decl] of Object.entries(vocab.contentTypes ?? {})) {
    const ref = decl?.quiltPolicy;
    if (ref !== undefined && dangling(ref)) {
      errors.push(
        `vocabulary.contentTypes.${typeName}.quiltPolicy "${ref}" references no declared vocabulary.quiltPolicies entry`,
      );
    }
  }
  return errors;
}
```

- [ ] **Step 4: Run to verify all pass**

Run: `node elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs`
Expected: `14 passed, 0 failed`

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/scripts/lib/manifest-quilt-refs.mjs elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs
git commit -m "feat(schema): loader-enforced quiltPolicy referential integrity (closes the staged-intents typo-trap shape)"
```

---

### Task 5: Wire the ref check into manifest codegen

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-manifest.mjs`

- [ ] **Step 1: Add the import + gate**

In `codegen-manifest.mjs`, add to the imports at the top:

```js
import { validateQuiltPolicyRefs } from './lib/manifest-quilt-refs.mjs';
```

Inside `main()`, immediately after `const manifest = JSON.parse(raw);`, add:

```js
  // Loader-enforced referential integrity (tiered-quilt §4 v0.2): a dangling
  // quiltPolicy reference must fail codegen loud, never silently not-apply.
  const quiltRefErrors = validateQuiltPolicyRefs(manifest);
  if (quiltRefErrors.length > 0) {
    console.error('Manifest quilt-policy referential-integrity errors:');
    for (const e of quiltRefErrors) console.error(`  - ${e}`);
    process.exit(1);
  }
```

- [ ] **Step 2: Verify codegen stays green on the real manifest**

Run: `pnpm run manifest:codegen:verify && pnpm run lamad:codegen:verify`
Expected: both PASS unchanged (the lamad manifest declares no quiltPolicies → zero ref errors, zero generated-output drift).

- [ ] **Step 3: Verify the gate actually fires**

Run (creates a throwaway dangling-ref fixture, expects exit 1, then cleans up):

```bash
node -e "
const fs = require('fs');
const m = JSON.parse(fs.readFileSync('elohim/sdk/domains/lamad/manifest.json', 'utf8'));
m.vocabulary.quiltPolicyDefault = 'does-not-exist';
fs.mkdirSync('/tmp/quilt-gate-check', { recursive: true });
fs.writeFileSync('/tmp/quilt-gate-check/manifest.json', JSON.stringify(m));
"
node elohim/sdk/schemas/scripts/codegen-manifest.mjs /tmp/quilt-gate-check/manifest.json /tmp/quilt-gate-check/out.ts
echo "exit=$?"
rm -rf /tmp/quilt-gate-check
```

Expected: `Manifest quilt-policy referential-integrity errors:` + `exit=1`. (Note: `codegen-manifest.mjs` resolves paths against the repo root — if the `/tmp` path does not resolve, pass a repo-relative fixture path instead, e.g. write the fixture to `elohim/sdk/schemas/scripts/.quilt-gate-check.json` and delete it after.)

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-manifest.mjs
git commit -m "feat(codegen): fail manifest codegen loud on dangling quiltPolicy references"
```

---

### Task 6: Wire into the test gate + close the loop

**Files:**
- Modify: `package.json` (repo root, line ~53)
- Modify (local state, not committed): `.claude/memory-kit/gap-items/architecture__2026-05-11-tiered-quilt-stewardship-design.json`

- [ ] **Step 1: Chain the new test into manifest:test**

In root `package.json`, change:

```json
"manifest:test": "node elohim/sdk/schemas/scripts/test-manifest-schema.mjs",
```

to:

```json
"manifest:test": "node elohim/sdk/schemas/scripts/test-manifest-schema.mjs && node elohim/sdk/schemas/scripts/test-manifest-quilt-policy.mjs",
```

- [ ] **Step 2: Run the full gate**

Run: `pnpm run manifest:test && pnpm run schema:test && pnpm run manifest:codegen:verify`
Expected: all PASS.

- [ ] **Step 3: Commit**

```bash
git add package.json
git commit -m "chore(schema): chain quilt-policy fixtures into manifest:test gate"
```

- [ ] **Step 4: Flip gap-item states (local, no commit)**

In `.claude/memory-kit/gap-items/architecture__2026-05-11-tiered-quilt-stewardship-design.json`, set `"state": "CLAIMED"` on items `#1` (schema shape), `#4` (referential integrity), `#5` (drawLatencyBudget + draw fields), `#12` (fixture tests + codegen round-trip). Leave `#2/#3/#6–#11` OPEN (HELD with the tier runtime). Then run `python3 .claude/scripts/memory-kit/placement-audit.py --ledger | head -16` to confirm the budget reflects it.

---

## Self-Review (run before handing off)

1. **Spec coverage:** §4 amendment schema-substrate claims → Task 2 (shape), Task 4 (referential integrity), Task 3+5 (fixtures + codegen gate), Task 6 (gate wiring). Classifier/negotiation/SDK claims explicitly out of scope (HELD).
2. **Placeholder scan:** the one open-ended instruction is the addSchema mirror in Task 1 Step 1 — it names the exact grep to resolve it; everything else is verbatim code.
3. **Type consistency:** field names `defaultTierFloor` / `shelveAfter` / `holdWarmMin` / `drawLatencyBudget` / `draw` / `preferDestinations` / `quiltPolicies` / `quiltPolicyDefault` / `quiltPolicy` are identical across schema (Task 2), fixtures (Tasks 1/3/4), and the spec §4 examples; `validateQuiltPolicyRefs` is the single exported symbol used in Tasks 4 and 5.
