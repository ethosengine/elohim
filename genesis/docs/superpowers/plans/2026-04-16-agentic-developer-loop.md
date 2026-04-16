# Agentic Developer Loop — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship v1 of the agentic developer loop — a first-class overnight developer that iterates a Jenkins pipeline toward a named Objective and produces a single markdown sprint-result artifact.

**Architecture:** Three layers. (1) Foundation: JSON schemas for Objective and Haiku output, safety taxonomy data, anti-pattern reference catalog. (2) Utilities: Node `.mjs` scripts under `genesis/agentic/` for palette pattern matching, permission generalization, and pre-shift readiness checks — tested with `node --test`. (3) Playbooks: markdown skill/command files under `.claude/skills/` and `.claude/commands/` that tell Opus how to orchestrate kickoff, iteration, and close. Code is thin; the intelligence lives in the playbook instructions and the top-tier model reading them.

**Tech Stack:** Node 20+ ESM (`.mjs`), built-in `node --test`, `ajv` + `ajv-formats` (JSON Schema validation, already root devDeps), `picomatch` (glob pattern matching, already root devDep), YAML-in-markdown frontmatter for Claude skills, no new runtime dependencies.

**Spec:** `genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md`

**P2P classification (per `.claude/skills/p2p-design-gate/SKILL.md`):** Category C — operational/tooling. All schemas in this plan (Objective YAML, Haiku output, journal, sprint result, settings.json entries) are local-only single-workstation state. No DHT entry types, no content addressing, no cross-peer sync. The eventual migration path — where measurement events graduate to `BuildAttestation` / `DeployAttestation` — is out of scope here and lives in brit Phase 2a + rakia's build-attestation-integration plan. The retrospective template's "Implications for brit" migration-bridge section is where today's operational events become tomorrow's protocol-data requirements.

---

## File Map

New files being created:

```
.claude/
  commands/
    shift.md                                     # /shift slash command (T13)
  skills/
    agentic-developer/
      SKILL.md                                   # main playbook (T12)
    generalize-permissions/
      SKILL.md                                   # /generalize-permissions skill (T11)
  schemas/
    objective.schema.json                        # Objective YAML validator (T2)
    haiku-output.schema.json                     # Haiku structured summary (T4)
  shifts/
    .gitkeep                                     # runtime dir (T1)

genesis/
  agentic/
    README.md                                    # orientation (T1)
    data/
      safety-taxonomy.json                       # command family classifications (T3)
      anti-patterns.json                         # pipeline anti-pattern catalog (T5)
    palette.mjs                                  # pattern-matching utility (T6)
    palette.test.mjs
    generalize.mjs                               # generalization algorithm (T7)
    generalize.test.mjs
    readiness.mjs                                # pre-shift check script (T8)
    readiness.test.mjs
  docs/
    retrospectives/
      TEMPLATE.md                                # retrospective stub (T10)
    shifts/
      JOURNAL-TEMPLATE.md                        # journal stanza reference (T9)
      SPRINT-RESULT-TEMPLATE.md                  # close-time output shape (T9)
```

Modified files:

- `.gitignore` — add `.claude/shifts/*` (except `.gitkeep`), preserve `.claude/settings.local.json` entry (T1)
- `package.json` — add `agentic:test` and `agentic:readiness` script entries (T8)

---

## Phase 1 — Foundation artifacts (schemas and data)

### Task 1: Repo scaffolding

**Files:**
- Create: `.claude/shifts/.gitkeep`
- Create: `genesis/agentic/README.md`
- Modify: `.gitignore`

- [ ] **Step 1: Create shifts directory with placeholder**

```bash
mkdir -p .claude/shifts
touch .claude/shifts/.gitkeep
```

- [ ] **Step 2: Add shift artifacts to `.gitignore`**

Locate the `# Claude Code` section near the top of `.gitignore` and append the shift-artifact exclusion:

```gitignore
# Claude Code
.claude/settings.local.json
.claude/hooks/.p2p-audit-cooldown.json
.mcp.json
.worktrees/

# Agentic developer shift artifacts (local only)
.claude/shifts/*
!.claude/shifts/.gitkeep
```

- [ ] **Step 3: Create `genesis/agentic/` with orientation**

```bash
mkdir -p genesis/agentic/data
```

Create `genesis/agentic/README.md`:

