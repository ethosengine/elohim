---
view: plain-language pass (pillar names, jargon, provenance copy)
urls: [https://doorway-alpha.elohim.host/community, https://doorway-alpha.elohim.host/epr/elohim-protocol]
personas: [anonymous, Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/qahal (landing), epr-home components, plus any string table the decision creates
habit: none
walked: 2026-10-16 · deployed alpha · walkers anon-visitor, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/community --as Adam --out recheck-plain-language-pass --budget 180
---
# Plain-language pass: decision

Internal vocabulary reaches people who have not learned it: slugs, reach, steward, notarized, household counts, agent ids, pillar names. This doc fixes the convention; views apply it (see the Depends on lines in lamad-home, lamad-path-step, community-qahal).

## Stations

- [ ] CONFUSES·H2 Decide one naming convention for pillars and remove jargon from Qahal landing copy
  - Persona / route: Adam, `/community`
  - Repro: 1. Load `/community`. 2. Read the logo, H1, subtitle, footer, card text and button labels.
  - Expected (as persona): plain words, one name per thing.
  - Observed: the same concept has four names: "Qahal" (logo), "Community" (H1 and URL), "Community & Governance" (subtitle, repeated in the footer), and "Community" (Shefa nav group). "Explore Lamad (Content)" and "View Shefa (Economy)" gloss Hebrew names in parentheses, but Avodah ("Work management") has no such link from Qahal and no gloss anywhere in Avodah. Card text includes "Loomio-style deliberation with Polis-inspired sensemaking" and "Graduated Intimacy".
  - Source: `grep -rn "Loomio-style\|Graduated Intimacy" /projects/elohim/app/elohim-app/src`
  - Owner class: design-decision
  - Depends on: none
  - Done when: decision recorded in `genesis/a2o/walks/plain-language-pass.md` (display name for each pillar, who owns the glossary, which technical terms are banned from first-screen copy) and applied to the Qahal landing so no parenthetical glosses and no third-party project names appear in its first screen.
  - Finding ids: COMMONS-19

- [ ] POLISH·H2 Fix plurals and self-contradicting empty states in the EPR provenance copy
  - Persona / route: anonymous, `/epr/elohim-protocol`
  - Repro: 1. Load `/epr/elohim-protocol`. 2. Read the households section, the path line and the challenges line.
  - Expected (as persona): correct, consistent copy.
  - Observed: "Eden Household · 1 peers" (plural for one); "Not on a learning path yet" on a page whose type is Path (also shown to Adam, alongside "Practicing 20%"); "No challenges, no labels. Nothing is in question."
  - Source: `grep -rn "learning path yet\|Nothing is in question" /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: none
  - Done when: counts use proper pluralisation; the "not on a learning path" line is not shown on a node that is itself a Path; recheck page text contains none of the three phrases as they stand.
  - Finding ids: ANON-03
