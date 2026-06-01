# Che Browser Feedback — L2 Completion Oracle: Local Visual Done-Gate for `/shift`

> Spec 2 of 2. Builds on L1 (`2026-05-30-che-browser-feedback-foundation-design.md`), which made a
> headless browser launch in Che and shipped the `look` primitive.
>
> **REVISED 2026-05-30 (extension, not invention).** The first draft of this spec proposed a
> parallel visual-verdict system (an Objective `visual` block + a new tier-3 verdict). That was
> written blind to an existing system: the agentic-developer skill **already has** a full
> visual-validation dimension. This revision rescopes L2 to *extend* that system with the three
> things it genuinely lacks — local rendering, a hard done-gate, and a kickoff baseline — and
> **drops** the duplicate machinery.

## What already exists (do NOT rebuild)

`.claude/skills/agentic-developer/SKILL.md` §"Visual validation as an integration candidate
dimension" (line ~378) already provides, today:

- **`summary.visualValidation`** — a 2×2 bucket object `{ validatedPassing, validatedRegressed,
  pendingPassing, pendingFailing }`, computed in `genesis/a2o/scripts/lib/aggregate.ts` by joining
  the **`@elohim-visually-validated`** Gherkin tag with each scenario's pass/fail status. Emitted
  into `genesis/a2o/reports/sprint-report-browser.{json,md}` by `pnpm build:sprint-report`.
- **The "goal" is already encoded** — a scenario tagged `@elohim-visually-validated` *is* the
  declaration that "this experience must be delivered." Real features already use it
  (`features/auth/threshold-login-domain-scoping.feature`, etc.).
- **The tier-3 steward verdict already exists** (Step 3 directive): open `Finding.screenshotPath`
  (`reports/screenshots/{featureSlug}/{scenarioSlug}--*.png`), describe it, and judge "does this
  screen carry the experience the protocol promises here?" anchored in the manifesto/epic narrative.
  This is exactly the screenshot-vs-promise judgment the old draft proposed to add.
- **Per-iteration visual journaling** (`visual: 12vP / 3vR / 47pP / 18pF`) and **sprint-result
  visual surfacing** are already specified.

The old draft's Objective `visual` block, `whatShouldBeVisible`, and standalone verdict procedure
are therefore **cut**. L2 reuses the tag, the buckets, the steward judgment, the journaling, and the
sprint-result surfacing as-is.

## The one fatal dependency L2 removes

The existing dimension is **CI-artifact-driven**: it reads `sprint-report-browser.json` produced by
the **genesis Jenkins browser stage**, which the skill itself admits *"often did not run (Playwright
probe failed, no Playwright in the image)"* — its documented fallback is *"graduate to a Playwright
sidecar."* **That dependency is the Jenkins round-trip the operator wants gone.** L1 made the browser
path work locally in Che. L2 closes the loop by generating the same artifact **locally**.

## Settled design — three deltas, all extensions

### Delta 1 — Local generation of the visual report (the round-trip closer)

Teach the agentic-developer skill that, post-L1, the browser path runs **in Che**. So the loop may
**produce** `sprint-report-browser.json` locally instead of only **consuming** it from CI:

- **At kickoff and at done-candidate**, the orchestrator (or a delegated Sonnet) runs locally:
  ```
  cd genesis/a2o && E2E_DEVICE_MODE=playwright E2E_DOORWAY_ALPHA=<target> \
    pnpm test:browser            # cucumber @browser → cucumber-report-browser.json + screenshots
  pnpm build:sprint-report       # → sprint-report-browser.{json,md} with visualValidation buckets
  ```
  Then the **existing** §378 Step-2/3/4 flow runs against the local artifact — no new bucket logic,
  no new verdict.
- Scope it to the in-scope features (a `--tags`/`--feature` filter) so a done-candidate render is
  cheap, not the full 110-scenario suite (which, per L1, also stresses the shared browser).
- The skill's "browser stage skipped → not measured" caveat gains a resolution path: *"if CI didn't
  render, render locally via the L1 path."* The CI artifact remains a valid source when present.

**Skill edits:** §378 Step-2 ("pass it `sprint-report-browser.json` *or generate it locally*"), and
the "Close" caveat. Small, additive.

### Delta 2 — Hard composite done-gate (genuinely new)

Today principle #4 "Done is stable" is **numeric-measure-only**; `validatedRegressed` is merely the
"highest-priority signal." L2 makes it a **gate** for delivery-oriented shifts:

