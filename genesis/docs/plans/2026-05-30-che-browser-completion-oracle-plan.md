# Che Browser Feedback — L2 Completion Oracle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the agentic-developer skill's existing visual-validation dimension so `/shift` can (1) generate the visual report **locally** in Che (closing the Jenkins round-trip), (2) treat `validatedRegressed == 0` as a hard, opt-in **done-gate**, and (3) capture a **kickoff baseline**.

**Architecture:** L2 is **prose edits to load-bearing skill/command/template files** — no code, no schema change, no change to `aggregate.ts` or the `@elohim-visually-validated` tag. The opt-in is a **journal-header flag** (`Visual gate: on/off`), so a gate-off shift is byte-identical to today (the safety property). The one runnable change is proving the local-render path produces `sprint-report-browser.json` in Che.

**Tech Stack:** Markdown skill/command/template files; the L1 browser path (`pnpm test:browser`, `pnpm build:sprint-report`); `genesis/a2o/scripts/lib/aggregate.ts` (reused, not modified).

**Spec:** `genesis/docs/superpowers/specs/2026-05-30-che-browser-completion-oracle-design.md`
**Depends on:** L1 (landed — `pnpm look` / `pnpm test:browser` work in Che).

---

## File Structure

| File | Change |
|---|---|
| `.claude/skills/agentic-developer/SKILL.md` | §378 Step 2 (+local generation); Close caveat (+local fallback); principle #4 (+conditional gate); done-candidate table row; integration-mode done note; Kickoff (+gate question, +baseline) |
| `.claude/commands/shift.md` | interview list (+visual-delivery-gated) |
| `genesis/docs/shifts/JOURNAL-TEMPLATE.md` | `## Visual Gate` + `## Visual baseline` header blocks |
| (reused, unchanged) `objective.schema.json`, `aggregate.ts`, the tag | no edits — the gate is a journal flag |

**Why journal-flag, not schema:** `objective.schema.json` is `additionalProperties:false` and the readiness validator parses it; adding a field means schema + validator changes. The gate is set once at kickoff and read by Opus (who reads the journal anyway), so the journal header is the right home — zero schema/validator risk, and consistent with "you may not edit the judge" (frozen at kickoff).

---

## Task 1: Delta 1 — local generation of the visual report (the round-trip closer)

**Files:**
- Modify: `.claude/skills/agentic-developer/SKILL.md` (§378 Step 2 paragraph ~line 395; Close caveat ~line 431)

- [ ] **Step 1: Prove the local-render path produces the artifact in Che (the real test)**

