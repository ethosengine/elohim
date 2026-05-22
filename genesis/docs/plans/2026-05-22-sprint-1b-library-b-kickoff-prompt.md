# Sprint 1B — Library B Pattern Stories Kickoff Prompt

> **For the operator or a fresh session:** This is the launch prompt for Plan B. It picks up where Sprint 1A landed and authors the Library B designed pattern stories + Storybook integration that compose the elements into the convergent Qahal homepage demonstration.

## What just landed in Sprint 1A

**533 tests passing across two Lit element packages:**

| Package | Path | Elements | Tests |
|---|---|---|---|
| `elohim-qahal` | `app/elohim-elements/elohim-qahal/` | 23 elements (5 primitives + 4 chrome + 5 deep-impl panels + 4 visual-stub panels + 4 resource list sections + standing-ring) | 433 |
| `elohim-imagodei` | `app/elohim-elements/elohim-imagodei/` | 5 elements (setting-control + protected-tier-marker + steward-configure-banner + settings-palette composite + introspection-panel) | 100 |
| `graphos default/qahal/fixtures` | `app/elohim-library/projects/graphos/src/default/qahal/fixtures/` | 4 typed mock-data fixture modules | — |

Every element ships with: `.ts` source + `.spec.ts` behavior+a11y tests + `.manifest.spec.ts` CEM contract test + capability profile JSDoc (9 tags) + three precondition gates (a11y via axe-core, i18n via RTL render + logical-properties scan, ua-prefs via reduced-motion + measureLuminanceChanges). Conventions are locked in `app/elohim-elements/elohim-qahal/CONVENTIONS.md`.

Final Sprint 1A commit ladder: `4a4e00b6a → 48d04efad → 7332010c8 → d406fe32c → 3e153e2e6 → 3a1221274 → 1acbb4470 → 0abf9846d → 86ee5352f → d77c3f919 → 7f660e275 → 7ffba2201 → 55e0323b9 → ef61d7c07 → 2c6b9b3c8 → 866307639 → 1237d92c9 → ce03d75fb → 57947fa10 → f2bb6429c`.

## What Plan B authors

The Library B graphos pattern stories — the designed (themed) Storybook compositions that render the Qahal homepage against the Tier-0 worked examples (Dowell household, CofC congregation, Hardins life-group, wisdom commons federation), plus the variations + simple→power-user toggles + capability-gating views.

Plus the Storybook configuration with the `0.0.0.0` binding so Eclipse Che surfaces the endpoint at the existing storybook port (`6006` per devfile.yaml).

## Inputs to consume

Required reading before brainstorming Plan B:

1. **`/projects/elohim/genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md`** — Sprint 1 UX design spec (the gospel for the homepage shape, panel composition, capability gating, settings palette, mock-data fixtures structure). Section 10 enumerates the expected Library B story shape (canonical / variations / user-toggles / capability-gating).
2. **`/projects/elohim/genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md`** — the gospel-tier vision spec (Sections 1.2 + 4 + 7.6a are most relevant: household as living core, the four Tier-0 worked examples as concentric rings, lived contrast as diffusion mechanism).
3. **`/projects/elohim/genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md`** — the storyteller's 5,067-word canonical narratives. Library B pattern stories should faithfully render the named moments (Tuesday-morning Dowell scene with sick James + Sheila's soup + Gertrude's check-in; Sunday-morning CofC with Brother Cal + Romans 12 + youth retreat; Tuesday-evening Hardins life-group at three years cohesion; Thursday-afternoon wisdom commons with Brother Cal's concern surface).
4. **`/projects/elohim/genesis/docs/plans/2026-05-22-sprint-1a-elohim-elements-plan.md`** — Sprint 1A plan (what was built, file paths, capability profile patterns).
5. **`/projects/elohim/app/elohim-elements/elohim-qahal/CONVENTIONS.md`** — locked conventions Plan B should inherit when extending Storybook config or story templates.
6. **`/projects/elohim/app/elohim-library/projects/graphos/src/default/qahal/fixtures/`** — the 4 mock-data fixture modules (imagodei profiles, rubrics, care-economy events, social-compute topology). Library B stories consume these.
7. **`/projects/elohim/devfile.yaml`** — confirms storybook endpoint at port `6006`.