- **Opt-in, zero schema change.** At kickoff the interview gains one question: *"Is this shift
  visual-delivery-gated?"* The answer is frozen in the **journal header** (`Visual gate: on/off` —
  markdown, consistent with "you may not edit the judge"). No change to `objective.schema.json`.
- **When the gate is on**, "done" requires BOTH:
  - the numeric `objective.measure` stable per `target.stability` (≥2 consecutive, ≥1 fresh), AND
  - `validatedRegressed == 0` over the in-scope `@elohim-visually-validated` scenarios, **two
    consecutive local renders** with ≥1 against a fresh build/deploy (mirrors the existing
    two-render flake guard).
- **When the gate is off**, behavior is **byte-identical to today** — `validatedRegressed` stays an
  advisory signal; backend-only shifts are unaffected. (This is the conditional-applicability safety
  property, now expressed via the journal flag instead of a schema block.)
- A `validatedRegressed > 0` at done-candidate blocks "done"; the failing tagged scenario's
  screenshot + steward judgment becomes the next hypothesis (already the §378 Step-3 behavior).

**Skill edits:** principle #4 (add the gate clause, gated on the journal flag), the done-candidate
decision row in §"Iteration loop" and §"adaptations in integration mode".

### Delta 3 — Kickoff baseline (genuinely new, small)

The existing system is per-build deltas with no frozen "before." L2 adds a kickoff baseline so the
before→after gradient is legible:

- At kickoff, when the visual gate is on, run the local render once and record the baseline
  `visualValidation` counts + the `reports/screenshots/...` paths in the journal header (a "Visual
  baseline" block beside the Stability Tracker). For a lighter baseline of a single surface,
  `pnpm look <url> --out baseline` is sufficient; for the tagged-scenario set, the
  `build:sprint-report` snapshot is the baseline.
- The implementer reads the baseline screenshot(s) and states the starting visual state in the
  kickoff stanza (iteration 0 legibility).

**Skill edits:** §"Kickoff" step list (add the conditional baseline render), JOURNAL-TEMPLATE header
(add an optional "Visual baseline" block).

## Bring-up vs integration mode

The existing visual dimension is documented under **integration mode**. The hard gate (Delta 2) must
also be reachable from a **bring-up / delivery** shift whose whole point is landing one feature's
visible experience. L2 notes that the visual gate is **mode-orthogonal**: it activates from the
journal `Visual gate: on` flag regardless of bring-up/integration, and the local-render path (Delta
1) is what makes it affordable in a bring-up shift (no CI browser stage needed).

## Files touched (L2)

| File | Change |
|---|---|
| `.claude/skills/agentic-developer/SKILL.md` | §378 Step-2 (+local generation), principle #4 (+conditional gate), done-candidate rows, §Kickoff (+baseline), Close caveat (+local fallback) |
| `.claude/commands/shift.md` | kickoff interview gains the "visual-delivery-gated?" question |
| `genesis/docs/shifts/JOURNAL-TEMPLATE.md` | optional `Visual gate:` + `Visual baseline` header block |
| (no change) `objective.schema.json` | **unchanged** — gate is a journal flag, not a schema field |
| (no change) `aggregate.ts` / `build-sprint-report.ts` / the tag | **reused as-is** |

## Verification

1. **Off-gate regression (safety property, prove first):** a shift with `Visual gate: off` runs the
   loop *byte-identical* to today — no local render forced, `validatedRegressed` advisory only.
2. **Local generation:** with the gate on, the loop runs `pnpm test:browser` + `pnpm build:sprint-report`
   locally in Che and produces `sprint-report-browser.json` with populated `visualValidation` buckets —
   no CI dependency.
3. **Gate blocks:** a tagged scenario whose surface is broken → `validatedRegressed > 0` → "done" does
   NOT fire even with the numeric measure green; the screenshot + steward judgment drives the next
   hypothesis.
4. **Gate clears:** fixing the surface → `validatedRegressed == 0` across two local renders (≥1 fresh)
   + numeric measure stable → composite done fires.
5. **Baseline:** kickoff records the baseline counts + screenshot paths; the before→after delta is
   legible in the sprint result.

## Out of scope (L2)

The cut machinery (Objective `visual` block, parallel verdict); pixel-diffing; multi-tab/Tauri;
re-architecting `aggregate.ts`'s bucket logic; making the CI genesis browser stage itself reliable
(separate ops task — L2 only adds the *local* path as an alternative source).
