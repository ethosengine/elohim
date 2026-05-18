# App-Manifest Modularization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `elohim/sdk/domains/lamad/manifest.json` (1,923 LOC) into per-concern files referenced via `$ref` from a thin manifest shell, with byte-identical generated TypeScript outputs verified per task. Pattern documented for the other 7 domains as follow-on.

**Architecture:** The manifest schema (`elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`) already supports `$ref` for sub-documents — used today by `metadataSchema` references. We extend the same pattern to `vocabulary.contentTypes`, `signalKinds`, `projections`, and `graph`. The loader/codegen reads the shell, resolves `$ref` pointers transparently, and produces the same in-memory manifest tree as before. Generated TypeScript stays byte-identical.

**Tech Stack:** Node.js (codegen), JSON Schema 2020-12, AJV-style $ref resolution, the existing `pnpm run lamad:codegen` and `pnpm run schema:codegen:ts` pipelines.

---

## Pre-execution gate (do once before Task 1)

- [ ] Confirm `lamad/manifest.json` LOC is still ~1,923: `wc -l /projects/elohim/elohim/sdk/domains/lamad/manifest.json`
- [ ] Confirm `pnpm run lamad:codegen` produces output today: run it, capture the generated files' state with `git status app/elohim-app/src/app/lamad/generated/ genesis/seeder/src/generated/`
- [ ] Confirm the lamad codegen script at `elohim/sdk/domains/lamad/scripts/codegen.mjs` (568 LOC) is the only consumer of the manifest's direct structure (vs the schema validator which works at the schema layer)
- [ ] Confirm the existing $ref pattern at lines 11, 135, 236, 276, 452 of `lamad/manifest.json` (metadataSchema refs to `./schemas/*.schema.json`) is what we're generalizing

## File Structure

**Files to be created:**
```
elohim/sdk/domains/lamad/manifest/
├── content-types/
│   ├── concept.json              (~120 LOC — currently lines 9-127 of manifest.json)
│   ├── exercise.json             (~70 LOC)
│   ├── assessment.json           (~100 LOC)
│   ├── lesson.json               (~80 LOC)
│   ├── article.json              (~70 LOC)
│   ├── path.json                 (~80 LOC)
│   ├── experience-story.json     (~50 LOC)
│   ├── experience-moment.json    (~50 LOC)
│   ├── chapter.json              (~50 LOC)
│   ├── book.json                 (~50 LOC)
│   ├── gate-process-declaration.json   (~80 LOC)
│   ├── universal-band-declaration.json (~80 LOC)
│   ├── gate-rules-declaration.json     (~80 LOC)
│   ├── aggregation-spec.json           (~50 LOC)
│   ├── escalation-target-spec.json     (~50 LOC)
│   └── (one file per content type — total ~17 files based on existing manifest)
├── formats.json                  (~50 LOC — currently the vocabulary.formats block)
├── signal-kinds.json             (~50 LOC — currently the signalKinds block)
├── projections.json              (~100 LOC — currently the projections block)
├── graph.json                    (~80 LOC — currently the graph block)
└── rendering.json                (~60 LOC — currently the rendering block)
```

**Files to be modified:**
- `elohim/sdk/domains/lamad/manifest.json` — reduced to <300 LOC shell with `$ref` pointers
- `elohim/sdk/domains/lamad/scripts/codegen.mjs` — add $ref resolution for top-level keys
- `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` — document that contentTypes/signalKinds/projections/graph values MAY be `$ref` strings or inline objects (already true; just clarify)
- `elohim/sdk/CLAUDE.md` — document the modularization convention

**Files NOT touched (guard against scope creep):**
- Any other domain manifest (shefa/qahal/imagodei/etc.) — pattern only, applied to lamad here
- The schema validator at `elohim/sdk/schemas/scripts/test-manifest-schema.mjs` — should work as-is
- The Rust loaders at `elohim-storage/tests/lamad_manifest_registration.rs` — JSON shape unchanged at parse time
- Generated TypeScript output files (`app/elohim-app/src/app/lamad/generated/`, `genesis/seeder/src/generated/`) — these MUST be byte-identical after each task

---

## Task 1: Add $ref resolution helper to lamad codegen

**Files:**
- Modify: `elohim/sdk/domains/lamad/scripts/codegen.mjs`