## Expected Library B deliverables

Per UX spec Section 10:

```
app/elohim-library/projects/graphos/src/designed/qahal/
  homepage/
    canonical/
      qahal-homepage-dowell-household.stories.ts        # Tier 0 — §4.1 narrative
      qahal-homepage-congregation.stories.ts            # Tier 0 — §4.2 narrative
      qahal-homepage-life-group.stories.ts              # Tier 0 — §4.3 narrative
      qahal-homepage-wisdom-commons.stories.ts          # Tier 0 — §4.4 narrative

    variations/
      qahal-homepage-household-with-toddlers.stories.ts
      qahal-homepage-household-multi-generation.stories.ts
      qahal-homepage-congregation-doctrinal-tension.stories.ts
      qahal-homepage-life-group-newly-formed.stories.ts
      qahal-homepage-life-group-departing-member.stories.ts
      qahal-homepage-wisdom-commons-reconciliation-recorded.stories.ts
      # additional variations per the UX spec's enumeration

    user-toggles/
      qahal-homepage-simple-user-view.stories.ts        # Dowell household, simple panel set only
      qahal-homepage-power-user-view.stories.ts         # Dowell household, all panels (toggle on)

    capability-gating/
      qahal-homepage-visitor-view.stories.ts            # Dowell household, viewer is a visitor
      qahal-homepage-engaged-view.stories.ts
      qahal-homepage-contributor-view.stories.ts
      qahal-homepage-steward-view.stories.ts
      qahal-homepage-protected-tier-view.stories.ts     # child / IDD / elder under guardianship
```

Plus Storybook configuration changes:

- Storybook config (likely Storybook v9 or v8 for Web Components) bound to `0.0.0.0` so the Che endpoint at `:6006` exposes it
- Brand-token theming applied via graphos design system (the Elohim brand surface)
- Story controls for: Qahal type selector, capability tier selector, simple/power toggle, provenance category toggle
- Documentation per story explaining what it demonstrates (the canonical narrative reference, the architecture surface being demonstrated)

## Variation seeds (from value-scanner content audit)

The value-scanner content audit (`/projects/elohim/genesis/docs/plans/2026-05-22-value-scanner-content-audit.md`) identifies ~1,700 lived-narrative scenarios across 21 archetypes that can seed variation stories. The 632 household-context scenarios are the deepest reservoir for Tier-0 household variations. Variation stories don't need to be exhaustive at MVP — pick 6–10 that demonstrate the architecture's reach across diverse households / congregations / life-groups / federation moments.

## Process

1. **Brainstorm with `superpowers:brainstorming` skill** — explore open questions:
   - Storybook version + framework (Web Components on Storybook v8 or v9? Check what graphos already uses, if anything)
   - Story rendering with Lit element composition (the `<elohim-qahal-collective-switcher>` + `<elohim-qahal-sidebar>` + `<elohim-qahal-main-viewer>` + `<elohim-qahal-context-column>` assembly)
   - Mock-data wiring (how do stories pass `MockCareEconomyEvent[]` to `<elohim-qahal-stream-panel>`? Via Lit property bindings in the story template)
   - Theme bindings — the Elohim brand tokens applied via graphos design system at the story decorator level
   - Per-story controls — Qahal type / capability tier / simple-power toggle
   - Variations strategy — which 6-10 to author + how to mine the value-scanner corpus for additional fixture content (transformation: extracting Gherkin-shape scenarios from `genesis/data/lamad/content/scenario-value-scanner-*` JSON if useful)

