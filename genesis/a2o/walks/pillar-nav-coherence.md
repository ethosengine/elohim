---
view: pillar navigation model (app switcher, context menu, pillar-to-pillar links)
urls: [https://doorway-alpha.elohim.host/lamad, https://doorway-alpha.elohim.host/qahal, https://doorway-alpha.elohim.host/community, https://doorway-alpha.elohim.host/shefa, https://doorway-alpha.elohim.host/avodah]
personas: [anonymous, Adam (signed in, fixture human)]
surface: app/lamad/src/app/components/lamad-layout/lamad-layout.component.ts, app/elohim-app/src/app/**/elohim-navigator.component.ts, pillar route files (qahal/community.routes.ts, shefa/shefa.routes.ts)
habit: none
walked: 2026-10-16 · deployed alpha · walkers anon-visitor, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/lamad --out recheck-pillar-nav-coherence --budget 180
---
# Pillar navigation coherence: decision

Navigation advertises places that do not exist or are not linked, and there is no stated way to move from one pillar to another. The decision here is applied in the views' own docs (shefa, governance-routes, community-qahal, avodah).

## Stations

- [ ] BLOCKS·H3 Point the Lamad context menu's Qahal entry at a page that exists
  - Persona / route: anonymous, `/lamad` context menu ("Qahal / Community & Governance")
  - Repro: 1. Load `/lamad`. 2. Open the "📚 Lamad ▾" menu. 3. Click "Qahal / Community & Governance". 4. Compare with loading `/qahal` directly and `/community`.
  - Expected (as persona): the community area opens.
  - Observed: `/qahal` shows "Page Not Found … drifted into uncharted space". The 404 page itself is good (Go Home / Explore Lamad / Go Back / suggestions); the dead end is that the switcher advertises it. Verify first: the walker loaded `/qahal` directly and did not click the menu item. The community area lives at `/community`, so the menu's target may already be `/community`; confirm the menu item's href and the click result.
  - Source: `lamad-layout.component.ts` / `elohim-navigator.component.ts` (`grep -rn "qahal" /projects/elohim/app/lamad/src/app/components /projects/elohim/app/elohim-app/src/app --include=*.ts`)
  - Owner class: component-fix
  - Depends on: none
  - Done when: clicking each menu entry from `/lamad` lands on a page whose status is 200 and whose title is not "Page Not Found" (recheck click rows); if `/qahal` should exist, add a redirect to `/community`.
  - Finding ids: ANON-16

- [ ] CONFUSES·H3 Decide how unbuilt or sign-in-only destinations are shown, and the pillar-to-pillar move
  - Persona / route: anonymous and Adam, all pillar pages
  - Repro: 1. From `/community` list the links to other pillars. 2. Repeat on `/shefa` and `/avodah`. 3. Try the unlabelled context-toggle square in each header.
  - Expected (as persona): a stated, consistent way to move sideways.
  - Observed: Qahal links to Lamad and Shefa but not Avodah; Shefa links to Lamad only; Avodah links nowhere. There is no pillar switcher beyond the unlabelled context-toggle square (in-page, opened nothing visible). Avodah's "View Projects" is its only content link.
  - Source: pillar header components under `app/elohim-app/src/app`; route tables `qahal/community.routes.ts`, `shefa/shefa.routes.ts`
  - Owner class: design-decision
  - Depends on: none
  - Done when: decision recorded in `genesis/a2o/walks/pillar-nav-coherence.md` (one placement for the pillar switcher; unbuilt or sign-in-only destinations are listed, greyed with a label, or hidden) and applied so `/community`, `/shefa` and `/avodah` each reach the other two pillars from the same control.
  - Finding ids: COMMONS-20

- [ ] CONFUSES·H4 Decide which pillar owns decisions about money in common and stop restating governance under three names
  - Persona / route: Adam, `/community` ("Governance" card), `/shefa/exchange`, `/shefa/constitutional`
  - Repro: 1. Load `/community` and read the Governance card. 2. Load Shefa and read the nav group labelled "Community" (Exchange, Insurance, Constitutional). 3. Open `/shefa/constitutional`.
  - Expected (as persona): one place for governance and one for exchange.
  - Observed: governance appears as the Qahal "Governance" card, Shefa "Constitutional", and a Shefa nav group labelled "Community", while Qahal is the community pillar.
  - Source: `shefa/shefa.routes.ts` nav group definitions; `grep -rn "Constitutional" /projects/elohim/app/elohim-app/src/app/shefa`
  - Owner class: design-decision
  - Depends on: none
  - Done when: decision recorded in `genesis/a2o/walks/pillar-nav-coherence.md` naming the owning pillar for each of governance and common-money decisions; the Shefa "Community" group is renamed or links into Qahal rather than restating it.
  - Finding ids: COMMONS-10