The codegen reads the manifest as a single JSON object today. To support modular manifests, we add a `resolveRefs` helper that walks the loaded JSON and inlines any `{ "$ref": "./relative/path.json" }` it finds. This is additive — manifests that don't use `$ref` work unchanged.

- [ ] **Step 1: Read the current codegen.mjs structure**

Run: `head -40 /projects/elohim/elohim/sdk/domains/lamad/scripts/codegen.mjs`

You should see imports, `MANIFEST_PATH` resolution, and `OUTPUT_DIRS`.

- [ ] **Step 2: Add the resolveRefs helper just below the imports**

Edit `elohim/sdk/domains/lamad/scripts/codegen.mjs`. Find the line `const VERIFY = process.argv.includes('--verify');` and insert AFTER it:

```javascript
/**
 * Recursively resolve $ref pointers in a JSON value.
 * - $ref strings that look like relative paths ("./...", "../...") are loaded
 *   from disk relative to `baseDir` and recursively resolved.
 * - $ref strings starting with "#" (JSON Pointer fragments) are left as-is —
 *   the manifest schema validator handles those.
 * - All other values are walked recursively.
 *
 * @param {*} value      Any JSON value
 * @param {string} baseDir  Absolute directory the current document was loaded from
 * @returns {Promise<*>} The value with relative $refs inlined
 */
async function resolveRefs(value, baseDir) {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) {
    return Promise.all(value.map((v) => resolveRefs(v, baseDir)));
  }
  if (typeof value.$ref === 'string' && (value.$ref.startsWith('./') || value.$ref.startsWith('../'))) {
    const refPath = resolve(baseDir, value.$ref);
    const raw = JSON.parse(await readFile(refPath, 'utf8'));
    return resolveRefs(raw, dirname(refPath));
  }
  const out = {};
  for (const [k, v] of Object.entries(value)) {
    out[k] = await resolveRefs(v, baseDir);
  }
  return out;
}
```

- [ ] **Step 3: Wire resolveRefs into the manifest load path**

In `codegen.mjs`, find where the manifest is loaded — search for `MANIFEST_PATH` usage. The current pattern is likely:

```javascript
const manifest = JSON.parse(await readFile(MANIFEST_PATH, 'utf8'));
```

Replace it with:

```javascript
const manifestRaw = JSON.parse(await readFile(MANIFEST_PATH, 'utf8'));
const manifest = await resolveRefs(manifestRaw, dirname(MANIFEST_PATH));
```

Use Read tool first to find the EXACT current line, then Edit.

- [ ] **Step 4: Verify codegen still produces byte-identical output**

```bash
cd /projects/elohim && pnpm run lamad:codegen 2>&1 | tail -5
git diff --no-color app/elohim-app/src/app/lamad/generated/ genesis/seeder/src/generated/
```

Expected: no diff. The manifest hasn't changed yet (no $refs added), so the additive helper should be a no-op.

If there IS a diff, STOP. The helper introduced a regression. Inspect the diff and fix before continuing.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/lamad/scripts/codegen.mjs
git commit -m "feat(lamad/codegen): add resolveRefs helper for modular manifests

Walks the loaded manifest JSON inlining \$ref strings that point to
relative .json paths. Additive — no behavior change until manifest
introduces top-level \$refs in later tasks.

