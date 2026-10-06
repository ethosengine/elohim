---
id: "backlog-genesis-seeders-read-node-agent-key-from-closed-auth-me"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis seeders learn a node's agent key from `/auth/me`, which no longer answers a caller from another machine"
slug: "genesis-seeders-read-node-agent-key-from-closed-auth-me"
written: "2026-10-06"
author: "claude-opus-5-5 (overnight shift on the device-consent landing, 2026-10-06)"
status: "backlog"
priority: "high"
ci_status: "blocked"
tags: [genesis, seeder, identity, auth-me, custody, device-consent, regression]
cites:
  - elohim/elohim-storage/src/http.rs
  - genesis/seeder/src/seed-commitments.ts
  - genesis/seeder/src/seed-provide-rows.ts
  - genesis/seeder/src/seed-household-formation.ts
  - genesis/seeder/src/seed-household-costeward.ts
  - genesis/seeder/src/seed-drill-fixtures.ts
  - genesis/seeder/src/seed-delegates-compute.ts
  - genesis/seeder/src/peer-id.ts
---

# Genesis seeders read a node's agent key from a route that is now closed to them

## Symptom (measured, elohim-genesis #1624, 2026-10-06)

`Seed Custody Commitments` failed where it passed on #1623. Every pair was skipped:

```
[?] matthew-manager→jessica-spouse: SKIPPED (unresolvable on this mesh) —
    human-matthew-manager: /auth/me returned HTTP 401
=== Results: 0 created, 0 already-exists, 7 skipped (7 total) — total failure ===
```

Two legs of `Verify Substrate Propagation` then failed as a consequence
(`propagation.custody-manifest`, `propagation.custody-convergence`: no custody-blob commitment on
matthew, adam or jessica after 300 s). The byte-moving legs of the same stage passed.

## Cause (read from source)

The device-consent landing (dev `0b85b7c38`) closed the node's open session routes. In
`handle_auth_me` → `resolve_local_session`, a request with no session cookie receives the node's
active session only when `caller_is_local`; any other caller gets 401. That is the intended
hardening (device-recognition backlog, "who is on the node's machine").

The seeders call `GET http://<pod>:8090/auth/me` from the CI pod with no cookie and read
`agentPubKey` (and, in five places, `humanId`) from the answer. They depended on the fallback that
was closed. Call sites: `seed-commitments.ts` `resolveCustodyPeerIds`, `seed-provide-rows.ts`,
`seed-household-formation.ts`, `seed-household-costeward.ts`, `seed-drill-fixtures.ts`,
`seed-delegates-compute.ts`. Only the custody stage is proven red by a build; the others share the
call and were already UNSTABLE for other reasons before the landing, so their part is not separated.

## The design question (operator's)

How does a CI seeder, on another machine, learn which agent a household node speaks as?

| Option | What changes | New exposure | Cost |
|---|---|---|---|
| A. `/p2p/status` also carries the node's own `agentPubKey` | storage view + schema + codegen; seeders read it there | agent key beside the peer id on a public route — the pair an `AgentPeerBinding` already notarizes; no person name, no human id | one edge roll; `humanId` readers need another source |
| B. The seeder presents a credential the node accepts for this read | storage authorizes an operator key on `/auth/me`; CI passes it | none public; a new authority on a hardened route | one edge roll; a secret in more CI steps |
| C. The seeder signs in to each node as its person | seeders only | none | a per-node secret in CI for every household human; no edge roll |
| D. The seeder reads the key from the conductor it already reaches | seeders only (`CONDUCTOR_URLS`, as `seed-conductor-identities.ts` does) | none | depends on CI reaching each conductor's app interface |

Recommendation: A for `agentPubKey`, with the five `humanId` reads replaced by the fixture's own
declared human id (the seeder already knows whom it is seeding for; the read was a cross-check).
A publishes nothing the DHT does not already hold and adds no authority to a hardened route. It
was not applied by the shift: it adds to the identity surface the operator ruled on, and every
storage-side option rolls the fleet again.

## Not the cure

Reopening the active-session fallback for remote callers. It restores the seeders and the hole.

## Probe

`elohim-genesis` `Seed Custody Commitments` green with `created + already-exists = 7`, and
`propagation.custody-manifest` / `propagation.custody-convergence` passing in the same build.
