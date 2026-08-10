---
title: Algedonic Slice 1 — concern-addressed pain in the delivery flow
id: algedonic-slice1-delivery-flow
status: Draft
cites:
  - algedonic-feedback-signal | Algedonic Feedback Signal | sha256:d0b1b524dc7240fc | path: genesis/docs/superpowers/specs/2026-08-10-algedonic-feedback-signal-design.md
  - evidence-ladder-push-left | Evidence Ladder + Push-Left Pressure | sha256:ac39aeb003dada60 | path: genesis/docs/superpowers/specs/2026-08-10-evidence-ladder-push-left-design.md
  - vision-gap-limit-governor-stub | Vision-Gap STUB | sha256:14ea8f3e81cd87c8 | path: genesis/docs/superpowers/plans/2026-06-14-vision-gap-limit-governor-stub.md
domain: process-meta
sprint: operator-directed-2026-08-10
---

# Algedonic Slice 1 — Delivery Flow Implementation Plan

**Status (2026-08-10):** Tasks 1, 2, 6 landed (`8a05236a7`, `11f334120`, `0a33fc356`+`23d95a96f`). Tasks 3-4 are re-homed to phase 2 — see `genesis/data/timeline/backlog/2026-08-10-algedonic-phase2-network-phase3-dedupe.md`. Task 5 was reshaped and landed as phase-1 Task 5 (`algedonic-phase1-epr-local-first-plan.md`). Task 7 (slice-2 backlog capture) is superseded by the same phase-2/phase-3 capture above.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the algedonic pattern into the development delivery loop first — every finding carries the `@concern` address of the promise it threatens, a CI no-measure becomes an addressed finding instead of silence, measurement-by-deploy becomes unwritable at push time, and the habits renderer joins live pain to each habit.

**Architecture:** Additive fields on the two existing sentinel ledgers (`ci-findings.jsonl`, `runtime-findings.jsonl`) routed by one tiny shared pure helper; one deny clause in the existing pre-push hook; one join in the existing habits renderer; two contract-first JSON schemas under the existing `feedback-signals/` family. No new pipelines, ledgers, or renderers.

**Tech Stack:** Python 3 stdlib only (`.claude/scripts` convention), bash (husky hook), JSON Schema wire contracts under sdk schemas v1 (source of truth: Holochain DHT `FeedbackSignal` entry — no storage table in this slice), plain-script tests (exit 0 = pass, importlib-loaded — see `.claude/scripts/_lib/__tests__/ci_harvest_echo_test.py` for the convention).

## Global Constraints

- **Fingerprints are byte-stable**: `concern` is additive metadata — it must NEVER enter `fingerprint()` inputs in either harvester (existing ledger entries must keep their fps).
- **Append-compat**: old ledger entries lack `concern`; every reader uses `.get("concern")`, never `["concern"]`.
- **Stdlib only** in `.claude/scripts` Python (no yaml import in harvesters — parse habits.yaml concerns by regex; `habits-status.py` already has its own yaml fallback and keeps it).
- **Commit-only**: commit each task; never `git push` (integrator pushes; the pre-push change itself rides the branch).
- **No new instruments**: the only new files are the shared route helper, its tests, two schemas, and one backlog capture.
- Tests are plain scripts: `python3 <test_file>` exits 0 on pass, non-zero with a message on fail.

---

### Task 1: Shared concern-route helper (`_lib/concern_routes.py`)

**Files:**
- Create: `.claude/scripts/_lib/concern_routes.py`
- Test: `.claude/scripts/_lib/__tests__/concern_routes_test.py`

**Interfaces:**
- Produces: `route(cls: str, context: dict) -> str | None` and `active_concern(habits_text: str) -> str | None` — consumed by Tasks 2 and 3.

- [ ] **Step 1: Write the failing test**

