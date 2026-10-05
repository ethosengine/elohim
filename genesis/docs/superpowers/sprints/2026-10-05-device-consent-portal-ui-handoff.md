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

## State at the end of 2026-10-05 (supersedes the first pass's "what does not work yet")

Two branches, neither pushed, held at the operator's direction:

- `feat/device-consent-portal-ui` (this one): the approval flow, identity standing and "begin your
  identity on this device" are one host-agnostic piece in `elohim-imagodei`, mounted by both
  portals. The portal is not owned by the doorway.
- `feat/device-authorization-grant`: the ceremony in the peer runtime and the `epr` CLI. Stories:
  `genesis/a2o/features/auth/device-consent-grant.feature` and
  `device-provisioning-paths.feature` (19 scenarios, all `@wip`, no step definitions).

### What works, and how far it was proven

Proven by gates (`consent-grant`, `elohim-storage`, `eprfs`, mishpat unit tests, the
`device_enrollment` sweettest) and by live runs of three agents on ONE isolated stock 0.7.0
conductor, each with its own storage process:

- A person begins an identity on their own node with no doorway: `POST /auth/identity/begin`,
  `epr identity begin`, or a declaration file (`ELOHIM_IDENTITY_DECLARATION_PATH`) the node
  reconciles at start and on change. It never overwrites or re-keys an existing identity.
- A device asks (`epr device ask`), the person approves on a node that speaks for them
  (`/auth/consent/agree`, `epr device approve`, or by declaration), the code returns, and the
  device verifies what it collected and enrolls itself (`epr device redeem`).
- The link and code can be carried by hand, or travel a private network: an asking node announces,
  a node that speaks for a person lists it (`epr device pending`), and the code returns the same
  way. Carried over libp2p local discovery; absent in iroh-only mode.
- One approval is enough; nothing waits on other nodes. A witness beat and a decider seam exist as
  named places where an attending elohim will act; today both pass through.
- Signing as the person is accepted only from the node's own machine.

Not proven: two conductors over a real network; the pinned fork conductor; Che with a deployed
conductor (needs the coordinator zome on alpha, which needs a push); the portal pages against a
live node; discovery across separate hosts; iroh-only and dual modes.

### Vocabulary (operator rulings, 2026-10-05)

- The person is the steward of their identity. A node is never a steward; it speaks for a person.
- Three things about a node are separate: whose it is, who operates it, whether it speaks for that
  person. Joining does not make a node speak for anyone; that is a separate act.
- Extra signers and witnesses are optional and affirm later; a required number is an opt-in.
- A doorway only optionally facilitates. No provisioning path may need one.
- A node that already began an identity: forget and start fresh (low stakes), remember and just
  join, called adopt (low stakes), or reconcile (high stakes, unbuilt, see
  `genesis/data/timeline/roadmap/reconciling-networks-that-grew-apart.md` on the grant branch).

### Open, with a home

`genesis/data/timeline/backlog/arch-device-recognition-backlog.md` and
`security-node-session-routes-unauthenticated.md` on the grant branch hold these in mintable
shape. The ones that change what a person can do today:

- The node's session routes take no proof of the person and are reachable from the network. Only
  the signing routes are closed (this-machine-only). So a browser on another machine cannot
  approve, and the native portal's sign-in form does not work on a person's own node.
- The node cannot tell the person from whoever operates the machine.
- Nothing writes a second node that speaks for a person, so "held between two" and a required
  number of approvals cannot be exercised. Later affirmation is not built.
- `device.bind-root` is recorded in the consent and binds nothing. A revoked device cannot
  re-enroll through the ceremony.
- The recommended-choice flow for a node with its own identity (start fresh, adopt, the node's
  confirmation) is in the story and not built; only joining as it is exists.
- The doorway's optional mount of the agree step, and the doorway relay carrier, are not built.
- Brand tokens are not bound in `app/imagodei-portal`.

## Before landing

- Land the grant branch first or together; this branch's pages are coded against its routes.
- First edge build: confirm the "Build Doorway App" stage's filtered install brings in what the
  element packages need to build (the portal's `prebuild` runs them).
- Run the browser lane for `threshold-login-domain-scoping` and `steward-login-portal-handoff`:
  the sign-in form now stays on screen while the request is in flight.
- The Angular dev-server cache under `/projects/elohim/.angular/cache/` is shared by every
  checkout and served stale element bundles during this work; production builds are unaffected.
- No habit atom covers this work yet, so no delta was recorded.

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