Refs: genesis/docs/plans/2026-05-18-app-manifest-modularization.md (T1)"
```

---

## Task 2: Add codegen --verify path for the loader

**Files:**
- Modify: `elohim/sdk/domains/lamad/scripts/codegen.mjs`

Verify that the codegen `--verify` flag (used by pre-push hook) also exercises the resolveRefs path. This ensures CI catches drift after future $ref additions.

- [ ] **Step 1: Run the verify path to baseline**

```bash
cd /projects/elohim && node elohim/sdk/domains/lamad/scripts/codegen.mjs --verify 2>&1 | tail -5
echo "Exit: $?"
```

Expected: exit 0 (no drift), or non-zero if generated files are stale. Either way, capture the current behavior.

- [ ] **Step 2: Inspect the verify branch**

Run: `grep -n "VERIFY" /projects/elohim/elohim/sdk/domains/lamad/scripts/codegen.mjs`

Confirm `VERIFY` is read once at process top and used to decide write-vs-diff in the codegen body. No changes needed if the verify branch already runs the same load path as the write branch — which it should, since we wired resolveRefs into the shared load.

- [ ] **Step 3: Add a smoke assertion (no commit yet — bundle with T3)**

Add a `console.log` at the top of the script (transient — removed at end):

```javascript
console.error(`[lamad codegen] resolveRefs ready; manifest = ${MANIFEST_PATH}`);
```

This will be removed in T3.

- [ ] **Step 4: Verify the smoke log appears in both paths**

```bash
cd /projects/elohim && node elohim/sdk/domains/lamad/scripts/codegen.mjs 2>&1 | head -2
cd /projects/elohim && node elohim/sdk/domains/lamad/scripts/codegen.mjs --verify 2>&1 | head -2
```

Both should print the resolveRefs-ready line. This confirms the helper is in the shared load path.

- [ ] **Step 5: Remove the smoke log**

Delete the `console.error` you added in Step 3.

- [ ] **Step 6: No commit for this task — proceed directly to T3**

(T2 is a verification pass; bundles into T3's commit.)

---

## Task 3: Extract `lamad/manifest/content-types/concept.json`

**Files:**
- Create: `elohim/sdk/domains/lamad/manifest/content-types/concept.json`
- Modify: `elohim/sdk/domains/lamad/manifest.json` (replace inline contentTypes.concept with `$ref`)

The "concept" content type is the first to be extracted as a proof of concept. It's the largest contentType definition in the manifest and the most-referenced.

- [ ] **Step 1: Capture the current concept block**

Read `elohim/sdk/domains/lamad/manifest.json`, lines 7-127 (the `"concept": {` block). Note the exact opening and closing line numbers.

Run: `awk 'NR>=7 && NR<=127' /projects/elohim/elohim/sdk/domains/lamad/manifest.json | head -5`

Confirm the first line is `"concept": {` and the last line at 127 is `},` or `}` (the closing brace).

If the line numbers have drifted from this plan, use Read tool to find the exact range and adjust below.

- [ ] **Step 2: Create the per-type directory and concept file**

```bash
mkdir -p /projects/elohim/elohim/sdk/domains/lamad/manifest/content-types
```

Then create `/projects/elohim/elohim/sdk/domains/lamad/manifest/content-types/concept.json` containing the inner JSON of the concept block — i.e. everything from `{` to its matching `}` from the manifest, but NOT including the `"concept":` key, NOT including a trailing comma.

For example, if the manifest has:

```json
"concept": {
  "description": "A fundamental idea ...",
  ...
},
```

The extracted file is:

```json
{
  "description": "A fundamental idea ...",
  ...
}
```

(Object value only, no key wrapper, no trailing comma.)

- [ ] **Step 3: Replace the inline block with a $ref in manifest.json**

In `elohim/sdk/domains/lamad/manifest.json`, replace the entire concept block (lines 7-127 or wherever it currently is) with:

```json
      "concept": { "$ref": "./manifest/content-types/concept.json" },
```

Preserve the leading indentation (6 spaces, as the concept key is inside `vocabulary.contentTypes`).

- [ ] **Step 4: Run codegen — verify byte-identical generated output**

```bash
cd /projects/elohim && pnpm run lamad:codegen 2>&1 | tail -5
git diff --no-color app/elohim-app/src/app/lamad/generated/ genesis/seeder/src/generated/
```

Expected: no diff. The resolveRefs helper inlines the concept.json content during load, producing the same in-memory manifest as before.

If there IS a diff, STOP. Either the extraction lost a field, gained whitespace, or the resolveRefs helper has a bug. Inspect the diff to determine which.

- [ ] **Step 5: Verify the manifest still validates against its schema**

```bash
cd /projects/elohim && pnpm run schema:test 2>&1 | tail -10
```

Expected: tests pass. The schema validator typically uses the raw JSON (with `$ref` unresolved), so this exercises that the schema layer treats `$ref` correctly (the meta-schema's $ref keyword is permitted at any position).

If the validator REJECTS the manifest because it expects an inline object at `vocabulary.contentTypes.concept`, the schema needs a tiny tweak in Task 9 — note the error message but do NOT change the schema yet; continue with the extractions and fix once.

- [ ] **Step 6: Commit (bundles T1+T2+T3)**

```bash
git add elohim/sdk/domains/lamad/scripts/codegen.mjs \
        elohim/sdk/domains/lamad/manifest.json \
        elohim/sdk/domains/lamad/manifest/content-types/concept.json
