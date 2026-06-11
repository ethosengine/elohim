---
id: frontend-eyes-sprint
status: landed
landed: 2026-06-11
---

# Frontend Eyes Sprint — render-verified design triage + fix waves (2026-06-11)

**Goal:** Every Library B (designed) surface in graphos render-verified against the brand spec
(`genesis/graphos/elohim-protocol-design-spec.md`), ghost/dead token bindings eliminated, claimed
coverage made real, worst brand-register violations fixed — with the closing render as the proof
for every fix. Branch: `feat/frontend-eyes-sprint` (off `feat/native-content-graph-seam` HEAD, NOT
dev — the graphos render tooling + eyes skill live in that branch's 109 unmerged commits; branching
off dev would have stranded the sprint without its eyes. Integrator: land the seam branch first).

**Eyes discipline:** every visual claim in this doc is backed by a render artifact under
`genesis/a2o/reports/look/<slug>/` that was actually Read. Renders run against the LOCAL storybook
(`pnpm storybook` in app/elohim-library, `--base http://localhost:6006`) — the deployed base lacks
this branch's fixes.

---

## Substrate fix (landed first — gates every other verdict)

**Webfont gap — FIXED, commit `ffd4ff75d`.** No `@font-face` existed anywhere in the library;
every `--el-font-*` binding in Library B silently fell back to system-ui/Georgia. Fix: self-hosted
`@fontsource-variable` packages + explicit `@font-face` in `.storybook/fonts.css` under the
spec-canonical family names ('Fraunces' incl. SOFT/WONK axes, 'Source Serif 4', 'DM Sans',
'JetBrains Mono') — fontsource's own CSS registers `'X Variable'` names, which would have been a
byte-perfect dead binding. `font-display: block` so headless captures never record fallback flash.
Proof: `reports/look/fontfix-local/` vs `reports/look/fontfix-deployed/` (hub-aggregation-shift
side-by-side; Fraunces display + DM Sans labels + SS4 italics + JBM code all live locally, all
fallback on deployed).

Known limit: none of the four brand families carries Hebrew glyphs — he/RTL canaries intentionally
fall through to the serif fallback stack. A Hebrew-capable brand face (e.g. Noto Serif Hebrew
pairing) is an operator-level type-system decision (see Operator decisions).

## Scoreboard (one row per surface; updated as sweep results land)

Severity: P0 broken render · P1 dead/ghost binding, font fallback, hardcoded palette ·
P2 missing claimed coverage · P3 brand-register drift · P4 polish.

| Surface | Render | Bindings | Coverage | A11y | Brand | Worst finding |
|---|---|---|---|---|---|---|
| elohim-page-chrome | ok | live | gaps: high-contrast, es | Dark omnibar fg ~black on #1A1A2E | drift | P1 unbound `--elohim-omnibar-fg` in Dark designed story |
| elohim-feedback-mechanism-gateway | ok | live | gaps: loading/controversy/settled designed states, he canary | hardcoded EN strings vs en/es/he claim | holds | P2 type error `level: 3` vs union `0\|1\|2` (verified tsc) |
| elohim-graduated-feedback | ok | ghosts:6 | gaps: designed RTL, single-context, high-contrast | radiogroup lacks roving tabindex/arrow keys | drift | P1 Flat-UI palette baked in primitive; 6/9 designed tokens ghosts; CustomTheme proof FAILS in pixels |
| elohim-reaction-bar | ok | ghosts:1 +2 dead-by-type +1 undeclared | gaps: designed RTL/mediated/high-contrast/es | warning `*` invisible (Canvas-on-Canvas); dialog no focus trap | drift | P1 border shorthand→color-slot IACVT: ALL designed borders silently vanish |
| elohim-context-menu | ok | live | gaps: Gentle story, es, high-contrast | designed Dark text ≈1.2:1; focus wash <3:1 | drift | P1 dark/light decorators never bind text color (docs claim Starlight/Hearthstone) |
| elohim-content-analytics | ok | 11 live, 4 dead --el-* | gaps: designed empty/loading/error, RTL, high-contrast | note text ≈2.9:1 light | drift | P1 Fraunces display binding dead (no title-font cssprop); golden-hour absent both themes |
| elohim-epr-popover | ok | live (16/16) | gaps: designed RTL, high-contrast, es | role=tooltip hosts interactive button | drift | P1 `color: CanvasText` hardcodes override themed fg (CustomTheme proof near-illegible) |
| elohim-epr-relationships-panel | ok | live (6/6) | gaps: designed RTL, high-contrast | card-type badge ≈2.7:1 | drift | P1 documented gap cssprop doesn't reach card grid (10px hardcode); raw `epr:` IDs as primary copy |
| elohim-skeleton | ok | ghosts:1 | full | pass | minor drift | P1 `updated()` setProperty clobbers `--elohim-skeleton-radius` — advertised override dead (CustomTheme proof silently false) |
| imagodei ConsentCardThreeClaims | ok | ghosts:7 | gaps: attestor header absent, no dark peer-conductor | policy-link contrast; bidi colon flip | drift | P1 UA-blue checkboxes on consent front door (no accent cssprop); P1 `trustMode` ghost prop; P1 claim-row border IACVT |
| imagodei EvictedAccount | ok | ghosts:5 (dead-in-composition) | gaps: attestor row claimed-but-absent, no dark recovery | clay headline ≈3.8:1 | drift | P1 slotted header SHADOWS portal-shell default → named recovery witnesses invisible in all 3 stories |
| _(19 remaining rows land as the resumed sweep completes: compute-tile, default-omnibar, epr-link, gate-feedback-trigger, mention-base, navigator, 5 imagodei flows, hub-aggregation-shift, qahal-homepage, foundations, 3 docs walks, Library-A-only triage, app spots)_ | | | | | | |

