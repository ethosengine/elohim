---
view: /lamad/path/:pathId/step/:n
urls: [https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/0, https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/1]
personas: [Adam (signed in, fixture human)]
surface: app/lamad/src/app/ (step page, content renderer, held-reach placeholder), app/lamad/src/app/models/content-node.model.ts
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-learning, anon-visitor
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/0 --as Adam --out recheck-lamad-path-step --budget 180
---
# Lamad path step: persona walk

Step 1 (the Manifesto) opens with a sidebar, breadcrumb and Explore panel. After that most steps render "PLACEHOLDER: Held at private reach: <slug>". Why content is held, and the blank anonymous step 0, are substrate questions in walks__held-content-reach. The stations here are the rendering of what the learner is shown.

## Stations

- [ ] CONFUSES·H2 Replace the slug-as-title and "reach/steward/standing" wording on the held-step placeholder
  - Persona / route: Adam, `/lamad/path/elohim-protocol/step/1` and steps 2, 3, 5-22
  - Repro: 1. Open a quiz or assessment step as Adam. 2. Read the title, breadcrumb, heading and body.
  - Expected (as persona): a human title and a sentence saying I cannot read this yet and what I can do.
  - Observed: the raw slug is the page title, breadcrumb and heading ("Held at private reach: assessment-constitutional-reasoning"; "Resource ID: quiz-manifesto-foundations"); the body uses "reach", "steward", "standing", "seed", and says "If you have standing for it, sign in" to someone already signed in. Verify first: if the walks__session-crosses-bundle-seam station shows the bundle simply lacks Adam's session, the sign-in sentence should vanish once that is fixed; fix the wording regardless.
  - Source: `grep -rn "Held at private reach" /projects/elohim/app/lamad/src /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: walks__plain-language-pass#1
  - Done when: a held step shows the node's title (or a neutral "This step" fallback), never its slug as heading; none of the words reach, steward, standing, seed appear in the learner-facing text; a signed-in viewer is not told to sign in. Recheck page text on a held step confirms it.
  - Finding ids: LEARN-04

- [ ] CONFUSES·H9 Resolve or strip relative-doc links rendered as literal backtick text
  - Persona / route: Adam, `/lamad/path/elohim-protocol/step/0` (Manifesto)
  - Repro: 1. Open step 0. 2. Find `./constitution.md` and `./theology.md` in the body.
  - Expected (as persona): a working link to the matching lamad node, or plain text.
  - Observed: inline-code links render as literal text with backticks (`./constitution.md`, `./theology.md`) styled as links; they are not real destinations.
  - Source: the markdown renderer's link/inline-code handling in `/projects/elohim/app/lamad/src/app`; `grep -rn "marked\|markdown" /projects/elohim/app/lamad/src/app --include=*.ts -l`
  - Owner class: component-fix
  - Depends on: none
  - Done when: relative `.md` links in lamad content either resolve to the lamad route for that node or render as plain non-link text without backticks; recheck step 0 shows no backtick-wrapped link text.
  - Finding ids: LEARN-11