2. **Author the UX spec extension if needed** — if brainstorming surfaces architectural decisions that affect the UX spec (e.g., a previously-unspecified story-control surface), patch the spec via the brainstorming → spec → writing-plans cycle.

3. **Write Plan B with `superpowers:writing-plans`** — produce a task-by-task plan covering:
   - Storybook scaffolding (config, story-runner integration, brand-token wiring, `0.0.0.0` Che endpoint)
   - The 4 canonical stories
   - The variations (6-10 picks)
   - The user-toggle pair
   - The capability-gating stories (5 tiers)
   - Documentation sweep
   - Story-render acceptance verification (does each story render visibly in Storybook at the Che endpoint?)

4. **Execute with `superpowers:subagent-driven-development`** — fresh subagent per task, two-stage review (spec compliance + code quality), iterate.

5. **Demo at the end:** load the Storybook in a browser via the Che endpoint URL; confirm the convergent Qahal homepage renders against the Dowell household fixture; confirm the simple→power toggle works; confirm the capability-gating stories surface the correct affordances per tier.

## Open questions to resolve in Plan B brainstorming

Per UX spec Section 11 + Plan A's deferred concerns:

1. **Storybook framework choice** — graphos already exists at `app/elohim-library/projects/graphos/`; what does it currently use for rendering? Plan B inherits or extends that.
2. **Brand-token wiring** — how do the Elohim brand tokens flow from graphos to the Lit elements' CSS custom properties at story-decorator level? Likely via a Storybook decorator that wraps stories in a themed container.
3. **Story controls** — Storybook's argTypes system for the Qahal/tier/toggle selectors.
4. **Documentation patterns** — does graphos already have a documentation convention (MDX, JSDoc-generated, README-per-story)? Inherit it.
5. **CI hook for Storybook build** — does CI need to build Storybook as a static asset for each PR? Out of scope for Sprint 1B unless the operator wants it; flag for a future sprint.
6. **Variation story authoring vs Sprint 5 (a2o scenarios)** — Plan B's variations are visual/structural; Sprint 5's variations are behavioral (Gherkin .feature files). They're complementary — both should be authored eventually; Plan B handles the visual layer.

## What "done" looks like for Sprint 1B

- Storybook runs at the Che endpoint URL (port 6006, `0.0.0.0` bind)
- The 4 canonical stories render the storyteller's named moments faithfully (sick James + Sheila's soup + Gertrude's check-in in the Dowell story; Brother Cal + Romans 12 + youth retreat in the congregation story; etc.)
- The 6-10 variation stories cover meaningful edge cases
- The user-toggle pair demonstrates the simple→power UX gradient
- The 5 capability-gating stories demonstrate the standing-tier affordance differences (especially the protected-tier-view showing how external links are hidden + co-steward voice register shifts for child/IDD/elder)
- Operator can demo Sprint 1A + Sprint 1B together to a non-technical observer (the recognition + distinction test from the roadmap's Checkpoint F)

## What's deferred to subsequent sprints

- Real backend wiring (Sprint 2+ substrate work)
- Sprint 5 a2o scenario authoring (~21 new Tier-0 scenarios per the archaeology document)
- Visual-stub panels' deep backend (Sprint 6+ shefa, attestation, graph-discovery work)
- Tier 3 substrate-extension primitives (per the vision spec's 18-item endgame)

## Companion roadmap

The MVP roadmap (`genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md`) places Sprint 1B between Sprint 1A (just completed) and Sprint 2 (the substrate spine wire-definitions pass). Brainstorming Checkpoint B in the roadmap is the gate after Sprint 1B completes. This kickoff prompt introduces no new storage entities; Sprint 1B work is purely UI composition + Storybook configuration over the Library A elements that already exist.

---

**To launch Plan B:** start a fresh session, paste this prompt as the opening message, invoke `superpowers:brainstorming` to explore the open questions above, then proceed to `superpowers:writing-plans` and `superpowers:subagent-driven-development` per the standard flow.
