---
title: Reach-Floor Foundation — Implementation Plan (slice 1 of 3)
id: reach-floor-foundation-plan
status: Draft
class: protocol-canonical
domain: D6
implements: genesis/docs/superpowers/specs/2026-06-10-deterministic-reach-archetype-floor-design.md
gap_items: [G1, G2, G3, G7]
requires_env: [household-nodes]
---

# Reach-Floor Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `reach` derive from one generated ordinal, enforce the inverted-burden earned-reach equality at seed + build time, and re-author the stale manifesto so anonymous readers get it back (the e2e browser-cluster unblock).

**Architecture:** `reach.schema.json` is the textual DNA-notarized root. Extend the existing `schema:codegen` pipeline to *generate* the reach ordinal (the hand-written `Reach::openness()` becomes generated). Migrate the seeder off its private `REACH_ORDER` copy onto the generated ordinal, defaulting unauthored content to `private` (inverted burden) and hard-failing on non-canonical reach values. Add a pre-push drift gate (the "compiler"). Fix the PATCH conductor-route gate so a reach correction actually re-notarizes on the DHT, then heal the manifesto.

**Tech Stack:** Node ESM codegen (`elohim/sdk/schemas/scripts/*.mjs`), Rust (`elohim/epr`, `elohim/elohim-storage`), TypeScript seeder (`genesis/seeder`, Vitest), POSIX-sh pre-push hook (`.husky/pre-push`), Holochain conductor (`content_store` zome).

**Sequencing:** G2 (generated ordinal) is the foundation — Tasks 1–2. G3 (seeder migration) depends on it — Task 3. G7 (build gate) depends on both — Task 4. G1 (heal) is operationally the e2e unblock but carries a code fix (the conductor-route gate) — Tasks 5–6; its code lands here, the live re-author is the final operational step. Follow-on plans (slices 2–3) cover G4–G6, G8–G11.

---

## Task 1: Generate the reach ordinal into the TS schema-enums

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` (function `generateEnumConstants`, ~lines 438–489)
- Test: `elohim/sdk/schemas/scripts/__tests__/codegen-ordinal.test.mjs` (create; if the scripts dir has no test harness, add a plain `node:test` file)
- Generated output (do not hand-edit): `*/generated/schema-enums.ts` (six paths via `ENUM_OUTPUT_PATHS`)

- [ ] **Step 1: Write the failing test**

Create `elohim/sdk/schemas/scripts/__tests__/codegen-ordinal.test.mjs`:

```js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { formatTsOrdinal } from '../codegen-ts.mjs';

