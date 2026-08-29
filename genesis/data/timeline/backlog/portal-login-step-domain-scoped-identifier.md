---
id: "backlog-portal-login-step-domain-scoped-identifier"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The deployed doorway names a signed-in hosted human with its gateway domain (`portal-a2o-<uuid>@alpha.elohim.host`) while the portal-login step asserts the bare registered name — the Act II sign-in scenario fails on an identifier convention, not on sign-in (the session was real)"
slug: "portal-login-step-domain-scoped-identifier"
written: "2026-08-29"
author: "m4-fleet-confirm shift (close)"
status: "open"
priority: "medium"
jobs: [elohim-genesis]
cluster: "arch-frontend-bundle-seams-backlog"
tags: [a2o, portal, auth, doorway, domain-scoping, act-ii]
---

genesis #1519 (fleet, playwright leg): `doorway-portal-login-neighbourhood.feature` "The deployed portal renders and
the doorway signs the human in" → `AssertionError: the doorway named "portal-a2o-<uuid>@alpha.elohim.host" for the
portal's token, not the human who signed in ("portal-a2o-<uuid>")`. The wrong-password twin PASSED, and the four
`auth-discovery-neighbourhood` scenarios PASSED (the API leg now reports the portal contract on every genesis run).
The deployed doorway scopes identifiers to its domain (see `threshold-login-domain-scoping.feature`); the household
mesh doorway does not, so the step `the doorway confirms a session for that human`
(`genesis/a2o/steps/ui/doorway-portal-login.steps.ts`) compares the bare name. Cure: the step accepts
`<name>` or `<name>@<doorway host>` (derive the host from `E2E_DOORWAY_ALPHA`), or the registration step records the
identifier the doorway itself returns. Steps are judge surface — not edited inside the shift that found this.