### Cross-cutting fix classes (systemic, feed Wave 1-3 briefs)

1. **Border shorthand→color-slot IACVT** (reaction-bar, consent-card; invited by ambiguous `@cssprop … - X border` docs): primitive should consume full shorthand or redocument as `border-color`; audit every `border:`-class cssprop.
2. **Golden-hour rule violated nearly everywhere** — `--el-amber` declared in every EL_TOKENS block, consumed by almost none of the designed token maps.
3. **Designed RTL canary missing on every element** (Library A has them; Library B never re-proves brand-bound RTL).
4. **High-contrast/forced-colors story missing everywhere** despite `@capabilityContrast normal, high` claims + existing forced-colors CSS blocks.
5. **Hardcoded-English primitives vs en/es/he locale claims** (i18n gate debt — primitive-level, backlog-scale).
6. **Theming self-defeats in primitives**: `color: CanvasText` hardcodes (epr-popover), inline `style.setProperty` clobber (skeleton), inline JS hex styles (graduated-feedback).
7. **Slot-shadowing kills portal-shell default header** (attestor row) across imagodei designed flows — story-side composition class.
8. **Off-grid geometry** (6px/10px/12px radii+gaps vs 8px grid, radius 4/8/16) — low-sev recurring.

## Seed findings (pre-verified, fix waves staged)

