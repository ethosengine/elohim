---
view: connection status (who owns "can't reach the network", empty vs unbuilt vs could-not-load)
urls: [https://doorway-alpha.elohim.host/, https://doorway-alpha.elohim.host/shefa, https://doorway-alpha.elohim.host/community/governance/challenges, https://doorway-alpha.elohim.host/avodah/projects]
personas: [Adam (signed in, fixture human)]
surface: HolochainClient connect path (grep "Chaperone connection failed"), shefa overview Disconnected pill, elohim-navigator.component.html tray status
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-shell, adam-learning, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/shefa --as Adam --out recheck-connection-status-ownership --budget 180
---
# Connection status ownership: decision

Every signed-in page logs 2 to 12 console errors from `/hc/connect` (503, then 401 "Authenticated actor does not match native app"). The same failure is invisible on some pages, a red pill on others, "Connection Error" in the tray, and never stops example data rendering as live. Empty, unbuilt and could-not-load all render as a bare sentence. The root cause is investigated in walks__session-crosses-bundle-seam#1; this doc decides display.

## Stations

- [ ] CONFUSES·H1 Decide where connection status lives and what pages say while the network is unreachable
  - Persona / route: Adam, `/`, `/identity`, `/account/security`, `/shefa`, profile tray
  - Repro: 1. Load `/` and `/shefa` as Adam. 2. Compare page text and console. 3. Open the tray on `/shefa`.
  - Expected (as persona): the same failure gives the same signal, and no example data posing as live.
  - Observed: `/hc/connect` 401s and "[HolochainClient] Chaperone connection failed" are silent on home, identity and account; `/shefa` shows a red "Disconnected" pill; the tray shows "Connection Error" with a red dot wherever there is a tray; `/shefa` still renders example-looking data (3 events, `agent-le...-001`, `content-node-abc`) beside the pill.
  - Source: `grep -rn "Chaperone connection failed\|Connection Error\|Disconnected" /projects/elohim/app/elohim-app/src --include=*.ts --include=*.html`
  - Owner class: design-decision
  - Depends on: walks__session-crosses-bundle-seam#1 (so a real outage can be told from a session mismatch)
  - Done when: decision recorded in `genesis/a2o/walks/connection-status-ownership.md` (one service owns status; the shell shows it one way; example data is never shown without a label) and applied so `/` and `/shefa` show the same signal for the same failure.
  - Finding ids: SHELL-08

- [ ] CONFUSES·H1 Give "nothing here" and "could not load" different states and copy
  - Persona / route: Adam, `/community/governance/challenges`, `/community/governance/proxy-votes`, `/community/directory`, `/avodah/projects`
  - Repro: 1. Load each page as Adam. 2. Read the empty-state text.
  - Expected (as persona): to tell "nothing yet" from "could not load", with a next step where one exists.
  - Observed: wording and design of "nothing here" differ every time: "No challenges have been filed yet." (small grey, no action), "No governance profile found." (centred, no heading, reads like an error), "No pending proxy votes.", "No members found.", and Avodah "No projects yet. Create a project to start tracking work stories." (the only one with a next step). Verify first: several are probably load failures masked as empty because Adam's session has the 401s; they cannot be told apart from true emptiness until that is fixed.
  - Source: `grep -rn "No challenges have been filed\|No governance profile found\|No pending proxy votes\|No members found" /projects/elohim/app/elohim-app/src`
  - Owner class: design-decision
  - Depends on: walks__session-crosses-bundle-seam#1
  - Done when: decision recorded in `genesis/a2o/walks/connection-status-ownership.md` for one empty-state component with three variants (empty, could-not-load, no-access), applied to the five pages; with the network blocked in the recheck each shows the could-not-load variant.
  - Finding ids: COMMONS-17

- [ ] POLISH·H1 Cap retry loops and quiet the console on every page
  - Persona / route: Adam, every page; heaviest on `/`
  - Repro: 1. Load `/` as Adam with the console captured (`capture.json`). 2. Count `/hc/connect` 401s and aborted requests.
  - Expected (as persona): no console noise from a known-failing connect.
  - Observed: every page logs 2-6 x 401 on `/hc/connect` and "Chaperone connection failed" errors (12 on the commons pages); home adds about 35 aborted `epr-head` and `api/v1/resilience` requests.
  - Source: `grep -rn "epr-head\|api/v1/resilience" /projects/elohim/app/elohim-app/src --include=*.ts`
  - Owner class: component-fix
  - Depends on: walks__session-crosses-bundle-seam#1
  - Done when: recheck `capture.json` for `/` shows at most one failed `/hc/connect` attempt per load and under 5 aborted requests, or a backoff cap is documented in the fulfil report.
  - Finding ids: SHELL-15
