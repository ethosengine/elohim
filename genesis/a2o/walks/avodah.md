---
view: /avodah, /avodah/projects
urls: [https://doorway-alpha.elohim.host/avodah, https://doorway-alpha.elohim.host/avodah/projects]
personas: [Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/avodah/
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-shell, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/avodah --as Adam --out recheck-avodah --budget 180
---
# Avodah (work): persona walk

Avodah is bare: a heading and one "View Projects" link, no side nav, a header with a search box, no footer, and "No projects yet. Create a project to start tracking work stories." with a "+ New Project" button. Shell and token differences against Qahal and Shefa are in walks__product-tokens-and-shell; empty-versus-failed is in walks__connection-status-ownership. Avodah has no finding id of its own in the four reports, so the single station below carries the SUMMARY.md P9 observation.

## Stations

- [ ] CONFUSES·H6 Decide what the Avodah landing is for and verify "No projects yet" is true
  - Persona / route: Adam, `/avodah` and `/avodah/projects`
  - Repro: 1. Load `/avodah` as Adam. 2. Click "View Projects". 3. Read the empty state. 4. Check the console for `/hc/connect` 401s on the same load.
  - Expected (as persona): the work I am doing, or a way to start some.
  - Observed: the landing is a heading and one link. The projects page says "No projects yet. Create a project to start tracking work stories." (the only empty state in the product with a next step). Avodah links nowhere else (no pillar switcher). Verify first: Adam's session has `/hc/connect` 401s on every page, so "No projects yet" may be a masked load failure; settle walks__session-crosses-bundle-seam#1 or fixture a project before judging.
  - Source: `app/elohim-app/src/app/avodah/` landing and projects components
  - Owner class: design-decision
  - Depends on: walks__signed-in-home#2, walks__session-crosses-bundle-seam#1
  - Done when: decision recorded in `genesis/a2o/walks/avodah.md` that states whether `/avodah` shows "what this is" or "my work"; with a working session the projects page either lists Adam's projects or shows the empty state, confirmed by creating a project via "+ New Project" in the recheck.
  - Finding ids: none (SUMMARY.md P9; no walker assigned an id)
