---
view: learning progress readouts (/epr vs /lamad overview vs step header)
urls: [https://doorway-alpha.elohim.host/epr/elohim-protocol, https://doorway-alpha.elohim.host/lamad/path/elohim-protocol, https://doorway-alpha.elohim.host/lamad/path/elohim-protocol/step/1]
personas: [Adam (signed in, fixture human)]
surface: epr-home "Your mark on it" component, app/lamad/src/app/ path-overview and step header components
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-learning
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/lamad/path/elohim-protocol --as Adam --out recheck-progress-ownership --budget 180
---
# Progress ownership: decision

The same path shows different progress depending on the surface, and counts steps the learner cannot open.

## Stations

- [ ] CONFUSES·H4 Decide which surface owns progress so /epr and /lamad agree
  - Persona / route: Adam, `/epr/elohim-protocol` and `/lamad/path/elohim-protocol`
  - Repro: 1. Load `/epr/elohim-protocol` and read "Your mark on it". 2. Load `/lamad/path/elohim-protocol` and read "Territory Mastery". 3. Compare.
  - Expected (as persona): one number for one path.
  - Observed: `/epr` says "Your mark on it, Practicing 20%" and recognises Adam; lamad's overview shows 0% (and shows him signed out). The same path is two UIs (cream serif vs sans indigo), and `/epr` says "Not on a learning path yet" for a Path. Verify first: the lamad 0% may come from the missing session (walks__session-crosses-bundle-seam#2), not a different data source.
  - Source: `grep -rn "Your mark on it" /projects/elohim/app/elohim-app/src`; `grep -rn "Territory Mastery" /projects/elohim/app/lamad/src`
  - Owner class: design-decision
  - Depends on: walks__session-crosses-bundle-seam#2
  - Done when: decision recorded in `genesis/a2o/walks/progress-ownership.md` (which surface owns, how the other reads it); with a working session the two surfaces show the same percentage in the recheck.
  - Finding ids: LEARN-06

- [ ] CONFUSES·H6 Decide what progress means for a held step and stop counting unopenable steps
  - Persona / route: Adam, `/lamad/path/elohim-protocol` and its step pages
  - Repro: 1. Open the overview and read each step's circle and the chapter counts. 2. Open step 4 (a held placeholder) and read the header.
  - Expected (as persona): see where I am and what is next, with progress only for what I can read.
  - Observed: every step has an empty circle "○" and Territory Mastery is 0% with "0/4" per chapter. The step page header says "1/4" with a filled bar and "STEP 4 OF 4" with a full bar, while the mastery chip reads "SEEN" on a placeholder. Progress is counted for content Adam cannot open. The outline promises 56 steps.
  - Source: step header component: `grep -rn "STEP .* OF" /projects/elohim/app/lamad/src/app`
  - Owner class: design-decision
  - Depends on: walks__held-content-reach#1
  - Done when: decision recorded in `genesis/a2o/walks/progress-ownership.md` (held step is seen, skipped, or blocked; whether the outline lists unopenable steps); the overview and step header show the same counts and a held step is not marked "SEEN".
  - Finding ids: LEARN-05
