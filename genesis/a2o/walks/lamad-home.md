---
view: /lamad
urls: [https://doorway-alpha.elohim.host/lamad]
personas: [anonymous, Adam (signed in, fixture human)]
surface: app/lamad/src/app/ (path-overview.component.html, lamad-layout.component.html)
habit: none
walked: 2026-10-16 · deployed alpha · walkers anon-visitor, adam-learning
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/lamad --as Adam --out recheck-lamad-home --budget 180
---
# Lamad home: persona walk

The learning home is clean: a featured journey, five path cards, plain labels, and a clear "Begin Journey". Its defects are card art, internal tags on cards, and a tab that behaves oddly. Header and sign-in defects live in walks__app-shell-chrome and walks__session-crosses-bundle-seam.

## Stations

- [ ] CONFUSES·H4 Make the Paths/Knowledge Graph tab state follow the route and stop looking like a button
  - Persona / route: anonymous and Adam, `/lamad`
  - Repro: 1. Load `/lamad`. 2. Click "Learning Paths" (the current tab). 3. Click "Knowledge Graph" and watch the highlight while content changes.
  - Expected (as persona): the current tab is plainly "current" (not a button); the highlighted tab matches the content shown.
  - Observed: "Learning Paths" is a boxed pill that does nothing on click (NO-EFFECT, since it is the current tab); unselected "Knowledge Graph" is plain text and goes to `/lamad/explore`. In one probe step the "Knowledge Graph" tab was highlighted while Learning Paths content was still shown.
  - Source: `grep -rn "Knowledge Graph" /projects/elohim/app/lamad/src/app`
  - Owner class: component-fix
  - Depends on: none
  - Done when: the current tab has `aria-current="page"` and no button affordance, and the highlight changes in the same frame as the content; recheck lists no NO-EFFECT on the tab row.
  - Finding ids: ANON-19, LEARN-09

- [ ] POLISH·H8 Give every path card a default thumbnail and fix the cropped header art
  - Persona / route: anonymous visitor, `/lamad`
  - Repro: 1. Load `/lamad`. 2. Look at "AI Governance for Policy Makers", "Know Thyself" and the Elohim path card. 3. Check network for `/lamad/assets/images/path-placeholder.svg`.
  - Expected (as persona): every card has art and rows align.
  - Observed: "AI Governance for Policy Makers" and "Know Thyself" cards have a large empty gap because `/lamad/assets/images/path-placeholder.svg` returns 404, giving uneven row heights. The Elohim path card thumbnail is cropped to legs/feet with "ELOHIM" cut off at the bottom. The Christian-technology cross art is a different aesthetic from the rest.
  - Source: `path-overview.component.html:63`; asset `app/lamad/src/assets/images/path-placeholder.svg` (create if missing)
  - Owner class: component-fix
  - Depends on: none
  - Done when: recheck shows no 404 for `path-placeholder.svg`, all cards have equal image height, and the Elohim card thumbnail shows its text uncropped. The vetting of catalogue header art stays open as a note in the fulfil report.
  - Finding ids: ANON-18

- [ ] POLISH·H2 Hide internal tags and centred descriptions on path cards
  - Persona / route: Adam, `/lamad`
  - Repro: 1. Load `/lamad` as Adam. 2. Read the tags on each path card.
  - Expected (as persona): plain tags I can understand.
  - Observed: tags such as "generated", "policy_maker", "imago-dei" appear on cards; descriptions are centred; the anonymous render shows empty grey image boxes while loading.
  - Source: `path-overview.component.html` (tag loop) in `/projects/elohim/app/lamad/src/app`
  - Owner class: component-fix
  - Depends on: walks__plain-language-pass#1 (naming convention)
  - Done when: card tags shown to learners exclude internal/generated tags (filter list lives in one place), descriptions are left-aligned, and the loading state shows a skeleton not a grey box.
  - Finding ids: LEARN-10
