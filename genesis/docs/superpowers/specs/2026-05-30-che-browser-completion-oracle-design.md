# Che Browser Feedback — L2 Completion Oracle: Visual Done-Gate for `/shift` + `/deliver`

> Spec 2 of 2. Builds on L1 (`2026-05-30-che-browser-feedback-foundation-design.md`), which makes
> a headless browser launch in Che and ships the `look` primitive. **Do not start L2 until L1 has
> landed and verified** — L2 consumes L1's `shot.png` + `capture.json` directly.
>
> The thesis: the rendered experience is not just a tool the agent *may* reach for — it becomes a
> **frozen-at-kickoff judge that gates "done."** A shift loops until the goal is *visibly*
> accomplished, not merely until CI is green.

## Why now — CI green ≠ visible delivery

The 2026-05-30 overnight cascade (7 layers, same defect class) ended with green pipelines while
standalone bundles still white-paged on bootstrap. `/deliver` exists precisely because "CI green ≠
human-visible delivery." Today that visual falsifier lives only in `/deliver`, run *after* a sprint
is already declared complete. The operator's insight: pull the falsifier **into** the `/shift`
iteration loop so a shift cannot call itself done while the screenshot contradicts the promise —
and establish the baseline **formally at kickoff** so the loop knows what "done" looks like before
it starts grinding.

## The sockets already exist (this is integration, not invention)

The agentic-developer (`/shift`) architecture already anticipates this:

- **The Objective's `measure` command *is* the judge.** "Done is stable: two consecutive passing
  measurements, at least one fresh trigger." "You may not edit the judge." (`agentic-developer`
  skill.) The judge freezes at kickoff.
- **Kickoff already asks for a baseline** — *"What's the baseline floor — the measurement we must
  not drop below?"* — but today that baseline is a **number** (`objective.schema.json` → `baseline`
  is `{predicate, value}`).
- **`/deliver` already runs a two-render-stable tier-3 verdict** (`delivered`/`partial`/
  `error_state`/`missing`) of `shot.png` vs a structural `what_should_be_visible` FeaturePromise,
  citing `plan_deliverables` verbatim.

L2 wires these together: the kickoff baseline gains a **rendered** form, the Objective gains a
**visual goal**, and `/deliver`'s verdict becomes a **gate inside `/shift`'s done-definition**.

## Settled design decisions (from brainstorming 2026-05-30)

| Decision | Choice | Rationale |
|---|---|---|
| Done-gate shape | **Composite** | Done = CI measure stable **AND** visual verdict `delivered`. Two independent falsifiers, both required. Existing numeric judge untouched. |
| Applicability | **Conditional** | The Objective `visual` block is **optional**. Present → gate runs. Absent → the loop behaves *exactly* as today. Backend-only shifts (zome validation, CI dispatch efficiency) are unaffected. |
| Cadence | **Kickoff baseline + done-candidate gate** | Render at kickoff (the "before"); run the verdict only when the CI measure goes to done-candidate. Two-render-stable like `/deliver`. `look` stays available for opportunistic mid-loop glances, but the *gate* fires at done-candidate — iterations stay scarce. |
| Baseline role | **Context, not pixel-diff** | The "before" render is qualitative context + the goal is the forward target. Judgment is structural (`what_should_be_visible`), never pixel comparison — pixel diffing is flaky and would poison the gate. |
| Goal source | **Hybrid: derive, else author** | If the Objective references a feature with an existing `/deliver` FeaturePromise, reuse its `what_should_be_visible`. Else the kickoff interview authors an inline goal. Reuse where possible; author for greenfield. |
| Verdict ownership | **Shared procedure** | Factor the tier-3 screenshot-vs-promise verdict so `/shift` and `/deliver` call the *same* judge. No duplication; `/deliver`'s local pathway (alive via L1) benefits too. |

## Design — Part A: Objective schema gains an optional `visual` block

