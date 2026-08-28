---
id: "backlog-doorway-portal-domain-suffix-not-submitted"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The doorway sign-in portal renders an @domain suffix it never submits — the human is told they are name@doorway while the doorway is sent bare name, and the doorway resolves nothing"
slug: "doorway-portal-domain-suffix-not-submitted"
written: "2026-08-28"
author: "opus (found by the first browser e2e that asserts the auth RESULT, not just the render)"
status: "backlog"
priority: "high"
area: "doorway/auth"
domain: "protocol"
jobs: [elohim-holochain]
relatedNodeIds:
  - "concern:doorway-portal-login"
tags: [doorway, auth, portal, threshold-login, identity, ux-contract, a2o-found]
---

# The suffix the portal shows and the identifier it sends are different strings

**Root fact.** `threshold-login.component.ts` renders a domain beside the identifier input —
`<span class="domain-suffix" data-testid="threshold-domain-suffix">@{{ gatewayDomain() }}</span>`
(:105-107) plus the hint *"Logging in at **{{ gatewayDomain() }}**"* (:109-112), where
`gatewayDomain()` is `window.location.hostname` with a leading `doorway-` stripped (:219-223).

The submitted body does not carry it. Measured on the local mesh 2026-08-28 by intercepting the
request in a real browser:

```
domain suffix shown: @localhost
>>> REQUEST BODY: {"identifier":"dbg3-1787950430538","password":"…"}
<<< 401 {"error":"Invalid credentials","code":"INVALID_CREDENTIALS"}
```

The human typed `dbg3-…`, read "you are signing in at localhost", and the doorway received
`dbg3-…` with no domain.

**And the doorway resolves nothing** — it stores and matches identifiers verbatim. Same day,
same doorway:

| registered as | logged in as | result |
|---|---|---|
| `bare-1787950454` | `bare-1787950454` | **200** + token |
| `bare-1787950454` | `bare-1787950454@localhost` | **401** INVALID_CREDENTIALS |
| `dbg-…@localhost` | `dbg-…@localhost` (via API) | **200** + token |
| `dbg-…@localhost` | typed local part in the portal | **401** |

So the two halves disagree: the portal's UI describes a domain-scoped identity
(`name@doorway`) while its wire contract is "whatever you typed is the whole identifier."

## Why it matters

A human registered with a domain in their identifier — which is the fleet convention
(`matthew.dowell@alpha.elohim.host`, `genesis/a2o/features/auth/auth-lifecycle.feature`) —
cannot sign in through the portal by following what the portal tells them. They must type their
FULL address into a field that then displays a second domain after it, reading as
`matthew.dowell@alpha.elohim.host@alpha.elohim.host`. The suffix is not merely decorative; it
is instructions, and the instructions are wrong.

This also silently couples the portal to how a doorway's humans happened to be seeded. Nothing
in the code says which convention is correct.

## The decision this needs (operator/architect, not a bugfix)

Three coherent resolutions; pick one, because today's state is none of them:

1. **The portal composes** — submit `${typed}@${gatewayDomain()}`. Matches what the UI promises.
   Breaks every human registered bare. Requires an identifier migration.
2. **The doorway resolves** — accept a bare local part and resolve it against the doorway's own
   domain before matching. Keeps both conventions working. Needs care that it cannot be used to
   collide two humans onto one record.
3. **The suffix is removed** — the identifier is opaque and the UI stops claiming otherwise.
   Cheapest and honest, but gives up the `name@doorway` federated-identity affordance that
   `threshold-login-domain-scoping.feature` and the "use a different doorway" link imply is
   intended.

## Evidence

Found by `genesis/a2o/features/browser/doorway-portal-login.feature`
(`@concern:doorway-portal-login`), the first browser scenario to assert the auth RESULT rather
than the render — the token the portal stores is replayed to `GET /auth/me` and the doorway
must name the same human. A render-only assertion passes against this defect, because the form
paints perfectly.

The a2o steps currently register their throwaway human under exactly the string the portal
submits, with the reasoning recorded inline
(`genesis/a2o/steps/ui/doorway-portal-login.steps.ts`). **When this row is resolved, that
registration and its comment are what must change** — the test encodes today's contract on
purpose, so a fix to the contract fails it loudly rather than passing by accident.