Run a scoped browser render + sprint-report locally and confirm `visualValidation` buckets are produced:
```bash
cd /projects/elohim/genesis/a2o && \
E2E_DEVICE_MODE=playwright E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
  pnpm exec cucumber-js --tags '@browser-only' features/auth/threshold-login-domain-scoping.feature --format summary 2>&1 | tail -5 ; \
pnpm build:sprint-report 2>&1 | tail -5
```
Then confirm the artifact + buckets exist:
```bash
cd /projects/elohim/genesis/a2o && \
test -f reports/sprint-report-browser.json && \
node -e "const r=require('./reports/sprint-report-browser.json'); console.log('visualValidation:', JSON.stringify(r.summary?.visualValidation))"
```
Expected: `reports/sprint-report-browser.json` exists; `visualValidation: {"validatedPassing":...,"validatedRegressed":...,"pendingPassing":...,"pendingFailing":...}` prints. (If `build:sprint-report` needs both API + browser cucumber reports, run the API `pnpm test` first; record the exact working invocation for Step 2's prose.)

- [ ] **Step 2: Add the local-generation paragraph to §378 Step 2**

Find this paragraph (SKILL.md, ~line 395):
```
**Step 2 (Observe) — visual-regression is a first-class candidate.** When dispatching `ci-observer` against an integration-mode build, pass it `genesis/a2o/reports/sprint-report-browser.json` (in addition to the API one) if the file exists. Ask the observer to count the `visualValidation` buckets and to flag every `visual-regression` finding as a candidate.
```
Immediately AFTER it, insert:
```
**Local generation (post-L1 — the round-trip closer).** The browser path now runs in Che (see `genesis/a2o/CLAUDE.md` → `pnpm look` and `E2E_DEVICE_MODE=playwright pnpm test:browser`). When `sprint-report-browser.json` is absent (CI browser stage skipped) or stale, you MAY produce it **locally** instead of waiting on CI:

```
cd genesis/a2o && E2E_DEVICE_MODE=playwright E2E_DOORWAY_ALPHA=<target> \
  pnpm exec cucumber-js --tags '@browser and <in-scope-filter>'   # cucumber-report-browser.json + screenshots
pnpm build:sprint-report                                          # sprint-report-browser.{json,md} with visualValidation buckets
```

Scope the tag/feature filter to the objective's in-scope features — a done-candidate render must be cheap, and the full `@browser` suite stresses the single shared browser (it dies partway through ~100+ scenarios; see the L1 memory `project_che_browser_feedback_loop`). Feed the resulting local artifact into the same observer/judge flow. The local artifact and the CI artifact are interchangeable inputs.
```

- [ ] **Step 3: Update the Close caveat to add the local fallback**

Find (SKILL.md, ~line 431):
```
If the browser stage did not run (probe failed, no Playwright in the image), say so explicitly: *"Visual validation: not measured — genesis pipeline browser stage skipped (Playwright probe failed). Follow-up: graduate to mcr.microsoft.com/playwright sidecar."* Don't omit the section.
```
Replace with:
```
If the CI browser stage did not run (probe failed, no Playwright in the image), **render locally** via the Step-2 local-generation path and report the local `visualValidation` counts — local rendering no longer blocks measurement (the CI sidecar graduation remains the durable fix for unattended CI runs). Only if the local render *also* cannot run, say so explicitly: *"Visual validation: not measured — browser render unavailable locally and in CI."* Don't omit the section.
```

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add .claude/skills/agentic-developer/SKILL.md
git commit -m "feat(shift): visual report can be generated locally in Che (L1 path)

Closes the Jenkins round-trip: the agentic developer renders the @browser
suite + build:sprint-report locally instead of waiting on the often-skipped
CI genesis browser stage. Reuses the existing visualValidation dimension.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: The opt-in flag — kickoff question + journal header

**Files:**
- Modify: `.claude/skills/agentic-developer/SKILL.md` (Kickoff step 1 interview, ~line 127)
- Modify: `.claude/commands/shift.md` (interview line, ~line 16)
- Modify: `genesis/docs/shifts/JOURNAL-TEMPLATE.md` (header block)

- [ ] **Step 1: Add the gate question to the SKILL.md kickoff interview**

Find (SKILL.md, ~line 124-128):
```
   - *"How do we measure it? (a command that returns a number)"*
   - *"What's the baseline floor — the measurement we must not drop below?"*
   - *"What paths may I edit? (globs)"*
   - *"Budget — how many iterations, how many minutes?"*
```
Replace with (add one line):
```
   - *"How do we measure it? (a command that returns a number)"*
   - *"What's the baseline floor — the measurement we must not drop below?"*
   - *"What paths may I edit? (globs)"*
   - *"Budget — how many iterations, how many minutes?"*
   - *"Visual-delivery-gated? (does 'done' require the user-facing experience to render correctly — not just a green measure? Default off; on for shifts landing a visible feature.)"*
```

- [ ] **Step 2: Record the flag in the journal at kickoff**

Find (SKILL.md, ~line 132-138):
```
   Compose an Objective conforming to
   `.claude/schemas/objective.schema.json`. Write as JSON at
   `.claude/shifts/<shift-id>.objective.json` — the readiness script
   parses JSON in v1 (YAML support deferred). Show it to the user.
   Wait for explicit *"yes, kick off"* before proceeding.
```
Immediately AFTER that paragraph, insert:
```
   Record the visual-gate answer in the journal header's **Visual Gate**
   block (`on`/`off`). The flag is frozen at kickoff like the measure —
   it is part of the judge and may not be edited mid-shift.
```

- [ ] **Step 3: Update the shift command's interview summary**

Find (`.claude/commands/shift.md`, ~line 16):
```
1. Interviews the user for the Objective (name, measure command,
   baseline, scope, budget).
```
Replace with:
```
1. Interviews the user for the Objective (name, measure command,
   baseline, scope, budget, and whether the shift is
   visual-delivery-gated).
```

- [ ] **Step 4: Add the Visual Gate block to the journal template**

In `genesis/docs/shifts/JOURNAL-TEMPLATE.md`, find the Stability Tracker block:
```
## Stability Tracker

- Consecutive passing measurements: `<counter>`
- Required for done: `<consecutive>`
- Fresh-trigger measurement captured: `<yes|no>`
```
Immediately AFTER it, insert:
```
## Visual Gate

- Visual-delivery-gated: `<on|off>`  *(frozen at kickoff; part of the judge)*
- When `on`, "done" additionally requires `validatedRegressed == 0` over the
  in-scope `@elohim-visually-validated` scenarios across two consecutive local
  renders (≥1 against a fresh build/deploy).
```

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add .claude/skills/agentic-developer/SKILL.md .claude/commands/shift.md genesis/docs/shifts/JOURNAL-TEMPLATE.md
git commit -m "feat(shift): opt-in visual-delivery-gate flag (kickoff question + journal header)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: The hard gate — done logic conditioned on the flag

**Files:**
- Modify: `.claude/skills/agentic-developer/SKILL.md` (principle #4 ~line 23; done-candidate table ~line 331; integration-mode done ~line 376)

- [ ] **Step 1: Add the conditional gate clause to principle #4**

Find (SKILL.md, ~line 23-27):
```
4. **Done is stable.** Two consecutive passing measurements, at least one
   from a **fresh trigger** — a new Jenkins build dispatched by the
   orchestrator from a `git push` you made this shift, *not* a poll or
   replay of a prior build id. A single green is a *done-candidate* (one
   pass, awaiting fresh-trigger confirmation), not *done*.
```
Immediately AFTER that paragraph (before the "Note on triggers:" paragraph), insert:
```
   **Visual gate (only when the journal header says `Visual gate: on`).**
   "Done" then requires BOTH the numeric measurement stable as above AND
   `validatedRegressed == 0` over the in-scope `@elohim-visually-validated`
   scenarios, confirmed across **two consecutive local renders** with ≥1
   against a fresh build/deploy (the same two-render flake guard `/deliver`
   uses). A `validatedRegressed > 0` at done-candidate blocks *done*; the
   failing tagged scenario's screenshot + steward judgment (§"Visual
   validation as an integration candidate dimension") becomes the next
   hypothesis. **When the header says `Visual gate: off`, this clause does
   not apply and "done" is the numeric measure alone — byte-identical to a
   non-visual shift.**
```

- [ ] **Step 2: Update the `done` row of the decision table**

Find (SKILL.md, ~line 331):
```
| done | predicate holds, stability counter ≥ required, fresh-trigger satisfied | terminal: close |
```
Replace with:
```
| done | predicate holds, stability counter ≥ required, fresh-trigger satisfied — AND (only if `Visual gate: on`) `validatedRegressed == 0` confirmed across two local renders | terminal: close |
```

- [ ] **Step 3: Note the gate in integration-mode done**

Find (SKILL.md, ~line 376):
```
Stability still requires two consecutive passing measurements, but in integration mode the predicate is per-candidate. The shift is "done" when the per-candidate measurements all stabilize OR the 5-loop budget is exhausted — whichever first. Bail if you've exhausted the candidate set without progress on any.
```
Immediately AFTER it, insert:
```
When `Visual gate: on`, the `validatedRegressed == 0` requirement (principle #4) is an additional, mode-orthogonal condition on the shift's done state — it applies in bring-up and integration mode alike, fed by the local render (Task 1).
```

- [ ] **Step 4: Verify the off-gate path is untouched (the safety property)**

Run:
```bash
cd /projects/elohim
grep -n "Visual gate: off\|byte-identical\|only when the journal header says \`Visual gate: on\`\|only if \`Visual gate: on\`" .claude/skills/agentic-developer/SKILL.md
```
Expected: every new gate clause is explicitly conditioned on `Visual gate: on`, and principle #4 states the `off` path is byte-identical. Confirm no unconditional gate language slipped in.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add .claude/skills/agentic-developer/SKILL.md
git commit -m "feat(shift): hard visual done-gate (validatedRegressed==0) gated on journal flag

Off-gate shifts are byte-identical to today; on-gate shifts require the
user-facing experience to render correctly (two local renders) before done.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Kickoff baseline (Delta 3)

**Files:**
- Modify: `.claude/skills/agentic-developer/SKILL.md` (Kickoff, after the Objective is composed)
- Modify: `genesis/docs/shifts/JOURNAL-TEMPLATE.md` (Visual baseline block)

- [ ] **Step 1: Add the conditional baseline render to kickoff**

In SKILL.md, find the paragraph added in Task 2 Step 2 (ends with "...part of the judge and may not be edited mid-shift."). Immediately AFTER it, insert:
```
   **Kickoff baseline (only when `Visual gate: on`).** After composing the
   Objective, run the local render once (Task 1 path, scoped to in-scope
   features) and record the baseline `visualValidation` counts + the
   `reports/screenshots/...` paths in the journal's **Visual baseline**
   block. Read the baseline screenshot(s) and state the starting visual
   state in the kickoff context — iteration-0 legibility for the
   before→after gradient. (Gate `off` → skip; no baseline render.)
```

- [ ] **Step 2: Add the Visual baseline block to the journal template**

In `genesis/docs/shifts/JOURNAL-TEMPLATE.md`, find the `## Visual Gate` block added in Task 2 Step 4. Immediately AFTER it, insert:
```
## Visual baseline *(only when visual-gated)*

- Captured at kickoff: `<yes|no>`
- Baseline buckets: `<vP>vP / <vR>vR / <pP>pP / <pF>pF`
- Baseline screenshots: `<reports/screenshots/... paths>`
- Starting visual state (one line): `<what the kickoff render showed>`
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add .claude/skills/agentic-developer/SKILL.md genesis/docs/shifts/JOURNAL-TEMPLATE.md
git commit -m "feat(shift): kickoff visual baseline render for gated shifts

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Coherence + safety verification

**Files:** none changed (verification only).

- [ ] **Step 1: Confirm no schema / aggregate / tag changes leaked in**

```bash
cd /projects/elohim
git diff --name-only dev..HEAD | grep -E "objective.schema.json|aggregate.ts|build-sprint-report" && echo "UNEXPECTED — investigate" || echo "clean: no schema/aggregate/report-generator changes (as designed)"
```
Expected: `clean: ...` — L2 only touched skill/command/template prose.

- [ ] **Step 2: Trace a gate-OFF shift (the safety property)**

Read the edited principle #4, the done-candidate table, and the integration-mode done note. Confirm by inspection: with `Visual gate: off`, none of the new clauses fire — the done condition is exactly the numeric measure + stability, identical to before L2. Write a one-line confirmation in the commit/PR notes: *"gate-off done logic unchanged."*

- [ ] **Step 3: Trace a gate-ON shift**

Read Kickoff (baseline render) → iteration loop (local render at done-candidate, Task 1) → principle #4 gate → done-candidate table row. Confirm the chain is coherent: kickoff captures baseline; at done-candidate the loop renders locally, computes `visualValidation`, and `done` requires `validatedRegressed == 0` across two renders. No dangling references (every term — `Visual gate`, `validatedRegressed`, local render path — is defined where used).

- [ ] **Step 4: Final commit (notes only, if any coherence fix was needed)**

If Steps 2-3 surfaced a wording gap, fix it in SKILL.md and commit:
```bash
cd /projects/elohim
git add .claude/skills/agentic-developer/SKILL.md
git commit -m "docs(shift): tighten visual-gate coherence (L2 self-review)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```
If no fix needed, skip.

---

## Self-Review

**Spec coverage:**
- Delta 1 (local generation, round-trip closer) → Task 1 (§378 Step 2 + Close caveat + runnable proof) ✓
- Delta 2 (hard composite gate, opt-in, no schema change) → Task 2 (flag) + Task 3 (done logic) ✓
- Delta 3 (kickoff baseline) → Task 4 ✓
- "Drop the duplicate machinery" → no Objective `visual` block, no new verdict; reuses tag/buckets/judgment (Task 5 Step 1 proves no schema/aggregate change) ✓
- Bring-up/integration mode-orthogonality → Task 3 Step 3 ✓
- Safety property (gate-off byte-identical) → Task 3 Step 4 + Task 5 Step 2 ✓

**Placeholder scan:** `<target>` and `<in-scope-filter>` in the inserted prose are intentional skill-runtime placeholders (the orchestrator fills them per shift), not plan gaps — every one is explained in the surrounding text. No TBD/TODO. ✓

**Consistency:** the flag name `Visual gate: on/off`, the metric `validatedRegressed == 0`, and "two consecutive local renders, ≥1 fresh" are identical across Tasks 2/3/4 and the journal template. ✓

**Out of scope:** making the CI genesis browser stage reliable (separate ops task); pixel-diffing; multi-tab/Tauri.
