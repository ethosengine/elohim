---
name: ux-walk
description: Persona walk of a human-facing surface — drive the deployed or local app as a persona (anonymous or a signed-in fixture human) with the click-probe and look rails, judge it by cognitive walkthrough + Nielsen heuristics, and write one append-only walk doc per view whose `- [ ]` stations become gap ids a fixer swarm claims, fixes and fulfils. Use after an app change ships, for a habit whose checks name an @browser-only feature, or when 'clicks do nothing', 'styling looks off', 'navigation feels weird', 'find the fuzzy incoherence', 'walk the app as X'. One attestation layer, never acceptance; the walker deletes its screenshots and leaves text only.
metadata:
  sourceRuntime: elohim-agent
  master: package
  governance: "epr:elohim-agent/skills/ux-walk"
---

# UX Walk

A persona walks a human-facing surface the way a person would. Every place where the experience breaks its promise becomes a narrow, fully documented station that a fixer swarm can claim, fix and attest. The aim is that the operator, logging in after a feature ships, finds nothing. Agents feel the pain first.

The method combines **exploratory testing** (a charter, not a script), **cognitive walkthrough** (at each step: would the persona know what to do, see the control, understand the result?) and **Nielsen's heuristics** (one tag per finding). In VSM terms it is System 3*: a sporadic audit that looks at operations directly instead of trusting their self-reports. CI can be green while the app is incoherent; this is the channel that sees it.

It is **one attestation layer, not acceptance.** A walk produces stations. Fixes discharge them with fulfilments. Nothing here is a human sign-off. Appointed experiential acceptance (`epr flow note --purpose acceptance`) stays a separate, optional rung that may cite a walk doc.

## When to walk

- After an app change reaches a deployed or local surface: a branch finish that touched `app/`, an app deploy to alpha, or a `/deliver` pass.
- For a habit with a human surface, meaning its `checks:` name an `@browser-only` a2o feature. Today that is epr-atom-home, served-under-standing, hosted-human-lifecycle, hosted-cell-promised, doorway-failover, runtime-death-witnessed and witness-rows-signed. The habit's `invariant:` is the walker's goal, and the feature's `Given human "X"` is the persona.
- When a walk doc's coverage gaps name a next charter.

## Instruments (from `genesis/a2o`)

- `pnpm -s probe <url> [--as <FixtureHuman>] --out <slug> [--max 40] [--budget 240] [--viewport 390x844]`: signs in, enumerates everything that invites a click (links, buttons, ARIA widgets, `cursor:pointer`, through open shadow roots), then clicks each one from a fresh load. Outcomes are `navigated / in-page / PIXELS-ONLY / NO-EFFECT / UNCLICKABLE / popup / skipped`. `PIXELS-ONLY` and `NO-EFFECT` need the walker's eyes on the click shot. `cursorOnly: true` marks an affordance with no semantics. The run also records console, page and HTTP errors on load and per click, duplicate nav, external link statuses, and same-origin `links` to crawl next. A hard `--budget` plus a watchdog guarantee a report, marked `truncated` when cut short. Destructive labels are never clicked.
- `pnpm -s look <url> [--as …] [--scheme light|dark] [--scroll] [--click-testid <id>]`: one render with a capture.
- Run one browser at a time; workspace RAM is tight. Read-only on shared alpha: no typing, no submits. The probe skips create, send, join, vote and similar labels; don't hand-click around that.
- Render before judging. A probe outcome is evidence, not a verdict.

## Finding classes

Tag each finding with ONE heuristic: H1 system status · H2 real-world language · H3 control and freedom · H4 consistency and standards · H5 error prevention · H6 recognition over recall · H7 flexibility · H8 minimalist design · H9 error recovery · H10 help. Severity: **BLOCKS** (the persona cannot proceed), **CONFUSES** (could misread or get lost), **POLISH** (looks off). Owner class: `component-fix`, `design-decision`, or `substrate:<layer>` (the cause is below the UI, for example a session that does not cross a bundle seam). Fuzzy incoherence matters most: "this felt wrong", even when nothing technically failed.

## Output: one walk doc per view

