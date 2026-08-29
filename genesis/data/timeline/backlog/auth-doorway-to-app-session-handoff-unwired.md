---
id: "backlog-auth-doorway-to-app-session-handoff-unwired"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A human who signs in at the doorway's portal arrives back at the app still signed out — the session-transfer mechanism exists at both ends and nothing connects them"
slug: "auth-doorway-to-app-session-handoff-unwired"
written: "2026-08-29"
author: "opus (measured on the household mesh via the new mesh-browser lane)"
status: "backlog"
priority: "high"
area: "app/imagodei"
domain: "protocol"
jobs: [elohim-app]
relatedNodeIds:
  - "concern:auth-discovery"
cites:
  - genesis/docs/superpowers/specs/2026-08-29-keep-client-control-surface-design.md
tags: [auth, session, handoff, doorway, portal, hosted-human, a2o]
---

# Both ends exist. The middle does not.

A hosted human's session belongs to their doorway. The doorway's portal owns the
password and mints the session. The app then needs that session — and today it
never gets one.

Every piece is already built:

- `GET /auth/session-token` mints a single-use transfer code, and
  `GET /auth/exchange-session` redeems it for a full JWT. Both verified working
  by hand against the household mesh.
- doorway-app already uses exactly this pattern for the STEWARD handoff
  (`threshold-login.component.ts`), with the right reasoning written down: the
  JWT must never ride a URL, so a single-use code goes instead.
- elohim-app already consumes `?session_token` — but only behind the `/account`
  route guard (`account/guards/account-guard.ts`), via `HandoffService`.

What is missing is the ordinary case. The portal's login ignores `returnUrl`
entirely and navigates to its own `/dashboard`, so a human sent there to sign in
is stranded; and if they do get back to the app, nothing on a normal landing
route redeems a code.

## Measured

`just test mesh-browser '@auth'` on the household mesh: 13 passed, 3 failed, 2
held. All three failures are the same assertion — `profile-bubble` never appears,
because the app is not authenticated after a successful portal sign-in:

- Matthew sees the Hosted Steward badge after OAuth login through doorway
- James sees the Hosted Visitor badge after OAuth login through doorway
- Agency badge upgrades when hosting account confirms stewardship

## What was tried, and why it was backed out

An attempt wired both halves: `returnUrl` support in the portal (same-origin
only, reusing the steward mint) plus an app-wide `provideAppInitializer` that
redeems `?session_token` wherever the human lands.

It made the lane WORSE, and the reason is the finding. The exchange SUCCEEDED —
the doorway logged the call and returned a valid JWT whose claims are
structurally identical to a login token — and the app still could not open a
chaperone connection: `Chaperone failed (401): Invalid or expired token`. So the
session was obtained and then not found by the component that needed it.

That is the token-path incoherence the Keep design already names: one seam,
three `SessionTokenStore` implementations, two key namespaces, plus a raw
`localStorage.getItem('elohim-auth-token')` literal in
`holochain-client.service.ts`. Wiring a fourth writer into that is how the next
person inherits a harder bug, so it was reverted rather than half-shipped.

## The fix

This is Keep slice 3's subject, not a patch: ONE token path, `openKeep`'s
Custodian composing the session client and the storage client over a single
store. Do that first; then the handoff is two small commits — `returnUrl` in the
portal, redemption at app bootstrap — and the three scenarios above become the
proof it worked.