```markdown
# genesis/agentic

Utility scripts and reference data backing the agentic developer loop
(see `genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md`).

## Contents

- `palette.mjs` — pattern-match a bash command against `.claude/settings.json` allowlist entries.
- `generalize.mjs` — cluster near-duplicate allowlist entries into broader patterns under a safety taxonomy.
- `readiness.mjs` — pre-shift environment check (tokens, connections, measure command, git state, palette sanity).
- `data/safety-taxonomy.json` — command family classifications (broadly-safe / subcommand-scoped / never-wildcard).
- `data/anti-patterns.json` — reference catalog of pipeline output patterns that waste Haiku's effort.

## Running

```sh
# From repo root:
node --test genesis/agentic/*.test.mjs
node genesis/agentic/readiness.mjs --objective .claude/shifts/<shift-id>.objective.yaml
```

No `package.json` here — scripts use root-level dev dependencies (`ajv`, `picomatch`).
```

- [ ] **Step 4: Commit**

```bash
git add .gitignore .claude/shifts/.gitkeep genesis/agentic/README.md
git commit -m "chore(agentic): scaffold agentic-developer directories

Add .claude/shifts/ runtime directory, genesis/agentic/ for utility
code, and .gitignore entries so shift journals stay local."
```

---

### Task 2: Objective JSON schema

**Files:**
- Create: `.claude/schemas/objective.schema.json`
- Create: `genesis/agentic/objective-schema.test.mjs`
- Create: `.claude/shifts/sample.objective.yaml` *(temp test fixture, deleted at end of task)*

- [ ] **Step 1: Write failing schema validation test**

Create `genesis/agentic/objective-schema.test.mjs`:

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv from 'ajv';
import addFormats from 'ajv-formats';

const schema = JSON.parse(
  readFileSync('.claude/schemas/objective.schema.json', 'utf8'),
);
const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);
const validate = ajv.compile(schema);

test('valid binary objective passes', () => {
  const obj = {
    name: 'lift-edge-green',
    description: 'Make alpha-deploy stage pass',
    measure: { type: 'cmd', run: 'echo 1' },
    target: {
      predicate: '>=',
      value: 1,
      stability: { consecutive: 2, across_triggers: true },
    },
    baseline: { predicate: '>=', value: 0 },
    budget: { iterations: 10, wall_clock_min: 480 },
    scope: { paths: ['genesis/orchestrator/**'] },
  };
  const ok = validate(obj);
  assert.equal(ok, true, JSON.stringify(validate.errors));
});

test('valid progressive objective passes', () => {
  const obj = {
    name: 'lift-gherkin-rate',
    description: 'Raise pass rate from 5% to 20%',
    measure: { type: 'cmd', run: 'echo 0.2' },
    target: {
      predicate: '>=',
      value: 0.2,
      stability: { consecutive: 2, across_triggers: true },
    },
    baseline: { predicate: '>=', value: 0.05 },
    budget: { iterations: 10, wall_clock_min: 480 },
    scope: { paths: ['genesis/a2o/**'] },
  };
  assert.equal(validate(obj), true, JSON.stringify(validate.errors));
});

test('rejects missing measure', () => {
  const obj = {
    name: 'bad',
    description: 'no measure',
    target: { predicate: '>=', value: 1, stability: { consecutive: 2, across_triggers: true } },
    baseline: { predicate: '>=', value: 0 },
    budget: { iterations: 1, wall_clock_min: 1 },
    scope: { paths: ['**'] },
  };
  assert.equal(validate(obj), false);
});

test('rejects unknown predicate', () => {
  const obj = {
    name: 'bad',
    description: 'unknown predicate',
    measure: { type: 'cmd', run: 'echo' },
    target: { predicate: '~~', value: 1, stability: { consecutive: 2, across_triggers: true } },
    baseline: { predicate: '>=', value: 0 },
    budget: { iterations: 1, wall_clock_min: 1 },
    scope: { paths: ['**'] },
  };
  assert.equal(validate(obj), false);
});

test('rejects missing stability', () => {
  const obj = {
    name: 'bad',
    description: 'stability missing',
    measure: { type: 'cmd', run: 'echo' },
    target: { predicate: '>=', value: 1 },
    baseline: { predicate: '>=', value: 0 },
    budget: { iterations: 1, wall_clock_min: 1 },
    scope: { paths: ['**'] },
  };
  assert.equal(validate(obj), false);
});
```

- [ ] **Step 2: Run test; confirm it fails (schema file doesn't exist)**

```bash
node --test genesis/agentic/objective-schema.test.mjs
```

Expected: ENOENT or SyntaxError on reading `.claude/schemas/objective.schema.json`.

- [ ] **Step 3: Create the Objective schema**

```bash
mkdir -p .claude/schemas
```

Create `.claude/schemas/objective.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/agentic/objective.schema.json",
  "title": "Agentic Developer Shift Objective",
  "description": "Measurable, bounded, scoped goal that an agentic developer shift iterates toward.",
  "type": "object",
  "additionalProperties": false,
  "required": ["name", "description", "measure", "target", "baseline", "budget", "scope"],
  "properties": {
    "name": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]{2,60}$",
      "description": "Slug-case identifier; used in shift id."
    },
    "description": {
      "type": "string",
      "minLength": 10,
      "maxLength": 1000
    },
    "measure": {
      "type": "object",
      "additionalProperties": false,
      "required": ["type", "run"],
      "properties": {
        "type": { "const": "cmd" },
        "run": {
          "type": "string",
          "minLength": 1,
          "description": "Shell command that returns a single parseable number on stdout."
        }
      }
    },
    "target": {
      "type": "object",
      "additionalProperties": false,
      "required": ["predicate", "value", "stability"],
      "properties": {
        "predicate": { "enum": [">=", "<=", "==", ">", "<"] },
        "value": { "type": "number" },
        "stability": {
          "type": "object",
          "additionalProperties": false,
          "required": ["consecutive", "across_triggers"],
          "properties": {
            "consecutive": { "type": "integer", "minimum": 1, "default": 2 },
            "across_triggers": { "type": "boolean", "default": true }
          }
        }
      }
    },
    "baseline": {
      "type": "object",
      "additionalProperties": false,
      "required": ["predicate", "value"],
      "properties": {
        "predicate": { "enum": [">=", "<=", "==", ">", "<"] },
        "value": { "type": "number" }
      },
      "description": "Regression floor; measurement must not drop below this during the shift."
    },
    "budget": {
      "type": "object",
      "additionalProperties": false,
      "required": ["iterations", "wall_clock_min"],
      "properties": {
        "iterations": { "type": "integer", "minimum": 1, "maximum": 100 },
        "wall_clock_min": { "type": "integer", "minimum": 1, "maximum": 1440 }
      }
    },
    "scope": {
      "type": "object",
      "additionalProperties": false,
      "required": ["paths"],
      "properties": {
        "paths": {
          "type": "array",
          "minItems": 1,
          "items": { "type": "string", "minLength": 1 },
          "description": "Glob patterns — Opus may only edit files matching these."
        }
      }
    }
  }
}
```

- [ ] **Step 4: Run test; confirm it passes**

```bash
node --test genesis/agentic/objective-schema.test.mjs
```

Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add .claude/schemas/objective.schema.json genesis/agentic/objective-schema.test.mjs
git commit -m "feat(agentic): objective JSON schema with validation tests

Defines the shape of shift Objectives (binary + progressive metric +
baseline floor + stability gate) per the v1 spec."
```

---

### Task 3: Safety taxonomy data

**Files:**
- Create: `genesis/agentic/data/safety-taxonomy.json`
- Create: `genesis/agentic/safety-taxonomy.test.mjs`

- [ ] **Step 1: Write failing taxonomy structure test**

Create `genesis/agentic/safety-taxonomy.test.mjs`:

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const data = JSON.parse(
  readFileSync('genesis/agentic/data/safety-taxonomy.json', 'utf8'),
);

test('has three tiers', () => {
  assert.ok(Array.isArray(data.broadly_safe), 'broadly_safe is array');
  assert.ok(Array.isArray(data.subcommand_scoped), 'subcommand_scoped is array');
  assert.ok(Array.isArray(data.never_wildcard), 'never_wildcard is array');
});

test('broadly_safe entries are command names', () => {
  for (const entry of data.broadly_safe) {
    assert.equal(typeof entry, 'string');
    assert.match(entry, /^[a-z][a-z0-9-]*$/, `${entry} is a command name`);
  }
});

test('subcommand_scoped entries have safe/prompt lists', () => {
  for (const entry of data.subcommand_scoped) {
    assert.equal(typeof entry.command, 'string');
    assert.ok(Array.isArray(entry.safe_subcommands));
    assert.ok(Array.isArray(entry.prompt_subcommands));
  }
});

test('never_wildcard has rationale per entry', () => {
  for (const entry of data.never_wildcard) {
    assert.equal(typeof entry.command, 'string');
    assert.equal(typeof entry.reason, 'string');
    assert.ok(entry.reason.length > 10);
  }
});

test('required commands present', () => {
  // Spec-mandated entries
  assert.ok(data.broadly_safe.includes('cargo'), 'cargo broadly safe');
  assert.ok(data.broadly_safe.includes('pnpm'), 'pnpm broadly safe');
  assert.ok(data.broadly_safe.includes('vitest'), 'vitest broadly safe');
  assert.ok(
    data.subcommand_scoped.find((e) => e.command === 'git'),
    'git is subcommand-scoped',
  );
  assert.ok(
    data.never_wildcard.find((e) => e.command === 'rm'),
    'rm is never wildcard',
  );
});
```

- [ ] **Step 2: Run test; confirm fail (file missing)**

```bash
node --test genesis/agentic/safety-taxonomy.test.mjs
```

Expected: ENOENT on safety-taxonomy.json.

- [ ] **Step 3: Create the taxonomy file**

Create `genesis/agentic/data/safety-taxonomy.json`:

```json
{
  "$description": "Command safety classifications for permission allowlist generalization. Source of truth for the /generalize-permissions skill.",
  "$spec_ref": "genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md#palette-safety-taxonomy",

  "broadly_safe": [
    "cargo",
    "pnpm",
    "npm",
    "npx",
    "pnpx",
    "yarn",
    "vitest",
    "jest",
    "pytest",
    "eslint",
    "stylelint",
    "prettier",
    "tsc",
    "rustup",
    "rustc",
    "node",
    "jq"
  ],

  "subcommand_scoped": [
    {
      "command": "git",
      "safe_subcommands": [
        "add",
        "commit",
        "diff",
        "status",
        "log",
        "show",
        "fetch",
        "pull",
        "rev-parse",
        "describe",
        "blame",
        "ls-files",
        "stash",
        "worktree"
      ],
      "prompt_subcommands": [
        "push",
        "reset",
        "branch",
        "checkout",
        "rebase",
        "merge",
        "cherry-pick",
        "tag",
        "remote"
      ]
    },
    {
      "command": "kubectl",
      "safe_subcommands": [
        "get",
        "describe",
        "logs",
        "config",
        "version",
        "cluster-info",
        "explain"
      ],
      "prompt_subcommands": [
        "apply",
        "delete",
        "patch",
        "rollout",
        "scale",
        "exec",
        "port-forward",
        "cp"
      ]
    },
    {
      "command": "docker",
      "safe_subcommands": ["ps", "logs", "inspect", "images", "version", "info"],
      "prompt_subcommands": ["rm", "rmi", "run", "exec", "build", "push", "pull"]
    },
    {
      "command": "gh",
      "safe_subcommands": [
        "pr list",
        "pr view",
        "issue list",
        "issue view",
        "run list",
        "run view",
        "auth status",
        "api"
      ],
      "prompt_subcommands": [
        "pr create",
        "pr merge",
        "pr close",
        "issue create",
        "issue close",
        "release create",
        "delete"
      ]
    }
  ],

  "never_wildcard": [
    {
      "command": "rm",
      "reason": "Destructive; path wildcarding is too dangerous for auto-approval."
    },
    {
      "command": "sudo",
      "reason": "Privilege elevation must always be deliberate and explicit."
    },
    {
      "command": "curl",
      "reason": "Can exfiltrate data to arbitrary endpoints; parameters too variable."
    },
    {
      "command": "wget",
      "reason": "Same exfiltration concern as curl."
    },
    {
      "command": "ssh",
      "reason": "Remote execution; must be approved per-target."
    },
    {
      "command": "scp",
      "reason": "Data exfiltration potential."
    },
    {
      "command": "aws",
      "reason": "Cloud credentials + destructive subcommands; per-command review required."
    },
    {
      "command": "gcloud",
      "reason": "Same cloud-credential concern as aws."
    }
  ]
}
```

- [ ] **Step 4: Run test; confirm pass**

```bash
node --test genesis/agentic/safety-taxonomy.test.mjs
```

Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/agentic/data/safety-taxonomy.json genesis/agentic/safety-taxonomy.test.mjs
git commit -m "feat(agentic): permission allowlist safety taxonomy

Three-tier classification (broadly-safe / subcommand-scoped /
never-wildcard) used by /generalize-permissions to propose pattern
collapses without introducing unsafe broad patterns."
```

---

### Task 4: Haiku output JSON schema

**Files:**
- Create: `.claude/schemas/haiku-output.schema.json`
- Create: `genesis/agentic/haiku-output-schema.test.mjs`

- [ ] **Step 1: Write failing Haiku output validation test**

Create `genesis/agentic/haiku-output-schema.test.mjs`:

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv from 'ajv';

const schema = JSON.parse(
  readFileSync('.claude/schemas/haiku-output.schema.json', 'utf8'),
);
const validate = new Ajv({ allErrors: true, strict: false }).compile(schema);

test('valid Haiku finding passes', () => {
  const out = {
    iteration: 4,
    measurement: { value: 0.18, delta: 0.03, baseline: 0.05, target: 0.2 },
    context: {
      build_id: 1247,
      status: 'failed',
      first_failing_stage: 'alpha-deploy',
    },
    primary_failure: {
      error_class: 'StatefulSet field forbidden',
      evidence: 'forbidden: updates to .spec.volumeClaimTemplates are forbidden',
      files_mentioned: ['genesis/orchestrator/manifests/doorway/alpha.yaml'],
    },
    observed_anti_patterns: [
      {
        pattern: 'full kubectl describe dump',
        evidence: '3200 lines; only 8 matter',
      },
    ],
    confidence: 'medium',
  };
  assert.equal(validate(out), true, JSON.stringify(validate.errors));
});

test('rejects confidence outside enum', () => {
  const out = {
    iteration: 1,
    measurement: { value: 0, delta: 0, baseline: 0, target: 1 },
    context: { build_id: 1, status: 'passed', first_failing_stage: null },
    primary_failure: null,
    observed_anti_patterns: [],
    confidence: 'certain',
  };
  assert.equal(validate(out), false);
});

test('primary_failure nullable when status passed', () => {
  const out = {
    iteration: 1,
    measurement: { value: 1, delta: 0, baseline: 0, target: 1 },
    context: { build_id: 1, status: 'passed', first_failing_stage: null },
    primary_failure: null,
    observed_anti_patterns: [],
    confidence: 'high',
  };
  assert.equal(validate(out), true, JSON.stringify(validate.errors));
});

test('observed_anti_patterns may be empty array', () => {
  const out = {
    iteration: 1,
    measurement: { value: 0, delta: 0, baseline: 0, target: 1 },
    context: { build_id: 1, status: 'failed', first_failing_stage: 'x' },
    primary_failure: {
      error_class: 'x',
      evidence: 'x',
      files_mentioned: [],
    },
    observed_anti_patterns: [],
    confidence: 'high',
  };
  assert.equal(validate(out), true, JSON.stringify(validate.errors));
});
```

- [ ] **Step 2: Run test; confirm fails**

```bash
node --test genesis/agentic/haiku-output-schema.test.mjs
```

Expected: ENOENT on haiku-output.schema.json.

- [ ] **Step 3: Create Haiku output schema**

Create `.claude/schemas/haiku-output.schema.json`:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/agentic/haiku-output.schema.json",
  "title": "Haiku Observation Output",
  "description": "Bounded structured summary that Haiku returns to Opus per iteration.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "iteration",
    "measurement",
    "context",
    "primary_failure",
    "observed_anti_patterns",
    "confidence"
  ],
  "properties": {
    "iteration": { "type": "integer", "minimum": 1 },

    "measurement": {
      "type": "object",
      "additionalProperties": false,
      "required": ["value", "delta", "baseline", "target"],
      "properties": {
        "value": { "type": "number" },
        "delta": { "type": "number", "description": "Signed delta vs. previous iteration." },
        "baseline": { "type": "number" },
        "target": { "type": "number" }
      }
    },

    "context": {
      "type": "object",
      "additionalProperties": false,
      "required": ["build_id", "status", "first_failing_stage"],
      "properties": {
        "build_id": { "type": ["integer", "null"] },
        "status": { "enum": ["passed", "failed", "running", "unknown"] },
        "first_failing_stage": { "type": ["string", "null"] }
      }
    },

    "primary_failure": {
      "oneOf": [
        { "type": "null" },
        {
          "type": "object",
          "additionalProperties": false,
          "required": ["error_class", "evidence", "files_mentioned"],
          "properties": {
            "error_class": { "type": "string", "minLength": 1 },
            "evidence": { "type": "string", "minLength": 1 },
            "files_mentioned": {
              "type": "array",
              "items": { "type": "string" }
            }
          }
        }
      ]
    },

    "observed_anti_patterns": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["pattern", "evidence"],
        "properties": {
          "pattern": { "type": "string", "minLength": 1 },
          "evidence": { "type": "string", "minLength": 1 }
        }
      }
    },

    "confidence": { "enum": ["low", "medium", "high"] }
  }
}
```

- [ ] **Step 4: Run test; confirm pass**

```bash
node --test genesis/agentic/haiku-output-schema.test.mjs
```

Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add .claude/schemas/haiku-output.schema.json genesis/agentic/haiku-output-schema.test.mjs
git commit -m "feat(agentic): haiku output JSON schema

Per-iteration structured summary contract — measurement deltas,
primary failure, observed anti-patterns, self-reported confidence."
```

---

### Task 5: Anti-pattern reference catalog

**Files:**
- Create: `genesis/agentic/data/anti-patterns.json`
- Create: `genesis/agentic/anti-patterns.test.mjs`

- [ ] **Step 1: Write failing catalog structure test**

Create `genesis/agentic/anti-patterns.test.mjs`:

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const data = JSON.parse(
  readFileSync('genesis/agentic/data/anti-patterns.json', 'utf8'),
);

test('catalog is an array', () => {
  assert.ok(Array.isArray(data.patterns));
  assert.ok(data.patterns.length >= 5, 'at least 5 seed patterns');
});

test('each pattern has required fields', () => {
  for (const p of data.patterns) {
    assert.equal(typeof p.id, 'string');
    assert.match(p.id, /^AP-[0-9]{3}$/, `${p.id} matches AP-NNN`);
    assert.equal(typeof p.name, 'string');
    assert.equal(typeof p.description, 'string');
    assert.ok(Array.isArray(p.detection_hints));
    assert.ok(p.detection_hints.length >= 1);
    assert.equal(typeof p.attestation_maps_to, 'string');
  }
});

test('ids unique', () => {
  const ids = data.patterns.map((p) => p.id);
  assert.equal(new Set(ids).size, ids.length, 'pattern ids must be unique');
});
```

- [ ] **Step 2: Run test; confirm fails**

```bash
node --test genesis/agentic/anti-patterns.test.mjs
```

Expected: ENOENT.

- [ ] **Step 3: Create the catalog**

Create `genesis/agentic/data/anti-patterns.json`:

```json
{
  "$description": "Reference catalog of pipeline output anti-patterns that waste Haiku's effort. Surfaced in Haiku summaries and aggregated into sprint results for between-sprint cleanup.",
  "$spec_ref": "genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md#pre-sprint-pipeline-readiness-out-of-loop-passive",

  "patterns": [
    {
      "id": "AP-001",
      "name": "Full log dump on failure",
      "description": "Stage emits entire kubectl/docker describe output or raw tool logs instead of an extracted error summary.",
      "detection_hints": [
        "stage output > 500 lines",
        "error message buried in the last 10% of output",
        "no explicit 'ERROR' or 'FAIL' prefix near the actual failure"
      ],
      "attestation_maps_to": "BuildAttestation.failure_summary field (brit Phase 2a+)"
    },
    {
      "id": "AP-002",
      "name": "Repetitive progress listing",
      "description": "Thousands of '✓ test-name' lines with no aggregation; takes tokens to reduce to pass/fail counts.",
      "detection_hints": [
        "repeated prefix like '✓ ', 'PASS ', 'ok ' over > 100 lines",
        "no trailing summary line with counts"
      ],
      "attestation_maps_to": "BuildAttestation.test_summary counts field"
    },
    {
      "id": "AP-003",
      "name": "Pre-emptive configuration noise",
      "description": "Stage echoes environment vars, tool versions, directory listings before any real work.",
      "detection_hints": [
        "> 50 lines before the first real command output",
        "lines matching ENV=, VERSION=, total usage, du -h, etc."
      ],
      "attestation_maps_to": "BuildAttestation.environment snapshot (reference, not log-embedded)"
    },
    {
      "id": "AP-004",
      "name": "Missing stack trace extraction",
      "description": "On exception, full thread-dump emitted instead of a structured stack-trace block keyed to the failure.",
      "detection_hints": [
        "no 'Caused by' or 'at <file>:<line>' structure in summary position",
        "stack text appears multiple times across concurrent threads"
      ],
      "attestation_maps_to": "BuildAttestation.failure_summary.stack_trace"
    },
    {
      "id": "AP-005",
      "name": "Unparseable stage boundaries",
      "description": "Stage start/end markers are free-text or inconsistent across the pipeline; Haiku cannot reliably segment the log.",
      "detection_hints": [
        "no consistent '=== Stage X ===' or '[stage:X]' marker",
        "stage names contain spaces or vary between runs"
      ],
      "attestation_maps_to": "BuildAttestation.stages[].name + .log_offset fields"
    },
    {
      "id": "AP-006",
      "name": "No machine-readable overall result",
      "description": "Pipeline produces no JSON or structured artifact describing overall pass/fail and per-stage outcome — everything must be scraped from the log.",
      "detection_hints": [
        "no `*-result.json`, `digest.json`, or similar artifact archived",
        "pipeline result only accessible via log grep"
      ],
      "attestation_maps_to": "BuildAttestation document as a whole replaces this"
    },
    {
      "id": "AP-007",
      "name": "Duplicate-error cascade",
      "description": "One root cause triggers many downstream errors all logged verbosely; Haiku must deduplicate.",
      "detection_hints": [
        "same error_class appears N+ times",
        "errors share a common timestamp window"
      ],
      "attestation_maps_to": "BuildAttestation.failure_summary.root_cause_id + .derived_errors[]"
    }
  ]
}
```

- [ ] **Step 4: Run test; confirm pass**

```bash
node --test genesis/agentic/anti-patterns.test.mjs
```

Expected: all 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/agentic/data/anti-patterns.json genesis/agentic/anti-patterns.test.mjs
git commit -m "feat(agentic): pipeline anti-pattern reference catalog

Seven seed patterns Haiku flags when encountered. Each entry maps to
the brit attestation field that would make the pattern structurally
impossible, feeding the migration retrospective."
```

---

## Phase 2 — Utility code

### Task 6: Palette pattern matcher

**Files:**
- Create: `genesis/agentic/palette.mjs`
- Create: `genesis/agentic/palette.test.mjs`

- [ ] **Step 1: Write failing palette matcher tests**

Create `genesis/agentic/palette.test.mjs`:

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { matchesPalette, loadPalette } from './palette.mjs';

const palette = [
  'Bash(pnpm run test)',
  'Bash(pnpm run test:*)',
  'Bash(RUSTFLAGS="" cargo *)',
  'Bash(git add :*)',
  'Bash(git commit -m :*)',
  'Bash(mcp__jenkins__*)',
];

test('exact match hits', () => {
  assert.equal(matchesPalette('pnpm run test', palette), true);
});

test('suffix wildcard hits', () => {
  assert.equal(matchesPalette('pnpm run test:unit', palette), true);
  assert.equal(matchesPalette('pnpm run test:e2e', palette), true);
});

test('RUSTFLAGS cargo wildcard hits all cargo subcommands', () => {
  assert.equal(
    matchesPalette('RUSTFLAGS="" cargo build', palette),
    true,
  );
  assert.equal(
    matchesPalette('RUSTFLAGS="" cargo test --workspace', palette),
    true,
  );
});

test('unrelated command misses', () => {
  assert.equal(matchesPalette('kubectl get pods', palette), false);
});

test('similar-but-different prefix misses', () => {
  assert.equal(matchesPalette('pnpm exec prettier', palette), false);
});

test('mcp tool prefix matches family', () => {
  assert.equal(matchesPalette('mcp__jenkins__getBuild', palette), true);
  assert.equal(matchesPalette('mcp__jenkins__triggerBuild', palette), true);
  assert.equal(matchesPalette('mcp__sonarqube__whatever', palette), false);
});

test('loadPalette reads settings files and extracts Bash entries', async () => {
  const { readFileSync, writeFileSync, mkdirSync, rmSync } = await import('node:fs');
  const { join } = await import('node:path');
  const { tmpdir } = await import('node:os');
  const dir = join(tmpdir(), `palette-test-${Date.now()}`);
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, 'settings.json'),
    JSON.stringify({ permissions: { allow: ['Bash(pnpm *)', 'Bash(git status)'] } }),
  );
  writeFileSync(
    join(dir, 'settings.local.json'),
    JSON.stringify({ permissions: { allow: ['Bash(cargo *)'] } }),
  );
  const loaded = loadPalette({
    durablePath: join(dir, 'settings.json'),
    localPath: join(dir, 'settings.local.json'),
  });
  assert.deepEqual(
    loaded.sort(),
    ['Bash(cargo *)', 'Bash(git status)', 'Bash(pnpm *)'],
  );
  rmSync(dir, { recursive: true, force: true });
});

test('loadPalette tolerates missing local file', async () => {
  const { readFileSync, writeFileSync, mkdirSync, rmSync } = await import('node:fs');
  const { join } = await import('node:path');
  const { tmpdir } = await import('node:os');
  const dir = join(tmpdir(), `palette-test-missing-${Date.now()}`);
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, 'settings.json'),
    JSON.stringify({ permissions: { allow: ['Bash(ls)'] } }),
  );
  const loaded = loadPalette({
    durablePath: join(dir, 'settings.json'),
    localPath: join(dir, 'does-not-exist.json'),
  });
  assert.deepEqual(loaded, ['Bash(ls)']);
  rmSync(dir, { recursive: true, force: true });
});
```

- [ ] **Step 2: Run; confirm fails**

```bash
node --test genesis/agentic/palette.test.mjs
```

Expected: module not found.

- [ ] **Step 3: Implement the matcher**

Create `genesis/agentic/palette.mjs`:

```javascript
import { readFileSync, existsSync } from 'node:fs';
import picomatch from 'picomatch';

const BASH_PATTERN = /^Bash\((.*)\)$/;

function extractBashPattern(entry) {
  const m = entry.match(BASH_PATTERN);
  return m ? m[1] : null;
}

function toGlob(palettePattern) {
  // Claude-style patterns use `*` as a wildcard, `:*` as "any trailing args".
  // Convert to picomatch glob: both become `*` spanning any chars.
  return palettePattern.replaceAll(':*', '*');
}

export function matchesPalette(command, paletteEntries) {
  const trimmed = command.trim();
  for (const entry of paletteEntries) {
    // Accept both Bash(...) and bare MCP tool names (mcp__foo__*).
    const bashBody = extractBashPattern(entry);
    const candidate = bashBody ?? entry;
    const glob = toGlob(candidate);
    if (picomatch.isMatch(trimmed, glob, { dot: true })) return true;
  }
  return false;
}

export function loadPalette({ durablePath, localPath }) {
  const out = [];
  for (const path of [durablePath, localPath]) {
    if (!existsSync(path)) continue;
    const contents = JSON.parse(readFileSync(path, 'utf8'));
    const allow = contents?.permissions?.allow ?? [];
    for (const entry of allow) if (typeof entry === 'string') out.push(entry);
  }
  return out;
}
```

- [ ] **Step 4: Run test; confirm pass**

```bash
node --test genesis/agentic/palette.test.mjs
```

Expected: all 8 tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/agentic/palette.mjs genesis/agentic/palette.test.mjs
git commit -m "feat(agentic): palette pattern matcher utility

Reads .claude/settings.json + settings.local.json allow lists and
pattern-matches candidate bash/mcp commands against them. Used by
Opus at kickoff (sanity check), at every iteration (per-command gate),
and by Sonnet (pre-check before dispatched sub-tasks)."
```

---

### Task 7: Generalization algorithm

**Files:**
- Create: `genesis/agentic/generalize.mjs`
- Create: `genesis/agentic/generalize.test.mjs`

- [ ] **Step 1: Write failing generalization tests**

Create `genesis/agentic/generalize.test.mjs`:

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { clusterAndPropose } from './generalize.mjs';
import { readFileSync } from 'node:fs';

const taxonomy = JSON.parse(
  readFileSync('genesis/agentic/data/safety-taxonomy.json', 'utf8'),
);

test('collapses RUSTFLAGS cargo variants into one pattern', () => {
  const entries = [
    'Bash(RUSTFLAGS="" cargo check)',
    'Bash(RUSTFLAGS="" cargo build)',
    'Bash(RUSTFLAGS="" cargo test --workspace)',
    'Bash(RUSTFLAGS="" cargo clippy -p brit-epr)',
    'Bash(RUSTFLAGS="" cargo clippy -p brit-verify)',
  ];
  const proposals = clusterAndPropose(entries, taxonomy);
  // Should propose one pattern covering all 5.
  const cargoCluster = proposals.find((p) =>
    p.proposed.includes('RUSTFLAGS') && p.proposed.includes('cargo'),
  );
  assert.ok(cargoCluster, 'cargo cluster produced');
  assert.equal(cargoCluster.absorbs.length, 5);
  assert.equal(cargoCluster.safety, 'broadly_safe');
});

test('does not generalize across never-wildcard commands', () => {
  const entries = [
    'Bash(rm tmp/a.log)',
    'Bash(rm tmp/b.log)',
    'Bash(rm tmp/c.log)',
  ];
  const proposals = clusterAndPropose(entries, taxonomy);
  // rm is never-wildcard — no generalization should be proposed.
  const rmCluster = proposals.find((p) => p.proposed.startsWith('Bash(rm'));
  assert.equal(rmCluster, undefined, 'rm not generalized');
});

test('respects subcommand-scope boundaries for git', () => {
  const entries = [
    'Bash(git add .claude/shifts/x)',
    'Bash(git add .claude/shifts/y)',
    'Bash(git add genesis/a2o/foo.feature)',
    'Bash(git push origin dev)',
    'Bash(git push origin main)',
  ];
  const proposals = clusterAndPropose(entries, taxonomy);
  // git add is safe → one generalized pattern proposed.
  const addCluster = proposals.find(
    (p) => p.proposed.includes('git add') && p.proposed.endsWith(' *)'),
  );
  assert.ok(addCluster, 'git add generalized');
  // git push is prompt-only → should NOT be auto-generalized at this tier.
  const pushCluster = proposals.find(
    (p) => p.proposed.includes('git push') && p.proposed.endsWith(' *)'),
  );
  assert.equal(pushCluster, undefined, 'git push not auto-generalized');
});

test('singletons pass through unchanged (no collapse opportunity)', () => {
  const entries = ['Bash(echo hello)'];
  const proposals = clusterAndPropose(entries, taxonomy);
  // Singletons produce no generalization proposal.
  assert.equal(proposals.length, 0);
});

test('proposal includes absorbed entries for user review', () => {
  const entries = [
    'Bash(pnpm run test)',
    'Bash(pnpm run test:unit)',
    'Bash(pnpm run test:e2e)',
  ];
  const proposals = clusterAndPropose(entries, taxonomy);
  const pnpmCluster = proposals.find((p) => p.proposed.includes('pnpm'));
  assert.ok(pnpmCluster);
  assert.ok(pnpmCluster.absorbs.includes('Bash(pnpm run test)'));
  assert.ok(pnpmCluster.absorbs.includes('Bash(pnpm run test:unit)'));
});
```

- [ ] **Step 2: Run; confirm fails**

```bash
node --test genesis/agentic/generalize.test.mjs
```

Expected: module not found.

- [ ] **Step 3: Implement the generalizer**

Create `genesis/agentic/generalize.mjs`:

```javascript
/**
 * Cluster near-duplicate palette entries and propose broader patterns
 * under the safety taxonomy. Pure function: returns proposals, never
 * writes to disk. Caller (skill invocation) is responsible for applying
 * approved proposals to settings.json.
 */