`genesis/a2o/walks/<view-slug>.md`, in a flat directory. `epr flow project` reads it as a plan (`.claude/epr-meta/recipes.yaml`, plan stage), so every `- [ ]` becomes a gap station `walks__<view-slug>#<n>`. **Ids are positional, so the doc is append-only.** Stations are ordered once (BLOCKS, then CONFUSES, then POLISH), later walks append, fixers tick, and nobody reorders or deletes. Use no checkboxes other than stations.

```
---
view: <route pattern or decision name>
urls: [<concrete URLs walked>]
personas: [anonymous | <FixtureHuman> (signed in)]
surface: <owning component/source files>
habit: <habit id or none>
walked: <date> · <target> · <walker charter(s)>
recheck: cd genesis/a2o && pnpm -s probe <url> [--as X] --out recheck-<view-slug> --budget 180
---
# <View>: persona walk

<2-3 lines: what this view is for, as the persona experiences it, and whether it delivers that.>

## Stations

- [ ] <SEVERITY>·<Hn> <one-line imperative title of the fix>
  - Persona / route: …
  - Repro: numbered steps from a cold load
  - Expected (as persona): …
  - Observed: the fact in words (status, console line, visible text). Start with "Verify first:" when environment-suspect.
  - Source: file:line, or the grep that finds it
  - Owner class: component-fix | design-decision | substrate:<layer>
  - Depends on: <station ids> | none
  - Done when: an observable check (usually the recheck plus what must now be true)
```

A station is self-contained: a fixer reads only it and the frontmatter. A pattern that spans views gets one doc named for the decision (for example `pillar-nav-coherence.md`). A substrate cause gets one doc named for the layer, and its stations are investigations with a concrete first probe.

## The walker owns its evidence: no loose ends

Screenshots are expensive scratch, never the record. Before reporting done, the walker:
1. writes or appends the walk doc(s) in text: every observed fact in words, never "see screenshot";
2. deletes its own capture directories (`genesis/a2o/reports/{probe,look}/<its slugs>*`) and verifies with `ls … | grep <slug>` returning nothing;
3. reports the doc paths, station counts by severity, and the coverage it did not reach (the next charter).
A walk that leaves image directories behind, or hands a parent "go look at these shots", is unfinished.

If the harness refuses a subagent's file write, the walker returns each doc's full text under `=== FILE: <path>` lines and the dispatcher places them. It never routes around the refusal with shell redirects.

## Burn-down: a fixer takes one station

1. `epr flow claim --on walks__<view>#<n> --as agent:<role>@<model> [--serves <habit-id>]`
2. Fix within the station's owner class. A `design-decision` station is decided where the station says, which is not the fixer's call when it is a value or product choice: surface it to the conveyor's ceiling menu instead.
3. Run the station's **Done when** check, normally the doc's `recheck`, judged against the station. Compare before and after in words.
4. Tick the box (`- [x]`). This is a claim awaiting verification, not evidence.
5. `epr flow fulfill --on walks__<view>#<n> --report <text report> --status DONE|DONE_WITH_CONCERNS`. The report holds the before/after facts, the recheck outcome and the commit. This fulfilment is the attestation.
6. Delete the recheck's captures.
A recheck that surfaces a NEW issue appends a new station to the same doc. It never widens the claimed one.

## Dispatching walkers and fixers

Subagents do not discover skills. Every dispatch prompt must carry the instruments, the doc format, the read-only rules and the evidence-ownership duty above, or point at `.claude/skills/ux-walk/SKILL.md` and say "follow it exactly". Give each walker ONE charter: a persona, a goal (a habit's invariant when there is one), and a starting territory. Use one charter per agent, and one agent per station on burn-down. Fixers touching the same component run in sequence, not in parallel.

## Integration

- `looking-at-frontend`: the render rails and their blind spots (no emoji font, no WebGL2 in headless).
- `/deliver`: the `@elohim-visually-validated` tag on a headline scenario is meaningful when that scenario's view has no open BLOCKS station.
- `delivery-stasis`: open `walks__*` stations are gap stations, counted in the backlog the conveyor drains.
- `story-harvest`: a fixed BLOCKS station that guards a real regression is a candidate a2o scenario.