git commit -m "refactor(lamad/manifest): extract concept content type to manifest/content-types/concept.json

First extraction of the manifest modularization sprint. The resolveRefs
helper added in T1 inlines the new \$ref transparently during codegen
load; generated TypeScript outputs are byte-identical.

Refs: genesis/docs/plans/2026-05-18-app-manifest-modularization.md (T1+T2+T3)"
```

---

## Task 4: Extract remaining contentTypes (batched per natural group)

**Files:**
- Create: `elohim/sdk/domains/lamad/manifest/content-types/{exercise,assessment,lesson,article,path,experience-story,experience-moment,chapter,book,gate-process-declaration,universal-band-declaration,gate-rules-declaration,aggregation-spec,escalation-target-spec}.json` (one file per content type)
- Modify: `elohim/sdk/domains/lamad/manifest.json` (replace each inline block with `$ref`)

Repeat the T3 procedure for each remaining contentType. The lamad manifest has approximately 17 content types; T3 handled `concept`, this task handles the rest in a single commit (mechanical, repetitive, no judgment needed).

- [ ] **Step 1: Enumerate the remaining content types**

```bash
grep -n '^      "[a-z-]*": {' /projects/elohim/elohim/sdk/domains/lamad/manifest.json | head -25
```

The output lists keys at 6-space-indentation under vocabulary.contentTypes. The `concept` key is gone (replaced with `$ref`); the rest are the candidates.

Capture the list. Expected count: ~16.

- [ ] **Step 2: For each remaining content type, perform the same extraction**

For each `<type-name>` in the list:
  a. Read the inline block from the manifest (key + `:` + object value + `,` or `}`)
  b. Create `elohim/sdk/domains/lamad/manifest/content-types/<type-name>.json` containing the object value only
  c. Replace the inline block in the manifest with `      "<type-name>": { "$ref": "./manifest/content-types/<type-name>.json" },` (preserve trailing comma if not last)

You can do these sequentially or write a small one-shot script to /tmp. If you script it, do NOT commit the script — it's a transient extraction tool.

- [ ] **Step 3: Run codegen — verify byte-identical generated output**

```bash
cd /projects/elohim && pnpm run lamad:codegen 2>&1 | tail -5
git diff --no-color app/elohim-app/src/app/lamad/generated/ genesis/seeder/src/generated/
```

Expected: no diff.

If diff is non-empty, the most likely cause is a misplaced comma in the manifest or a malformed extracted file. Inspect the diff to identify which.

- [ ] **Step 4: Verify manifest still validates**

```bash
cd /projects/elohim && pnpm run schema:test 2>&1 | tail -10
```

Expected: pass.

- [ ] **Step 5: Verify manifest.json line count dropped substantially**

```bash
wc -l /projects/elohim/elohim/sdk/domains/lamad/manifest.json
```

Expected: dropped from 1,923 to something around 800-1,000 LOC (the contentTypes block was the bulk).

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/domains/lamad/manifest.json \
        elohim/sdk/domains/lamad/manifest/content-types/
git commit -m "refactor(lamad/manifest): extract remaining content types to per-type files

Each contentType definition now lives in its own file under
manifest/content-types/. The shell manifest.json references each via \$ref.
Generated TypeScript outputs remain byte-identical.

Refs: genesis/docs/plans/2026-05-18-app-manifest-modularization.md (T4)"
```

---

## Task 5: Extract signalKinds to its own file

**Files:**
- Create: `elohim/sdk/domains/lamad/manifest/signal-kinds.json`
- Modify: `elohim/sdk/domains/lamad/manifest.json`

The `signalKinds` block extends the FeedbackSignal vocabulary. It's a single top-level key with a flat structure.

- [ ] **Step 1: Locate the signalKinds block**

```bash
grep -n '"signalKinds"' /projects/elohim/elohim/sdk/domains/lamad/manifest.json
```

You'll get the opening line. Read enough context to find the closing `}` of its value object.

- [ ] **Step 2: Extract to a new file**

Create `elohim/sdk/domains/lamad/manifest/signal-kinds.json` with the OBJECT VALUE of `signalKinds` (no key wrapper):

Example: if the manifest has `"signalKinds": { "mastery-credit": {...}, "endorsement": {...} },` the new file contains `{ "mastery-credit": {...}, "endorsement": {...} }`.

