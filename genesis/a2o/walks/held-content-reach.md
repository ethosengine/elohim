---
view: content listed in lamad paths but held at private reach
urls: [https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/0, https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/1, https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/5]
personas: [anonymous, Adam (signed in, fixture human)]
surface: app/lamad/src/app/models/content-node.model.ts, doorway/storage content routes (/db/content/:id, /api/v1/cache/Content, /api/v1/governance/content/manifesto/accumulation)
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-learning, anon-visitor
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/1 --as Adam --out recheck-held-content-reach --budget 180
---
# Held content reach: substrate investigation

The elohim-protocol path lists about 56 steps; steps 1-22 (quizzes, assessments, most readmes) render "PLACEHOLDER: Held at private reach: <slug>" for Adam, with 403 and 404 beneath. The anonymous step 0 renders blank. These are investigation stations with a concrete first probe.

## Stations

- [ ] BLOCKS·H9 Find why signed-in Adam gets 403 on `/db/content/<id>` and 404 on `/api/v1/cache/Content` for path steps
  - Persona / route: Adam, `/lamad/path/elohim-protocol/step/1`, 2, 3, 5-22
  - Repro: 1. `pnpm -s probe https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/1 --as Adam --out recheck-held-content-reach --budget 180`. 2. In `probe.json` list the failed requests: 404 `/api/v1/cache/Content`, 403 `/db/content/<id>` (id from the node, for example `quiz-manifesto-foundations`). 3. Fetch the same `/db/content/<id>` with and without Adam's session and read the reach on the stored node.
  - Expected (as persona): open a quiz or assessment and answer it.
  - Observed: content shows "PLACEHOLDER: Held at private reach: quiz-manifesto-foundations" saying "If you have standing for it, sign in" to an already-signed-in Adam. Verify first: is the node's reach really private (data) or is the 403 produced because the bundle does not present Adam's session (walks__session-crosses-bundle-seam#2)? The step-0 Manifesto does open for him, so some content is reachable.
  - Source: `app/lamad/src/app/models/content-node.model.ts` (reach field); doorway handler: `grep -rn "private reach\|ContentReach" /projects/elohim/doorway /projects/elohim/elohim/elohim-storage/src --include=*.rs -l`
  - Owner class: substrate:content reach / doorway content route
  - Depends on: walks__session-crosses-bundle-seam#2
  - Done when: a written cause (data reach too narrow, missing grant, or missing session) with the response excerpt is in the fulfil report; the fix yields 200 for Adam on `/db/content/quiz-manifesto-foundations` in the recheck, or the nodes' reach is declared and the outline stops listing them (record the choice in walks__progress-ownership#2).
  - Finding ids: LEARN-03

- [ ] BLOCKS·H1 Find why the anonymous step 0 renders blank and `/api/v1/governance/content/manifesto/accumulation` returns 404
  - Persona / route: anonymous, `/lamad/path/elohim-protocol/step/0`
  - Repro: 1. `pnpm -s probe https://doorway-alpha.elohim.host/lamad/path/elohim-protocol --out recheck-held-content-reach-anon --budget 180` and click "Begin Journey". 2. Read `httpErrors` for `/api/v1/governance/content/manifesto/accumulation`. 3. Also `pnpm look https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/0 --wait-testid <content testid>` with a long wait.
  - Expected (as persona): the first lesson, or a loading and failure message.
  - Observed: Verify first: white page with only the bar and "sign in" in two captures (plain and `--scroll`) and one timeout (more than 150 s); it may be slow content rather than failure. The probe's "Begin Journey" click also logged a 404 on `/api/v1/governance/content/manifesto/accumulation`. Anonymous learners can browse everything else, so whether an anonymous learner is meant to start here at all is open.
  - Source: `grep -rn "content/manifesto/accumulation" /projects/elohim --include=*.ts --include=*.rs -l` (route and caller)
  - Owner class: substrate:content route / lamad step loader
  - Depends on: none
  - Done when: a written cause (route missing, content held from anonymous, or slow load) is in the fulfil report; recheck as anonymous shows either lesson text or a visible loading/failure message within 30 s and no 404 for that route. If the fix is only a missing loading/failure state, add a component-fix station to walks__lamad-path-step.
  - Finding ids: ANON-17