const BASH_PATTERN = /^Bash\((.*)\)$/;

function parseEntry(entry) {
  const m = entry.match(BASH_PATTERN);
  if (!m) return null;
  const body = m[1];
  const tokens = body.trim().split(/\s+/);
  // Strip leading VAR=value env assignments; keep the first real command.
  let i = 0;
  while (i < tokens.length && /^[A-Z_][A-Z0-9_]*=/.test(tokens[i])) i++;
  const cmd = tokens[i];
  if (!cmd) return null;
  const envPrefix = tokens.slice(0, i).join(' ');
  const subcommand = tokens[i + 1] ?? null;
  return { entry, body, cmd, subcommand, envPrefix };
}

function classifyCommand(cmd, taxonomy) {
  if (taxonomy.broadly_safe.includes(cmd)) return { tier: 'broadly_safe' };
  const sub = taxonomy.subcommand_scoped.find((e) => e.command === cmd);
  if (sub) return { tier: 'subcommand_scoped', rule: sub };
  if (taxonomy.never_wildcard.find((e) => e.command === cmd)) {
    return { tier: 'never_wildcard' };
  }
  return { tier: 'unknown' };
}

function clusterKey(parsed, classification) {
  const { cmd, subcommand, envPrefix } = parsed;
  const envKey = envPrefix || '';
  if (classification.tier === 'broadly_safe') {
    // All subcommands of a broadly-safe cmd can share one cluster per env prefix.
    return `${envKey}|${cmd}|*`;
  }
  if (classification.tier === 'subcommand_scoped') {
    const safe = classification.rule.safe_subcommands;
    // Only cluster within the safe-subcommand set. The cluster key pins the
    // (cmd, subcommand) pair — generalize args under it with `*`.
    if (!subcommand) return null;
    // Some "subcommands" are multi-token (e.g. 'pr list'); pick the longest prefix match.
    const match = [...safe]
      .sort((a, b) => b.length - a.length)
      .find((s) => {
        const parts = s.split(' ');
        const body = parsed.body
          .split(/\s+/)
          .slice(envKey ? envKey.split(/\s+/).length : 0);
        return parts.every((p, idx) => body[idx + 1] === p || body[idx] === p);
      });
    if (!match) return null;
    return `${envKey}|${cmd}|${match}|*`;
  }
  return null; // never_wildcard + unknown produce no cluster
}