- [ ] **Step 3: Replace inline with $ref**

In the manifest, replace the inline signalKinds block with:

```json
  "signalKinds": { "$ref": "./manifest/signal-kinds.json" },
```

(Preserve trailing comma; preserve 2-space indentation since signalKinds is at the top level.)

- [ ] **Step 4: Verify codegen output unchanged**

```bash
cd /projects/elohim && pnpm run lamad:codegen 2>&1 | tail -5
git diff --no-color app/elohim-app/src/app/lamad/generated/ genesis/seeder/src/generated/
```

Expected: no diff.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/lamad/manifest.json elohim/sdk/domains/lamad/manifest/signal-kinds.json
git commit -m "refactor(lamad/manifest): extract signalKinds to manifest/signal-kinds.json

Refs: genesis/docs/plans/2026-05-18-app-manifest-modularization.md (T5)"
```

---

## Task 6: Extract projections to its own file

**Files:**
- Create: `elohim/sdk/domains/lamad/manifest/projections.json`
- Modify: `elohim/sdk/domains/lamad/manifest.json`

The `projections` block is an array of EPR→pillar SQL projection declarations. Same pattern as T5.

- [ ] **Step 1: Locate the projections block**

```bash
grep -n '"projections"' /projects/elohim/elohim/sdk/domains/lamad/manifest.json
```

- [ ] **Step 2: Extract to file**

Note: `projections` is an ARRAY (not an object). The extracted file holds the array:

```json
[
  { "kind": "...", ... },
  { "kind": "...", ... }
]
```

The shell manifest's `$ref` resolver inlines arrays the same way it inlines objects.

- [ ] **Step 3: Replace inline with $ref**

```json
  "projections": { "$ref": "./manifest/projections.json" },
```

Wait — the issue here is that JSON Schema `$ref` for an array type works, but the in-place pattern `{ "$ref": "..." }` only works when the target is loaded as JSON and the value substituted. The resolveRefs helper from T1 handles this: it returns whatever the $ref target evaluates to. If the file contains an array, the helper returns the array; the shell's `"projections":` key then holds that array.

So the line in the manifest is:

```json
  "projections": { "$ref": "./manifest/projections.json" },
```

And the resolveRefs helper substitutes the array in. Verify this works.

- [ ] **Step 4: Verify codegen output unchanged**

```bash
cd /projects/elohim && pnpm run lamad:codegen 2>&1 | tail -5
git diff --no-color app/elohim-app/src/app/lamad/generated/ genesis/seeder/src/generated/
```

Expected: no diff. If the resolveRefs helper fails on `{$ref}` returning an array (because the parent expected an object key set), STOP — the helper needs to handle this case. The Step-2 code in T1 returns whatever the $ref target's JSON parses to, so arrays should work; if not, fix the helper.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/lamad/manifest.json elohim/sdk/domains/lamad/manifest/projections.json
git commit -m "refactor(lamad/manifest): extract projections to manifest/projections.json

Refs: genesis/docs/plans/2026-05-18-app-manifest-modularization.md (T6)"
```

---

## Task 7: Extract graph extension to its own file

**Files:**
- Create: `elohim/sdk/domains/lamad/manifest/graph.json`
- Modify: `elohim/sdk/domains/lamad/manifest.json`

The `graph` extension declares domain edge types, node augmentations, indexes, and Datalog rules (per 2026-05-16 graph-native-projection-substrate spec).

- [ ] **Step 1: Locate the graph block**

```bash
grep -n '"graph": {' /projects/elohim/elohim/sdk/domains/lamad/manifest.json
```

- [ ] **Step 2: Extract**

Create `elohim/sdk/domains/lamad/manifest/graph.json` with the object value.

- [ ] **Step 3: Replace inline with $ref**

```json
  "graph": { "$ref": "./manifest/graph.json" },
```

- [ ] **Step 4: Verify codegen + schema validation**

```bash
cd /projects/elohim && pnpm run lamad:codegen 2>&1 | tail -5
git diff --no-color app/elohim-app/src/app/lamad/generated/ genesis/seeder/src/generated/
cd /projects/elohim && pnpm run schema:test 2>&1 | tail -10
```

Expected: no diff; tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/lamad/manifest.json elohim/sdk/domains/lamad/manifest/graph.json
git commit -m "refactor(lamad/manifest): extract graph extension to manifest/graph.json