```python
#!/usr/bin/env python3
"""concern_routes: findings get the @concern address of the promise they threaten."""
import importlib.util, sys
from pathlib import Path

LIB = Path(__file__).resolve().parents[1] / "concern_routes.py"
spec = importlib.util.spec_from_file_location("concern_routes", LIB)
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)

HABITS_FIXTURE = """
habits:
  - id: notary-authority
    status: red
    active: true
    checks:
      - "a2o @concern:notary-authority (genesis/a2o/features/dataplane/notary-authority.feature)"
  - id: other-habit
    status: green
    active: false
    checks:
      - "a2o @concern:saga-06-heads-converge"
"""

fails = []
def check(name, got, want):
    if got != want: fails.append(f"{name}: got {got!r}, want {want!r}")

# active_concern: first @concern tag of the first active habit
check("active", m.active_concern(HABITS_FIXTURE), "notary-authority")
check("active-empty", m.active_concern("habits: []"), None)

# route: no-measure class resolves to the active habit's concern via context
check("no-measure", m.route("ci-no-measure", {"active_concern": "notary-authority"}), "notary-authority")
# route: explicit concern in context wins
check("explicit", m.route("ci-failure", {"concern": "saga-04-doorway-serves"}), "saga-04-doorway-serves")
# route: unknown class, no context → None (honest absence)
check("none", m.route("ci-failure", {}), None)

if fails:
    print("FAIL:\n  " + "\n  ".join(fails)); sys.exit(1)
print("concern_routes_test: PASS")
```

- [ ] **Step 2: Run to verify it fails** — `python3 .claude/scripts/_lib/__tests__/concern_routes_test.py` → FAIL (file not found / no attribute).

- [ ] **Step 3: Implement**

```python
"""concern_routes.py — resolve a finding to the @concern address it threatens.

The algedonic address book for the dev-plane sentinels (algedonic slice-1,
spec: algedonic-feedback-signal). Pure, stdlib-only, deterministic. A None
return is honest absence — never guess an address.
"""
import re

_CONCERN_TAG = re.compile(r"@concern:([a-z0-9][a-z0-9-]*)")
_ACTIVE_BLOCK = re.compile(
    r"^\s*-\s+id:.*?(?=^\s*-\s+id:|\Z)", re.M | re.S
)

def active_concern(habits_text: str):
    """First @concern tag inside the first `active: true` habit block."""
    for block in _ACTIVE_BLOCK.findall(habits_text or ""):
        if re.search(r"^\s*active:\s*true\s*$", block, re.M):
            m = _CONCERN_TAG.search(block)
            if m:
                return m.group(1)
    return None

def route(cls: str, context: dict):
    """Deterministic class→concern routing. context keys (all optional):
    concern (explicit, wins) · active_concern (fallback for measure-shaped classes)."""
    if context.get("concern"):
        return context["concern"]
    if cls in ("ci-no-measure",):
        return context.get("active_concern")
    return None
```

- [ ] **Step 4: Run to verify pass** — same command → `concern_routes_test: PASS`.
- [ ] **Step 5: Commit** — `git commit -m "feat(sentinels): concern-route helper — the algedonic address book" -- .claude/scripts/_lib/concern_routes.py .claude/scripts/_lib/__tests__/concern_routes_test.py`

---

### Task 2: ci-harvest — `concern` address + the `ci-no-measure` finding class

**Files:**
- Modify: `.claude/scripts/ci-harvest.py` (anchors: `collect_build_findings` ~line 193, `reconcile` ~line 301, new-entry dict ~line 333)
- Test: `.claude/scripts/_lib/__tests__/ci_harvest_no_measure_test.py`

**Interfaces:**
- Consumes: `concern_routes.route` / `active_concern` (Task 1).
- Produces: ledger entries with optional `"concern": str` and the new class `"ci-no-measure"` — consumed by Task 5.