function proposedPattern(clusterKey, parsed, classification) {
  const [envKey, cmd, rest] = clusterKey.split('|');
  const envStr = envKey ? `${envKey} ` : '';
  if (classification.tier === 'broadly_safe') {
    return `Bash(${envStr}${cmd} *)`;
  }
  // subcommand_scoped
  const subcmd = clusterKey.split('|').slice(2, -1).join(' ');
  return `Bash(${envStr}${cmd} ${subcmd} *)`;
}

export function clusterAndPropose(entries, taxonomy) {
  const clusters = new Map();

  for (const entry of entries) {
    const parsed = parseEntry(entry);
    if (!parsed) continue;
    const classification = classifyCommand(parsed.cmd, taxonomy);
    if (classification.tier === 'never_wildcard') continue;
    if (classification.tier === 'unknown') continue;
    const key = clusterKey(parsed, classification);
    if (!key) continue;
    if (!clusters.has(key)) {
      clusters.set(key, { key, classification, parsed, absorbs: [] });
    }
    clusters.get(key).absorbs.push(entry);
  }

  const proposals = [];
  for (const cluster of clusters.values()) {
    if (cluster.absorbs.length < 2) continue; // require at least 2 to justify
    proposals.push({
      proposed: proposedPattern(cluster.key, cluster.parsed, cluster.classification),
      absorbs: cluster.absorbs,
      safety: cluster.classification.tier,
    });
  }
  return proposals;
}
```

- [ ] **Step 4: Run test; confirm pass**

```bash
node --test genesis/agentic/generalize.test.mjs
```

Expected: all 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/agentic/generalize.mjs genesis/agentic/generalize.test.mjs
git commit -m "feat(agentic): permission allowlist generalization algorithm

Cluster near-duplicate palette entries under the safety taxonomy and
propose broader patterns (e.g. ten RUSTFLAGS cargo variants → one
'Bash(RUSTFLAGS=\"\" cargo *)' entry). Never-wildcard commands are
excluded; subcommand-scoped commands generalize only within their
safe-subcommand set."
```

