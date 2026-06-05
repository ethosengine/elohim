# Bundle styling / token contract — the canonical layer the lamad split deferred

**Status:** Captured (2026-06-05 styling-migration audit) · **Priority:** medium-high (fires again on the NEXT pillar split)

## The gap
- `elohim-core/tokens.scss` holds the harvested `--lamad-*` token set but no bundle imported it until lamad's B18 wiring (2026-06-05). The next pillar bundle (shefa/qahal/avodah) inherits the same hole: the pillar-bundle-split runbook's bundle-creation checklist has NO styling/token section (§4.X unwritten).
- The Lit chrome's `--elohim-*` @cssprop surface has no canonical shipped binding — Library B binds tokens in Storybook decorators only; `compute-capacity-tokens.designed.mdx` explicitly anticipates "a global graphos-tokens.css" that does not exist. lamad's chrome-binding block in `app/lamad/src/styles.scss` is an interim, per-bundle copy.

## The work
1. **graphos ships a token artifact** (e.g. `@elohim/graphos-tokens` css/scss): brand palettes (Linen light / Indigo Night dark) + per-pillar `--elohim-{pillar}-*` and chrome `--elohim-nav-*`/`--elohim-omnibar-*` bindings — decorator inlines in Library B migrate to consuming it (graphos-designer lane).
2. **Runbook §4.X bundle-styling contract**: every new bundle's checklist gains "import elohim-core tokens + graphos binding artifact in styles.scss; never duplicate token definitions" (runbook is co-owned — coordinate with its current editors).
3. **De-duplicate**: when the artifact ships, elohim-app/src/styles.css and elohim-core/tokens.scss reconcile to one source; lamad's interim chrome-binding block deletes.
4. **On-accent pair — PARTIALLY RESOLVED (2026-06-05, theme-authority spec):** minted
   `--lamad-on-accent: #ffffff` + `--lamad-accent-emphasis: #4f46e5` in tokens.scss; the binding
   pairs bubble/badge with them. Key finding the original item missed: NO foreground passes 4.5:1
   on the dark-mode accent-primary `#6366f1` (pure white computes 4.47:1) — "near-white on accent"
   was insufficient; small-text-on-accent surfaces need the darker emphasis accent. Remaining for
   the artifact: carry both tokens into graphos-tokens and re-audit every accent consumer.

## Follow-ups filed by the theme-authority work (2026-06-05)

5. **`--lamad-text-muted` fails dark (3.07:1 on bg-secondary)** — palette-wide consumer audit
   needed before any small-text use; chrome avoids it. (theme-authority spec §1 C4)
6. **capabilityContract gate write-back unwired** — `cem-plugins/capability-contract.mjs` stubs
   a11y/i18n/uaPrefs as "unknown"; the theme-contrast + sibling gates now produce real grades;
   wire the test-runner write-back into `dist/custom-elements.json`. (theme-authority spec §8.3)
7. **Gateway badges invisible by default** (`badge-controversy`/`badge-settled` bg=Canvas on
   Canvas) — text passes contrast but the badge SHAPE has no surface differentiation;
   1.4.11-adjacent, needs a designed treatment, not a default tweak. (theme-authority spec §4.4)
8. **SSR early-theme inline script** — doorway-served pages flash the default-dark palette before
   ThemeStore applies a persisted light preference; needs a doorway-ssr-context design pass.
   (theme-authority spec §8.6)
9. **Button variant brand styling granularity** — de-branding the primitive (the gospel's
   cardinal-sin fix) leaves one binding pair for all variants; per-variant brand treatment
   belongs to graphos-tokens / Library B. (theme-authority spec §4.3 triage)
10. **Navigator CustomTheme story phantom cssprops** — the Library A story binds
    `--elohim-nav-switcher-*` / `-profile-*` props the element never declared; banner props now
    exist (minted 2026-06-05), the rest need story-vs-manifest reconciliation.
    (component-architect lane)

## Pointers
- Audit evidence: omnibar-consolidation spec §9.8 (2026-06-05)
- Library boundary gospel: app/elohim-library/CLAUDE.md; blank-slate: app/elohim-elements/CLAUDE.md
- Interim wiring: app/lamad/src/styles.scss (B18)
