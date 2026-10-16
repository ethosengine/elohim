---
view: /epr/:id
urls: [https://doorway-alpha.elohim.host/epr/elohim-protocol]
personas: [anonymous, Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/**/epr-home (testids epr-home-open-in-bundle, epr-home-focal-open)
habit: epr-atom-home
walked: 2026-10-16 · deployed alpha · walkers anon-visitor, adam-shell, adam-learning, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/epr/elohim-protocol --out recheck-epr-atom-page --budget 180
---
# EPR atom page: persona walk

The atom page shows what a thing is, who holds it and how it is vouched for. A newcomer arrives here from the landing "Start the journey" and meets a ledger rather than a lesson (see walks__landing#1). Within its own terms it is legible, but it repeats controls and has a glyph that may not render.

## Stations

- [ ] CONFUSES·H4 Collapse the duplicated "Open in Lamad", "Network detail" and "Raw node" controls to one placement each
  - Persona / route: anonymous visitor, `/epr/elohim-protocol`
  - Repro: 1. Load `/epr/elohim-protocol`. 2. List controls with text "Open in Lamad", "Network detail", "Raw node" (probe `duplicateNav`).
  - Expected (as persona): one primary action to continue into learning.
  - Observed: two identical "Open in Lamad" buttons (header right, testid `epr-home-open-in-bundle`, and body left, testid `epr-home-focal-open`), "Network detail" three times (aside link, footer link, fold toggle) and "Raw node" twice.
  - Source: `grep -rn "epr-home-open-in-bundle\|epr-home-focal-open" /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: none
  - Done when: recheck `duplicateNav` lists no control text repeated with the same href, and the habit `epr-atom-home` still passes (if a testid is removed, update the feature that names it in the same change).
  - Finding ids: ANON-02

- [ ] POLISH·H4 Give the "Open in Lamad" arrow a rendering fallback
  - Persona / route: anonymous visitor, `/epr/*`
  - Repro: 1. Load `/epr/elohim-protocol`. 2. Read the "Open in Lamad" button label.
  - Expected (as persona): a visible arrow after the label.
  - Observed: Verify first: an empty box appears after the label ("Open in Lamad ▯"). The glyph is U+2197-class (not emoji), so it may be a headless font gap; the same arrow is used in Lamad source text "Open in Lamad ↗". Confirm in a browser with a normal system font before changing.
  - Source: `grep -rn "Open in Lamad" /projects/elohim/app/elohim-app/src /projects/elohim/app/lamad/src`
  - Owner class: component-fix
  - Depends on: none
  - Done when: the arrow is an inline SVG or a glyph with a declared fallback, and the capture shows no empty box.
  - Finding ids: ANON-04
