# Bundle styling / token contract — the canonical layer the lamad split deferred

**Status:** Captured (2026-06-05 styling-migration audit) · **Priority:** medium-high (fires again on the NEXT pillar split)

## The gap
- `elohim-core/tokens.scss` holds the harvested `--lamad-*` token set but no bundle imported it until lamad's B18 wiring (2026-06-05). The next pillar bundle (shefa/qahal/avodah) inherits the same hole: the pillar-bundle-split runbook's bundle-creation checklist has NO styling/token section (§4.X unwritten).
- The Lit chrome's `--elohim-*` @cssprop surface has no canonical shipped binding — Library B binds tokens in Storybook decorators only; `compute-capacity-tokens.designed.mdx` explicitly anticipates "a global graphos-tokens.css" that does not exist. lamad's chrome-binding block in `app/lamad/src/styles.scss` is an interim, per-bundle copy.

## The work
1. **graphos ships a token artifact** (e.g. `@elohim/graphos-tokens` css/scss): brand palettes (Linen light / Indigo Night dark) + per-pillar `--elohim-{pillar}-*` and chrome `--elohim-nav-*`/`--elohim-omnibar-*` bindings — decorator inlines in Library B migrate to consuming it (graphos-designer lane).
2. **Runbook §4.X bundle-styling contract**: every new bundle's checklist gains "import elohim-core tokens + graphos binding artifact in styles.scss; never duplicate token definitions" (runbook is co-owned — coordinate with its current editors).
3. **De-duplicate**: when the artifact ships, elohim-app/src/styles.css and elohim-core/tokens.scss reconcile to one source; lamad's interim chrome-binding block deletes.
4. **Mint an on-accent token** — the chrome binding puts --lamad-text-primary on accent surfaces (nav bubble, toggle badge); light-mode contrast computes ≈3.0:1, exactly at the WCAG 1.4.11 threshold with no headroom. The graphos-tokens artifact should define --lamad-on-accent (near-white) and the binding layer should consume it.

## Pointers
- Audit evidence: omnibar-consolidation spec §9.8 (2026-06-05)
- Library boundary gospel: app/elohim-library/CLAUDE.md; blank-slate: app/elohim-elements/CLAUDE.md
- Interim wiring: app/lamad/src/styles.scss (B18)