- [ ] **Step 1: Write the failing test** — plain script, importlib-loads `ci-harvest.py` (hyphenated name: `spec_from_file_location("ci_harvest", SCRIPTS/"ci-harvest.py")`). Feed a fixture console tail containing the exact banner `run-dataplane-validation.sh` prints on gate-skip (`=== Dataplane Validation: DID NOT MEASURE ===`) plus the habits fixture from Task 1; assert `collect_build_findings` (or the console-scan helper it calls — mirror the echo test's entry point) yields one finding with `class == "ci-no-measure"`, `category == "NO_MEASURE"`, `concern == "notary-authority"`, and that its fingerprint equals `fingerprint(job, "NO_MEASURE", "dataplane-validation-did-not-measure")` — a FIXED identifier so repeat no-measures dedupe to ONE open finding (algedonic: pain is a held state, not a stream).
- [ ] **Step 2: Run to verify it fails.**
- [ ] **Step 3: Implement** — in the console-scan path of `collect_build_findings`: detect the banner line; mint the finding with the fixed identifier above; resolve `concern` via `route("ci-no-measure", {"active_concern": active_concern(HABITS.read_text())})` where `HABITS = REPO/"genesis/manifests/habits.yaml"` (guard `FileNotFoundError` → context `{}`). In `reconcile`'s new-entry dict add `**({"concern": f["concern"]} if f.get("concern") else {})`. For ordinary `ci-failure` findings, also attach `concern` when the failing line carries an `@concern:` tag (reuse `_CONCERN_TAG` via the Task-1 helper) — otherwise omit (honest absence).
- [ ] **Step 4: Run new test + the existing echo test** (`python3 .claude/scripts/_lib/__tests__/ci_harvest_echo_test.py`) — both PASS (proves fingerprints and the console-scan budget are undisturbed).
- [ ] **Step 5: Commit.**

---

### Task 3: runtime-harvest — `concern` address on exhaustion findings

**Files:**
- Modify: `.claude/scripts/_lib/runtime_harvest.py` (anchors: `evaluate` ~line 131, `reconcile` ~line 146)
- Test: extend the existing runtime-harvest test if present (`ls .claude/scripts/_lib/__tests__/ | grep runtime`), else create `.claude/scripts/_lib/__tests__/runtime_harvest_concern_test.py` on the same plain-script convention.

**Interfaces:**
- Consumes: `concern_routes.route` (Task 1). `evaluate(window)` gains an optional `context: dict = None` parameter (default preserves every existing call site).
- Produces: runtime findings with optional `"concern"` — consumed by Task 5.

- [ ] **Step 1: Failing test** — build a minimal `window` that trips one predicate (mirror whichever fixture shape the existing tests use for `_circuit_open`; if none exists, construct the 8-poll ring with a stuck-Open circuit field per the predicate's field names read from the source). Call `evaluate(window, context={"concern": "saga-04-doorway-serves"})`; assert the finding dict carries `concern`, and that `fingerprint()` inputs are unchanged (same fp with and without context).
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** — thread `context` through `evaluate` → each predicate's finding dict gets `concern = route(cls, context)` when non-None. `reconcile` copies `concern` onto new entries (`.get`, additive).
- [ ] **Step 4: Run new + existing runtime tests → PASS.**
- [ ] **Step 5: Commit.**

---

### Task 4: pre-push — dispatch-tag / changeset coherence (measurement-by-deploy becomes unwritable)

**Files:**
- Create: `.claude/scripts/edge-watch-match.py`
- Modify: `.husky/pre-push.bash` — insert AFTER the `while read` loop that fills `CHANGED` (~line 163) and **BEFORE** the `if [ -z "$CHANGED" ]` early-exit (~line 165). Placement is load-bearing: the anti-pattern's canonical form is an *empty* changeset with a `[build:edge]` tag, which the early-exit would otherwise bypass.
- Test: `.claude/scripts/_lib/__tests__/edge_watch_match_test.py`

**Interfaces:**
- Produces: `edge-watch-match.py` — stdin = newline-separated changed paths; stdout = count of lines matching the union of `elohim/holochain/build-manifest.json` `steps.*.inputs.sources` globs; errors print `ERR` and exit 2 (the hook treats non-`0` output as "don't block").

- [ ] **Step 1: Failing test** for the matcher:

```python
#!/usr/bin/env python3
import subprocess, sys
from pathlib import Path
SCRIPT = Path(__file__).resolve().parents[2] / "edge-watch-match.py"
def run(paths):
    p = subprocess.run([sys.executable, str(SCRIPT)], input="\n".join(paths),
                       capture_output=True, text=True)
    return p.stdout.strip()
fails = []
if run(["elohim/elohim-storage/src/lib.rs"]) == "0": fails.append("storage path should match edge globs")
if run(["genesis/manifests/habits.yaml"]) != "0": fails.append("habits.yaml must NOT match edge globs")
if run([]) != "0": fails.append("empty changeset → 0")
if fails: print("FAIL:\n  " + "\n  ".join(fails)); sys.exit(1)
print("edge_watch_match_test: PASS")
```

- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement the matcher** — stdlib `json` + glob matching (translate `**` globs with `pathlib.PurePath.full_match` on py≥3.13, else a small `re.compile(glob→regex)`; check `python3 --version` in-container first and use the simplest that passes Step 1's fixtures). Union globs from every `steps.*.inputs.sources` array in the manifest — read from the manifest at run time, never hardcoded (the manifest is the single source the orchestrator itself walks).
- [ ] **Step 4: Add the hook clause** in `.husky/pre-push.bash`:

```bash
# ---- dispatch-tag/changeset coherence (algedonic slice-1) -------------------
# [build:edge] with no edge-watched changes = measurement-by-deploy: the push
# restarts the 7-pod fleet it is trying to measure. Sanctioned measure verb:
# [build:edge] [edge:validate-only]. Deliberate deploy anyway: MEASURE_DEPLOY=1.
case " $PUSH_TARGETS " in *"refs/heads/dev"*)
  HEAD_MSG=$(git log -1 --pretty=%B 2>/dev/null || echo "")
  if printf '%s' "$HEAD_MSG" | grep -q '\[build:edge\]' \
     && ! printf '%s' "$HEAD_MSG" | grep -q '\[edge:validate-only\]' \
     && [ "${MEASURE_DEPLOY:-0}" != "1" ]; then
    EDGE_TOUCHED=$(printf '%s\n' "$CHANGED" | python3 .claude/scripts/edge-watch-match.py) || EDGE_TOUCHED="ERR"
    if [ "$EDGE_TOUCHED" = "0" ]; then
      echo "✗ PUSH DENIED — [build:edge] with no edge-watched changes is measurement-by-deploy."
      echo "  To MEASURE:  tag the empty commit  [build:edge] [edge:validate-only]"
      echo "  To DEPLOY anyway:  MEASURE_DEPLOY=1 git push"
      exit 1
    fi
  fi
;; esac
```

- [ ] **Step 5: Verify both directions** — `bash -n .husky/pre-push.bash` (syntax), then a sourced-function harness that sets `PUSH_TARGETS="refs/heads/dev"`, `CHANGED=""` and stubs `git log -1` output via a wrapper function. Expected: bare `[build:edge]`+empty → denied; `[build:edge] [edge:validate-only]`+empty → allowed; `[build:edge]`+`elohim/elohim-storage/src/lib.rs` → allowed.
- [ ] **Step 6: Commit.**

---

### Task 5: habits-status — join live pain + render the address

**Files:**
- Modify: `.claude/scripts/habits-status.py` (anchors: `headline` ~line 51, `full` ~line 80)
- Test: `.claude/scripts/_lib/__tests__/habits_status_pain_test.py`

**Interfaces:**
- Consumes: ledger entries with `.get("concern")` (Tasks 2-3); habits checks' `@concern:` tags.
- Produces: headline suffix `· pain: N open @<concern>` when the TOP RED's concern has open findings; `--full` gains one `pain:` line per habit listing `N open (fp1, fp2, …≤3)`.

- [ ] **Step 1: Failing test** — importlib-load `habits-status.py`, override a new module-level `LEDGERS` constant (added in Step 3; test sets `m.LEDGERS = [fixture_path]`) pointing at a fixture jsonl containing two entries: `{"fp":"abc123","class":"ci-no-measure","status":"open","concern":"notary-authority"}` and `{"fp":"def456","status":"open"}` (no concern — must not crash, must not count). Assert `headline(...)` contains `pain: 1 open` and `full(...)` contains `abc123`.
- [ ] **Step 2: Run → FAIL.**
- [ ] **Step 3: Implement** — `LEDGERS` constant (ci-findings + runtime-findings absolute paths derived the same way the existing `HABITS` constant derives the repo root); `def open_pain()` → dict concern→[fps] over lines where `status=="open"` and `.get("concern")`; join in `headline` (top-red's first `@concern:` tag extracted from its check string with an inline copy of the `@concern:` regex — habits-status stays import-free) and per-habit in `full`. Missing/unreadable ledgers → empty dict (never a crash at session start).
- [ ] **Step 4: Run test + `python3 .claude/scripts/habits-status.py` (live smoke — must render; a real pain line is a pass, not a failure) → PASS.**
- [ ] **Step 5: Commit.**

---

### Task 6: Contract foothold — the two algedonic schemas

**Files:**
- Create: `elohim/sdk/schemas/v1/feedback-signals/algedonic-approach.schema.json`
- Create: `elohim/sdk/schemas/v1/feedback-signals/algedonic-breach.schema.json`
- Reference (read first, mirror exactly): `elohim/sdk/schemas/v1/feedback-signals/rate-limit-exceeded.schema.json` (`$schema`/`$id` conventions, `target`/`declarer`/`evidence`/`severity`/`signed_at` shapes)

**Source of truth (p2p-design-gate):** Holochain DHT — these schemas are wire contracts for instances of the EXISTING `FeedbackSignal` DHT entry type (Category A, spec §3). No storage table, route, or projection is created in this slice; slice 2 owns the zome whitelist + projection. Each schema carries `"$comment": "Source of truth: DHT (FeedbackSignal entry); this file is the wire contract only."`

**Interfaces:**
- Produces: the slice-2 wire contract. `evidence` requires `{stock: number, limit: number, bound_ref: string (CID of the bounding commitment/manifest)}`; `algedonic-approach` additionally requires `threshold_pct: number` (the band edge, per the limit-governor stub); `target` = CID of the promise threatened; `severity` enum `info|warn|critical`; both kinds document in `description` that emission is hysteresis-bounded, one open signal per (declarer, target, kind), `standing_impact` fixed `advisory`.

- [ ] **Step 1:** Read `rate-limit-exceeded.schema.json`; write both schemas mirroring its envelope conventions byte-for-byte where shared (same `$schema` draft, same `$id` pattern, same `signed_at` format), differing only in the `evidence` block + kind constants above.
- [ ] **Step 2: Verify** — `pnpm run schema:test` and `pnpm run schema:validate` from repo root → green (Expected: the suite picks up new files in `feedback-signals/` the same way it validates the three existing ones; if it enumerates explicitly, add the two filenames wherever `rate-limit-exceeded` is listed).
- [ ] **Step 3: Commit.**

---

### Task 7: Complementary capture — slice-2 backlog entry

**Files:**
- Create: `genesis/data/timeline/backlog/2026-08-10-algedonic-protocol-slice2.md`

- [ ] **Step 1:** Write the capture (follow the naming + frontmatter conventions visible in `genesis/data/timeline/backlog/`; satisfy that dir's `.epr-meta` if the write hook demands fields): one-line items for — zome `SIGNAL_KINDS` whitelist extension + kind-gates in `create_feedback_signal` (`evidence`+`bound_ref` required); `FloorClass::CounterEvidence` routing + property-test extension; storage projection + emitters at the self-heal exhaustion sites (`/admin/self-healing` completion rides this); C15 algedonic-channel minting in the concern canon; app-manifest `algedonicHandler` field; `.epr-meta` authoring policy on `elohim/epr/src/kind.rs` + `elohim/sdk/domains/*/manifest/`. Each item cites `algedonic-feedback-signal` §5 slice 2.
- [ ] **Step 2: Commit.**

---

## Self-Review (done at authoring)

- **Spec coverage**: §5 slice-1 bullets map to Tasks 2 (no-measure addressed), 3 (runtime pain addressed), 4 (measurement-by-deploy unwritable), 5 (renderer join), 6 (schema foothold); the address book (Task 1) underlies 2-3; slice-2 items are captured, not planned (Task 7) — scope stays one rung.
- **Type consistency**: `route(cls, context)` / `active_concern(text)` names used identically in Tasks 1-3; ledger field is `concern` everywhere; `ci-no-measure` class string identical in Tasks 2 and 5's fixture.
- **Known unknowns, stated honestly**: exact insertion lines in `ci-harvest.py`'s console-scan and the runtime predicate fixture shape are anchored by function name, not line number — the implementer verifies against source (both files were surveyed 2026-08-10; anchors confirmed present).
