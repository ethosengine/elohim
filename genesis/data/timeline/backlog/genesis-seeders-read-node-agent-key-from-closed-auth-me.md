---
id: "backlog-genesis-seeders-read-node-agent-key-from-closed-auth-me"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis seeders learn a node's agent key from `/auth/me`, which no longer answers a caller from another machine"
slug: "genesis-seeders-read-node-agent-key-from-closed-auth-me"
written: "2026-10-06"
author: "claude-opus-5-5 (overnight shift on the device-consent landing, 2026-10-06)"
status: "wip"
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

## Decision (2026-10-06, option D)

A seeder learns which agent a node speaks as from that node's own conductor, never from a session
route. Which agent a node speaks as is a fact about the node's own cell, and the conductor is where
that fact lives. `/auth/me` answers a different question (which person is signed in here); the
seeders were reading the first question through the second.

One shared resolver, `genesis/seeder/src/node-identity.ts` (`createNodeIdentityResolver`,
`resolveNodeIdentity(humanId) -> { agentPubKey, embodiedHumanId | null }`), connects to the human's
own conductor (a `name=url` CONDUCTOR_URLS entry or the `elohim-<name>-<env>` host; a human with no
such entry resolves to nothing, never the first conductor that answers), reads the steward app's
`agent_pub_key` from AppInfo and the Human the cell embodies (`get_my_human`), caches per run,
closes its sockets, times out each connect, and fails naming the human and the conductor URL. All
six `/auth/me` reads now go through it (seed-commitments, seed-provide-rows,
seed-household-formation, seed-household-costeward x2, seed-drill-fixtures,
seed-delegates-compute). There is no fallback to `/auth/me`. The genesis Jenkinsfile exports
CONDUCTOR_URLS (from `getConductorAppUrls()`) to the custody and provide-rows legs; household
formation already ran under the probe-then-seed helper, which exports it.

Honest cost: this leans on CI's reach to conductor admin and app sockets, which the genesis
Jenkinsfile already names as bootstrap debt. It adds a reader of that reach, not a new authority.

Rejected:

- **A. Publish `agentPubKey` beside the peer id on `/p2p/status`.** It would hand any outside
  observer a join between two identity namespaces that today is notarized only inside the network
  (`AgentPeerBinding`), it needs a fleet roll, and it adds surface to fix a consumer that was
  asking the wrong place.
- **B. An operator credential storage accepts on `/auth/me`.** A new authority on a route that was
  just hardened, plus an edge roll.
- **C. The seeder signs in to each node as its person.** A per-node secret in CI for every
  household human: CI would become a device that speaks for each person with no enrollment.

## Not the cure

Reopening the active-session fallback for remote callers. It restores the seeders and the hole.

## Probe

`elohim-genesis` `Seed Custody Commitments` green with `created + already-exists = 7`, and
`propagation.custody-manifest` / `propagation.custody-convergence` passing in the same build.