Refs: genesis/docs/plans/2026-05-18-app-manifest-modularization.md (T7)"
```

---

## Task 8: Extract rendering, formats, observation_kinds, writeThrough (if present)

**Files:**
- Create: `elohim/sdk/domains/lamad/manifest/{rendering,formats,observation-kinds,write-through}.json` (only if present)
- Modify: `elohim/sdk/domains/lamad/manifest.json`

Apply the same recipe to any remaining top-level blocks. Some may not exist in lamad's manifest — skip those.

- [ ] **Step 1: Enumerate remaining top-level keys**

```bash
grep -n '^  "[a-zA-Z_]*":' /projects/elohim/elohim/sdk/domains/lamad/manifest.json | head -20
```

Top-level keys not yet extracted: `id`, `name`, `version`, `description`, `vocabulary` (formats subkey may remain inline), `rendering`, `writeThrough`, `observation_kinds`.

Top-level scalars (`id`, `name`, `version`, `description`) STAY inline — they're shell-identity, not concern data.

- [ ] **Step 2: For each non-scalar remaining block (rendering, writeThrough, observation_kinds, formats if it's substantive)**

Apply the same recipe: extract object/array value to a per-concern file under `manifest/`, replace inline with `$ref`.

If a block is small (<20 LOC and only one such block), the operator may choose to keep it inline — judgment call. Default rule: extract anything ≥30 LOC.

- [ ] **Step 3: Verify codegen + schema validation**

```bash
cd /projects/elohim && pnpm run lamad:codegen 2>&1 | tail -5
git diff --no-color app/elohim-app/src/app/lamad/generated/ genesis/seeder/src/generated/
cd /projects/elohim && pnpm run schema:test 2>&1 | tail -10
```

Expected: no diff; tests pass.

- [ ] **Step 4: Verify final manifest.json shell line count**

```bash
wc -l /projects/elohim/elohim/sdk/domains/lamad/manifest.json
```

Expected: <300 LOC. The shell is now identity metadata + $ref pointers + the vocabulary key wrapper + the formats subkey if you kept it inline.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/domains/lamad/manifest.json elohim/sdk/domains/lamad/manifest/
git commit -m "refactor(lamad/manifest): extract remaining top-level concerns to manifest/

Lamad manifest shell now sits under 300 LOC of identity + \$ref pointers.
Each concern (contentTypes, signalKinds, projections, graph, rendering,
etc.) lives in its own file under manifest/.

Refs: genesis/docs/plans/2026-05-18-app-manifest-modularization.md (T8)"
```

---

## Task 9: Schema-side documentation of the $ref convention

**Files:**
- Modify: `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`
- Modify: `elohim/sdk/CLAUDE.md`

JSON Schema 2020-12 allows `$ref` at any position by default, so the schema doesn't technically need changes. But we document the convention so future contributors know modular manifests are first-class.

- [ ] **Step 1: Add a comment block to app-manifest.schema.json (in `description`)**

Read `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` and find the top-level `"description"` field (line ~6).

Append to the description:

```
Manifest values MAY be inlined or referenced via { "$ref": "./relative/path.json" } pointers — the schema validator and the codegen loader both inline references during load. The recommended convention is to keep the shell manifest under 300 LOC and split each concern (contentTypes, signalKinds, projections, graph, etc.) into its own file under a sibling `manifest/` directory. See elohim/sdk/domains/lamad/ for the canonical example.
```

- [ ] **Step 2: Add a section to elohim/sdk/CLAUDE.md**

If `elohim/sdk/CLAUDE.md` doesn't already discuss manifest structure, add a new section near the end:

```markdown
## Modular manifests (lamad pattern)

For app manifests that exceed ~500 LOC, split each top-level concern into a sibling `manifest/` directory and reference via `$ref`:

```
elohim/sdk/domains/<app>/
├── manifest.json                # shell — <300 LOC of identity + $refs
└── manifest/
    ├── content-types/<name>.json
    ├── signal-kinds.json
    ├── projections.json
    ├── graph.json
    └── rendering.json
```

The lamad manifest at `elohim/sdk/domains/lamad/manifest.json` is the canonical example. Codegen and validators resolve `$ref` transparently during load.
```

- [ ] **Step 3: Commit**