---

### Task 8: Pre-shift readiness check script

**Files:**
- Create: `genesis/agentic/readiness.mjs`
- Create: `genesis/agentic/readiness.test.mjs`
- Modify: `package.json`

- [ ] **Step 1: Write failing readiness check tests**

Create `genesis/agentic/readiness.test.mjs`:

```javascript
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkGitClean, checkMeasureRuns, checkPaletteGaps } from './readiness.mjs';

test('checkGitClean passes on clean tree', async () => {
  // Uses `git status --porcelain`; in a clean worktree produces empty output.
  // We run against an isolated fixture dir to avoid false positives.
  const { execSync } = await import('node:child_process');
  const { mkdtempSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const dir = mkdtempSync(join(tmpdir(), 'git-clean-test-'));
  execSync('git init -q', { cwd: dir });
  execSync('git commit --allow-empty -m init -q', {
    cwd: dir,
    env: { ...process.env, GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@t', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@t' },
  });
  const result = await checkGitClean({ cwd: dir });
  assert.equal(result.ok, true, result.reason);
});

test('checkGitClean fails with untracked files', async () => {
  const { execSync } = await import('node:child_process');
  const { mkdtempSync, writeFileSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const dir = mkdtempSync(join(tmpdir(), 'git-dirty-test-'));
  execSync('git init -q', { cwd: dir });
  execSync('git commit --allow-empty -m init -q', {
    cwd: dir,
    env: { ...process.env, GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@t', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@t' },
  });
  writeFileSync(join(dir, 'dirty.txt'), 'x');
  const result = await checkGitClean({ cwd: dir });
  assert.equal(result.ok, false);
  assert.match(result.reason, /untracked|uncommitted/i);
});

test('checkMeasureRuns succeeds on numeric output', async () => {
  const result = await checkMeasureRuns({ cmd: 'echo 0.42' });
  assert.equal(result.ok, true);
  assert.equal(result.baseline, 0.42);
});

test('checkMeasureRuns fails on non-numeric output', async () => {
  const result = await checkMeasureRuns({ cmd: 'echo not-a-number' });
  assert.equal(result.ok, false);
  assert.match(result.reason, /numeric|parse/i);
});

test('checkMeasureRuns fails on nonzero exit', async () => {
  const result = await checkMeasureRuns({ cmd: 'false' });
  assert.equal(result.ok, false);
});

test('checkPaletteGaps reports missing commands', () => {
  const palette = ['Bash(git status)', 'Bash(pnpm run test)'];
  const planned = ['git status', 'pnpm run test', 'kubectl get pods'];
  const result = checkPaletteGaps({ palette, planned });
  assert.equal(result.ok, false);
  assert.deepEqual(result.missing, ['kubectl get pods']);
});

test('checkPaletteGaps passes when all covered', () => {
  const palette = ['Bash(git status)', 'Bash(pnpm run test)'];
  const planned = ['git status', 'pnpm run test'];
  const result = checkPaletteGaps({ palette, planned });
  assert.equal(result.ok, true);
  assert.deepEqual(result.missing, []);
});
```

- [ ] **Step 2: Run; confirm fails**

```bash
node --test genesis/agentic/readiness.test.mjs
```

Expected: module not found.

- [ ] **Step 3: Implement readiness checks**

Create `genesis/agentic/readiness.mjs`:

```javascript
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { matchesPalette } from './palette.mjs';

const execFileP = promisify(execFile);

/**
 * Run `git status --porcelain`; empty output means clean.
 */
export async function checkGitClean({ cwd }) {
  try {
    const { stdout } = await execFileP('git', ['status', '--porcelain'], {
      cwd,
      env: process.env,
    });
    if (stdout.trim().length > 0) {
      return {
        ok: false,
        reason: `git has untracked or uncommitted changes:\n${stdout.trim()}`,
      };
    }
    return { ok: true };
  } catch (err) {
    return { ok: false, reason: `git status failed: ${err.message}` };
  }
}

/**
 * Run the Objective's measure command; parse stdout as a number.
 * Returns the baseline for iteration 1's delta tracking.
 */
export async function checkMeasureRuns({ cmd }) {
  try {
    const { stdout } = await execFileP('sh', ['-c', cmd], { env: process.env });
    const value = Number(stdout.trim());
    if (Number.isNaN(value)) {
      return {
        ok: false,
        reason: `measure command did not return a numeric value (got: ${JSON.stringify(stdout.trim().slice(0, 120))})`,
      };
    }
    return { ok: true, baseline: value };
  } catch (err) {
    return { ok: false, reason: `measure command failed: ${err.message}` };
  }
}

/**
 * Compare planned commands against the palette. Returns the set that would
 * trigger permission prompts.
 */
export function checkPaletteGaps({ palette, planned }) {
  const missing = [];
  for (const cmd of planned) {
    if (!matchesPalette(cmd, palette)) missing.push(cmd);
  }
  return { ok: missing.length === 0, missing };
}

/**
 * CLI entry point. Reads an Objective YAML, runs all applicable checks,
 * emits a structured JSON readiness report to stdout. Exit 0 if ready,
 * 1 if any check failed.
 */
export async function runReadiness({ objectivePath }) {
  const { readFileSync } = await import('node:fs');
  // Minimal YAML-ish loader: expect the Objective file to be JSON or
  // properly-structured YAML. For simplicity in v1, accept JSON.
  const obj = JSON.parse(readFileSync(objectivePath, 'utf8'));
  const reports = {};

  reports.measure = await checkMeasureRuns({ cmd: obj.measure.run });
  reports.git = await checkGitClean({ cwd: process.cwd() });

  const ok = Object.values(reports).every((r) => r.ok);
  const out = {
    ready: ok,
    checks: reports,
    baseline: reports.measure.ok ? reports.measure.baseline : null,
  };
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
  process.exit(ok ? 0 : 1);
}

// Invoked directly?
if (import.meta.url === `file://${process.argv[1]}`) {
  const idx = process.argv.indexOf('--objective');
  const objectivePath = idx >= 0 ? process.argv[idx + 1] : null;
  if (!objectivePath) {
    console.error('Usage: node genesis/agentic/readiness.mjs --objective <path>');
    process.exit(2);
  }
  runReadiness({ objectivePath }).catch((e) => {
    console.error(e);
    process.exit(2);
  });
}
```

- [ ] **Step 4: Run test; confirm pass**

```bash
node --test genesis/agentic/readiness.test.mjs
```

Expected: all 7 tests pass.

- [ ] **Step 5: Add root-level test + readiness scripts**

Edit `package.json`. Locate the `"scripts"` block; add two new entries near the existing `schema:*` scripts:

```json
    "agentic:test": "node --test genesis/agentic/*.test.mjs",
    "agentic:readiness": "node genesis/agentic/readiness.mjs"
