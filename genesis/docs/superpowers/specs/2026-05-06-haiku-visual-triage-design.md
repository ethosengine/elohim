# Haiku Visual Triage — Tier-1 of the Visual Judgment Escalation

**Date:** 2026-05-06
**Branch:** dev
**Status:** Design (skill + schema + memory updates land same day)
**Related:** `genesis/docs/superpowers/specs/2026-05-05-visual-validation-design.md` (visual validation as integration-mode candidate dimension)

## Problem

The new Playwright per-feature screenshots in the genesis pipeline have started landing. The first results show heterogeneous failure states — some screenshots are clearly blank ("did anything render at all?"), some show an app shell but no feature ("the app loaded but we never got to what we were testing"), some show the right feature but in an error state, and some show feature content the steward needs to judge against design intent.

Today's agent escalation has only two tiers for visual judgment:

- **ci-investigator (Sonnet)** — used when the orchestrator dispatches against a `visual-regression` finding. Returns specific UI element descriptions and stewardship-judgment-ish output. Cost: $$.
- **Opus orchestrator** — handles fallback when investigator's model can't see, and final UX/design-spec stewardship. Cost: $$$.

There is **no Tier-1 cheap categorical triage**. Every screenshot review is a Sonnet-tier dispatch — fine for one or two regressed scenarios, but expensive when there are dozens of `pendingPassing` screenshots a steward wants to skim through. It also means that "this screenshot is just blank" — a clear infra-failure signal that needs no semantic judgment — costs the same as "does this delivered experience match the protocol's vision".

Empirically: the operator articulated five distinct visual judgment levels during the iter-3 reflection of shift `2026-05-06T02-44-rca-genesis-browser-failure-classes`:

1. *"Blank page — clearly didn't work."*
2. *"App shell loaded but the feature isn't there."*
3. *"I can identify the feature being tested in the screenshot."*
4. *"Is the feature technically complete?"*
5. *"Does this meet the human's UX and design specification?"*

Levels 1-3 are **categorical** — cheap, closed-taxonomy, no judgment. Level 4 is **technical completeness** — bounded specifics, comparison to expected UI elements. Level 5 is **stewardship** — vision alignment, design coherence, full context.

These map cleanly to the existing Haiku/Sonnet/Opus tier model. Tier-1 is the missing piece.

## Design

### Tier model

| Tier | Agent | Model | Question | Output shape | Cost |
|---:|---|:--:|---|---|---:|
| 1 | `ci-observer` (visual-triage mode) | Haiku | What category of visual state is on screen? Is the feature visually identifiable? | `image_state` enum + `feature_identifiable` bool + one-line evidence | $ |
| 2 | `ci-investigator` (visual-completeness directive) | Sonnet | Are the expected UI elements present? Is the feature technically rendered correctly? | Specific UI element list + completeness verdict | $$ |
| 3 | Opus orchestrator | Opus | Does this delivered experience match the protocol's vision and the feature's design spec? | Stewardship judgment + reasoning anchored in epic/manifesto context | $$$ |

The orchestrator dispatches by triage outcome:

- Tier-1 returns `blank | loading | error_overlay | partial_render | unreadable` → orchestrator typically classifies as infra failure or test-framework regression. No tier-2 dispatch needed; act on the test/infra side.
- Tier-1 returns `feature_visible` with `feature_identifiable: true` → tier-2 dispatched for completeness check. (If `feature_identifiable: false`, that's still a tier-1 stop — the test landed on the wrong page.)
- Tier-2 returns "complete" → tier-3 (Opus orchestrator) does the stewardship judgment if/when the steward asks. Most builds skip tier-3.

### Schema — extend `haiku-output.schema.json`

Add an optional `visual_triage` field, populated only when the caller passes a screenshot artifact reference:

```json
"visual_triage": {
  "oneOf": [
    { "type": "null" },
    {
      "type": "object",
      "required": ["image_state", "feature_identifiable", "evidence_one_line", "screenshot_artifact_ref"],
      "properties": {
        "image_state": {
          "enum": ["blank", "loading", "error_overlay", "partial_render", "feature_visible", "unreadable"]
        },
        "feature_identifiable": { "type": "boolean" },
        "evidence_one_line": { "type": "string", "maxLength": 160 },
        "screenshot_artifact_ref": { "type": "string" }
      }
    }
  ]
}
```

Pattern matches the existing `dispatch_drift` field — optional, populated only in a specific mode, deterministic shape.

### `image_state` enum semantics

| Value | Meaning |
|---|---|
| `blank` | Mostly empty canvas (white/black/single color), no app chrome, no content. The browser navigated somewhere but nothing rendered. |
| `loading` | Spinner, skeleton, or "Loading…" text dominates. Real content has not arrived. Distinct from `blank` because *some* loading affordance is visible. |
| `error_overlay` | Error dialog, HTTP error page (e.g. 502, 504), uncaught-exception overlay, or "Content Not Found"-style application error dominates the viewport. |
| `partial_render` | App shell (navbar, sidebar, footer) is visible but the main content area is empty, broken, or shows only placeholder/error state. The app loaded; the feature didn't. |
| `feature_visible` | App shell + main content area both rendered with what looks like real feature content. Whether it's the *right* feature is the `feature_identifiable` boolean. |
| `unreadable` | Image cannot be loaded, is corrupted, or is too small/distorted to classify. Surface as a tier-1 confidence-low signal. |

Note the asymmetry: `partial_render` is **stronger evidence than `blank`** that the app is alive but couldn't deliver the feature. The orchestrator uses this to discriminate "test framework didn't navigate correctly" from "deploy is broken" from "feature isn't seeded".

### `feature_identifiable` semantics

`feature_identifiable: true` requires BOTH:

1. `image_state === "feature_visible"`, AND
2. The rendered content matches the scenario's feature label (the caller passes the scenario name; Haiku judges whether the visible content belongs to that feature)

If `image_state === "feature_visible"` but the content is for the wrong feature (e.g. test landed on the home page when the scenario expected the assessment page), `feature_identifiable: false`. This is a **routing failure** — distinct from infra failure but still a tier-1 stop.

### `evidence_one_line` discipline

Strict observational only. Examples of acceptable lines:

- `"Lamad navbar visible; main area shows 'Content Not Found' heading with URL code-block"`
- `"Browser viewport is solid white with no chrome visible"`
- `"App shell rendered; sidebar shows 4 items; main panel empty with skeleton placeholders"`

Examples of **violations** (would fail review):

- `"Looks broken"` (judgment, not observation)
- `"User can't proceed from here"` (UX claim, not observation)
- `"Probably a routing bug"` (inference, not observation)
- `"Feature is incomplete"` (Tier-2 territory)

The 160-char cap is intentional — forces single-clause observations. Tier-2 is where multi-clause descriptions live.

### Dispatch contract

The orchestrator's prompt to ci-observer for visual-triage mode:

```
Visual triage dispatch.

Screenshot URL: <jenkins artifact URL>
Scenario name: <e.g. "Susan completes the Attachment Style assessment">
Feature label: <e.g. "lamad/know-thyself-discovery">

Pull the screenshot via WebFetch, then Read the resulting local path. Return
a haiku-output JSON with `visual_triage` populated:

  - image_state: closed enum from spec
  - feature_identifiable: true ONLY if image_state is feature_visible AND
    visible content matches the feature_label
  - evidence_one_line: ≤160 chars, observational only, no judgment
  - screenshot_artifact_ref: echo the URL

Do NOT name specific UI elements beyond what evidence_one_line allows.
Do NOT assess completeness, design quality, or UX. Those are tier-2 / tier-3.
```

The agent's reply is one JSON object on the haiku-output schema. The orchestrator either acts on it directly (for blank/error/partial cases) or hands it to ci-investigator with the same screenshot ref + a tier-2 directive.

### Tier-2 directive (for orchestrator's use, not part of this spec)

When tier-1 returns `feature_visible` + `feature_identifiable: true`, the orchestrator's tier-2 dispatch to ci-investigator gets a directive shaped like:

```
Technical completeness check.

Screenshot: <URL>  (already triaged tier-1 as feature_visible)
Feature: <label>
Expected UI elements (from feature spec / page-model selectors): <list>

For each expected element, report visible | not visible | partially visible
| ambiguous. Then return a verdict: complete | incomplete | uncertain.

Do NOT make stewardship/UX judgments. That's tier-3.
```

Not in this spec's scope; recorded for context. Tier-2 directive stays in the agentic-developer skill text.

## Implementation

Same-day landing alongside this spec:

1. **`haiku-output.schema.json`** — add optional `visual_triage` field.
2. **`.claude/agents/ci-observer.md`** — add a "Visual triage mode" section after "Validate mode", describing the dispatch contract, the `image_state` enum, and the discipline rules. Add `feature_identifiable` and `evidence_one_line` to the "What you DO report" section.
3. **`feedback_haiku_observe_only_no_specifics.md`** memory — add a footnote noting the visual-triage exception (Haiku CAN populate `visual_triage` because it's closed-taxonomy + bounded one-liner, same discipline shape as the rest of the schema).

Subsequent (visual-validation sprint):

4. Update `agentic-developer/SKILL.md` integration-mode visual-validation section to specify the tier-1 → tier-2 → tier-3 dispatch decision tree.
5. Add visual_triage logic to the orchestrator's screenshot-finding handling — every `visual-regression` and any caller-flagged `pendingPassing` screenshot goes through tier-1 first.

## Why this stays disciplined

The whole reason ci-observer is cheap-and-trustworthy is the closed-taxonomy schema. Adding visual triage breaks none of that:

- `image_state` is a closed enum — no free-form judgment.
- `feature_identifiable` is a boolean — no nuance, no scoring.
- `evidence_one_line` is bounded length and explicitly forbidden from judgment-language.
- `screenshot_artifact_ref` is just an echo — pure factual.

What we're NOT doing (and would break the discipline):

- ❌ Adding a `description` field that lets Haiku narrate freely about the screenshot.
- ❌ Asking Haiku to compare against design specs.
- ❌ Asking Haiku to judge UX quality.
- ❌ Asking Haiku to enumerate UI elements (that's tier-2).

The same memory-pattern that justified `feedback_haiku_observe_only_no_specifics.md` justifies this addition: structural constraint via schema. If Haiku stays in closed taxonomies and one-line bounded observations, it's reliable. If we let it narrate, it hallucinates.

## Cost economics

For a hypothetical visual-validation sprint with 50 screenshots to triage:

| Today (no tier-1) | With tier-1 |
|---|---|
| 50 × Sonnet ci-investigator dispatches @ ~5K tokens each = 250K Sonnet tokens | 50 × Haiku ci-observer triage @ ~1.5K tokens = 75K Haiku tokens |
| ≈ $0.75 | ≈ $0.06 |
| Plus: ~5 of 50 are blank/error and don't need Sonnet anyway | Plus: only ~10 of 50 (feature_visible + identifiable) need tier-2 escalation = 50K Sonnet = $0.15 |

Total: ~$0.21 with tiering vs. $0.75 flat — about 3.5× cheaper, AND faster (Haiku triage in parallel for all 50, sequential Sonnet only for the ~10 that need it).

The savings compound when the operator is grinding the long tail of `pendingPassing` screenshots in a stewardship sprint.

## Failure modes to watch

- **Tier-1 false-confidence on partial_render.** Haiku says "feature_visible, identifiable" when the rendered feature has a critical missing element only a tier-2 review would catch. Mitigation: tier-2 always re-classifies image_state as part of its completeness check; a tier-1 misread surfaces as a tier-2 disagreement.
- **Tier-1 misclassifying error_overlay as feature_visible.** E.g. an error toast at the bottom that Haiku doesn't notice. Mitigation: the dispatch prompt could explicitly ask "is there an error overlay, toast, or dialog visible? if yes, prefer error_overlay". Add to the agent doc.
- **Image-load fallback.** WebFetch on a binary content-type saves to a tmp path; Read on that path renders. If the WebFetch tool changes behavior, this pathway breaks silently. Mitigation: tier-1 returns `unreadable` with `confidence: low` when the WebFetch+Read pathway returns an empty/text-only result, and the orchestrator immediately escalates to tier-2.

## Out of scope (by design)

- **Multi-image comparison** (regression detection between two screenshots). That's a separate capability — could be tier-1 (Haiku categorical: "same | different in chrome | different in content") but deserves its own spec when we get there.
- **Pixel-level analysis** (color values, dimensions, OCR). Out of scope for any tier — that's image-processing-tooling work, not LLM judgment.
- **Continuous visual baselines** (golden master, visual diffing). Different problem space; spec elsewhere.

## Validation

This spec succeeds when:

1. The schema lands and the agentic-developer skill can dispatch tier-1 visual-triage without modifying ci-observer's existing modes.
2. Cost-per-screenshot drops by ≥ 3× vs. all-Sonnet baseline on a synthetic 50-screenshot sweep.
3. Tier-1 outputs are stable (the same screenshot returns the same `image_state` across N runs of Haiku) — verifiable with a small fixture corpus once the visual-validation sprint generates one.
4. The "blank vs partial vs feature_visible" three-way distinction is empirically discriminating on real Playwright screenshots from genesis pipelines (no degenerate "everything is partial_render").