```bash
git add elohim/sdk/schemas/v1/manifest/app-manifest.schema.json elohim/sdk/CLAUDE.md
git commit -m "docs(manifest): document the modular-manifest \$ref convention

Refs: genesis/docs/plans/2026-05-18-app-manifest-modularization.md (T9)"
```

---

## Task 10: End-to-end verification + pre-push gate

**Files:**
- No file changes — validation only

Final pass to confirm the entire pipeline works against the split manifest.

- [ ] **Step 1: Fresh codegen run**

```bash
cd /projects/elohim && pnpm run lamad:codegen 2>&1 | tail -10
```

Expected: clean run, no errors.

- [ ] **Step 2: Codegen verify flag (drift detector)**

```bash
cd /projects/elohim && pnpm run lamad:codegen:verify 2>&1 | tail -5
echo "Exit: $?"
```

Expected: exit 0 (no drift).

- [ ] **Step 3: Schema test suite**

```bash
cd /projects/elohim && pnpm run schema:test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 4: Rust-side manifest registration tests**

```bash
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev cargo test --test lamad_manifest_registration 2>&1 | tail -15
```

Expected: tests pass. The Rust validator reads the manifest as JSON; resolveRefs is the codegen's job, so on the Rust side the `$ref` strings appear unresolved. If the Rust validator REJECTS the manifest because it doesn't understand `$ref` at non-schema positions, that's expected — file a follow-up to extend the Rust loader with the same resolveRefs walk, OR pre-resolve the manifest to a snapshot file that Rust consumes. NOTE this if it happens; do not block this plan on it (the Rust loader fix is its own ticket).

- [ ] **Step 5: Husky pre-push smoke**

```bash
cd /projects/elohim && bash .husky/pre-push 2>&1 | tail -30
```

Expected: clean run. If lamad-related gates flag staleness, run codegen again and re-test.

- [ ] **Step 6: Final LOC report**

```bash
echo "=== Shell manifest.json ==="
wc -l /projects/elohim/elohim/sdk/domains/lamad/manifest.json
echo "=== Per-concern files ==="
find /projects/elohim/elohim/sdk/domains/lamad/manifest -type f -name "*.json" | sort | xargs wc -l
echo "=== Total ==="
find /projects/elohim/elohim/sdk/domains/lamad/manifest -type f -name "*.json" -o -name "manifest.json" | xargs wc -l | tail -1
```

Expected:
- Shell manifest.json: <300 LOC (was 1,923)
- Per-concern files: each <500 LOC, most <200 LOC
- Total LOC: roughly equivalent to original 1,923 (split doesn't change content, just distributes it)

- [ ] **Step 7: No commit — this is verification only.**

If anything failed, return to the failing task and fix.

---

## Task 11: (Optional) Apply pattern to next-largest domain

**Files:**
- Create: `elohim/sdk/domains/shefa/manifest/...`
- Modify: `elohim/sdk/domains/shefa/manifest.json`

The shefa manifest is 447 LOC — much smaller than lamad but still a worthwhile target. Apply the same recipe (T3-T8) to shefa.

ONLY do this task if the operator explicitly requests it. The pattern is documented (T9); leaving the other domains for follow-on plans is fine.

- [ ] **Step 1-N: Repeat the T3-T8 procedure with shefa paths substituted**

(Skip if not requested.)

---

## Self-Review (already performed by plan author)

**Spec coverage:**
- Modular manifests via `$ref` — Tasks 1-8 (lamad)
- Codegen loader updated — Tasks 1, 2
- Schema-side documentation — Task 9
- End-to-end verification — Task 10
- Pattern documented for other domains — Task 9 (CLAUDE.md) + Task 11 (optional shefa application)

**Placeholder scan:** No `TBD`, `TODO`, `FILL IN`, or unimplemented references. Each task has concrete files, exact commands, expected outputs.

**Type consistency:** Function name used across tasks: `resolveRefs` (T1 defines, T1 consumes). Path naming: `manifest/content-types/<name>.json` consistent across T3, T4. Top-level concern names match between schema description (T9) and CLAUDE.md (T9).

**Risk:** If the lamad codegen.mjs has a non-standard JSON loader that bypasses the resolveRefs path, T3 will produce a diff in Step 4. The recipe says STOP in that case — the operator will need to find the divergent code path and route it through resolveRefs too. This is mentioned in T3 Step 4 and T6 Step 4.

**Execution handoff:** This plan is ready to execute via superpowers:subagent-driven-development.
