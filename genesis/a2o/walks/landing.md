---
view: /
urls: [https://doorway-alpha.elohim.host/, https://doorway-alpha.elohim.host/ (390px phone)]
personas: [anonymous]
surface: app/elohim-app/src/app/**/hero (hero.component.scss, hero.component.html, landing-cta-start)
habit: none
walked: 2026-10-16 · deployed alpha · walkers anon-visitor, adam-shell, adam-learning, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/ --out recheck-landing --budget 180
---
# Landing: persona walk

The landing is the first page a newcomer sees. The hero is calm and clear ("Technology organized around love", two calls to action) and the "Watch the vision" privacy gate is honest. It then fails the newcomer: "Start the journey" lands on a provenance ledger, the page is mostly blank until scrolled, and the phone layout crops the hero art.

## Stations

- [ ] CONFUSES·H2 Decide and apply where "Start the journey" sends a first-time visitor
  - Persona / route: anonymous visitor, `/`
  - Repro: 1. Load `/` cold. 2. Click "Start the journey" (testid `landing-cta-start`). 3. Observe the landing page at `/epr/elohim-protocol`.
  - Expected (as persona): the first lesson, or a short orientation in plain words, so I can start learning.
  - Observed: lands on an EPR provenance dashboard: "Notarized, anchor not yet verified here", "Held by 3 of 3 households", "Who's here: counted as protocol events", a truncated hash `uhCkkQIaPDx-…DAivFl0a`, "Path · v3.1.0". A ledger, not a lesson. The hero button href is `/epr/elohim-protocol`.
  - Source: `grep -rn "landing-cta-start" /projects/elohim/app/elohim-app/src`
  - Owner class: design-decision
  - Depends on: none
  - Done when: decision recorded in `genesis/a2o/walks/landing.md` (append a "Decision" note under this station) naming the newcomer front door (lamad path step, a lamad orientation, or an /epr page with a plain-language lead), the CTA href is changed to it, and the recheck shows "Start the journey" ending on a page whose first visible text is not the words Notarized, households or hash.
  - Finding ids: ANON-01

- [ ] CONFUSES·H1 Make the landing content visible without scrolling-triggered reveal
  - Persona / route: anonymous visitor, `/` desktop and phone
  - Repro: 1. Load `/` with a full-page capture and no scrolling (`pnpm look https://doorway-alpha.elohim.host/` without `--scroll`). 2. Compare with a capture taken with `--scroll`.
  - Expected (as persona): content is present when I print, jump to the bottom, scroll fast, or have no IntersectionObserver.
  - Observed: the page is 13,433 px tall on desktop and 23,201 px on a 390 px phone; about 10,000 px is a blank blue gradient until scrolled (reveal-on-scroll). Adam signed in sees the same blank page.
  - Source: `grep -rln "IntersectionObserver\|reveal" /projects/elohim/app/elohim-app/src/app` then the landing section components
  - Owner class: component-fix
  - Depends on: none
  - Done when: recheck full-page capture without `--scroll` shows section content (no 10,000 px void), and with JS reveal disabled the sections still render; page height on phone is recorded in the report and is below 15,000 px or the length is declared intentional in the fulfil report.
  - Finding ids: ANON-10

- [ ] CONFUSES·H8 Recompose the hero art for a 390 px viewport
  - Persona / route: anonymous visitor on a phone, `/`
  - Repro: 1. Load `/` at 390 px width. 2. Look at the top of the page.
  - Expected (as persona): the whole constellation figure, scaled or recomposed.
  - Observed: the constellation figure is cropped to a few nodes and the top of a head, with a large empty band above the title. Verify first: this comes from a single 390 px capture; re-capture before changing.
  - Source: `hero.component.scss` (hero art sizing and object-position)
  - Owner class: component-fix
  - Depends on: none
  - Done when: a 390 px capture of `/` shows the full figure (or an intentional crop with the subject centred) and no empty band above the title.
  - Finding ids: ANON-09

- [ ] POLISH·H4 Make the "Start the journey" gradient the same on phone and desktop
  - Persona / route: anonymous visitor, `/` at 1280 px and 390 px
  - Repro: 1. Capture `/` at desktop width, note the CTA. 2. Capture at 390 px.
  - Expected (as persona): the same primary-action colour.
  - Observed: desktop CTA is purple-to-pink; at 390 px it is blue-to-orange. Verify first: single phone capture.
  - Source: `hero.component.scss` (CTA background and any media query / prefers-color-scheme rule)
  - Owner class: component-fix
  - Depends on: walks__product-tokens-and-shell#2 (which tokens are product-wide)
  - Done when: the CTA gradient is identical at both widths, or the difference is written as intentional next to the style rule.
  - Finding ids: ANON-08

- [ ] POLISH·H4 Remove the pointer cursor and hover effect from the hero h1
  - Persona / route: anonymous visitor, `/`
  - Repro: 1. Load `/`. 2. Hover and click the "Elohim Protocol" h1.
  - Expected (as persona): text does not look clickable unless it is.
  - Observed: the h1 has `cursor: pointer` and a hover effect but no click handler.
  - Source: `hero.component.scss:88`
  - Owner class: component-fix
  - Depends on: none
  - Done when: the h1 has no pointer cursor or hover transform, or it has a handler that goes somewhere; recheck probe lists no NO-EFFECT click on the h1.
  - Finding ids: ANON-11

- [ ] POLISH·H1 Align the video button label with its privacy card and explain SUMMARY
  - Persona / route: anonymous visitor, `/`
  - Repro: 1. Load `/`. 2. Click "Watch the vision". 3. Read the card.
  - Expected (as persona): consistent duration, and a hint of what SUMMARY gives instead of the video.
  - Observed: the button says "50 min", the card says "~51min". The card ("Outside content … nobody here vouches for it … Playing it lets YouTube see that you watched.") has BACK and SUMMARY buttons; SUMMARY's purpose is not explained. The gate wording itself is honest and should stay.
  - Source: `grep -rn "Outside content\|Watch the vision" /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: none
  - Done when: both places show the same duration (single source) and SUMMARY has a one-line label or tooltip saying what it opens.
  - Finding ids: ANON-12
