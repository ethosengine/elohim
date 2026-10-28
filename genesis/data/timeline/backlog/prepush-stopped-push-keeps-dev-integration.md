---
id: "backlog-prepush-stopped-push-keeps-dev-integration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A push stopped by TERM keeps dev-integration held, and the hook says it released it"
slug: "prepush-stopped-push-keeps-dev-integration"
written: "2026-10-28"
author: "claude-opus-5-5"
status: "open"
priority: "medium"
area: ".husky/pre-push.bash berth_release_claims / genesis/agentic/bin/berth release"
domain: "D5"
relatedNodeIds:
  - "habit:devices-share-one-berth"
tags: [berth, pre-push, dev-integration, lease, steward-space]
---

# A push stopped by TERM keeps dev-integration held

## Observed

On 2026-10-28 two pushes from `.worktrees/rules-digest` were stopped by a TERM to the
`git push` process group (at about 14:40Z and 15:12Z). Each ended its log with:

```
[pre-push] berth: dev-integration released here, but did not reach the channel — run: berth release dev-integration
```

Both times `berth status` still showed `dev-integration held by a60b1c22-… (verify, ttl 30m …)
— pre-push to dev`, so nothing was released locally either. The lease was claimed under the
pushing Claude session ("claimed by a60b1c22-… (session from envelope)"), not a hook mooring.
`berth release dev-integration --session a60b1c22-…` released it at once.

Logs: `genesis/local-dev/steward-space/push-rules-digest.log:3783` and
`push-rules-digest-2.log:3142`.

## Why it matters

The other device reads the lease as held until its ttl (30 min) lapses, so its pushes wait or
are refused for a lease nobody uses. The message also tells the operator the release happened
locally when it did not.

## Not yet known

Why `berth release` fails at the hook's exit. Candidates: the release runs while the group TERM
is still being delivered and is itself terminated, or the release resolves a different session
than the envelope session that claimed (the hook exports no `ELOHIM_SESSION_ID` on that path).

## Done when

A pre-push stopped by TERM (or INT) leaves no lease it took, proven by a test that sends the
group signal mid-gate. Any release that fails says so without claiming a local release.