```

- [ ] **Step 6: Verify agentic tests pass via pnpm**

```bash
pnpm run agentic:test
```

Expected: all tests across palette, generalize, readiness, schemas, anti-patterns, taxonomy pass.

- [ ] **Step 7: Commit**

```bash
git add genesis/agentic/readiness.mjs genesis/agentic/readiness.test.mjs package.json
git commit -m "feat(agentic): pre-shift readiness check script

Validates git state cleanliness, measure command runs and returns
a parseable number (captures baseline), and planned commands are
covered by the palette. Exits nonzero on any failure; CLI-invocable
with --objective <path>. Adds pnpm run agentic:test and
agentic:readiness wrappers."
```

---

## Phase 3 — Templates

### Task 9: Shift journal and sprint result templates

**Files:**
- Create: `genesis/docs/shifts/JOURNAL-TEMPLATE.md`
- Create: `genesis/docs/shifts/SPRINT-RESULT-TEMPLATE.md`

- [ ] **Step 1: Create shifts docs directory**

```bash
mkdir -p genesis/docs/shifts
```

- [ ] **Step 2: Write the journal stanza template**

Create `genesis/docs/shifts/JOURNAL-TEMPLATE.md`:

````markdown
# Shift Journal — `<shift-id>`

**Objective:** `<objective-name>` — `<one-line description>`
**Kicked off:** `<ISO timestamp>`
**Budget:** `<iterations>` iterations, `<wall_clock_min>` minutes
**Operator:** `<git user.name>`

## Stability Tracker

- Consecutive passing measurements: `<counter>`
- Required for done: `<consecutive>`
- Fresh-trigger measurement captured: `<yes|no>`

## Trajectory Summary *(last 3 iterations)*

`<auto-maintained header; Opus refreshes at start of each iteration>`

---

## Iteration stanza shape

Each iteration appends one stanza of this shape:

### Iteration `N` — `<iteration type>` — `<timestamp>`

**Measurement:** `<value>` (delta `<+X|-X|0>` from iteration `N-1`)
**Context:** build `<id>`, status `<passed|failed|running>`, first failing stage `<name|none>`

**Observation (Haiku):**

```yaml
primary_failure:
  error_class: <short tag>
  evidence: |
    <5-10 lines>
  files_mentioned:
    - <path>
confidence: <low|medium|high>
```

**Anti-patterns observed this iteration:**

- `AP-<NNN>` — `<name>`: `<one-line evidence>`

**Verification pass (Sonnet):** `<dispatched|skipped>` — `<directive if dispatched>`

**Decision (Opus):** `<progress|stall|novel|done-candidate|bail>`

**Rationale:** `<one paragraph — why this decision from Haiku's finding and trajectory>`

**Action taken:** `<none|edit <file>|commit+push <sha>|retrigger build|dispatch Sonnet|bail>`

**Next iteration:** `<observe-only|act-on-hypothesis|verify-done-candidate|-|terminal>`

---

## Permission wishlist (accumulated across iterations)

- **Blocker:** `<pattern>` — reason: `<why needed>` — iterations: `<list>`
- **Wishlist:** `<pattern>` — reason: `<why convenient>` — iterations: `<list>`
- **Redirect (resolved):** `<pattern>` — redirected to `<approved alternative>` — iteration: `<n>`

## Observed anti-patterns (accumulated across iterations)

| ID | Name | Occurrences | Evidence snippet |
|----|------|-------------|------------------|
| `AP-NNN` | `<name>` | `<count>` | `<excerpt>` |
````

- [ ] **Step 3: Write the sprint result template (close-time output)**

Create `genesis/docs/shifts/SPRINT-RESULT-TEMPLATE.md`:

````markdown
# Sprint Result — `<shift-id>`

**Objective:** `<objective-name>` — `<description>`
**Status:** `<done|bailed|interrupted>`
**Iterations run:** `<n>` of `<budget.iterations>`
**Wall clock:** `<elapsed-minutes>` of `<budget.wall_clock_min>`

## Outcome

### If done

- Final measurement: `<value>`
- Stability evidence: passing measurements at iteration `<n>` and iteration `<m>`, with at least one on a fresh trigger (`<build-id>`)
- Landing commit: `<sha>` (if any)
- Files changed: `<list>`

### If bailed

**Bail reason:** `<one-paragraph reason from Opus>`
**Question for operator:** `<explicit question Opus needs answered>`
**Proposed next step:** `<what Opus recommends once the question is answered>`
**Last measurement:** `<value>` at iteration `<n>`

### If interrupted

**Interruption type:** `<stop|budget-exhausted|tool-interruption>`
**State at interruption:** `<what was in flight>`

---

## Proposed palette additions

Ordered by priority (blockers first). Next shift's kickoff should review and
approve or reject each.

### Blockers *(approve before next shift)*

- `<narrow literal>` → proposed generalization: `<broader pattern>`
  - Purpose: `<why Opus / Sonnet needed this>`
  - Iterations where it arose: `<list>`
  - Safety taxonomy: `<broadly_safe|subcommand_scoped|never_wildcard>`

### Wishlist *(low priority, approve when convenient)*

- `<entry>` as above

---

## Proposed pipeline legibility improvements

Aggregated anti-patterns Haiku observed. Addressing these between sprints
reduces cost and increases signal quality for future shifts.

| ID | Name | Occurrences | Attestation maps to |
|----|------|-------------|----------------------|
| `AP-NNN` | `<name>` | `<n>` | `<brit-attestation-field>` |

## Judgment calls log

Iterations where Opus bailed, dispatched Sonnet for verification, or
distrusted a measurement. Feeds retrospective analysis of Objective
schema and playbook improvements.

- Iteration `<n>`: `<one-line summary of the call and its outcome>`

## Measurement trustworthiness notes

Low-confidence Haiku findings, done-regressions, oracle-skepticism events.

- Iteration `<n>`: `<event>`
````

- [ ] **Step 4: Commit**

```bash
git add genesis/docs/shifts/
git commit -m "docs(agentic): shift journal + sprint result templates

Canonical shapes for the running journal (appended per iteration) and
the sprint result (written at close). Referenced by the
agentic-developer skill."
```

---

### Task 10: Retrospective template

**Files:**
- Create: `genesis/docs/retrospectives/TEMPLATE.md`

- [ ] **Step 1: Create retrospectives directory**

```bash
mkdir -p genesis/docs/retrospectives
```

- [ ] **Step 2: Write the retrospective template**

Create `genesis/docs/retrospectives/TEMPLATE.md`:

```markdown
# Sprint Retrospective — `<sprint-name>`

**Date range:** `<YYYY-MM-DD>` to `<YYYY-MM-DD>`
**Shifts included:** `<list of shift-ids>`
**Compiled by:** `<operator>`

## Top anti-patterns by frequency *(permanent section)*

Aggregated from every shift's observed anti-patterns. Rank by total
occurrences across all shifts in the sprint.

| Rank | ID | Name | Total occurrences | Shifts hit |
|------|-----|------|-------------------|------------|
| 1 | `AP-NNN` | `<name>` | `<count>` | `<n>/<total>` |

**Actions:** per entry, list the fix decided on (Jenkinsfile change, tooling
PR, defer, etc.) and the owner.

## Graduated palette entries *(permanent section)*

Wishlist items that appeared across multiple shifts; promote to
`.claude/settings.json` in one batched PR.

- `<proposed pattern>` — appeared in shifts `<list>` — absorbs `<count>` literal variants
- …

---

## Implications for brit *(migration-bridge; remove once brit Phase 2a + rakia attestation-consumption lands)*

Itemized: observed pain → proposed attestation field or shape. Each item
cites shift-result evidence.

- **Observed:** `<anti-pattern or measurement-trustworthiness event>`
  - **Shift evidence:** `<shift-id#iteration>`
  - **Proposed brit shape:** `<BuildAttestation.field | DeployAttestation.field | new type>`
  - **Value if shipped:** `<which sprints / shifts would have been cheaper>`

