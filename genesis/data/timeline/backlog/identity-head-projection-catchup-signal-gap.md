---
id: "backlog-identity-head-projection-catchup-signal-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "did:elohim answers NeverDeclared for a missing identity_heads row because the mishpat→identity_heads projection carries no caught-up signal — a never-delivered declaration is indistinguishable from one that was never made"
slug: "identity-head-projection-catchup-signal-gap"
written: "2026-08-02"
author: "rust-architect (did-bridge downstream closure)"
status: "backlog"
priority: "medium"
relatedNodeIds: []
tags: [did-bridge, identity, identity-heads, mishpat, projection, caught-up, gossip, fail-closed, honest-absence, C4, storage]
cites:
  - elohim/elohim-storage/src/services/did_identity_store.rs
  - elohim/elohim-storage/src/db/identity_heads.rs
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/signals.rs
  - elohim/elohim-storage/src/projector/status.rs
  - elohim/elohim-storage/src/p2p/replication.rs
  - bridges/did/did-bridge/src/did_elohim.rs
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
---

# The missing-row ruling is implemented as honest degradation, not as the ruling

`DidIdentityStore::identity_head` answers `IdentityHeadAnswer::NeverDeclared` when
`identity_heads` has no notarized row for an `agent_cid`. `NeverDeclared` is a
**positive claim** in the did-bridge contract — *we looked, and no declaration
exists* — and it is the one answer that assembles the phase-1 implicit-self
document (no `controller`, which DID 1.1 reads as *the subject controls it*).

`identity_heads` is a **gossip-fed projection**. So strictly, a missing row means
"no `binds-identity` declaration has been **delivered to this node**", which
establishes never-declared only while the projection is caught up. On a full-arc
fleet every zome `get`/`get_links` is local-only, so a miss is a fact about
gossip, never about existence.

**The ruling the integrator made** (and which this entry exists to finish):

| projection state | missing row means | answer |
|---|---|---|
| caught up | no declaration exists | `NeverDeclared` |
| **not** caught up | not yet delivered | `Unresolvable("projection lagging")` |

## Why it is not implemented

There is no caught-up signal for **this** pipeline. `identity_heads` is populated
by the mishpat `AppSignal` subscriber — `main.rs` → `subscribe_mishpat_signals`
→ `signals::handle_mishpat_signal` → `db::identity_heads::upsert_with_anchor` —
which is a **fire-and-forget callback closure**: no cursor row, no lag stamp, no
liveness state, and no registration at all when the DB pool or conductor bridge
is unavailable (it logs a `warn!` and the projection silently never starts).

The two catch-up signals that DO exist belong to other pipelines and answer a
different question:

- `p2p::replication::ReconcileState::caught_up` (`src/p2p/replication.rs`) tracks
  **content/blob replication** — whether pending content ids have drained. It says
  nothing about whether a mishpat commitment signal was delivered.
- `projector::status::compute_projector_status` (`src/projector/status.rs`) tracks
  the **EPR-atom projector** (`epr_atoms` + `projector_cursor`), per
  `(pillar, kind)` declared in the manifest registry. `identity_heads` is not one
  of its projections; no cursor row exists for it.

Wiring either one in would make a DID document's controller semantics depend on
whether *blob replication* had drained. It would look like the ruling was
implemented while leaving the same hole — the invented-wiring failure the
four-way `IdentityHeadAnswer` enum exists to prevent. Honest degradation with a
ledgered gap is the better state.

## Blast radius (bounded, and smaller than it was)

The **revoked** case is CLOSED (2026-08-02, same change): a revoked head now
surfaces via `db::identity_heads::find_notarized_head_by_head_key` and answers
`IdentityHeadAnswer::Revoked`, which assembles a **deactivated** document. The
prior behaviour — a `revoked_at IS NULL` filter that made a revoked head vanish,
so it resolved to a fully-armed, implicitly self-controlled document — is gone.

What remains is only the **never-delivered** case, whose worst outcome is the
*unchanged* phase-1 implicit-self document: a node that has not received an
agent's `binds-identity` yet serves the same document it served before identity
heads existed. That is a stale answer, not a newly-opened hole. It matters most
on a **freshly restarted node** (empty `identity_heads`, conductor still
integrating) and on a node whose conductor bridge is down (subscriber never
registered), where every agent reads as `NeverDeclared`.

## Definition of done

1. Give the mishpat→projection path a caught-up signal in its **own** pipeline.
   The cheapest shape that matches existing precedent is a cursor row per
   projected commitment action (mirroring `projector_cursor`), advanced by
   `handle_mishpat_signal`, plus a subscriber-registered/liveness stamp so
   "never subscribed" is distinguishable from "subscribed and idle".
2. Flip the `else` arm of `DidIdentityStore::identity_head` to the ruling: caught
   up ⇒ `NeverDeclared`; not caught up ⇒
   `IdentityHeadAnswer::Unresolvable("projection lagging")`. The did-bridge
   resolver already fails closed on `Unresolvable` (no document), and both
   elohim-storage's `/db/identity/did/{did}` handler and doorway's
   `routes::identity::error_status` already map it to **503** (retryable), so no
   downstream change is needed — this is one branch.
3. Test the lagging branch asserts `Unresolvable`, not `NeverDeclared` (asserting
   the latter would launder the gap into a green test).

## Adjacent, deliberately separate: the successor column has no producer

`identity_heads.successor_head_key` (migration
`2026-08-02-140000_identity_heads_successor_head_key`) is the honest home for the
head a rotation/recovery names as the identity's continuation — the re-anchor
half of C9, riding the deactivated document's `alsoKnownAs`.

**No declaration names one today.** The mishpat coordinator validator
`validate_binds_identity` does not require a successor field, and
`revokes-commitment` carries only `{target_cid, reason, signed_at}`. So every
revoked row currently reads as a **terminal** revocation — which is an *accurate*
reading of what those declarations say, not a guess, and is why
`RevokedIdentity::successor = None` is honest here rather than the
`Unresolvable` the did-bridge contract demands of a store that cannot
*determine* the successor.

`mishpat_projection::parse_binds_identity` reads the field **optionally**, so the
column is correct-but-dormant: a rotation that starts declaring a successor lands
in it with no further storage change. Closing this half means extending the
`binds-identity` / `revokes-commitment` payload (a coordinator-zome change —
DNA-hash-neutral, healed by `update_coordinators`) and a matching validator
clause. Tracked here so the dormancy is recorded rather than read as a wired
feature.
