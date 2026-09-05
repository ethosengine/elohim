---
title: "Two portals, shared like SSO — the doorway portal is the primary sign-in for hosted humans; apps are relying parties"
id: two-portals-sso-consolidation-design
status: Decided
class: protocol-canonical
context-tier: disclosed
steward: angular-architect
domain: D2
habits: [hosted-human-lifecycle]
graduation-trigger: the consolidation plan lands (elohim-app renders no password field and posts no registration; both identity routes redirect to the discovered doorway portal) and the hosted-human station-1 "registering because an application asked me" scenario passes on the household mesh — then this spec decomposes into the doorway/app gospel paragraphs it names
topic: [auth, portal, sso, oauth, imagodei, doorway, elohim-app]
cites:
  - "two-portals-sso-consolidation-plan | Two-portals SSO consolidation | sha256:613ab0adf9e530fc | path: genesis/docs/superpowers/plans/2026-09-05-two-portals-sso-consolidation-plan.md"
  - "hosted-human-lifecycle-e2e-plan | Hosted-human lifecycle E2E | sha256:17e945afeb8ea4ca | path: genesis/docs/superpowers/plans/2026-09-04-hosted-human-lifecycle-e2e-plan.md"
  - genesis/a2o/features/auth/oauth-authorization-code.feature
  - genesis/a2o/features/auth/auth-discovery.feature
  - genesis/a2o/features/auth/hosted-human/README.md
  - doorway/doorway-service/src/routes/auth_discovery.rs
  - doorway/doorway-service/src/routes/auth_routes.rs
  - app/elohim-app/src/app/imagodei/components/login/login.component.ts
  - app/imagodei-portal/src/app/app.component.ts
---

# Two portals, shared like SSO

**Decision (operator, 2026-09-05).** There are exactly two sign-in portals in the protocol:

1. **The doorway portal** — `doorway-app` served under `/threshold/*` — the identity provider for
   HOSTED humans, whose doorway keeps their credential and runs their cell. This is the PRIMARY
   portal today: every feature story about signing in, registering, staying signed in, and
   leaving is told against it (the hosted-human series).
2. **The p2p-native portal** — `app/imagodei-portal/` over elohim-storage's `/auth/me`,
   `/session`, `/session/exchange` — the identity provider for stewards whose own runtime holds
   their key. Today it is reached only by hand-off from the doorway portal (the steward
   redirect). It is the start of the graduation series, not part of the hosted one.

**Apps are relying parties, never portals.** elohim-app, the lamad bundle, doorway-app's own
dashboard, and any future EPR app sign a human in by (a) discovering where to send them from
`GET /.well-known/elohim-auth`, (b) redirecting to that doorway's `authorize` endpoint with
OAuth parameters, and (c) consuming the code on `/auth/callback`. An app never renders a
password field and never posts a registration. A session moves between apps on one doorway
through the existing single-use session-transfer pair.

**Why now.** The tree already declared "ZERO third portal" (the app login component's own
preamble; the doorway login handler's comment), yet elohim-app still carried a password card
posting straight to a doorway and a registration page posting straight to `/auth/register`.
That third portal drifted from the real one (snake_case keys dropped the display name; the
form gated on a conductor socket anonymous visitors cannot open; profile fields collected
at sign-up that belong to the profile surface), and it split the hosted human's story across
two surfaces. Clarity on the primary portal is what lets the feature stories serve elohim
core instead of each app's copy of auth.

**What is deliberately deferred.** Native in-app authentication for layered contexts — a
collective's inner membrane, or an extra lock on a sensitive set of files — is a real future
need. It is NOT a third portal: it is a second factor or a context gate layered on a session
the two portals already minted. Nothing in this consolidation forecloses it; it is out of
scope until the primary portal is settled and measured.

## The shape after consolidation

| Surface | Before | After |
|---|---|---|
| elohim-app `/identity/login` | resolver step → password card posting to a doorway, or OAuth start | resolver step (which doorway) → immediate redirect to that doorway's discovered portal via `authorize`, `login_hint` carrying the identifier. If a doorway is already proven (workspace / environment), no resolver: straight redirect. No password field. |
| elohim-app `/identity/register` | in-app form posting to `/auth/register` with profile fields | redirect to the doorway's `authorize` with `prompt=create`; the doorway sends the human to `/threshold/register` and returns them by OAuth. The route stays only as a redirector so old links keep working. |
| Visitor → hosted migration | in-app registration with session-derived profile fields | after the OAuth callback, if a visitor session exists, apply its display name / bio / interests to the profile through the profile surface and link the session. Registration itself happened at the portal. |
| `PasswordAuthProvider` (app) | login, register, refresh, me | retired. Refresh already rides `DoorwaySessionClient`; `me` has no remaining caller once the provider goes. |
| doorway `authorize` | unauthenticated → `/threshold/login?…` | unauthenticated + `prompt=create` → `/threshold/register?…` (same params). Everything else unchanged. |
| doorway portal register/login | as fixed on the 2026-09-04 branch | unchanged; it is the primary portal. |
| Native portal | hand-off target | unchanged by this consolidation. |
| Tauri desktop | native hand-off | unchanged. |
| a2o harness | signs a browser in by API + token injection; the browser login step drives the doorway portal | unchanged — nothing in the harness used the in-app forms. |

## Invariants this protects

- **One place ever sees a hosted human's password**: the doorway portal on the doorway's own
  origin. The `foreignIdentifierDomain` guard the app grew to stop mis-targeted password posts
  becomes unnecessary because the app no longer posts passwords.
- **Discovery, not configuration**: an app carries no auth endpoints; the discovery document
  is the only source, and it cannot name a foreign origin (`auth-discovery.feature`).
- **Callback is bounded**: every app is a registered OAuth client with an allowed callback; the
  hostile-callback refusal in `oauth-authorization-code.feature` stays the proof.
- **Profile is not auth**: bio, interests, location, reach are set on the profile surface after
  sign-in, never at registration.

## Out of scope, named

The DEV_MODE fleet posture; the native portal's own sign-in (proof-of-key) wiring; multi-context
native auth; the lamad bundle's guard (it already redirects to `/identity/login`, which becomes
the redirector — no change needed).
