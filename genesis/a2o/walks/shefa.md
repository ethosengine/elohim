---
view: /shefa and /shefa/*
urls: [https://doorway-alpha.elohim.host/shefa, https://doorway-alpha.elohim.host/shefa/accounts, https://doorway-alpha.elohim.host/shefa/exchange, https://doorway-alpha.elohim.host/shefa/constitutional, https://doorway-alpha.elohim.host/shefa/resources]
personas: [Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/shefa/ (shefa.routes.ts:45-340, components/shared/shefa-placeholder.component.ts, overview component)
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-shell, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/shefa --as Adam --out recheck-shefa --budget 180
---
# Shefa (economy): persona walk

The most product-like pillar: side nav, stats, resource folders. But ten of about nineteen nav entries are "Planned features" stubs, the overview shows network-wide sample-looking data beside a red "Disconnected" pill, and the tray and sidebar duplicate each other. Adam is a fixture human, so example-looking rows may be fixture data.

## Stations

- [ ] CONFUSES·H4 Mark unbuilt Shefa nav entries in the nav, before the click
  - Persona / route: Adam, `/shefa` side nav and `/shefa/accounts`, `/shefa/exchange`, `/shefa/constitutional`
  - Repro: 1. Load `/shefa`. 2. Click Accounts, Exchange, Constitutional. 3. Count nav items that land on the same "Planned features" stub.
  - Expected (as persona): entries lead somewhere, or say they are not ready.
  - Observed: 10 of 19 side-nav items (Accounts, Transactions, Exchange, Insurance, Constitutional, Planning, Settings, Property, Energy, Knowledge...) are identical "Planned features" stubs with a tools icon, shown at the same weight as working pages (Resources, Cluster). Accounts promises "Plaid integration" in a P2P product.
  - Source: `app/elohim-app/src/app/shefa/shefa.routes.ts:45-340`, `shefa/components/shared/shefa-placeholder.component.ts`
  - Owner class: component-fix
  - Depends on: walks__pillar-nav-coherence#2 (decision on list, grey or hide)
  - Done when: per the recorded decision, stub entries are hidden or visibly marked (for example a "soon" label, muted style) in the nav on load; the word Plaid does not appear on Accounts; recheck shows the nav distinguishes built from unbuilt.
  - Finding ids: COMMONS-09

- [ ] CONFUSES·H8 Show Adam's own holdings on the overview and render agents and resources in words, not truncated ids
  - Persona / route: Adam, `/shefa`
  - Repro: 1. Load `/shefa` as Adam. 2. Read the stats and event rows.
  - Expected (as persona): what I hold.
  - Observed: network-wide counts ("8 Active Agents", "35 Recognition Points") and sample-looking rows (`agent-le...-001`, `content-node-abc`, "Great explanation of REA concepts!"); nothing is Adam's balance, holdings or stewardship; truncated ids in two colours are the identity of actors. Verify first: rows may be fixture data, and a "Disconnected" pill is present, so also check what the 401s (walks__session-crosses-bundle-seam#1) hide.
  - Source: the Shefa overview component: `grep -rln "Active Agents" /projects/elohim/app/elohim-app/src/app/shefa`
  - Owner class: design-decision
  - Depends on: walks__signed-in-home#2
  - Done when: decision recorded in `genesis/a2o/walks/shefa.md` (mine, household or network) and the overview leads with it; no visible text matches `agent-` or `content-node-` as a label (display names or a fallback phrase instead).
  - Finding ids: COMMONS-07

- [ ] CONFUSES·H4 Pick one entry point for each duplicated Shefa destination
  - Persona / route: Adam, `/shefa` with the profile tray open
  - Repro: 1. Load `/shefa`. 2. Open the tray and read it. 3. Compare the sidebar and the bottom button row.
  - Expected (as persona): one way to each place.
  - Observed: the tray has "Shefa Dashboard" while already on `/shefa`; the sidebar says "Overview". The bottom row has "Explore Lamad", "My Cluster", "Peer Network" which duplicate the sidebar's "My Cluster" and "Peer Topology".
  - Source: `elohim-navigator.component.html` (tray) and the Shefa sidebar/footer component under `app/elohim-app/src/app/shefa`
  - Owner class: component-fix
  - Depends on: none
  - Done when: each destination appears once in the Shefa chrome (tray item hidden on its own page; bottom row duplicates removed or renamed to match the sidebar); recheck link list shows no duplicate hrefs in chrome.
  - Finding ids: SHELL-09

- [ ] CONFUSES·H1 Say what "Disconnected" means and make Refresh Data and Test Connection give feedback
  - Persona / route: Adam, `/shefa`
  - Repro: 1. Load `/shefa`. 2. Read the red "Disconnected" pill. 3. Click "Refresh Data", then "Test Connection".
  - Expected (as persona): to learn what is disconnected, whether the numbers are live, and the result of a test.
  - Observed: red "Disconnected" top-right while the page shows numbers; no explanation or retry; both buttons are NO-EFFECT with no toast, spinner or result. It correlates with the `/hc/connect` 401s.
  - Source: `grep -rn "Test Connection" /projects/elohim/app/elohim-app/src/app/shefa`
  - Owner class: component-fix
  - Depends on: walks__connection-status-ownership#1
  - Done when: each button visibly responds (spinner then success or error text) in the recheck, and the pill has an adjacent line naming what is unreachable.
  - Finding ids: COMMONS-08

- [ ] POLISH·H4 Fix the body gradient that stops at the first viewport height
  - Persona / route: Adam, any `/shefa` page, light scheme
  - Repro: 1. Load `/shefa/accounts` with a full-page capture. 2. Look from y of about 800 downward.
  - Expected (as persona): a continuous page background.
  - Observed: a saturated blue gradient panel starts at about y=800 below the content on every Shefa page (resources, accounts, exchange); the body background stops at the first viewport height.
  - Source: Shefa shell/body background rules: `grep -rn "background-attachment\|linear-gradient" /projects/elohim/app/elohim-app/src/app/shefa --include=*.scss`
  - Owner class: component-fix
  - Depends on: none
  - Done when: a full-page capture of `/shefa/accounts` shows one continuous background to the bottom of the page.
  - Finding ids: COMMONS-21
