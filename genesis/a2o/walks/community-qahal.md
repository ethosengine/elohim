---
view: /community, /community/directory, /community/collective/:id
urls: [https://doorway-alpha.elohim.host/community, https://doorway-alpha.elohim.host/community/directory, https://doorway-alpha.elohim.host/community/collective/garden-club]
personas: [Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/qahal/ (community.routes.ts, components/collective-detail, directory component)
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-shell, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/community/directory --as Adam --out recheck-community-qahal --budget 180
---
# Community (Qahal): persona walk

Adam wants "my communities, what's being decided". The landing is a four-card brochure, the directory lists seven people with no groups, and member cards lift on hover but do nothing. Governance routes are in walks__governance-routes; whether the landing should be "mine" is walks__signed-in-home#2.

## Stations

- [ ] CONFUSES·H4 Make directory member cards open something, or drop their hover lift
  - Persona / route: Adam, `/community/directory`
  - Repro: 1. Load `/community/directory`. 2. Hover then click each member card.
  - Expected (as persona): click a member or household to see who they are.
  - Observed: cards lift and shadow on hover but 6 of 7 click with NO-EFFECT; the Eve card only changed in-page.
  - Source: Qahal directory component: `grep -rln "directory" /projects/elohim/app/elohim-app/src/app/qahal`
  - Owner class: component-fix
  - Depends on: none
  - Done when: recheck probe shows no NO-EFFECT among member cards; either each card navigates to a profile/relationship view or the hover style is removed.
  - Finding ids: COMMONS-04

- [ ] CONFUSES·H1 Name the collective on its page and distinguish "not found" from "no members"
  - Persona / route: Adam, `/community/collective/garden-club`
  - Repro: 1. Load `/community/collective/garden-club`. 2. Read the heading and the Members tab.
  - Expected (as persona): the Garden Club's name, its members, proposals and discussions.
  - Observed: tabs Members / Proposals / Discussions render and Members says "No members found." with no heading naming the collective, only a "Directory" back link. Verify first: the slug `garden-club` was guessed by the walker; find a real collective id from the directory or seed data, and the "No members" may be the /hc/connect 401 masked as empty (walks__session-crosses-bundle-seam#1).
  - Source: `app/elohim-app/src/app/qahal/components/collective-detail`
  - Owner class: component-fix
  - Depends on: walks__connection-status-ownership#2
  - Done when: with a real collective id the page heading is the collective's name; with an unknown id it says "collective not found" with a link back; recheck on both shows distinct text.
  - Finding ids: COMMONS-06

- [ ] CONFUSES·H2 Show a display name for every member card and a placeholder for an empty household line
  - Persona / route: Adam, `/community/directory`
  - Repro: 1. Load `/community/directory`. 2. Read the seven cards.
  - Expected (as persona): readable names.
  - Observed: one card shows the raw id "human-matt..." (probe text "human-matthew-manager") with "Dowell Household", beside "Adam" and "Eve". Gertrude, James and Susan show no household line while others do, giving uneven card heights. Verify first: ids may come from fixture humans without a profile.
  - Source: Qahal directory component: `grep -rn "household" /projects/elohim/app/elohim-app/src/app/qahal --include=*.html`
  - Owner class: component-fix
  - Depends on: walks__plain-language-pass#1
  - Done when: no card text starts with `human-`; a member with no profile shows a fallback name by one stated rule; the household line is either always present (placeholder) or its space is reserved so card heights match.
  - Finding ids: COMMONS-05

- [ ] CONFUSES·H4 Decide whether the four landing cards are links or labels, and remove pillar jargon from the buttons
  - Persona / route: Adam, `/community`
  - Repro: 1. Load `/community`. 2. Click each of the four cards (Graduated Intimacy, Governance, Place-Based Community, Affinity Circles). 3. Read "Explore Lamad (Content)" and "View Shefa (Economy)".
  - Expected (as persona): cards that act or that clearly do not; buttons in words I know.
  - Observed: the cards look like links, were not verified clickable by the shell walker, and the commons walker found them not clickable. "Community Directory" is a primary button. "Explore Lamad (Content)" and "View Shefa (Economy)" put internal pillar names plus a gloss in parentheses on buttons.
  - Source: `grep -rn "Explore Lamad (Content)" /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: walks__plain-language-pass#1, walks__signed-in-home#2
  - Done when: recheck probe shows the cards either navigate or lose the card-link styling; the buttons use the names chosen in the plain-language decision.
  - Finding ids: SHELL-11
