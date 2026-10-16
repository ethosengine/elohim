---
view: product-wide shell and design tokens (landing, /epr, /lamad, pillars, utility routes)
urls: [https://doorway-alpha.elohim.host/, https://doorway-alpha.elohim.host/epr/elohim-protocol, https://doorway-alpha.elohim.host/lamad, https://doorway-alpha.elohim.host/community, https://doorway-alpha.elohim.host/shefa, https://doorway-alpha.elohim.host/avodah, https://doorway-alpha.elohim.host/doorway, https://doorway-alpha.elohim.host/map, https://doorway-alpha.elohim.host/account/security]
personas: [anonymous, Adam (signed in, fixture human)]
surface: app/elohim-app/src (global styles, pillar layouts), app/lamad/src/app (lamad-layout), app/elohim-library/projects/graphos (token source)
habit: none
walked: 2026-10-16 · deployed alpha · walkers anon-visitor, adam-shell, adam-learning, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/shefa --as Adam --out recheck-product-tokens-and-shell --budget 180
---
# Product tokens and shell: decision

The product is several visual systems joined by a floating chip. Three primary colours, three dark palettes, three nav models, two icon systems. Stations here are decisions that fix the tokens and minimum chrome; other docs apply them.

## Stations

- [ ] CONFUSES·H4 Decide the minimum chrome every page carries
  - Persona / route: Adam, `/community`, `/shefa`, `/avodah` vs `/doorway`, `/map`, `/account`, `/resolve`
  - Repro: 1. Load each route as Adam. 2. Note bar, side nav, footer, search and surface style on each.
  - Expected (as persona): one shell: wherever I am, a stable way home and a stable "where am I".
  - Observed: three or more shells: a dark pillar bar with sidebar (shefa); a dark bar without sidebar (community, avodah, with search only on avodah); no bar at all (doorway, map, account, resolve). Surface styles also differ: blue gradient, white, cream serif. Verify first: RAM-guard shed processes during this walk, so bar-less renders of `/map`, `/doorway`, `/account` may be partial.
  - Source: `app/elohim-app/src/app/` pillar layout components and route table (`app.routes.ts`)
  - Owner class: design-decision
  - Depends on: none
  - Done when: decision recorded in `genesis/a2o/walks/product-tokens-and-shell.md` (list of chrome every route carries: header, way home, footer or none, page-info chip slot) and applied so each walked route shows at least that chrome in the recheck.
  - Finding ids: SHELL-04

- [ ] CONFUSES·H4 Decide which tokens are product-wide across landing, /epr and /lamad
  - Persona / route: anonymous, `/` then `/epr/elohim-protocol` then `/lamad`
  - Repro: 1. Click "Start the journey" then "Open in Lamad". 2. Compare palette, buttons, header and footer at each step.
  - Expected (as persona): one product.
  - Observed: three visual systems: landing (sky gradient, purple-pink gradient CTA, uppercase outline button), `/epr` (cream, serif headings, green buttons), `/lamad` (white, indigo buttons, "elohim.host" header). The click path changes palette twice.
  - Source: global styles and token files: `grep -rn "\-\-color-primary\|--primary" /projects/elohim/app/elohim-app/src/styles* /projects/elohim/app/lamad/src`
  - Owner class: design-decision
  - Depends on: none
  - Done when: decision recorded in `genesis/a2o/walks/product-tokens-and-shell.md` naming the shared tokens (primary action, surface, heading font, dark surface) and whether each zone is deliberate with a visible transition; /epr and /lamad consume them.
  - Finding ids: ANON-07

- [ ] CONFUSES·H4 Give each pillar the same layout model (nav, footer, title, width)
  - Persona / route: Adam, `/community` then `/shefa` then `/avodah`
  - Repro: 1. Load the three pillars. 2. Compare nav, footer, container width and title block.
  - Expected (as persona): one layout model with per-pillar content.
  - Observed: Qahal has no side nav, a centred 740 px column and the footer "Qahal - Community & Governance | Powered by Elohim Protocol"; Shefa has a left sidebar with section labels and no footer; Avodah has no sidebar, left-aligned content, a header search box and no footer.
  - Source: pillar layout components under `app/elohim-app/src/app/{qahal,shefa,avodah}`
  - Owner class: component-fix
  - Depends on: walks__product-tokens-and-shell#1
  - Done when: after the chrome decision, the three pillars share one layout component (or declared slots) and the recheck on each shows the same header, footer policy and content-width rule.
  - Finding ids: COMMONS-11

- [ ] POLISH·H4 Use one primary-action colour and one heading scale across pillars
  - Persona / route: Adam, `/community`, `/shefa`, `/avodah/projects`
  - Repro: 1. Read the primary button and heading style on each page.
  - Expected (as persona): the same role looks the same.
  - Observed: primary buttons are Qahal blue (about #1976d2) "Community Directory", Shefa indigo "Refresh Data", Avodah violet "+ New Project" with purple links; secondary: Qahal grey-filled "View Shefa", Shefa outlined. Headings: Qahal 56 px black, Shefa 32 px navy, Avodah 32 px slate.
  - Source: pillar stylesheets: `grep -rn "button" /projects/elohim/app/elohim-app/src/app/{qahal,shefa,avodah} --include=*.scss -l`
  - Owner class: design-decision
  - Depends on: walks__product-tokens-and-shell#2
  - Done when: decision recorded naming the "primary action" and "secondary action" tokens and heading sizes, and all three pillars reference them (computed styles equal in the recheck).
  - Finding ids: COMMONS-15

- [ ] POLISH·H4 Share dark-scheme surface tokens across pillars
  - Persona / route: Adam, `/community`, `/shefa`, `/avodah` with `--scheme dark`
  - Repro: 1. `pnpm look <url> --as Adam --scheme dark` for each pillar. 2. Compare cards and backgrounds.
  - Expected (as persona): the same dark theme in every pillar.
  - Observed: Qahal dark has neutral #1a1a1a to #2a2a2a charcoal cards on a navy page; Shefa dark has navy glass cards plus a purple-black gradient filler below content; Avodah dark is flat #0e1527 with no cards. Light-scheme Shefa has a blue gradient block below content (the bug itself is in walks__shefa).
  - Source: pillar dark-scheme rules: `grep -rn "prefers-color-scheme: dark" /projects/elohim/app/elohim-app/src/app/{qahal,shefa,avodah}`
  - Owner class: design-decision
  - Depends on: walks__product-tokens-and-shell#2
  - Done when: decision recorded for shared surface tokens; dark captures of the three pillars use the same page and card colours.
  - Finding ids: COMMONS-18

- [ ] POLISH·H4 Choose one icon system and a fallback for a missing glyph
  - Persona / route: Adam, `/community`, `/shefa`, `/avodah`
  - Repro: 1. Load each pillar. 2. Look at card icon slots, button leading space and Shefa stat cards.
  - Expected (as persona): icons that render, in one style.
  - Observed: Community cards have an empty icon slot above each title; buttons "Explore Lamad" and "View Shefa" have blank leading space where an emoji sits; Shefa stat cards (3 / 2 / 8 / 35) and event rows show blank squares. Shefa nav and the Resources page use real SVG or Material icons that do render. So it is emoji in Qahal, Material in Shefa, none in Avodah. Verify first: emoji may be a headless font gap; the mix of systems is real.
  - Source: `grep -rn "mat-icon\|material-icons" /projects/elohim/app/elohim-app/src/app -l` and emoji literals in qahal templates
  - Owner class: design-decision
  - Depends on: none
  - Done when: decision recorded naming one icon system (inline SVG or Material) and the fallback when it fails; qahal templates use it; a capture on a machine with emoji fonts removed shows no blank slot.
  - Finding ids: COMMONS-14
