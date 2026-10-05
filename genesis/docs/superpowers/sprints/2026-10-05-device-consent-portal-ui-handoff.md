---
title: "Handoff — sign-in portal polish and the device approval screen"
id: device-consent-portal-ui-handoff-20261005
date: 2026-10-05
status: handoff
author: "claude-fable-5-1 (Che workspace), at the operator's direction"
habits: [hosted-human-lifecycle]
cites:
  - genesis/docs/superpowers/specs/2026-09-05-two-portals-sso-consolidation-design.md
---

# Handoff — sign-in portal polish and the device approval screen

Branch `feat/device-consent-portal-ui`, based on `origin/dev` at `52727b6d1`. Not merged, not
pushed. The story it serves is `genesis/a2o/features/auth/device-consent-grant.feature`, which
lives on `feat/device-authorization-grant` (not yet on `dev`). Marks: **[V]** verified here,
**[P]** my proposal, not agreed with any backend.

## What is on the branch

1. `<elohim-imagodei-device-consent-card>`: the consent screen as one shared element. **[V]** 68 tests.
2. `<elohim-imagodei-witness-trail>`: who is vouching while a sign-in or approval resolves. Shows
   only the steps it is given; `this-device` leads; `others` is a count and never a name; nothing
   is shown for an instant result and it adds no delay. **[V]** 114 tests.
3. Library A stories for both, designed scenes for device approval, and a repaired designed
   first-time sign-in scene.
4. `doorway-app`: sign-in and register restyled onto the protocol tokens with behaviour, routes and
   every a2o selector unchanged; `/threshold/consent/device`; the witness trail in place of a
   spinner after clicking Sign in and during an approval. **[V]** production AOT build, 154 unit
   tests, eslint, prettier.
5. `hc-mesh.sh` builds the element packages before it serves the portal.

Operator's rule for the trail: the sign-in form stays plain. Enrichment only takes the place of a
wait after the click, and the return to the app is never held.

## What does not work yet

- **Approving a device end to end.** The page calls `POST /auth/consent/view` (exists in the peer
  runtime on `feat/device-authorization-grant`; the doorway does not mount it) and
  `POST /auth/consent/agree` **[P]** (exists nowhere). Proposed agree shape, in
  `doorway-app/src/app/services/device-consent.service.ts`: request `{ request, agreedActs }`,
  response `{ returnTarget: { kind: 'display', value } | { kind: 'redirect', url }, expiresAt,
  witnesses? }`. Until a doorway serves both, the page shows that it cannot approve devices yet.
- **The link format.** The page reads the terminal's request from `?request=<base64url JSON>`
  **[P]**. The command-line side is not written; the decode is one pure function.
- **The witness feed.** No backend reports who signed or who has seen a record. Today a hosted
  sign-in shows one true step, the doorway. The optional `witnesses` field **[P]** is where a
  backend would add the rest.
- **The native portal.** `app/imagodei-portal` does not mount either new element. That is where
  the `this-device` alone case belongs (a person whose own node holds their key).

## Before landing

- The story file is on another branch. Land this after, or with, `feat/device-authorization-grant`.
- First edge build: confirm the "Build Doorway App" stage's filtered install brings in what the
  element packages need to build (the portal's `prebuild` runs them).
- Not rendered against a live doorway while signed in; signed-in phases were rendered from a
  development-only preview route that is absent from the production bundle **[V]**.
- Hebrew layout was checked in the page, not by eye: this workspace has no Hebrew font.
- The sign-in form now stays on screen while the request is in flight (it used to give way to a
  loading block). Run the browser lane for `threshold-login-domain-scoping` and
  `steward-login-portal-handoff` before merging.

## Findings left alone

- `elohim-imagodei-trust-indicator` hard-codes "(flywheel)", "Hosted via" and "Your conductor";
  a host cannot supply its own words.
- `elohim-imagodei-federated-resolver` documents `--elohim-input-border` as a border but uses it as
  a colour; its visible strings are hard-coded English.
- The existing consent card's `PeerConductorContext` story sets `trust-mode`, but the element
  listens on `trustmode`, so that story never shows the peer case.
- 29 tests in `elohim-imagodei` (`portal-shell`, `federated-identifier`) fail on `dev` already.
- The `brand-vocabulary-boundary` advisory flags the `elohim-imagodei-*` tag names; the new
  elements follow the package's existing naming.
- doorway-app's landing page and operator dashboard still use the old indigo palette under the new
  toolbar.
- A return address is not carried through the steward hand-off or through Create account.
- Create account shows no witness line: the one sentence available claimed the account was being
  recorded where anyone can check it, which nobody has confirmed.
- The `hc-dev-orchestrator` skill prose does not mention the portal's element build step.
