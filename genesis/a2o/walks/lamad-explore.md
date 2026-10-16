---
view: /lamad/explore
urls: [https://doorway-alpha.elohim.host/lamad/explore]
personas: [Adam (signed in, fixture human)]
surface: app/lamad/src/app/ (explore / knowledge-graph component)
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-learning
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/lamad/explore --as Adam --out recheck-lamad-explore --budget 180
---
# Lamad explore (Knowledge Graph): persona walk

A graph of the learning territory. Only a single render was taken; interaction with the graph was not walked.

## Stations

- [ ] CONFUSES·H1 Fix label collisions, the cropped "Path" node, the dark canvas in a light shell, and explain the sync bar
  - Persona / route: Adam, `/lamad/explore`
  - Repro: 1. Load `/lamad/explore`. 2. Read the bar at the top and the graph.
  - Expected (as persona): a graph whose labels are readable and whose theme matches the page.
  - Observed: a bar reads "Syncing: 62/63 items, 6 peers". The graph's chapter circles overlap labels (Public Observer / Economic Coordination text collisions) and a half-cropped "Path" circle sits at the bottom right. A dark panel sits inside a light page (theme break). Verify first: is the dark canvas intentional (check the component's styles) before recolouring.
  - Source: `grep -rln "Syncing" /projects/elohim/app/lamad/src/app` and the graph layout component beside it
  - Owner class: component-fix
  - Depends on: walks__product-tokens-and-shell#2 (theme tokens), for the dark/light decision only
  - Done when: a capture of `/lamad/explore` shows no overlapping labels and no node cropped by the viewport; the canvas theme matches the page or the dark canvas is documented as intentional; the sync bar states what "62/63 items" and "6 peers" mean in plain words.
  - Finding ids: LEARN-13
