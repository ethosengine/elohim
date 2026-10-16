---
view: session handoff from the host shell to the separately served lamad bundle
urls: [https://doorway-alpha.elohim.host/lamad, https://doorway-alpha.elohim.host/lamad/path/elohim-protocol, https://doorway-alpha.elohim.host/epr/elohim-protocol]
personas: [Adam (signed in, fixture human)]
surface: app/lamad/src/app/components/lamad-layout/lamad-layout.component.html, HolochainClient / Chaperone connect (grep "Authenticated actor does not match native app"), doorway-service /hc/connect handler, app/CLAUDE.md section "Bundle seams are not domain seams"
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-learning, adam-shell, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/lamad --as Adam --out recheck-session-crosses-bundle-seam --budget 180
---
# Session crosses the bundle seam: substrate investigation

Adam is signed in on the host, but lamad treats him as anonymous: "sign in" in the host bar, a generic "T" avatar, a sign-in prompt on held content, a 404 after return. In parallel, `/hc/connect` returns 503 then 401 "Authenticated actor does not match native app" on every signed-in page. One or both may explain many other findings. These are investigation stations with a concrete first probe, not UI fixes.

## Stations

- [ ] BLOCKS·H9 Find why `/hc/connect` returns 401 "Authenticated actor does not match native app" for Adam
  - Persona / route: Adam, any signed-in page (6 x 401 on `/` and `/identity`, 12 on commons pages)
  - Repro: 1. `pnpm -s probe https://doorway-alpha.elohim.host/lamad --as Adam --out recheck-session-crosses-bundle-seam --budget 180`. 2. In `probe.json` read the `/hc/connect` requests (first 503, then 401) and the console line "[HolochainClient] Chaperone connection failed". 3. Compare the actor in the session token Adam holds with the app/cell the connect request names.
  - Expected (as persona): the connect succeeds as Adam, or fails once with a stated reason.
  - Observed: `/hc/connect` returns 503 then 401 with body text "Authenticated actor does not match native app"; no page error. This is the suspected root of the unreachable data behind: the empty "No projects yet", "No members found", the Shefa "Disconnected" pill and zero tiles, and plausibly the held-content 403s. Verify first: Adam is a fixture human on the household-hosted path (hosted only on matthew/adam doorways); his key and the cell the bundle asks for may not be the same agent.
  - Source: `grep -rn "does not match native app" /projects/elohim --include=*.rs --include=*.ts -l` (doorway-service handler and client)
  - Owner class: substrate:session handoff / doorway /hc/connect
  - Depends on: none
  - Done when: a written cause (which two identities disagree) is in the fulfil report with the request/response excerpt; either the 401 is fixed (probe shows `/hc/connect` 200 for Adam on `/lamad`, and the pages in the Depends-on lists get re-walked) or a fix station is added to this doc naming the owner. This unblocks re-judging of walks__connection-status-ownership, walks__community-qahal, walks__shefa and walks__avodah.
  - Finding ids: LEARN-14

- [ ] BLOCKS·H4 Carry the host session into lamad so a signed-in member is shown as signed in
  - Persona / route: Adam, `/lamad`
  - Repro: 1. Sign in as Adam on the host (`--as Adam`). 2. Load `/lamad`. 3. Compare with the anonymous render (`pnpm look https://doorway-alpha.elohim.host/lamad` without `--as`). 4. Open the avatar menu.
  - Expected (as persona): lamad shows Adam's name or initial and his progress.
  - Observed: the host bar shows "sign in", the avatar is a generic "T" whose menu offers Sign in / Register, and the page is pixel-identical to the anonymous render. Whose job it is to hand the session to a separately served app, and what the bundle shows when it cannot prove it, are the open design questions; app/CLAUDE.md "Bundle seams are not domain seams" is the governing note.
  - Source: `lamad-layout.component.html` in `/projects/elohim/app/lamad/src/app/components/lamad-layout/`; start with `grep -rn "session\|currentUser\|isAuthenticated" /projects/elohim/app/lamad/src/app --include=*.ts | head -40`
  - Owner class: substrate:host shell / lamad bundle session handoff
  - Depends on: walks__session-crosses-bundle-seam#1
  - Done when: a written account of how the host passes identity to the bundle (cookie, token, postMessage or shared service) and where it breaks for Adam is in the fulfil report; after the fix, the recheck on `/lamad` as Adam shows no "sign in" link, an avatar that is not the anonymous one, and a differing capture from the anonymous render. Re-walk walks__app-shell-chrome, walks__identity-and-account#1 and walks__progress-ownership#1 afterward.
  - Finding ids: LEARN-01
