---
title: Submodule pin attestation gate — implementation plan (rung 3)
id: submodule-pin-attestation-gate-plan
status: Draft
class: plan
serves: pin-attestation
date: 2026-09-23
steward: agent:orchestrator@claude-fable-5-1
cites:
  - "submodule-pin-attestation-gate-design | the design this plan implements task by task — four landing commits across rakia, brit, sophia and the monorepo | sha256:15ad7b065f2d1e68 | path: genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md"
---

# Submodule Pin Attestation Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A submodule pointer move (brit, rakia, sophia) is gated by the pinned commit's own upstream attestation and re-gates its direct consumers, instead of re-running the submodule's suite from the monorepo or running nothing.

**Architecture:** The rakia build-manifest schema gains an `attested` gate kind (and the queued `cargo.env`), each component declares itself in its own repo with the gitlink path as the step input, consumers declare `depends` edges on steps, and the orchestrator's gate runner asks `rakia affected` for the affected set (depth one, shadow mode first) and reads the pinned SHA's GitHub check through a new `gate-attest.mjs` module. Every attested read is an `epr flow note` observation on `pin-attestation@1`.

**Tech Stack:** JSON Schema 2020-12 + ajv (manifest validation), the rakia `schema-to-rust` codegen (Node ESM), rakia-core (Rust, `cargo test`), Node `node:test` for the orchestrator, `gh api` for check runs, cucumber-js + tsx for the a2o story, `epr flow note` for observations, `habits-project.py` for the register projection.

**Spec:** `genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md`

## Global Constraints