## Implications for rakia *(migration-bridge; remove once rakia is the primary orchestrator)*

Itemized: observed orchestrator gap → proposed rakia behavior.

- **Observed:** `<permission blocker, missing action type, opaque build state, etc.>`
  - **Shift evidence:** `<shift-id#iteration>`
  - **Proposed rakia behavior:** `<stepwise action, attestation read, reach check, etc.>`
  - **Value if shipped:** `<which shifts would have been simpler>`

---

## Implications for the agentic-developer itself *(permanent section)*

v1.1 priorities, ordered by how many shifts would have benefited.

- **Proposal:** `<change to playbook, Objective schema, tier model, etc.>`
  - **Shifts that would have benefited:** `<list>`
  - **Estimated lift:** `<S|M|L>`

## Measurement trustworthiness review *(permanent section)*

Aggregated low-confidence / regression-after-done / oracle-skepticism
events. Decide per entry whether to tighten the Objective schema,
adjust stability defaults, or rewrite a specific measurement command.
```

- [ ] **Step 3: Commit**

```bash
git add genesis/docs/retrospectives/TEMPLATE.md
git commit -m "docs(agentic): sprint retrospective template

Permanent sections (anti-patterns, palette graduates, agentic-developer
improvements, measurement trustworthiness) + migration-bridge
sections (brit/rakia implications, removed once those land)."
```

---

## Phase 4 — Playbooks

### Task 11: `/generalize-permissions` skill

**Files:**
- Create: `.claude/skills/generalize-permissions/SKILL.md`

- [ ] **Step 1: Create the skill directory**

```bash
mkdir -p .claude/skills/generalize-permissions
```

- [ ] **Step 2: Write the skill document**

Create `.claude/skills/generalize-permissions/SKILL.md`:

````markdown
---
name: generalize-permissions
description: Cluster near-duplicate entries in .claude/settings.json + settings.local.json allow list into broader patterns under a safety taxonomy. Propose bulk collapses (10 entries → 1 pattern), user approves per-cluster. Invoked as first step of every agentic-developer shift kickoff AND standalone when the allowlist is getting bloated.
---

# Generalize Permissions

Reduce Claude Code permission-prompt pain by replacing many literal
allowlist entries with fewer broader patterns. Safety is preserved via
the taxonomy in `genesis/agentic/data/safety-taxonomy.json`.

## When to invoke

- **Automatically:** as the first step of every `/shift` kickoff, before
  Opus starts iteration 1. If novel commands the shift will need are not
  yet in the palette, this is where the gap closes.
- **Standalone:** any time the allowlist is large or the user has been
  approving near-duplicate entries frequently. Run `/generalize-permissions`.

## Procedure

1. **Load current allowlists.**

   Read:
   - `.claude/settings.json` (committed, durable — the permanent palette)
   - `.claude/settings.local.json` (local, shift-scoped additions)

2. **Run the generalization algorithm.**

   ```bash
   node --input-type=module -e "
     import('./genesis/agentic/generalize.mjs').then(async ({ clusterAndPropose }) => {
       const { readFileSync, existsSync } = await import('node:fs');
       const tax = JSON.parse(readFileSync('genesis/agentic/data/safety-taxonomy.json', 'utf8'));
       const durable = JSON.parse(readFileSync('.claude/settings.json', 'utf8')).permissions?.allow ?? [];
       const local = existsSync('.claude/settings.local.json')
         ? JSON.parse(readFileSync('.claude/settings.local.json', 'utf8')).permissions?.allow ?? []
         : [];
       const all = [...durable, ...local];
       const proposals = clusterAndPropose(all, tax);
       console.log(JSON.stringify(proposals, null, 2));
     });
   "
   ```

3. **Present each proposal to the user for review.**

   Group by safety tier. For each cluster, show:
   - Proposed pattern
   - The N literal entries it would replace
   - Safety classification

   Ask: *"Apply this generalization? [y/n/skip]"* per cluster. Bulk-approve
   groups where the user says "all broadly-safe proposals".

4. **Apply approved proposals.**

   - Remove the absorbed literal entries from whichever file they came from.
   - Add the single generalized pattern to `.claude/settings.json` (durable),
     unless it originated solely from `settings.local.json` shift-scoped
     entries, in which case it stays local and tagged.

5. **Summarize and return.**

   Print: `"<N> clusters applied, <M> literal entries collapsed to <P>
   patterns. Allowlist is now <X> entries (was <Y>)."`

## Safety invariants

- NEVER apply a generalization without explicit user approval (blanket
  "all broadly-safe" approval is fine; silent apply is not).
- NEVER generalize across the `never_wildcard` list in the taxonomy.
- NEVER promote a shift-scoped local entry to durable without the user
  explicitly approving the promotion.
- Back up the settings files (copy to `.bak` with timestamp) before
  writing modifications. If anything goes wrong, restore.

## Exit criteria

Allowlist has fewer entries than when you started, every removal is
justified by an approved generalization, and no novel patterns were
introduced outside the safety taxonomy.
````

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/generalize-permissions/SKILL.md
git commit -m "feat(agentic): /generalize-permissions skill

Playbook for clustering allowlist near-duplicates into broader patterns
under the safety taxonomy. Invoked at every shift kickoff and standalone."
```

---

### Task 12: `agentic-developer` skill

**Files:**
- Create: `.claude/skills/agentic-developer/SKILL.md`

- [ ] **Step 1: Create the skill directory**

```bash
mkdir -p .claude/skills/agentic-developer
```

- [ ] **Step 2: Write the skill document**

Create `.claude/skills/agentic-developer/SKILL.md`:

````markdown
---
name: agentic-developer
description: First-class overnight agentic developer. Iterates a named Objective against a CI pipeline — observe via Haiku, orchestrate + attempt as Opus, delegate to Sonnet on Opus's discretion, judge trajectory and bail with an explicit question if stuck. Uses stability-gated "done", path-scoped authority, palette-based command permission, and produces a single sprint-result markdown artifact. Invoked by the /shift slash command.
---

# Agentic Developer

You are the agentic developer. A first-class dev — not a watcher, not a
babysitter — who claims work (an Objective), iterates toward it, and
closes with done or a clean bail.

**Spec:** `genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md`
— read it if any of the principles below are unclear.

## Principles (load-bearing)

1. **First-class developer framing.** You own the shift end-to-end.
2. **Iterations are the scarce resource.** A cheap iteration that needs to
   re-run is more expensive than an expensive iteration that converges.
   You (Opus) orchestrate and attempt every iteration.
3. **Judgment governs loop length.** Bail when stuck with an explicit
   question. Budget is the safety net, not the primary exit.
4. **Done is stable.** Two consecutive passing measurements, at least one
   from a fresh trigger. A single green is a *done-candidate*, not *done*.
5. **Allowlist is the authority surface.** Never run bash outside the
   palette. Log wishlist instead; curate in sprint result.
6. **You may not edit the judge.** The Objective, measure command, files
   the measure reads, test runners, and test fixtures are off-limits.
   Bail with a proposal if they need to change.

## Kickoff (interactive — first 2-3 minutes)

When `/shift` invokes this skill:

1. **Run the `/generalize-permissions` skill first.** This bulk-collapses
   near-duplicate palette entries before the shift starts. Present user
   with proposals; wait for approval.

2. **Interview the user for the Objective.** Ask short, pointed questions:

   - *"What's the outcome you're aiming at? (one sentence)"*
   - *"How do we measure it? (a command that returns a number)"*
   - *"What's the baseline floor — the measurement we must not drop below?"*
   - *"What paths may I edit? (globs)"*
   - *"Budget — how many iterations, how many minutes?"*

   Compose an Objective YAML conforming to
   `.claude/schemas/objective.schema.json`. Show it to the user.
   Wait for explicit *"yes, kick off"* before proceeding.

3. **Predict the command palette for this shift.** Based on the Objective
   (paths, measure command, likely actions), list the bash/MCP commands
   you expect to need. Pattern-match against existing palette. Any gap
   becomes a proposed shift-scoped addition — present to user, wait for
   approval, write approved additions to `.claude/settings.local.json`
   under a `// shift:<id>` comment.

4. **Run pre-shift readiness check.**

   ```bash
   pnpm run agentic:readiness -- --objective .claude/shifts/<shift-id>.objective.yaml
   ```

   If `ready: false` in output, ABORT. Write a readiness report to
   `.claude/shifts/<shift-id>.readiness-report.md` explaining what failed,
   commit nothing, exit. User fixes in the morning.

5. **Initialize the journal.** Create
   `.claude/shifts/<shift-id>.journal.md` from
   `genesis/docs/shifts/JOURNAL-TEMPLATE.md`. Fill header; stability
   counter = 0; trajectory summary = empty.

## Iteration loop

Each iteration follows this skeleton. You (Opus) orchestrate; you decide
when Haiku and Sonnet are dispatched.

### 1. Ground

Read: Objective, last 3 journal stanzas (or all if fewer), current
palette union. Decide iteration type:

- `observe-only` — a build is running; nothing to do but check on it
- `act-on-hypothesis` — you have a theory, apply a change
- `retrigger` — last failure looked transient
- `verify-done-candidate` — last iteration's measurement passed; confirm
  with a fresh trigger