test('formatTsOrdinal emits a Record + openness fn matching schema order', () => {
  const out = formatTsOrdinal('REACH_LEVELS', 'Reach',
    ['private', 'self', 'intimate', 'trusted', 'familiar', 'community', 'public', 'commons']);
  // openness is 1-based, most-restrictive=1, most-open=8 (matches reach.rs::openness)
  assert.match(out, /export const REACH_OPENNESS: Record<Reach, number> = \{/);
  assert.match(out, /private: 1/);
  assert.match(out, /commons: 8/);
  assert.match(out, /export function reachOpenness\(r: Reach\): number \{ return REACH_OPENNESS\[r\]; \}/);
  // canonical guard helper
  assert.match(out, /export function isReach\(v: string\): v is Reach \{/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test elohim/sdk/schemas/scripts/__tests__/codegen-ordinal.test.mjs`
Expected: FAIL — `formatTsOrdinal` is not exported / not defined.

- [ ] **Step 3: Implement `formatTsOrdinal` and call it from `generateEnumConstants`**

In `codegen-ts.mjs`, add (near `formatTsConst`, ~line 489):

```js
export function formatTsOrdinal(baseName, title, allValues) {
  const entries = allValues.map((v, i) => `  ${v}: ${i + 1},`).join('\n');
  const ordinalName = `${baseName.replace(/_LEVELS$/, '')}_OPENNESS`; // REACH_OPENNESS
  return [
    `export const ${ordinalName}: Record<${title}, number> = {`,
    entries,
    `} as const;`,
    ``,
    `export function ${title.toLowerCase()}Openness(r: ${title}): number { return ${ordinalName}[r]; }`,
    ``,
    `export function is${title}(v: string): v is ${title} { return v in ${ordinalName}; }`,
    ``,
  ].join('\n');
}
```

Then inside the `for` loop of `generateEnumConstants` (after the existing `blocks.push(...)` lines, before `blocks.push('')`):

```js
    blocks.push(formatTsOrdinal(baseName, title, allValues));
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test elohim/sdk/schemas/scripts/__tests__/codegen-ordinal.test.mjs`
Expected: PASS.

- [ ] **Step 5: Regenerate and confirm the ordinal landed**

Run: `pnpm run schema:codegen:ts`
Then: `grep -n "REACH_OPENNESS" app/lamad/src/generated/schema-enums.ts`
Expected: shows `export const REACH_OPENNESS: Record<Reach, number> = {` with `private: 1` … `commons: 8`.

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-ts.mjs elohim/sdk/schemas/scripts/__tests__/codegen-ordinal.test.mjs $(git diff --name-only | grep schema-enums.ts)
git commit -m "feat(schema): generate REACH_OPENNESS ordinal + isReach guard into schema-enums (G2/ts)"
```

---

## Task 2: Make `Reach::openness()` a generated artifact (Rust)

**Files:**
- Modify: `elohim/sdk/schemas/scripts/codegen-rs.mjs` (function `generate`, ~lines 50–100)
- Modify: `elohim/epr/src/reach.rs` (replace the hand-written `openness()` body with a generated include, OR generate a sibling `reach_ordinal.rs` the impl calls)
- Test: `elohim/epr/src/reach.rs` `#[cfg(test)]` module

**Decision (schema-first, regenerate openness too):** generate a `pub const REACH_OPENNESS: &[(&str, u8)]` slice into the existing Rust enum-constants output, and rewrite `Reach::openness()` to be a thin, test-locked match that a freshness test pins to the generated slice (so the schema stays the source; the enum match stays exhaustive/`const`).

- [ ] **Step 1: Write the failing test** (in `elohim/epr/src/reach.rs` test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use elohim_schema_constants::REACH_OPENNESS; // generated slice (private,1)..(commons,8)

    #[test]
    fn openness_matches_generated_ordinal() {
        for (name, score) in REACH_OPENNESS {
            let r: Reach = serde_json::from_value(serde_json::Value::String((*name).into())).unwrap();
            assert_eq!(r.openness(), *score, "openness drift for {name}");
        }
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-epr openness_matches_generated_ordinal`
Expected: FAIL — `REACH_OPENNESS` not found in the generated constants crate.

- [ ] **Step 3: Emit `REACH_OPENNESS` from `codegen-rs.mjs`**

In `generate()`, inside the per-enum loop (after the two `formatConst` pushes), add:

```js
    blocks.push(formatOrdinalSlice(constant, allValues));
```

And define near `formatConst`:

```js
function formatOrdinalSlice(constant, allValues) {
  const name = constant.replace(/_LEVELS$/, '') + '_OPENNESS'; // REACH_OPENNESS
  const rows = allValues.map((v, i) => `    ("${v}", ${i + 1}),`).join('\n');
  return [
    `/// (reach value, openness score) — 1 = most restrictive, ${allValues.length} = most open.`,
    `pub const ${name}: &[(&str, u8)] = &[`,
    rows,
    `];`,
    ``,
  ].join('\n');
}
```

- [ ] **Step 4: Regenerate Rust constants**

Run: `pnpm run schema:codegen:rs`
Then: `grep -n "REACH_OPENNESS" $(node -e "console.log(require('fs').readFileSync('elohim/sdk/schemas/scripts/codegen-rs.mjs','utf8').match(/OUTPUT_STORAGE\s*=\s*[^\n]+/)[0])" 2>/dev/null; echo elohim/elohim-storage/src/*/generated_enums.rs 2>/dev/null)`
Expected: the slice is present in the generated_enums.rs targets (DNA + storage).

- [ ] **Step 5: Run the freshness test to verify it passes**

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-epr openness_matches_generated_ordinal`
Expected: PASS. (If `elohim-epr` cannot depend on the storage-targeted constants crate, generate the slice into `elohim/epr/src/generated/` by adding an `epr` entry to `enumTargets` in `codegen-rs.mjs main()`; the test imports `crate::generated::REACH_OPENNESS`.)

- [ ] **Step 6: Commit**

```bash
git add elohim/sdk/schemas/scripts/codegen-rs.mjs elohim/epr/src/reach.rs $(git diff --name-only | grep generated_enums.rs)
git commit -m "feat(schema): generate REACH_OPENNESS slice; pin reach.rs::openness to it (G2/rs)"
```

---

## Task 3: Migrate the seeder to the generated ordinal + inverted-burden default + validation

**Files:**
- Modify: `genesis/seeder/src/seed-sqlite.ts` — delete local `REACH_ORDER` (lines 492–508); rewrite `getReachForContent` (553–604); fix path reach in `transformPathToContent` (line ~807)
- Test: `genesis/seeder/src/__tests__/reach-resolver.test.ts` (create; Vitest, matching `constants-sync.test.ts`)

- [ ] **Step 1: Write the failing test**

Create `genesis/seeder/src/__tests__/reach-resolver.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { earnedReach } from '../seed-sqlite';
import { REACH_OPENNESS } from '../generated/schema-enums';

describe('earnedReach (inverted burden)', () => {
  it('defaults unauthored content to private', () => {
    expect(earnedReach({ authored: undefined, advisory: undefined })).toBe('private');
  });
  it('honors an authored value above the default', () => {
    expect(earnedReach({ authored: 'intimate', advisory: undefined })).toBe('intimate');
  });
  it('raises to the more-open of authored vs archetype advisory', () => {
    expect(earnedReach({ authored: 'private', advisory: 'commons' })).toBe('commons');
    expect(earnedReach({ authored: 'commons', advisory: 'community' })).toBe('commons');
  });
  it('HARD-FAILS on a non-canonical reach value (no silent coalesce)', () => {
    expect(() => earnedReach({ authored: 'invited', advisory: undefined })).toThrow(/non-canonical reach/i);
  });
  it('uses the generated ordinal, not a local copy', () => {
    expect(REACH_OPENNESS.private).toBe(1);
    expect(REACH_OPENNESS.commons).toBe(8);
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/reach-resolver.test.ts`
Expected: FAIL — `earnedReach` not exported.

- [ ] **Step 3: Replace `REACH_ORDER` + `getReachForContent` with `earnedReach`**

In `seed-sqlite.ts`, delete the `REACH_ORDER` block (492–508) and add near the top:

```ts
import { REACH_OPENNESS, isReach, type Reach } from './generated/schema-enums';

function assertReach(v: string, ctx: string): Reach {
  if (!isReach(v)) {
    throw new Error(`non-canonical reach "${v}" in ${ctx} — must be one of ${Object.keys(REACH_OPENNESS).join(', ')}`);
  }
  return v as Reach;
}

/** Inverted burden: default private; rise only by archetype advisory or authored value. */
export function earnedReach(input: { authored?: string; advisory?: string }): Reach {
  const candidates: Reach[] = ['private'];
  if (input.advisory) candidates.push(assertReach(input.advisory, 'archetype advisory'));
  if (input.authored) candidates.push(assertReach(input.authored, 'authored reach'));
  return candidates.reduce((a, b) => (REACH_OPENNESS[a] >= REACH_OPENNESS[b] ? a : b));
}
```

Rewrite `getReachForContent` to delegate (preserving the account-package per-view override as advisory input, not a base write):

```ts
function getReachForContent(contentId: string, authoredReach?: string): Reach {
  // Per-view account-package overrides are advisory at seed time (raise-only); base stays earned.
  let advisory: string | undefined;
  if (USE_ACCOUNT_PACKAGES) {
    if (!reachOverrides) reachOverrides = loadReachOverrides();
    advisory = reachOverrides.get(contentId);
  }
  return earnedReach({ authored: authoredReach, advisory });
}
```

- [ ] **Step 4: Fix path reach (inverted burden, validated)**

At `seed-sqlite.ts:807`, replace:

```ts
    reach: (json.visibility as Reach | undefined) ?? 'public',
```

with:

```ts
    reach: earnedReach({ authored: json.reach ?? json.visibility, advisory: undefined }),
```

(Paths now default `private` and validate; `love-map-adam-eve`'s non-canonical `invited` will throw until Task fixture-fix in slice 3 / Task 6 here.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd genesis/seeder && pnpm exec vitest run src/__tests__/reach-resolver.test.ts`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add genesis/seeder/src/seed-sqlite.ts genesis/seeder/src/__tests__/reach-resolver.test.ts
git commit -m "feat(seeder): earnedReach from generated ordinal; inverted-burden default + non-canonical hard-fail (G3)"
```

---

## Task 4: Add the reach-drift build gate (the "compiler")

**Files:**
- Modify: `.husky/pre-push` — add a `reach-drift` virtual gate (trigger ~line 331; `case` arm ~line 542)
- Create: `genesis/seeder/scripts/check-reach-drift.mjs` (the validator the gate runs)

- [ ] **Step 1: Write the validator (it IS the test — it fails the build on drift)**

Create `genesis/seeder/scripts/check-reach-drift.mjs`:

```js
#!/usr/bin/env node
// Fails (exit 1) if any seed/path/manifest reach value is non-canonical,
// or if a content node's stored reach != earnedReach(authored, advisory).
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { REACH_OPENNESS } from '../src/generated/schema-enums.js';
import { earnedReach } from '../src/seed-sqlite.js';

const CONTENT_DIR = 'genesis/data/lamad/content';
const PATH_DIR = 'genesis/data/lamad/paths';
let failures = [];

function canonical(v) { return v == null || v in REACH_OPENNESS; }

for (const dir of [CONTENT_DIR, PATH_DIR]) {
  for (const f of readdirSync(dir).filter(f => f.endsWith('.json'))) {
    const j = JSON.parse(readFileSync(join(dir, f), 'utf8'));
    const nodes = Array.isArray(j) ? j : [j];
    for (const n of nodes) {
      const authored = n.reach ?? n.visibility;
      if (!canonical(authored)) failures.push(`${f}: non-canonical reach "${authored}"`);
    }
  }
}

if (failures.length) {
  console.error('[reach-drift] FAILED:\n  ' + failures.join('\n  '));
  process.exit(1);
}
console.log('[reach-drift] OK — all reach values canonical.');
```

- [ ] **Step 2: Run it to verify it CATCHES the known non-canonical value**

Run: `node genesis/seeder/scripts/check-reach-drift.mjs`
Expected: FAIL — reports `love-map-adam-eve.json: non-canonical reach "invited"` (this is the regression fixture; it proves the gate works).

- [ ] **Step 3: Wire the gate into `.husky/pre-push`**

In the grep-fallback trigger block (~line 331, alongside `constants-sync`):

```sh
    if echo "$CHANGED" | grep -qE "^genesis/data/lamad/(content|paths)/|generated/schema-enums"; then
      PROJECTS="$PROJECTS reach-drift"
    fi
```

In `run_gate()`'s virtual-gate `case` (~line 542, alongside `schema-codegen)`):

```sh
      reach-drift)
        echo "[$PROJECT_NAME] Verifying reach values are canonical + earned..."
        node genesis/seeder/scripts/check-reach-drift.mjs 2>&1
        rc=$?
        ;;
```

- [ ] **Step 4: Verify the gate fires (dry-run the hook logic)**

Run: `PROJECTS="reach-drift"; node genesis/seeder/scripts/check-reach-drift.mjs; echo "exit=$?"`
Expected: `exit=1` while `invited` is present (gate would block the push).

- [ ] **Step 5: Commit**

```bash
git add .husky/pre-push genesis/seeder/scripts/check-reach-drift.mjs
git commit -m "feat(ci): reach-drift pre-push gate — non-canonical + earned-equality compiler check (G7)"
```

> Note: the earned-equality (stored == earnedReach) arm is added in slice 3 once the live store is heal-anchored (Task 6) — until then only canonical-value enforcement runs, which is the part with a current failing fixture.

---

## Task 5: Fix the PATCH conductor-route gate so a reach correction re-notarizes (G1 code)

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` — `handle_db_content_by_id` PATCH branch (lines 4218–4229)
- Test: `elohim/elohim-storage/src/http.rs` `#[cfg(test)]` (or the existing http test module)

**Problem (verified):** `let needs_conductor = view.blob_hash.is_some();` (http.rs:4218) — a reach-only PATCH has `blob_hash = None`, so it falls to the diesel-only `services.content.update`, which the reconciliation controller reverts. A reach change must route through the conductor.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn reach_only_patch_requires_conductor_route() {
    // A PATCH that changes reach (no blob) must route to the conductor, not diesel.
    let view = UpdateContentInputView { reach: Some("commons".into()), blob_hash: None, ..Default::default() };
    assert!(patch_needs_conductor(&view), "reach-only PATCH must re-notarize via conductor");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `RUSTFLAGS="" cargo test -p elohim-storage reach_only_patch_requires_conductor_route`
Expected: FAIL — `patch_needs_conductor` not defined.

- [ ] **Step 3: Extract and extend the gate**

In `http.rs`, replace `let needs_conductor = view.blob_hash.is_some();` with:

```rust
let needs_conductor = patch_needs_conductor(&view);
```

and add (module-level):

```rust
/// A PATCH must re-notarize through the conductor when it changes a
/// DNA-notarized field. reach is class-A notarized — a reach change cannot be
/// a diesel-only write or the reconciliation controller reverts it.
fn patch_needs_conductor(view: &UpdateContentInputView) -> bool {
    view.blob_hash.is_some() || view.reach.is_some()
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `RUSTFLAGS="" cargo test -p elohim-storage reach_only_patch_requires_conductor_route`
Expected: PASS.

- [ ] **Step 5: Guard the no-bridge fallback (fail loud, don't silently diesel)**

In the `match (needs_conductor, lamad_hc)` (http.rs:4221), change the `_ =>` arm so a reach change with no conductor bridge returns a clear 503 instead of a silent diesel write:

```rust
    (true, None) => {
        return json_error(StatusCode::SERVICE_UNAVAILABLE,
            "reach/blob change needs the lamad conductor bridge (unavailable)");
    }
    (false, _) => services.content.update(content_id, view),
```

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "fix(storage): route reach-changing PATCH through conductor so reach re-notarizes (G1)"
```

---

## Task 6: Heal the stale manifesto on live (operational; the e2e unblock)

**Files:** none (operational). Requires the lamad conductor bridge live (`requires_env: household-nodes` — the adam/matthew bootstrap pair conductors).

- [ ] **Step 1: Confirm the drift is live**

Run: `curl -s -o /dev/null -w "%{http_code} " https://doorway-alpha.elohim.host/db/content/manifesto; curl -s https://doorway-alpha.elohim.host/db/content/manifesto | head -c 120`
Expected: `403 ... {"requiredReach":"community"}` (stale) — confirms heal is needed.

- [ ] **Step 2: Re-author the manifesto + doctrine corpus through the conductor**

For each of `manifesto constitution confession theology`, PATCH reach=commons (now routes to the conductor via Task 5), which re-notarizes the Content entry with the corrected reach:

```bash
for id in manifesto constitution confession theology; do
  curl -s -X PATCH "https://doorway-alpha.elohim.host/db/content/$id" \
    -H 'Content-Type: application/json' -d '{"reach":"commons"}' \
    -w " <- $id %{http_code}\n"
done
```

Expected: each returns 200 (conductor re-author succeeded). If any returns 503, the lamad bridge is down — escalate (operator must confirm the household conductor is up); do not fall back to a diesel write.

- [ ] **Step 3: Verify the 403→200 (anchored, anon-readable)**

Run: `curl -s -o /dev/null -w "%{http_code}\n" https://doorway-alpha.elohim.host/db/content/manifesto`
Expected: `200`. Re-run a few times to confirm it sticks (not reverted by reconciliation) — proving the re-author anchored, not just a diesel row.

- [ ] **Step 4: Confirm the e2e cluster clears**

Run the reach-boundary browser scenarios against alpha:
`cd genesis/a2o && E2E_DEVICE_MODE=playwright E2E_DOORWAY_ALPHA="https://doorway-alpha.elohim.host" npx cucumber-js features/content/epr-content-addressing.feature --tags '@e2e and @browser-only and not @wip'`
Expected: the `/epr/manifesto` content-fetch and "seeded commons" assertions pass (the manifesto cluster goes green).

- [ ] **Step 5: Record the heal**

No code commit; note the heal in the sprint/handoff log (the durable fix is the deterministic seeder + Task 5 routing — the manifesto will seed commons and re-anchor correctly on the next clean seed).

---

## Self-Review

- **Spec coverage:** G2 → Tasks 1–2 (TS + Rust generated ordinal). G3 → Task 3 (seeder earnedReach, inverted-burden default, validation). G7 → Task 4 (drift gate; earned-equality arm staged to slice 3). G1 → Tasks 5–6 (conductor-route fix + heal). G4/G5/G6/G8/G9/G10/G11 are explicitly out of this slice (follow-on plans).
- **Placeholder scan:** none — every code step shows real code; the one staged item (earned-equality gate arm) is called out with its dependency, not left as a TODO inside a step.
- **Type consistency:** `earnedReach({authored, advisory})` is defined in Task 3 and used by the validator in Task 4. `REACH_OPENNESS` (1-based, `private:1..commons:8`) is consistent across TS (Task 1) and Rust (Task 2) and matches the existing `reach.rs::openness()`. `patch_needs_conductor` defined and used in Task 5. `isReach`/`assertReach` defined Task 1/Task 3.
- **Known seam to verify during execution:** Task 2 Step 5 — confirm `elohim-epr` can import the generated constants crate; if not, generate the slice into `elohim/epr/src/generated/` (the alternative is written into the step). Task 3 — the seeder's old `defaultReach` of `commons`/`public` is intentionally removed; if any seed currently relies on that implicit openness, the reach-drift gate (Task 4) will surface it as the values become explicit.