- The gate run kind name is exactly `attested`; the attestation block has exactly three required fields: `provider` (enum `github-checks`), `repo` (`owner/name`), `check` (check-run name). Nothing else (spec §4.1, §7).
- The pin is the input: every component step lists the gitlink path itself (`elohim/brit`, `elohim/rakia`, `sophia`) alongside its tree glob (spec §4.2).
- Consumer edges are declared on **steps** via `depends`; gate-only projects with `inputs` add the gitlink path to `inputs.sources` (spec §4.3).
- Local propagation is **depth one**: direct change, or one `depends` hop from a direct change (spec §4.4).
- The floor completes offline: an unreachable attestation read passes as `claimed`; an absent `rakia` binary falls back to path-only selection with one printed line (spec §4.4, §4.5; memoized-derivation Law III).
- A red, cancelled, timed-out, in-progress, or absent check **refuses** the gate (spec §4.5).
- Nothing runs a submodule's suite from the monorepo under any condition (spec §7).
- Every attested read is recorded as `epr flow note --kind observation --measure pin-attestation@1` (spec §4.6).
- Native cargo commands need `RUSTFLAGS=""` and an explicit `CARGO_TARGET_DIR` (a PreToolUse hook denies native cargo without one). rakia's pool slot on this host: `/projects/.cargo-target-pool/family/dev/elohim__rakia/dev` if `/projects` is writable; otherwise `/tmp/rakia-gate-target`. Judge cargo by `EXIT=$?` on its own line, never by tailed output.
- Pushes from this host are SSH-only and a long pre-push gate drops the session: run the named gate to ALL CLEAR first, then `HUSKY=0 git push`.
- Never commit `rakia-executor/` into the rakia repo from a machine where it is untracked (backlog `rakia-executor-untracked-in-submodule-pin`). On this host it is absent; check `git -C elohim/rakia status --short` before every rakia commit.
- Commit messages end with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`.

## Review Focus

1. **A gitlink path whose submodule dir is missing or not initialised** (`elohim/rakia` on a fresh clone without `submodule update`): `git rev-parse HEAD:elohim/rakia` still resolves the pin, so the attested gate must read the pin, not the worktree. Pinned by Task 10's "reads the pin, not the checkout" test.
2. **A check that ran more than once at the same SHA** (re-run after a flake): the newest run's conclusion wins. Pinned by Task 10's "latest run wins" test.
3. **A changed-file list that contains both the gitlink path and files under it** (a dirty submodule plus a pin bump): the component project is selected once, with both reasons. Pinned by Task 9's dedupe test.
4. **A consumer that is itself attested** (rakia depends on nothing today, but a future brit→rakia edge would make an attested project a depth-one dependent): gate-runner must dispatch it to `gate-attest.mjs`, never to `run-local-gate.sh`. Pinned by Task 10's dispatch test.
5. **`rakia affected` present but failing** (bad manifest, non-zero exit): selection must fall back and print the line, not crash the push. Pinned by Task 8's "non-zero exit falls back" test.

---

## Landing map

| commit | tasks | repos | gate to ALL CLEAR before push |
|---|---|---|---|
| 1 | 1 – 3 | rakia, monorepo | `just gate rakia-codegen && just gate rakia-validate` |
| 2 | 4 – 7 | brit, sophia, rakia, monorepo | `just gate rakia-validate && just gate orchestrator && just gate pipeline-list-fresh` |
| 3 | 8 – 13 | monorepo | `just gate orchestrator && just gate genesis-a2o` (a2o lint) |
| 4 | 14 | monorepo | `just gate orchestrator` + one real pin-moving push |

---

### Task 1: codegen names a string enum by its schema `title`

Without this, lifting `gateRun.kind` allocates the Rust enum name `Kind` first and silently renames the existing `AffectedReason.kind` enum to `AffectedReasonKind`, breaking `constellation.rs`.

**Files:**
- Modify: `elohim/rakia/schemas/scripts/lib/schema-to-rust.mjs:160-175` (`allocateEnum`)
- Test: `elohim/rakia/schemas/tests/codegen-rs.test.mjs`

**Interfaces:**
- Produces: a string-enum property carrying `"title": "<PascalName>"` is emitted as `pub enum <PascalName>`; untitled enums keep today's field-name allocation.

- [ ] **Step 1: Write the failing test**

Append to `elohim/rakia/schemas/tests/codegen-rs.test.mjs`:

```js
describe('schemaToRust — enum naming', () => {
  it('names a string enum by its title when one is given, leaving the field-name slot free', () => {
    const schema = {
      title: 'Root',
      type: 'object',
      properties: {},
      $defs: {
        gateRun: {
          type: 'object',
          required: ['kind'],
          properties: {
            kind: { type: 'string', title: 'GateRunKind', enum: ['just', 'root-just', 'attested'] },
          },
        },
        affectedReason: {
          type: 'object',
          required: ['kind'],
          properties: {
            kind: { type: 'string', enum: ['changedFile', 'upstreamNode'] },
          },
        },
      },
    };
    const out = schemaToRust(schema);
    assert.match(out, /pub kind: GateRunKind,/);
    assert.match(out, /pub enum GateRunKind \{/);
    assert.match(out, /pub enum Kind \{/, 'the untitled enum still owns the bare field name');
    assert.doesNotMatch(out, /AffectedReasonKind/);
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /home/matthew/git/elohim && node --test elohim/rakia/schemas/tests/codegen-rs.test.mjs`
Expected: FAIL — `pub kind: GateRunKind,` not found (the enum is named `Kind` and the second becomes `AffectedReasonKind`).

- [ ] **Step 3: Implement**

In `schema-to-rust.mjs`, replace the `allocateEnum` function inside `schemaToRust` with:

```js
    allocateEnum(prop, fieldPath) {
      // A declared `title` names the enum outright — the schema author owns the Rust name.
      if (typeof prop.title === 'string' && prop.title.length > 0) {
        const titled = pascalCase(prop.title);
        const existingTitled = enumRegistry.get(titled);
        if (existingTitled && JSON.stringify([...existingTitled].sort()) !== JSON.stringify([...prop.enum].sort())) {
          throw new Error(`enum title '${titled}' is already used with different values (path: ${fieldPath})`);
        }
        enumRegistry.set(titled, prop.enum);
        return titled;
      }
      const name = pascalCase(fieldPath.split('.').pop()) || 'Unnamed';
      const existing = enumRegistry.get(name);
      const sortedValues = [...prop.enum].sort();
      if (existing) {
        const existingSorted = [...existing].sort();
        if (JSON.stringify(existingSorted) === JSON.stringify(sortedValues)) {
          return name;
        }
        const altName = pascalCase(fieldPath.replace(/\./g, '-'));
        enumRegistry.set(altName, prop.enum);
        return altName;
      }
      enumRegistry.set(name, prop.enum);
      return name;
    },
```

- [ ] **Step 4: Run the codegen tests**

Run: `cd /home/matthew/git/elohim && node --test elohim/rakia/schemas/tests/codegen-rs.test.mjs`
Expected: PASS (all existing cases plus the new one).

- [ ] **Step 5: Commit (rakia repo, on a branch)**

```bash
cd /home/matthew/git/elohim/elohim/rakia
git status --short            # must be empty apart from the two files below
git checkout -b feat/gate-run-attested
git add schemas/scripts/lib/schema-to-rust.mjs schemas/tests/codegen-rs.test.mjs
git commit -m "codegen: a string enum carrying a schema title is emitted under that name

Lifting GateProject.run into \$defs adds a second 'kind' enum; without a
declared name it would steal 'Kind' from AffectedReason and rename the
existing generated enum.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: lift `GateProject.run` into `$defs`, add `attested`, `attestation`, and `cargo.env`

**Files:**
- Modify: `elohim/rakia/schemas/v1/build-manifest.schema.json` (`$defs.gateProject`, lines 253–330; add `gateRun`, `gateCargo`, `gateAttestation`)
- Regenerate: `elohim/rakia/rakia-core/src/generated_types.rs` (via `codegen-rs.mjs`)
- Create: `elohim/rakia/schemas/tests/gate-run.test.mjs`

**Interfaces:**
- Produces (schema): `$defs.gateRun { kind: "just"|"root-just"|"attested", recipe?, cargo?: gateCargo, attestation?: gateAttestation }`, `$defs.gateCargo { workspace?, targetDir?, profile, rustflags, env?: {[A-Z_0-9]+: string} }`, `$defs.gateAttestation { provider: "github-checks", repo, check }`.
- Produces (Rust): `GateProject { dir, steps, run: Option<GateRun>, inputs: Option<StepInputs> }`, `GateRun { kind: GateRunKind, recipe: Option<String>, cargo: Option<GateCargo>, attestation: Option<GateAttestation> }`, `GateCargo { …, env: Option<BTreeMap<String, String>> }`, `GateAttestation { provider: AttestationProvider, repo, check }`, enums `GateRunKind`, `AttestationProvider`.

- [ ] **Step 1: Write the failing schema-gate test**

Create `elohim/rakia/schemas/tests/gate-run.test.mjs`:

```js
#!/usr/bin/env node
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const here = dirname(fileURLToPath(import.meta.url));
const schema = JSON.parse(readFileSync(resolve(here, '../v1/build-manifest.schema.json'), 'utf8'));
const ajv = new Ajv2020({ allErrors: true, strict: false });
addFormats(ajv);
const validate = ajv.compile(schema);

function manifest(run) {
  return {
    manifestVersion: '1.0',
    pipeline: 'p',
    description: 'd',
    steps: {
      s: {
        description: 's',
        inputs: { sources: ['x'], buildProcess: [] },
        outputs: { artifacts: [], verify: null },
        depends: [],
        executor: { stage: 'S', function: null },
      },
    },
    gate: { projects: { g: { dir: 'x', steps: ['s'], run } } },
  };
}

const attestation = { provider: 'github-checks', repo: 'ethosengine/brit', check: 'Tests pass' };
const cargo = { profile: 'dev', rustflags: '' };
const why = () => JSON.stringify(validate.errors);

describe('gateRun — the three kinds', () => {
  it('accepts just with a recipe', () => {
    assert.equal(validate(manifest({ kind: 'just', recipe: 'gate' })), true, why());
  });
  it('accepts root-just with a cargo contract', () => {
    assert.equal(validate(manifest({ kind: 'root-just', recipe: '_gate-x', cargo: { ...cargo, workspace: 'elohim' } })), true, why());
  });
  it('accepts attested with an attestation and no recipe', () => {
    assert.equal(validate(manifest({ kind: 'attested', attestation })), true, why());
  });
  it('refuses attested without an attestation', () => {
    assert.equal(validate(manifest({ kind: 'attested' })), false);
  });
  it('refuses attested with a recipe or a cargo block', () => {
    assert.equal(validate(manifest({ kind: 'attested', attestation, recipe: 'gate' })), false);
    assert.equal(validate(manifest({ kind: 'attested', attestation, cargo })), false);
  });
  it('refuses just with an attestation', () => {
    assert.equal(validate(manifest({ kind: 'just', recipe: 'gate', attestation })), false);
  });
  it('refuses just without a recipe', () => {
    assert.equal(validate(manifest({ kind: 'just' })), false);
  });
  it('refuses an unknown provider, a malformed repo, and an empty check', () => {
    assert.equal(validate(manifest({ kind: 'attested', attestation: { ...attestation, provider: 'jenkins' } })), false);
    assert.equal(validate(manifest({ kind: 'attested', attestation: { ...attestation, repo: 'brit' } })), false);
    assert.equal(validate(manifest({ kind: 'attested', attestation: { ...attestation, check: '' } })), false);
  });
  it('refuses a fourth attestation field', () => {
    assert.equal(validate(manifest({ kind: 'attested', attestation: { ...attestation, threshold: 2 } })), false);
  });
});

describe('gateCargo.env — the per-project resource cap', () => {
  it('accepts upper-case keys with string values', () => {
    assert.equal(validate(manifest({ kind: 'just', recipe: 'gate', cargo: { ...cargo, env: { CARGO_BUILD_JOBS: '1' } } })), true, why());
  });
  it('refuses lower-case keys and non-string values', () => {
    assert.equal(validate(manifest({ kind: 'just', recipe: 'gate', cargo: { ...cargo, env: { jobs: '1' } } })), false);
    assert.equal(validate(manifest({ kind: 'just', recipe: 'gate', cargo: { ...cargo, env: { CARGO_BUILD_JOBS: 1 } } })), false);
  });
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /home/matthew/git/elohim && node --test elohim/rakia/schemas/tests/gate-run.test.mjs`
Expected: FAIL — `attested` is not in the `kind` enum; `env` and `attestation` are refused as additional properties.

- [ ] **Step 3: Edit the schema**

In `elohim/rakia/schemas/v1/build-manifest.schema.json`, replace the entire `"gateProject": { … }` definition (the last entry in `$defs`) with these four definitions:

```json
    "gateProject": {
      "type": "object",
      "required": ["dir"],
      "additionalProperties": false,
      "properties": {
        "dir": {
          "type": "string",
          "description": "Repo-root-relative working directory scanned for changes"
        },
        "steps": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Step IDs in this manifest that this gate project is responsible for. Omit to associate with all steps."
        },
        "run": {
          "$ref": "#/$defs/gateRun",
          "description": "Typed local-gate execution (gate-runner.mjs is the consumer)"
        },
        "inputs": {
          "$ref": "#/$defs/stepInputs",
          "description": "Optional content-hash inputs for gate-level change detection (same shape as step inputs)"
        }
      }
    },
    "gateRun": {
      "type": "object",
      "required": ["kind"],
      "additionalProperties": false,
      "description": "How a gate project is executed locally. just / root-just run a recipe; attested runs nothing and reads the pinned commit's upstream attestation instead.",
      "properties": {
        "kind": {
          "type": "string",
          "title": "GateRunKind",
          "enum": ["just", "root-just", "attested"],
          "description": "just = recipe in the project dir's justfile; root-just = recipe in the repo-root justfile; attested = no local recipe — the gate reads the attestation the component's own CI recorded at the pinned commit"
        },
        "recipe": {
          "type": "string",
          "description": "Recipe name to invoke (just and root-just only)"
        },
        "cargo": { "$ref": "#/$defs/gateCargo" },
        "attestation": { "$ref": "#/$defs/gateAttestation" }
      },
      "oneOf": [
        {
          "properties": { "kind": { "enum": ["just", "root-just"] } },
          "required": ["recipe"],
          "not": { "required": ["attestation"] }
        },
        {
          "properties": { "kind": { "const": "attested" } },
          "required": ["attestation"],
          "not": { "anyOf": [ { "required": ["recipe"] }, { "required": ["cargo"] } ] }
        }
      ]
    },
    "gateCargo": {
      "type": "object",
      "required": ["profile", "rustflags"],
      "additionalProperties": false,
      "description": "Native-cargo contract: resolves the cargo-pool slot and RUSTFLAGS so the gate cannot drift from the pool discipline",
      "properties": {
        "workspace": {
          "type": "string",
          "description": "Cargo-pool family workspace key (slot derived)"
        },
        "targetDir": {
          "type": "string",
          "description": "Explicit CARGO_TARGET_DIR (used where the pool slot is not applicable)"
        },
        "profile": {
          "type": "string",
          "description": "Cargo profile (dev/release)"
        },
        "rustflags": {
          "type": "string",
          "description": "RUSTFLAGS for this gate; empty string means CLEAR, not inherit"
        },
        "env": {
          "type": "object",
          "description": "Extra environment exported around this project's cargo run — never passed through argv. The declaration home for per-project resource caps (CARGO_BUILD_JOBS, RUST_TEST_THREADS) so a heavy crate's gate cannot cross the workspace RAM guard's shed line.",
          "propertyNames": { "pattern": "^[A-Z][A-Z0-9_]*$" },
          "additionalProperties": { "type": "string" }
        }
      }
    },
    "gateAttestation": {
      "type": "object",
      "required": ["provider", "repo", "check"],
      "additionalProperties": false,
      "description": "Where the pinned commit's attestation is read from. Exactly three fields by design: no thresholds, no signer lists, no reputation (spec 2026-09-23 §7).",
      "properties": {
        "provider": {
          "type": "string",
          "title": "AttestationProvider",
          "enum": ["github-checks"],
          "description": "github-checks = GET /repos/{repo}/commits/{sha}/check-runs, the named check's newest run"
        },
        "repo": {
          "type": "string",
          "pattern": "^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$",
          "description": "owner/name on the provider"
        },
        "check": {
          "type": "string",
          "minLength": 1,
          "description": "The check-run name that stands for 'this commit is attested' (brit: 'Tests pass'; rakia: 'test'; sophia: 'build')"
        }
      }
    }
```

- [ ] **Step 4: Run the schema-gate test and the manifest validator**

Run: `cd /home/matthew/git/elohim && node --test elohim/rakia/schemas/tests/gate-run.test.mjs && pnpm -s run rakia:schema:validate; echo EXIT=$?`
Expected: all gate-run cases PASS; every existing manifest still validates; `EXIT=0`.

- [ ] **Step 5: Regenerate the Rust types and verify freshness**

Run: `cd /home/matthew/git/elohim && pnpm -s run rakia:codegen:rs && pnpm -s run rakia:codegen:rs:verify; echo EXIT=$?`
Expected: `EXIT=0`. `git -C elohim/rakia diff --stat` shows only `rakia-core/src/generated_types.rs` changed. Inspect the diff: `GateProject` gains `run: Option<GateRun>` and `inputs: Option<StepInputs>`; new structs `GateRun`, `GateCargo`, `GateAttestation`; new enums `GateRunKind { Just, RootJust, Attested }` and `AttestationProvider { GithubChecks }`; the existing `Kind` enum is unchanged.

- [ ] **Step 6: Run rakia-core's tests, fmt and clippy the way its CI does**

```bash
cd /home/matthew/git/elohim/elohim/rakia
export RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/rakia-gate-target
cargo fmt --all --check; echo EXIT=$?
cargo clippy --workspace --all-targets -- -D warnings; echo EXIT=$?
cargo test --workspace; echo EXIT=$?
```
Expected: three `EXIT=0` lines. If `cargo fmt --check` fails on `generated_types.rs`, run `cargo fmt --all` and re-run the verify step (the codegen must produce formatted output; if it does not, fix the codegen emitter's spacing rather than hand-editing the generated file).

- [ ] **Step 7: Commit (rakia repo)**

```bash
cd /home/matthew/git/elohim/elohim/rakia
git status --short          # exactly the three paths below, nothing else
git add schemas/v1/build-manifest.schema.json schemas/tests/gate-run.test.mjs rakia-core/src/generated_types.rs
git commit -m "schema(build-manifest): GateProject.run lifted to \$defs — attested kind, attestation, cargo.env

The typed-gate commit added run inline; codegen refuses inline nested
objects, so the generated GateProject never carried it. Lifting it gives
the three widenings a home: kind gains 'attested' (no recipe — the gate
reads the pinned commit's upstream attestation), a three-field
gateAttestation (provider/repo/check, github-checks only), and
gateCargo.env for per-project resource caps. Design:
elohim monorepo genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: land rakia commit 1 and bump the monorepo pin

**Files:**
- Modify: monorepo gitlink `elohim/rakia`

- [ ] **Step 1: Push the rakia branch and let its CI run**

```bash
cd /home/matthew/git/elohim/elohim/rakia
git push -u origin feat/gate-run-attested
gh run watch --repo ethosengine/rakia --exit-status $(gh run list --repo ethosengine/rakia --branch feat/gate-run-attested --limit 1 --json databaseId --jq '.[0].databaseId')
```
Expected: the `test` job concludes `success`.

- [ ] **Step 2: Fast-forward rakia main and push**

```bash
cd /home/matthew/git/elohim/elohim/rakia
git fetch origin
git checkout main && git merge --ff-only feat/gate-run-attested && git push origin main
git log -1 --format=%H    # record this SHA as RAKIA_SHA_1
```

- [ ] **Step 3: Bump the monorepo pin and run the two gates**

```bash
cd /home/matthew/git/elohim
git add elohim/rakia
just gate rakia-codegen; echo EXIT=$?
just gate rakia-validate; echo EXIT=$?
```
Expected: both `EXIT=0`. (The codegen gate was red at the old pin; this is the first green.)

- [ ] **Step 4: Commit the pin**

```bash
cd /home/matthew/git/elohim
git commit -m "chore(rakia): bump to <RAKIA_SHA_1 short> — GateProject.run lifted to \$defs; attested kind; cargo.env

rakia-codegen is green again at this pin (it was red: the typed-gate
schema commit had never been regenerated).

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: brit declares itself (brit repo)

**Files:**
- Create: `elohim/brit/build-manifest.json`

**Interfaces:**
- Produces: pipeline `elohim-brit`, step `elohim-brit:brit-ci`, gate project `brit` (attested).

- [ ] **Step 1: Write the manifest**

Create `elohim/brit/build-manifest.json`:

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-brit",
  "manualOnly": true,
  "triggersGenesis": false,
  "cascades": false,
  "dependsOn": [],
  "description": "brit — covenantal git (gitoxide fork). Built, tested and attested by its own GitHub CI; the elohim monorepo consumes it as a submodule pin. The pin is the build input; the upstream attestation is the local gate.",
  "steps": {
    "brit-ci": {
      "description": "brit's own CI at the pinned commit (ci.yml → 'Tests pass'); on main it also publishes the brit crate set to the elohim registry",
      "inputs": {
        "sources": ["elohim/brit", "elohim/brit/**"],
        "buildProcess": ["elohim/brit/.github/workflows/ci.yml"]
      },
      "outputs": {
        "artifacts": ["brit-crates"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "Tests pass",
        "function": null
      }
    }
  },
  "gate": {
    "projects": {
      "brit": {
        "dir": "elohim/brit",
        "steps": ["brit-ci"],
        "run": {
          "kind": "attested",
          "attestation": {
            "provider": "github-checks",
            "repo": "ethosengine/brit",
            "check": "Tests pass"
          }
        }
      }
    }
  },
  "deployment": {}
}
```

- [ ] **Step 2: Validate it from the monorepo root**

Run: `cd /home/matthew/git/elohim && pnpm -s run rakia:schema:validate | grep -E "brit|FAIL|valid"; echo EXIT=${PIPESTATUS[0]}`
Expected: `PASS  elohim/brit/build-manifest.json`, `EXIT=0`.

- [ ] **Step 3: Commit and push (brit main; a JSON file does not touch brit's build)**

```bash
cd /home/matthew/git/elohim/elohim/brit
git fetch origin && git status -sb | head -1     # expect main up to date with origin/main
git add build-manifest.json
git commit -m "build-manifest: declare brit as an attested component of the elohim monorepo

The monorepo's local gate reads this manifest: the submodule pin is the
step input and the 'Tests pass' check at the pinned commit is the gate.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
git push origin main
git log -1 --format=%H     # record as BRIT_SHA
```

---

### Task 5: rakia declares itself (rakia repo)

**Files:**
- Create: `elohim/rakia/build-manifest.json`

- [ ] **Step 1: Write the manifest**

Create `elohim/rakia/build-manifest.json`:

```json
{
  "manifestVersion": "1.0",
  "pipeline": "elohim-rakia",
  "manualOnly": true,
  "triggersGenesis": false,
  "cascades": false,
  "dependsOn": [],
  "description": "rakia — the build-manifest schema, codegen and constellation planner. Built, tested and attested by its own GitHub CI; the elohim monorepo consumes it as a submodule pin (schema mirror in elohim-storage, compute-executor in the edge image, rakia-validate/rakia-codegen gates).",
  "steps": {
    "rakia-ci": {
      "description": "rakia's own CI at the pinned commit (ci.yml → 'test': fmt, clippy -D warnings, cargo test --workspace); on main it publishes rakia-core/rakia-brit to the elohim registry",
      "inputs": {
        "sources": ["elohim/rakia", "elohim/rakia/**"],
        "buildProcess": ["elohim/rakia/.github/workflows/ci.yml"]
      },
      "outputs": {
        "artifacts": ["rakia-crates"],
        "verify": null
      },
      "depends": [],
      "executor": {
        "stage": "test",
        "function": null
      }
    }
  },
  "gate": {
    "projects": {
      "rakia": {
        "dir": "elohim/rakia",
        "steps": ["rakia-ci"],
        "run": {
          "kind": "attested",
          "attestation": {
            "provider": "github-checks",
            "repo": "ethosengine/rakia",
            "check": "test"
          }
        }
      }
    }
  },
  "deployment": {}
}
```

- [ ] **Step 2: Validate, then run rakia's own discovery test against it**

```bash
cd /home/matthew/git/elohim && pnpm -s run rakia:schema:validate | grep -E "rakia/build|FAIL"; echo EXIT=${PIPESTATUS[0]}
cd elohim/rakia && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/rakia-gate-target cargo test -p rakia-core --test discover_test; echo EXIT=$?
```
Expected: `PASS  elohim/rakia/build-manifest.json`; both `EXIT=0`.

- [ ] **Step 3: Commit and push (rakia main)**

```bash
cd /home/matthew/git/elohim/elohim/rakia
git status --short          # only build-manifest.json
git add build-manifest.json
git commit -m "build-manifest: declare rakia as an attested component of the elohim monorepo

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
git push origin main
git log -1 --format=%H     # record as RAKIA_SHA_2
```

---

### Task 6: sophia's gate becomes attested (sophia repo)

**Files:**
- Modify: `sophia/build-manifest.json` (step `build-sophia-umd` sources; gate project `sophia`)

- [ ] **Step 1: Edit the manifest**

In `sophia/build-manifest.json` change the step's sources and the gate project's run:

```json
      "inputs": {
        "sources": ["sophia", "sophia/**"],
        "buildProcess": ["sophia.Jenkinsfile"]
      },
```

```json
  "gate": {
    "projects": {
      "sophia": {
        "dir": "sophia",
        "steps": ["build-sophia-umd"],
        "run": {
          "kind": "attested",
          "attestation": {
            "provider": "github-checks",
            "repo": "ethosengine/sophia",
            "check": "build"
          }
        }
      }
    }
  },
```

- [ ] **Step 2: Validate from the monorepo root**

Run: `cd /home/matthew/git/elohim && pnpm -s run rakia:schema:validate | grep -E "sophia|FAIL"; echo EXIT=${PIPESTATUS[0]}`
Expected: `PASS  sophia/build-manifest.json`, `EXIT=0`.

- [ ] **Step 3: Commit and push (sophia main)**

```bash
cd /home/matthew/git/elohim/sophia
git fetch origin && git status -sb | head -1
git add build-manifest.json
git commit -m "chore(build-manifest): the pin is the input; the local gate reads sophia's own CI attestation

The monorepo no longer runs sophia's suite on a pointer move — the
'build' check at the pinned commit is what it reads (design:
elohim genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md).

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
git push origin main
git log -1 --format=%H     # record as SOPHIA_SHA
```

---

### Task 7: monorepo consumer edges, orchestrator inputs, storage cap, three pin bumps

**Files:**
- Modify: `elohim/holochain/build-manifest.json` (steps `cargo-build-storage.depends`, `build-edge-image.depends`; gate project `elohim-storage.run.cargo`)
- Modify: `genesis/orchestrator/build-manifest.json` (gate projects `rakia-codegen.inputs.sources`, `rakia-validate.inputs.sources`)
- Modify: `genesis/agentic/pool-policy.json` (`cargo_env_overrides.elohim-storage` removed)
- Modify: `genesis/orchestrator/pipeline-registry.mjs:132-140` (registry carries `run` as-is; no change needed unless it validates `recipe`)
- Modify: `genesis/orchestrator/gate-runner.test.mjs:16-21` (kind enum assertion)
- Modify: gitlinks `elohim/brit`, `elohim/rakia`, `sophia`

**Interfaces:**
- Produces: `elohim-edge:cargo-build-storage.depends` includes `elohim-rakia:rakia-ci`; `elohim-edge:build-edge-image.depends` includes `elohim-brit:brit-ci` and `elohim-rakia:rakia-ci`; the registry accepts `run.kind === 'attested'` with `run.recipe === undefined`.

- [ ] **Step 1: Write the failing registry test**

In `genesis/orchestrator/gate-runner.test.mjs`, replace the `every project has typed execution metadata` test body with:

```js
  test('every project has typed execution metadata', () => {
    assert.ok(registry.size >= 35);
    for (const project of registry.values()) {
      assert.ok(['just', 'root-just', 'attested'].includes(project.run.kind), project.name);
      if (project.run.kind === 'attested') {
        assert.equal(project.run.recipe, undefined, `${project.name}: attested projects run no recipe`);
        assert.equal(project.run.attestation.provider, 'github-checks', project.name);
        assert.match(project.run.attestation.repo, /^[\w.-]+\/[\w.-]+$/, project.name);
        assert.ok(project.run.attestation.check.length > 0, project.name);
      } else {
        assert.match(project.run.recipe, /^_?[a-z][a-z0-9-]*$/);
      }
    }
    for (const name of ['brit', 'rakia', 'sophia']) {
      assert.equal(registry.get(name).run.kind, 'attested', `${name} is an attested component`);
    }
  });

  test('a pin move is a direct input of its component step, in both matchers', () => {
    for (const [component, gitlink] of [['brit', 'elohim/brit'], ['rakia', 'elohim/rakia'], ['sophia', 'sophia']]) {
      const project = registry.get(component);
      const manifest = loadManifests(ROOT).find(m => m.content.pipeline === project.pipeline);
      const step = manifest.content.steps[project.steps[0]];
      assert.ok(step.inputs.sources.includes(gitlink), `${component}: ${gitlink} listed verbatim, not only as a glob`);
    }
  });

  test('a rakia pin move fires the two schema gates directly', () => {
    const selected = projectsForChanges(ROOT, ['elohim/rakia']).map(p => p.name);
    assert.ok(selected.includes('rakia-validate'));
    assert.ok(selected.includes('rakia-codegen'));
    assert.ok(selected.includes('rakia'));
  });
```

Add `import { loadManifests } from './manifest-utils.mjs';` at the top of the file.

- [ ] **Step 2: Run it to verify it fails**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test gate-runner.test.mjs`
Expected: FAIL — `registry.get('brit')` is undefined (pins not yet bumped).

- [ ] **Step 3: Bump the three pins**

```bash
cd /home/matthew/git/elohim
git -C elohim/brit checkout main && git -C elohim/brit pull --ff-only
git -C elohim/rakia checkout main && git -C elohim/rakia pull --ff-only
git -C sophia checkout main && git -C sophia pull --ff-only
git submodule status elohim/brit elohim/rakia sophia     # must show BRIT_SHA, RAKIA_SHA_2, SOPHIA_SHA
git add elohim/brit elohim/rakia sophia
```

- [ ] **Step 4: Declare the consumer edges**

In `elohim/holochain/build-manifest.json`:

`cargo-build-storage` — change
```json
      "depends": [
        "elohim-conductor:build-conductor-image"
      ],
```
to
```json
      "depends": [
        "elohim-conductor:build-conductor-image",
        "elohim-rakia:rakia-ci"
      ],
```

`build-edge-image` — append `"elohim-brit:brit-ci"` and `"elohim-rakia:rakia-ci"` to its `depends` array (keep every existing entry).

`elohim-storage` gate project — change its `run.cargo` to
```json
        "run": {
          "kind": "just",
          "recipe": "gate",
          "cargo": {
            "workspace": "elohim/elohim-storage",
            "profile": "dev",
            "rustflags": "--cfg getrandom_backend=\"custom\"",
            "env": { "CARGO_BUILD_JOBS": "1" }
          }
        }
```

In `genesis/agentic/pool-policy.json`, delete the `"elohim-storage": { "CARGO_BUILD_JOBS": "1" }` entry from `cargo_env_overrides` (leave `eprfs` and `memory-ceremony`; the manifest now declares storage's cap and the runner's merge rule lets the manifest win).

In `genesis/orchestrator/build-manifest.json`, add `"elohim/rakia"` as the first entry of `rakia-codegen.inputs.sources` and of `rakia-validate.inputs.sources`.

- [ ] **Step 5: Prove the cap still reaches the storage gate**

Run: `cd /home/matthew/git/elohim && node genesis/orchestrator/gate-runner.mjs --target elohim-storage --print | node -e "const r=JSON.parse(require('fs').readFileSync(0,'utf8'));if(r.resolvedCargoEnv.CARGO_BUILD_JOBS!=='1')process.exit(1);console.log('cap ok', r.resolvedCargoEnv)"`
Expected: `cap ok { CARGO_BUILD_JOBS: '1' }`.

- [ ] **Step 6: Run the gates and the orchestrator suite**

```bash
cd /home/matthew/git/elohim
just gate rakia-validate; echo EXIT=$?
just gate pipeline-list-fresh; echo EXIT=$?
just gate cargo-coverage; echo EXIT=$?
just gate orchestrator; echo EXIT=$?
```
Expected: four `EXIT=0`. If `cargo-coverage` now sees brit's or rakia's `[[bin]]` targets and reports one uncovered, the component manifest's `<dir>/**` glob is the coverage — add the reported path to that manifest's step sources in its own repo and re-pin; do not add it to a monorepo manifest.

- [ ] **Step 7: Commit**

```bash
cd /home/matthew/git/elohim
git add elohim/brit elohim/rakia sophia elohim/holochain/build-manifest.json genesis/orchestrator/build-manifest.json genesis/agentic/pool-policy.json genesis/orchestrator/gate-runner.test.mjs genesis/orchestrator/pipeline-list.json
git commit -m "gate(manifests): brit, rakia and sophia are attested components; consumers declare the edge

Three pins: brit <BRIT short>, rakia <RAKIA_SHA_2 short>, sophia <SOPHIA short>
— each repo now carries a manifest whose step input is the gitlink path
and whose gate is 'attested' on its own CI check. Consumer edges on
steps: cargo-build-storage and build-edge-image depend on rakia-ci,
build-edge-image on brit-ci; the two schema gates list the rakia pin as
a direct input. The storage CARGO_BUILD_JOBS cap moves from pool-policy
into its manifest now that the schema accepts run.cargo.env.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 8: `gate-oracle.mjs` — ask rakia, filter to depth one, fall back honestly

**Files:**
- Create: `genesis/orchestrator/gate-oracle.mjs`
- Test: `genesis/orchestrator/gate-oracle.test.mjs`
- Modify: `genesis/orchestrator/package.json:6` (add the test file to the `test` script list)

**Interfaces:**
- Produces:
  - `resolveRakiaBin(env) → string | null` — `env.RAKIA_BIN` if set and executable, else `rakia` on PATH, else `null`.
  - `rakiaAffected(root, changedFiles, { rakiaBin, spawn }) → Map<string, string[]> | null` — qualified step → reasons (`source: <path>` / `upstream: <step>`), or `null` when the binary is absent or exits non-zero (the caller falls back).
  - `depthOne(affected: Array<{qualified_name, affected_by: Array<{kind, path?, upstream?}>}>) → Map<string, string[]>` — pure; keeps direct steps and steps whose `upstream` is a direct step.

- [ ] **Step 1: Write the failing tests**

Create `genesis/orchestrator/gate-oracle.test.mjs`:

```js
import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { depthOne, rakiaAffected, resolveRakiaBin } from './gate-oracle.mjs';

const direct = (name, path) => ({ qualified_name: name, affected_by: [{ kind: 'changedFile', path }] });
const viaUp = (name, upstream) => ({ qualified_name: name, affected_by: [{ kind: 'upstreamNode', upstream }] });

describe('depthOne — the local gate stops one hop from a direct change', () => {
  test('keeps direct steps and their immediate dependents, drops the second hop', () => {
    const plan = [
      direct('elohim-sophia:build-sophia-umd', 'sophia'),
      viaUp('elohim:build-angular', 'elohim-sophia:build-sophia-umd'),
      viaUp('elohim:build-site-image', 'elohim:build-angular'),
      viaUp('elohim-genesis:seed-content', 'elohim:build-site-image'),
    ];
    const kept = depthOne(plan);
    assert.deepEqual([...kept.keys()], ['elohim-sophia:build-sophia-umd', 'elohim:build-angular']);
    assert.deepEqual(kept.get('elohim-sophia:build-sophia-umd'), ['source: sophia']);
    assert.deepEqual(kept.get('elohim:build-angular'), ['upstream: elohim-sophia:build-sophia-umd']);
  });

  test('a step reached both directly and via upstream carries both reasons, once each', () => {
    const plan = [
      direct('a:one', 'x/file'),
      { qualified_name: 'a:two', affected_by: [{ kind: 'changedFile', path: 'y/file' }, { kind: 'upstreamNode', upstream: 'a:one' }] },
    ];
    const kept = depthOne(plan);
    assert.deepEqual(kept.get('a:two'), ['source: y/file', 'upstream: a:one']);
  });

  test('an empty plan is an empty map', () => {
    assert.equal(depthOne([]).size, 0);
  });
});

describe('rakiaAffected — the binary is optional and its failure is a fallback, never a crash', () => {
  const root = '/repo';
  test('returns null when no binary resolves', () => {
    assert.equal(rakiaAffected(root, ['sophia'], { rakiaBin: null, spawn: () => { throw new Error('must not spawn'); } }), null);
  });

  test('returns null on a non-zero exit', () => {
    const spawn = () => ({ status: 2, stdout: '', stderr: 'manifest error' });
    assert.equal(rakiaAffected(root, ['sophia'], { rakiaBin: '/bin/rakia', spawn }), null);
  });

  test('passes --repo, --files as one comma-joined argument, and parses the affected list', () => {
    let seen;
    const spawn = (bin, args) => {
      seen = [bin, args];
      return { status: 0, stdout: JSON.stringify({ changed_paths: ['sophia'], affected: [direct('elohim-sophia:build-sophia-umd', 'sophia'), viaUp('elohim:build-angular', 'elohim-sophia:build-sophia-umd')] }), stderr: '' };
    };
    const kept = rakiaAffected(root, ['sophia', 'a b'], { rakiaBin: '/bin/rakia', spawn });
    assert.deepEqual(seen, ['/bin/rakia', ['affected', '--repo', root, '--files', 'sophia,a b']]);
    assert.deepEqual([...kept.keys()], ['elohim-sophia:build-sophia-umd', 'elohim:build-angular']);
  });

  test('an empty changed list never spawns and returns an empty map', () => {
    const kept = rakiaAffected(root, [], { rakiaBin: '/bin/rakia', spawn: () => { throw new Error('must not spawn'); } });
    assert.equal(kept.size, 0);
  });
});

describe('resolveRakiaBin', () => {
  test('prefers RAKIA_BIN when it points at an executable file', () => {
    assert.equal(resolveRakiaBin({ RAKIA_BIN: process.execPath }), process.execPath);
  });
  test('returns null when RAKIA_BIN is set but not executable and PATH has no rakia', () => {
    assert.equal(resolveRakiaBin({ RAKIA_BIN: '/nonexistent/rakia', PATH: '/nonexistent' }), null);
  });
});
```

- [ ] **Step 2: Run to verify they fail**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test gate-oracle.test.mjs`
Expected: FAIL — cannot find module `./gate-oracle.mjs`.

- [ ] **Step 3: Implement**

Create `genesis/orchestrator/gate-oracle.mjs`:

```js
// The rakia oracle for local gate selection — `rakia affected` propagates
// through step `depends`; this module asks it, keeps ONE hop (the local gate's
// depth), and treats every failure as "fall back to path-only", never as a red.
//
// Reason strings match graph-walker's: `source: <path>` for a direct change,
// `upstream: <qualified step>` for a one-hop dependent.

import { accessSync, constants, statSync } from 'fs';
import { spawnSync } from 'child_process';
import { delimiter, isAbsolute, join } from 'path';

function isExecutable(candidate) {
  try {
    if (!statSync(candidate).isFile()) return false;
    accessSync(candidate, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/** RAKIA_BIN if executable, else `rakia` on PATH, else null. */
export function resolveRakiaBin(env = process.env) {
  const declared = (env.RAKIA_BIN || '').trim();
  if (declared) {
    if (isExecutable(isAbsolute(declared) ? declared : join(process.cwd(), declared))) return declared;
    return null;
  }
  for (const dir of (env.PATH || '').split(delimiter).filter(Boolean)) {
    const candidate = join(dir, 'rakia');
    if (isExecutable(candidate)) return candidate;
  }
  return null;
}

/** Pure: direct steps plus their immediate dependents, with graph-walker-shaped reasons. */
export function depthOne(affected) {
  const direct = new Set(
    affected.filter(step => step.affected_by.some(r => r.kind === 'changedFile')).map(step => step.qualified_name),
  );
  const kept = new Map();
  for (const step of affected) {
    const reasons = [];
    for (const reason of step.affected_by) {
      if (reason.kind === 'changedFile' && reason.path) reasons.push(`source: ${reason.path}`);
      else if (reason.kind === 'upstreamNode' && reason.upstream && direct.has(reason.upstream)) reasons.push(`upstream: ${reason.upstream}`);
    }
    if (reasons.length > 0) kept.set(step.qualified_name, [...new Set(reasons)]);
  }
  return kept;
}

/**
 * Ask rakia which steps a change set affects. Returns a Map of qualified step →
 * reasons at depth one, or null when the oracle is unavailable (no binary,
 * non-zero exit, unparsable output) so the caller can fall back.
 */
export function rakiaAffected(root, changedFiles, { rakiaBin, spawn = spawnSync } = {}) {
  if (!rakiaBin) return null;
  const files = changedFiles.filter(Boolean);
  if (files.length === 0) return new Map();
  const result = spawn(rakiaBin, ['affected', '--repo', root, '--files', files.join(',')], { encoding: 'utf8' });
  if (!result || result.status !== 0) return null;
  let parsed;
  try {
    parsed = JSON.parse(result.stdout);
  } catch {
    return null;
  }
  if (!parsed || !Array.isArray(parsed.affected)) return null;
  return depthOne(parsed.affected);
}
```

Add `gate-oracle.test.mjs` to the `test` script in `genesis/orchestrator/package.json` (after `gate-runner.test.mjs`).

- [ ] **Step 4: Run the tests**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test gate-oracle.test.mjs`
Expected: PASS, 9 tests.

- [ ] **Step 5: Commit**

```bash
cd /home/matthew/git/elohim
git add genesis/orchestrator/gate-oracle.mjs genesis/orchestrator/gate-oracle.test.mjs genesis/orchestrator/package.json
git commit -m "gate(oracle): ask rakia affected, keep one hop, fall back honestly

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 9: selection uses the oracle in shadow mode; walker exposes the step→project mapping

**Files:**
- Modify: `genesis/orchestrator/graph-walker.mjs:90-150` (extract Phase 4 into `projectsFromStale`)
- Modify: `genesis/orchestrator/gate-runner.mjs:29-38` (`projectsForChanges`), `:143` (usage), `:157-170` (CLI root)
- Test: `genesis/orchestrator/graph-walker.test.mjs`, `genesis/orchestrator/gate-runner.test.mjs`

**Interfaces:**
- Produces:
  - `projectsFromStale(manifests, stale: Map<string,string[]>, changedFiles) → Array<{name, dir, reasons}>` (graph-walker) — the existing Phase 4 + ordering, callable with an externally supplied stale map.
  - `walkGraph` unchanged in behaviour.
  - `projectsForChanges(root, changedFiles, opts = {}) → projects` (gate-runner): `opts.oracle` = `'shadow' | 'rakia' | 'path'` (default from `env.GATE_ORACLE`, else `'shadow'`), `opts.env`, `opts.log` (line sink, default `process.stdout.write`).
  - `GATE_ROOT` env var overrides the CLI's repository root (fixture repos in tests and the a2o story).

- [ ] **Step 1: Write the failing walker test**

Append to `genesis/orchestrator/graph-walker.test.mjs`:

```js
import { projectsFromStale } from './graph-walker.mjs';

describe('projectsFromStale — an external oracle can supply the stale set', () => {
  it('maps stale steps to gate projects and keeps gate-only input matches', () => {
    const manifests = [
      makeManifest('comp', { ci: makeStep(['elohim/comp', 'elohim/comp/**']) }, {
        projects: { comp: { dir: 'elohim/comp', steps: ['ci'], run: { kind: 'attested', attestation: { provider: 'github-checks', repo: 'o/comp', check: 'ci' } } } },
      }),
      makeManifest('app', { build: makeStep(['app/**'], ['comp:ci']) }, {
        projects: {
          app: { dir: 'app', steps: ['build'], run: { kind: 'just', recipe: 'gate' } },
          schema: { dir: '.', inputs: { sources: ['elohim/comp'], buildProcess: [] }, run: { kind: 'root-just', recipe: '_gate-schema' } },
        },
      }),
    ];
    const stale = new Map([
      ['comp:ci', ['source: elohim/comp']],
      ['app:build', ['upstream: comp:ci']],
    ]);
    const projects = projectsFromStale(manifests, stale, ['elohim/comp']);
    assert.deepEqual(projects.map(p => p.name), ['comp', 'app', 'schema']);
    assert.deepEqual(projects.find(p => p.name === 'app').reasons, ['upstream: comp:ci']);
    assert.deepEqual(projects.find(p => p.name === 'schema').reasons, ['source: elohim/comp']);
  });

  it('walkGraph is projectsFromStale over its own path-only stale set', () => {
    const manifests = [makeManifest('x', { s: makeStep(['x/**']) }, { projects: { x: { dir: 'x', steps: ['s'], run: { kind: 'just', recipe: 'gate' } } } })];
    assert.deepEqual(walkGraph(manifests, ['x/a.ts']).projects, projectsFromStale(manifests, new Map([['x:s', ['source: x/a.ts']]]), ['x/a.ts']));
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test graph-walker.test.mjs`
Expected: FAIL — `projectsFromStale` is not exported.

- [ ] **Step 3: Extract Phase 4 in graph-walker.mjs**

Replace the body of `walkGraph` from the `// Phase 3` comment through the `const projects = …` sort with:

```js
  const projects = projectsFromStale(manifests, stale, changedFiles);
```

and add, above `walkGraph`:

```js
/**
 * Phase 4 as a function: map a stale-step set (qualified name → reasons) to the
 * gate projects that own those steps, plus gate-only projects whose `inputs`
 * match the changed files. Ordered by topological position of the earliest
 * stale step. `stale` may come from the path-only walk (walkGraph) or from
 * an external oracle (gate-oracle.mjs).
 */
export function projectsFromStale(manifests, stale, changedFiles) {
  const stepIndex = new Map();
  for (const { content } of manifests) {
    for (const [name, step] of Object.entries(content.steps)) {
      stepIndex.set(`${content.pipeline}:${name}`, { step, pipeline: content.pipeline, manifest: content });
    }
  }
  const order = topoSort(stepIndex);
  const projectMap = new Map();

  for (const { content } of manifests) {
    if (!content.gate?.projects) continue;
    for (const [projectName, config] of Object.entries(content.gate.projects)) {
      const triggerSteps = config.steps || (config.inputs ? [] : Object.keys(content.steps));
      const reasons = matchInputs(config.inputs, changedFiles);
      let minOrder = Infinity;
      for (const stepName of triggerSteps) {
        const qualified = `${content.pipeline}:${stepName}`;
        if (stale.has(qualified)) {
          reasons.push(...stale.get(qualified));
          const idx = order.indexOf(qualified);
          if (idx >= 0 && idx < minOrder) minOrder = idx;
        }
      }
      if (reasons.length > 0) {
        projectMap.set(projectName, { dir: config.dir, reasons: [...new Set(reasons)], minOrder });
      }
    }
  }

  return [...projectMap.entries()]
    .sort((a, b) => a[1].minOrder - b[1].minOrder)
    .map(([name, { dir, reasons }]) => ({ name, dir, reasons }));
}
```

Keep Phase 5 (pipelines) exactly as it is; it still reads `stale` and `stepIndex` inside `walkGraph`, so `walkGraph` keeps building its own `stepIndex` for Phase 5.

- [ ] **Step 4: Run the walker tests**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test graph-walker.test.mjs`
Expected: PASS (all existing plus the two new).

- [ ] **Step 5: Write the failing runner tests**

Append to `genesis/orchestrator/gate-runner.test.mjs`:

```js
describe('selection oracle — shadow, rakia, path', () => {
  const stale = new Map([
    ['elohim-sophia:build-sophia-umd', ['source: sophia']],
    ['elohim:build-angular', ['upstream: elohim-sophia:build-sophia-umd']],
  ]);
  const withOracle = () => stale;
  const noOracle = () => null;

  test('path mode ignores the oracle entirely', () => {
    const names = projectsForChanges(ROOT, ['sophia'], { oracle: 'path', rakia: withOracle, log: () => {} }).map(p => p.name);
    assert.ok(names.includes('sophia'));
    assert.ok(!names.includes('elohim-app'), 'path-only never propagates');
  });

  test('shadow mode selects by path and prints one oracle-diff line when the sets differ', () => {
    const lines = [];
    const names = projectsForChanges(ROOT, ['sophia'], { oracle: 'shadow', rakia: withOracle, log: l => lines.push(l) }).map(p => p.name);
    assert.ok(!names.includes('elohim-app'), 'shadow mode does not change selection');
    assert.equal(lines.length, 1);
    assert.match(lines[0], /^\[gate\] oracle-diff: \+elohim-app/);
  });

  test('shadow mode is silent when the sets agree', () => {
    const lines = [];
    projectsForChanges(ROOT, ['elohim/sdk/epr-ts/src/index.ts'], { oracle: 'shadow', rakia: () => new Map([['elohim-epr:ts-test', ['source: elohim/sdk/epr-ts/src/index.ts']]]), log: l => lines.push(l) });
    assert.deepEqual(lines, []);
  });

  test('rakia mode selects the direct component and its one-hop consumer with the upstream reason', () => {
    const projects = projectsForChanges(ROOT, ['sophia'], { oracle: 'rakia', rakia: withOracle, log: () => {} });
    const app = projects.find(p => p.name === 'elohim-app');
    assert.ok(app, 'elohim-app is a depth-one consumer of the sophia pin');
    assert.deepEqual(app.reasons, ['upstream: elohim-sophia:build-sophia-umd']);
    assert.ok(projects.some(p => p.name === 'sophia'));
  });

  test('rakia mode falls back to path selection and says so when the oracle is unavailable', () => {
    const lines = [];
    const names = projectsForChanges(ROOT, ['sophia'], { oracle: 'rakia', rakia: noOracle, log: l => lines.push(l) }).map(p => p.name);
    assert.ok(names.includes('sophia'));
    assert.ok(!names.includes('elohim-app'));
    assert.deepEqual(lines, ['[gate] rakia unavailable — path-only selection']);
  });

  test('a gitlink path and files beneath it select the component once, reasons merged', () => {
    const projects = projectsForChanges(ROOT, ['elohim/brit', 'elohim/brit/Cargo.toml'], { oracle: 'path', log: () => {} });
    assert.equal(projects.filter(p => p.name === 'brit').length, 1);
    const brit = projects.find(p => p.name === 'brit');
    assert.ok(brit.reasons.some(r => r === 'source: elohim/brit'));
  });

  test('GATE_ROOT points the CLI at another repository', () => {
    const out = spawnSync(process.execPath, [resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs'), '--list'], {
      cwd: ROOT, encoding: 'utf8', env: { ...process.env, GATE_ROOT: resolve(ROOT, 'genesis/a2o') },
    });
    assert.equal(out.status, 0);
    assert.equal(out.stdout.trim(), '', 'a2o has no build-manifest.json, so the registry is empty');
  });
});
```

- [ ] **Step 6: Run to verify they fail**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test gate-runner.test.mjs`
Expected: FAIL — `projectsForChanges` ignores the third argument; no oracle-diff line; `GATE_ROOT` unknown.

- [ ] **Step 7: Implement in gate-runner.mjs**

Replace the `ROOT` constant and `projectsForChanges`:

```js
import { projectsFromStale, walkGraph } from './graph-walker.mjs';
import { rakiaAffected, resolveRakiaBin } from './gate-oracle.mjs';

const ROOT = process.env.GATE_ROOT
  ? resolve(process.env.GATE_ROOT)
  : resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/** 'shadow' until the flip (spec §8 commit 4) — one line of evidence per differing push, no change in selection. */
export function oracleMode(env = process.env) {
  const declared = (env.GATE_ORACLE || '').trim();
  return ['shadow', 'rakia', 'path'].includes(declared) ? declared : 'shadow';
}

function nameSet(projects) {
  return new Set(projects.map(p => p.name));
}

function oracleDiffLine(pathProjects, oracleProjects) {
  const a = nameSet(pathProjects);
  const b = nameSet(oracleProjects);
  const plus = [...b].filter(n => !a.has(n));
  const minus = [...a].filter(n => !b.has(n));
  if (plus.length === 0 && minus.length === 0) return null;
  return `[gate] oracle-diff: ${plus.map(n => `+${n}`).join(' ')}${plus.length && minus.length ? ' ' : ''}${minus.map(n => `-${n}`).join(' ')}`;
}

export function projectsForChanges(root, changedFiles, opts = {}) {
  const env = opts.env || process.env;
  const mode = opts.oracle || oracleMode(env);
  const log = opts.log || (line => process.stdout.write(`${line}\n`));
  const manifests = loadManifests(root);
  const registry = loadGateRegistry(root);
  const files = filterChanged(changedFiles);

  const byPath = walkGraph(manifests, files).projects;
  let chosen = byPath;

  if (mode !== 'path') {
    const ask = opts.rakia || (() => rakiaAffected(root, files, { rakiaBin: resolveRakiaBin(env) }));
    const stale = ask();
    if (stale === null) {
      if (mode === 'rakia') log('[gate] rakia unavailable — path-only selection');
    } else {
      const byOracle = projectsFromStale(manifests, stale, files);
      if (mode === 'rakia') {
        chosen = byOracle;
      } else {
        const diff = oracleDiffLine(byPath, byOracle);
        if (diff) log(diff);
      }
    }
  }

  return chosen.map(project => {
    const registered = registry.get(project.name);
    if (!registered) throw new Error(`Detected unregistered gate project: ${project.name}`);
    return { ...registered, reasons: project.reasons };
  });
}
```

Update `usage()` to mention the environment: `usage: gate-runner.mjs (--target <project-or-path> | --changed-file-list | --list) [--print] [--names]   env: GATE_ORACLE=shadow|rakia|path (default shadow), RAKIA_BIN, GATE_ROOT`.

- [ ] **Step 8: Run the runner tests and the whole orchestrator suite**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test gate-runner.test.mjs && pnpm -s test; echo EXIT=$?`
Expected: PASS; `EXIT=0`.

- [ ] **Step 9: Commit**

```bash
cd /home/matthew/git/elohim
git add genesis/orchestrator/graph-walker.mjs genesis/orchestrator/graph-walker.test.mjs genesis/orchestrator/gate-runner.mjs genesis/orchestrator/gate-runner.test.mjs
git commit -m "gate(select): rakia is the oracle in shadow mode — one diff line, selection unchanged

projectsFromStale is Phase 4 as a function so an external stale set maps
to gate projects the same way the path-only walk does. GATE_ORACLE=rakia
selects at depth one with upstream reasons; absent rakia falls back to
path-only and says so; GATE_ROOT lets a fixture repository drive the CLI.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 10: `gate-attest.mjs` — read the pin's check; the runner dispatches `attested` to it

**Files:**
- Create: `genesis/orchestrator/gate-attest.mjs`
- Test: `genesis/orchestrator/gate-attest.test.mjs`
- Modify: `genesis/orchestrator/gate-runner.mjs` (`runProject` dispatch), `genesis/orchestrator/package.json:6`

**Interfaces:**
- Produces:
  - `pinnedSha(root, dir, { spawn }) → string` — `git rev-parse HEAD:<dir>`; throws `ManifestError` (exit 2) when the path is not a gitlink.
  - `worktreeDirty(root, dir, { spawn }) → boolean` — `git -C <root>/<dir> status --porcelain` non-empty; `false` when the dir is absent.
  - `readCheck(attestation, sha, { ghBin, spawn }) → { read: 'ok' | 'failed', conclusion?: string, status?: string, reason?: string }` — newest run of the named check.
  - `judge(read) → { outcome: 'pass' | 'refuse', tier: 'witnessed' | 'claimed', conclusion: string, line: string }`.
  - `runAttested(project, { root, env, spawn, log, runEpr }) → number` — exit status; records the observation.
  - Observation: `epr flow note --kind observation --measure pin-attestation@1 --subject <dir> --value 1 --unit reads --env provider=<p> --env check=<c> --env conclusion=<success|failure|absent|pending|unreachable> --env tier=<witnessed|claimed>`.

- [ ] **Step 1: Write the failing tests**

Create `genesis/orchestrator/gate-attest.test.mjs`:

```js
import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { judge, observationArgs, readCheck, runAttested } from './gate-attest.mjs';

const attestation = { provider: 'github-checks', repo: 'ethosengine/brit', check: 'Tests pass' };
const SHA = 'b86c5104d3398d89fe56d23199a9393fec534870';

function ghReturning(runs) {
  return (bin, args) => ({ status: 0, stdout: JSON.stringify({ check_runs: runs }), stderr: '', args });
}
const run = (name, status, conclusion, started_at) => ({ name, status, conclusion, started_at });

describe('readCheck — the newest run of the named check', () => {
  test('asks the check-runs endpoint for the SHA and returns the newest named run', () => {
    let seen;
    const spawn = (bin, args) => { seen = args; return ghReturning([run('Tests pass', 'completed', 'failure', '2026-09-01T00:00:00Z'), run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z'), run('lint', 'completed', 'failure', '2026-09-03T00:00:00Z')])(bin, args); };
    const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn });
    assert.deepEqual(read, { read: 'ok', status: 'completed', conclusion: 'success' });
    assert.deepEqual(seen, ['api', `repos/ethosengine/brit/commits/${SHA}/check-runs?per_page=100`]);
  });
  test('an absent check reads as conclusion "absent"', () => {
    assert.deepEqual(readCheck(attestation, SHA, { ghBin: 'gh', spawn: ghReturning([run('lint', 'completed', 'success', '2026-09-01T00:00:00Z')]) }), { read: 'ok', status: 'absent', conclusion: 'absent' });
  });
  test('a gh failure is read: failed with the stderr as reason', () => {
    const read = readCheck(attestation, SHA, { ghBin: 'gh', spawn: () => ({ status: 4, stdout: '', stderr: 'gh: not logged in' }) });
    assert.deepEqual(read, { read: 'failed', reason: 'gh: not logged in' });
  });
  test('no gh binary is read: failed', () => {
    assert.deepEqual(readCheck(attestation, SHA, { ghBin: null, spawn: () => { throw new Error('must not spawn'); } }), { read: 'failed', reason: 'gh not found' });
  });
});

describe('judge — four outcomes', () => {
  test('success passes as witnessed', () => {
    const v = judge({ read: 'ok', status: 'completed', conclusion: 'success' }, attestation, SHA);
    assert.equal(v.outcome, 'pass'); assert.equal(v.tier, 'witnessed'); assert.equal(v.conclusion, 'success');
    assert.equal(v.line, 'attested: ethosengine/brit@b86c5104d339 Tests pass success');
  });
  for (const bad of ['failure', 'cancelled', 'timed_out', 'action_required']) {
    test(`${bad} refuses as witnessed`, () => {
      const v = judge({ read: 'ok', status: 'completed', conclusion: bad }, attestation, SHA);
      assert.equal(v.outcome, 'refuse'); assert.equal(v.tier, 'witnessed'); assert.equal(v.conclusion, 'failure');
      assert.match(v.line, new RegExp(`Tests pass ${bad} at b86c5104d339 — a pin without its attestation is not a green pin`));
    });
  }
  test('absent refuses', () => {
    const v = judge({ read: 'ok', status: 'absent', conclusion: 'absent' }, attestation, SHA);
    assert.equal(v.outcome, 'refuse'); assert.equal(v.conclusion, 'absent');
  });
  test('in progress refuses as pending', () => {
    const v = judge({ read: 'ok', status: 'in_progress', conclusion: null }, attestation, SHA);
    assert.equal(v.outcome, 'refuse'); assert.equal(v.conclusion, 'pending');
    assert.match(v.line, /not yet concluded/);
  });
  test('an unreachable read passes as claimed', () => {
    const v = judge({ read: 'failed', reason: 'offline' }, attestation, SHA);
    assert.equal(v.outcome, 'pass'); assert.equal(v.tier, 'claimed'); assert.equal(v.conclusion, 'unreachable');
    assert.equal(v.line, 'attested: claimed — offline');
  });
});

describe('observationArgs', () => {
  test('shapes the epr flow note exactly as measures.yaml declares', () => {
    assert.deepEqual(observationArgs('elohim/brit', attestation, { conclusion: 'success', tier: 'witnessed' }), [
      'flow', 'note', '--kind', 'observation', '--measure', 'pin-attestation@1',
      '--subject', 'elohim/brit', '--value', '1', '--unit', 'reads',
      '--env', 'provider=github-checks', '--env', 'check=Tests pass', '--env', 'conclusion=success', '--env', 'tier=witnessed',
    ]);
  });
});

describe('runAttested — reads the pin, not the checkout', () => {
  const project = { name: 'brit', dir: 'elohim/brit', run: { kind: 'attested', attestation } };
  function fakeGit({ sha = SHA, dirty = false, gitlink = true } = {}) {
    return (bin, args) => {
      if (bin !== 'git') return null;
      if (args[0] === 'rev-parse') return gitlink ? { status: 0, stdout: `${sha}\n`, stderr: '' } : { status: 128, stdout: '', stderr: 'fatal: path not in tree' };
      if (args.includes('status')) return { status: 0, stdout: dirty ? ' M Cargo.toml\n' : '', stderr: '' };
      return { status: 1, stdout: '', stderr: `unexpected git ${args.join(' ')}` };
    };
  }
  function harness(gh, git) {
    const lines = []; const epr = [];
    const spawn = (bin, args, o) => bin === 'git' ? git(bin, args, o) : gh(bin, args, o);
    return { lines, epr, deps: { root: '/repo', env: { GH_BIN: 'gh' }, spawn, log: l => lines.push(l), runEpr: a => { epr.push(a); return 0; } } };
  }

  test('green check → exit 0, one attested line, one witnessed observation', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z')]), fakeGit());
    assert.equal(runAttested(project, h.deps), 0);
    assert.deepEqual(h.lines, ['attested: ethosengine/brit@b86c5104d339 Tests pass success']);
    assert.equal(h.epr.length, 1);
    assert.ok(h.epr[0].includes('tier=witnessed'));
  });
  test('red check → exit 1', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'failure', '2026-09-02T00:00:00Z')]), fakeGit());
    assert.equal(runAttested(project, h.deps), 1);
    assert.ok(h.epr[0].includes('conclusion=failure'));
  });
  test('unreachable → exit 0 as claimed', () => {
    const h = harness(() => ({ status: 1, stdout: '', stderr: 'offline' }), fakeGit());
    assert.equal(runAttested(project, h.deps), 0);
    assert.deepEqual(h.lines, ['attested: claimed — offline']);
    assert.ok(h.epr[0].includes('tier=claimed'));
  });
  test('a dirty worktree is reported on its own line and changes nothing', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z')]), fakeGit({ dirty: true }));
    assert.equal(runAttested(project, h.deps), 0);
    assert.equal(h.lines[0], 'attested: worktree dirty — the committed pin is what is read; the component’s own gate governs the worktree');
  });
  test('a path that is not a gitlink is a manifest error (exit 2), not a gate red', () => {
    const h = harness(ghReturning([]), fakeGit({ gitlink: false }));
    assert.equal(runAttested(project, h.deps), 2);
    assert.equal(h.epr.length, 0);
  });
  test('the observation is best-effort: a failing epr never changes the exit', () => {
    const h = harness(ghReturning([run('Tests pass', 'completed', 'success', '2026-09-02T00:00:00Z')]), fakeGit());
    h.deps.runEpr = () => { throw new Error('epr missing'); };
    assert.equal(runAttested(project, h.deps), 0);
  });
});
```

- [ ] **Step 2: Run to verify they fail**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test gate-attest.test.mjs`
Expected: FAIL — cannot find module `./gate-attest.mjs`.

- [ ] **Step 3: Implement**

Create `genesis/orchestrator/gate-attest.mjs`:

```js
// The attested gate — a submodule pin is gated by the pinned commit's own
// upstream attestation. Reads the pin (never the worktree), reads the named
// check at that SHA, and records the read as a pin-attestation@1 observation.
//
// Reading is free, trusting is a choice: an unreachable read passes as
// `claimed` (memoized-derivation Law III); a red, cancelled, pending or absent
// check refuses. Nothing here ever runs the component's suite.

import { accessSync, constants, statSync } from 'fs';
import { spawnSync } from 'child_process';
import { delimiter, isAbsolute, join } from 'path';

export const PIN_ATTESTATION_MEASURE = 'pin-attestation@1';

const RED = new Set(['failure', 'cancelled', 'timed_out', 'action_required', 'stale', 'startup_failure']);

function isExecutable(candidate) {
  try {
    if (!statSync(candidate).isFile()) return false;
    accessSync(candidate, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

export function resolveGhBin(env = process.env) {
  const declared = (env.GH_BIN || '').trim();
  if (declared) return isExecutable(isAbsolute(declared) ? declared : join(process.cwd(), declared)) ? declared : null;
  for (const dir of (env.PATH || '').split(delimiter).filter(Boolean)) {
    const candidate = join(dir, 'gh');
    if (isExecutable(candidate)) return candidate;
  }
  return null;
}

export class ManifestError extends Error {}

export function pinnedSha(root, dir, { spawn = spawnSync } = {}) {
  const result = spawn('git', ['-C', root, 'rev-parse', `HEAD:${dir}`], { encoding: 'utf8' });
  const sha = (result?.stdout || '').trim();
  if (result?.status !== 0 || !/^[0-9a-f]{40}$/.test(sha)) {
    throw new ManifestError(`${dir} is not a gitlink at HEAD (${(result?.stderr || '').trim()})`);
  }
  return sha;
}

export function worktreeDirty(root, dir, { spawn = spawnSync } = {}) {
  const result = spawn('git', ['-C', join(root, dir), 'status', '--porcelain'], { encoding: 'utf8' });
  return result?.status === 0 && (result.stdout || '').trim().length > 0;
}

export function readCheck(attestation, sha, { ghBin, spawn = spawnSync } = {}) {
  if (!ghBin) return { read: 'failed', reason: 'gh not found' };
  const result = spawn(ghBin, ['api', `repos/${attestation.repo}/commits/${sha}/check-runs?per_page=100`], { encoding: 'utf8' });
  if (!result || result.status !== 0) return { read: 'failed', reason: (result?.stderr || 'gh exited non-zero').trim() };
  let runs;
  try {
    runs = JSON.parse(result.stdout).check_runs || [];
  } catch {
    return { read: 'failed', reason: 'unparsable check-runs response' };
  }
  const named = runs.filter(r => r.name === attestation.check).sort((a, b) => String(b.started_at || '').localeCompare(String(a.started_at || '')));
  if (named.length === 0) return { read: 'ok', status: 'absent', conclusion: 'absent' };
  return { read: 'ok', status: named[0].status, conclusion: named[0].conclusion };
}

export function judge(read, attestation, sha) {
  const short = sha.slice(0, 12);
  const where = `${attestation.repo}@${short} ${attestation.check}`;
  if (read.read !== 'ok') {
    return { outcome: 'pass', tier: 'claimed', conclusion: 'unreachable', line: `attested: claimed — ${read.reason}` };
  }
  if (read.status === 'absent') {
    return { outcome: 'refuse', tier: 'witnessed', conclusion: 'absent', line: `attested: REFUSED — ${where} has no run at ${short} — a pin without its attestation is not a green pin` };
  }
  if (read.status !== 'completed') {
    return { outcome: 'refuse', tier: 'witnessed', conclusion: 'pending', line: `attested: REFUSED — ${where} not yet concluded (${read.status}) at ${short}` };
  }
  if (read.conclusion === 'success') {
    return { outcome: 'pass', tier: 'witnessed', conclusion: 'success', line: `attested: ${where} success` };
  }
  const label = RED.has(read.conclusion) ? read.conclusion : `${read.conclusion} (not success)`;
  return { outcome: 'refuse', tier: 'witnessed', conclusion: 'failure', line: `attested: REFUSED — ${where} ${label} at ${short} — a pin without its attestation is not a green pin` };
}

export function observationArgs(subject, attestation, verdict) {
  return [
    'flow', 'note', '--kind', 'observation',
    '--measure', PIN_ATTESTATION_MEASURE,
    '--subject', subject, '--value', '1', '--unit', 'reads',
    '--env', `provider=${attestation.provider}`,
    '--env', `check=${attestation.check}`,
    '--env', `conclusion=${verdict.conclusion}`,
    '--env', `tier=${verdict.tier}`,
  ];
}

/** Run one attested gate project. Returns the exit status (0 pass, 1 refuse, 2 manifest error). */
export function runAttested(project, { root, env = process.env, spawn = spawnSync, log, runEpr }) {
  const say = log || (line => process.stdout.write(`${line}\n`));
  const attestation = project.run.attestation;
  let sha;
  try {
    sha = pinnedSha(root, project.dir, { spawn });
  } catch (error) {
    say(`gate ${project.name}: ${error.message}`);
    return 2;
  }
  if (worktreeDirty(root, project.dir, { spawn })) {
    say('attested: worktree dirty — the committed pin is what is read; the component’s own gate governs the worktree');
  }
  const verdict = judge(readCheck(attestation, sha, { ghBin: resolveGhBin(env), spawn }), attestation, sha);
  say(verdict.line);
  try {
    (runEpr || (args => spawn(env.EPR_BIN || 'epr', args, { cwd: root, stdio: 'ignore', timeout: 30000 })?.status))(observationArgs(project.dir, attestation, verdict));
  } catch { /* best-effort; the verdict never depends on the observation */ }
  return verdict.outcome === 'pass' ? 0 : 1;
}
```

- [ ] **Step 4: Dispatch from the runner**

In `gate-runner.mjs`, import `runAttested` from `./gate-attest.mjs`, and at the top of `runProject` (after the `namesOnly` / `printOnly` branches, before the `[gate]` banner) add:

```js
  if (project.run.kind === 'attested') {
    process.stdout.write(`\n[gate] ${project.name} (${project.dir}) — attested, no local recipe\n`);
    return runAttested(project, {
      root: ROOT,
      env: process.env,
      log: line => process.stdout.write(`${line}\n`),
      runEpr: eprArgs => spawnSync(process.env.EPR_BIN || 'epr', eprArgs, { cwd: ROOT, stdio: 'ignore', timeout: 30000 }).status,
    });
  }
```

Also guard the `--print` branch so it never reads `project.run.cargo` for attested projects (`const cargo = project.run.cargo || {};` already does).

Add `gate-attest.test.mjs` to the `test` script in `genesis/orchestrator/package.json`.

Append to `gate-runner.test.mjs`:

```js
  test('an attested project never reaches run-local-gate.sh', () => {
    const out = spawnSync(process.execPath, [resolve(ROOT, 'genesis/orchestrator/gate-runner.mjs'), '--target', 'brit'], {
      cwd: ROOT, encoding: 'utf8', env: { ...process.env, GH_BIN: '/nonexistent/gh', EPR_BIN: '/nonexistent/epr' },
    });
    assert.equal(out.status, 0, out.stdout + out.stderr);
    assert.match(out.stdout, /attested, no local recipe/);
    assert.match(out.stdout, /attested: claimed — gh not found/);
    assert.doesNotMatch(out.stdout, /cargo target:/);
  });
```

- [ ] **Step 5: Run the tests, then one real read**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test gate-attest.test.mjs gate-runner.test.mjs && pnpm -s test; echo EXIT=$?`
Expected: PASS; `EXIT=0`.

Then a live read with the real authed CLI: `cd /home/matthew/git/elohim && node genesis/orchestrator/gate-runner.mjs --target brit; echo EXIT=$?`
Expected: `attested: ethosengine/brit@b86c5104d339 Tests pass success` (or the current pin) and `EXIT=0`.

- [ ] **Step 6: Commit**

```bash
cd /home/matthew/git/elohim
git add genesis/orchestrator/gate-attest.mjs genesis/orchestrator/gate-attest.test.mjs genesis/orchestrator/gate-runner.mjs genesis/orchestrator/gate-runner.test.mjs genesis/orchestrator/package.json
git commit -m "gate(attested): a pin is gated by the pinned commit's own check — red refuses, unreachable passes as claimed

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 11: the measure, the habit atom, and the spec amendment

**Files:**
- Modify: `.claude/epr-meta/measures.yaml` (measure list, after `gate-cycle-seconds`)
- Create: `genesis/orchestrator/.epr-meta/pin-attestation.habit.md`
- Regenerate: `genesis/manifests/habits.yaml`
- Modify: `genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md` §4.5 first paragraph

- [ ] **Step 1: Declare the measure**

In `.claude/epr-meta/measures.yaml`, directly after the `gate-cycle-seconds` measure entry (the one whose `established_by` is `operator-requested-2026-09-19`), add:

```yaml
  - id: pin-attestation
    version: 1
    family: dev-cycle
    unit: reads
    procedure: "genesis/orchestrator/gate-attest.mjs runAttested — ONE read of a submodule pin's upstream attestation, recorded as `epr flow note --kind observation --measure pin-attestation@1 --subject <gate dir> --value 1 --unit reads --env provider=<github-checks> --env check=<check-run name> --env conclusion=<success|failure|absent|pending|unreachable> --env tier=<witnessed|claimed>`. tier=witnessed when the read succeeded whatever it concluded; tier=claimed when the read itself failed (no gh, offline) and the gate passed on the floor. No ceiling reads it yet — it is the seed rung 1 grows from."
    provenance: "2026-09-23: a sophia pointer move ran sophia's whole suite on a brit-focused host; a brit pointer move ran nothing. genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md §4.6."
    default-authority: observation
    status: active
    established_by: spec-submodule-pin-attestation-gate-2026-09-23
```

- [ ] **Step 2: Write the habit atom**

Create `genesis/orchestrator/.epr-meta/pin-attestation.habit.md`:

```markdown
---
epr-habit-version: 1
id: pin-attestation
invariant: >
  A submodule pin move is gated by the pinned commit's own attestation and re-gates
  the pin's direct consumers; the component's suite is never re-run from the monorepo.
  A red, cancelled, pending or absent attestation refuses the pin; an unreachable read
  passes on the floor and says `claimed`.
status: red
active: false
checks:
  - "a2o @concern:pin-attestation (genesis/a2o/features/devflow/pin-attestation.feature — five scenarios over a scratch superproject with one gitlink and a fake `gh`: green passes, red refuses, absent refuses, unreachable passes as claimed, and a pin move selects the one-hop consumer and nothing deeper; default profile: cd genesis/a2o && npx cucumber-js --tags '@concern:pin-attestation')"
  - "node --test genesis/orchestrator/gate-attest.test.mjs genesis/orchestrator/gate-oracle.test.mjs (the four outcomes, latest-run-wins, reads-the-pin-not-the-checkout, depth one)"
  - "epr flow note observations on pin-attestation@1 exist for every pin-moving push (GATE_ORACLE and the attested dispatch are live in gate-runner.mjs)"
guard: >
  Regression risks: (1) a matcher that special-cases gitlinks instead of the manifest
  listing the pin path — both oracles must agree by declaration, not by code;
  (2) widening `gateAttestation` past its three fields (thresholds, signers) before
  rung 2 exists to read them; (3) greening this habit by trusting a green badge
  rather than reading it — `tier=claimed` is a pass, never evidence.
refs:
  - "spec: genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md"
  - "plan: genesis/docs/superpowers/plans/2026-09-23-submodule-pin-attestation-gate-plan.md"
  - "ladder: genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (rung 3 of the component ladder named 2026-09-23)"
  - "oracle: elohim/rakia/docs/specs/2026-04-12-rakia-design.md §4 open question 4 (shadow, then switch)"
retire-when: >
  when attestations are read from the dataplane rather than a forge — rung 2 landing makes
  `github-checks` one provider among peers — and this habit describes a product, not a practice.
---
DELTA 2026-09-23 (born RED): spec and plan authored; the codegen gate was found red at the
pinned rakia schema and the two oracles disagree on a bare gitlink path. No landing yet.
```

- [ ] **Step 3: Re-project the register and check it**

Run: `cd /home/matthew/git/elohim && python3 .claude/scripts/habits-project.py && python3 .claude/scripts/habits-project.py --check; echo EXIT=$?`
Expected: `EXIT=0`; `git diff --stat genesis/manifests/habits.yaml` shows the new habit; `python3 .claude/scripts/habits-status.py` lists `pin-attestation` red, not active (the two-active fence untouched).

- [ ] **Step 4: Amend the spec §4.5 to name the module boundary this plan chose**

In the spec, replace the first paragraph of §4.5 (`run-local-gate.sh gains an attested branch …` through `… stays at its current arity.`) with:

```markdown
`gate-runner.mjs` dispatches a project whose `run.kind` is `attested` to a new module,
`genesis/orchestrator/gate-attest.mjs`, and never to `run-local-gate.sh` (which keeps refusing
unknown kinds). Provider, repo and check come from the registry entry; nothing travels through
argv or the environment. Keeping the read in Node lets the four outcomes be pinned by `node:test`
with a fake `gh` and reuse the gate-cycle observation path.
```

Then re-seal: `epr flow cites seal genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md`.

- [ ] **Step 5: Commit**

```bash
cd /home/matthew/git/elohim
git add .claude/epr-meta/measures.yaml genesis/orchestrator/.epr-meta/pin-attestation.habit.md genesis/manifests/habits.yaml genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md
git commit -m "habit(pin-attestation): born red — the measure, the atom, the register projection

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 12: the story — five scenarios over a scratch superproject

**Files:**
- Create: `genesis/a2o/features/devflow/pin-attestation.feature`
- Create: `genesis/a2o/steps/devflow/pin-attestation.steps.ts`
- Create: `genesis/a2o/steps/devflow/rakia-cli.guard.ts`

**Interfaces:**
- Consumes: `gate-runner.mjs` CLI with `GATE_ROOT`, `GATE_ORACLE`, `GH_BIN`, `EPR_BIN`, `RAKIA_BIN`.
- The fixture: a temp superproject with a committed gitlink `comp` (a local sub-repository added with `-c protocol.file.allow=always`), a component manifest (`comp/build-manifest.json`, attested on `o/comp` check `ci`), and a consumer manifest (`app/build-manifest.json`, step `build` depends `comp:ci`, gate project `app`; step `deep` depends `build`, gate project `deep`). A fake `gh` script on `GH_BIN` reads `FAKE_GH_CONCLUSION` (`success` | `failure` | `absent` | `unreachable`).

- [ ] **Step 1: Write the feature**

Create `genesis/a2o/features/devflow/pin-attestation.feature`:

```gherkin
# A submodule pin move is gated by the pinned commit's own attestation and re-gates
# its direct consumers. Driven the way `steps/devflow/run-plane.steps.ts` drives the
# `epr` CLI: every scenario mints its OWN scratch superproject (one gitlink, two
# manifests, a fake `gh` on GH_BIN) and runs the real gate-runner against it with
# GATE_ROOT, so nothing here reads or writes this repository's manifests or ledgers.
#
# Habit: genesis/orchestrator/.epr-meta/pin-attestation.habit.md
# Spec:  genesis/docs/superpowers/specs/2026-09-23-submodule-pin-attestation-gate-design.md
#
# The tags are suite-routing labels, not behaviour: @e2e and @devflow route this file
# into the devflow suite; @concern:pin-attestation joins each scenario to the habit
# check that claims it. The propagation scenario needs the `rakia` binary and carries
# @requires:rakia-cli — steps/devflow/rakia-cli.guard.ts skips it, never fails it, when
# the binary is absent.

@e2e @devflow @act:host
Feature: A submodule pin is gated by its own attestation

  Background:
    Given a scratch superproject with a component pinned as the gitlink "comp"
    And the component declares an attested gate on "o/comp" check "ci"
    And a consumer step "app:build" depends on "comp:ci" and a deeper step "app:deep" depends on "app:build"

  @concern:pin-attestation
  Scenario: A green upstream check passes the pin
    Given the upstream check at the pinned commit concluded "success"
    When the gate runs for project "comp"
    Then the gate exits 0
    And the gate printed "attested: o/comp@" followed by "ci success"
    And one pin-attestation observation was recorded with tier "witnessed" and conclusion "success"

  @concern:pin-attestation
  Scenario: A red upstream check refuses the pin
    Given the upstream check at the pinned commit concluded "failure"
    When the gate runs for project "comp"
    Then the gate exits 1
    And the gate printed "a pin without its attestation is not a green pin"
    And one pin-attestation observation was recorded with tier "witnessed" and conclusion "failure"

  @concern:pin-attestation
  Scenario: An absent upstream check refuses the pin
    Given the upstream has no run of the check at the pinned commit
    When the gate runs for project "comp"
    Then the gate exits 1
    And the gate printed "has no run at"

  @concern:pin-attestation
  Scenario: An unreachable read passes on the floor and says so
    Given the upstream cannot be read
    When the gate runs for project "comp"
    Then the gate exits 0
    And the gate printed "attested: claimed —"
    And one pin-attestation observation was recorded with tier "claimed" and conclusion "unreachable"

  @concern:pin-attestation @requires:rakia-cli
  Scenario: A pin move selects the component and its one-hop consumer, and nothing deeper
    When selection runs for the changed path "comp" with the rakia oracle
    Then the selected projects are "comp, app"
    And project "app" was selected because of "upstream: comp:ci"
```

- [ ] **Step 2: Write the guard**

Create `genesis/a2o/steps/devflow/rakia-cli.guard.ts` — the same shape as `epr-cli.guard.ts`, for `rakia`:

```ts
/**
 * The `@requires:rakia-cli` gate — the rakia planner is a FIXTURE precondition (a binary on
 * PATH or RAKIA_BIN), not a substrate capability, so the cluster-state arms ignore it. A
 * scenario that needs it is SKIPPED with the reason printed once when the binary is absent;
 * it is never failed and nothing is installed. Mirrors steps/devflow/epr-cli.guard.ts.
 */
import { accessSync, constants, statSync } from 'node:fs';
import { delimiter, isAbsolute, join } from 'node:path';

import { Before } from '@cucumber/cucumber';

const binary = process.env.RAKIA_BIN ?? 'rakia';

function isExecutableFile(candidate: string): boolean {
  try {
    if (!statSync(candidate).isFile()) return false;
    accessSync(candidate, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

function resolveBinary(): string | null {
  if (isAbsolute(binary) || binary.includes('/')) return isExecutableFile(binary) ? binary : null;
  for (const dir of (process.env.PATH ?? '').split(delimiter).filter(Boolean)) {
    const candidate = join(dir, binary);
    if (isExecutableFile(candidate)) return candidate;
  }
  return null;
}

let announced = false;

Before({ tags: '@requires:rakia-cli' }, function () {
  if (resolveBinary()) return undefined;
  if (!announced) {
    announced = true;
    console.log(`  [rakia-cli] '${binary}' is not on PATH — @requires:rakia-cli scenarios SKIPPED, not measured`);
  }
  return 'skipped';
});
```

- [ ] **Step 3: Write the steps**

Create `genesis/a2o/steps/devflow/pin-attestation.steps.ts`:

```ts
/**
 * Drives genesis/orchestrator/gate-runner.mjs against a scratch superproject. One gitlink
 * (`comp`), two manifests, a fake `gh` whose answer is FAKE_GH_CONCLUSION, a fake `epr` that
 * appends its argv to a file so the observation can be asserted. Every scenario owns its
 * fixture and removes it afterwards.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

const REPO_ROOT = resolve(__dirname, '../../../..');
const GATE_RUNNER = join(REPO_ROOT, 'genesis/orchestrator/gate-runner.mjs');
const scratchBase = process.env.A2O_TMPDIR ?? tmpdir();

interface Fixture {
  root: string;
  bin: string;
  eprLog: string;
  conclusion: string;
  status?: number;
  stdout?: string;
}

let fx: Fixture | undefined;

function git(cwd: string, args: string[]): string {
  const r = spawnSync('git', ['-c', 'protocol.file.allow=always', '-c', 'user.email=a2o@example.test', '-c', 'user.name=a2o', ...args], { cwd, encoding: 'utf8' });
  assert.equal(r.status, 0, `git ${args.join(' ')}: ${r.stderr}`);
  return r.stdout.trim();
}

function writeJson(path: string, value: unknown): void {
  mkdirSync(join(path, '..'), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function step(sources: string[], depends: string[] = []) {
  return { description: 's', inputs: { sources, buildProcess: [] }, outputs: { artifacts: [], verify: null }, depends, executor: { stage: 'S', function: null } };
}

Given('a scratch superproject with a component pinned as the gitlink {string}', function (gitlink: string) {
  const base = mkdtempSync(join(scratchBase, 'pin-attestation-'));
  const sub = join(base, 'sub');
  mkdirSync(sub);
  git(sub, ['init', '-q', '-b', 'main']);
  writeFileSync(join(sub, 'README.md'), 'component\n');
  git(sub, ['add', '.']);
  git(sub, ['commit', '-q', '-m', 'component']);

  const root = join(base, 'super');
  mkdirSync(root);
  git(root, ['init', '-q', '-b', 'main']);
  writeFileSync(join(root, '.ci-ignore'), '');
  git(root, ['submodule', 'add', '-q', sub, gitlink]);

  const bin = join(base, 'bin');
  mkdirSync(bin);
  const eprLog = join(base, 'epr.log');
  writeFileSync(join(bin, 'gh'), [
    '#!/usr/bin/env bash',
    'case "${FAKE_GH_CONCLUSION:-success}" in',
    '  unreachable) echo "gh: could not resolve host" >&2; exit 1 ;;',
    '  absent) echo \'{"check_runs":[]}\' ;;',
    '  *) printf \'{"check_runs":[{"name":"ci","status":"completed","conclusion":"%s","started_at":"2026-09-23T00:00:00Z"}]}\' "$FAKE_GH_CONCLUSION" ;;',
    'esac',
    '',
  ].join('\n'));
  chmodSync(join(bin, 'gh'), 0o755);
  writeFileSync(join(bin, 'epr'), `#!/usr/bin/env bash\nprintf '%s\\n' "$*" >> '${eprLog}'\n`);
  chmodSync(join(bin, 'epr'), 0o755);

  fx = { root, bin, eprLog, conclusion: 'success' };
});

Given('the component declares an attested gate on {string} check {string}', function (repo: string, check: string) {
  assert.ok(fx);
  writeJson(join(fx.root, 'comp/build-manifest.json'), {
    manifestVersion: '1.0', pipeline: 'comp', manualOnly: true, description: 'component',
    steps: { ci: step(['comp', 'comp/**']) },
    gate: { projects: { comp: { dir: 'comp', steps: ['ci'], run: { kind: 'attested', attestation: { provider: 'github-checks', repo, check } } } } },
  });
  // The manifest lives in the submodule's worktree in the real tree; here it lives beside the
  // gitlink so the superproject can commit it without touching the sub-repository.
  git(fx.root, ['add', '-f', 'comp/build-manifest.json']);
});

Given('a consumer step {string} depends on {string} and a deeper step {string} depends on {string}', function (build: string, on: string, deep: string, onBuild: string) {
  assert.ok(fx);
  const [pipeline, buildStep] = build.split(':');
  const [, deepStep] = deep.split(':');
  const [, onBuildStep] = onBuild.split(':');
  writeJson(join(fx.root, 'app/build-manifest.json'), {
    manifestVersion: '1.0', pipeline, description: 'consumer',
    steps: { [buildStep]: step(['app/**'], [on]), [deepStep]: step(['app/deep/**'], [onBuildStep]) },
    gate: { projects: { app: { dir: 'app', steps: [buildStep], run: { kind: 'just', recipe: 'gate' } }, deep: { dir: 'app', steps: [deepStep], run: { kind: 'just', recipe: 'gate' } } } },
  });
  git(fx.root, ['add', '.']);
  git(fx.root, ['commit', '-q', '-m', 'superproject']);
});

Given('the upstream check at the pinned commit concluded {string}', function (conclusion: string) { assert.ok(fx); fx.conclusion = conclusion; });
Given('the upstream has no run of the check at the pinned commit', function () { assert.ok(fx); fx.conclusion = 'absent'; });
Given('the upstream cannot be read', function () { assert.ok(fx); fx.conclusion = 'unreachable'; });

function runGate(args: string[], extraEnv: NodeJS.ProcessEnv = {}): void {
  assert.ok(fx);
  const r = spawnSync(process.execPath, [GATE_RUNNER, ...args], {
    cwd: fx.root, encoding: 'utf8',
    env: { ...process.env, GATE_ROOT: fx.root, GH_BIN: join(fx.bin, 'gh'), EPR_BIN: join(fx.bin, 'epr'), FAKE_GH_CONCLUSION: fx.conclusion, ...extraEnv },
  });
  fx.status = r.status ?? -1;
  fx.stdout = `${r.stdout}${r.stderr}`;
}

When('the gate runs for project {string}', function (project: string) { runGate(['--target', project]); });

When('selection runs for the changed path {string} with the rakia oracle', function (changed: string) {
  assert.ok(fx);
  const r = spawnSync(process.execPath, [GATE_RUNNER, '--changed-file-list', '--print'], {
    cwd: fx.root, encoding: 'utf8', input: `${changed}\n`,
    env: { ...process.env, GATE_ROOT: fx.root, GATE_ORACLE: 'rakia' },
  });
  fx.status = r.status ?? -1;
  fx.stdout = `${r.stdout}${r.stderr}`;
});

Then('the gate exits {int}', function (code: number) { assert.ok(fx); assert.equal(fx.status, code, fx.stdout); });

Then('the gate printed {string}', function (text: string) { assert.ok(fx); assert.ok(fx.stdout?.includes(text), `expected ${JSON.stringify(text)} in:\n${fx.stdout}`); });

Then('the gate printed {string} followed by {string}', function (a: string, b: string) {
  assert.ok(fx);
  const line = (fx.stdout ?? '').split('\n').find(l => l.includes(a));
  assert.ok(line && line.includes(b), `expected a line with ${JSON.stringify(a)} and ${JSON.stringify(b)} in:\n${fx.stdout}`);
});

Then('one pin-attestation observation was recorded with tier {string} and conclusion {string}', function (tier: string, conclusion: string) {
  assert.ok(fx);
  assert.ok(existsSync(fx.eprLog), 'the fake epr was never invoked');
  const lines = readFileSync(fx.eprLog, 'utf8').trim().split('\n');
  assert.equal(lines.length, 1, lines.join('\n'));
  assert.match(lines[0], /^flow note --kind observation --measure pin-attestation@1 --subject comp --value 1 --unit reads /);
  assert.ok(lines[0].includes(`--env tier=${tier}`) && lines[0].includes(`--env conclusion=${conclusion}`), lines[0]);
});

Then('the selected projects are {string}', function (expected: string) {
  assert.ok(fx);
  const names = (fx.stdout ?? '').split('\n').filter(l => l.startsWith('{')).map(l => JSON.parse(l).name as string);
  assert.deepEqual(names, expected.split(',').map(s => s.trim()), fx.stdout);
});

Then('project {string} was selected because of {string}', function (name: string, reason: string) {
  assert.ok(fx);
  const row = (fx.stdout ?? '').split('\n').filter(l => l.startsWith('{')).map(l => JSON.parse(l)).find(r => r.name === name);
  assert.ok(row, `${name} not selected:\n${fx.stdout}`);
  assert.ok((row.reasons as string[]).includes(reason), JSON.stringify(row.reasons));
});

After(function () {
  if (fx) rmSync(resolve(fx.root, '..'), { recursive: true, force: true });
  fx = undefined;
});
```

Note on the propagation scenario: `rakia affected` discovers manifests by walking the fixture root, so it finds both manifests; `comp` matches the gitlink path verbatim because the manifest lists it (the pin is the input).

- [ ] **Step 4: Run the scoped story**

Run: `cd /home/matthew/git/elohim/genesis/a2o && npx cucumber-js --config '' --require-module tsx --require steps/devflow/rakia-cli.guard.ts --require steps/devflow/pin-attestation.steps.ts features/devflow/pin-attestation.feature`
Expected: `5 scenarios (5 passed)`. If `rakia` is not on PATH the fifth reports skipped with the guard's line; on this host it is installed, so all five must pass.

Then the lint the pre-push runs: `cd /home/matthew/git/elohim && just gate genesis-a2o; echo EXIT=$?` — expected `EXIT=0`.

- [ ] **Step 5: Commit**

```bash
cd /home/matthew/git/elohim
git add genesis/a2o/features/devflow/pin-attestation.feature genesis/a2o/steps/devflow/pin-attestation.steps.ts genesis/a2o/steps/devflow/rakia-cli.guard.ts
git commit -m "a2o(devflow): @concern:pin-attestation — five scenarios over a scratch superproject

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 13: land commit 3 with the habit delta

**Files:**
- Modify: `genesis/orchestrator/.epr-meta/pin-attestation.habit.md` (append a DELTA line)
- Regenerate: `genesis/manifests/habits.yaml`

- [ ] **Step 1: Run the gates this landing owns, to ALL CLEAR**

```bash
cd /home/matthew/git/elohim
just gate orchestrator; echo EXIT=$?
just gate genesis-a2o; echo EXIT=$?
just gate rakia-validate; echo EXIT=$?
python3 .claude/scripts/habits-project.py --check; echo EXIT=$?
```
Expected: four `EXIT=0`.

- [ ] **Step 2: Append the delta and re-project**

Append to the habit atom body:

```markdown
DELTA 2026-09-23 (rung 3 landed in shadow mode; RED preserved): schema lifted and widened
(rakia <RAKIA_SHA_2 short>), brit/rakia/sophia declare attested gates with the pin as the step
input, consumer edges on cargo-build-storage and build-edge-image, gate-runner asks `rakia
affected` in shadow mode (GATE_ORACLE=shadow default) and dispatches attested projects to
gate-attest.mjs. Evidence: node --test gate-attest/gate-oracle/gate-runner EXIT=0; a2o
@concern:pin-attestation 5/5 on the default profile; `gate-runner --target brit` read
`Tests pass success` live. Not yet: the oracle flip (commit 4) and a real pin-moving push.
```

Run: `cd /home/matthew/git/elohim && python3 .claude/scripts/habits-project.py && git add genesis/orchestrator/.epr-meta/pin-attestation.habit.md genesis/manifests/habits.yaml && git commit -m "habit(pin-attestation): delta — rung 3 landed in shadow mode

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"`

- [ ] **Step 3: Push commits 1–3 (hook bypassed after ALL CLEAR)**

Run: `cd /home/matthew/git/elohim && HUSKY=0 git push origin dev`
Expected: the push lands; the orchestrator dispatches whatever the changed manifests select in CI (the edge manifest changed, so expect an `elohim-edge` run; it is a `depends` edit only).

---

### Task 14: flip the oracle after one real pin-moving push

Do this only after a push that moved a pin has printed an `[gate] oracle-diff:` line whose `+` set is exactly the intended depth-one consumers (for a sophia pin: `+elohim-app`; for a rakia pin: `+elohim-storage`; for a brit pin: nothing extra, since `build-edge-image` has no gate project). Record that line verbatim in the habit delta.

**Files:**
- Modify: `genesis/orchestrator/gate-runner.mjs` (`oracleMode` default)
- Modify: `genesis/orchestrator/gate-runner.test.mjs`
- Modify: `genesis/orchestrator/.epr-meta/pin-attestation.habit.md`, regenerate `genesis/manifests/habits.yaml`

- [ ] **Step 1: Write the failing test**

Append to `gate-runner.test.mjs`:

```js
  test('rakia is the default oracle once the shadow run has been read', () => {
    assert.equal(oracleMode({}), 'rakia');
    assert.equal(oracleMode({ GATE_ORACLE: 'shadow' }), 'shadow');
    assert.equal(oracleMode({ GATE_ORACLE: 'path' }), 'path');
  });
```
(import `oracleMode` from `./gate-runner.mjs`.)

- [ ] **Step 2: Run to verify it fails**

Run: `cd /home/matthew/git/elohim/genesis/orchestrator && node --test gate-runner.test.mjs`
Expected: FAIL — default is `shadow`.

- [ ] **Step 3: Flip the default**

In `oracleMode`, change the fallback from `'shadow'` to `'rakia'` and update the comment to: `// 'rakia' since <date of the shadow evidence>: one real pin-moving push printed a diff that was only the intended propagation (habit delta records the line). GATE_ORACLE=shadow|path remain available.`

- [ ] **Step 4: Run the suite and the gate**

Run: `cd /home/matthew/git/elohim && just gate orchestrator; echo EXIT=$?`
Expected: `EXIT=0`.

- [ ] **Step 5: Delta, re-project, commit, push**

Append to the habit atom:

```markdown
DELTA <date> (oracle flipped; RED preserved until two pin-moving pushes hold): the shadow push
<sha> printed `[gate] oracle-diff: +<projects>` — exactly the depth-one consumers — so
GATE_ORACLE defaults to rakia. The habit flips green when two further pin-moving pushes select
by the oracle with no override and every pin read is tier=witnessed.
```

```bash
cd /home/matthew/git/elohim
python3 .claude/scripts/habits-project.py
git add genesis/orchestrator/gate-runner.mjs genesis/orchestrator/gate-runner.test.mjs genesis/orchestrator/.epr-meta/pin-attestation.habit.md genesis/manifests/habits.yaml
git commit -m "gate(select): rakia is the oracle — the shadow push showed only the intended propagation

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
HUSKY=0 git push origin dev
```

---

## Self-review

**Spec coverage.** §4.1 schema → Tasks 1–3. §4.2 manifests → Tasks 4–6. §4.3 consumer edges, orchestrator inputs, storage cap → Task 7. §4.4 oracle, depth one, shadow, fallback, `GATE_ROOT` → Tasks 8–9, flip → Task 14. §4.5 attested gate, four outcomes, dirty worktree, not-a-gitlink → Task 10 (module boundary amended in the spec by Task 11). §4.6 measure → Tasks 10–11. §5 habit → Tasks 11, 13, 14. §6 proof layers → Tasks 1–2 (schema), 8–10 (unit), 12 (story). §8 landing order → the landing map. §9 open question 1 (sophia check name) is a manifest field only; question 2 (`gh` on CI) is honoured by `claimed`; question 3 (storage as a depth-one consumer) is by design.

**Placeholder scan.** `<RAKIA_SHA_1>`, `<RAKIA_SHA_2>`, `<BRIT_SHA>`, `<SOPHIA_SHA>`, `<date>`, `<sha>` are values the executor records at the named step, not missing content.

**Type consistency.** `projectsFromStale(manifests, stale, changedFiles)` (Task 9) is the name Task 9's runner code calls; `rakiaAffected` / `depthOne` / `resolveRakiaBin` (Task 8) match Task 9's imports; `runAttested(project, { root, env, spawn, log, runEpr })` (Task 10) matches the runner dispatch; `observationArgs(subject, attestation, verdict)` matches its test; the feature's step text matches the step definitions verbatim; the measure id `pin-attestation@1` is identical in gate-attest.mjs, measures.yaml, the habit atom, and the story.

**Review Focus.** (1) reads-the-pin → Task 10 `fakeGit`/`pinnedSha` tests; (2) latest run wins → Task 10 `readCheck` first test; (3) gitlink plus files beneath → Task 9 dedupe test; (4) attested consumer dispatch → Task 10's `never reaches run-local-gate.sh` test plus the `kind === 'attested'` branch placed before any cargo handling; (5) rakia present but failing → Task 8 `non-zero exit` test and Task 9 fallback test.
