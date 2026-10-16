---
view: /community/governance/{challenges,disposition,proxy-votes,sensemaking}
urls: [https://doorway-alpha.elohim.host/community/governance/challenges, https://doorway-alpha.elohim.host/community/governance/disposition, https://doorway-alpha.elohim.host/community/governance/proxy-votes, https://doorway-alpha.elohim.host/community/governance/sensemaking]
personas: [Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/qahal/community.routes.ts:59-108 (sensemaking at :67)
habit: none
walked: 2026-10-16 · deployed alpha · walkers adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/community --as Adam --out recheck-governance-routes --budget 180
---
# Governance routes: persona walk

Four governance views exist as routes but nothing on `/community` or the directory links to them. When reached by URL they are empty or speak internal-routing language. Under the /hc/connect 401s, "empty" cannot be told from "could not load" (walks__connection-status-ownership#2).

## Stations

- [ ] BLOCKS·H3 Link challenges, disposition, proxy votes and sensemaking from the community landing
  - Persona / route: Adam, `/community`
  - Repro: 1. Load `/community` as Adam. 2. List its links (probe `links`). 3. Try to reach "what's being decided".
  - Expected (as persona): a visible entry point to what is being decided.
  - Observed: the probe link crawl of `/community` found only directory, lamad and shefa. The four governance routes exist (`community.routes.ts:59-108`) and are reachable only by typing the URL.
  - Source: `app/elohim-app/src/app/qahal/community.routes.ts:59-108`; landing component under `app/elohim-app/src/app/qahal`
  - Owner class: component-fix
  - Depends on: walks__pillar-nav-coherence#2 (what shows for unbuilt or context-needing destinations)
  - Done when: recheck probe `links` for `/community` includes each governance route that is meant to be reachable (or the fulfil report names which are intentionally deep-link only and why); each linked route loads with a page title.
  - Finding ids: COMMONS-02

- [ ] CONFUSES·H9 Replace "Missing entity context. Please navigate here from a content item." with a way forward
  - Persona / route: Adam, `/community/governance/sensemaking`
  - Repro: 1. Load `/community/governance/sensemaking` directly. 2. Read the page.
  - Expected (as persona): a useful state, or a picker.
  - Observed: "Missing entity context. Please navigate here from a content item." Internal-routing language shown to a person; the only exit is "Back to Community".
  - Source: `app/elohim-app/src/app/qahal/community.routes.ts:67` and the sensemaking component
  - Owner class: component-fix
  - Depends on: none
  - Done when: a bare visit shows plain words (what sensemaking is, how to start from a content item) with a link, or redirects to a picker; the phrase "entity context" does not appear in page text.
  - Finding ids: COMMONS-03

- [ ] POLISH·H8 Make the "Governance Challenges" title legible and give the four pages one header and width
  - Persona / route: Adam, challenges, proxy-votes, sensemaking
  - Repro: 1. Load the three pages. 2. Compare the title colour and the left offset of content.
  - Expected (as persona): one page-header pattern.
  - Observed: "Governance Challenges" is pale periwinkle on near-white, nearly invisible (fails contrast); sibling governance pages use slate. Container widths and left offsets differ: challenges 24 px, proxy-votes about 356 px centred, sensemaking 256 px.
  - Source: governance page components under `app/elohim-app/src/app/qahal`
  - Owner class: component-fix
  - Depends on: walks__product-tokens-and-shell#2
  - Done when: all four pages use one header/empty-state component, the title meets 4.5:1 contrast, and container left offsets are equal; capture confirms.
  - Finding ids: COMMONS-16
