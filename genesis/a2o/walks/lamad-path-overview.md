---
view: /lamad/path/:pathId
urls: [https://doorway-alpha.elohim.host/lamad/path/elohim-protocol]
personas: [Adam (signed in, fixture human)]
surface: app/lamad/src/app/ (path-overview.component.html)
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-learning
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/lamad/path/elohim-protocol --as Adam --out recheck-lamad-path-overview --budget 180
---
# Lamad path overview: persona walk

A coherent chapter outline with "Territory Mastery 0%" and a "Begin Journey" that opens step 1. It is the best moment in the learning walk. Progress readouts are in walks__progress-ownership; the held steps behind the rows are in walks__held-content-reach.

## Stations

- [ ] CONFUSES·H4 Make the Governance Epic and Value Scanner Epic rows navigate or look inert
  - Persona / route: Adam, `/lamad/path/elohim-protocol`
  - Repro: 1. Open the path overview as Adam. 2. Click the "Governance Epic" row, then the "Value Scanner Epic" row. 3. Click a sibling chapter row.
  - Expected (as persona): every chapter row opens something, or does not look like a row you can click.
  - Observed: both epic rows are UNCLICKABLE in the probe (cursor suggests a row, nothing happens) while all sibling rows navigate. "Start Chapter" jumps to the chapter's first step, which in this path is a held placeholder (see walks__held-content-reach#1).
  - Source: `path-overview.component.html` in `/projects/elohim/app/lamad/src/app`; find with `grep -rn "Start Chapter" /projects/elohim/app/lamad/src/app`
  - Owner class: component-fix
  - Depends on: none
  - Done when: the recheck probe reports no UNCLICKABLE rows on the overview; either the rows navigate to the chapter or they lose the pointer cursor and hover style and say why (for example a "soon" label).
  - Finding ids: LEARN-12
