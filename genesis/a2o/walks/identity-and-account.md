---
view: /identity, /account, /identity/login?returnUrl=
urls: [https://doorway-alpha.elohim.host/identity, https://doorway-alpha.elohim.host/account, https://doorway-alpha.elohim.host/account/security, https://doorway-alpha.elohim.host/identity/login?returnUrl=%2Flamad%2Fpath%2Felohim-protocol]
personas: [Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/imagodei/ (identity routes, account routes), host-shell route table
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-shell, adam-learning
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/account/security --as Adam --out recheck-identity-and-account --budget 180
---
# Identity and account: persona walk

Where a signed-in person looks for who they are and how they sign in. Every route walked either redirected silently, rendered a bare shell, or dropped the person on a 404 after sign-in. The 401s on `/hc/connect` may be starving the account page of content; judge after walks__session-crosses-bundle-seam#1.

## Stations

- [ ] BLOCKS·H9 Return to the path you were on after sign-in (the returnUrl resolves to a 404)
  - Persona / route: Adam, `/lamad/path/elohim-protocol` then "sign in"
  - Repro: 1. Open `/lamad/path/elohim-protocol`. 2. Click "sign in" (goes to `/identity/login?returnUrl=%2Flamad...`). 3. Observe where the click ends.
  - Expected (as persona): after sign-in I return to the path I was reading.
  - Observed: the click ended on "Page Not Found" for `/lamad/path/elohim-protocol`. The 404 offers "Go Home / Explore Lamad / Go Back" and suggestions that lead elsewhere than the path being read. Verify first: the walker clicked the sign-in link as an already signed-in fixture human; reproduce from a signed-out browser to separate "returnUrl not resolvable by the host shell" from "already signed in".
  - Source: `grep -rn "returnUrl" /projects/elohim/app/elohim-app/src /projects/elohim/app/lamad/src --include=*.ts`
  - Owner class: component-fix
  - Depends on: walks__session-crosses-bundle-seam#2
  - Done when: from a signed-out cold load, "sign in" then completing login returns to `/lamad/path/elohim-protocol` (recheck final URL and title are the path, not "Page Not Found"); a returnUrl the shell cannot resolve falls back to `/lamad` with a message.
  - Finding ids: LEARN-02

- [ ] BLOCKS·H9 Answer a request for `/identity` instead of redirecting silently to `/`
  - Persona / route: Adam, `/identity`
  - Repro: 1. Load `/identity` as Adam. 2. Read the final URL and title.
  - Expected (as persona): my identity page, or a clear message why not.
  - Observed: silent redirect to `/` (landing), title unchanged, no message (`capture.json` finalUrl `/`). Console on this page also shows 6 x 401 on `/hc/connect` with nothing visible.
  - Source: `grep -rn "'identity'\|\"identity\"" /projects/elohim/app/elohim-app/src/app/app.routes.ts /projects/elohim/app/elohim-app/src/app/imagodei`
  - Owner class: component-fix
  - Depends on: none
  - Done when: `/identity` renders an identity/profile page (or redirects to `/account` with a visible one-line reason); recheck finalUrl is not `/` unless the page says why.
  - Finding ids: SHELL-02

- [ ] CONFUSES·H4 Give `/account/security` the shared header, a way back, and visible content
  - Persona / route: Adam, `/account` (redirects to `/account/security`)
  - Repro: 1. Load `/account` as Adam. 2. Look at the rendered page.
  - Expected (as persona): the same bar and layout as other pages, with my security settings.
  - Observed: no top bar, no back/home, an empty content area under the heading "Security & sign-in", only a left menu, on the bare background gradient. Verify first: the shell walk ran under RAM-guard shedding, so the render may be partial, and the empty content may be fed by the /hc/connect 401s. Re-capture with `--wait` before judging.
  - Source: `grep -rn "Security & sign-in" /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: walks__session-crosses-bundle-seam#1, walks__product-tokens-and-shell#1
  - Done when: `/account/security` shows the product's minimum chrome (per the shell decision) and its settings content or an explicit message why it cannot load; recheck capture shows more than the heading and menu.
  - Finding ids: SHELL-03
