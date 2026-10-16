---
view: footers and external links (landing footer, lamad footer)
urls: [https://doorway-alpha.elohim.host/, https://doorway-alpha.elohim.host/lamad]
personas: [anonymous]
surface: footer.component.html:15 (landing footer), app/lamad/src/app/components/lamad-layout/lamad-layout.component.html:28 (lamad footer)
habit: none
walked: 2026-10-16 · deployed alpha · walker anon-visitor
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/ --out recheck-footer-and-external-links --budget 180
---
# Footers and external links: persona walk

Two footers say different things, and the landing footer carries donation links that are broken or unlabelled.

## Stations

- [ ] CONFUSES·H9 Repair or remove the dead Coinbase link and give icon-only donate links accessible names
  - Persona / route: anonymous, `/` footer
  - Repro: 1. Load `/`. 2. Run the probe `externalLinks` check. 3. Read links i=10 and i=11.
  - Expected (as persona): working links with text or an accessible name.
  - Observed: the Coinbase checkout link returns HTTP 502; "Buy me a coffee" and the Coinbase button have no text (image/icon only, empty link text).
  - Source: `footer.component.html` (`grep -n "coinbase\|buymeacoffee" /projects/elohim/app/elohim-app/src/app/**/footer.component.html`)
  - Owner class: component-fix
  - Depends on: none
  - Done when: recheck `externalLinks` shows no 5xx and every footer link has non-empty text or `aria-label`; a dead payment link is removed rather than shown.
  - Finding ids: ANON-13

- [ ] POLISH·H4 Make the landing and lamad footers say distinct, visitor-appropriate things
  - Persona / route: anonymous, `/` and `/lamad`
  - Repro: 1. Read the landing footer. 2. Read the lamad footer.
  - Expected (as persona): one footer pattern, with developer facts kept out of the public view.
  - Observed: the landing footer lists "📚 Lamad Learning Platform", GitHub, a personal-name block and a git commit hash `0ebcc287`. The lamad footer is a single line "Lamad Learning Platform | Where Understanding is Earned | Powered by Elohim Protocol", and "Lamad is part of the Elohim Protocol" appears just above it (same idea twice in three lines).
  - Source: `footer.component.html:15`, `lamad-layout.component.html:28`
  - Owner class: design-decision
  - Depends on: walks__product-tokens-and-shell#1
  - Done when: decision recorded in `genesis/a2o/walks/footer-and-external-links.md` listing which footer facts are for visitors and which for developers (commit hash moved behind a developer affordance), and lamad's footer states "part of the Elohim Protocol" once.
  - Finding ids: ANON-14
