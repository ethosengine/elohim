---
view: app shell chrome (header bars, profile tray, floating page-info chip), every page
urls: [https://doorway-alpha.elohim.host/, https://doorway-alpha.elohim.host/lamad, https://doorway-alpha.elohim.host/shefa, https://doorway-alpha.elohim.host/community, https://doorway-alpha.elohim.host/avodah]
personas: [anonymous, Adam (signed in, fixture human)]
surface: app/elohim-app/src/app/**/elohim-navigator.component.{html,scss,ts}, app/lamad/src/app/components/lamad-layout/lamad-layout.component.{html,ts}, the page-info chip component (grep "click for details")
habit: none
walked: 2026-10-16 · deployed alpha · walkers anon-visitor, adam-shell, adam-learning, adam-commons
recheck: cd /projects/elohim/genesis/a2o && pnpm -s probe https://doorway-alpha.elohim.host/shefa --as Adam --out recheck-app-shell-chrome --budget 180
---
# App shell chrome: persona walk

The header, the profile tray and the floating "elohim-protocol" page-info chip appear on every page, so they are walked once here. The chip is the only element every page shares and it collides with content; the tray calls a signed-in fixture human "Traveler"; lamad and the pillars each stack their own bars. Whether Adam is recognised at all is the substrate question in walks__session-crosses-bundle-seam.

## Stations

- [ ] CONFUSES·H8 Give the page-info chip its own place so it never overlaps page content
  - Persona / route: anonymous and Adam, every page (clearest on `/avodah`, `/shefa`, `/community`, lamad step pages)
  - Repro: 1. Load `/avodah` and `/shefa` as Adam. 2. Look at the top-centre pill "elohim-protocol". 3. Load `/lamad/path/elohim-protocol/step/0` and look at the breadcrumb.
  - Expected (as persona): a page-info control that sits inside the bar or in a reserved slot, never on top of other content.
  - Observed: the pill floats over whatever is at top-centre: it covers the "Search A..." placeholder on avodah, "by Elohim Protocol" on shefa and community, and splits the lamad breadcrumb ("Elohim Protocol: Living Docu... / [chip] ..."). The chip overlaps the lamad host bar on every lamad page.
  - Source: `grep -rn "click for details" /projects/elohim/app/elohim-app/src /projects/elohim/app/lamad/src`
  - Owner class: component-fix
  - Depends on: none
  - Done when: a capture of each of the pages above shows no element hidden under the chip (bounding-box check in `capture.json` or visual), at 1280 px and 390 px.
  - Finding ids: SHELL-05, LEARN-07

- [ ] CONFUSES·H2 Make the chip say what page this is in words, not the same internal slug everywhere
  - Persona / route: anonymous, `/`, `/epr/elohim-protocol`, `/lamad`, `/qahal`, `/map`
  - Repro: 1. Load `/`, `/lamad`, `/map`. 2. Read the chip. 3. Click it and read the bar that opens.
  - Expected (as persona): a label that names this page or app in plain words, and a detail bar whose three buttons have icons or labels.
  - Observed: the chip reads "elohim-protocol" on every page including `/lamad`, `/qahal` (404) and `/map`. Clicking opens a bar "EPR elohim-host-landing" (landing) or "EPR elohim-protocol" with three small empty rounded buttons (no icons, no labels, no accessible names) and a hovering empty circle at top-right. Probe label is "Elohim Protocol - click for details". Verify first: empty buttons may be missing-glyph in headless; check accessible names in the DOM.
  - Source: `grep -rn "click for details" /projects/elohim/app/elohim-app/src`
  - Owner class: design-decision
  - Depends on: none
  - Done when: decision recorded in `genesis/a2o/walks/app-shell-chrome.md` (a "Decision" note under this station) stating what the chip shows (page title versus slug) and applied so the chip text differs on `/`, `/lamad`, `/map`; each button in the opened bar has an `aria-label`.
  - Finding ids: ANON-05

- [ ] CONFUSES·H6 Show the person's name and plain role in the profile tray instead of "Traveler", "Hosted visitor" and a session id
  - Persona / route: Adam, open the avatar tray on `/shefa`
  - Repro: 1. Load `/shefa` as Adam. 2. Click the avatar "T". 3. Read the tray.
  - Expected (as persona): "Adam" and what he is in plain words.
  - Observed: Adam appears as "Traveler" with avatar "T", adam@alpha.elohim.host, a Human ID like "session-...414k", "Steward" and "Hosted visitor" together, and "Connection Error". Verify first: Adam is a fixture human, so "Traveler"/"Hosted visitor" may be fixture data or a consequence of the missing session (walks__session-crosses-bundle-seam#1); decide the words regardless.
  - Source: `elohim-navigator.component.html:117-131`
  - Owner class: design-decision
  - Depends on: walks__session-crosses-bundle-seam#1
  - Done when: decision recorded in `genesis/a2o/walks/app-shell-chrome.md` for which identity words a person sees (display name, one role word, no session id, no hosting stage) and the tray shows Adam's display name; the words Traveler, Hosted visitor and `session-` do not appear in the tray text.
  - Finding ids: SHELL-06

- [ ] CONFUSES·H8 Give the profile tray an opaque surface
  - Persona / route: Adam, open the avatar tray on `/shefa`
  - Repro: 1. Load `/shefa` as Adam. 2. Open the tray. 3. Read the tray text over page content.
  - Expected (as persona): a menu that reads cleanly.
  - Observed: the tray is translucent; page text ("Disconnected", "35 Recognition Points", "2 appreciations", "contribu...") reads through the tray text; "Shefa Dashboard" is overprinted.
  - Source: `.profile-tray` styles in `elohim-navigator.component.scss`
  - Owner class: component-fix
  - Depends on: none
  - Done when: a capture with the tray open shows no page text through it (background is a solid or sufficiently opaque token with a shadow), in light and dark schemes.
  - Finding ids: SHELL-07

- [ ] CONFUSES·H4 Reduce lamad's two stacked header bars to one owner of sign-in and explain the "A" icon
  - Persona / route: anonymous and Adam, `/lamad`
  - Repro: 1. Load `/lamad` at 1280 px and 390 px. 2. Read the top two bars. 3. Click the "📚 Lamad ▾" switcher and the small purple "A".
  - Expected (as persona): one header, one place to sign in, labelled controls.
  - Observed: two stacked bars: "elohim.host … sign in" and "Lamad … search … A … T". The "A" is a tiny purple circle with no label beside the profile bubble (a theme toggle). The "📚 Lamad ▾" context switcher's menu changed nothing visible (PIXELS-ONLY). On phone both bars persist and the chip squeezes between them. Verify first: emoji-prefixed labels ("📚 Lamad ▾", "☀️ A") render blank in headless; check on a real font.
  - Source: `lamad-layout.component.html` in `/projects/elohim/app/lamad/src/app/components/lamad-layout/`
  - Owner class: design-decision
  - Depends on: walks__product-tokens-and-shell#1
  - Done when: decision recorded in `genesis/a2o/walks/app-shell-chrome.md` stating which bar owns identity and what the switcher switches to; `/lamad` shows one header (or two with distinct, labelled jobs) and the theme toggle has an accessible name.
  - Finding ids: ANON-20, LEARN-08

- [ ] CONFUSES·H4 Show a signed-out person as signed out (generic person icon, one sign-in affordance)
  - Persona / route: anonymous, `/lamad`
  - Repro: 1. Load `/lamad` signed out. 2. Look top right. 3. Click the avatar.
  - Expected (as persona): a person icon and a single "Sign in".
  - Observed: header has "sign in" (link to `/identity/login?returnUrl=%2Flamad`), then an indigo avatar circle "T" opening a menu with Sign in / Register / Theme / English. The avatar implies a logged-in person named T; sign-in is offered in two places; an unlabelled "A" badge (theme) sits next to it.
  - Source: `lamad-layout.component.html` in `/projects/elohim/app/lamad/src/app/components/lamad-layout/`
  - Owner class: component-fix
  - Depends on: walks__session-crosses-bundle-seam#2 (for a signed-in member the same avatar must show their initial)
  - Done when: a signed-out capture of `/lamad` shows a generic person icon (no letter), exactly one sign-in link outside the menu, and the avatar menu does not duplicate it; a signed-in capture shows the member's initial or name.
  - Finding ids: ANON-15

- [ ] CONFUSES·H4 Make the pillar header's controls do what they look like they do
  - Persona / route: Adam, `/community`, `/shefa`, `/avodah`
  - Repro: 1. Load each pillar as Adam. 2. Click "by Elohim Protocol". 3. Click the pillar logo-link ("Qahal", "Shefa", "Avodah") on its own page. 4. Look at the square next to the pillar name in light scheme.
  - Expected (as persona): links go somewhere or do not look like links; the toggle square shows what it is.
  - Observed: "by Elohim Protocol" is UNCLICKABLE on Qahal and Shefa (cursor says link) but navigates on Avodah; the logo link is a no-op (NO-EFFECT) on its own page; the context-toggle button is an empty dark square in the light scheme (chevron only visible in dark) and opened nothing visible. The chip overlap on the same header is its own station above.
  - Source: pillar header component(s): `grep -rn "by Elohim Protocol" /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: none
  - Done when: the recheck probe on each pillar reports no UNCLICKABLE/NO-EFFECT among the header controls except the current-page logo, which carries `aria-current`; the toggle is visible in light scheme with an accessible name and opens a menu.
  - Finding ids: COMMONS-12

- [ ] POLISH·H9 Degrade the chip silently when its context endpoint is missing
  - Persona / route: anonymous, `/epr/elohim-protocol` and `/lamad`
  - Repro: 1. Load `/epr/elohim-protocol`, click the chip, watch network. 2. Repeat on `/lamad`.
  - Expected (as persona): the detail bar opens with what is known, with no failing requests.
  - Observed: clicking the chip fires 404s: `/api/v1/epr/elohim-protocol/raw` on `/epr` and `/api/v1/epr/lamad/nav-context` on `/lamad`.
  - Source: `grep -rn "nav-context\|/raw" /projects/elohim/app/elohim-app/src --include=*.ts`
  - Owner class: component-fix
  - Depends on: none
  - Done when: recheck `httpErrors` after a chip click lists neither URL; either the endpoints exist or the chip does not call them for that page.
  - Finding ids: ANON-06

- [ ] POLISH·H4 Replace the blank grey square beside the pillar name and fix low-contrast header text
  - Persona / route: Adam, `/community`, `/avodah`, `/shefa`
  - Repro: 1. Load each as Adam. 2. Look at the dark bar.
  - Expected (as persona): a logo or nothing; legible secondary text.
  - Observed: a blank rounded grey square sits next to the pillar name; "by Elohim Protocol" and "Human Session" are low-contrast grey on navy; icon slots are blank on the Community cards and on buttons "Explore Lamad" and "View Shefa". Verify first: emoji/icon glyphs may be a headless font gap; the logo square is a separate element, check it in the DOM.
  - Source: pillar header component(s): `grep -rn "Human Session" /projects/elohim/app/elohim-app/src`
  - Owner class: component-fix
  - Depends on: walks__product-tokens-and-shell#6 (icon system)
  - Done when: the square holds a logo or is removed; header secondary text meets 4.5:1 contrast against its background; capture confirms.
  - Finding ids: SHELL-10

- [ ] POLISH·H4 Stop the Material icon ligature name "face" leaking as text in the pillar header
  - Persona / route: Adam, `/community`, `/shefa`, `/avodah`
  - Repro: 1. Load a pillar as Adam. 2. Read the header text via the probe (`results` rows 3, 4).
  - Expected (as persona): a face icon, and a name or initial for the avatar bubble.
  - Observed: the probe reads the header as "face Human Session": "face" is the Material icon ligature name appearing as text when the icon font is unavailable. The avatar bubble reads "T" for Adam (the identity part follows walks__session-crosses-bundle-seam#2 and walks__app-shell-chrome station on the tray).
  - Source: `grep -rn ">face<\|face" /projects/elohim/app/elohim-app/src --include=*.html | head`
  - Owner class: component-fix
  - Depends on: none
  - Done when: the header's text content (probe row) contains no ligature word; the icon is an SVG or the font has `font-display` and the label is `aria-hidden` when it fails.
  - Finding ids: COMMONS-13
