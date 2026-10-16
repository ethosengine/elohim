---
view: signed-in home (landing for a known member, pillar landings as "mine")
urls: [https://doorway-alpha.elohim.host/, https://doorway-alpha.elohim.host/community, https://doorway-alpha.elohim.host/shefa, https://doorway-alpha.elohim.host/avodah]
personas: [Adam (signed in, fixture human)]
surface: app/elohim-app/src/app (landing/hero, pillar landing components)
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-shell, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/ --as Adam --out recheck-signed-in-home --budget 180
---
# Signed-in home: decision

Adam is not recognised on the landing, and each pillar landing answers "what this pillar is" or "what the network is doing" rather than "what is mine".

## Stations

- [ ] CONFUSES·H6 Decide what the landing shows a member it knows
  - Persona / route: Adam, `/`
  - Repro: 1. Load `/` as Adam. 2. Compare with the anonymous render.
  - Expected (as persona): a "you are here, signed in as Adam" surface with a way to my things.
  - Observed: the anonymous-looking marketing landing: no header, no avatar, no nav, nothing saying he is signed in (same as the anonymous shot). Verify first: if the host shell has no session at all (walks__session-crosses-bundle-seam#1), the landing cannot know him; settle that first.
  - Source: landing/hero components: `grep -rln "landing-cta-start" /projects/elohim/app/elohim-app/src`
  - Owner class: design-decision
  - Depends on: walks__session-crosses-bundle-seam#1
  - Done when: decision recorded in `genesis/a2o/walks/signed-in-home.md` stating whether a signed-in `/` redirects to a member home, adds a header, or only changes its CTA; applied so the recheck of `/` as Adam differs visibly from the anonymous render.
  - Finding ids: SHELL-01

- [ ] CONFUSES·H6 Decide whether a pillar landing means "what this is" or "what is mine", and apply it to Qahal
  - Persona / route: Adam, `/community`, `/community/directory`
  - Repro: 1. Load `/community` as Adam. 2. Open "Community Directory".
  - Expected (as persona): my communities (Garden Club, Regenerative Ag Network) and what is being decided.
  - Observed: the landing is four descriptive cards (Graduated Intimacy, Governance, Place-Based Community, Affinity Circles), not clickable and not about Adam. The directory shows 7 people or households, no groups, nothing about his memberships. `/shefa` (network counts) and `/avodah` (heading plus one link) have the same shape.
  - Source: `app/elohim-app/src/app/qahal/community.routes.ts`, landing component under `app/elohim-app/src/app/qahal`
  - Owner class: design-decision
  - Depends on: walks__session-crosses-bundle-seam#1
  - Done when: decision recorded in `genesis/a2o/walks/signed-in-home.md` for all three pillar landings (member-first or brochure-first); the Qahal landing, as Adam, lists his collectives or states he has none; shefa and avodah docs reference the decision.
  - Finding ids: COMMONS-01