`.claude/schemas/objective.schema.json` (`additionalProperties: false`, so this is an explicit,
**not**-`required` property — absence = today's behavior):

```jsonc
"visual": {
  "type": "object",
  "additionalProperties": false,
  "required": ["surfaces", "goal"],
  "properties": {
    "surfaces": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["name", "url"],
        "properties": {
          "name":       { "type": "string", "pattern": "^[a-z][a-z0-9-]{1,40}$" },
          "url":        { "type": "string", "minLength": 1 },
          "as":         { "type": "string" },         // fixture human for --as
          "waitTestid": { "type": "string" }          // --wait-testid
        }
      }
    },
    "goal": {
      "type": "object",
      "additionalProperties": false,
      "required": ["source", "whatShouldBeVisible"],
      "properties": {
        "source":     { "enum": ["feature-promise", "inline"] },
        "featureRef": { "type": "string" },            // feature slug when source=feature-promise
        "whatShouldBeVisible": {
          "type": "array", "minItems": 1,
          "items": { "type": "string", "minLength": 1 }
        }
      }
    },
    "stability": {                                      // visual-verdict stability; mirrors target.stability
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "consecutive":     { "type": "integer", "minimum": 1, "default": 2 },
        "across_triggers": { "type": "boolean", "default": true }   // ≥1 render against a FRESH build/deploy, not a re-screenshot of unchanged state
      }
    }
  }
}
```

`whatShouldBeVisible` entries are **structural** ("the shefa balance card shows a numeric balance
and a provision button"), never transcribed pixel text — same discipline as `/deliver`'s
`screenshot_targets`.

## Design — Part B: kickoff ritual (agentic-developer skill + `/shift` command)

Inserted into the existing kickoff sequence (after the Objective interview, before the first
iteration). **Runs only if the operator declares a visible surface for the objective.**

1. **Establish the goal (hybrid).** If the objective references a feature with an existing
   `/deliver` FeaturePromise, derive `whatShouldBeVisible` from it (`source: feature-promise`,
   record `featureRef`). Else the interview authors the goal inline (`source: inline`) — short,
   pointed: *"Which surface(s)? At what URL? What must be visible there when this is done?"*
2. **Render the baseline.** For each surface, run L1 `look <url> [--as] [--wait-testid]
   --out baseline`. Store under `.claude/shifts/<shift-id>/baseline/<surface>/{shot.png,capture.json}`.
   This is the formal "before."
3. **Freeze.** Write the `visual` block into `.claude/shifts/<shift-id>.objective.json` and record
   goal + baseline paths in the journal header next to the measure. Goal and baseline are now
   immutable for the shift — *you may not edit the judge.*
4. The implementer `Read`s the baseline `shot.png` and states the starting visual state in the
   kickoff stanza (so the before→after gradient is legible from iteration 0).

## Design — Part C: iteration-loop change (the gate)

The existing loop (ground → observe → verify → act → measure → judge → journal) is **unchanged**
until the CI `measure` reaches a **done-candidate** (one passing measurement). Then:

1. **Render.** For each surface, `look <url> [--as] [--wait-testid] --out iter-<n>`. Archive under
   `.claude/shifts/<shift-id>/renders/iter-<n>/` so every iteration's render is retained.
2. **Verdict.** Opus runs the **shared tier-3 procedure**: judge each `shot.png` against the
   surface's `whatShouldBeVisible`, citing each goal line verbatim. Per-surface verdict ∈
   `{delivered, partial, error_state, missing}`. Roll up: the shift's visual verdict is `delivered`
   only if **every** surface is `delivered`. `capture.json.pageErrors`/`failedRequests` non-empty →
   cannot be `delivered`.
3. **Composite done.** The shift is **done** iff:
   - CI measure is stable per `target.stability` (≥2 consecutive, ≥1 fresh trigger), **AND**
   - visual verdict is `delivered`, stable per `visual.stability` (≥2 consecutive `delivered`
     verdicts, with ≥1 rendered against a **fresh build/deploy** — not two screenshots of the same
     unchanged artifact; the tired-agent flake guard from `/deliver` applies). In practice the
     visual gate's "fresh" render rides the same fresh CI trigger that confirms the numeric measure.
4. **Not done → continue.** If the verdict is anything below `delivered`, the shift is **not** done
   even with green CI. The verdict's gap (which `whatShouldBeVisible` line failed, which
   `pageError` appeared) becomes the next iteration's hypothesis. Loop within budget.
5. **Regression floor.** If a render that was previously `delivered` drops to `partial`/worse, treat
   it like dropping below the numeric `baseline` floor — surface it loudly; it blocks done.

### Done state machine (surface declared)

```
CI measure passes ─▶ done-candidate
        │
        ├─ render + verdict ──▶ delivered?  ── no ─▶ gap → next hypothesis (loop)
        │                          │ yes
        │                          ▼
        └─ CI stable (2,fresh) AND visual stable (2 renders,fresh) ─▶ DONE
```

## Design — Part D: sprint-result artifact

The single sprint-result markdown gains a **Visible Delivery** section: baseline `shot.png`, final
render `shot.png`, the cited verdict, and the before→after narration. This is the notarized
visible-delivery proof (mirroring `/deliver`'s evidence shape). For backend-only shifts (no
`visual` block) the section is simply absent.

## Files touched

| File | Change |
|---|---|
| `.claude/schemas/objective.schema.json` | + optional `visual` block (Part A) |
| `.claude/skills/agentic-developer/SKILL.md` | kickoff ritual (Part B); done-gate in the iteration loop + "Done is stable" definition (Part C); sprint-result Visible Delivery section (Part D) |
| `.claude/commands/shift.md` | surface the visual-goal questions in the Objective interview |
| `.claude/skills/deliver/SKILL.md` | point the tier-3 verdict at the **shared** procedure; note the local pathway now works in Che via L1 |
| shared verdict helper (new, e.g. `.claude/scripts/visual-verdict.md` or a documented procedure) | the one tier-3 screenshot-vs-promise judge both skills call |
| `genesis/docs/shifts/JOURNAL-TEMPLATE.md` | header fields for goal + baseline; per-iteration render slot |
| `pnpm run agentic:readiness` (objective validator) | accept + validate the `visual` block |

## Verification

1. **Backend-only regression:** an objective with **no** `visual` block runs the loop *byte-identical*
   to today (no render, no gate). Prove first — this is the safety property.
2. **UI shift, happy path:** an objective with a `visual` block + a known-good alpha surface →
   kickoff renders a baseline; at done-candidate the verdict reads `delivered`; composite done fires
   only after both stabilities are met.
3. **Gate actually blocks:** point the goal at a surface known to be broken (or use a stale bundle) →
   CI green but verdict `partial`/`missing` → the loop does **not** declare done; the gap appears as
   the next hypothesis.
4. **Hybrid goal:** an objective referencing a feature with an existing FeaturePromise derives
   `whatShouldBeVisible` without inline authoring; a greenfield objective authors inline.
5. **Shared verdict:** `/deliver` and `/shift` produce the same verdict shape on the same
   `shot.png` + goal (no divergence).

## Out of scope (L2)

Pixel-diffing; scriptable mini-flows; multi-tab/Tauri rendering; image-bake (L1 follow-up);
auto-generating FeaturePromises for objectives that lack them (author inline instead); changing the
numeric measure semantics (it stays exactly as-is — L2 only *adds* a co-equal gate).
