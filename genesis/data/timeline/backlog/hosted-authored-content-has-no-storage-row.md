---
id: "backlog-hosted-authored-content-has-no-storage-row"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Content a hosted person authors directly on their cell never becomes a row in any storage"
slug: "hosted-authored-content-has-no-storage-row"
written: "2026-10-04"
author: "claude-opus-5-5 (integrator, shem), from the Che workspace's campaign 1.4 leg 2 finding"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-hosted-app-coordinator-coverage-gaps"
  - "habit:dataplane-convergence"
tags: [hosted-apps, projection, head-plane, plane-separation, p2p-design-gate]
---

Found 2026-10-04 in campaign 1.4 leg 2. The operator's hosted app on alpha conductor-4 authored a
root (`fct-leg2-operator-root`) by calling `create_content` on its own cell. The Che workspace then
adopted that root on its cell: lineage, local head and earned canonical election all read back
correctly. `GET /db/content/fct-leg2-operator-root` is 404 on Che's storage and on the alpha
doorway, and stays 404. The leg 2 run refuses to start without the row.

Marks: **[V]** read in the code here, **[R]** reported by the Che workspace, not re-checked.

## Why no storage holds the row

Three facts, each correct on its own:

1. **Storage hears one app's signals.** Its signal subscription is an app websocket authenticated
   with a token for the node's own app (`elohim/elohim-storage/src/hc_client.rs`,
   `issue_app_auth_token` with `config.app_id`). A hosted app's `ContentCommitted` signal is never
   delivered to it. **[V]** A hosted cell has no storage of its own. **[R]**
2. **A head declaration never creates a row.** The signal arm stamps an existing row only
   (`rea_projection.rs`, the `ContentHeadDeclared` arm: "a HEAD declaration for a row this node
   never seeded is a no-op") and so does the reconcile path
   (`p2p/projection_reconcile.rs::project_authenticated_content_head`). **[V]**
3. **Acquisition fills only what a peer already holds.** The reconcile sweep discovers ids from
   peers' inventories; its own doc says "absent → the acquisition plane's job; never fabricated".
   No path creates a row from a conductor-verified head alone. **[V]** (The fill step itself was not
   traced to confirm it inserts.)

Together: a root authored on a hosted cell outside a storage write path has no way to become a row
anywhere.

## The missing station

`chain: a delegate publishes updates to a hosted person's root / between "a hosted agent authors a
root" → "a delegate publishes an update to it" / missing node: some storage holds a row for that
root / probe: GET /db/content/<root id> on the hosting node's doorway returns 200 after the hosted
cell's create_content / current state: 404, by construction.`

## What was done (2026-10-04, same day)

Re-authoring the root through a storage write path was considered and dropped: storage's write
path calls `create_content` through the node's own app (`conductor_writes::call_create_content`
on storage's `HcClient`), so the root's author would be the node's agent, not the hosted person.
**[V]** That changes whose root it is, which is the thing leg 2 is proving.

Narrow fix instead, in `rea_projection.rs::apply_ordered_content_head`: when this node's OWN cell
declares an earned canonical head (an ordered `ContentHeadDeclared`, which storage only receives
from its own app) and the conductor has resolved and authenticated the exact payload, a missing
local row is seeded from that verified head and then stamped through the usual canonical guard.
The adopting node holds the head, so it holds the row. Unit test:
`a_head_this_cell_declared_seeds_the_row_when_no_local_row_exists`.

Unchanged on purpose: the legacy unordered signal arm and the reconcile sweep still never insert,
and a node that has not itself declared the head still gets no row.

To use it on an already-declared head: rebuild and restart storage, then repeat the declaration.
An idempotent declaration still emits the signal (`content_store`, the declare extern's doc).

P2P design gate, short form: no new entity, entry type, route or wire message. Content stays
Notarized (A) with the DHT as truth; this changes only when its SQLite projection is created.
Head-plane cost: one row per foreign-authored head a node's own cell adopts. Reach is carried
from the verified entry unchanged.

Still open below: a node whose cell has NOT declared the head (the hosting node itself, any
third peer) holds no row, so the root is still invisible there until the adopter's row is
advertised and acquired. Not proven on a household or the fleet; the a2o story is owed.

## Receipt, 2026-10-05: the adopter's row is seeded on Che

Che's storage (build 52727b6d1, iroh) held no row after two declarations. Cause, read from its log
and `main.rs`: every conductor signal subscriber sat inside the `Ok` arm of the peer-policy load,
and Che's storage, launched by hand with no `./config/peer-policy.toml`, logged only
`PeerStatus heartbeat disabled: policy config load failed`. Neither run logged
`ReaProjectionSignal subscriber registered`, so the declaration signal had no reader. The zome was
not the cause: `declare_canonical_head_inner` writes a new link and emits `ContentHeadDeclared`
with its ordering on every call, including a repeat.

With a policy file in place and storage restarted (same binary, same environment), all four
subscribers registered. Che then repeated the declaration with the acceptance already on disk and
its own copy of the root record (`fleet/redeclare-root-on-che.mts`; no new grant, no operator-cell
connection). Storage logged `row seeded from the conductor-verified head` at 01:57:58Z and
`GET :8095/db/content/fct-leg2-operator-root` returns 200. An accepted head survives its
delegation's expiry for that exact version, as `verify_accepted_publication` states.

The trap is closed in code on `fix/coordinator-acceptance-contract`: a missing or malformed policy
file now runs the node under the built-in policy and the subscribers always register.

Still owed from this incident: a declaration made while storage is down, or before its subscriber
is up, leaves no row and nothing retries it. A seed that does not depend on a fresh signal (at
boot or on the sweep, for heads this node's own cell holds as earned canonical with no row) is
the same decision as "acquisition by id" below.

## Before designing a fix

This is not the coordinator sweep's walk reused. That sweep makes admin calls per cell; projecting
hosted content needs storage to receive each hosted app's signals, or to be told about the content
some other way. Questions the design has to answer first, under the P2P design gate and the
plane-separation spec (`genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md`):

- Whose row is it? Which of a hosted person's content should the hosting node project into its own
  database, and at what reach. Peer inventories advertise only `community`, `public` and `commons`.
- What does it cost? One signal subscription per hosted app, on conductors where gertrude and eve
  were CPU-throttled above 90% on 2026-10-04.
- Is direct authorship on a hosted cell a supported path at all, or should hosted writes only go
  through the doorway's storage write path, with a direct `create_content` refused or documented
  as unprojected?
- If projection is wanted, the smaller shape may be acquisition by id: storage resolves a named
  content id through its own lamad cell (the DHT read is already authenticated) and inserts the row
  from that verified head. The leg 2 runbook forbids creating the row by hand; a verified
  acquisition is a different act, but it needs the same decision about whose row it is.

Needs a storage change, an edge roll and a household proof whichever shape is chosen.