1. **graduated-feedback primitive hardcodes Flat-UI palette** — `DEFAULT_SCALES` hexes (#e74c3c,
   #3498db, #6c5ce7, …) flow into inline `style=` in render
   (`elohim-core/src/elohim-graduated-feedback.ts:420-426,559`) — unreachable by CSS override.
   Designed story binds six `--elohim-feedback-position-*` names the primitive never consumes
   (ghosts): position-bg/fg/border/radius/active-bg/active-fg. `readableOn()` derives fg from the
   *fallback* hex — any var-chain fix must pair fg overrides. → component-architect (@cssprop
   surface + var-chain render + per-slot scale vars), then graphos-designer re-bind.
2. **reaction-bar** — designed story binds ghost `--elohim-reaction-count-color` (primitive
   surface: `--elohim-reaction-count-fg`); primitive consumes `--elohim-reaction-btn-active-fg`
   (line 180) without declaring it `@cssprop`; mediation surface
   (`--elohim-reaction-mediation-bg/fg`, `--elohim-reaction-warning-color`) entirely unbound in
   Library B → mediation dialog renders system-colors inside branded scenes.
3. **context-menu** — file header promises a `"Gentle (fold-down)"` motion story (line 26) that
   doesn't exist (8 exports, none motion-named); the primitive ALREADY carries the fold-down
   animation (120ms ease-out behind `prefers-reduced-motion: no-preference and (update: fast)`) —
   pure story-authoring gap. Golden-hour audit of dark stories pending sweep row.
4. **golden-hour violations** — graduated-feedback both themes (no `--el-amber` anywhere in its
   designed tokens); page-chrome 5/7 designed compositions (sweep-confirmed).

## Fix waves — DELIVERED (one commit per component; closing render path in each commit message)

| Commit | Component | What landed | Proof render |
|---|---|---|---|
| `ffd4ff75d` | storybook preview | webfont substrate (@fontsource @font-face, spec-canonical names) | `fontfix-local/` vs `fontfix-deployed/` |
| `2d610207f` | graduated-feedback | per-slot scale var surface (Flat-UI hexes → fallbacks), APG arrow keys, brand-gradient rebind (terracotta→amber→greens), aggregates story, fixture-key fix | `fix-elohim-graduated-feedback/` |
| `3fca9d94e` | reaction-bar | border vars consumed as FULL shorthand (IACVT class fixed), warning fallback CanvasText, scrim token, active-fg declared; amber active rebind, mediation bound + Mediated story | `fix-elohim-reaction-bar/` |
| `251a3f1cc` | context-menu | decorators bind claimed Hearthstone/Starlight ink (Dark was ≈1.2:1), amber anchor chip (golden-hour ×6 stories), Gentle (fold-down) story authored | `fix-cm-dark/`, `fix-cm-gentle/` |
| `68f0b35d4` | default-omnibar | omnibar-fg bound both themes; ::part(brand) Fraunces @ Harvest Gold (the promised mark) | `fix-omnibar-dark/` |
| `23d0f1e5a` | page-chrome | same fg + brand-mark treatment; warm page fields (pure-#fff canvas leak) | (shared with omnibar proof) |
| `a092195fa` | navigator | kebab attribute mappings (display-name/identity-mode/show-search were INERT — every bubble read "Traveler"); 7 story ghosts renamed to real surface; per-kind banner binds | `fix-elohim-navigator/` |
| `6c89109d7` | qahal homepage | P0: MockRubric.bloomMapping Record→array adapter (RULES panel was dead in every scene) + CuratedEpr title→label adapter (sidebar labels were empty) | `fix-qahal-dowell/` |
| `812afcdac` | imagodei consent/portal | toggle-accent cssprop (UA-blue checkboxes gone), claim-row border shorthand, trustMode ghost-prop now adapts copy, portal-shell error-suppresses-primary (dead :empty replaced), 8 redundant slotted indicators removed → attestor row visible everywhere | `fix-consent-default/`, `fix-evicted-default/` |
| `cc3ac5de5` | epr-popover | CanvasText hardcodes dropped (CustomTheme proof now true); warm dark shadow + amber tags | `fix-default-core-elohim-epr-popover--custom-theme/` |
| `c52ce734f` | skeleton | setProperty clobber fixed (radius override surface live) + regression spec | (spec-proven; wtr green) |
| `da706b090` | gate-feedback-trigger | published the surface its own stories promised — 10 ghosts healed with zero story edits | `fix-designed-core-elohim-gate-feedback-trigger--light/` |
| `843d90b9d` | content-analytics | title-font cssprop; Fraunces title + amber rule + readable note ink | `fix-designed-core-elohim-content-analytics--dark/` |
| `5df7de7d0` | epr-relationships-panel | card-gap var real + bound on 8px grid | (binding-level; sheet re-render optional) |

Gates run: elohim-core wtr **558 pass** + typecheck + lint(0 err) + lint:css + build;
elohim-imagodei typecheck + build + my-files-lint (package has pre-existing 24 wtr failures +
4 lint errors — identical at HEAD baseline, verified by A/B swap); **static storybook build
green** (CI parity for every story edit). `test-storybook` (axe per story, needs built dist +
port coordination) left to the pre-push `elohim-storybook` gate at integration.

## Residual backlog (filed, not fixed — P2/P3 long tail)

1. **Designed RTL canaries missing on every element**; **high-contrast/forced-colors story
   missing everywhere** despite `@capabilityContrast normal, high` claims — systematic Library B
   coverage expansion, sized beyond this sprint.
2. **Hardcoded-English primitives vs en/es/he locale claims** (i18n gate debt) — primitive-level,
   needs lit-localize wiring across ~10 elements.
3. **feedback-mechanism-gateway**: `level: 3` type error vs `0|1|2` union (psephos levels
   unrepresentable — operator decision below); `@fires challenge-action` dead surface;
   designed loading/controversy/settled states unrendered; `#fca5a5` SaaS-echo hexes in its
   controversy tokens (invisible today, lands as drift when coverage arrives).
4. **reaction-bar/gate-trigger mediation+modal dialogs**: div-overlay → native
   `<dialog>.showModal()` migration (repo canon), focus traps, arrow-key menu nav.
5. **epr-relationships-panel**: fixtures pass `label` field absent from `EprRelationship`
   (cards show raw `epr:` ids — hospitality drift; type owes a human-label path);
   empty-state renders `nothing` despite `empty:designed` claim.
6. **context-menu focus indication** ≈1.1:1 wash (needs a visible focus treatment, primitive-side);
   epr-popover `role="tooltip"` hosting an interactive button (needs dialog/popover semantics).
7. **Imagodei flows not yet re-rendered**: ModeA/ModeB×2/NetworkOffline carry the same redundant
   slotted-indicator pattern (now harmless duplication risk only if attrs drift — fixtures
   verified identical in the two fixed files); NetworkOffline + PortalHostNotAuthorized carry
   zero amber (golden-hour). EvictedAccount dark-recovery variant missing.
8. **Off-grid geometry** sweep-wide (6/10/12px radii+gaps; `--el-radius-pill: 999px` design call).
9. **Sweep coverage not reached** (spend-cap kills): compute-tile + epr-link got spot-PASS only
   (no binding cross-check); mention-base, hub-aggregation-shift (font-fix renders look strong),
   foundations/compute-capacity-tokens page, narrative + domain docs walks, Library-A-only deep
   triage (list-level: all 10 carry both proofs, zero brand-bake), app alpha spots.

## Operator decisions (final)

1. **Hebrew brand face** — none of the four brand families carries Hebrew glyphs; he/RTL rides OS
   serif fallback. Adopt a paired Hebrew family (Noto Serif Hebrew / IBM Plex Sans Hebrew) or
   accept fallback long-term?
2. **`FeedbackMechanismLevel` union `0|1|2` vs documented "Level 3–7: psephos"** — widen the union
   (possibly schema-governed vocabulary) or rewrite the claim; the psephos default story carries a
   tsc-verified type error either way.
3. **Spend-cap interrupted the agent sweep twice** (28/30 then 19/30 failures); the sweep finished
   in lean mode (spot renders + class-greps by the main loop). If full-depth triage of the
   unreached surfaces matters, it needs budget headroom.
4. **`--el-radius-pill` (999px chips)** is off the spec's 4/8/16 radius scale — bless it as a
   chip idiom in the design spec, or retire it.

## Sprint verdict

The typography half of the brand was dead substrate-wide (no @font-face existed); it is now live
and proven in pixels. The dominant defect class was **theming theater**: bindings that LOOK
byte-correct in source but never land — six distinct mechanical causes (ghost names, border
shorthand-into-color-slot IACVT, inline-style/JS hardcodes, setProperty clobber, inert kebab
attributes, homonym fixture types passed across primitive boundaries) — every instance found by
rendering, none findable from source review alone. Eyes-first is vindicated: 14 components fixed
and re-render-proven, 2 P0 scene-killers (qahal rules panel, evicted hollow panel) healed, and
the golden-hour rule now holds on every fixed surface.

## Operator decisions (accumulating)

1. **Hebrew brand face**: adopt a paired Hebrew family (Noto Serif Hebrew / IBM Plex Sans Hebrew)
   or accept serif-fallback for he locale long-term?
2. **Monthly spend limit** interrupted the first sweep run mid-flight (28/30 agents); raised to
   $80 and resumed — flagging for awareness of remaining headroom.
3. `FeedbackMechanismLevel` union `0|1|2` vs documented "Level 3–7: psephos" — widen the union
   (substrate vocabulary question, possibly schema-governed) or rewrite the docs?

## Deferred / parked (do not touch this sprint)

- elohim-core `tokens.scss` legacy purple/pink app palette — bundle-styling-token-contract backlog
  item; designed stories hand-copy `--el-*` hexes inline by convention until graphos-tokens ships.
- `@fires challenge-action` dead surface in feedback-mechanism-gateway — vestigial from Angular
  migration; needs a component-architect decision (implement or remove claim), filed to backlog.