- `bail` — stuck, measurement untrustworthy, or out of ideas

### 2. Observe (Haiku dispatch)

Use the `Task` tool with a Haiku model to fetch and summarize current
state. Your prompt to Haiku must specify:

- Source artifacts (log paths, Jenkins build id, command outputs, etc.)
- The Haiku-output schema at `.claude/schemas/haiku-output.schema.json`
- A hint to flag anti-patterns from
  `genesis/agentic/data/anti-patterns.json`

Example Haiku dispatch:

> *"Reduce the following Jenkins build output into the structured
> summary defined by `.claude/schemas/haiku-output.schema.json`.
> Iteration is 4. Previous measurement was 0.15; current is in
> context.measurement.value of the artifact you'll fetch. Flag any
> anti-patterns per `genesis/agentic/data/anti-patterns.json` with IDs
> and evidence. Be bounded: 5-10 lines max for evidence fields."*

### 3. Verify (Sonnet dispatch, optional)

If Haiku's `confidence` is `low`, OR you suspect the summary:

- contradicts prior iteration findings
- is suspiciously clean given the change surface
- is missing critical detail (line numbers, file paths, timing)

Dispatch Sonnet via `Task` tool with a specific directive AND the
current palette. Sonnet must:

- Pattern-match its intended commands against the palette before running
  anything
- Return its finding + `wanted_commands: []` for anything it would have
  needed to run but couldn't

### 4. Act

Pick ONE action:

- **Edit/write a file.** Check: path is in `objective.scope.paths` AND
  not in the high-risk denylist AND not a measurement-oracle file.
- **Commit + push.** Use palette-approved `git add` and `git commit`
  commands. Never force-push, amend, rebase, or branch delete.
- **Retrigger build.** Via `mcp__jenkins__triggerBuild`.
- **Nothing.** You're waiting on a prior action; next iteration is
  `observe-only`.

### 5. Measure

Wait for the external action to produce a result (build completes,
test run finishes). For overnight shifts, use `ScheduleWakeup` with a
delay ≈ pipeline-run-time + 5 min. When you wake, run
`objective.measure` and capture the number.

### 6. Judge

Decide:

| Decision | Meaning | Next iteration |
|----------|---------|----------------|
| progress | measurement moved toward target | continue |
| stall | no delta over 2+ iterations | consider bail |
| novel | unexpected symptom, new hypothesis needed | continue cautiously |
| done-candidate | predicate holds, stability counter = 1 | verify with fresh trigger |
| done | predicate holds, stability counter ≥ required, fresh-trigger satisfied | terminal: close |
| bail | stuck, untrustworthy measurement, out of ideas | terminal: close with question |

### 7. Journal

Append an iteration stanza to `.claude/shifts/<shift-id>.journal.md`
following the shape in `genesis/docs/shifts/JOURNAL-TEMPLATE.md`.
Update the stability counter. Update the trajectory summary header.

### 8. Next

- Done or bail → go to Close.
- Otherwise → `ScheduleWakeup` with an appropriate delay, then return to
  step 1 next wake.

## Sonnet delegation patterns

Common directives (pick whichever matches; always include the palette):

- *"Re-read Jenkins stage log for any timing correlation contradicting
  the 'transient k8s' read. Return findings only, do not run commands
  outside the palette."*
- *"Read files X, Y, Z and confirm whether the manifest change is
  consistent with the CRD version referenced in `<path>`."*
- *"Draft a unified diff implementing the following diagnosis:
  `<paragraph>`. Do not apply; return the diff for my review."*
- *"Write the bail report stanza in the shape of
  `genesis/docs/shifts/SPRINT-RESULT-TEMPLATE.md`, including the
  wishlist curation from this journal."*

Sonnet never escalates directly. All permission signal flows back to
you. You curate its `wanted_commands` into redirect / wishlist / blocker.

## Wishlist curation

For every command you or Sonnet wanted to run but couldn't:

- **Redirect** if there's an already-approved alternative. Journal note,
  not surfaced in sprint result.
- **Wishlist** if it's convenient but you worked around it.
- **Blocker** if it genuinely stopped progress. This iteration counts as
  stalled; consider bail rather than burn another iteration missing the
  same signal.

Each wishlist/blocker entry in the journal (and eventually sprint
result) carries:

- Narrow literal pattern + proposed generalization
- Purpose
- Iteration(s) where it arose
- Safety taxonomy note

## Close

When done or bail:

1. Write final stanza to the journal.
2. Transform the journal into the sprint result by filling in the
   outcome section from `genesis/docs/shifts/SPRINT-RESULT-TEMPLATE.md`.
   Aggregate wishlist and anti-patterns across iterations.
3. Clean up shift-scoped `settings.local.json` entries — every entry
   tagged `// shift:<shift-id>` gets removed. Durable entries stay.
4. Do NOT commit the shift journal (it's gitignored).
5. Print the path of the sprint result and a one-paragraph summary.

## Invariants to never violate

- Never run bash outside the palette.
- Never modify the Objective, measure command, or oracle files mid-shift.
- Never force-push, amend, rebase, or branch delete.
- Never promote shift-scoped palette entries to durable without explicit
  user approval.
- Never declare done on a single passing measurement.
- Never commit the journal, readiness report, or Objective YAML.
````

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/agentic-developer/SKILL.md
git commit -m "feat(agentic): agentic-developer skill playbook

Main orchestration playbook. Interactive kickoff with Objective
authoring and palette proposal; iteration loop with Haiku observe,
Opus orchestrate + attempt, Sonnet-on-demand for verification;
stability-gated done with fresh-trigger requirement; wishlist
curation; sprint-result artifact on close."
```

---

### Task 13: `/shift` slash command

**Files:**
- Create: `.claude/commands/shift.md`

- [ ] **Step 1: Create the commands directory if missing**

```bash
mkdir -p .claude/commands
```

- [ ] **Step 2: Write the slash command**

Create `.claude/commands/shift.md`:

````markdown
---
description: Kick off an agentic developer shift — interactive Objective authoring, pre-shift readiness check, iteration loop, sprint result on close.
---

# /shift

Invokes the `agentic-developer` skill to run an agentic developer shift.

## Usage

- `/shift` — interactive kickoff (author Objective live, compose palette, start iteration)
- `/shift resume <shift-id>` — *(v2, not yet implemented)* resume a bailed shift after operator answers the bail question

## What it does

1. Runs the `generalize-permissions` skill on the current allowlist
   (bulk-collapse proposals).
2. Interviews the user for the Objective (name, measure command,
   baseline, scope, budget).
3. Composes a shift id, writes Objective YAML to
   `.claude/shifts/<shift-id>.objective.yaml`, writes initial journal
   to `.claude/shifts/<shift-id>.journal.md`.
4. Pattern-matches the predicted command palette against current
   allowlists; proposes shift-scoped additions to
   `.claude/settings.local.json` for user approval.
5. Runs `pnpm run agentic:readiness -- --objective <path>`. Aborts
   on any readiness failure with a report.
6. Enters the iteration loop, using `ScheduleWakeup` to pace between
   iterations until done, bail, or budget exhaustion.
7. On terminal state, writes a sprint result markdown at
   `.claude/shifts/<shift-id>.journal.md` and prints its path.

## See also

- Skill: `.claude/skills/agentic-developer/SKILL.md`
- Spec: `genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md`
- Templates: `genesis/docs/shifts/`, `genesis/docs/retrospectives/TEMPLATE.md`

## Loading the skill

Use the `Skill` tool with `skill: agentic-developer`.
````

- [ ] **Step 3: Verify all agentic tests still pass**

```bash
pnpm run agentic:test
```

Expected: all tests pass (no regression from skill/command files, which are markdown only).

- [ ] **Step 4: Commit**

```bash
git add .claude/commands/shift.md
git commit -m "feat(agentic): /shift slash command

Entry point that invokes the agentic-developer skill. Kickoff,
iteration, close. Resume (bailed-shift continuation) deferred to v2."
```

---

## Self-review checklist

Before handing off to execution, the engineer should verify:

- [ ] **Spec coverage.** Each requirement in `genesis/docs/superpowers/specs/2026-04-16-agentic-developer-loop-design.md` maps to a task. Particularly: kickoff flow (T11, T12, T13), Objective schema (T2), stability gate (Objective schema + T12 playbook), authority model (T12 playbook references paths allowlist, denylist, oracle-gate, palette), wishlist tiers (T9 + T12), anti-pattern reference (T5), retrospective hooks (T10, T12).
- [ ] **No placeholders.** No "TBD" / "TODO" / "similar to Task N" strings in code or templates.
- [ ] **Type consistency.** Function names, schema field names, and template variable names match across tasks. Examples: `observed_anti_patterns` appears in Haiku schema (T4), journal template (T9), anti-pattern data (T5). `stability.consecutive` / `stability.across_triggers` appear in Objective schema (T2) and playbook (T12).
- [ ] **All tests pass.** `pnpm run agentic:test` after Task 8 and again after every later task.

## Execution

**Plan complete and saved to `genesis/docs/superpowers/plans/2026-04-16-agentic-developer-loop.md`.**

Two execution options:

**1. Subagent-driven (recommended).** One fresh subagent per task, review between tasks, fast iteration. Best for this plan because tasks are bite-sized and independent until T11/T12/T13 (which depend on earlier schemas/data).

**2. Inline execution.** Execute tasks in this session using `superpowers:executing-plans`, with checkpoints for review.

Which approach?
